# Slint native feasibility preview — F22.02

Separate, non-shipping experiment; depends on the reviewed decision in ADR-001. No shared backend, adapter access, vehicle commands, persistence, network or release packaging. The sole displayed measurement is explicitly synthetic and disappears when the user exits the demo. This is not feature parity or an implemented diagnostic application.

Candidate Slint 1.19.1 runtime and compiler, Winit + software renderer, GPL-3.0-only option. Rust 1.100.0 in repository CI. New dependency tree needs licence and security review. Initial lockfile is intentionally not fabricated: the read-only bootstrap CI generates a candidate Cargo.lock artifact for review and later commit, then runs all commands with --locked. Until committed and independently reviewed, this PR must remain draft. Linux bootstrap only, not proof of Windows/macOS support or actual window launch. Existing Tauri CI stays unchanged.

After lockfile review and commit, from this directory:

    cargo fmt --all -- --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked
    cargo build --release --locked
    cargo run --locked

Test window launch, resize and keyboard navigation on each OS, verify no physical adapter or file access. No measured startup, CPU or memory claim. Do not distribute this experiment. Next: locked matrix, headless UI-level tests, dependency review and typed core extraction after governance approval.
