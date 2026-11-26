//! Reassemble classic CAN ISO-TP replies using normal addressing.
//! Input is already tokenized into CAN IDs and data bytes; no adapter I/O.

use std::collections::BTreeMap;

const MAX_FRAMES: usize = 4096;
const MAX_ECUS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanId {
    Standard(u16),
    Extended(u32),
}

impl CanId {
    fn is_valid(self) -> bool {
        match self {
            Self::Standard(id) => id <= 0x7FF,
            Self::Extended(id) => id <= 0x1FFF_FFFF,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanFrame<'a> {
    pub ecu: CanId,
    pub data: &'a [u8],
}

#[derive(Debug, PartialEq, Eq)]
pub struct CanReply {
    pub ecu: CanId,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IsoTpError {
    #[error("Too many frames in a diagnostic transaction")]
    TooManyFrames,
    #[error("Too many responding ECUs")]
    TooManyEcus,
    #[error("Invalid CAN identifier: {0:?}")]
    InvalidId(CanId),
    #[error("Invalid classic CAN ISO-TP frame: {0:?}")]
    InvalidFrame(CanId),
    #[error("Unsupported ISO-TP frame format: {0:?}")]
    UnsupportedFrame(CanId),
    #[error("Unexpected or duplicate ISO-TP frame: {0:?}")]
    UnexpectedFrame(CanId),
    #[error("Incorrect ISO-TP sequence number: {0:?}")]
    Sequence(CanId),
    #[error("Incomplete ISO-TP reply: {0:?}")]
    Incomplete(CanId),
}

struct Assembly {
    length: usize,
    payload: Vec<u8>,
    next_sequence: u8,
}

/// Return one complete reply per ECU, sorted by CAN ID.
///
/// A single diagnostic transaction may contain interleaved replies. Never join
/// bytes from distinct ECUs or return a partial batch as success. The caller
/// must establish transaction boundaries, normal addressing and classic CAN
/// framing; this function does not support CAN FD, extended/mixed addressing,
/// extended-length first frames or flow-control frames in the reply stream.
pub fn assemble_classic_can_replies(
    frames: &[CanFrame<'_>],
) -> Result<Vec<CanReply>, IsoTpError> {
    if frames.len() > MAX_FRAMES {
        return Err(IsoTpError::TooManyFrames);
    }

    let mut assemblies = BTreeMap::<CanId, Assembly>::new();
    for frame in frames {
        let ecu = frame.ecu;
        if !ecu.is_valid() {
            return Err(IsoTpError::InvalidId(ecu));
        }
        if frame.data.is_empty() || frame.data.len() > 8 {
            return Err(IsoTpError::InvalidFrame(ecu));
        }

        let pci = frame.data[0];
        match pci >> 4 {
            0 | 1 => {
                if assemblies.contains_key(&ecu) {
                    return Err(IsoTpError::UnexpectedFrame(ecu));
                }
                if assemblies.len() >= MAX_ECUS {
                    return Err(IsoTpError::TooManyEcus);
                }
                let (length, payload) = if pci >> 4 == 0 {
                    let length = usize::from(pci & 0x0F);
                    if length == 0 || length > 7 || frame.data.len() < length + 1 {
                        return Err(IsoTpError::InvalidFrame(ecu));
                    }
                    (length, frame.data[1..length + 1].to_vec())
                } else {
                    if frame.data.len() != 8 {
                        return Err(IsoTpError::InvalidFrame(ecu));
                    }
                    let length = (usize::from(pci & 0x0F) << 8) | usize::from(frame.data[1]);
                    if length == 0 {
                        return Err(IsoTpError::UnsupportedFrame(ecu));
                    }
                    if length <= 7 {
                        return Err(IsoTpError::InvalidFrame(ecu));
                    }
                    (length, frame.data[2..].to_vec())
                };
                assemblies.insert(
                    ecu,
                    Assembly {
                        length,
                        payload,
                        next_sequence: 1,
                    },
                );
            }
            2 => {
                let state = assemblies
                    .get_mut(&ecu)
                    .ok_or(IsoTpError::UnexpectedFrame(ecu))?;
                if state.payload.len() == state.length {
                    return Err(IsoTpError::UnexpectedFrame(ecu));
                }
                if pci & 0x0F != state.next_sequence {
                    return Err(IsoTpError::Sequence(ecu));
                }
                let take = (state.length - state.payload.len()).min(7);
                if frame.data.len() < take + 1 {
                    return Err(IsoTpError::InvalidFrame(ecu));
                }
                state.payload.extend_from_slice(&frame.data[1..take + 1]);
                state.next_sequence = (state.next_sequence + 1) & 0x0F;
            }
            _ => return Err(IsoTpError::UnsupportedFrame(ecu)),
        }
    }

    assemblies
        .into_iter()
        .map(|(ecu, state)| {
            if state.payload.len() != state.length {
                return Err(IsoTpError::Incomplete(ecu));
            }
            Ok(CanReply {
                ecu,
                payload: state.payload,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: CanId = CanId::Standard(0x7E8);
    const B: CanId = CanId::Standard(0x7E9);

    fn frame(ecu: CanId, data: &[u8]) -> CanFrame<'_> {
        CanFrame { ecu, data }
    }

    #[test]
    fn single_frame_preserves_zero_payload_and_ignores_padding() {
        let replies = assemble_classic_can_replies(&[frame(A, &[4, 0x43, 1, 0x14, 0, 0, 0, 0])])
            .unwrap();
        assert_eq!(replies[0].ecu, A);
        assert_eq!(replies[0].payload, [0x43, 1, 0x14, 0]);
    }

    #[test]
    fn interleaved_ecus_are_never_combined() {
        let replies = assemble_classic_can_replies(&[
            frame(A, &[0x10, 9, 1, 2, 3, 4, 5, 6]),
            frame(B, &[0x10, 8, 11, 12, 13, 14, 15, 16]),
            frame(A, &[0x21, 7, 8, 9, 0, 0, 0, 0]),
            frame(B, &[0x21, 17, 18]),
        ])
        .unwrap();
        assert_eq!(replies[0].ecu, A);
        assert_eq!(replies[0].payload, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(replies[1].ecu, B);
        assert_eq!(replies[1].payload, [11, 12, 13, 14, 15, 16, 17, 18]);
    }

    #[test]
    fn extended_id_is_distinct_from_standard_id() {
        let extended = CanId::Extended(0x7E8);
        let replies =
            assemble_classic_can_replies(&[frame(A, &[1, 1]), frame(extended, &[1, 2])]).unwrap();
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[1].ecu, extended);
        assert_eq!(replies[1].payload, [2]);
    }

    #[test]
    fn incomplete_batch_cannot_report_success() {
        assert_eq!(
            assemble_classic_can_replies(&[
                frame(A, &[1, 0x44]),
                frame(B, &[0x10, 9, 1, 2, 3, 4, 5, 6]),
            ]),
            Err(IsoTpError::Incomplete(B))
        );
    }

    #[test]
    fn rejects_wrong_sequence_and_orphan_continuation() {
        assert_eq!(
            assemble_classic_can_replies(&[
                frame(A, &[0x10, 8, 1, 2, 3, 4, 5, 6]),
                frame(A, &[0x22, 7, 8]),
            ]),
            Err(IsoTpError::Sequence(A))
        );
        assert_eq!(
            assemble_classic_can_replies(&[frame(B, &[0x21, 7, 8])]),
            Err(IsoTpError::UnexpectedFrame(B))
        );
    }

    #[test]
    fn rejects_duplicate_complete_reply() {
        assert_eq!(
            assemble_classic_can_replies(&[frame(A, &[1, 1]), frame(A, &[1, 2])]),
            Err(IsoTpError::UnexpectedFrame(A))
        );
    }

    #[test]
    fn rejects_truncated_frames_and_unsupported_formats() {
        for data in [&[][..], &[4, 1][..], &[0x10, 8, 1][..], &[0; 9][..]] {
            assert_eq!(
                assemble_classic_can_replies(&[frame(A, data)]),
                Err(IsoTpError::InvalidFrame(A))
            );
        }
        assert_eq!(
            assemble_classic_can_replies(&[frame(A, &[0x30, 0, 0])]),
            Err(IsoTpError::UnsupportedFrame(A))
        );
        assert_eq!(
            assemble_classic_can_replies(&[frame(A, &[0x10, 0, 0, 0, 0, 0, 0, 8])]),
            Err(IsoTpError::UnsupportedFrame(A))
        );
    }

    #[test]
    fn sequence_wraps_from_fifteen_to_zero() {
        let mut data = vec![vec![0x10, 118, 0, 0, 0, 0, 0, 0]];
        for number in 1..=16 {
            let mut bytes = vec![0x20 | (number & 0x0F)];
            bytes.extend([number; 7]);
            data.push(bytes);
        }
        let frames: Vec<_> = data.iter().map(|bytes| frame(A, bytes)).collect();
        let replies = assemble_classic_can_replies(&frames).unwrap();
        assert_eq!(replies[0].payload.len(), 118);
        assert_eq!(&replies[0].payload[111..], &[16; 7]);
    }

    #[test]
    fn validates_id_and_bounds() {
        let invalid = CanId::Standard(0x800);
        assert_eq!(
            assemble_classic_can_replies(&[frame(invalid, &[1, 1])]),
            Err(IsoTpError::InvalidId(invalid))
        );
        let frames = vec![frame(A, &[1, 1]); MAX_FRAMES + 1];
        assert_eq!(
            assemble_classic_can_replies(&frames),
            Err(IsoTpError::TooManyFrames)
        );
        let frames: Vec<_> = (0..=MAX_ECUS)
            .map(|id| frame(CanId::Standard(id as u16), &[1, 1]))
            .collect();
        assert_eq!(
            assemble_classic_can_replies(&frames),
            Err(IsoTpError::TooManyEcus)
        );
    }
}
