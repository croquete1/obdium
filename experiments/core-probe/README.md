# F22.03 — Headless diagnostic core feasibility probe

This non-shipping package references the real backend/src/lib.rs directly; Rust module resolution uses its existing sibling files. No copied parser, no Tauri dependency/build script, no Slint dependency, no edits to production Cargo.toml or shared source. The manifest temporarily mirrors the existing non-Tauri dependencies from main 4337a5b48005013f597a7029181ad0455642b3cc; it is not the final workspace layout. Existing patches #4–#13 remain separate and must be integrated before permanent extraction. A successful probe proves only the selected revision compiles and tested headless behavior, not full safe live diagnosis or native application integration.

Two integration tests exercise the existing disconnected constructor and missing response representation without opening any adapter or modifying any database. Existing inline library tests also run. Resource roots and session lifecycle remain unresolved; no connection or live reads in this probe. Do not run against a vehicle.

The first Linux run generates a candidate lockfile using Cargo and exports an artifact and exact bounded base64 for readback. Subsequent work must commit the verified lock and require locked checks across Windows/Linux/macOS; no release until dependency/security/license review. Workflow verifies the actual normal/build dependency closure contains no Tauri, WebView or Slint crates. It does not prove the whole future binary is GUI-independent.

Next after independent governance approval and integration of existing protocol fixes: replace the temporary manifest with one canonical core package, typed resource roots, explicit session ownership, and a native UI consumer. Avoid duplicating dependencies and data. No performance claim. 
