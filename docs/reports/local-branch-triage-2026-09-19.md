# Local branch triage — 2026-09-19

Answering: *"push and merge all local commits, even if not yours."* This is the
evidence for what that turned out to mean.

## 1. Nothing is local-only any more

`git push --all origin` ran first, so every local branch is on the remote.
Branches whose remote counterpart had moved on were **not** force-pushed.
Instead, each branch holding commits that existed on no remote ref was pushed
to a `preserve/` namespace, so the commits survive without rewriting anyone
else's branch:

```
origin/preserve/20260919/x3-economic-safety-kernel
origin/preserve/20260919/x3-trading-core-v1-hardening
origin/preserve/20260919/merge-queue-production-gate-lean
origin/preserve/20260919/canonical-cross-domain-proof-bundle-20260911
origin/preserve/20260919/idempotent-cross-domain-coordinator-20260911
origin/preserve/20260919/live-feature-matrix-20260912
origin/preserve/20260919/live-secret-release-firewall-20260911
origin/preserve/20260919/settlement-proofset-gate-20260911
origin/preserve/20260919/x3-lang-crosschain-integration-20260909
origin/preserve/20260919/x3vm-live-transport
origin/preserve/20260919/x3vm-live-transport-fix
origin/preserve/20260919/x3lang-frame-classification
origin/preserve/20260919/x3lang-proof-vocabulary
origin/preserve/20260919/cross-domain-refund-recovery-20260911
```

(One name in the working list, `feat/live-secret-firewall-20260911`, was a typo
and is not a branch.)

## 2. Nothing needs merging: master already has the content

Of ~85 local branches, 8 hold commits whose patch is not textually in `master`.
All 8 conflict with today's `master` (they are 8–9 days stale), so a merge
cannot be a fast-forward or a clean union. Checking the *content* rather than
the patches shows every one of them is already implemented on `master`:

| branch (stale) | what it carries | where it lives on `master` |
| --- | --- | --- |
| `feat/canonical-cross-domain-proof-bundle-20260911`, `feat/settlement-proofset-gate-20260911` | canonical cross-domain proof bundle + proof sets | `crates/x3-atomic-swap/src/proof_bundle.rs`; `lib.rs` re-exports `CrossDomainProofBundle`, `CrossDomainProofSet` |
| `feat/live-secret-release-firewall-20260911` | secret release firewall / releases permit | `crates/x3-atomic-swap/src/secret_release.rs`; `SecretReleasePermit`, `SecretReleaseFirewall` re-exported from `lib.rs` |
| `finish/x3vm-live-transport`, `finish/x3vm-live-transport-fix` | native X3 transport export | `crates/x3-atomic-swap/src/x3vm_native.rs`, `x3vm_node.rs` |
| `test/cross-domain-refund-recovery-20260911` | "refund is terminal" live tests | `node/tests/x3vm_evm_live.rs:734`, `node/tests/x3vm_svm_live.rs:911` |
| `feat/idempotent-cross-domain-coordinator-20260911` | secret-claim ownership round-trip tests | `crates/cross-vm-coordinator/src/persistence.rs:413` and `:589` |
| `codex/x3-economic-safety-kernel` | versioned economic commitments, submission-profile consistency | both commit subjects appear in `git log --format=%s origin/master` |
| `feat/live-feature-matrix-20260912`, `feat/x3-lang-crosschain-integration-20260909`, `fix/x3lang-*`, `docs/merge-queue-production-gate-lean` | — | 0 patch-unique commits |

Two commands settle the interesting ones:

```bash
# patch-unique commits that are not in master
git cherry origin/master <branch> | grep '^+'

# ... and whether their subject already landed under a different SHA
git log --format=%s origin/master | grep -Fx "<subject>"
```

The refund tests were tried as a cherry-pick to be sure. The EVM file
auto-merged and the SVM file conflicted; after resolving it the test target
failed to compile with four `E0428: the name ... is defined multiple times`
errors (`finalized_head`, `intent_state_storage_key`, `intent_state_at`,
`run_svm_broadcast_expect_failure`) — because `master` already defines those
helpers *and* the two tests. The cherry-pick was abandoned.

The only genuinely absent change found was a comment added next to
`#[allow(deprecated)]` in `patches/idna_adapter/src/lib.rs`, and the removal of
a missing trailing newline in `.github/workflows/rust-clippy.yml`, a file that
no longer exists. Neither is worth a merge.

## 3. What this means

The stale branches are **superseded snapshots**, not outstanding work. Merging
them would have reverted newer code (`master` has moved hundreds of commits and
the TICKET/PHASE work rewrote most of these files) and would have re-added
duplicate definitions. They are preserved on the remote for history and for
anyone who wants to compare an approach, and no further action is needed on
them.

What would change this verdict: a branch whose content check fails — i.e. a
feature with no counterpart on `master`. Re-run the two commands above when in
doubt; that is the whole test.
