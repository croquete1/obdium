#[path = "../src/reference_data.rs"]
mod reference_data;

use reference_data::parse_dbc_message;
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

#[test]
fn simple_multiplex_uses_raw_selector_and_rejects_inactive_signals() {
    let dbc = concat!(
        "BO_ 123 Example: 2 ECU\n",
        " SG_ First m1 : 8|8@1+ (2,0) [0|510] \"\" ECU\n",
        " SG_ Second m2 : 8|8@1+ (3,0) [0|765] \"\" ECU\n",
        " SG_ Selector M : 0|2@1+ (10,5) [5|35] \"\" ECU\n",
        " SG_ Common : 2|6@1+ (1,0) [0|63] \"\" ECU\n",
    );
    let first = decode_dbc_signal(dbc, 123, "First", &[1, 0]).expect("active zero");
    assert_eq!(first.raw, 0);
    assert_eq!(first.scaled, 0.0);
    assert!(matches!(
        decode_dbc_signal(dbc, 123, "Second", &[1, 0]),
        Err(ReferenceError::InactiveSignal)
    ));
    let second = decode_dbc_signal(dbc, 123, "Second", &[2, 10]).expect("second branch");
    assert_eq!(second.scaled, 30.0);
    let selector = decode_dbc_signal(dbc, 123, "Selector", &[1, 0]).expect("selector");
    assert_eq!(selector.scaled, 15.0);
    assert!(decode_dbc_signal(dbc, 123, "Common", &[3, 0]).is_ok());
}

#[test]
fn simple_multiplex_rejects_missing_duplicate_and_unrepresentable_selectors() {
    let dbc = concat!(
        "BO_ 123 Example: 2 ECU\n",
        " SG_ Value m1 : 8|8@1+ (1,0) [0|255] \"\" ECU\n",
        " SG_ Selector M : 0|2@1+ (1,0) [0|3] \"\" ECU\n",
    );
    for invalid in [
        dbc.replace("Value m1", "Value m4"),
        dbc.replace("Selector M", "Selector"),
        dbc.replace("Selector M", "Selector m1M"),
        dbc.replace("0|2@1+", "0|2@1-"),
        format!("{dbc} SG_ Other M : 2|2@1+ (1,0) [0|3] \"\" ECU\n"),
        format!("{dbc}SG_MUL_VAL_ 123 Value Selector 1-2;"),
    ] {
        assert!(decode_dbc_signal(&invalid, 123, "Value", &[1, 10]).is_err());
    }
}

#[test]
fn dbc_rejects_simultaneously_active_overlap_in_both_orders() {
    let dbc = concat!(
        "BO_ 123 Example: 2 ECU\n",
        " SG_ Selector M : 0|2@1+ (1,0) [0|3] \"\" ECU\n",
        " SG_ First m1 : 8|8@1+ (1,0) [0|255] \"\" ECU\n",
        " SG_ Second m2 : 8|8@1+ (1,0) [0|255] \"\" ECU\n",
    );
    assert!(decode_dbc_signal(dbc, 123, "First", &[1, 10]).is_ok());
    let common = " SG_ Common : 15|8@0+ (1,0) [0|255] \"\" ECU\n";
    let same = " SG_ Other m1 : 12|4@1+ (1,0) [0|15] \"\" ECU\n";
    let before = dbc.replace(" SG_ Selector", &format!("{common} SG_ Selector"));
    for invalid in [
        format!("{dbc}{common}"),
        before,
        format!("{dbc}{same}"),
        dbc.replace("0|2@1+", "8|2@1+"),
    ] {
        assert!(matches!(
            decode_dbc_signal(&invalid, 123, "First", &[1, 10]),
            Err(ReferenceError::SignalOverlap)
        ));
    }
}

#[test]
fn dbc_validates_inactive_and_unselected_signal_definitions() {
    let dbc = concat!(
        "BO_ 123 Example: 2 ECU\n",
        " SG_ Selector M : 0|2@1+ (1,0) [0|3] \"\" ECU\n",
        " SG_ First m1 : 8|8@1+ (1,0) [0|255] \"\" ECU\n",
        " SG_ Second m2 : 8|8@1+ (1,0) [0|255] \"\" ECU\n",
    );
    let bad_layout = dbc.replace("Second m2 : 8|8", "Second m2 : 12|8");
    assert!(matches!(
        decode_dbc_signal(&bad_layout, 123, "First", &[1, 10]),
        Err(ReferenceError::Truncated)
    ));
    let bad_scale = dbc.replace("Second m2 : 8|8@1+ (1,0)", "Second m2 : 8|8@1+ (NaN,0)");
    assert!(matches!(
        decode_dbc_signal(&bad_scale, 123, "First", &[1, 10]),
        Err(ReferenceError::Scaling)
    ));
}

