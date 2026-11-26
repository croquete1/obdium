# Diagnostic data staging catalog

Version 1. Source revisions, hashes, license paths, and limitations: [manifest.json](manifest.json). Existing SQLite/VIN data and recorded responses stay at their existing paths; no migration, replacement, runtime auto-discovery or packaging change in this PR.

- dbc/: 19 standalone candidate CAN signal definitions, for offline/passive interpretation only. No live transmit, actuator control, or generation. Verify provenance, parser and vehicle compatibility before activation. Generator fragments are not standalone files and are excluded.
- fixtures/dbc/: one synthetic parser fixture.
- obd2/: 198 normalized reference rows in services 01/03/06/07/09; unique request hex; not a capability claim or dispatch allowlist. No mode 04, 08, executable decoder or implied mode 02 support.
- uds/: 65 unique negative response code groups, 68 labels with contexts. No commands or automatic retry policy.
- licenses/: license texts and attribution. External ECU fixtures/translations already staged under docs/references/vendor/ddt4all; reference them instead of duplicating them.

Native Rust integration gates (F10/F15/F16/F20): schema version, typed parser, size bounds, duplicate and conflicting definitions, finite scaling and bit layout, provenance, explicit activation, per-ECU capability and read-only policy, rollback, synthetic/offline tests. Never equate source catalog presence to confirmed hardware support. No Python or external runtime dependency. Remaining data coverage, decoder formulas, OEM DIDs/DTCs and rights are pending. Existing main data remain canonical until a separately tested migration is approved.
