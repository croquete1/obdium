//! Standard ELM327 OBD-II protocol selections (0–9).
//! Adapter-reported configuration is not evidence that an ECU responded.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolSelection {
    Automatic,
    Manual(ObdProtocol),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ObdProtocol {
    J1850Pwm = 1,
    J1850Vpw = 2,
    Iso9141 = 3,
    KwpSlow = 4,
    KwpFast = 5,
    Can11At500 = 6,
    Can29At500 = 7,
    Can11At250 = 8,
    Can29At250 = 9,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    Unsupported,
    Malformed,
    Unacknowledged,
}

impl TryFrom<u8> for ObdProtocol {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::J1850Pwm),
            2 => Ok(Self::J1850Vpw),
            3 => Ok(Self::Iso9141),
            4 => Ok(Self::KwpSlow),
            5 => Ok(Self::KwpFast),
            6 => Ok(Self::Can11At500),
            7 => Ok(Self::Can29At500),
            8 => Ok(Self::Can11At250),
            9 => Ok(Self::Can29At250),
            _ => Err(ProtocolError::Unsupported),
        }
    }
}

impl TryFrom<u8> for ProtocolSelection {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(Self::Automatic)
        } else {
            ObdProtocol::try_from(value).map(Self::Manual)
        }
    }
}

impl ProtocolSelection {
    pub fn command(self) -> &'static [u8] {
        match self {
            Self::Automatic => b"ATSP0",
            Self::Manual(ObdProtocol::J1850Pwm) => b"ATSP1",
            Self::Manual(ObdProtocol::J1850Vpw) => b"ATSP2",
            Self::Manual(ObdProtocol::Iso9141) => b"ATSP3",
            Self::Manual(ObdProtocol::KwpSlow) => b"ATSP4",
            Self::Manual(ObdProtocol::KwpFast) => b"ATSP5",
            Self::Manual(ObdProtocol::Can11At500) => b"ATSP6",
            Self::Manual(ObdProtocol::Can29At500) => b"ATSP7",
            Self::Manual(ObdProtocol::Can11At250) => b"ATSP8",
            Self::Manual(ObdProtocol::Can29At250) => b"ATSP9",
        }
    }
}

impl ObdProtocol {
    /// CAN identifier width and nominal bus bit rate; never the host serial baud rate.
    pub fn can_parameters(self) -> Option<(u8, u32)> {
        match self {
            Self::Can11At500 => Some((11, 500_000)),
            Self::Can29At500 => Some((29, 500_000)),
            Self::Can11At250 => Some((11, 250_000)),
            Self::Can29At250 => Some((29, 250_000)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportedProtocol {
    pub automatic: bool,
    pub protocol: Option<ObdProtocol>,
}

/// Parse an already prompt-framed AT DPN response with echo disabled.
/// This describes adapter configuration, not verified vehicle communication.
pub fn parse_reported_protocol(reply: &str) -> Result<ReportedProtocol, ProtocolError> {
    let bytes = reply.trim_ascii().as_bytes();
    let (automatic, digit) = match bytes {
        [digit] => (false, *digit),
        [b'A' | b'a', digit] => (true, *digit),
        _ => return Err(ProtocolError::Malformed),
    };
    let protocol = match digit {
        b'0' => None,
        b'1'..=b'9' => Some(ObdProtocol::try_from(digit - b'0')?),
        b'A'..=b'F' | b'a'..=b'f' => return Err(ProtocolError::Unsupported),
        _ => return Err(ProtocolError::Malformed),
    };
    Ok(ReportedProtocol {
        automatic: automatic || digit == b'0',
        protocol,
    })
}

/// Exact acknowledgement from a prompt-framed reply, not a substring match.
pub fn validate_selection_ack(reply: &str) -> Result<(), ProtocolError> {
    if reply.trim_ascii() == "OK" {
        Ok(())
    } else {
        Err(ProtocolError::Unacknowledged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ten_selections_map_to_exact_commands() {
        for id in 0..=9 {
            let command = ProtocolSelection::try_from(id).unwrap().command();
            assert_eq!(command, &[b'A', b'T', b'S', b'P', b'0' + id]);
        }
        for id in 10..=u8::MAX {
            assert_eq!(
                ProtocolSelection::try_from(id),
                Err(ProtocolError::Unsupported)
            );
        }
    }

    #[test]
    fn all_nine_reported_protocols_support_manual_and_auto() {
        for id in 1..=9 {
            let protocol = Some(ObdProtocol::try_from(id).unwrap());
            for automatic in [false, true] {
                let reply = if automatic {
                    format!("A{id}\r")
                } else {
                    format!("{id}\r")
                };
                assert_eq!(
                    parse_reported_protocol(&reply),
                    Ok(ReportedProtocol {
                        automatic,
                        protocol
                    })
                );
            }
        }
    }

    #[test]
    fn automatic_without_known_protocol_is_not_vehicle_detection() {
        for reply in ["0", "A0", "a0\r\n"] {
            assert_eq!(
                parse_reported_protocol(reply),
                Ok(ReportedProtocol {
                    automatic: true,
                    protocol: None
                })
            );
        }
    }

    #[test]
    fn malformed_or_unsupported_reports_are_rejected() {
        for reply in [
            "",
            "OK",
            "NO DATA",
            "ATDPN\rA6",
            "6\r7",
            "A 6",
            "A6>",
            "16",
            "é",
        ] {
            assert!(parse_reported_protocol(reply).is_err(), "{reply:?}");
        }
        for reply in ["A", "B", "C", "AA", "AB", "AC"] {
            assert_eq!(
                parse_reported_protocol(reply),
                Err(ProtocolError::Unsupported)
            );
        }
    }

    #[test]
    fn can_metadata_is_distinct_from_legacy_protocols() {
        let expected = [(11, 500_000), (29, 500_000), (11, 250_000), (29, 250_000)];
        for id in 1..=5 {
            assert_eq!(ObdProtocol::try_from(id).unwrap().can_parameters(), None);
        }
        for (id, parameters) in (6..=9).zip(expected) {
            assert_eq!(
                ObdProtocol::try_from(id).unwrap().can_parameters(),
                Some(parameters)
            );
        }
    }

    #[test]
    fn only_exact_ok_acknowledges_selection() {
        assert_eq!(validate_selection_ack(" \rOK\r\n"), Ok(()));
        for reply in ["", "?", "STOPPED", "NOT OK", "OK\rERROR", "ATSP6\rOK"] {
            assert_eq!(
                validate_selection_ack(reply),
                Err(ProtocolError::Unacknowledged)
            );
        }
    }
}
