# Diagnostic data staging catalog

Version 1. Source revisions, hashes, license paths, and limitations: [manifest.json](manifest.json). Existing SQLite/VIN data and recorded responses stay at their existing paths; no migration, replacement, runtime auto-discovery or packaging change in this PR.

- dbc/: 58 standalone candidate CAN signal definitions, for offline/passive interpretation only. No live transmit, actuator control, or generation. Verify provenance, parser and vehicle compatibility before activation. Generator fragments are not standalone files and are excluded.
- fixtures/dbc/: one synthetic parser fixture.
- obd2/: 198 normalized reference rows in services 01/03/06/07/09; unique request hex; not a capability claim or dispatch allowlist. No mode 04, 08, executable decoder or implied mode 02 support.
- dtc/: 2066 unique reference code groups; source descriptions retained, not merged with the existing SQLite database. OEM meanings require validated manufacturer context.
- uds/: 65 unique negative response code groups, 68 labels with contexts. No commands or automatic retry policy.
- licenses/: license texts and attribution. External ECU fixtures/translations already staged under docs/references/vendor/ddt4all; reference them instead of duplicating them.

Native Rust integration gates (F10/F15/F16/F20): schema version, typed parser, size bounds, duplicate and conflicting definitions, finite scaling and bit layout, provenance, explicit activation, per-ECU capability and read-only policy, rollback, synthetic/offline tests. Never equate source catalog presence to confirmed hardware support. No Python or external runtime dependency. Remaining data coverage, decoder formulas, OEM DIDs/DTCs and rights are pending. Existing main data remain canonical until a separately tested migration is approved.

## Offline Rust inspection

From backend: `cargo run --locked --example catalog_lookup -- obd 010C`, `cargo run --locked --example catalog_lookup -- dtc P0001`, or `cargo run --locked --example catalog_lookup -- nrc 78`. These return reference JSON or null for an unknown key, not a vehicle response. `cargo run --locked --example catalog_lookup -- decode 3412 0 16 intel unsigned 0.5 -1` decodes a synthetic payload. The example and integration tests share `src/reference_data.rs`; no Tauri command, live CAN traffic, automatic activation or SQLite migration. The bit extractor is not a DBC file parser; multiplex, checksums and vehicle compatibility still require validation.
