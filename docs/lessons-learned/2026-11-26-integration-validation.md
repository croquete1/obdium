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
