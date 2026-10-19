# Primary staged compiler/runtime follow-up

Observed 2026-10-18T20:33:51.630186-06:00. Read-only investigation after publication of audit commit `23a91f96e00ab7c4dbaf1312f35e8805a065dace`.

All 58 staged file versions differ from current master and the refreshed PR #623 head. Fourteen staged paths are absent from both tips. None of the 58 staged blob IDs appeared in the successful path-limited object walk over 635 captured/refreshed public branch and PR heads (5,515 reachable objects for these paths). This supports keeping this refactor at preservation priority P0. It does **not** prove global absence, unique functionality, correctness or readiness for integration.

## Snapshot boundaries

- Primary branch: feat/x3-guardian at `1beb644603a0f1b6de8bb5882e699fdc99bb2e58`.
- Current master: `70423054e2d9560a212ab763d4f265f9cdc448d9`.
- Current PR #623 head: `2df37cf53f0baf9b5357f4a0305a95d03f3ebd0f`; open, mergeable but blocked at observation.
- The earlier audit captured PR #623 at `2ae185d7e0f2ff19da48744a4acc425d9f67ec08`. The PR has advanced; its prior snapshot must not be mistaken for current state.
- All 58 staged file bytes matched their working-tree bytes. The original index SHA-256, HEAD and complete porcelain status stayed unchanged during this review.
- No source files, original refs, indexes or recovery archives were changed or created. No runtime tests were run.

## Locally staged paths absent from both compared tips

- `crates/x3-integration/src/canonical.rs`
- `crates/x3-integration/tests/canonical_compiler.rs`
- `pallets/x3-kernel/src/canonical_trading_tests.rs`
- `pallets/x3-kernel/src/native_trading.rs`
- `x3-lang/compiler/tests/test_runtime_profile.rs`
- `x3-lang/crates/x3-execution/Cargo.toml`
- `x3-lang/crates/x3-execution/src/artifact.rs`
- `x3-lang/crates/x3-execution/src/ir.rs`
- `x3-lang/crates/x3-execution/src/lib.rs`
- `x3-lang/crates/x3-execution/src/profit.rs`
- `x3-lang/crates/x3-execution/src/trading.rs`
- `x3-lang/crates/x3-execution/src/verify.rs`
- `x3-lang/crates/x3-execution/tests/profit_overflow.rs`
- `x3-lang/examples/native_trading.x3`

The new paths include a no_std x3-lang-execution crate, canonical integration intake, native kernel trading and corresponding tests. Source inspection found a canonical metadata-profile decoder and an X3-native host using storage transactions; these are implementation observations, not execution or security proofs. Do not infer that PR #623 already contains these exact source files from a similar trading/canonical title.

## Integration risk

A bulk application of this index onto current master or PR #623 is unsafe to treat as a clean recovery. The staged snapshot is based on a different checkout and its diff against the PR includes substantial changes to existing runtime and test files. For example, relative to the PR, node/tests/supply_invariant_distributed.rs has 15 additions and 719 deletions, runtime/src/tests.rs has 2 additions and 331 deletions, and crates/x3-integration/tests/source_language.rs has 1 addition and 318 deletions. These counts are textual differences, not a finding that tests were intentionally weakened or functionality was removed. They require a focused port that retains current branch behavior and test coverage.

## Next action

Preserve the original index and all staged source bytes in a new independent archive before any recovery, checkout, cleanup or ref repair. Include indexed blob content and per-file hashes, then prove that the new copy reconstructs the recorded staged state. Once preservation and recovery are approved, review artifact-policy compatibility and port only missing behavior into an isolated branch. Never apply the whole old snapshot over newer tests. No preservation archive or integration was executed by this follow-up.

## Evidence and verification

- [staged-file-comparison.json](staged-file-comparison.json): all 58 paths, exact blob IDs, modes, working hashes and per-target diff sizes.
- [public-reachability.json](public-reachability.json): bounded comparison scope, successful exit and result.
- [summary.json](summary.json): before/after invariants and exact compared heads.
- [pr623.json](pr623.json) and [master.json](master.json): fresh GitHub metadata.
- [VALIDATION.json](VALIDATION.json): report integrity checks.

Commands: git status --porcelain=v1 -uall, git branch --show-current, git rev-parse HEAD, git log -10, git ls-files --stage -z, git diff --cached --name-only -z, git ls-tree -r -z, git diff --numstat between blobs, git rev-list --objects restricted to the staged paths, GitHub read APIs, and SHA-256/blob hashing. Git used optional locks disabled and no lazy fetch.

Proof result: read-only comparison PASS; compiler/runtime/live integration tests NOT RUN. Remaining blockers: independent preservation, semantic reconciliation, existing PR gates and the original audit coverage gaps. Next 10 tasks: [the original ranked queue](../2026-10-18-lost-work-audit/RECOVERY_QUEUE.md). Completion percentage: not assigned; this is one verification checkpoint, not a complete recovery or feature qualification.
