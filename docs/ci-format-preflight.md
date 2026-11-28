# Rust format preflight — F01.03

Run the pinned formatter before pushing Rust changes:

```sh
cd backend
rustup toolchain install 1.100.0 --profile minimal --component rustfmt
cargo +1.100.0 fmt --all -- --check
```

If the check fails, use `cargo +1.100.0 fmt --all`, inspect the diff and rerun the check. For contributors without local Rust, the separate preflight workflow checks the backend without GUI/apt prerequisites and uploads an official patch on formatting failure. It has read-only repository permissions and no publishing step.

Evidence: PR #13 at `1006331` failed only Rustfmt in [CI 46046536544](https://github.com/croquete1/obdium/actions/runs/46046536544); 25 lookup tests passed. The exact formatter correction `fc674ca` passed [CI 46051272324](https://github.com/croquete1/obdium/actions/runs/46051272324). PR #15 at `3ff47ca` passed its existing separate Slint format and test workflow; both macOS cancellations in other workflows passed on targeted reruns.

The new check is additive: existing CI, check names, formatting gate, Clippy, tests and builds are unchanged. The workflows run concurrently, so this does not yet prevent the matrix from starting or demonstrate savings. Deliberate temporary duplication avoids touching the shared CI file modified by PRs #8/#10/#9. Once those changes are consolidated, evaluate sequencing the matrix after formatting, preserving mandatory check outcomes and failure evidence. Do not mark F01.03 complete or claim performance gains solely on this proposal.

Acceptance: exact-head preflight and existing CI pass; verify a malformed formatting fixture fails the preflight and produces a usable patch, with independent review before merge. No local Rust execution or negative hosted run claimed at creation.
