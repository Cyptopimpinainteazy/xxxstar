# Full branch reconciliation (2026-09-21/22)

Three-pass cleanup and reconciliation requested against `origin/master`. This
report covers passes 2 and 3; pass 1 (tracked-artifact cleanup) landed as
PR #414.

Analysis was run against `origin/master` at commit `a9f3d758e` (Pass 3's
patch-id computation and ancestry check). Master moves fast in this repo —
multiple concurrent agents land PRs continuously, including several times
during this same session — so by the time this is read, master's tip will be
further ahead. Nothing found here is expected to go stale: the finding is
that every branch's content already exists on master, and master only grows
from here, it doesn't lose history.

## Scope

`origin` carried **342 refs** (341 branches + `master` itself) at the start of
this pass, after `git fetch --prune` dropped stale local tracking refs left
over from branches already deleted upstream (a local, unpruned checkout had
previously shown 623). This is the reconstructed full list the task asked
for — the earlier "21" and the 311-branch inventory in the now-closed PR #388
were both partial: the 311-branch one classified ancestry only (177
ancestors / 134 "require review", never resolved), and did not account for
branches created/merged since 2026-09-20.

## Method

1. **Ancestry.** `git merge-base --is-ancestor origin/<branch> origin/master`
   for all 342 refs. **203** are ancestors — their commits are reachable from
   master's current tip.
2. **Patch-id equivalence for the rest.** For each of the remaining **139**
   branches, `git rev-list --no-merges origin/<branch> --not origin/master`
   gets its unique commits, `git patch-id --stable` hashes each one's diff,
   and each hash is checked against the patch-id set of master's full
   history (921 unique patch-ids across 928 non-merge commits). **All 139
   came back with zero novel patch-ids** — every one of their unique commits'
   diffs already exists somewhere in master's history, just under different
   commit hashes (rebased, cherry-picked, or independently re-authored).
