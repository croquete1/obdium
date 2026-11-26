# 2026-11-26 — Bootstrap CI, formatting and diagnostic placeholders

## Evidence

- [Baseline CI run 45793443189](https://github.com/croquete1/obdium/actions/runs/45793443189), Linux job 139817978883: 14 official rustfmt diff hunks in cmd.rs and obd.rs; Clippy rejected 19 compiler warnings. Rust 1.100.0 was observed.
- [Dependency CI run 45793531502](https://github.com/croquete1/obdium/actions/runs/45793531502), Windows job 139818263441: identical warning failures; cargo test and release build passed, but the log shows **zero tests**. A passing empty test suite is not evidence of correct behavior.
- [Dependency preview 45792986407](https://github.com/croquete1/obdium/actions/runs/45792986407) completed successfully, including resolver, Linux check and test command; the preview used the pre-fix application code.

## Root causes and corrective work

- Existing source had not been formatted using the configured formatter; applied all 14 hunks directly from its official output, not guessed formatting.
- Mode 06 APIs were public placeholders with todo!(), unused imports/parameters and a private MID field with no accessor. Returning explicit Mode06Unavailable errors avoids panic or fake success; no actual Mode 06 protocol support is claimed. Parameters intentionally unused until implementation retain their meaning. Public return types change to Result; in-tree consumers must compile in CI and external callers must handle errors. Removed the unrelated unused std::default import identified by the same baseline check.
- Added two actual unit tests: one checks all sixteen unavailable Mode 06 entry points and another verifies inclusive limits and MID identity. These tests are written, not yet proven green at the time of this entry.
- Pin the Rust toolchain observed in CI. Provide official formatter diff artifacts on failure without changing committed source or masking the failing check.
- To unblock initial strict CI, include a narrowly scoped baseline remediation in infrastructure PR #3 as an explicit bootstrap exception, in separate commits. Keep future functional fixes and CI changes separate.

## Prevention

Use cargo fmt before commits; take formatter output seriously across all files. Reject unreachable/placeholder success paths and unused API scaffolding in production. Examine actual executed test counts and add meaningful tests for real behavior. Check same errors on the baseline and dependency candidate before attributing failures to updated packages. Do not weaken -D warnings, bypass tests, or count a green rerun as proof. Recheck final HEAD, independent review and full matrix before merging. F05 Mode 06 remains open; no hardware validation or OEM compatibility claim.

## Follow-up: second CI run

[Run 45795237255](https://github.com/croquete1/obdium/actions/runs/45795237255), exact head 3c4cc19b97e3eede4b8186cd4ca5dd141142b633: frontend syntax passed; Windows/macOS test steps passed; Linux rustfmt and all three Clippy steps failed. At inspection time full Rust jobs were still active, and the job log endpoint returned BlobNotFound. Artifact 15105241732 contained an official one-hunk patch for the test's five-element array. Applied the exact hunk, not guessed formatting. Clippy root cause is not yet known; do not infer that all warnings are resolved. Retain active CI runs during bootstrap and publish per-platform Clippy failure logs using pipefail so diagnostics are available before long release builds finish. Restrict rustfmt artifacts to actual formatter failures. No gate waived; new head needs validation.

## Queued next diagnostic safety task

BUG-004 / F04.04: reconfirmed frontend/scripts/features.js:15–33 falsely announces DTC clearance before acknowledgement, clears the UI and refreshes prematurely; backend/src/bridge/events.rs:365–371 ignores the operation Result. backend/src/pid/diagnostics.rs:256–268 checks only formatted equality with 44. Preserve the prior diagnostic evidence, require explicit confirmation and warnings, propagate typed verified/failed/partial/uncertain outcomes bound to the current session, reconcile only with read-only refresh and never retry uncertain writes automatically. Add deterministic simulator regressions and authorized physical validation when applicable. No implementation yet. Issue creation was rejected by GitHub (410, Issues disabled on fork); maintain this backlog in the roadmap until Issues is enabled by the owner.
