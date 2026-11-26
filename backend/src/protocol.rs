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

/// Conservative evidence for the standard Mode 01 PID 00 CAN reply.
/// Requires ELM headers, spaces and the single-frame length byte.
/// Other valid adapter layouts are deliberately unverified here.
pub fn can_probe_identifier_width(reply: &str) -> Result<u8, ProtocolError> {
    let mut width = None;
    for line in reply.split(['\r', '\n']).map(str::trim_ascii) {
        if line.is_empty() || line == "SEARCHING..." {
            continue;
        }
        let mut tokens = line.split_ascii_whitespace();
        let identifier = tokens.next().ok_or(ProtocolError::Malformed)?;
        if !identifier.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ProtocolError::Malformed);
        }
        let id = u32::from_str_radix(identifier, 16).map_err(|_| ProtocolError::Malformed)?;
        let current_width = match identifier.len() {
            3 if (0x7E8..=0x7EF).contains(&id) => 11,
            8 if id & 0x1FFF_FF00 == 0x18DA_F100 && id <= 0x1FFF_FFFF => 29,
            _ => return Err(ProtocolError::Malformed),
        };
        if width.is_some_and(|previous| previous != current_width) {
            return Err(ProtocolError::Malformed);
        }

        let mut bytes = [0; 8];
        let mut count = 0;
        for token in tokens {
            if count == bytes.len()
                || token.len() != 2
                || !token.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(ProtocolError::Malformed);
            }
            bytes[count] = u8::from_str_radix(token, 16).map_err(|_| ProtocolError::Malformed)?;
            count += 1;
        }
        // PCI 06 + 41 00 + exactly four bitmap bytes, optional single padding byte.
        if !(7..=8).contains(&count) || bytes[..3] != [0x06, 0x41, 0x00] {
            return Err(ProtocolError::Malformed);
        }
        width = Some(current_width);
    }
    width.ok_or(ProtocolError::Malformed)
}

/// Correlate a positive CAN probe with the adapter's reported configuration.
/// The bus rate remains adapter-reported, not independently measured.
pub fn confirm_can_probe(
    selection: ProtocolSelection,
    report: ReportedProtocol,
    reply: &str,
) -> Result<ObdProtocol, ProtocolError> {
    let width = can_probe_identifier_width(reply)?;
    let protocol = report.protocol.ok_or(ProtocolError::Malformed)?;
    if protocol.can_parameters().map(|(bits, _)| bits) != Some(width) {
        return Err(ProtocolError::Malformed);
    }
    if let ProtocolSelection::Manual(requested) = selection {
        if requested != protocol || report.automatic {
            return Err(ProtocolError::Unacknowledged);
        }
    } else if !report.automatic {
        return Err(ProtocolError::Unacknowledged);
    }
    Ok(protocol)
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
    fn standard_can_probes_preserve_identifier_width() {
        assert_eq!(
            can_probe_identifier_width("SEARCHING...\r7E8 06 41 00 00 00 00 00\r"),
            Ok(11)
        );
        assert_eq!(
            can_probe_identifier_width("18DAF110 06 41 00 FF FF FF FF 00"),
            Ok(29)
        );
        assert_eq!(
            can_probe_identifier_width("7E8 06 41 00 00 00 00 00\r7E9 06 41 00 FF FF FF FF"),
            Ok(11)
        );
    }

    #[test]
    fn can_probes_reject_unrelated_incomplete_or_mixed_responses() {
        for reply in [
            "NO DATA",
            "7E8 03 7F 01 11",
            "7E8 06 41 00 00 00 00",
            "7E8 06 41 20 00 00 00 00",
            "7DF 06 41 00 00 00 00 00",
            "7E8 06 41 00 GG 00 00 00",
            "7E8 06 41 00 00 00 00 00 00 00",
            "7E8 06 41 00 00 00 00 00\r18DAF110 06 41 00 00 00 00 00",
            "7E8 06 41 00 00 00 00 00\rCAN ERROR",
            "SEARCHING...",
        ] {
            assert!(can_probe_identifier_width(reply).is_err(), "{reply:?}");
        }
    }

    #[test]
    fn can_protocol_confirmation_requires_matching_report_and_selection() {
        let reply = "7E8 06 41 00 00 00 00 00";
        for id in [6, 8] {
            let report = parse_reported_protocol(&format!("A{id}")).unwrap();
            let expected = ObdProtocol::try_from(id).unwrap();
            assert_eq!(
                confirm_can_probe(ProtocolSelection::Automatic, report, reply),
                Ok(expected)
            );
            let manual = parse_reported_protocol(&id.to_string()).unwrap();
            assert_eq!(
                confirm_can_probe(ProtocolSelection::Manual(expected), manual, reply),
                Ok(expected)
            );
        }
        for report in ["A0", "A3", "A7", "6"] {
            assert!(confirm_can_probe(
                ProtocolSelection::Automatic,
                parse_reported_protocol(report).unwrap(),
                reply
            )
            .is_err());
        }
        assert!(confirm_can_probe(
            ProtocolSelection::Manual(ObdProtocol::Can11At500),
            parse_reported_protocol("8").unwrap(),
            reply
        )
        .is_err());
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