#[test]
fn dbc_bit_ownership_handles_cross_byte_and_64_byte_boundaries() {
    let dbc = concat!(
        "BO_ 123 Example: 64 ECU\n",
        " SG_ First : 63|16@0+ (1,0) [0|65535] \"\" ECU\n",
        " SG_ Last : 511|1@1+ (1,0) [0|1] \"\" ECU\n",
    );
    let mut payload = [0; 64];
    payload[63] = 128;
    let last = decode_dbc_signal(dbc, 123, "Last", &payload).expect("last bit");
    assert_eq!(last.raw, 1);
    let overlap = format!("{dbc} SG_ Other : 64|1@1+ (1,0) [0|1] \"\" ECU\n");
    assert!(matches!(
        decode_dbc_signal(&overlap, 123, "Last", &payload),
        Err(ReferenceError::SignalOverlap)
    ));
    let outside = dbc.replace("511|1", "511|2");
    assert!(matches!(
        decode_dbc_signal(&outside, 123, "Last", &payload),
        Err(ReferenceError::Truncated)
    ));
}

#[test]
fn parsed_dbc_message_reuses_definition_without_reusing_observations() {
    let dbc = concat!(
        "BO_ 123 Example: 2 ECU\n",
        " SG_ Selector M : 0|2@1+ (1,0) [0|3] \"\" ECU\n",
        " SG_ First m1 : 8|8@1+ (1,0) [0|255] \"\" ECU\n",
        " SG_ Second m2 : 8|8@1+ (2,0) [0|510] \"\" ECU\n",
    );
    let message = parse_dbc_message(dbc, 123).expect("valid definition");
    assert_eq!(message.decode("First", &[1, 7]).expect("first").raw, 7);
    assert_eq!(message.decode("First", &[1, 0]).expect("zero").raw, 0);
    assert!(matches!(
        message.decode("First", &[2, 8]),
        Err(ReferenceError::InactiveSignal)
    ));
    assert_eq!(
        message.decode("Second", &[2, 8]).expect("second").scaled,
        16.0
    );
    assert!(matches!(
        message.decode("Second", &[2]),
        Err(ReferenceError::Truncated)
    ));
    assert!(message.decode("Unknown", &[2, 8]).is_err());
}

#[test]
fn parsed_dbc_message_rejects_invalid_unobserved_definitions() {
    let dbc = concat!(
        "BO_ 123 Example: 1 ECU\n",
        " SG_ First : 0|8@1+ (1,0) [0|255] \"\" ECU\n",
        " SG_ Other : 7|1@1+ (1,0) [0|1] \"\" ECU\n",
    );
    assert!(matches!(
        parse_dbc_message(dbc, 123),
        Err(ReferenceError::SignalOverlap)
    ));
    let long_name = "X".repeat(129);
    let oversized =
        format!("BO_ 123 Example: 1 ECU\n SG_ {long_name} : 0|8@1+ (1,0) [0|255] \"\" ECU");
    assert!(matches!(
        parse_dbc_message(&oversized, 123),
        Err(ReferenceError::Layout)
    ));
}

#[test]
fn dbc_namespace_symbols_do_not_trigger_extended_metadata_errors() {
    let header = "VERSION \"\"\nNS_ :\n\tSIG_VALTYPE_\n\tBU_SG_REL_\n\tSG_MUL_VAL_\n\nBS_:\n";
    let frame = concat!(
        "BO_ 123 Example: 1 ECU\n",
        " SG_ Value : 0|8@1+ (1,0) [0|255] \"\" ECU\n",
    );
    let dbc = format!("{header}{frame}");
    let result = decode_dbc_signal(&dbc, 123, "Value", &[42]).expect("namespace declarations");
    assert_eq!(result.raw, 42);
    for suffix in [
        "SIG_VALTYPE_ 123 Value : 1;",
        "SG_MUL_VAL_ 123 Value Selector 1-2;",
        "SIG_VALTYPE_",
        "NS_ :\nSG_MUL_VAL_",
    ] {
        assert!(decode_dbc_signal(&format!("{dbc}{suffix}"), 123, "Value", &[42]).is_err());
    }
    let misplaced = format!("NS_:\nSIG_VALTYPE_ 123 Value : 1;\n{frame}");
    assert!(decode_dbc_signal(&misplaced, 123, "Value", &[42]).is_err());
}

#[test]
fn dbc_reads_staged_definition_with_namespace_header() {
    let dbc = include_str!("../data/catalogs/dbc/acura_ilx_2016_nidec.dbc");
    let message = parse_dbc_message(dbc, 768).expect("reference definition");
    let sample = message
        .decode("VEHICLE_SPEED", &[0, 42, 0, 0, 0, 0, 0, 0])
        .expect("synthetic payload");
    assert_eq!(sample.raw, 42);
    assert_eq!(sample.scaled, 42.0);
}

