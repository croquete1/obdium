# Bootstrap integration validation — 2026-11-26

Validation-only candidate. Do not merge this aggregate PR. Main remains f9ca1772387a4ea9f590bb4ac2299a4fe6e1d2aa.

## Exact inputs

- PR #1: d09283bb7537d7cf22d0557bc4fc3dfdc080d203
- PR #2: 326795e4a75de5aac476036ad9beeef338f737bb
- PR #3: b7b4f064453e40083ad995f33dbf3a6a3dcc5dc9
- PR #4: 60131ac9e597cd8fd0fdb7fb5187c923b6e43285
- PR #5: 7d51b7b646ebf286d6cbb3aea471b4f5ca4e2ae2
- PR #6: 773019c4501952241ac73c676b3b937a85533cfa
- PR #7: 98c3a1ff7b5a9cc394f165af9343869e90816700
- PR #8: 50370da649275ca269466e27880212df7b85db64

The most-derived snapshots resolve overlapping paths: #4 owns obd.rs, #7 owns diagnostics.rs and lib.rs (includes #5 and #6), #8 owns ci.yml (includes #3 and Node guards). #2 removes obsolete root Cargo.lock and supplies the resolver-generated backend lock; #1 supplies governance checks. No source snippets manually copied or checks disabled. Original branches remain untouched. Verify the source blob IDs and deletion against the assembled tree.

## Verified before integration

- PRs #1–#6: successful runs on recorded heads.
- PR #7: CI 45828201352 successful; duplicate run 45828522089 cancelled on macOS while Linux/Windows passed. Cancellation is not a code failure or additional pass.
- PR #8: CI 45828384899 passed frontend 3/3 structural tests, Linux and Windows; macOS cancelled, job 139938387105 rerun requested.
- No independent reviews recorded on PRs #1–#8. Physical adapter validation and application wiring of the new diagnostic API remain open. Mode 04 UI action deliberately unavailable; low-level arbitrary commands remain outside that mitigation.

## Acceptance and learning

Require combined CI matrix, House Rules, dependency preview and frontend guards on the same candidate. Verify actual test counts and exact SHA. A green result on separate branches does not establish compatibility of their combination. Rebuild or reconcile the candidate when source heads move. No roadmap item becomes DONE merely because these checks pass. No merge, release, paid review or branch-protection change.

## Second integration candidate — 26 November 2026

Previous exact candidate 8be90f09fa9c2f28a43c72c2bb847788e3719ac3 passed all three workflows: CI 45834209032 (41 Rust tests, all three OS builds/lint and Linux fmt), House Rules 45834208879 (9 tests), dependency preview 45834208844 and Node DTC guards (3 tests): 53 distinct tests. Duplicate CI 45836702515 also passed. The original snapshot remains accessible by SHA.

Add PR #10 at 8acbbffc16bac82ff31425b1208135d858d48bfd, retaining all eight original PR heads. CI 45844134126 on b79b138bcce004620ee5653e9b35c30b1ffc3628 ran 15 Rust tests in Linux and six Node graph tests, all successful; Linux/Windows Clippy/build succeeded; macOS cancelled and Linux fmt reported one hunk in air.rs, corrected in 8acbbffc16bac82ff31425b1208135d858d48bfd. New head CI pending.

Nine non-overlapping file blobs copied verbatim after comparison to the common CI ancestor. Two overlapping files reconciled explicitly: features.js retains the disabled DTC route from #8 while adding the #10 graph sampler; ci.yml retains the #8 DTC regression step and adds the #10 graph step and .mjs syntax checking. Original main.js, index.html, events.rs bridge and DTC tests unchanged. Expected integrated count: 54 Rust + 9 Python + 9 Node = 72 distinct tests. Must verify actual logs. No merge/release or physical validation. The new candidate is for validation only; independent PR gates still apply.

Learning: never copy a whole overlapping file from a newer PR without reconciling safety mitigations already present. Validate the union of regression tests on the same tree.

## Third integration candidate — 26 November 2026

Candidate 37cef8e3686501a97f0a1ebb199b6965c945ce9c passed Linux/Windows, 54 Rust tests in Linux, nine Node tests, nine House Rules tests and dependency preview. macOS was cancelled in CI 45847992138; do not classify that run as fully green. Isolated #10 exact head f60a198cb788527bbd5120322c8b18f2e3ccdf7c passed all three platforms and frontend in CI 45850327949, with 15 Rust tests and 14 Node tests. PR #8 CI 45852465867 on 10c3f7c2dff1d64058c3abf45e145b61da4a4afe passed Linux/Windows lint/test/release and frontend; two library tests and three binary session tests passed. macOS cancelled and Linux Rustfmt reported two official hunks, corrected in 5760cbd67e8279a6e8852129d18a3e9b85348d3b. Final #8 CI pending.

Add exact #8 session fix and #10 reconnect fix. Compare both source deltas against the previously integrated heads; nine nonoverlapping blobs copied verbatim, features.js and ci.yml reconciled to preserve disabled DTC clearing and both regression suites. Expected integrated total: 54 library + 3 binary Rust + 9 Python + 17 Node = 83 tests. Verify actual logs and exact SHA. No physical validation or claim that all races are solved. Backend session IDs over IPC and remaining in-flight callback and scheduling issues remain open. Standing merge authorization confirmed by maintainer and recorded in PR #1 comment 7196789660; independent review is still absent and rule 33 remains mandatory. No merge or release. Keep validation PR draft.

