// Offline validation of staged reference data; never dispatch vehicle requests.
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_DBC_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Deserialize)]
struct Catalog<T> {
    schema_version: u32,
    status: String,
    entries: Vec<T>,
}

#[derive(Deserialize)]
struct ObdEntry {
    request_hex: String,
    name: String,
    description: String,
    upstream_response_length_hint: Option<u32>,
}

#[derive(Deserialize)]
struct DtcEntry {
    code: String,
    descriptions: Vec<String>,
}

#[derive(Deserialize)]
struct NrcEntry {
    code_hex: String,
    labels: Vec<NrcLabel>,
}

#[derive(Deserialize)]
struct NrcLabel {
    name: String,
    context: String,
}

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    status: String,
    sources: Vec<Source>,
}

#[derive(Deserialize)]
struct Source {
    id: String,
    revision: String,
    #[serde(default)]
    files: Vec<SourceFile>,
    derived_path: Option<String>,
    license_path: Option<String>,
}

#[derive(Deserialize)]
struct SourceFile {
    path: String,
    git_blob_sha: String,
    bytes: u64,
    role: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/catalogs")
}

fn uppercase_hex(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
}

fn valid_obd_request(value: &str) -> bool {
    uppercase_hex(value)
        && match value.as_bytes() {
            [b'0', b'3' | b'7'] => true,
            [b'0', b'1' | b'6' | b'9', _, _] => true,
            _ => false,
        }
}

fn valid_dtc_code(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 5
        && matches!(bytes[0], b'P' | b'C' | b'B' | b'U')
        && (b'0'..=b'3').contains(&bytes[1])
        && bytes[2..]
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(byte))
}

fn catalog_path(relative: &str) -> Option<PathBuf> {
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || relative.contains(':')
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(root().join(path))
}

fn load<T: serde::de::DeserializeOwned>(relative: &str) -> T {
    let path = catalog_path(relative).expect("valid catalog path");
    let metadata = fs::metadata(&path).expect("catalog exists");
    assert!(metadata.is_file());
    assert!(metadata.len() <= MAX_JSON_BYTES as u64);
    let text = fs::read_to_string(path).expect("UTF-8 catalog");
    assert!(text.len() <= MAX_JSON_BYTES);
    serde_json::from_str(&text).expect("valid catalog JSON")
}

fn assert_header<T>(catalog: &Catalog<T>) {
    assert_eq!(catalog.schema_version, 1);
    assert_eq!(catalog.status, "reference_only");
    assert!(!catalog.entries.is_empty());
}

#[test]
fn obd_reference_catalog_contains_unique_read_requests() {
    let catalog: Catalog<ObdEntry> = load("obd2/read_catalog.json");
    assert_header(&catalog);
    let mut seen = HashSet::new();
    for entry in &catalog.entries {
        assert!(valid_obd_request(&entry.request_hex));
        assert!(seen.insert(&entry.request_hex));
        assert!(!entry.name.trim().is_empty());
        assert!(!entry.description.trim().is_empty());
        if let Some(length) = entry.upstream_response_length_hint {
            assert!(length > 0);
        }
    }
    assert_eq!(seen.len(), 198);
}

#[test]
fn dtc_reference_catalog_contains_unique_valid_codes() {
    let catalog: Catalog<DtcEntry> = load("dtc/reference_catalog.json");
    assert_header(&catalog);
    let mut seen = HashSet::new();
    for entry in &catalog.entries {
        assert!(valid_dtc_code(&entry.code));
        assert!(seen.insert(&entry.code));
        assert!(!entry.descriptions.is_empty());
        let descriptions: HashSet<_> = entry.descriptions.iter().collect();
        assert_eq!(descriptions.len(), entry.descriptions.len());
        assert!(descriptions.iter().all(|text| !text.trim().is_empty()));
    }
    assert_eq!(seen.len(), 2066);
}

#[test]
fn nrc_reference_catalog_retains_context_and_excludes_success() {
    let catalog: Catalog<NrcEntry> = load("uds/negative_response_catalog.json");
    assert_header(&catalog);
    let mut seen = HashSet::new();
    for entry in &catalog.entries {
        assert_eq!(entry.code_hex.len(), 2);
        assert!(uppercase_hex(&entry.code_hex));
        assert_ne!(entry.code_hex, "00");
        assert!(seen.insert(&entry.code_hex));
        assert!(!entry.labels.is_empty());
        let mut labels = HashSet::new();
        for label in &entry.labels {
            assert!(!label.name.trim().is_empty());
            assert!(matches!(
                label.context.as_str(),
                "uds_reference" | "security_transport_reference"
            ));
            assert!(labels.insert((&label.name, &label.context)));
        }
    }
    assert_eq!(seen.len(), 65);
    assert!(seen.iter().any(|code| code.as_str() == "78"));
}

#[test]
fn manifest_accounts_for_all_staged_dbc_files() {
    let manifest: Manifest = load("manifest.json");
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.status, "staged_not_enabled");
    let mut sources = HashSet::new();
    let mut paths = HashSet::new();
    let mut dbc_paths = HashSet::new();
    for source in &manifest.sources {
        assert!(sources.insert(&source.id));
        assert_eq!(source.revision.len(), 40);
        assert!(source.revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
        for file in &source.files {
            assert!(paths.insert(&file.path));
            let path = catalog_path(&file.path).expect("relative manifest path");
            assert!(path.is_file());
            assert_eq!(fs::metadata(&path).expect("metadata").len(), file.bytes);
            assert_eq!(file.git_blob_sha.len(), 40);
            assert!(file.git_blob_sha.bytes().all(|byte| byte.is_ascii_hexdigit()));
            if file.role == "passive_reference" {
                assert!(file.path.starts_with("dbc/"));
                assert!(file.path.ends_with(".dbc"));
                assert!(file.bytes > 0 && file.bytes <= MAX_DBC_BYTES);
                let text = fs::read_to_string(path).expect("UTF-8 DBC");
                assert!(text.lines().any(|line| line.starts_with("BO_ ")));
                dbc_paths.insert(file.path.clone());
            }
        }
        for relative in [&source.derived_path, &source.license_path]
            .into_iter()
            .flatten()
        {
            assert!(catalog_path(relative).expect("valid path").is_file());
        }
    }
    let actual: HashSet<_> = fs::read_dir(root().join("dbc"))
        .expect("DBC directory")
        .map(|entry| {
            let entry = entry.expect("directory entry");
            assert!(entry.file_type().expect("file type").is_file());
            format!("dbc/{}", entry.file_name().to_str().expect("UTF-8 path"))
        })
        .collect();
    assert_eq!(actual, dbc_paths);
    assert_eq!(actual.len(), 58);
}

#[test]
fn rejects_unsafe_or_malformed_reference_keys() {
    for value in ["04", "0801", "010", "01GG", "010C00", "01ff", ""] {
        assert!(!valid_obd_request(value));
    }
    for value in ["P4000", "X0001", "P00GG", "P000", "p0001", "P0é1"] {
        assert!(!valid_dtc_code(value));
    }
    for value in ["../outside", "/absolute", "dbc/../x", "C:/file", "a\\b", ""] {
        assert!(catalog_path(value).is_none());
    }
}