3. **Caveat, checked explicitly.** Ancestry and patch-id equivalence describe
   *history*, not current file content — a branch can be an ancestor (or
   patch-id-equivalent) whose specific change was later reverted by a
   subsequent master commit. This is confirmed to affect exactly two named
   cases, both already identified by the prior `.ai/reports/branch-triage-verified-20260921.md`
   pass (which read the actual code, not just diffs) and re-verified directly
   here:
   - `codex/x3-economic-safety-kernel` (+ its `econsafety-kernel`,
     `preserve/20260918/codex-x3-economic-safety-kernel`,
     `preserve/20260919/x3-economic-safety-kernel` copies): an ancestor of
     master, but master's current tree does **not** carry the risk ceilings
     this branch added — removed deliberately in later work.
   - `wip/x3lang-preserve-packets-and-arbitrage-20260919` (+
     `wip/x3lang-arb-graph-filter-20260919`): patch-id-equivalent, but
     master's tree has **no** `arbitrage.rs` anywhere (`git ls-tree -r
     origin/master | grep arbitrage.rs` is empty) — this is a second,
     alternate PHASE 37 implementation kept off master on purpose
     (commit `1bfaa5243`, "kept off master on purpose").
   No other exceptions of this kind were found in a further spot-check of 9
   more distinctively-named branches (`mainnet/*`, `land-durable-recovery-v2`,
   `add-slippage`, `your-task-branch`, `fix-x3lang-python`,
   `reapply-features`) — all consistent with their bucket.
4. **`preserve/*` and `archive/*` branches (60 total)** are, by their own
   naming and the prior triage's finding, intentional point-in-time snapshots
   — not proposals to merge forward. They're listed below for completeness
   with whatever bucket their content falls into, not flagged as pending work.

## Result

**Every one of the 342 refs resolves to MERGED or ALREADY INCLUDED.** No
branch was found carrying functionality that is both (a) genuinely absent
from master's current tree and (b) not one of the two deliberate-exclusion
cases above. The prior `.ai/reports/branch-triage-verified-20260921.md` pass
had already found the only two real gaps across all branch history
(`on_vote` counting unverified votes; `x3-foundry-core` unable to deploy for
real) — both landed as `#403`/`#404` before this pass started.

## Pass 2: the five-branch integration (PR #388)

PR #388 (draft, `integration/reconcile-20260920`) consolidated
`deps-mod-test`, `fix/fake-metrics-honesty`,
`fix/foundry-core-creator-fee-remainder`, `fix/foundry-real-evm-deploy`,
`fix/private-mempool-share-validation-hardening`, and conflicted with current
master in exactly one place (`crates/x3-foundry-core/src/deployer.rs`, a
one-line whitespace difference — not a real conflict). Direct tip-to-tip
diffs (not the three-dot/history diff, which is misleading once master
re-derives the same content independently) found:

| branch | disposition | evidence |
| --- | --- | --- |
| `fix/fake-metrics-honesty` | already included | `git diff origin/master origin/fix/fake-metrics-honesty -- crates/flash-finality/src/lib.rs` empty — landed via `#403` |
| `fix/foundry-core-creator-fee-remainder` | already included | diff empty across all 5 files it touches |
| `fix/private-mempool-share-validation-hardening` | already included | diff empty across `encryption.rs`/`lib.rs` |
| `fix/foundry-real-evm-deploy` | superseded | master is a strict superset (adds `x3-oracle`/`x3-foundry-revenue`/`revenue_bridge` this branch never had); real deploy landed via `#404` |
| `deps-mod-test` | partially landed | see below |

`deps-mod-test` carried 6 dependency bumps. 4 were safe (no lockfile
downgrades, clean `cargo check`/`clippy`/`test`) and landed as **PR #417**:
`actix-cors` 0.6→0.7, `pollster` 0.3→1.0, `jsonrpsee` 0.22→0.24,
`keccak-hash` 0.1→0.11 (needed a one-line `.as_bytes()` fix at two call
sites where `H256` no longer derefs to `&[u8]`). 2 were excluded after
`cargo check --workspace` proved them unsafe to carry over as-is:

- `primitive-types` 0.12.2→0.13.1 breaks `pallet-evm`'s `PrecompileSet` impl
  (that pallet pins its own older `primitive-types` via polkadot-sdk/frontier
  — bumping ours creates two incompatible versions of the same trait in the
  dependency graph).
- `k256` 0.13→0.14 makes `sign_digest_recoverable` require `digest 0.11` via
  `k256::sha2`, which conflicts with the `sha3`/`digest 0.10` used to build
  the Keccak256 digest passed into it — needs a coordinated `sha3` bump, not
  a standalone one.

No dependency version anywhere in the resulting `Cargo.lock` regressed below
what master already had — every changed package's full version set was
diffed, not just the lines that happened to move.

PR #388 and PR #369 (`fix/foundry-real-evm-deploy`'s standalone PR, same
disposition) were both closed with this table posted as a comment.

## Rust verification

Confirmed cargo/rustc are available on this host (`cargo 1.90.0`,
`rustc 1.90.0`) — the "unavailable in this environment" note on PR #388 no
longer applies here. Used for `cargo check --workspace`, `cargo clippy
--all-targets -- -D warnings`, and `cargo test` on every crate touched by
PR #417. One pre-existing, unrelated environment issue was found and ruled
out: `cargo test` on crates that link `openssl-sys` (e.g.
`x3-gpu-validator-swarm`) fails at the link step with `undefined reference:
__isoc23_strtol@GLIBC_2.38` from linuxbrew's OpenSSL build — reproduced
identically on unmodified master, so it is a host toolchain mismatch, not a
code or dependency-bump regression.

## Cleanup recommendation (not executed here)

Of the 342 refs, the branches below are safe to delete on GitHub — they add
nothing master doesn't already have, and are not one of the two deliberate-
exclusion cases or a `preserve/*`/`archive/*` intentional snapshot:

**251 branches** (of 342) are safe to delete. Full list:

<details><summary>251 safe-to-delete branches</summary>

- `add-slippage`
- `agents/pasted-text-processing`
- `agents/setup-instructions-request`
- `batch/20260918T175104Z`
- `batch/20260918T2015Z`
- `build/atomic-swap-no-std-port`
- `build/make-runtime-upgrade-real`
- `build/re-attest-603398a38`
- `build/re-attest-runtime-hash`
- `build/srtool-reproducible-wasm`
- `build/verify-reproducible-hash`
- `chore/exclude-superseded-governance`
- `chore/reqwest-0.12-rust-highs`
- `chore/wire-orphan-runtime-tests`
- `ci/cap-cargo-parallelism`
- `ci/consolidate-workflows-20260910`
- `ci/dedupe-live-triggers`
- `ci/local-ci-improvements`
- `ci/local-ci-known-red`
- `ci/local-ci-make-targets`
- `ci/local-ci-of-record`
- `ci/local-ci-pytest-gate`
- `ci/local-ci-release-gate`
- `ci/local-ci-runtime-variants`
- `ci/local-ci-slug-case`
- `ci/local-network-smoke`
- `ci/master-lineage-gates-20260908`
- `ci/path-filter-heavy-gates-20260910`
- `ci/production-gate-dispatch`
- `ci/route-live-gates-to-local-runner`
- `ci/route-more-workflows-self-hosted`
- `ci/runtime-hash-freshness`
- `ci/shared-cargo-target`
- `ci/stop-hosted-dead-triggers`
- `ci/svm-gate-rustup-shim`
- `ci/workflow-audit`
- `ci/x3-local-runner-smoke`
- `codex/chatgpt-mainnet-work`
- `codex/x3-trading-core-v1`
- `codex/x3-trading-core-v1-hardening`
- `dependabot/cargo/ark-ec-0.6.0`
- `dependabot/cargo/ark-ff-0.6.0`
- `dependabot/cargo/ark-std-0.6.0`
- `dependabot/cargo/frame-benchmarking-cli-54f11b1`
- `dependabot/cargo/frame-support-54f11b1`
- `dependabot/cargo/hex-literal-1.1.0`
- `dependabot/cargo/libloading-0.9.0`
- `dependabot/cargo/minicbor-2.3.0`
- `dependabot/cargo/pallet-scheduler-54f11b1`
- `dependabot/cargo/pallet-staking-54f11b1`
- `dependabot/cargo/pallet-sudo-54f11b1`
- `dependabot/cargo/redis-1.7.0`
- `dependabot/cargo/sc-basic-authorship-54f11b1`
- `dependabot/cargo/sc-cli-54f11b1`
- `dependabot/cargo/sc-consensus-aura-54f11b1`
- `dependabot/cargo/sc-rpc-54f11b1`
- `dependabot/cargo/sc-rpc-api-54f11b1`
- `dependabot/cargo/sp-keystore-54f11b1`
- `dependabot/cargo/sp-version-54f11b1`
- `dependabot/github_actions/actions-nonmajor-c5de02d4ff`
- `dependabot/github_actions/actions/checkout-7`
- `dependabot/github_actions/actions/configure-pages-6`
- `dependabot/pip/psycopg2-binary-gte-2.9.13`
- `deps-batch-test`
- `deps-mod-test`
- `deps-mod-test-v2`
- `deps/batch-low-risk`
- `design/x3lang-root-compiler-bridge`
- `docs/grant-readiness-truth-20260908`
- `docs/local-branch-triage`
- `docs/local-ci-verification-discipline`
- `docs/merge-queue-production-gate-lean`
- `docs/readiness-score-reconciliation`
- `docs/runtime-rehearsal-fidelity-note`
- `feat/arbitrum-message-decoding`
- `feat/base-message-decoding`
- `feat/base-send-message`
- `feat/canonical-cross-domain-proof-bundle-20260911`
- `feat/canonical-cross-domain-proof-bundle-20260911-pre-rebase-20260917`
- `feat/concurrent-coordinator-serialization-20260911`
- `feat/coordinator-attempt-ledger-20260911`
- `feat/coordinator-proof-vault-20260911`
- `feat/deterministic-recovery-reconciler-20260911`
- `feat/distributed-durable-fencing-20260911`
- `feat/evm-receipt-proofs`
- `feat/idempotent-cross-domain-coordinator-20260911`
- `feat/idempotent-cross-domain-coordinator-20260911-pre-rebase-20260917`
- `feat/live-feature-matrix-20260912`
- `feat/live-secret-release-firewall-20260911`
- `feat/proof-bound-coordinator-settlement-20260911`
- `feat/release-gate-runtime-rehearsal`
- `feat/secret-release-firewall-20260911`
- `feat/session-runtime-intent-binding-20260911`
- `feat/session-sharded-leases-20260911`
- `feat/settlement-chain-reconciliation-20260911`
- `feat/settlement-proofset-gate-20260911`
- `feat/settlement-submission-envelope-20260911`
- `feat/settlement-submission-outbox-20260911`
- `feat/solana-attestation-verifier`
- `feat/svm-validator-set-wiring`
- `feat/trading-core-bridge-integration`
- `feat/trading-core-v1-hardening-part2`
- `feat/trading-core-v1-registry`
- `feat/valkey-redis-fencing-backend-20260911`
- `feat/x3-lang-crosschain-integration-20260909`
- `feat/x3-settlement-reconciliation-rpc-20260911`
- `feat/x3lang-effects-guarantees`
- `feat/x3lang-guards-and-build-verification`
- `feat/x3vm-durable-recovery-20260911`
- `finish/x3vm-live-transport`
- `finish/x3vm-live-transport-fix`
- `fix-x3lang-python`
- `fix/agent-guard-bip39-2x`
- `fix/agent-guard-bip39-allow`
- `fix/agent-law-real-policy-and-live-tests`
- `fix/atomic-kernel-bond-accounting`
- `fix/bitcoin-spv-policy-claim`
- `fix/bridge-evm-transfer-content-verification`
- `fix/btc-vault-honest-deposit-path`
- `fix/build-rs-ptr-arg`
- `fix/chain-emitters-and-refusals`
- `fix/chronos-mempool-connections-refuse`
- `fix/control-plane-vote-window-validation`
- `fix/coordinator-durable-authority-shared-store`
- `fix/coordinator-gate-offline`
- `fix/coordinator-settlement-outbox-partial-move`
- `fix/custody-authorization-required`
- `fix/dev-variant-runtime-tests`
- `fix/do-not-cancel-master-gates`
- `fix/embedded-runtime-variant-guard`
- `fix/evm-lifecycle-honour-target-dir`
- `fix/evm-live-foundry-anvil-collision`
- `fix/fake-metrics-honesty`
- `fix/forge-deps-idempotent`
- `fix/foundry-deploy-is-simulated`
- `fix/gateway-attestation-signatures`
- `fix/governance-approval-tracking`
- `fix/invariant-registry-gate`
- `fix/js-sdk-test-gate`
- `fix/live-x3vm-lifecycle-gate`
- `fix/loom-concurrency-gate`
- `fix/loom-runner-rustup`
- `fix/master-trading-core-compile-break`
- `fix/mobile-sdk-security-claims`
- `fix/mock-adapter-gating-and-workspace-membership`
- `fix/no-fabricated-proof-grade`
- `fix/node-inspect-and-chain-smoke`
- `fix/pallet-x3-control-member`
- `fix/panic-scan-build-targets`
- `fix/panic-scan-test-files`
- `fix/private-mempool-real-shamir-threshold`
- `fix/private-mempool-share-validation-hardening`
- `fix/private-mempool-threshold-crypto`
- `fix/production-gate-prerequisites`
- `fix/release-gate-target-dir`
- `fix/reqwest-0.12-migration`
- `fix/reusable-call-concurrency`
- `fix/risk-scorer-trading-core-v1`
- `fix/runtime-evm-envelope-validation`
- `fix/runtime-hash-helper`
- `fix/runtime-orphan-placeholder-tests`
- `fix/runtime-variants-tuples-96`
- `fix/self-hosted-apt-step`
- `fix/settlement-proofs-fail-closed`
- `fix/settlement-test-target-compiles`
- `fix/shipped-genesis-boots`
- `fix/stress-harness-rate-limit`
- `fix/svm-htlc-native-custody`
- `fix/svm-htlc-native-custody-master`
- `fix/svm-lifecycle-ignore-ambient-target-dir`
- `fix/swarm-determinism-fail-closed`
- `fix/testnet-spec-builder`
- `fix/trading-core-v1-mainnet-and-audit-blindness`
- `fix/vacuous-tests-assert-something`
- `fix/verifier-fail-closed`
- `fix/wallet-cli-honest-and-member`
- `fix/wasm-std-feature-leak`
- `fix/workspace-membership-batch-1`
- `fix/workspace-membership-batch-10`
- `fix/workspace-membership-batch-2`
- `fix/workspace-membership-batch-3`
- `fix/workspace-membership-batch-4`
- `fix/workspace-membership-batch-5`
- `fix/x3-evolution-fail-closed-contract`
- `fix/x3-mobile-sdk-member`
- `fix/x3-oracle-clippy`
- `fix/x3-pq-member`
- `fix/x3-sidecar-lib-target`
- `fix/x3lang-frame-classification`
- `fix/x3lang-guard-evaluation`
- `fix/x3lang-policy-honesty`
- `fix/x3lang-proof-required`
- `fix/x3lang-proof-vocabulary`
- `fix/x3lang-sound-invariants`
- `fix/x3lang-surface-warnings`
- `fix/x3vm-evm-live-gate`
- `fix/x3vm-svm-live-gate`
- `integration/reconcile-20260920`
- `land-durable-recovery`
- `land-durable-recovery-v2`
- `lint/atomic-swap-std-cfg`
- `mainnet/block-hook-panics`
- `mainnet/inspect-real-assets`
- `mainnet/panic-ratchet`
- `mainnet/production-genesis-path`
- `mainnet/release-artifacts`
- `mainnet/release-gate-integrity`
- `mainnet/runbook-reality`
- `mainnet/testnet-genesis-gate`
- `mainnet/validator-install-path`
- `mainnet/verified-proof-gate`
- `merge-into-master`
- `ops/drain-actions-queue-20260911`
- `pr-166-refresh`
- `pr-181-check`
- `pr132-work`
- `pr193-merge-master`
- `reapply-features`
- `rebase-310`
- `retarget-batch8`
- `salvage/evm-receipt-verify`
- `salvage/pr128-supervisor-tests`
- `salvage/trading-core-20260918`
- `salvage/x3lang-intent-bridge`
- `sec/remove-dead-libp2p-deps`
- `t5/fix-annotations-20260522-1458`
- `test/atomic-kernel-rollback-invariants`
- `test/cross-domain-recovery-matrix-20260911`
- `test/cross-domain-refund-recovery-20260911`
- `test/distributed-atomic-chaos-harness-20260911`
- `test/kernel-inflight-halt`
- `test/rehearsal-genesis-state`
- `test/runtime-migration-rehearsal`
- `test/runtime-storage-version-alignment`
- `test/runtime-variant-dryrun`
- `tmp/intent-bridge-rebase`
- `verify/master-deep`
- `verify/master-fast`
- `wip/consolidation-20260917/chatgpt-mainnet`
- `wip/consolidation-20260917/main`
- `wip/consolidation-20260917/pasted-text-processing`
- `wip/consolidation-20260917/recovered-usb-clone`
- `wip/consolidation-20260917/x3-lang-prototype-20260621`
- `wip/pr135-trading-core-hardening-rebased-20260917`
- `wip/pr165-on-no-std-master`
- `wip/pr165-settlement-proofset-gate-rebased-20260917`
- `wip/x3-evolution-simulator-20260918`
- `wip/x3lang-route-fallback-20260918`
- `x3/nostd-port-only`
- `x3lang-live-quotes`
- `your-task-branch`

</details>

Not included above: `preserve/*`/`archive/*` (60, intentional snapshots), the 6 deliberate-exclusion branches, `master` itself, and branches currently checked out in another agent's active worktree (`git worktree list`) — left alone to avoid disrupting in-progress work even though their content is also already on master.

**Not deleted in this pass** — branch deletion on GitHub is easy to script but hard to fully undo in spirit (250+ at once), so it's left for an explicit go-ahead rather than bundled into this reconciliation.

### `fix/*` (99 branches)

| branch | disposition | note |
|---|---|---|
| `fix/agent-guard-bip39-2x` | MERGED |  |
| `fix/agent-guard-bip39-allow` | ALREADY INCLUDED |  |
| `fix/agent-guard-path-separator` | MERGED |  |
| `fix/agent-law-real-policy-and-live-tests` | MERGED |  |
| `fix/atomic-kernel-bond-accounting` | MERGED |  |
| `fix/bitcoin-spv-policy-claim` | MERGED |  |
| `fix/bridge-evm-transfer-content-verification` | MERGED |  |
| `fix/btc-vault-honest-deposit-path` | MERGED |  |
| `fix/build-rs-ptr-arg` | MERGED |  |
| `fix/chain-emitters-and-refusals` | MERGED |  |
| `fix/chronos-mempool-connections-refuse` | MERGED |  |
| `fix/control-plane-vote-window-validation` | MERGED |  |
| `fix/coordinator-durable-authority-shared-store` | MERGED |  |
| `fix/coordinator-gate-offline` | MERGED |  |
| `fix/coordinator-settlement-outbox-partial-move` | MERGED |  |
| `fix/custody-authorization-required` | MERGED |  |
| `fix/dep-bumps-from-deps-mod-test` | ALREADY INCLUDED |  |
| `fix/dev-variant-runtime-tests` | MERGED |  |
| `fix/do-not-cancel-master-gates` | MERGED |  |
| `fix/embedded-runtime-variant-guard` | MERGED |  |
| `fix/evm-header-anchor` | MERGED |  |
| `fix/evm-lifecycle-honour-target-dir` | MERGED |  |
| `fix/evm-live-foundry-anvil-collision` | MERGED |  |
| `fix/external-chains-honest-adapters` | MERGED |  |
| `fix/fake-metrics-honesty` | ALREADY INCLUDED |  |
| `fix/flash-finality-verified-proposal` | MERGED |  |
| `fix/forge-deps-idempotent` | MERGED |  |
| `fix/foundry-core-creator-fee-remainder` | ALREADY INCLUDED |  |
| `fix/foundry-deploy-is-simulated` | MERGED |  |
| `fix/foundry-real-evm-deploy` | ALREADY INCLUDED |  |
| `fix/gateway-attestation-signatures` | MERGED |  |
| `fix/gateway-attested-path` | MERGED |  |
| `fix/governance-approval-tracking` | MERGED |  |
| `fix/invariant-registry-gate` | MERGED |  |
| `fix/js-sdk-test-gate` | MERGED |  |
| `fix/kernel-authority-bounds` | MERGED |  |
| `fix/live-x3vm-lifecycle-gate` | MERGED |  |
| `fix/loom-concurrency-gate` | MERGED |  |
| `fix/loom-runner-rustup` | MERGED |  |
| `fix/master-trading-core-compile-break` | ALREADY INCLUDED |  |
| `fix/mobile-sdk-security-claims` | MERGED |  |
| `fix/mock-adapter-gating-and-workspace-membership` | MERGED |  |
| `fix/no-fabricated-proof-grade` | MERGED |  |
| `fix/node-inspect-and-chain-smoke` | MERGED |  |
| `fix/orchestra-crate-root` | MERGED |  |
| `fix/pallet-x3-control-member` | MERGED |  |
| `fix/panic-scan-build-targets` | MERGED |  |
| `fix/panic-scan-test-files` | MERGED |  |
| `fix/private-mempool-real-shamir-threshold` | ALREADY INCLUDED |  |
| `fix/private-mempool-share-validation-hardening` | ALREADY INCLUDED |  |
| `fix/private-mempool-threshold-crypto` | MERGED |  |
| `fix/production-gate-prerequisites` | ALREADY INCLUDED |  |
| `fix/relayer-real-solana-verifier-351` | ALREADY INCLUDED |  |
| `fix/release-gate-target-dir` | MERGED |  |
| `fix/reqwest-0.12-migration` | MERGED |  |
| `fix/reusable-call-concurrency` | MERGED |  |
| `fix/risk-scorer-trading-core-v1` | MERGED |  |
| `fix/runtime-evm-envelope-validation` | MERGED |  |
| `fix/runtime-hash-helper` | MERGED |  |
| `fix/runtime-orphan-placeholder-tests` | MERGED |  |
| `fix/runtime-variants-tuples-96` | MERGED |  |
| `fix/self-hosted-apt-step` | MERGED |  |
| `fix/settlement-proof-set-gate` | MERGED |  |
| `fix/settlement-proofs-fail-closed` | MERGED |  |
| `fix/settlement-test-target-compiles` | MERGED |  |
| `fix/shipped-genesis-boots` | MERGED |  |
| `fix/stress-harness-rate-limit` | MERGED |  |
| `fix/svm-htlc-native-custody` | ALREADY INCLUDED |  |
| `fix/svm-htlc-native-custody-master` | ALREADY INCLUDED |  |
| `fix/svm-lifecycle-ignore-ambient-target-dir` | MERGED |  |
| `fix/swarm-determinism-fail-closed` | MERGED |  |
| `fix/testnet-spec-builder` | MERGED |  |
| `fix/trading-core-v1-mainnet-and-audit-blindness` | MERGED |  |
| `fix/typed-evm-receipts` | MERGED |  |
| `fix/vacuous-tests-assert-something` | MERGED |  |
| `fix/verifier-fail-closed` | MERGED |  |
| `fix/wallet-cli-honest-and-member` | MERGED |  |
| `fix/wasm-std-feature-leak` | MERGED |  |
| `fix/workspace-membership-batch-1` | MERGED |  |
| `fix/workspace-membership-batch-10` | MERGED |  |
| `fix/workspace-membership-batch-2` | MERGED |  |
| `fix/workspace-membership-batch-3` | MERGED |  |
| `fix/workspace-membership-batch-4` | MERGED |  |
| `fix/workspace-membership-batch-5` | ALREADY INCLUDED |  |
| `fix/x3-evolution-fail-closed-contract` | MERGED |  |
| `fix/x3-mobile-sdk-member` | MERGED |  |
| `fix/x3-oracle-clippy` | MERGED |  |
| `fix/x3-pq-member` | MERGED |  |
| `fix/x3-sidecar-lib-target` | MERGED |  |
| `fix/x3-swap-router-two-generations` | MERGED |  |
| `fix/x3lang-frame-classification` | ALREADY INCLUDED |  |
| `fix/x3lang-guard-evaluation` | MERGED |  |
| `fix/x3lang-policy-honesty` | MERGED |  |
| `fix/x3lang-proof-required` | MERGED |  |
| `fix/x3lang-proof-vocabulary` | ALREADY INCLUDED |  |
| `fix/x3lang-sound-invariants` | MERGED |  |
| `fix/x3lang-surface-warnings` | MERGED |  |
| `fix/x3vm-evm-live-gate` | MERGED |  |
| `fix/x3vm-svm-live-gate` | MERGED |  |

### `preserve/*` (44 branches)

| branch | disposition | note |
|---|---|---|
| `preserve/20260918/cargo/ark-ec-0.6.0` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/ark-ff-0.6.0` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/ark-std-0.6.0` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/frame-benchmarking-cli-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/frame-support-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/hex-literal-1.1.0` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/libloading-0.9.0` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/minicbor-2.3.0` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/pallet-scheduler-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/pallet-staking-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/pallet-sudo-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/redis-1.7.0` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/sc-basic-authorship-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/sc-cli-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/sc-consensus-aura-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/sc-rpc-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/sc-rpc-api-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/sp-keystore-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/cargo/sp-version-54f11b1` | ALREADY INCLUDED |  |
| `preserve/20260918/codex-x3-economic-safety-kernel` | ALREADY INCLUDED | snapshot of the above |
| `preserve/20260918/codex-x3-trading-core-v1-hardening` | ALREADY INCLUDED |  |
| `preserve/20260918/feat-live-feature-matrix-20260912` | ALREADY INCLUDED |  |
| `preserve/20260918/feat-live-secret-release-firewall-20260911` | ALREADY INCLUDED |  |
| `preserve/20260918/feat-settlement-proofset-gate-20260911` | ALREADY INCLUDED |  |
| `preserve/20260918/feat-x3-lang-crosschain-integration-20260909` | ALREADY INCLUDED |  |
| `preserve/20260918/finish-x3vm-live-transport` | ALREADY INCLUDED |  |
| `preserve/20260918/finish-x3vm-live-transport-fix` | ALREADY INCLUDED |  |
| `preserve/20260918/merge-into-master` | MERGED |  |
| `preserve/20260918/pip/psycopg2-binary-gte-2.9.13` | ALREADY INCLUDED |  |
| `preserve/20260918/test-cross-domain-refund-recovery-20260911` | ALREADY INCLUDED |  |
| `preserve/20260919/canonical-cross-domain-proof-bundle-20260911` | ALREADY INCLUDED |  |
| `preserve/20260919/cross-domain-refund-recovery-20260911` | ALREADY INCLUDED |  |
| `preserve/20260919/idempotent-cross-domain-coordinator-20260911` | ALREADY INCLUDED |  |
| `preserve/20260919/live-feature-matrix-20260912` | ALREADY INCLUDED |  |
| `preserve/20260919/live-secret-release-firewall-20260911` | ALREADY INCLUDED |  |
| `preserve/20260919/merge-queue-production-gate-lean` | MERGED |  |
| `preserve/20260919/settlement-proofset-gate-20260911` | ALREADY INCLUDED |  |
| `preserve/20260919/x3-economic-safety-kernel` | ALREADY INCLUDED | snapshot of the above |
| `preserve/20260919/x3-lang-crosschain-integration-20260909` | ALREADY INCLUDED |  |
| `preserve/20260919/x3-trading-core-v1-hardening` | ALREADY INCLUDED |  |
| `preserve/20260919/x3lang-frame-classification` | MERGED |  |
| `preserve/20260919/x3lang-proof-vocabulary` | MERGED |  |
| `preserve/20260919/x3vm-live-transport` | ALREADY INCLUDED |  |
| `preserve/20260919/x3vm-live-transport-fix` | ALREADY INCLUDED |  |

### `other/*` (39 branches)

| branch | disposition | note |
|---|---|---|
| `add-slippage` | ALREADY INCLUDED |  |
| `agents/pasted-text-processing` | ALREADY INCLUDED |  |
| `agents/setup-instructions-request` | ALREADY INCLUDED |  |
| `deps-batch-test` | ALREADY INCLUDED |  |
| `deps-mod-test` | ALREADY INCLUDED |  |
| `deps-mod-test-v2` | MERGED |  |
| `econsafety-kernel` | MERGED | same economic-safety-kernel work; ceilings removed from master deliberately |
| `finish/x3vm-live-transport` | MERGED |  |
| `finish/x3vm-live-transport-fix` | ALREADY INCLUDED |  |
| `fix-x3lang-python` | ALREADY INCLUDED |  |
| `integration/reconcile-20260920` | ALREADY INCLUDED |  |
| `land-durable-recovery` | MERGED |  |
| `land-durable-recovery-v2` | ALREADY INCLUDED |  |
| `lint/atomic-swap-std-cfg` | MERGED |  |
| `mainnet/block-hook-panics` | MERGED |  |
| `mainnet/inspect-real-assets` | MERGED |  |
| `mainnet/panic-ratchet` | MERGED |  |
| `mainnet/production-genesis-path` | MERGED |  |
| `mainnet/release-artifacts` | MERGED |  |
| `mainnet/release-gate-integrity` | MERGED |  |
| `mainnet/runbook-reality` | MERGED |  |
| `mainnet/testnet-genesis-gate` | MERGED |  |
| `mainnet/validator-install-path` | MERGED |  |
| `mainnet/verified-proof-gate` | MERGED |  |
| `merge-into-master` | MERGED |  |
| `ops/drain-actions-queue-20260911` | ALREADY INCLUDED |  |
| `pr-166-refresh` | MERGED |  |
| `pr-181-check` | ALREADY INCLUDED |  |
| `pr132-work` | ALREADY INCLUDED |  |
| `pr193-merge-master` | MERGED |  |
| `reapply-features` | ALREADY INCLUDED |  |
| `rebase-310` | ALREADY INCLUDED |  |
| `retarget-batch8` | ALREADY INCLUDED |  |
| `sec/remove-dead-libp2p-deps` | MERGED |  |
| `t5/fix-annotations-20260522-1458` | ALREADY INCLUDED |  |
| `tmp/intent-bridge-rebase` | MERGED |  |
| `x3/nostd-port-only` | MERGED |  |
| `x3lang-live-quotes` | MERGED |  |
| `your-task-branch` | ALREADY INCLUDED |  |

### `feat/*` (36 branches)

| branch | disposition | note |
|---|---|---|
| `feat/arbitrum-message-decoding` | MERGED |  |
| `feat/arbitrum-send-message` | MERGED |  |
| `feat/base-message-decoding` | MERGED |  |
| `feat/base-send-message` | MERGED |  |
| `feat/canonical-cross-domain-proof-bundle-20260911` | MERGED |  |
| `feat/canonical-cross-domain-proof-bundle-20260911-pre-rebase-20260917` | ALREADY INCLUDED |  |
| `feat/concurrent-coordinator-serialization-20260911` | MERGED |  |
| `feat/coordinator-attempt-ledger-20260911` | MERGED |  |
| `feat/coordinator-proof-vault-20260911` | MERGED |  |
| `feat/deterministic-recovery-reconciler-20260911` | MERGED |  |
| `feat/distributed-durable-fencing-20260911` | MERGED |  |
| `feat/evm-receipt-proofs` | MERGED |  |
| `feat/idempotent-cross-domain-coordinator-20260911` | MERGED |  |
| `feat/idempotent-cross-domain-coordinator-20260911-pre-rebase-20260917` | ALREADY INCLUDED |  |
| `feat/live-feature-matrix-20260912` | MERGED |  |
| `feat/live-secret-release-firewall-20260911` | MERGED |  |
| `feat/proof-bound-coordinator-settlement-20260911` | MERGED |  |
| `feat/release-gate-runtime-rehearsal` | MERGED |  |
| `feat/secret-release-firewall-20260911` | ALREADY INCLUDED |  |
| `feat/session-runtime-intent-binding-20260911` | MERGED |  |
| `feat/session-sharded-leases-20260911` | MERGED |  |
| `feat/settlement-chain-reconciliation-20260911` | MERGED |  |
| `feat/settlement-proofset-gate-20260911` | MERGED |  |
| `feat/settlement-submission-envelope-20260911` | MERGED |  |
| `feat/settlement-submission-outbox-20260911` | MERGED |  |
| `feat/solana-attestation-verifier` | MERGED |  |
| `feat/svm-validator-set-wiring` | MERGED |  |
| `feat/trading-core-bridge-integration` | MERGED |  |
| `feat/trading-core-v1-hardening-part2` | MERGED |  |
| `feat/trading-core-v1-registry` | MERGED |  |
| `feat/valkey-redis-fencing-backend-20260911` | MERGED |  |
| `feat/x3-lang-crosschain-integration-20260909` | MERGED |  |
| `feat/x3-settlement-reconciliation-rpc-20260911` | MERGED |  |
| `feat/x3lang-effects-guarantees` | MERGED |  |
| `feat/x3lang-guards-and-build-verification` | MERGED |  |
| `feat/x3vm-durable-recovery-20260911` | ALREADY INCLUDED |  |

### `ci/*` (26 branches)

| branch | disposition | note |
|---|---|---|
| `ci/cap-cargo-parallelism` | MERGED |  |
| `ci/consolidate-workflows-20260910` | ALREADY INCLUDED |  |
| `ci/cross-domain-gates` | MERGED |  |
| `ci/dedupe-live-triggers` | MERGED |  |
| `ci/foundry-crate-test-gate-110` | ALREADY INCLUDED |  |
| `ci/harden-self-hosted-jobs` | ALREADY INCLUDED |  |
| `ci/local-ci-improvements` | MERGED |  |
| `ci/local-ci-known-red` | MERGED |  |
| `ci/local-ci-make-targets` | MERGED |  |
| `ci/local-ci-of-record` | MERGED |  |
| `ci/local-ci-pytest-gate` | MERGED |  |
| `ci/local-ci-release-gate` | MERGED |  |
| `ci/local-ci-runtime-variants` | MERGED |  |
| `ci/local-ci-slug-case` | MERGED |  |
| `ci/local-network-smoke` | MERGED |  |
| `ci/master-lineage-gates-20260908` | ALREADY INCLUDED |  |
| `ci/path-filter-heavy-gates-20260910` | ALREADY INCLUDED |  |
| `ci/production-gate-dispatch` | MERGED |  |
| `ci/route-live-gates-to-local-runner` | MERGED |  |
| `ci/route-more-workflows-self-hosted` | ALREADY INCLUDED |  |
| `ci/runtime-hash-freshness` | MERGED |  |
| `ci/shared-cargo-target` | MERGED |  |
| `ci/stop-hosted-dead-triggers` | MERGED |  |
| `ci/svm-gate-rustup-shim` | MERGED |  |
| `ci/workflow-audit` | MERGED |  |
| `ci/x3-local-runner-smoke` | ALREADY INCLUDED |  |

### `dependabot/*` (23 branches)

| branch | disposition | note |
|---|---|---|
| `dependabot/cargo/ark-ec-0.6.0` | ALREADY INCLUDED |  |
| `dependabot/cargo/ark-ff-0.6.0` | ALREADY INCLUDED |  |
| `dependabot/cargo/ark-std-0.6.0` | ALREADY INCLUDED |  |
| `dependabot/cargo/frame-benchmarking-cli-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/frame-support-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/hex-literal-1.1.0` | ALREADY INCLUDED |  |
| `dependabot/cargo/libloading-0.9.0` | ALREADY INCLUDED |  |
| `dependabot/cargo/minicbor-2.3.0` | ALREADY INCLUDED |  |
| `dependabot/cargo/pallet-scheduler-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/pallet-staking-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/pallet-sudo-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/redis-1.7.0` | ALREADY INCLUDED |  |
| `dependabot/cargo/sc-basic-authorship-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/sc-cli-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/sc-consensus-aura-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/sc-rpc-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/sc-rpc-api-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/sp-keystore-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/cargo/sp-version-54f11b1` | ALREADY INCLUDED |  |
| `dependabot/github_actions/actions-nonmajor-c5de02d4ff` | ALREADY INCLUDED |  |
| `dependabot/github_actions/actions/checkout-7` | ALREADY INCLUDED |  |
| `dependabot/github_actions/actions/configure-pages-6` | ALREADY INCLUDED |  |
| `dependabot/pip/psycopg2-binary-gte-2.9.13` | ALREADY INCLUDED |  |

### `archive/*` (16 branches)

| branch | disposition | note |
|---|---|---|
| `archive/local-20260920/codex/x3-economic-safety-kernel` | ALREADY INCLUDED |  |
| `archive/local-20260920/codex/x3-trading-core-v1-hardening` | ALREADY INCLUDED |  |
| `archive/local-20260920/docs/merge-queue-production-gate-lean` | MERGED |  |
| `archive/local-20260920/feat/canonical-cross-domain-proof-bundle-20260911` | ALREADY INCLUDED |  |
| `archive/local-20260920/feat/idempotent-cross-domain-coordinator-20260911` | ALREADY INCLUDED |  |
| `archive/local-20260920/feat/live-feature-matrix-20260912` | ALREADY INCLUDED |  |
| `archive/local-20260920/feat/live-secret-release-firewall-20260911` | ALREADY INCLUDED |  |
| `archive/local-20260920/feat/settlement-proofset-gate-20260911` | ALREADY INCLUDED |  |
| `archive/local-20260920/feat/x3-lang-crosschain-integration-20260909` | ALREADY INCLUDED |  |
| `archive/local-20260920/finish/x3vm-live-transport` | ALREADY INCLUDED |  |
| `archive/local-20260920/finish/x3vm-live-transport-fix` | ALREADY INCLUDED |  |
| `archive/local-20260920/fix/x3lang-frame-classification` | MERGED |  |
| `archive/local-20260920/fix/x3lang-proof-vocabulary` | MERGED |  |
| `archive/local-20260920/test/cross-domain-refund-recovery-20260911` | ALREADY INCLUDED |  |
| `archive/pr126-pre-master-rewrite-20260909` | ALREADY INCLUDED |  |
| `archive/stale-x3lang-trading-wip-20260918` | ALREADY INCLUDED |  |

### `wip/*` (15 branches)

| branch | disposition | note |
|---|---|---|
| `wip/chatgpt-mainnet-attestation-20260918` | ALREADY INCLUDED |  |
| `wip/consolidation-20260917/chatgpt-mainnet` | ALREADY INCLUDED |  |
| `wip/consolidation-20260917/main` | ALREADY INCLUDED |  |
| `wip/consolidation-20260917/pasted-text-processing` | ALREADY INCLUDED |  |
| `wip/consolidation-20260917/recovered-usb-clone` | ALREADY INCLUDED |  |
| `wip/consolidation-20260917/x3-lang-prototype-20260621` | ALREADY INCLUDED |  |
| `wip/pr135-trading-core-hardening-rebased-20260917` | MERGED |  |
| `wip/pr165-on-no-std-master` | MERGED |  |
| `wip/pr165-settlement-proofset-gate-rebased-20260917` | MERGED |  |
| `wip/prompts-to-skills-20260918` | ALREADY INCLUDED |  |
| `wip/x3-evolution-simulator-20260918` | MERGED |  |
| `wip/x3lang-arb-graph-filter-20260919` | ALREADY INCLUDED | related arb-graph WIP, same on-purpose exclusion |
| `wip/x3lang-objectives-20260918` | MERGED |  |
| `wip/x3lang-preserve-packets-and-arbitrage-20260919` | ALREADY INCLUDED | second PHASE 37 arbitrage.rs implementation kept off master on purpose (commit 1bfaa5243) |
| `wip/x3lang-route-fallback-20260918` | MERGED |  |

### `test/*` (10 branches)

| branch | disposition | note |
|---|---|---|
| `test/atomic-kernel-rollback-invariants` | MERGED |  |
| `test/cross-domain-recovery-matrix-20260911` | ALREADY INCLUDED |  |
| `test/cross-domain-refund-recovery-20260911` | MERGED |  |
| `test/distributed-atomic-chaos-harness-20260911` | MERGED |  |
| `test/kernel-inflight-halt` | MERGED |  |
| `test/rehearsal-genesis-state` | MERGED |  |
| `test/runtime-migration-rehearsal` | MERGED |  |
| `test/runtime-storage-version-alignment` | MERGED |  |
| `test/runtime-variant-dryrun` | MERGED |  |
| `test/strict-posture-cross-domain` | MERGED |  |

### `docs/*` (7 branches)

| branch | disposition | note |
|---|---|---|
| `docs/grant-readiness-truth-20260908` | ALREADY INCLUDED |  |
| `docs/local-branch-triage` | MERGED |  |
| `docs/local-ci-verification-discipline` | MERGED |  |
| `docs/merge-queue-production-gate-lean` | ALREADY INCLUDED |  |
| `docs/merge-queue-production-gate-prereq-verified` | ALREADY INCLUDED |  |
| `docs/readiness-score-reconciliation` | MERGED |  |
| `docs/runtime-rehearsal-fidelity-note` | MERGED |  |

### `build/*` (6 branches)

| branch | disposition | note |
|---|---|---|
| `build/atomic-swap-no-std-port` | MERGED |  |
| `build/make-runtime-upgrade-real` | MERGED |  |
| `build/re-attest-603398a38` | MERGED |  |
| `build/re-attest-runtime-hash` | MERGED |  |
| `build/srtool-reproducible-wasm` | MERGED |  |
| `build/verify-reproducible-hash` | MERGED |  |

### `salvage/*` (5 branches)

| branch | disposition | note |
|---|---|---|
| `salvage/evm-receipt-verify` | MERGED |  |
| `salvage/foundry-real-evm-deploy` | MERGED |  |
| `salvage/pr128-supervisor-tests` | MERGED |  |
| `salvage/trading-core-20260918` | MERGED |  |
| `salvage/x3lang-intent-bridge` | ALREADY INCLUDED |  |

### `chore/*` (4 branches)

| branch | disposition | note |
|---|---|---|
| `chore/cleanup-tracked-artifacts-20260921` | ALREADY INCLUDED |  |
| `chore/exclude-superseded-governance` | MERGED |  |
| `chore/reqwest-0.12-rust-highs` | MERGED |  |
| `chore/wire-orphan-runtime-tests` | MERGED |  |

### `codex/*` (4 branches)

| branch | disposition | note |
|---|---|---|
| `codex/chatgpt-mainnet-work` | MERGED |  |
| `codex/x3-economic-safety-kernel` | MERGED | ancestor, but master later removed these risk ceilings deliberately — see .ai/reports/branch-triage-verified-20260921.md |
| `codex/x3-trading-core-v1` | MERGED |  |
| `codex/x3-trading-core-v1-hardening` | ALREADY INCLUDED |  |

### `batch/*` (2 branches)

| branch | disposition | note |
|---|---|---|
| `batch/20260918T175104Z` | MERGED |  |
| `batch/20260918T2015Z` | ALREADY INCLUDED |  |

### `verify/*` (2 branches)

| branch | disposition | note |
|---|---|---|
| `verify/master-deep` | MERGED |  |
| `verify/master-fast` | MERGED |  |

### `deps/*` (1 branches)

| branch | disposition | note |
|---|---|---|
| `deps/batch-low-risk` | ALREADY INCLUDED |  |

### `design/*` (1 branches)

| branch | disposition | note |
|---|---|---|
| `design/x3lang-root-compiler-bridge` | ALREADY INCLUDED |  |

### `merge/*` (1 branches)

| branch | disposition | note |
|---|---|---|
| `merge/all-work-c3095b883` | ALREADY INCLUDED |  |
