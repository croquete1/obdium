# ADR-001 — Desktop native Slint migration

Status: proposed for independent review. Owner authorized the migration on 2026-11-27; this PR does not approve itself or change the running application. Baseline main: 314fa378b4de35cd9c44a6bc2df2aab9dddefa11. PR #13 exact head cc3046abfe1e5e7811ea8d8c7aed32178d7816bc: CI 45972903586 and House Rules 45972903417 passed. All existing PRs remain separate; integrate reviewed fixes individually and rebase before extracting shared code.

## Verified baseline

- backend/Cargo.toml: Rust package with Tauri 2, tauri-build, shell/fs/dialog plugins; SQLite, serialport, Tokio and other core libraries.
- frontend/index.html, layout/styles CSS and scripts/main.js, events.js, features.js, settings.js: vanilla JavaScript, not React; no package.json in observed tree.
- backend/src/lib.rs is a starting boundary for diagnostic logic, not proof that the core can compile without Tauri. main.rs, bridge/events.rs, bridge/mod.rs and stats.rs use Tauri event/window/runtime integration. VIN app-data initialization and resource resolution are currently tied to app setup; relative SQLite paths are a known risk.
- .github/workflows/ci.yml builds Rust on Linux/Windows/macOS and checks JS; publish.yml remains legacy packaging. Preserve required gates. New native checks supplement them until retirement. No direct commits to main.

## Decision and scope

Target: Slint declarative .slint UI compiled to Rust, typed Rust application services, existing diagnostic core, SQLite and resources. Native Winit backend on Windows, macOS, X11 and Wayland; begin with software renderer candidate, evaluate GPU renderer only with measurements. No Qt, WebView, React, Node or Python in shipped native UI. Existing Python housekeeping scripts are not product dependencies. No mandatory network, account, subscription or telemetry. Keep GPL-3.0. Slint candidate 1.19.1 (exact pin for runtime and build compiler); published package metadata states Rust 1.95 and GPL-3.0-only as a license option. Existing CI Rust 1.100.0 exceeds stated minimum, but transitive resolution, licenses, vulnerability audit and locked builds must still pass. No commercial license purchase. Evaluate full dependency tree and distribution terms before enabling. Sources: https://slint.dev/faqs ; https://docs.slint.dev/latest/docs/rust/slint/docs/cargo_features/ ; https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_winit/ ; https://docs.rs/crate/slint/1.19.1/source/Cargo.toml ; https://docs.rs/crate/slint-build/1.19.1 .

## Architecture and staged acceptance

| ID | Work | Acceptance |
|---|---|---|
| F22.01 | Governance and ADR | Dedicated reviewed PR, no weakening of rules; coordinate F07/F12/F13 and open PRs. |
| F22.02 | Isolated proof | Separate native executable, pinned runtime/compiler, committed lockfile, three-OS fmt/clippy/test/build, no vehicle dispatch, demonstrable launch and keyboard navigation. Keep legacy build intact. |
| F22.03 | Core extraction | Split core from Tauri-specific binary/build script; no duplicated PID/DTC/protocol logic; cargo tree shows no Tauri in native core closure. Existing regressions and SQLite fixtures unchanged; coordinate #4–#13 before touching shared sources. |
| F22.04 | Application services | One owned worker per diagnostic session, bounded commands/events, monotonic deadline, cancellation/generation, typed states and errors, no blocking UI thread; deterministic replay and stale callback tests. UI callbacks never authorize writes. |
| F22.05 | Read-only parity | Adapter discovery, protocol selection (all nine with evidence), connect/disconnect, live values and charts, DTC states, readiness, freeze frame, VIN, demo/replay, custom PIDs, terminal read-only restrictions. Preserve actual capabilities; distinguish absent/stale from zero, show demo label. |
| F22.06 | Persistence and UX | Explicit app-data/resource roots on all OS, reuse existing SQLite/recordings/settings, transactional migration + backup/restore, no data loss; PT-PT/EN, units, keyboard/focus/screen reader validation, contrast and scalable layouts. |
| F22.07 | Sensitive operations | Independent review and backend authorization, explicit confirmation, target-specific preconditions, no automatic replay; test acknowledgment/error semantics. No hardware actions in CI. |
| F22.08 | Packaging and quality | Locked native matrix, fast format preflight, security/license audit, clean installation and upgrade tests on Windows/macOS/Linux incl. X11/Wayland, native dialogs, offline startup, metrics vs legacy (startup/RSS/CPU/chart refresh), rollback. No publishing without permission. |
| F22.09 | Cutover | Verify feature and migration parity, human review and physical test requirements, switch default only after acceptance. Then remove Tauri build/plugins/IPC/WebView/frontend and obsolete JS gates in separate reviewed PR, replace with native equivalents. Preserve old release and data backup; no history rewrite. |

## Safety, trade-offs and risks

No performance claim until measurement. Slint is a toolkit, not a graphing, installer, serial transport or database migration solution: implement and test those explicitly. Keep protocol parsing and policy in Rust, not in .slint expressions. UI thread must consume bounded snapshots with session identity; old samples are never rendered as new. No raw command execution or sensitive ECU actions in prototype. Test screen reader support and OS-level file dialogs instead of assuming WebView parity. Mobile F13 is separate and requires target-specific transport/permissions/testing. Rollback means retain the legacy executable and original user data; do not change data paths silently.

Issues disabled in fork; dedicated PR acts as claim until issues are enabled by owner. CI green is necessary but not independent approval. This ADR does not authorize releases, paid services, changes to branch protection or physical vehicle operation.
