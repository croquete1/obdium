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

From backend: `cargo run --locked --example catalog_lookup -- obd 010C`, `cargo run --locked --example catalog_lookup -- dtc P0001`, or `cargo run --locked --example catalog_lookup -- nrc 78`. These return reference JSON or null for an unknown key, not a vehicle response. `cargo run --locked --example catalog_lookup -- decode 3412 0 16 intel unsigned 0.5 -1` decodes a synthetic payload. The example and integration tests share `src/reference_data.rs`; no Tauri command, live CAN traffic, automatic activation or SQLite migration. The DBC preview supports a validated subset described below; checksums and vehicle compatibility still require validation.

## Localization preview

16 draft JSON locale resources with 12 stable plain-text keys each (192 entries), no fuzzy or executable expressions. PT-PT terminology adapted, not independently reviewed. `cargo run --locked --example catalog_lookup -- translate pt-PT file` previews a message; unknown locales fall back to English. No UI localization claim. Source PO notices and translator credits preserved in the technical reference snapshot and license directory.

### Limited DBC preview

`cargo run --locked --example catalog_lookup -- dbc data/catalogs/fixtures/dbc/opendbc-test.dbc 316 COUNTER 000F000000000000` reads a synthetic fixture, selects a decimal standard CAN identifier and exact signal name, validates frame length, and decodes a simple integer signal. Rejects duplicate target messages/signals and unsupported extended signal metadata; standard and encoded extended IDs and simple multiplexing are supported. Bounds file read at 2 MiB. No transmit, automatic vehicle selection, value labels, checksum/counter validation, full DBC parsing or hardware compatibility claims.

### Extended CAN identifiers

The offline DBC selector accepts decimal DBC IDs: standard 0–2047, or extended 29-bit IDs encoded with bit 31 set (0x80000000–0x9fffffff). Reserved bits and unflagged values above 2047 are rejected; no automatic conversion or inference. Standard and extended frames with the same lower bits remain distinct. This is not a transport change. Advanced multiplexing and floating-point definitions remain unsupported; metadata keywords separated by tabs are recognized and rejected for the selected frame.

### Simple multiplexing

The preview accepts one unsigned integer selector (`M`) and branches (`m<number>`). Branch activation uses raw selector bits, never the scaled display value. Inactive signals produce an explicit error, distinct from zero. Missing/duplicate selectors, unrepresentable branch values, nested multiplexing and `SG_MUL_VAL_` metadata are rejected. This is an offline subset; floating-point definitions, complete DBC validation and live integration remain outside this implementation.

### Signal ownership validation

Every signal in the selected message now undergoes bit-bound and finite-scaling validation before decoding, including inactive branches. A 512-bit occupancy mask distinguishes common signals and each simple multiplex branch; overlapping bits in mutually exclusive branches are allowed, simultaneous overlap returns `SignalOverlap`. This conservative subset rejects ambiguous aliases rather than guessing. Intel and Motorola traversal share the same helper. No claim of support for every staged DBC or physical CAN FD framing.

### Reusable validated message

`parse_dbc_message` validates one selected definition once and returns an immutable `DbcMessage` with a bounded signal index (up to 4096 signals, names up to 128 bytes). `DbcMessage::decode` validates each new payload and resolves the selector from that payload, never caching observed values. The single-observation wrapper retains the same public interface. This removes repeated definition parsing when a consumer reuses the returned object; no throughput improvement is quantified without measurement. Still offline; no live frame or GUI integration.

### Namespace declaration handling

The staged DBC reader distinguishes bare, known `NS_` declarations from actual extended metadata. The declaration list is recognized only once before a message, and ends at the first non-declaration statement. Selected-message `SIG_VALTYPE_`/`SG_MUL_VAL_` records and malformed bare keywords outside that section remain errors. Added synthetic regressions and a staged Acura definition with synthetic payload. No new catalog data, no vehicle-specific compatibility claim, and no full DBC grammar support. Multiline quoted comment parsing remains a separate limitation; do not trust arbitrary imported DBC input for live diagnostics.

### Quoted comment boundary

The limited parser now skips `CM_` quoted comments spanning multiple lines, including semicolons and escaped quotes inside the text, before interpreting message, namespace and metadata keywords. Missing quotes/terminators, dangling escapes and trailing statements on the closing line fail closed. It intentionally requires the closing quote and semicolon on the same line and does not implement a general DBC lexer (multiline non-comment records remain unsupported and must not be assumed safe). Three synthetic regressions exercise spoofed definitions, real metadata after a comment, and malformed comments. No live activation or data import.