#[test]
fn dbc_multiline_comments_do_not_create_messages_or_metadata() {
    let dbc = concat!(
        "CM_ \"comment with a semicolon; and an escaped quote \\\"\n",
        "BO_ 123 Fake: 1 ECU\n",
        "SIG_VALTYPE_ 123 Fake : 1;\n",
        "NS_ :\n",
        "end\";\n",
        "BO_ 123 Real: 1 ECU\n",
        " SG_ Value : 0|8@1+ (1,0) [0|255] \"\" ECU\n",
        "CM_ BO_ 123 \"another comment\n",
        "SG_MUL_VAL_ 123 Value Selector 1-2;\n",
        "BO_ 123 Duplicate: 1 ECU\n",
        "end\";\n",
    );
    let sample = decode_dbc_signal(dbc, 123, "Value", &[42]).expect("real definition");
    assert_eq!(sample.raw, 42);
    let actual_metadata = format!("{dbc}SIG_VALTYPE_ 123 Value : 1;");
    assert!(decode_dbc_signal(&actual_metadata, 123, "Value", &[42]).is_err());
}

#[test]
fn dbc_rejects_unterminated_and_ambiguous_comments() {
    let dbc = concat!(
        "BO_ 123 Example: 1 ECU\n",
        " SG_ Value : 0|8@1+ (1,0) [0|255] \"\" ECU\n",
    );
    for suffix in [
        "CM_ \"never closed",
        "CM_ \"missing semicolon\"",
        "CM_ \"closed\"; SIG_VALTYPE_ 123 Value : 1;",
        "CM_ without_string;",
        "CM_ \"dangling escape\\",
    ] {
        let input = format!("{dbc}{suffix}");
        assert!(matches!(
            parse_dbc_message(&input, 123),
            Err(ReferenceError::Dbc(_))
        ));
    }
}

#[test]
fn dbc_comments_cannot_supply_missing_definitions() {
    let dbc = concat!("CM_ \"fake message\n", "BO_ 123 Fake: 1 ECU\n", "end\";\n",);
    assert!(matches!(
        parse_dbc_message(dbc, 123),
        Err(ReferenceError::Dbc(_))
    ));
}

#[test]
fn dbc_rejects_multiline_non_comment_strings_before_decoding() {
    let frame = "BO_ 123 Real: 1 ECU\n SG_ Value : 0|8@1+ (1,0) [0|255] \"\" ECU\n";
    for prefix in [
        "VERSION \"open\n",
        "BA_ \"Name\" \"open\n",
        "VAL_ 456 Other 0 \"open\n",
        "BO_ 456 Other: 1 ECU\n SG_ Other : 0|8@1+ (1,0) [0|255] \"open\n",
    ] {
        let dbc = format!("{prefix}{frame}end\";\n");
        assert!(matches!(
            parse_dbc_message(&dbc, 123),
            Err(ReferenceError::Dbc(_))
        ));
    }
}

#[test]
fn dbc_accepts_balanced_non_comment_strings_with_escapes() {
    let dbc = concat!(
        "VERSION \"quoted \\\"value\\\"\"\n",
        "BO_ 123 Real: 1 ECU\n",
        " SG_ Value : 0|8@1+ (1,0) [0|255] \"\" ECU\n",
        "VAL_ 123 Value 0 \"zero; \\\"quoted\\\"\" 1 \"one\";\n",
        "BA_ \"Name\" \"backslash \\\\\";\n",
    );
    let sample = decode_dbc_signal(dbc, 123, "Value", &[0]).expect("balanced strings");
    assert_eq!(sample.raw, 0);
}

#[test]
fn reusable_and_one_shot_dbc_decoders_share_signal_name_limits() {
    let name = "X".repeat(128);
    let dbc = format!("BO_ 123 Example: 1 ECU\n SG_ {name} : 0|8@1+ (1,0) [0|255] \"\" ECU\n");
    let message = parse_dbc_message(&dbc, 123).expect("valid maximum-length name");
    assert_eq!(message.decode(&name, &[42]).expect("reusable").raw, 42);
    assert_eq!(
        decode_dbc_signal(&dbc, 123, &name, &[42])
            .expect("one shot")
            .raw,
        42
    );
    let oversized = "X".repeat(129);
    for invalid in ["", oversized.as_str()] {
        assert!(matches!(
            message.decode(invalid, &[42]),
            Err(ReferenceError::Layout)
        ));
        assert!(matches!(
            decode_dbc_signal(&dbc, 123, invalid, &[42]),
            Err(ReferenceError::Layout)
        ));
        // Both entry points must reject invalid names before other work.
        assert!(matches!(
            message.decode(invalid, &[]),
            Err(ReferenceError::Layout)
        ));
        assert!(matches!(
            decode_dbc_signal("not a DBC", 123, invalid, &[]),
            Err(ReferenceError::Layout)
        ));
    }
    assert!(matches!(
        message.decode("unknown", &[42]),
        Err(ReferenceError::Dbc(_))
    ));
}
