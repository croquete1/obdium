#[path = "../src/reference_data.rs"]
mod reference_data;

use reference_data::{
    decode_dbc_signal, decode_signal, translation_preview, BitOrder, ReferenceCatalogs,
    ReferenceError,
};

#[test]
fn bundled_catalogs_return_known_and_unknown_reference_values() {
    let catalogs = ReferenceCatalogs::bundled().expect("valid bundled data");
    assert_eq!(catalogs.obd("010c").expect("RPM").name, "RPM");
    assert!(catalogs.obd("04").is_none());
    assert!(catalogs.obd("0801").is_none());
    assert!(catalogs.dtc("p0001").expect("known DTC").descriptions[0].contains("Fuel"));
    assert!(catalogs.dtc("X0000").is_none());
    assert!(catalogs.obd(&"0".repeat(100_000)).is_none());
    assert!(catalogs.dtc(&"P".repeat(100_000)).is_none());
    assert!(catalogs.nrc(0).is_none());
    assert!(catalogs
        .nrc(0x78)
        .expect("pending")
        .labels
        .iter()
        .any(|label| label.name.contains("ResponsePending")));
}

#[test]
fn malformed_catalog_is_rejected_before_lookup() {
    let bad = r#"{"schema_version":2,"status":"reference_only","entries":[]}"#;
    assert!(matches!(
        ReferenceCatalogs::from_json(bad, "", ""),
        Err(ReferenceError::Header)
    ));
    let active = r#"{"schema_version":1,"status":"reference_only","entries":[{"request_hex":"04","name":"clear","description":"clear","upstream_response_length_hint":null}]}"#;
    assert!(matches!(
        ReferenceCatalogs::from_json(active, "", ""),
        Err(ReferenceError::Entry(_))
    ));
    let large = " ".repeat(2 * 1024 * 1024 + 1);
    assert!(matches!(
        ReferenceCatalogs::from_json(&large, "", ""),
        Err(ReferenceError::TooLarge)
    ));
}

#[test]
fn decodes_aligned_and_cross_byte_signals() {
    let little = decode_signal(&[0x34, 0x12], 0, 16, BitOrder::Intel, false, 0.5, -1.0)
        .expect("valid Intel");
    assert_eq!(little.raw, 0x1234);
    assert_eq!(little.scaled, 2329.0);
    let big = decode_signal(&[0x12, 0x34], 7, 16, BitOrder::Motorola, false, 1.0, 0.0)
        .expect("valid Motorola");
    assert_eq!(big.raw, 0x1234);
    let signed = decode_signal(&[0xf0, 0x0f], 4, 8, BitOrder::Intel, true, 1.0, 0.0)
        .expect("signed crossing");
    assert_eq!(signed.raw, -1);
    let cross = decode_signal(&[0x03, 0xa0], 1, 5, BitOrder::Motorola, false, 1.0, 0.0)
        .expect("Motorola crossing");
    assert_eq!(cross.raw, 29);
}

#[test]
fn rejects_invalid_layout_truncation_and_nonfinite_values() {
    assert!(matches!(
        decode_signal(&[0], 0, 0, BitOrder::Intel, false, 1.0, 0.0),
        Err(ReferenceError::Layout)
    ));
    assert!(matches!(
        decode_signal(&[0], 7, 2, BitOrder::Intel, false, 1.0, 0.0),
        Err(ReferenceError::Truncated)
    ));
    assert!(matches!(
        decode_signal(&[1], 0, 1, BitOrder::Intel, false, f64::NAN, 0.0),
        Err(ReferenceError::Scaling)
    ));
    let max =
        decode_signal(&[0xff; 8], 0, 64, BitOrder::Intel, true, 1.0, 0.0).expect("64-bit signed");
    assert_eq!(max.raw, -1);
}

#[test]
fn translation_preview_resolves_all_locales_and_falls_back() {
    let locales = [
        "en", "cs-CZ", "de", "es", "fr", "hu", "it", "nl", "pl", "pt-PT", "ro", "ru", "sr", "tr",
        "uk-UA", "zh-CN",
    ];
    let keys = [
        "confirm", "cancel", "error", "refresh", "stop", "close", "file", "exit", "help", "about",
        "no", "timeout",
    ];
    for locale in locales {
        for key in keys {
            let preview = translation_preview(locale, key)
                .expect("valid preview")
                .expect("known message");
            assert_eq!(preview.locale, locale);
            assert_eq!(preview.key, key);
            assert!(!preview.fallback);
            assert!(!preview.text.trim().is_empty());
        }
    }
    let portuguese = translation_preview("pt-PT", "file")
        .expect("valid preview")
        .expect("known message");
    assert_eq!(portuguese.text, "Ficheiro");
    let fallback = translation_preview("unknown", "file")
        .expect("valid fallback")
        .expect("known message");
    assert_eq!(fallback.locale, "en");
    assert!(fallback.fallback);
    assert!(translation_preview("en", "missing")
        .expect("valid preview")
        .is_none());
}

