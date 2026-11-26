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
            bit = match order {
                BitOrder::Intel => bit + 1,
                BitOrder::Motorola if bit.is_multiple_of(8) => bit + 15,
                BitOrder::Motorola => bit - 1,
            };
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