Learning: count Rust binary tests separately from library tests; validate exact candidate on all platforms and distinguish cancelled jobs from confirmed code failures. Keep prior evidence accessible by immutable SHA; never replace safety mitigations during reconciliation.

## Fourth integration candidate — 26 November 2026

Candidate 24354ef26666cf2d4b820f214567e039959f24aa: CI 45855929314 passed Linux/Windows, Rustfmt, Clippy, release builds, 54 library + 3 binary Rust tests and 17 Node tests. macOS cancelled. House Rules 45855929610 and dependency preview 45855929430 passed. Isolated PR #8 final head 5760cbd67e8279a6e8852129d18a3e9b85348d3b passed all platforms in 45855836757. PR #4 final source head 7177cfc35e17eef7a3e705fc8bd740e979fc6d23, CI 45861507687: 19 Rust tests, Linux/Windows lint/test/build, Linux fmt and frontend syntax passed; macOS cancelled without executing steps. No confirmed code error.

Integrate the exact two changed blobs from PR #4 (obd.rs and serial-receive-deadline lesson); the old integrated blobs matched the prior PR #4 source, so no overlap or other changes to code. Includes four absolute receive deadline tests and four tests of failed framing and complete NO DATA replies. Expected combined count 62 library + 3 binary Rust + 17 Node + 9 Python = 91. Verify actual CI before claiming approval. Roadmap evidence updated on validation branch only; no item marked DONE. Main unchanged. No independent review recorded for #1. Standing merge authorization remains valid but review gate is not satisfied. Hardware and full error propagation still open.

Learning: preserve tested blob identities when integrating independent deltas; do not modify application code in response to a runner cancellation without a diagnosed code failure. Do not infer a fixed macOS environment from green runs on other commits.

## Fifth integration candidate — 26 November 2026

Candidate 091478cb8996cf426e238e0c31dd4460e8053894: CI 45865688548 passed Linux/Windows fmt/lint/build, 62 library + 3 binary Rust tests and 17 Node tests; macOS cancelled. House Rules 45865688683 (9 tests) and dependency preview 45865688621 passed, for 91 distinct tests on completed jobs. PR #8 security rendering CI 45870696212 passed all four jobs on f02790f9f4bd324fd5425f3a5a6d3061a0c1ec56. PR #8 export/scope CI 45872032979 passed Linux/Windows/macOS and frontend on 79d0a185bdfbe48022db4ca3a0b48ee730369f64: two library and three binary Rust tests and 15 Node tests. No confirmed code errors. Integrate seven non-overlapping file blobs verbatim and reconcile three overlapping files: events.js takes only the PID/DTC/readiness rendering regions; features.js takes the JSON export and notification changes but retains graph sampler, tracker ownership and disabled DTC clearing; ci.yml retains graph/reconnect tests and adds literal rendering and JSON export regression suites. Source checks confirm both families of protections survive. Expected integrated count 65 Rust + 29 Node + 9 Python = 103; verify actual runs. Native picker, denied unrelated files, automatic app-data export and WebView smoke tests still required, CSP and Rust recording paths remain open. No independent review of #1 recorded; standing merge authorization does not waive that gate. Validation only, no merge or release.

Learning: green isolated PRs do not prove that overlapping changes coexist. Integrate only the affected regions and keep exact source blob identities where no overlap exists. Frontend helper tests and permission schema compilation are not proof of runtime ACL enforcement.

## Sixth integration candidate — 26 November 2026

Prior integrated 2961474d7859cc53bb1f9a13b8b3463b9599b7cd: CI 45875595379, House Rules 45875595389 and dependency preview 45875595364 all passed. 62 library + 3 binary Rust, 29 Node and 9 Python tests. PR #8 27ee11a4bbee595d758d8c5c5bfae06c5800122a: CI 45885249882 passed Linux/Windows/macOS, Linux Rustfmt, Clippy, release builds and 8 library + 3 binary Rust and 26 Node tests. The earlier CI 45883935232 failed eight Rustfmt hunks and items_after_test_module; tests and builds passed. The corrected HEAD applies the official formatting and puts tests last. Integrate eleven non-overlapping blobs verbatim. Reconcile diagnostics.rs preserving all eight DTC decoder tests and six Readiness tests after production code; events.js preserves generation and graph lifecycle while clearing and updating Readiness snapshot; features.js retains scoped export/graph protection and removes false Freeze Frame banner; ci.yml retains graph tests and adds both suites. Expected combined total 68 library + 3 binary Rust + 40 Node + 9 Python = 120; verify new CI. Freeze Frame temporarily disabled rather than falsely claiming historical data. Readiness accepts narrow single-ECU CAN format; multi-ECU, legacy layouts, runtime WebView, physical validation, explicit session identity remain open. JSON nullable status compatibility must be considered. Main unchanged; independent review not recorded for #1. Validation only; never merge this candidate directly.

Learning: retain test modules at the end of source; distinguish previously passing isolated checks from new combined results. Verify both decoder suites and session protections after resolving overlaps. Do not confuse disabled functionality with completed diagnostic capability.