#[test]
fn rejects_duplicate_descriptions_and_labels_in_consumer() {
    let obd = include_str!("../data/catalogs/obd2/read_catalog.json");
    let dtc = include_str!("../data/catalogs/dtc/reference_catalog.json");
    let duplicate_dtc = r#"{"schema_version":1,"status":"reference_only","entries":[{"code":"P0001","descriptions":["one","one"]}]}"#;
    assert!(matches!(
        ReferenceCatalogs::from_json(obd, duplicate_dtc, ""),
        Err(ReferenceError::Entry(key)) if key == "P0001"
    ));
    let duplicate_nrc = r#"{"schema_version":1,"status":"reference_only","entries":[{"code_hex":"78","labels":[{"name":"Pending","context":"uds_reference"},{"name":"Pending","context":"uds_reference"}]}]}"#;
    assert!(matches!(
        ReferenceCatalogs::from_json(obd, dtc, duplicate_nrc),
        Err(ReferenceError::Entry(key)) if key == "78"
    ));
}

#[test]
fn dbc_preview_decodes_synthetic_signal_and_requires_exact_frame_length() {
    let dbc = include_str!("../data/catalogs/fixtures/dbc/opendbc-test.dbc");
    let payload = [0, 0x0f, 0, 0, 0, 0, 0, 0];
    let result = decode_dbc_signal(dbc, 316, "COUNTER", &payload).expect("fixture signal");
    assert_eq!(result.raw, 15);
    assert_eq!(result.scaled, 15.0);
    assert!(matches!(
        decode_dbc_signal(dbc, 316, "COUNTER", &payload[..7]),
        Err(ReferenceError::Truncated)
    ));
    assert!(decode_dbc_signal(dbc, 316, "missing", &payload).is_err());
    assert!(decode_dbc_signal(dbc, 999, "COUNTER", &payload).is_err());
}

#[test]
fn dbc_preview_rejects_ambiguous_and_unsupported_layouts() {
    let header = "BO_ 123 Example: 2 ECU\n";
    let signal = " SG_ Value : 0|16@1+ (0.5,-1) [0|100] \"\" ECU\n";
    let valid = format!("{header}{signal}");
    let result = decode_dbc_signal(&valid, 123, "Value", &[0x34, 0x12]).expect("simple");
    assert_eq!(result.raw, 0x1234);
    assert_eq!(result.scaled, 2329.0);
    for dbc in [
        format!("{valid}{signal}"),
        format!("{valid}{header}{signal}"),
        valid.replace("Value :", "Value m1 :"),
        format!("{valid}SIG_VALTYPE_ 123 Value : 1;"),
        format!("{valid}SG_MUL_VAL_ 123 Value Switch 1-2;"),
        valid.replace("0|16", "8|16"),
        valid.replace("0.5", "NaN"),
    ] {
        assert!(decode_dbc_signal(&dbc, 123, "Value", &[0x34, 0x12]).is_err());
    }
    assert!(decode_dbc_signal(&valid, 0x8000_007b, "Value", &[0x34, 0x12]).is_err());
}

#[test]
fn dbc_preview_preserves_standard_and_extended_identity() {
    let dbc = concat!(
        "BO_ 123 Standard: 1 ECU\n",
        " SG_ Value : 0|8@1+ (1,0) [0|255] \"\" ECU\n",
        "BO_ 2147483771 Extended: 1 ECU\n",
        " SG_ Value : 0|8@1+ (2,0) [0|510] \"\" ECU\n",
        "BO_ 2684354559 MaxExtended: 1 ECU\n",
        " SG_ Value : 0|8@1+ (3,0) [0|765] \"\" ECU\n",
    );
    let standard = decode_dbc_signal(dbc, 123, "Value", &[10]).expect("standard");
    let extended = decode_dbc_signal(dbc, 0x8000_007b, "Value", &[10]).expect("extended");
    let maximum = decode_dbc_signal(dbc, 0x9fff_ffff, "Value", &[10]).expect("maximum");
    assert_eq!(standard.scaled, 10.0);
    assert_eq!(extended.scaled, 20.0);
    assert_eq!(maximum.scaled, 30.0);
    for invalid in [0x800, 0x1fff_ffff, 0xa000_007b, 0xc000_007b, u32::MAX] {
        assert!(matches!(
            decode_dbc_signal(dbc, invalid, "Value", &[10]),
            Err(ReferenceError::Layout)
        ));
    }
}

#[test]
fn dbc_preview_rejects_tab_separated_extended_metadata() {
    let dbc = concat!(
        "BO_ 2147483771 Extended: 1 ECU\n",
        " SG_ Value : 0|8@1+ (1,0) [0|255] \"\" ECU\n",
    );
    for metadata in [
        "SIG_VALTYPE_\t2147483771 Value : 1;",
        "SG_MUL_VAL_\t2147483771 Value Switch 1-2;",
    ] {
        let input = format!("{dbc}{metadata}");
        assert!(matches!(
            decode_dbc_signal(&input, 0x8000_007b, "Value", &[10]),
            Err(ReferenceError::Dbc(_))
        ));
    }
    let unrelated = format!("{dbc}SIG_VALTYPE_\t123 Value : 1;");
    assert!(decode_dbc_signal(&unrelated, 0x8000_007b, "Value", &[10]).is_ok());
}
