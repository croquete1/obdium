//! Read-only decoding of a single collected classic CAN DTC transaction.
//! No adapter commands are sent by this module.

use crate::isotp::{parse_elm_classic_can_reply, CanId, IsoTpError};
use crate::diagnostics::{decode_can_dtc_payload, DecodedDtc, DtcPayloadError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtcService {
    Confirmed,
    Pending,
    Permanent,
}

impl DtcService {
    fn response(self) -> u8 {
        match self {
            Self::Confirmed => 0x43,
            Self::Pending => 0x47,
            Self::Permanent => 0x4A,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EcuDtcReply {
    pub ecu: CanId,
    pub payload: Vec<u8>,
    pub codes: Vec<DecodedDtc>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DtcTransactionError {
    #[error(transparent)]
    Transport(#[from] IsoTpError),
    #[error("ECU {ecu:?} returned service {actual:#04x}, expected {expected:#04x}")]
    UnexpectedService {
        ecu: CanId,
        expected: u8,
        actual: u8,
    },
    #[error("Invalid DTC payload from ECU {ecu:?}: {source}")]
    Payload {
        ecu: CanId,
        #[source]
        source: DtcPayloadError,
    },
}

/// Interpret a collected ELM transaction for exactly one requested DTC service.
///
/// Keep empty successful replies per ECU, preserve original payloads and reject
/// the entire batch on malformed, negative or mismatched responses. A complete
/// observed batch is not proof that every ECU on the vehicle has responded.
/// The caller is responsible for freshness, addressing and adapter configuration.
pub fn decode_elm_dtc_transaction(
    raw: &str,
    service: DtcService,
) -> Result<Vec<EcuDtcReply>, DtcTransactionError> {
    let expected = service.response();
    parse_elm_classic_can_reply(raw)?
        .into_iter()
        .map(|reply| {
            if let Some(&actual) = reply.payload.first() {
                if actual != expected {
                    return Err(DtcTransactionError::UnexpectedService {
                        ecu: reply.ecu,
                        expected,
                        actual,
                    });
                }
            }
            let codes = decode_can_dtc_payload(&reply.payload).map_err(|source| {
                DtcTransactionError::Payload {
                    ecu: reply.ecu,
                    source,
                }
            })?;
            Ok(EcuDtcReply {
                ecu: reply.ecu,
                payload: reply.payload,
                codes,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::DtcStatus;

    #[test]
    fn interleaved_replies_preserve_ecu_identity_and_exact_codes() {
        let raw = "7E8 10 08 43 03 14 3C 03 00\r7E9 04 43 01 14 3C\r7E8 21 43 4A";
        let replies = decode_elm_dtc_transaction(raw, DtcService::Confirmed).unwrap();
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0].ecu, CanId::Standard(0x7E8));
        assert_eq!(replies[1].ecu, CanId::Standard(0x7E9));
        let names: Vec<_> = replies[0].codes.iter().map(|code| code.code.as_str()).collect();
        assert_eq!(names, ["P143C", "P0300", "C034A"]);
        assert_eq!(replies[1].codes[0].code, "P143C");
        assert_eq!(replies[0].payload, [0x43, 3, 0x14, 0x3C, 3, 0, 0x43, 0x4A]);
    }

    #[test]
    fn same_code_can_have_distinct_statuses() {
        for (service, raw, status) in [
            (DtcService::Confirmed, "7E8 04 43 01 14 3C", DtcStatus::Confirmed),
            (DtcService::Pending, "7E8 04 47 01 14 3C", DtcStatus::Pending),
            (DtcService::Permanent, "7E8 04 4A 01 14 3C", DtcStatus::Permanent),
        ] {
            let replies = decode_elm_dtc_transaction(raw, service).unwrap();
            assert_eq!(replies[0].codes[0].status, status);
            assert_eq!(replies[0].codes[0].code, "P143C");
        }
    }

    #[test]
    fn explicit_zero_count_retains_responding_ecu() {
        let replies =
            decode_elm_dtc_transaction("7E8 02 43 00", DtcService::Confirmed).unwrap();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].ecu, CanId::Standard(0x7E8));
        assert!(replies[0].codes.is_empty());
        assert_eq!(replies[0].payload, [0x43, 0]);
    }

    #[test]
    fn wrong_service_cannot_be_reported_as_requested_status() {
        assert_eq!(
            decode_elm_dtc_transaction("7E8 04 47 01 14 3C", DtcService::Confirmed),
            Err(DtcTransactionError::UnexpectedService {
                ecu: CanId::Standard(0x7E8),
                expected: 0x43,
                actual: 0x47,
            })
        );
    }

    #[test]
    fn negative_reply_rejects_otherwise_valid_batch() {
        assert_eq!(
            decode_elm_dtc_transaction("7E8 02 43 00\r7E9 03 7F 03 11", DtcService::Confirmed),
            Err(DtcTransactionError::UnexpectedService {
                ecu: CanId::Standard(0x7E9),
                expected: 0x43,
                actual: 0x7F,
            })
        );
    }

    #[test]
    fn bad_count_preserves_failing_ecu_in_error() {
        assert_eq!(
            decode_elm_dtc_transaction("7E8 02 43 00\r7E9 04 43 02 14 3C", DtcService::Confirmed),
            Err(DtcTransactionError::Payload {
                ecu: CanId::Standard(0x7E9),
                source: DtcPayloadError::InvalidLength,
            })
        );
    }

    #[test]
    fn missing_or_incomplete_replies_are_not_no_codes() {
        assert_eq!(
            decode_elm_dtc_transaction("", DtcService::Confirmed),
            Err(DtcTransactionError::Transport(IsoTpError::NoFrames))
        );
        assert_eq!(
            decode_elm_dtc_transaction("7E8 10 08 43 03 14 3C 03 00", DtcService::Confirmed),
            Err(DtcTransactionError::Transport(IsoTpError::Incomplete(
                CanId::Standard(0x7E8)
            )))
        );
    }
}
