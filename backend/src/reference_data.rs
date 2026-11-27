//! Offline reference lookups and passive signal decoding.
//! Reference data is not an ECU capability claim or authorization policy.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_CATALOG_BYTES: usize = 2 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;

#[derive(Debug, thiserror::Error)]
pub enum ReferenceError {
    #[error("catalog exceeds the size limit")]
    TooLarge,
    #[error("invalid catalog JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported catalog version or status")]
    Header,
    #[error("invalid or duplicate reference entry: {0}")]
    Entry(String),
    #[error("invalid signal layout")]
    Layout,
    #[error("unsupported DBC definition: {0}")]
    Dbc(String),
    #[error("signal is inactive for the observed multiplexer value")]
    InactiveSignal,
    #[error("overlapping simultaneously active DBC signals")]
    SignalOverlap,
    #[error("signal exceeds frame payload")]
    Truncated,
    #[error("non-finite scaling or result")]
    Scaling,
}

#[derive(Deserialize)]
struct Catalog<T> {
    schema_version: u32,
    status: String,
    entries: Vec<T>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObdReference {
    pub request_hex: String,
    pub name: String,
    pub description: String,
    pub upstream_response_length_hint: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DtcReference {
    pub code: String,
    pub descriptions: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NrcLabel {
    pub name: String,
    pub context: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NrcReference {
    pub code_hex: String,
    pub labels: Vec<NrcLabel>,
}

pub struct ReferenceCatalogs {
    obd: BTreeMap<String, ObdReference>,
    dtc: BTreeMap<String, DtcReference>,
    nrc: BTreeMap<String, NrcReference>,
}

fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<Vec<T>, ReferenceError> {
    if text.len() > MAX_CATALOG_BYTES {
        return Err(ReferenceError::TooLarge);
    }
    let catalog: Catalog<T> = serde_json::from_str(text)?;
    if catalog.schema_version != 1 || catalog.status != "reference_only" {
        return Err(ReferenceError::Header);
    }
    if catalog.entries.is_empty() || catalog.entries.len() > MAX_ENTRIES {
        return Err(ReferenceError::Entry("entry count".into()));
    }
    Ok(catalog.entries)
}

fn hex(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

fn valid_obd(key: &str) -> bool {
    hex(key)
        && matches!(
            key.as_bytes(),
            [b'0', b'3' | b'7'] | [b'0', b'1' | b'6' | b'9', _, _]
        )
}

fn valid_dtc(key: &str) -> bool {
    let bytes = key.as_bytes();
    bytes.len() == 5
        && matches!(bytes[0], b'P' | b'C' | b'B' | b'U')
        && (b'0'..=b'3').contains(&bytes[1])
        && bytes[2..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(b))
}

impl ReferenceCatalogs {
    pub fn bundled() -> Result<Self, ReferenceError> {
        Self::from_json(
            include_str!("../data/catalogs/obd2/read_catalog.json"),
            include_str!("../data/catalogs/dtc/reference_catalog.json"),
            include_str!("../data/catalogs/uds/negative_response_catalog.json"),
        )
    }

    pub fn from_json(obd: &str, dtc: &str, nrc: &str) -> Result<Self, ReferenceError> {
        let mut result = Self {
            obd: BTreeMap::new(),
            dtc: BTreeMap::new(),
            nrc: BTreeMap::new(),
        };
        for entry in parse::<ObdReference>(obd)? {
            let key = entry.request_hex.clone();
            if !valid_obd(&key)
                || entry.name.trim().is_empty()
                || entry.description.trim().is_empty()
                || entry.upstream_response_length_hint == Some(0)
                || result.obd.insert(key.clone(), entry).is_some()
            {
                return Err(ReferenceError::Entry(key));
            }
        }
        for entry in parse::<DtcReference>(dtc)? {
            let key = entry.code.clone();
            if !valid_dtc(&key)
                || entry.descriptions.is_empty()
                || entry.descriptions.iter().any(|s| s.trim().is_empty())
                || entry.descriptions.iter().collect::<BTreeSet<_>>().len()
                    != entry.descriptions.len()
                || result.dtc.insert(key.clone(), entry).is_some()
            {
                return Err(ReferenceError::Entry(key));
            }
        }
        for entry in parse::<NrcReference>(nrc)? {
            let key = entry.code_hex.clone();
            if key.len() != 2
                || !hex(&key)
                || key == "00"
                || entry.labels.is_empty()
                || entry.labels.iter().any(|label| {
                    label.name.trim().is_empty()
                        || !matches!(
                            label.context.as_str(),
                            "uds_reference" | "security_transport_reference"
                        )
                })
                || entry
                    .labels
                    .iter()
                    .map(|label| (&label.name, &label.context))
                    .collect::<BTreeSet<_>>()
                    .len()
                    != entry.labels.len()
                || result.nrc.insert(key.clone(), entry).is_some()
            {
                return Err(ReferenceError::Entry(key));
            }
        }
        Ok(result)
    }

    pub fn obd(&self, request: &str) -> Option<&ObdReference> {
        if !matches!(request.len(), 2 | 4) || !request.is_ascii() {
            return None;
        }
        self.obd.get(&request.to_ascii_uppercase())
    }

    pub fn dtc(&self, code: &str) -> Option<&DtcReference> {
        if code.len() != 5 || !code.is_ascii() {
            return None;
        }
        self.dtc.get(&code.to_ascii_uppercase())
    }

    pub fn nrc(&self, code: u8) -> Option<&NrcReference> {
        self.nrc.get(&format!("{code:02X}"))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BitOrder {
    Intel,
    Motorola,
}

#[derive(Debug, Serialize)]
pub struct DecodedSignal {
    pub raw: i128,
    pub scaled: f64,
}

fn next_signal_bit(bit: usize, order: BitOrder) -> usize {
    match order {
        BitOrder::Intel => bit + 1,
        BitOrder::Motorola if bit.is_multiple_of(8) => bit + 15,
        BitOrder::Motorola => bit - 1,
    }
}

/// DBC start-bit semantics: Intel starts at LSB; Motorola at MSB (sawtooth).
/// No frame transmission, multiplex selection or checksum inference.
pub fn decode_signal(
    payload: &[u8],
    start: usize,
    width: u8,
    order: BitOrder,
    signed: bool,
    factor: f64,
    offset: f64,
) -> Result<DecodedSignal, ReferenceError> {
    if payload.len() > 64 || !(1..=64).contains(&width) || start >= 512 {
        return Err(ReferenceError::Layout);
    }
    if !factor.is_finite() || !offset.is_finite() {
        return Err(ReferenceError::Scaling);
    }
    let mut raw = 0u64;
    let mut bit = start;
    for index in 0..width {
        let byte = payload.get(bit / 8).ok_or(ReferenceError::Truncated)?;
        let value = u64::from((byte >> (bit % 8)) & 1);
        match order {
            BitOrder::Intel => raw |= value << index,
            BitOrder::Motorola => raw = (raw << 1) | value,
        }
        if index + 1 < width {
            bit = next_signal_bit(bit, order);
        }
    }
    let raw = if signed && raw & (1u64 << (width - 1)) != 0 {
        i128::from(raw) - (1i128 << width)
    } else {
        i128::from(raw)
    };
    let scaled = (raw as f64) * factor + offset;
    if !scaled.is_finite() {
        return Err(ReferenceError::Scaling);
    }
    Ok(DecodedSignal { raw, scaled })
}

#[derive(Deserialize)]
struct TranslationPreview {
    schema_version: u32,
    status: String,
    locale: String,
    messages: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct PreviewMessage {
    pub locale: String,
    pub key: String,
    pub text: String,
    pub fallback: bool,
}

/// Read the explicitly staged, unreviewed preview strings only.
pub fn translation_preview(
    locale: &str,
    key: &str,
) -> Result<Option<PreviewMessage>, ReferenceError> {
    let english = include_str!("../data/catalogs/translations/en.json");
    let source = match locale {
        "cs-CZ" => Some(include_str!("../data/catalogs/translations/cs-CZ.json")),
        "de" => Some(include_str!("../data/catalogs/translations/de.json")),
        "es" => Some(include_str!("../data/catalogs/translations/es.json")),
        "fr" => Some(include_str!("../data/catalogs/translations/fr.json")),
        "hu" => Some(include_str!("../data/catalogs/translations/hu.json")),
        "it" => Some(include_str!("../data/catalogs/translations/it.json")),
        "nl" => Some(include_str!("../data/catalogs/translations/nl.json")),
        "pl" => Some(include_str!("../data/catalogs/translations/pl.json")),
        "pt-PT" => Some(include_str!("../data/catalogs/translations/pt-PT.json")),
        "ro" => Some(include_str!("../data/catalogs/translations/ro.json")),
        "ru" => Some(include_str!("../data/catalogs/translations/ru.json")),
        "sr" => Some(include_str!("../data/catalogs/translations/sr.json")),
        "tr" => Some(include_str!("../data/catalogs/translations/tr.json")),
        "uk-UA" => Some(include_str!("../data/catalogs/translations/uk-UA.json")),
        "zh-CN" => Some(include_str!("../data/catalogs/translations/zh-CN.json")),
        "en" => Some(english),
        _ => None,
    };
    let parse_preview =
        |text: &str, expected: &str| -> Result<TranslationPreview, ReferenceError> {
            if text.len() > MAX_CATALOG_BYTES {
                return Err(ReferenceError::TooLarge);
            }
            let preview: TranslationPreview = serde_json::from_str(text)?;
            if preview.schema_version != 1
                || preview.status != "draft_reference"
                || preview.locale != expected
            {
                return Err(ReferenceError::Header);
            }
            if preview.messages.values().any(|message| {
                message.trim().is_empty()
                    || message
                        .chars()
                        .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | '{' | '}' | '%'))
            }) {
                return Err(ReferenceError::Entry("invalid preview text".into()));
            }
            Ok(preview)
        };
    let fallback_catalog = parse_preview(english, "en")?;
    let Some(default_text) = fallback_catalog.messages.get(key) else {
        return Ok(None);
    };
    if let Some(source) = source {
        let preview = parse_preview(source, locale)?;
        if let Some(text) = preview.messages.get(key) {
            return Ok(Some(PreviewMessage {
                locale: preview.locale,
                key: key.to_owned(),
                text: text.clone(),
                fallback: false,
            }));
        }
    }
    Ok(Some(PreviewMessage {
        locale: "en".to_owned(),
        key: key.to_owned(),
        text: default_text.clone(),
        fallback: true,
    }))
}

/// DBC wire identity: standard 11-bit IDs are unflagged; extended 29-bit
/// IDs carry bit 31. Bits 29 and 30 are reserved, not part of the CAN ID.
fn valid_dbc_id(id: u32) -> bool {
    id <= 0x7ff || (id & 0xe000_0000) == 0x8000_0000
}

#[derive(Clone, Copy)]
enum Multiplex {
    None,
    Selector,
    Branch(u64),
}

struct DbcSignal {
    start: usize,
    width: u8,
    order: BitOrder,
    signed: bool,
    factor: f64,
    offset: f64,
    multiplex: Multiplex,
}

type BitMask = [u64; 8];

fn masks_overlap(left: &BitMask, right: &BitMask) -> bool {
    left.iter().zip(right).any(|(a, b)| a & b != 0)
}

fn merge_mask(target: &mut BitMask, source: &BitMask) {
    for (word, bits) in target.iter_mut().zip(source) {
        *word |= bits;
    }
}

#[derive(Default)]
struct SignalOccupancy {
    common: BitMask,
    all_branches: BitMask,
    branches: BTreeMap<u64, BitMask>,
}

impl SignalOccupancy {
    fn add(&mut self, signal: &DbcSignal, size: usize) -> Result<(), ReferenceError> {
        let mask = signal.bit_mask(size)?;
        match signal.multiplex {
            Multiplex::Branch(value) => {
                let branch = self.branches.entry(value).or_default();
                if masks_overlap(&self.common, &mask) || masks_overlap(branch, &mask) {
                    return Err(ReferenceError::SignalOverlap);
                }
                merge_mask(branch, &mask);
                merge_mask(&mut self.all_branches, &mask);
            }
            Multiplex::None | Multiplex::Selector => {
                if masks_overlap(&self.common, &mask) || masks_overlap(&self.all_branches, &mask) {
                    return Err(ReferenceError::SignalOverlap);
                }
                merge_mask(&mut self.common, &mask);
            }
        }
        Ok(())
    }
}

impl DbcSignal {
    fn bit_mask(&self, size: usize) -> Result<BitMask, ReferenceError> {
        if !(1..=64).contains(&self.width) || self.start >= 512 || size > 64 {
            return Err(ReferenceError::Layout);
        }
        if !self.factor.is_finite() || !self.offset.is_finite() {
            return Err(ReferenceError::Scaling);
        }
        let mut mask = [0; 8];
        let mut bit = self.start;
        for index in 0..self.width {
            if bit >= size * 8 {
                return Err(ReferenceError::Truncated);
            }
            mask[bit / 64] |= 1u64 << (bit % 64);
            if index + 1 < self.width {
                bit = next_signal_bit(bit, self.order);
            }
        }
        Ok(mask)
    }

    fn decode(&self, payload: &[u8]) -> Result<DecodedSignal, ReferenceError> {
        decode_signal(
            payload,
            self.start,
            self.width,
            self.order,
            self.signed,
            self.factor,
            self.offset,
        )
    }
}

/// A validated, reusable message definition. No vehicle or bus is opened.
pub struct DbcMessage {
    size: usize,
    signals: BTreeMap<String, DbcSignal>,
    selector: Option<(usize, u8, BitOrder)>,
}

impl DbcMessage {
    /// Decode only an explicitly named signal from an exact-length payload.
    pub fn decode(
        &self,
        signal_name: &str,
        payload: &[u8],
    ) -> Result<DecodedSignal, ReferenceError> {
        if self.size != payload.len() {
            return Err(ReferenceError::Truncated);
        }
        let definition = self
            .signals
            .get(signal_name)
            .ok_or_else(|| ReferenceError::Dbc("signal not found".into()))?;
        if let Multiplex::Branch(value) = definition.multiplex {
            let (start, width, order) = self
                .selector
                .ok_or_else(|| ReferenceError::Dbc("missing multiplexer".into()))?;
            let raw = decode_signal(payload, start, width, order, false, 1.0, 0.0)?.raw;
            if raw != i128::from(value) {
                return Err(ReferenceError::InactiveSignal);
            }
        }
        definition.decode(payload)
    }
}

// Scan the quoted body of a CM_ statement, not its prefix. A semicolon
// inside the string is text; escaped quotes do not close the comment.
fn dbc_comment_closed(body: &str) -> Result<bool, ReferenceError> {
    let mut escaped = false;
    for (index, byte) in body.bytes().enumerate() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            if body[index + 1..].trim() != ";" {
                return Err(ReferenceError::Dbc("invalid comment terminator".into()));
            }
            return Ok(true);
        }
    }
    if escaped {
        return Err(ReferenceError::Dbc("dangling comment escape".into()));
    }
    Ok(false)
}

/// Validate one explicitly selected DBC message, including simple multiplexing.
/// Multiple or nested multiplexers and extended multiplex metadata are rejected.
/// The caller supplies the encoded DBC ID, including bit 31 for extended IDs.
/// This is a strict subset, not a general DBC importer or vehicle capability check.
/// Metadata such as value labels, checksum rules and range constraints is not interpreted.
pub fn parse_dbc_message(dbc: &str, message_id: u32) -> Result<DbcMessage, ReferenceError> {
    // These are declarations in NS_, not actual metadata records.
    const NAMESPACE_SYMBOLS: &[&str] = &[
        "NS_DESC_",
        "CM_",
        "BA_DEF_",
        "BA_",
        "VAL_",
        "CAT_DEF_",
        "CAT_",
        "FILTER",
        "BA_DEF_DEF_",
        "EV_DATA_",
        "ENVVAR_DATA_",
        "SGTYPE_",
        "SGTYPE_VAL_",
        "BA_DEF_SGTYPE_",
        "BA_SGTYPE_",
        "SIG_TYPE_REF_",
        "VAL_TABLE_",
        "SIG_GROUP_",
        "SIG_VALTYPE_",
        "SIGTYPE_VALTYPE_",
        "BO_TX_BU_",
        "BA_DEF_REL_",
        "BA_REL_",
        "BA_DEF_DEF_REL_",
        "BU_SG_REL_",
        "BU_EV_REL_",
        "BU_BO_REL_",
        "SG_MUL_VAL_",
    ];
    if dbc.len() > MAX_CATALOG_BYTES {
        return Err(ReferenceError::TooLarge);
    }
    if !valid_dbc_id(message_id) {
        return Err(ReferenceError::Layout);
    }
    let message = regex::Regex::new(r"^BO_\s+([0-9]+)\s+\S+\s*:\s*([0-9]+)\s+\S+\s*$")
        .map_err(|error| ReferenceError::Dbc(error.to_string()))?;
    let signal = regex::Regex::new(
        r#"^SG_\s+([A-Za-z0-9_]+)(?:\s+(M|m[0-9]+))?\s*:\s*([0-9]+)\|([0-9]+)@([01])([+-])\s*\(\s*([^,]+),\s*([^\)]+)\)\s*\[[^\]]*\]\s*"[^"]*"\s+.+$"#,
    )
    .map_err(|error| ReferenceError::Dbc(error.to_string()))?;
    let mut selected = false;
    let mut length = None;
    let mut signals = BTreeMap::new();
    let mut selector = None;
    let mut branches = Vec::new();
    let mut occupancy = SignalOccupancy::default();
    let mut namespace_seen = false;
    let mut in_namespace = false;
    let mut seen_message = false;
    let mut in_comment = false;
    for line in dbc.lines().map(str::trim) {
        if in_comment {
            in_comment = !dbc_comment_closed(line)?;
            continue;
        }
        if line
            .strip_prefix("NS_")
            .is_some_and(|rest| rest.trim() == ":")
        {
            if namespace_seen || seen_message {
                return Err(ReferenceError::Dbc("misplaced namespace section".into()));
            }
            namespace_seen = true;
            in_namespace = true;
            continue;
        }
        if in_namespace {
            if line.is_empty() || NAMESPACE_SYMBOLS.contains(&line) {
                continue;
            }
            in_namespace = false;
        }
        let keyword = line.split_whitespace().next();
        if keyword == Some("CM_") {
            let (_, body) = line
                .split_once('"')
                .ok_or_else(|| ReferenceError::Dbc("missing comment string".into()))?;
            in_comment = !dbc_comment_closed(body)?;
            continue;
        }
        if keyword == Some("BO_") {
            seen_message = true;
            let fields = message
                .captures(line)
                .ok_or_else(|| ReferenceError::Dbc("invalid message header".into()))?;
            let id = fields[1]
                .parse::<u32>()
                .map_err(|_| ReferenceError::Layout)?;
            selected = id == message_id;
            if selected {
                let size = fields[2]
                    .parse::<usize>()
                    .map_err(|_| ReferenceError::Layout)?;
                if length.is_some() || size == 0 || size > 64 {
                    return Err(ReferenceError::Layout);
                }
                length = Some(size);
            }
        } else if selected && keyword == Some("SG_") {
            let fields = signal.captures(line).ok_or_else(|| {
                ReferenceError::Dbc("unsupported signal syntax or multiplexing".into())
            })?;
            let name = fields[1].to_owned();
            if name.len() > 128 || signals.len() >= 4096 {
                return Err(ReferenceError::Layout);
            }
            if signals.contains_key(&name) {
                return Err(ReferenceError::Dbc("duplicate signal".into()));
            }
            let multiplex = match fields.get(2).map(|value| value.as_str()) {
                None => Multiplex::None,
                Some("M") => Multiplex::Selector,
                Some(value) => {
                    Multiplex::Branch(value[1..].parse().map_err(|_| ReferenceError::Layout)?)
                }
            };
            let signal = DbcSignal {
                start: fields[3].parse().map_err(|_| ReferenceError::Layout)?,
                width: fields[4].parse().map_err(|_| ReferenceError::Layout)?,
                order: if &fields[5] == "1" {
                    BitOrder::Intel
                } else {
                    BitOrder::Motorola
                },
                signed: &fields[6] == "-",
                factor: fields[7]
                    .trim()
                    .parse()
                    .map_err(|_| ReferenceError::Scaling)?,
                offset: fields[8]
                    .trim()
                    .parse()
                    .map_err(|_| ReferenceError::Scaling)?,
                multiplex,
            };
            let size = length.ok_or(ReferenceError::Layout)?;
            occupancy.add(&signal, size)?;
            if matches!(multiplex, Multiplex::Selector) {
                if selector.is_some() || signal.signed || !(1..=64).contains(&signal.width) {
                    return Err(ReferenceError::Dbc("invalid or multiple selectors".into()));
                }
                selector = Some((signal.start, signal.width, signal.order));
            }
            if let Multiplex::Branch(value) = multiplex {
                branches.push(value);
            }
            signals.insert(name, signal);
        } else if matches!(keyword, Some("SIG_VALTYPE_" | "SG_MUL_VAL_")) {
            let id = line
                .split_whitespace()
                .nth(1)
                .and_then(|value| value.parse::<u32>().ok())
                .ok_or_else(|| ReferenceError::Dbc("invalid extended signal metadata".into()))?;
            if id == message_id {
                return Err(ReferenceError::Dbc("extended signal metadata".into()));
            }
        }
    }
    if in_comment {
        return Err(ReferenceError::Dbc("unterminated comment".into()));
    }
    let size = length.ok_or_else(|| ReferenceError::Dbc("message not found".into()))?;
    if let Some((_, width, _)) = selector {
        if branches
            .iter()
            .any(|value| u64::BITS - value.leading_zeros() > u32::from(width))
        {
            return Err(ReferenceError::Dbc("branch outside selector range".into()));
        }
    } else if !branches.is_empty() {
        return Err(ReferenceError::Dbc("missing multiplexer".into()));
    }
    Ok(DbcMessage {
        size,
        signals,
        selector,
    })
}

/// Convenience wrapper for a single offline observation.
pub fn decode_dbc_signal(
    dbc: &str,
    message_id: u32,
    signal_name: &str,
    payload: &[u8],
) -> Result<DecodedSignal, ReferenceError> {
    if signal_name.is_empty() || signal_name.len() > 128 {
        return Err(ReferenceError::Layout);
    }
    parse_dbc_message(dbc, message_id)?.decode(signal_name, payload)
}
