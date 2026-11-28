use serde::{Deserialize, Serialize};
use sqlite::State;
use std::fmt;

use crate::{
    scalar::{Scalar, Unit},
    Command, Error, CODE_DESC_DB_PATH, OBD,
};

#[derive(Debug)]
pub enum OBDStandard {
    Standard(&'static str),
}

impl OBDStandard {
    pub fn from_u8(num: u8) -> Self {
        match num {
            1u8 => Self::Standard("OBD-II as defined by CARB"),
            2u8 => Self::Standard("OBD as defined by the EPA"),
            3u8 => Self::Standard("OBD and OBD-II"),
            4u8 => Self::Standard("OBD-I"),
            5u8 => Self::Standard("Not OBD compliant"),
            6u8 => Self::Standard("EOBD"),
            7u8 => Self::Standard("EOBD and OBD-II"),
            8u8 => Self::Standard("EOBD and OBD"),
            9u8 => Self::Standard("EOBD, OBD and OBD-II"),
            10u8 => Self::Standard("JOBD"),
            11u8 => Self::Standard("JOBD and OBD-II"),
            12u8 => Self::Standard("JOBD and EOBD"),
            13u8 => Self::Standard("JOBD, EOBD and OBD-II"),
            // 14-16: Reserved
            17u8 => Self::Standard("Engine Manufacturer Diagnostics"),
            18u8 => Self::Standard("Engine Manufacturer Diagnostics Enhanced"),
            19u8 => Self::Standard("Heavy Duty On-Board Diagnostics (Child/Partial)"),
            20u8 => Self::Standard("Heavy Duty On-Board Diagnostics"),
            21u8 => Self::Standard("World Wide Harmonized OBD"),
            // 22: Reserved
            23u8 => Self::Standard("Heavy Duty Euro OBD Stage I without NOx control"),
            24u8 => Self::Standard("Heavy Duty Euro OBD Stage I with NOx control"),
            25u8 => Self::Standard("Heavy Duty Euro OBD Stage II without NOx control"),
            26u8 => Self::Standard("Heavy Duty Euro OBD Stage II with NOx control"),
            // 27: Reserved
            28u8 => Self::Standard("Brazil OBD Phase 1"),
            29u8 => Self::Standard("Brazil OBD Phase 2"),
            30u8 => Self::Standard("Korean OBD"),
            31u8 => Self::Standard("India OBD I"),
            32u8 => Self::Standard("India OBD II"),
            33u8 => Self::Standard("Heavy Duty Euro OBD Stage VI"),
            // 34-250: Reserved
            // 251-255: Unavailable
            _ => Self::Standard("No data"),
        }
    }
}

impl fmt::Display for OBDStandard {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OBDStandard::Standard(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug)]
pub enum AuxiliaryInputStatus {
    InUse,
    NotInUse,
}

impl AuxiliaryInputStatus {
    pub fn as_str(&self) -> &str {
        match self {
            AuxiliaryInputStatus::InUse => "Active",
            AuxiliaryInputStatus::NotInUse => "Inactive",
        }
    }
}

impl fmt::Display for AuxiliaryInputStatus {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TroubleCodeCategory {
    Powertrain,
    Chassis,
    Body,
    Network,
    #[default]
    Unknown,
}

impl TroubleCodeCategory {
    pub fn system_letter(&self) -> char {
        match self {
            TroubleCodeCategory::Powertrain => 'P',
            TroubleCodeCategory::Chassis => 'C',
            TroubleCodeCategory::Body => 'B',
            TroubleCodeCategory::Network => 'U',
            TroubleCodeCategory::Unknown => '?',
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            TroubleCodeCategory::Powertrain => "Powertrain",
            TroubleCodeCategory::Chassis => "Chassis",
            TroubleCodeCategory::Body => "Body",
            TroubleCodeCategory::Network => "Network",
            TroubleCodeCategory::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for TroubleCodeCategory {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Status is determined by the positive response service, never by the code bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtcStatus {
    Confirmed,
    Pending,
    Permanent,
}

/// One code decoded from a complete, reassembled CAN diagnostic message.
/// ECU identity and transport framing belong to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedDtc {
    pub code: String,
    pub status: DtcStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DtcPayloadError {
    #[error("CAN DTC response is missing its service or code count")]
    MissingHeader,
    #[error("Unsupported DTC response service: {0:#04x}")]
    UnsupportedService(u8),
    #[error("CAN DTC response length does not match its declared count")]
    InvalidLength,
    #[error("CAN DTC response contains a zero code inside the declared count")]
    InvalidZeroCode,
}

/// Decode exactly one reassembled SAE CAN Mode 03, 07 or 0A positive payload.
///
/// The caller must remove ISO-TP framing and padding using its declared message
/// length and retain the source ECU. This is not a parser for raw ELM strings,
/// concatenated ECU replies, legacy non-CAN messages or UDS records.
pub fn decode_can_dtc_payload(payload: &[u8]) -> Result<Vec<DecodedDtc>, DtcPayloadError> {
    let [service, count, data @ ..] = payload else {
        return Err(DtcPayloadError::MissingHeader);
    };
    let status = match service {
        0x43 => DtcStatus::Confirmed,
        0x47 => DtcStatus::Pending,
        0x4A => DtcStatus::Permanent,
        other => return Err(DtcPayloadError::UnsupportedService(*other)),
    };
    if data.len() != usize::from(*count) * 2 {
        return Err(DtcPayloadError::InvalidLength);
    }

    let mut codes = Vec::with_capacity(usize::from(*count));
    for pair in data.as_chunks::<2>().0 {
        let left = pair[0];
        let right = pair[1];
        if left == 0 && right == 0 {
            return Err(DtcPayloadError::InvalidZeroCode);
        }
        let category = match left >> 6 {
            0 => 'P',
            1 => 'C',
            2 => 'B',
            _ => 'U',
        };
        let code = format!(
            "{category}{:X}{:X}{right:02X}",
            (left >> 4) & 0x03,
            left & 0x0F
        );
        codes.push(DecodedDtc { code, status });
    }
    Ok(codes)
}

#[derive(Debug, Default, Clone)]
pub struct TroubleCode {
    pub category: TroubleCodeCategory,
    pub dtc: String,
    pub description: String,

    /// Is this a permanant OBD code or a pending one ?
    pub permanant: bool,
}

impl TroubleCode {
    pub fn new(category: TroubleCodeCategory, dtc: String, permanant: bool) -> Self {
        let mut code = Self {
            category,
            dtc,
            description: String::default(),
            permanant,
        };

        code.set_description();
        code
    }

    pub fn set_description(&mut self) {
        self.description = "none".to_string(); // default

        // connect to trouble code data base
        let con = match sqlite::Connection::open(CODE_DESC_DB_PATH) {
            Ok(con) => con,
            Err(err) => {
                println!("when connecting to codes database: {err}");
                return;
            }
        };

        // query description for 'dtc'
        let query = "SELECT desc FROM codes WHERE id = ?";
        let mut statement = match con.prepare(query) {
            Ok(statement) => statement,
            Err(err) => {
                println!("when sanitizing statement {query}: {err}");
                return;
            }
        };

        match statement.bind((1, self.dtc.as_str())) {
            Ok(_) => {}
            Err(err) => {
                println!("when binding dtc '{}' to query {query}: {err}", self.dtc);
                return;
            }
        };

        if let Ok(State::Row) = statement.next() {
            self.description = statement.read::<String, _>("desc").unwrap_or_default();
        }
    }
}

impl fmt::Display for TroubleCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Showing details for trouble code {}:", self.dtc)?;
        writeln!(f, "Trouble code: {}", self.dtc)?;
        writeln!(f, "System letter: {}", self.category.system_letter())?;
        writeln!(f, "Location: {}", self.category)?;
        writeln!(f, "Description: {}", self.description)?;
        write!(f, "Overview: {} is an issue related to {}. This code is from the {} component of the vehicle.", self.dtc, self.description, self.category)
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Test {
    pub name: &'static str,
    pub available: bool,
    pub complete: bool,
}

impl fmt::Display for Test {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Test name: {}", self.name).unwrap();
        writeln!(
            f,
            "Available: {}",
            if self.available { "Yes" } else { "No" }
        )
        .unwrap();
        write!(f, "Complete: {}", if self.complete { "Yes" } else { "No" })
    }
}

impl Test {
    pub fn new(name: &'static str, available: bool, complete: bool) -> Self {
        Self {
            name,
            available,
            complete,
        }
    }

    fn no_data() -> Self {
        Self {
            name: "Unknown",
            ..Default::default()
        }
    }
}

// ELM header-on, spaces-on, visible classic CAN single frame only.
// A single response avoids silently merging different ECUs. The generalized
// ISO-TP transaction parser can replace this narrow adapter later.
fn decode_readiness_reply(raw: &str) -> Option<[Test; 11]> {
    let mut lines = raw
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let line = lines.next()?;
    if lines.next().is_some() {
        return None;
    }
    let mut tokens = line.split_ascii_whitespace();
    let id = tokens.next()?;
    let valid_id = matches!(id.len(), 3 | 8)
        && id.bytes().all(|byte| byte.is_ascii_hexdigit())
        && u32::from_str_radix(id, 16)
            .ok()
            .is_some_and(|value| value <= if id.len() == 3 { 0x7ff } else { 0x1fffffff });
    if !valid_id {
        return None;
    }

    let mut bytes = [0u8; 7];
    for byte in &mut bytes {
        let token = tokens.next()?;
        if token.len() != 2 || !token.bytes().all(|value| value.is_ascii_hexdigit()) {
            return None;
        }
        *byte = u8::from_str_radix(token, 16).ok()?;
    }
    if tokens.next().is_some() || bytes[..3] != [0x06, 0x41, 0x01] {
        return None;
    }
    Some(decode_readiness_bytes([
        bytes[3], bytes[4], bytes[5], bytes[6],
    ]))
}

// J1979 PID 01: A is MIL/count; B is common monitors + ignition flag;
// C is advanced availability, D advanced incompleteness.
fn decode_readiness_bytes([_, b, c, d]: [u8; 4]) -> [Test; 11] {
    let mut tests = [Test::no_data(); 11];
    for (index, name) in ["Components", "Fuel System", "Misfire"]
        .into_iter()
        .enumerate()
    {
        let bit = 1 << (2 - index);
        let available = b & bit != 0;
        tests[index] = Test::new(name, available, available && b & (bit << 4) == 0);
    }
    let names = if b & 0x08 == 0 {
        [
            "EGR and/or VVT System",
            "Oxygen Sensor Heater",
            "Oxygen Sensor",
            "Gasoline Particulate Filter",
            "Secondary Air System",
            "Evaporative System",
            "Heated Catalyst",
            "Catalyst",
        ]
    } else {
        [
            "EGR and/or VVT System",
            "PM filter monitoring",
            "Exhaust Gas Sensor",
            "Reserved",
            "Boost Pressure",
            "Reserved",
            "NOx/SCR Monitor",
            "NMHC Catalyst",
        ]
    };
    for (index, name) in names.into_iter().enumerate() {
        let bit = 1 << (7 - index);
        let available = name != "Reserved" && c & bit != 0;
        tests[index + 3] = Test::new(name, available, available && d & bit == 0);
    }
    tests
}

impl OBD {
    /// Get the DTC that caused the freeze frame.
    pub fn get_freeze_frame_dtc(&mut self) -> Vec<TroubleCode> {
        if let Err(err) = self.send_command(&mut Command::new_pid(b"0102")) {
            println!("when getting dtc3: {err}");
            return Vec::new();
        }

        let mut raw_response = self.read_iso_tp_response().join("");
        OBD::decode_trouble_codes(&mut raw_response)
    }

    pub fn get_permanant_trouble_codes(&mut self) -> Vec<TroubleCode> {
        if let Err(err) = self.send_command(&mut Command::new_svc(b"0A")) {
            println!("when getting dtc6: {err}");
            return Vec::new();
        }

        let mut raw_response = self.read_iso_tp_response().join("");
        OBD::decode_trouble_codes(&mut raw_response)
    }

    pub fn clear_trouble_codes(&mut self) -> Result<(), Error> {
        let response = self.query(Command::new_svc(b"04"));
        let raw = response.formatted_response.unwrap_or_default();

        // positive response
        if raw == "44" {
            Ok(())
        } else {
            Err(Error::DTCClearFailed(raw))
        }
    }

    /// Read monitor status from a single, unambiguous Mode 01 PID 01 response.
    /// Unsupported adapter layouts or multiple ECU replies remain unknown.
    pub fn get_readiness_status(&mut self) -> [Test; 11] {
        let mut request = Command::new_pid(b"0101");
        let response = if self.replay_requests {
            self.get_recorded_response(&request)
        } else {
            if self.send_command(&mut request).is_err() {
                return [Test::no_data(); 11];
            }
            match self.get_at_response() {
                Ok(response) => response,
                Err(_) => return [Test::no_data(); 11],
            }
        };

        if self.record_requests {
            self.save_request(&request, &response);
        }

        response
            .raw_response()
            .as_deref()
            .and_then(decode_readiness_reply)
            .unwrap_or([Test::no_data(); 11])
    }

    pub fn get_common_tests_status(&mut self) -> [Test; 3] {
        let tests = self.get_readiness_status();
        [tests[0], tests[1], tests[2]]
    }

    pub fn get_advanced_tests_status(&mut self) -> [Test; 8] {
        let tests = self.get_readiness_status();
        std::array::from_fn(|index| tests[index + 3])
    }

    pub fn get_test_results(&mut self) {
        // TODO. Test and get a mode 06 response
        if let Err(err) = self.send_command(&mut Command::new_svc(b"06")) {
            println!("when sending test results. mode 06: {err}");
        }

        match self.read_until(b'>') {
            Ok(raw_response) => {
                println!("< {}", raw_response.escape_default());
            }
            Err(err) => {
                println!("when getting test results. mode 06: {err}");
            }
        }
    }

    pub fn has_check_engine_light(&mut self) -> bool {
        let response = self.query(Command::new_pid(b"0101"));
        (response.a_value() as u32 & 0x80) != 0
    }

    pub fn warm_ups_since_codes_cleared(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0130"))
            .map_no_data(|r| Scalar::new(r.a_value(), Unit::NoData, Some(self.unit_preferences)))
    }

    pub fn distance_traveled_since_codes_cleared(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0131")).map_no_data(|r| {
            Scalar::new(
                (256.0 * r.a_value()) + r.b_value(),
                Unit::Kilometers,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn get_num_trouble_codes(&mut self) -> u32 {
        let response = self.query(Command::new_pid(b"0101"));
        response.a_value() as u32 & 0x7F
    }

    // Check on this. Might be broken when there are more than 3 DTC's
    pub fn get_trouble_codes(&mut self) -> Vec<TroubleCode> {
        let n_dtcs = self.get_num_trouble_codes();
        if n_dtcs == 0 {
            // no trouble codes
            return Vec::new();
        }

        if let Err(err) = self.send_command(&mut Command::new_svc(b"03")) {
            println!("when getting dtc1: {err}");
            return Vec::new();
        }

        let mut raw_response = self.read_iso_tp_response().join("");
        OBD::decode_trouble_codes(&mut raw_response)
    }

    pub fn decode_trouble_codes(response: &mut String) -> Vec<TroubleCode> {
        println!(
            "==> Decoding trouble codes. Raw response: '{}'",
            response.escape_default()
        );
        let ecu_names = OBD::extract_ecu_names(response);
        println!(" -> ECU names extracted: '{:?}'", ecu_names);
        OBD::strip_ecu_names(response, &ecu_names);

        let binding = response.replace("\r", "").replace(" ", "");
        println!(" -> ECU names stripped, escaped, no spaces: '{binding}'");

        let (sanitized, permanant) = {
            if binding.contains("43") {
                (
                    binding
                        .split("43")
                        .filter_map(|chunk| {
                            if chunk.len() >= 4 {
                                Some(chunk.trim_end_matches("00"))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                    false,
                )
            } else if binding.contains("4A") {
                (
                    binding
                        .split("4A")
                        .filter_map(|chunk| {
                            if chunk.len() >= 4 {
                                Some(chunk.trim_end_matches("00"))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                    true,
                )
            } else {
                return Vec::new();
            }
        };

        println!("dbg: dtc response from ecu: {sanitized}");

        if sanitized.to_lowercase().contains("nodata") {
            return Vec::new();
        }

        let mut codes = Vec::new();

        for chunk in sanitized.as_bytes().chunks(4) {
            if chunk.len() != 4 {
                break;
            }

            let as_string = String::from_utf8_lossy(chunk);
            let a = as_string.chars().nth(0).unwrap_or('\0');
            let b = as_string.chars().nth(1).unwrap_or('\0');
            let c = as_string.chars().nth(2).unwrap_or('\0');
            let d = as_string.chars().nth(3).unwrap_or('\0');
            let left = u8::from_str_radix(&format!("{}{}", a, b), 16).unwrap_or_default();
            let right = u8::from_str_radix(&format!("{}{}", c, d), 16).unwrap_or_default();

            if left == 0x00 && right == 0x00 || (chunk == b"0000") {
                break;
            }

            //println!("\nchunk: {:?}, {:?}. a: {a}, b: {b}, c: {c}, d: {d}", as_string, chunk);
            let bit_7 = (left & 0b1000_0000) >> 7;
            let bit_6 = (left & 0b0100_0000) >> 6;
            let bit_5 = (left & 0b0010_0000) >> 5;
            let bit_4 = (left & 0b0001_0000) >> 4;
            let c2 = (bit_5 << 1) | bit_4;
            let new_left = left & 0b00001111;
            // println!("left: {left:b}, right: {right:b}");
            // println!("bit 7: {bit_7:b}, bit 6: {bit_6:b}, bit 5: {bit_5:b}, bit 4: {bit_4:b}");
            // println!("C2: {c2:b}");

            let category = match (bit_7, bit_6) {
                (0, 0) => TroubleCodeCategory::Powertrain,
                (0, 1) => TroubleCodeCategory::Chassis,
                (1, 0) => TroubleCodeCategory::Body,
                (1, 1) => TroubleCodeCategory::Network,
                _ => TroubleCodeCategory::Unknown,
            };

            let dtc_code = format!(
                "{}{:01X}{:01X}{:02X}",
                category.system_letter(),
                c2,
                new_left,
                right
            );

            codes.push(TroubleCode::new(category, dtc_code, permanant));
        }

        codes
    }

    pub fn obd_standards(&mut self) -> OBDStandard {
        let response = self.query(Command::new_pid(b"011C"));
        OBDStandard::from_u8(response.a_value() as u8)
    }

    pub fn aux_input_status(&mut self) -> AuxiliaryInputStatus {
        let response = self.query(Command::new_pid(b"011E"));
        let in_use = (response.a_value() as u32 & 1) != 0;

        if in_use {
            AuxiliaryInputStatus::InUse
        } else {
            AuxiliaryInputStatus::NotInUse
        }
    }

    pub fn distance_traveled_with_mil(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0121")).map_no_data(|r| {
            Scalar::new(
                (256.0 * r.a_value()) + r.b_value(),
                Unit::Kilometers,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn time_run_with_mil(&mut self) -> Scalar {
        self.query(Command::new_pid(b"014D")).map_no_data(|r| {
            Scalar::new(
                (256.0 * r.a_value()) + r.b_value(),
                Unit::Minutes,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn control_module_voltage(&mut self) -> Scalar {
        self.query(Command::new_pid(b"0142")).map_no_data(|r| {
            Scalar::new(
                ((256.0 * r.a_value()) + r.b_value()) / 1000.0,
                Unit::Volts,
                Some(self.unit_preferences),
            )
        })
    }

    pub fn time_since_codes_cleared(&mut self) -> Scalar {
        self.query(Command::new_pid(b"014E")).map_no_data(|r| {
            Scalar::new(
                (256.0 * r.a_value()) + r.b_value(),
                Unit::Minutes,
                Some(self.unit_preferences),
            )
        })
    }
}

#[cfg(test)]
mod readiness_decoder_tests {
    use super::*;

    #[test]
    fn common_monitors_use_b_not_mil_or_dtc_count() {
        let tests = decode_readiness_bytes([0xff, 0x27, 0, 0]);
        assert!(tests[0].available && tests[0].complete);
        assert!(tests[1].available && !tests[1].complete);
        assert!(tests[2].available && tests[2].complete);
        let changed = decode_readiness_bytes([0, 0x27, 0, 0]);
        for (left, right) in tests.iter().zip(changed) {
            assert_eq!(
                (left.available, left.complete),
                (right.available, right.complete)
            );
        }
    }

    #[test]
    fn unsupported_is_never_complete() {
        let tests = decode_readiness_bytes([0xff, 0, 0, 0]);
        assert!(tests.iter().all(|test| !test.available && !test.complete));
    }

    #[test]
    fn ignition_flag_and_completion_share_one_snapshot() {
        let spark = decode_readiness_bytes([0, 0, 0x81, 0x01]);
        assert_eq!(spark[10].name, "Catalyst");
        assert!(spark[3].complete && !spark[10].complete);
        let diesel = decode_readiness_bytes([0, 8, 0xff, 0]);
        assert_eq!(diesel[10].name, "NMHC Catalyst");
        assert!(diesel[10].complete);
        for index in [6, 8] {
            assert_eq!(diesel[index].name, "Reserved");
            assert!(!diesel[index].available && !diesel[index].complete);
        }
    }

    #[test]
    fn exact_can_response_decodes() {
        for id in ["7E8", "18DAF110"] {
            let raw = format!("{id} 06 41 01 80 27 81 01\r\n");
            let tests = decode_readiness_reply(&raw).unwrap();
            assert!(tests[0].complete && !tests[1].complete);
            assert!(tests[3].complete && !tests[10].complete);
        }
    }

    #[test]
    fn malformed_wrong_service_and_unsupported_layouts_are_unknown() {
        for raw in [
            "",
            "NO DATA",
            "7E8 06 42 01 00 07 00 00",
            "7E8 06 41 02 00 07 00 00",
            "7E8 05 41 01 00 07 00",
            "7E8 06 41 01 00 07 ZZ 00",
            "7E8 06 41 01 00 +7 00 00",
            "7E8 06 41 01 00 07 00 00 00",
            "FFF 06 41 01 00 07 00 00",
            "41 01 00 07 00 00",
        ] {
            assert!(decode_readiness_reply(raw).is_none(), "{raw}");
        }
    }

    #[test]
    fn multiple_ecus_cannot_be_silently_combined() {
        assert!(
            decode_readiness_reply("7E8 06 41 01 00 07 00 00\r\n7E9 06 41 01 00 00 00 00")
                .is_none()
        );
    }
}

#[cfg(test)]
mod can_dtc_payload_tests {
    use super::*;

    #[test]
    fn decodes_reported_p143c_without_consuming_count_as_code() {
        let codes = decode_can_dtc_payload(&[0x43, 1, 0x14, 0x3C]).unwrap();
        assert_eq!(
            codes,
            vec![DecodedDtc {
                code: "P143C".into(),
                status: DtcStatus::Confirmed,
            }]
        );
    }

    #[test]
    fn preserves_service_like_bytes_inside_codes_and_zero_low_byte() {
        let codes = decode_can_dtc_payload(&[0x43, 3, 0x43, 0x4A, 0x14, 0x00, 0x47, 0x43]).unwrap();
        let names: Vec<_> = codes.iter().map(|code| code.code.as_str()).collect();
        assert_eq!(names, ["C034A", "P1400", "C0743"]);
        assert!(codes.iter().all(|code| code.status == DtcStatus::Confirmed));
    }

    #[test]
    fn distinguishes_all_three_services_for_the_same_code() {
        for (service, status) in [
            (0x43, DtcStatus::Confirmed),
            (0x47, DtcStatus::Pending),
            (0x4A, DtcStatus::Permanent),
        ] {
            let codes = decode_can_dtc_payload(&[service, 1, 0x14, 0x3C]).unwrap();
            assert_eq!(codes[0].code, "P143C");
            assert_eq!(codes[0].status, status);
        }
    }

    #[test]
    fn decodes_all_four_system_categories() {
        let codes =
            decode_can_dtc_payload(&[0x43, 4, 0x30, 0x01, 0x70, 0x01, 0xB0, 0x01, 0xF0, 0x01])
                .unwrap();
        let names: Vec<_> = codes.iter().map(|code| code.code.as_str()).collect();
        assert_eq!(names, ["P3001", "C3001", "B3001", "U3001"]);
    }

    #[test]
    fn accepts_explicit_zero_count() {
        assert_eq!(decode_can_dtc_payload(&[0x43, 0]), Ok(Vec::new()));
    }

    #[test]
    fn rejects_missing_or_unsupported_header() {
        assert_eq!(
            decode_can_dtc_payload(&[]),
            Err(DtcPayloadError::MissingHeader)
        );
        assert_eq!(
            decode_can_dtc_payload(&[0x43]),
            Err(DtcPayloadError::MissingHeader)
        );
        assert_eq!(
            decode_can_dtc_payload(&[0x7F, 0x03, 0x11]),
            Err(DtcPayloadError::UnsupportedService(0x7F))
        );
    }

    #[test]
    fn rejects_truncation_padding_and_concatenated_messages() {
        for payload in [
            &[0x43, 2, 0x14, 0x3C][..],
            &[0x43, 1, 0x14][..],
            &[0x43, 1, 0x14, 0x3C, 0, 0][..],
            &[0x43, 0, 0x4A, 0][..],
        ] {
            assert_eq!(
                decode_can_dtc_payload(payload),
                Err(DtcPayloadError::InvalidLength)
            );
        }
    }

    #[test]
    fn rejects_zero_code_inside_declared_count_without_partial_success() {
        assert_eq!(
            decode_can_dtc_payload(&[0x43, 2, 0x14, 0x3C, 0, 0]),
            Err(DtcPayloadError::InvalidZeroCode)
        );
    }
}