### Non-comment quoted strings

Outside recognized `CM_` comments, every line must have balanced double-quoted strings, respecting backslash escapes inside quotes. Unterminated and multiline non-comment strings now fail closed before apparent message/signal declarations can be interpreted as definitions. Balanced ordinary version, attribute, signal-unit and value-label strings remain acceptable to the existing limited reader; the strings' metadata semantics are not interpreted. This is a conservative line-boundary check, not a full DBC lexer or universal compatibility guarantee.

## Staging integrity gate — 2026-11-28

The manifest now records the local Git blob identities of all three derived catalogs and the remaining license texts, separately from upstream source identities. House Rules discovers `scripts/tests/test_catalog_integrity.py`, which verifies the 82 versioned data/license files against their recorded hashes, existing byte lengths, inventory, canonical relative paths and staged status, plus synthetic negative controls. Uses only the existing Python standard-library repository housekeeping environment, not the DDT4All dependency stack or a product runtime. Run `python3 -m unittest discover -s scripts/tests -p 'test_catalog_integrity.py' -v`. Baseline `fc674ca` CI 46051272324 and House Rules 46051272211 passed. New CI pending; local execution not claimed. Git SHA-1 identities are consistency checks, not signatures, upstream byte verification for adapted data, independent review, legal clearance or vehicle validation. Preserve draft/non-activation until provenance, rights and independent review are resolved.

## Translation schema and unambiguous JSON checks — 2026-11-28

Baseline `1fe5a2f` passed CI 46161359665 and House Rules 46161359662 (23/23 tests, including vendor snapshot and translation lineage). All 16 checked-in translation previews currently have the same 12 stable keys. Extend the offline integrity gate to enforce the exact key set, draft translation schema, nonempty plain text and strict JSON without duplicate fields or nonstandard numeric constants. Reject boolean schema versions and non-integer vendor byte counts. Nine new synthetic tests cover the new checks; no catalog payload or runtime code changed. New CI pending; no local test run claimed. This is a staging consistency guard, not a complete runtime parser or a linguistic/legal review.

## Rust preview schema enforcement — 2026-11-28

Baseline `7b46091`: CI 46163918340 and House Rules 46163918313 passed; 32/32 governance tests. The Rust preview previously deserialized messages into a plain map, which can silently overwrite duplicate message keys, and accepted incomplete or extra keys. Add a bounded unique-key map visitor and require exactly the 12 staged keys before returning a preview. Unknown locales still use English; malformed known preview files fail rather than silently substituting a missing translation. Three Rust regression tests cover identical duplicates, missing/unknown keys, invalid markup and wrong locale. Existing 16-locale lookup test remains. No payload changes, new dependencies, or production UI activation. Local Rustfmt/build/tests unavailable; new exact HEAD CI pending. This validates the message map, not arbitrary unknown JSON metadata or every runtime catalog type.

## Rust preview metadata and single-catalog lookup — 2026-11-28

Baseline `321e655`: CI 46164962326 and House Rules 46164962325 passed; Linux job 141040333801 ran 28/28 reference tests including the three new schema regressions. Because known locales now require all 12 keys, lookup selects and validates exactly one bundled catalog, instead of parsing English for every request and twice for English requests. Unknown locales select English with the fallback flag; unknown keys remain absent. Strict top-level schema accepts the existing optional note/source fields only; non-English catalogs require a 40-character lowercase Git blob identity, English must have no source identity. This is a syntactic check, not upstream hash authentication (the staging tests verify recorded lineage). Two new Rust regressions cover metadata and lookup semantics. All 16 existing preview metadata shapes inspected before editing. No payload changes, dependencies or measured speedup claim; local Rustfmt/compiler unavailable, new CI pending. No production activation.

## Strict OBD-II, DTC and UDS reference schemas — 2026-11-28

Baseline `e2a8390` passed CI 46167890996 and House Rules 46167890939; Linux job 141049418800 confirmed 30/30 reference tests. Inspect all three bundled JSON schemas (198 OBD rows, 2066 DTC groups and 65 NRC groups). Reject unknown header, row and nested NRC label fields rather than silently ignoring them. Existing optional string metadata (license, source revision, source file and note) remains accepted; it is not treated as authenticated provenance or authorization. Three regression tests mutate actual bundled fixtures, with positive controls, covering unknown fields at each level, incorrect metadata type and duplicate known row fields. No data payload, dependency or production behavior change. Local Rustfmt/build/tests unavailable; exact HEAD CI pending. This is an offline strict schema gate, not normative automotive coverage or redistribution clearance.
