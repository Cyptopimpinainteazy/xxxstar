# Agent Memory — X3 Repo

## 2026-09-17 — Repo consolidation pass

### Facts to remember
- `origin/master` is `21771aee9` (`ci: add manually-triggered x3 self-hosted runner smoke test (#186)`); local `chore/reqwest-0.12-rust-highs` sits 1 commit ahead of it at `a5a5e5796`.
- There are three live working trees of this repo: `/home/lojak/Desktop/xxxstar-main`, `/home/lojak/Desktop/xxxstar-chatgpt`, `/home/lojak/Desktop/xxxstar-main.worktrees/pasted-text-processing`.
- `origin` carries 65 refs, including `codex/x3-economic-safety-kernel` (PR #185, 45 commits) which never existed in the local clone before this pass.
- The `/tmp/x3-*` worktrees are gone; their commits survive on normal branches (`fix/svm-htlc-native-custody`, `pr-166-refresh`, `finish/x3vm-live-transport-fix`, `test/cross-domain-recovery-matrix-20260911`, `codex/x3-trading-core-v1`).
- `/home/lojak/Desktop/Recovered-from-USB/Desktop/xxxstar-main` is a second clone on branch `main`; all of its commits already exist in this repo.

### Decisions made this session
- Captured uncommitted work as WIP commits on `wip/consolidation-20260917/*` refs rather than committing onto the working branches, so no branch history was rewritten and no working tree or index was modified.
- Kept the main tree's artifact churn (`apps/dashboard/dist`, `site/` output) in the snapshot for safety but left it uncommitted on a branch, since mixing build output with the reqwest migration would produce an unreviewable commit.
- Pruned 7 worktree registrations whose directories no longer exist; no refs or commits were removed.

### Canonical paths chosen
- `xxxstar-main` is the canonical clone; the other two worktrees are secondary and should be retired once their uncommitted work is landed.
- Snapshot refs `wip/consolidation-20260917/{main,chatgpt-mainnet,pasted-text-processing}` are the authoritative copies of that uncommitted work.

### Known blockers
- `/home/lojak/Desktop/xxxstar-main/.git` is read-only inside the sandbox, so every git write (fetch, commit, ref update, prune) requires an escalated command.
- Network access is blocked in-sandbox; `git fetch`/`gh` need escalated execution.
- Pushing the local-only branches to `origin` requires explicit user approval — it is an external write to the shared GitHub repo.

### Dead ends to avoid
- `mktemp -t` for a scratch git index fails in this environment; use an explicit `/tmp/...idx` path.
- `git branch -r --contains` on a tip that is a few commits ahead of its origin branch reports "not on origin" even when the base is pushed — check the ahead count, not just containment.

### Environment notes
- Inventory + snapshot of the 23k-file main tree took ~90s; allow long timeouts for `git add -A` and full-tree `git status` on these clones.

### Next task seed
- Push `fix/svm-htlc-native-custody`, `pr-181-check`, `test/cross-domain-recovery-matrix-20260911`, `feat/idempotent-cross-domain-coordinator-20260911`, `feat/canonical-cross-domain-proof-bundle-20260911`, `fix/svm-htlc-native-custody-master`, `agents/setup-instructions-request` and the 3 `wip/consolidation-20260917/*` refs to `origin`, then split the main tree's reqwest migration from artifact churn into separate commits.

## 2026-09-17 — Remote durability pass (completed)

### Facts to remember
- All 26 local branches now have tips reachable from `origin`; `origin` holds 76 branches.
- Two PR branches were force-rebased upstream, so their local lineages were preserved under `-pre-rebase-20260917` names: `feat/canonical-cross-domain-proof-bundle-20260911` (3 commits) and `feat/idempotent-cross-domain-coordinator-20260911` (8 commits, all subjects already present on the live branch).
- `wip/consolidation-20260917/*` (5 refs) are preservation branches: `main`, `chatgpt-mainnet`, `pasted-text-processing`, `recovered-usb-clone`, `x3-lang-prototype-20260621`.
- The recovered-USB clone was a stale copy: files such as `crates/x3-atomic-swap/src/bitcoin_htlc.rs` and `config/rpc-endpoints.toml` are byte-identical to today's master, so its 244 "dirty" files were mostly the delta between an old HEAD and later upstream commits.
- The standalone x3-lang prototype (7 crates, 169KB) exists in no branch of this repo; it is now preserved at `prototypes/x3-lang-20260621/`.

### Decisions made this session
- Never force-pushed: upstream-rebased branches were preserved under new ref names instead of rewriting the live PR branches.
- Pushed preservation work to `wip/*` refs rather than `master`, so no unreviewed content entered the default branch.
- Excluded nested `.git` directories and `target/`/`node_modules/` when importing the prototype.

### Dead ends to avoid
- `git push origin <branch>` fails as non-fast-forward when a PR branch was rebased upstream — push the local lineage to a new ref name instead of retrying.

### Next task seed
- Fast-forward local `master` to `origin/master` and triage the ~45 open PRs, starting with the two branches whose upstream was rebased.

## 2026-09-17 — Coordinator PR stack landed on master

### Facts to remember
- The stacked coordinator/settlement series #164 and #166-#180 is merged; `origin/master` was at `e6f9267cc` when the pass finished.
- `crates/cross-vm-coordinator` is EXCLUDED from the root workspace by the stack's own commits; `cargo check --workspace` therefore does NOT compile it. Its only real gate is `cargo test --manifest-path crates/cross-vm-coordinator/Cargo.toml`, which is what the coordinator CI runs.
- Two real defects were hiding in the stack: `DurableLeaseAuthority::shared_store()` was defined on the wrong type (with `#[derive(Clone)]` demanding `S: Clone`), and four `settlement_outbox.rs` transitions borrowed a partially moved record. Both are fixed on master (#188, #191).
- The local GitHub runner is `home/lojak/actions-runner` registered as `x3star1`, run as `actions.runner.Cyptopimpinainteazy-xxxstar.x3star1.service`, labels `self-hosted,Linux,X64,x3`. Only the smoke/soak/fronend workflows target it; `benchmark-regression` needs an `x3-benchmark` label it does not have.
- `master` has no branch protection, and PR checks report `UNSTABLE`, so merges are not gated by GitHub today.
- A local build with the warm 64 GB `target/` dir: full `cargo check --workspace --offline` ≈ 3-5 min; gate every merge on it.

### Decisions made this session
- Merged the stack with MERGE COMMITS (not squashes) so dependents keep ancestry and rebase cleanly; this is what made the 15-PR chain tractable.
- Rebased each branch with `git rebase --onto origin/master <cut>`, where `<cut>` is `merge-base(<branch>, <previous branch's pre-push tip>)`. Subject-matching and prefix-scanning both misfire on this repo because many branches end with the same generic `build(workspace): …` commit.
- Forced pushes used `--force-with-lease` against the pre-fetch tip on every branch.

### Dead ends to avoid
- `cargo check ... | tail` swallows the exit code: one build failed and still merged (#169). Always use `set -o pipefail` or `PIPESTATUS[0]` when gating a push on a build.
- Deriving the cut point from commit subjects picked the branch's own tip once and pushed a no-op branch (#175); recover from this with `git push --force-with-lease=<branch>:<current> origin <old-tip>:<branch>`.
- Offline builds fail with "can't checkout ... offline mode" when a PR adds a dependency; rerun without `--offline` and commit the refreshed `Cargo.lock`.

### Next task seed
- Rebase the now-merged-but-conflicting PRs onto master: #165 (its base #164 just landed), then #130 -> #161/#163 for the X3VM transport chain; gate each on `cargo check --workspace` plus the standalone coordinator test command.

## 2026-09-17 — X3VM transport chain landed; #165 blocked

### Facts to remember
- PRs #130 (production X3VM transport boundary), #161 (cross-domain refund recovery) and #163 (fail-closed secret release firewall) are merged; `origin/master` finished at `6abd06b65`. Verified: `cargo check --workspace --offline` exit 0, `cargo test -p x3-atomic-swap` 667+31+44 passing.
- `crates/x3-atomic-swap` declares `no_std` gating in Cargo.toml (`std` feature) and `extern crate alloc`, but the crate's `#![cfg_attr(not(feature = "std"), no_std)]` line is NOT on master. Master's `secret_release.rs` (672 lines, public fields, redacting Debug) supersedes the earlier 469-line variant that the live-firewall branch carried.
- The transport branches carried an older duplicate of #130's lineage, so rebasing them replayed many already-merged commits; those conflicts were resolved by keeping master's version, which is the superseding one.
- `SecretReleasePermit` on master exposes public fields (`intent_id`, `evidence_domains`, `preimage`); the transport call sites use accessor methods. Accessors were added additively rather than reverting master's newer field layout.
- CI workflow conflicts recur in this pile: master carries the hardened `pull_request -> ubuntu-latest` / `push -> self-hosted x3` split in `production-gate.yml`, `rust-clippy.yml` and `x3vm-*-live-lifecycle.yml`. Never regress that split when resolving; a rebase also produced a duplicated `concurrency:` key in `rust-clippy.yml` that had to be de-duplicated.

### Decisions made this session
- Un-drafted PR #130 only after its rebased branch built green and its crate tests passed; draft state was otherwise respected.
- Resolved "belt" CI conflicts by keeping master's security hardening and folding in the PR's legitimate changes (longer timeouts, extra apt packages, `needs:` wiring) instead of picking one side wholesale.
- Kept the `prefer master` rule only for files that already exist on master, and verified the surviving PR work by inspecting the net diff and by compiling.

### Dead ends to avoid
- Auto-resolving every conflict to master silently dropped `SecretReleasePermit::intent_id()/preimage()` accessors; the compiler caught it (5 E0599 errors). After any bulk-resolution rebase, inspect `git diff --stat origin/master HEAD` and re-run the build before merging.
- Rebasing with a cut point taken from `merge-base(branch, old_base_tip)` works only when the tip is an ancestor; for branches cut from an older lineage, expect a long duplicated prefix and conflicts.

### Next task seed
- PR #165 (settlement proof-set gate) stays blocked on the runtime/`no_std` question: either port `x3-atomic-swap` to `no_std` (110 errors remain across ~25 files once declared) or extract the proof types into a small `no_std` crate. Its rebased work is preserved on `wip/pr165-settlement-proofset-gate-rebased-20260917`.

## 2026-09-17 — x3-atomic-swap is now genuinely no_std (PR #192)

### Facts to remember
- The `no_std` blocker is resolved: `#![cfg_attr(not(feature = "std"), no_std)]` plus the missing `alloc` imports (String/Vec/Box/ToString/ToOwned/BTreeSet, `vec!`/`format!`) are on master as `66819a8c0`; the crate still defaults to no features and the pallet consumes it with `default-features = false`.
- Two `no_std` traps in this crate: `f64::round()` is std-only (replaced with integer round-half-up in the scoreboard), and `alloc` has no `HashSet` (switched to `BTreeSet`). `sp_std::prelude` does NOT export `String` in this SDK — pallets need `use scale_info::prelude::string::String;` for wasm builds.
- The std-only binaries under `crates/x3-atomic-swap/src/bin/` must never get `use alloc::...` imports; they are separate std crates without `extern crate alloc`.
- Verified: `cargo check -p x3-atomic-swap --no-default-features` exit 0, `cargo check --workspace` exit 0, `cargo test -p x3-atomic-swap` 667+31 pass, `cargo test -p pallet-x3-settlement-engine` 114+23 pass.

### Known blockers
- PR #165 (`feat/settlement-proofset-gate-20260911`) now rebases onto master with the port and the whole workspace builds, but it fails 10 legacy pallet tests: its gate forbids finalizing on local claims alone (`assertion failed: matches!(final_state, IntentState::Finalized)` in `tests.rs`), which is exactly what those tests assert. The original PR branch never even compiled its test target, so this regression was never caught. Landing it needs either those tests updated to submit canonical proof sets or the gate re-scoped to cross-domain legs only — a product decision, not a mechanical fix. Preserved at `wip/pr165-settlement-proofset-gate-rebased-20260917` (`319f00187`).

### Next task seed
- Decide PR #165's semantics (gate scope vs legacy-test rewrite), then work the remaining draft PRs (#185, #135, #183, #129) and the 19 Dependabot PRs one build at a time.

## 2026-09-18 — Trading Core compile break repaired, economic safety kernel landed, CI root cause found

### Facts to remember
- `origin/master` was genuinely broken before this pass: `x3-lang/compiler/tests/test_ir_verifier.rs` built `TradingOperation::BeginAtomicTrade` with `policy_id:`, but the variant is `{ trade_id, policy: CompiledTradingPolicy }` (`compiler/src/ir.rs:379`). The whole x3-lang test target did not compile. The earlier handoff note that "X3IR does carry policy_id" referred to the *compiled policy* struct, not this variant.
- PR #196 (`fix/master-trading-core-compile-break`, tip `4ac8d9ae9`, authored by a parallel agent) restores machinery that master had dropped when `bfd0a7748` was merged: `invariant solvent`, real `max_gas` enforcement (`max_gas_asset`), and cross-chain asset-mixing rejection. Verified locally before merging: 403 x3-lang tests pass, clippy clean. Merged as `78a1dce99`.
- The compile break existed because `22391f5f4` (branch tip) is *ahead* of `bfd0a7748` (what got merged via #195): `git diff --stat 22391f5f4 bfd0a7748` is ~+119/-589 in x3-lang. #196 is effectively the re-land of those dropped lines.
- PR #185 rebased with `git rebase --onto origin/master 22391f5f4` **without conflicts** — but a clean rebase is not a correct merge. `CompiledTradingPolicy` gained 9 fields in #196, leaving 4 fixtures incomplete (E0063) in `test_ir_verifier.rs`, `trading_execution.rs` (x2), `economic_types.rs`. Fixed in `4aa67b8cc` using the repo's canonical lowered-policy shape.
- Nothing in the verifier or the VM execution path enforces `max_total_cost`, `max_price_impact_bps`, `max_mev_leakage_bps`, `quote_freshness_blocks`, `allowed_cost_kinds`, `allow_mint` or `allow_burn`; only the economic kernel's policy-strength comparison (`x3-lang/vm/src/economic.rs`) reads them. Fixture values therefore only need to be valid and intent-preserving.
- **CI root cause (repo-wide):** every GitHub-hosted job fails in ~2s with zero steps. The check-run annotation reads `The job was not started because your account is locked due to a billing issue.` 40 of 43 workflows use `ubuntu-latest`, so they are all inert on every branch, including `master`. This is an account-level lock, not a YAML or code problem.
- The local runner is unaffected: `x3star1` is `online` with labels `self-hosted,Linux,X64,x3`, and `x3-local-runner-smoke.yml` run `35295307485` (dispatched this session) completed **success**. Self-hosted jobs do not consume GitHub-hosted minutes, so the billing lock does not block them.
- Two workflow labels have no matching runner and therefore queue forever: `[self-hosted, x3-benchmark]` (`benchmark-regression.yml`) and `[self-hosted, linux, gpu]` (`swarm-tps-gpu-soak.yml`).
- Open Dependabot alerts on the default branch: 30 (12 high, 18 medium). The high ones are node-networking Rust crates: `libp2p-quic` → 0.13.1, `libp2p-gossipsub` → 0.49.4 and 0.49.3, `yamux` → 0.13.10, `rustls-webpki` → 0.103.13, `hickory-proto` (no patched version).
- Offline root-workspace checks need `SKIP_WASM_BUILD=1`; otherwise Substrate's wasm builder re-resolves against crates.io and dies with `Could not resolve host: index.crates.io` in-sandbox. The native workspace itself is clean.

### Verified state of master after this pass (`2c6062687`)
- `cargo test --workspace` in `x3-lang/` → 409 passed, 0 failed.
- `cargo test --manifest-path crates/cross-vm-coordinator/Cargo.toml` → 154 passed, 0 failed (still not wired into any CI).
- `SKIP_WASM_BUILD=1 cargo check --workspace --offline` → exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` in `x3-lang/` → exit 0; `cargo fmt --all -- --check` → clean.

### Dead ends to avoid
- Do not treat "rebase applied with no conflicts" as proof the merge is sound on a branch that predates a struct-growing PR. Always compile the test targets (`cargo test --no-run`/full test) after rebasing, not just `cargo check`.
- Do not attribute red CI to code: read a check-run annotation first. A 2-second, zero-step failure is infrastructure.
- Do not "fix" the billing lock by rewriting workflows to `self-hosted` on `pull_request`; the parallel agent's `ci/harden-self-hosted-jobs` branch owns that split and the prior hardening (PR runs on hosted, push runs on the local runner) must not be regressed.

### Next task seed
- CI: only the local runner can execute today. The durable fix is the account unlock (user action); the in-repo fix is routing `push`-to-master gates to `[self-hosted, Linux, X64, x3]` plus wiring `crates/cross-vm-coordinator` into a gate, since nothing runs it.
- Then clear the 12 high Dependabot alerts (libp2p/gossipsub/yamux/rustls-webpki are the consensus-critical ones; they must bump as a coherent family, as with the ark-* family).
- Leave PR #193 alone: another agent has it checked out in the main worktree with uncommitted `reqwest` bump edits.

### Follow-up in the same pass: orphaned work from the parallel agent (PR #197)
- `openclaw-agent` pushed `6cffd44d9` ("live price quotes and real slippage enforcement") to the head branch of **already-merged** PR #196. GitHub never extends a merged PR, so that commit could not reach master from that branch, and no new PR was opened for it. Detect this class of loss with: `git log --oneline origin/master..origin/<branch>` on every branch whose PR shows `MERGED`.
- Salvaged as PR #197: cherry-picked onto `2c6062687` as `8d98d1eff` into a new branch `x3lang-live-quotes` (the parallel agent's branch left untouched), verified, merged. Master is now `d5f6b50ab`.
- The feature adds a **required** `TradingHost::quote(&self, QuoteRequest) -> Result<QuoteResult, HostError>` so a host with no pricing source cannot claim to satisfy a declared slippage ceiling; `max_slippage_bps` previously compiled into the policy and was never measured against anything.
- Verification: 412 passed / 0 failed on the merged master (`master-d5f6b50ab-x3lang-test.log`), clippy exit 0, fmt clean; 406 passed on the original commit pre-rebase.
- `git cherry origin/master <branch>` is a cheap triage tool for the 32 unmerged remote branches, but read it carefully: master's history was partly rebuilt, so `+` lines can still be content that already landed under a different SHA (e.g. `codex/x3-trading-core-v1-hardening` shows 4 ahead / 0 unlanded).

### Live agents observed (do not collide with them)
- `openclaw-agent` is actively iterating the x3-lang trading core on `fix/master-trading-core-compile-break` ("fourth pass" as of 2026-09-17 19:31 -0600). Expect more host-interface commits; rebase-and-land rather than editing their branch.
- A second agent has `ci/harden-self-hosted-jobs` checked out in `/tmp/claude-1000/.../scratchpad/wf-audit`, and `chore/reqwest-0.12-rust-highs` (PR #193) checked out in the **main** worktree with uncommitted `Cargo.toml` edits across ~30 crates. Never `git checkout`/`stash` in the main worktree.

## 2026-09-18 (later) — the real local-node X3VM lifecycle gate now compiles and passes (PR #198)

### Facts to remember
- `node/tests/x3vm_live_lifecycle.rs` **did not compile on master**: `real_local_node_refund_before_timeout_fails_closed` was defined three times (E0428, merge artifact) and `SecretReleaseEvidence` was built with `rpc_quorum_agreed`/`refunded`, fields that no longer exist. The real struct is `{ lock, finality, rpc_quorum: RpcQuorumAttestation, refund: RefundObservation }`. Because the tests are `#[ignore]`d and every hosted CI job is billing-blocked, nothing ever surfaced it.
- `crates/x3-atomic-swap/src/x3vm_live.rs` carried the same stale evidence fields, but `lib.rs` declares `#[cfg(feature = "std")] pub mod x3vm_live;`. **`cargo test -p x3-atomic-swap` with default features does not compile that module** — only `--features std` (and the node, which enables `std`) hits it. A green crate test run is therefore not proof that the std-gated live paths compile.
- Pallet index lesson: the node compiles the **default** (no `dev`, no `frontier`, no `mainnet-rc1`) `construct_runtime!` variant, and hand-counting the declaration order gave 32 for the settlement engine while the truth is **31**. Probe with `<<Runtime as frame_system::Config>::PalletInfo as PalletInfo>::index::<pallet_x3_settlement_engine::Pallet<Runtime>>()`; a throwaway `node/tests/zz_index_probe.rs` answered it in ~20s. `Module(index: 31, error: [2])` = settlement-engine `InvalidIntentState`.
- The timeout→refund contract: a terminal `Refunded` state requires a **complete verified canonical refund proof set** for every escrowed leg (`all_required_operation_proofs`); BOTH `on_initialize` (automatic) and `refund_settlement` (explicit, documented fallback) enforce it. `on_initialize` re-schedules a deadline entry one block ahead while the condition is unsatisfied, and the intent must be in `Created | FundingInProgress | FullyFunded` for that to keep happening.
- The canonical escrow-domain descriptor for `ExternalChainId::X3Native` is `("x3-native", ProofVmType::X3Vm)` — while `LiveX3VmAdapter` labels its own chain `x3-local`. A proof bundle must carry the canonical descriptor or `bundle_matches_intent_domain` rejects the set.
- The secret-release firewall, in order: `intent.verify_hash()` (so the fixture must call `compute_hash()`), a releasable lifecycle status (`BothLocked`/`FinalityPending`/`Claimable`/`PreimageRevealed`/`ClaimSubmitted` — not `Pending`), requirements equal to the intent's own policy with `min_confirmations` derived from the policy level (`Bft` ⇒ 0), requirements covering the destination chain too, then per-leg evidence with `refund.refunded == false` and `rpc_quorum` whose `tx_id`/`block_hash` equal the lock's.
- The bare `X3VmAdapter::claim` path is gated client-side: it never reaches the chain, so it fails with `Proof missing: secret-release permit required...`, never `ExtrinsicFailed`. The node dev chain seals ~6 blocks/second.
- Sandbox reality: an in-sandbox `x3-chain-node --dev` dies with `Operation not permitted` binding `0.0.0.0:30379` / `127.0.0.1:9615`. All live-node tests must run with `sandbox_permissions: require_escalated`. The node also takes ~35-40s before its RPC answers.
- Pre-existing, unfixed: `cargo clippy -p x3-atomic-swap --features std --all-targets -- -D warnings` fails on unused `alloc` imports across ~15 untouched files plus a dead `transport()` accessor in `x3vm_live.rs`.

### Verified state (master `1035e6579`)
- `cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored --test-threads=1` → **3 passed / 0 failed** (159s) on a real dev node: lock → finalized claim with firewall permit, early refund fails closed, timeout → finalized `Refunded`.
- `cargo test -p x3-atomic-swap --features std` → **764 passed / 0 failed**.
- `cargo test -p x3-chain-node` → 49 passed (the one sandbox-only TCP failure passes unsandboxed).
- Merged master's tree is byte-identical to the verified branch (`git diff --stat origin/master 8dd7f36ab` is empty).

### Dead ends to avoid
- Do not add `finalized_index_of`-style helpers that scan *forward* from the current finalized head to locate an already-finalized extrinsic; capture the block hash returned by `wait_finalized`/the inclusion proof and look the index up in that block.
- When a patch's context text is identical in two tests, `apply_patch` edits the **first** match. The leg-1 verification landed in test 1 instead of test 3 and made the suite flaky. Anchor such patches with the test's unique string literal.
- Do not retry a rejected refund extrinsic in a loop: once the runtime auto-refunds, the intent is terminal and every retry returns `InvalidIntentState`.

### Next task seed
- Fix the pre-existing `--features std` clippy failures in `crates/x3-atomic-swap` (unused `alloc` imports + dead `transport()`), then wire `cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored` and `cargo test -p x3-atomic-swap --features std` into a self-hosted gate so these paths can never silently rot again.
- The EVM/SVM live lifecycles (`x3vm-evm-live-lifecycle.yml`, `x3vm-svm-live-lifecycle.yml`) are still unrun; the SVM one needs the Solana CLI toolchain.

## 2026-09-18 (EVM + SVM live gates) — PR #199, and the false-green class

### Facts to remember
- **A CI step can be a no-op false green.** `cargo test ... <name>` exits 0 when the filter matches nothing, so a workflow step naming a test that does not exist reports success while running zero tests. `x3vm-evm-live-lifecycle.yml` had exactly that: `real_x3vm_evm_refund_is_terminal_on_both_domains` exists nowhere in the repo. Every `--test <target> <name>` invocation in every workflow was scanned; that was the only phantom (beware parsing `2>&1` as a test name).
- Contract-level live gates, both self-contained, both verified passing on the local runner **without any code change**:
  - `X3-contracts/evm/test-live-lifecycle.sh` (real anvil + real `x3-evm-broadcast` + raw `cast` state reads) → **11 passed / 0 failed**.
  - `programs/svm/x3_atomic_swap/test-live-lifecycle.sh` (real `cargo build-sbf` + `solana-test-validator` + raw `getAccountInfo` byte decoding) → **15 passed / 0 failed**.
- The two *cross-domain* EVM tests failed for the same two contract reasons as the X3-native gate: the bare `X3VmAdapter::claim` is always rejected by the secret-release firewall (a permit is required), and a terminal `Refunded` needs a verified canonical refund proof set plus every leg escrowed. Fixed in PR #199 (`node/tests/x3vm_evm_live.rs`): permit built from real evidence for **both** declared domains (the X3 escrow and the EVM escrow just claimed), requirements mirroring the intent's policy (X3 BFT ⇒ 0 confirmations, Ethereum `Confirmations(1)` ⇒ 1), second X3 leg escrowed, canonical Refund bundle for the X3-native domain submitted through the real extrinsic, dispatch success verified, then the automatic `on_initialize` refund observed.
- The firewall matches domains by **chain family**: `chain_matches_kind` accepts a requirement id that equals `eth`/`ethereum` or starts with `ethereum-`, `sol`/`solana` or `solana-`, `x3` or `x3-`. The live adapters label their chains `anvil` / `x3-local` / the SVM RPC, so cross-domain evidence must be presented under a canonical label (`ethereum-anvil`, `x3-native`, `solana-mainnet`) for the requirement and the evidence to agree.
- Tooling gotchas for reproducing these gates locally:
  - `forge install` refuses to run in a worktree with uncommitted changes ("target or .gitmodules has existing changes"). Clone the pins from `X3-contracts/evm/foundry.lock` directly instead: `forge-std@v1.16.1` and `openzeppelin-contracts@v4.9.6` into `X3-contracts/evm/lib/`.
  - If `CARGO_TARGET_DIR` is exported, the EVM script's own `[ -x "$REPO_ROOT/target/release/x3-evm-broadcast" ]` check fails; symlink `<worktree>/target` → the shared warm target. The SVM script instead writes to its own `programs/svm/x3_atomic_swap{,/client}/target`, so **do not** redirect its target dir.
- Verified state after this pass: master `d3bcf8dfa`; `cargo test -p x3-chain-node` unsandboxed → **88 passed / 0 failed**; EVM cross-domain lifecycles → **2/2 PASS** (57s, 61s).

### Next task seed
- The SVM **cross-domain** tests (`node/tests/x3vm_svm_live.rs`) still fail for the same two reasons and want the same treatment as EVM: fixture hash/status, a permit covering X3 + `solana-mainnet`, second X3 leg escrowed, and the canonical X3-native Refund bundle. For Solana lock/finality evidence, read the real `slot`/`blockhash` with `getTransaction` (finalized commitment) on `X3_TEST_SVM_RPC`. The SVM workflow's env block is the recipe for the validator (`--rpc-port 18999`, dynamic ports, `--bpf-program`).

## 2026-09-18 (CI reality) — hosted CI dead, the local runner is the only executor, SVM gate green in CI

### Facts to remember
- **Hosted CI is still billing-locked.** Every GitHub-hosted run on master completes in 2-4 seconds with zero steps (`gh run view <id> --json jobs` shows `"steps": []`), including `production-gate`, `mainnet-readiness`, `Rust Clippy`, `Trivy`, `Semgrep`, `OSV-Scan`, `feature-matrix`. Do not read "queued"/"in_progress" from `gh run list` as progress: its run-level status lags job-level status. Use `gh run view <id> --json jobs` (and `~/actions-runner/_diag/Worker_*.log`) for the truth.
- **The x3 self-hosted runner is the only executor**, and it has **no usable sudo**: `sudo: a terminal is required to read the password; sudo: a password is required` (plus `no new privileges`). Any workflow step that unconditionally runs `sudo apt-get ...` fails on it. The toolchain is already provisioned there: `protoc` (libprotoc 36.0), `clang`, `cmake`, `openssl`, `jq`, `forge`, `anvil`, `cast`, `solana`, `cargo-build-sbf`.
- The reusable pattern for a dependency step that must work on both runners: check `command -v <tool>` for each requirement; if nothing is missing, just log it; else if `sudo -n true` works, install; else emit `::error::` and `exit 1`. Never silently continue without a required tool.
- **SVM live gate is green in CI**: run `35303169753` (master `cf76f28f4`) → `success`, all 17 steps, including the real `solana-test-validator` lifecycle (15 checks), the program + client unit tests, the SBF build, and **both** real X3VM-SVM cross-domain lifecycles. That was the first live cross-VM gate to execute and pass in CI for this repository. PRs #200 (test fixes) and #201 (apt step) made it possible.
- PR #202 landed the routing: `x3vm-live-lifecycle.yml` had **no push trigger at all** (its three real local-node lifecycles could only run on a PR, which hosted CI cannot run) and now pushes to master; both it and `x3vm-evm-live-lifecycle.yml` route `push`/`workflow_dispatch`/`workflow_call` to `[self-hosted, Linux, X64, x3]` while `pull_request` stays on `ubuntu-latest` (public repo + fork PRs ⇒ RCE risk). Master is `123dc8f52`.
- The single runner serialises jobs: the EVM workflow's `live-x3vm-evm` job went first, with `live-lifecycle` and the X3-native workflow queued behind it. The runner checkout (`~/actions-runner/_work/xxxstar/xxxstar`) keeps its own `target/` (~7 GB and warming), so repeat runs get faster.

### Next task seed
- Watch the runs triggered by `123dc8f52`: `x3vm-evm-live-lifecycle` (run `35304527539`; `real X3VM to Anvil lock claim lifecycle` was mid-flight, past Foundry install, contract deploy and Anvil start) and the queued `x3vm-live-lifecycle`, then fix whatever the first real CI failures reveal.
- With the EVM job confirmed green, the routing + idempotent-deps pattern can be extended to the remaining self-hostable gates (`rust-clippy`, `production-gate`, `mainnet-readiness`, the coordinator suite) so they execute at all.

## 2026-09-18 (CI round 2) — cross-domain gates green in CI; the reused-workspace trap

### Facts to remember
- **`x3vm-evm-live-lifecycle` job `real X3VM to Anvil lock claim lifecycle` → CI success** (run `35304527539`), including `Run real X3VM-EVM atomic lifecycle` and `Run real X3VM-EVM timeout/refund atomic lifecycle`. Together with the SVM run, both cross-domain gates now pass in real CI.
- **The self-hosted runner reuses one workspace across jobs** (`~/actions-runner/_work/xxxstar/xxxstar`), so state leaks between jobs. `forge install` refuses to run when `lib/` or `.gitmodules` is dirty ("cannot safely install dependency ... because the target or .gitmodules has existing changes"), which is exactly how the EVM contract job (`EVM HTLC live anvil lifecycle`) failed after its sibling job had installed the deps. PR #203 replaced both `forge install` calls with an idempotent `ensure_pinned` helper that clones/fetches the tags pinned in `foundry.lock` and never touches `.gitmodules`. Master is `66d7c525b`.
- **GitHub's job/run status lags badly for self-hosted jobs.** A job can be actively streaming console output (visible in `~/actions-runner/_diag/Worker_*.log`) while the API still reports `queued`. Use log freshness (`stat`/`tail` on the newest `Worker_*.log`) as the authority for "is it running".
- **I cannot see host processes from inside the exec sandbox**: `/proc/1` is `codex-linux-sandbox`, so `pgrep Runner.Listener` returns nothing even when the runner is healthy. Never conclude the runner is dead from that; check log freshness or the GitHub API instead. `systemctl`/`bus` access is also denied in-sandbox.
- The CI checkout's `target/` is separate from the working clones (`~/Desktop/xxxstar-main/target`, `/tmp/x3-mine/target`); it warms up across runs (was ~7 GB), which is why the first routed runs took ~15-30 min mostly compiling.

### Next task seed
- Read the result of the X3-native run (`35304527633`, was mid `Run live lock finalized claim lifecycle`) and of the re-triggered EVM run for `66d7c525b` (queued behind it). The EVM contract job should now pass; if it does, all three live workflows are green in CI.
- Then extend the routing + idempotent-deps pattern to `rust-clippy`, `production-gate` and `mainnet-readiness`, and add a `forge install`-free equivalent anywhere else a reused workspace can bite.

## 2026-09-18 (round 3) — all three live gates green in CI; an atomic-kernel fund-trapping bug

### Facts to remember
- **All three live cross-VM workflows now pass in real CI** (self-hosted `x3star1`): SVM (`35303169753`, 17/17 steps), EVM cross-domain (`35304527539`, both `Run real X3VM-EVM …` lifecycles), X3-native (`35304527633`, all three `Run live …` lifecycles).
- **`assert_noop!` is the right tool for proving a guard leaves no trace** — it asserts the storage root is unchanged, so it proves "no bundle, no extra bond reservation, no burned nonce" in one line.
- **`pallet-x3-atomic-kernel` rollback accounting was broken** (fixed in PR #204): `do_rollback_atomic_bundle` called `Currency::slash` before `Currency::unreserve`, and `slash` spends *free* balance before reserved balance, so for any submitter with free funds the penalty was charged twice — once from free balance and once by leaving `bond - unreserve_amount` stuck in reserve, with no path to release it (`RolledBack` bundles can never finalize or roll back again). The fix unreserves the whole bond first, then slashes the penalty out of it. Penalty policy unchanged (50% SubmitterCancelled, 10% ExecutionFailed/AccessSetViolation, 100% DeadlineExceeded). The extrinsic's doc comment had also promised "no slash" while the code applied 50% — the doc now matches the code.
- `mock.rs`'s `NoEconomicHalt` made the halt guard unobservable. The pattern that works: a `SwitchableEconomicHalt` reading an `AtomicBool`, plus an `economy_open()` guard that takes a `Mutex` (serialising halt tests) and clears the flag on `Drop` (so a panicking test cannot leak the halt into other tests).
- Balance assertions must be anchored *before* the reserve happens: capturing `free_balance` after `submit_atomic_bundle` makes the expected post-rollback value `free + (bond - penalty)`, not `free - penalty`.
- A `bundle_id` is `sha2(submitter ‖ block ‖ legs_hash)`, so re-submitting the same legs in the same block collides (`BundleAlreadyExists`) — tests must advance the block number.
- `FEATURE_REGISTRY.toml`'s `required_tests` are verified against real `fn` names by `scripts/check-readiness-consistency.sh`; `atomic_kernel` now lists `economic_halt_blocks_bundle_submission` and `economic_halt_does_not_trap_pending_bundle_funds`, and its score moved 40 → 45 with the remaining gaps recorded in `blockers`.

### Verified state (master 92338906f)
- `cargo test -p pallet-x3-atomic-kernel` → 82 passed / 0 failed; `cargo test -p pallet-x3-settlement-engine` → 138 passed / 0 failed; `SKIP_WASM_BUILD=1 cargo check --workspace` → exit 0; clippy on the pallet → exit 0; readiness consistency → PASS.

### Next task seed
- Poll the EVM re-run (`35306574985`) for the contract-level job, which is the first run exercising PR #203's `ensure_pinned` fix; the cross-domain job re-runs alongside it.
- Apply the same guard-and-balance test treatment to the other rollback reasons (ExecutionFailed 10%, AccessSetViolation 10%, DeadlineExceeded 100%) — only SubmitterCancelled is covered today.

## 2026-09-18 (round 4) — every rollback reason now has invariant coverage (pre-fix reproduction included)

### Facts to remember
- `pallet-x3-atomic-kernel` rollback coverage is complete (PR #205): `ExecutionFailed` (10%), `AccessSetViolation` (10%) and `DeadlineExceeded` (100%) join `SubmitterCancelled`, plus authorization and early-deadline negatives. **Pre-fix reproduction proved the tests are real guards**: against `HEAD~1`'s pallet source, `DeadlineExceeded` left `reserved_balance` at **10,000,000** (the entire bond) and `ExecutionFailed` at **1,000,000** (the penalty), while the two authorization tests passed either way. `assert_bond_settled_once` pins reserved == 0, the exact submitter loss, unchanged issuance, and a matching `BondSlashed` event.
- Rollback authorization matrix: `SubmitterCancelled` requires caller == submitter; `ExecutionFailed`/`AccessSetViolation` require caller == the *assigned executor* (`record.executor`); `DeadlineExceeded` requires `now > record.deadline_block` **and** caller ∈ {submitter, executor}. Bundle must be `Pending`/`Executing`. `assign_bundle_executor` accepts any `X3LangOrigin` (signed in the mock) but requires `now <= deadline_block`.
- `assert_noop!` remains the cleanest way to assert "rejected with no state change" for each abuse case.
- Registry (`atomic_kernel`): seven real `required_tests`, score 40 → 45 → 50, blockers now name only the genuinely external gaps (independent audit, multi-validator network exercise). `scripts/check-readiness-consistency.sh` PASS after each change.
- Worktree hygiene: committing while still on an already-merged branch strands the commit (the `gh pr merge` earlier had left me on `fix/atomic-kernel-bond-accounting`). After a merge, create the new branch *before* committing, or `git branch -f <new> HEAD` before pushing — pushing to a merged PR's branch is the orphan trap from PR #196.

### Verified state (master 9a5c1edd6)
- `cargo test -p pallet-x3-atomic-kernel` → 64 + 3 + 5 + 9 + 6 passed / 0 failed (87 total).
- clippy on the pallet → exit 0; readiness consistency → PASS.
- PRs merged this round: #203 (forge deps), #204 (bond accounting), #205 (rollback invariants).

### Next task seed
- Poll EVM run `35306574985`: the `EVM HTLC live anvil lifecycle` job is the first exercise of #203's `ensure_pinned`; the cross-domain job ahead of it re-runs with a rebuilt runtime (my kernel PRs changed the runtime, so the runner's checkout rebuilds — expect ~20-30 min per run).
- Then route `rust-clippy`, `production-gate` and `mainnet-readiness` to the self-hosted runner so they execute instead of failing in 3s, and consider a cargo cache key for the runner checkout so cross-commit runs stop rebuilding the runtime.

### Resolution (same round)
- **All three live cross-VM workflows are green in CI**: `x3vm-live-lifecycle` `35304527633` success, `x3vm-evm-live-lifecycle` `35306574985` success (**both** jobs, including the contract gate and its `Install forge dependencies (pinned via foundry.lock)` step that #203 fixed), `x3vm-svm-live-lifecycle` `35303169753` success (17/17 steps). Every live gate now executes and passes in real Actions on `x3star1`.

## 2026-09-18 (round 5) — master gates were being *cancelled*, not failing

### Facts to remember
- **`rust-clippy` and `production-gate` already route `push` to the self-hosted runner.** My earlier note that they "need routing" was wrong; their `pull_request` runs die on the hosted billing lock, but their push runs were reaching the runner and then being **cancelled**. `production-gate` additionally invokes the X3-native live gate via `workflow_call` (its job shows up as `native-x3vm-live / real local-node X3VM lifecycles`).
- **The cancellation bug (PR #206, master `bad90cc48`)**: 23 push-triggered workflows carried
  `group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}` with
  `cancel-in-progress: true`. On push, `pull_request.number` is empty so the group collapses to
  `<workflow>-refs/heads/master`, and every new commit **cancels the gate already running**. With one
  self-hosted runner and 20-30 minute gates, master gates could never finish while commits kept landing.
  Evidence: consecutive `completed/cancelled` push runs for Rust Clippy and production-gate at 04:20 and 04:37.
- The fix: `group: … || github.sha` (per-commit for pushes, so a newer commit cannot kill an older
  commit's run) and `cancel-in-progress: ${{ github.event_name == 'pull_request' }}` (PRs keep cancelling
  superseded pushes). **A run's concurrency group comes from the workflow file in the pushed commit**, so
  merging the fix immediately protected the gate that was already in flight — it stayed `in_progress` while
  the new commit's runs queued behind it. That is the live verification of the fix.
- Diagnosis heuristic that generalises: when a self-hosted job's steps never start, read the run's
  conclusion first — `cancelled` is a concurrency/queueing problem, `failure` with `"steps": []` is the
  billing lock, and only a started-but-red job is a code problem.

### Verified state (master bad90cc48)
- PRs merged: #203 (forge deps), #204 (bond accounting), #205 (rollback invariants), #206 (no cancelling
  master gates).
- Live check after the merge: run `35308164935` (production-gate → nested live lifecycle) stayed
  `in_progress`; `production-gate`, `x3vm-evm-live-lifecycle`, `x3vm-svm-live-lifecycle` and `Rust Clippy`
  for `bad90cc48` were all `queued` rather than cancelling each other.

### Next task seed
- Watch `Rust Clippy` on master now that it can run to completion: it is the repo's declared lint gate
  (`cargo clippy --workspace --all-targets -- -D warnings` plus per-crate feature matrix) and has never
  completed. Expect a real, possibly large, lint backlog — measure it before deciding scope.
- Consider a cargo cache key for the runner checkout so cross-commit gate runs stop rebuilding the runtime.

## 2026-09-18 (round 6) — x3-atomic-swap lints clean under `std` (PR #207)

### Facts to remember
- `cargo clippy -p x3-atomic-swap --all-targets --features std -- -D warnings` used to fail with **52 errors**; it is now clean. The node depends on that crate *with* `std`, and the repo's clippy workflow lints `x3-chain-node` with features, so this was on the declared lint gate's path.
- The pattern for no_std crates that also build with `std`: keep `alloc::*` imports for the no_std build and gate them out when std is on — `#[cfg(not(feature = "std"))]` on 49 whole-line imports. Deleting them would break `default-features = false` consumers (the settlement pallet).
- **This toolchain rejects attributes on nested use-tree items**: `use alloc::string::{String, #[cfg(not(feature = "std"))] ToString};` fails with `error: expected identifier, found '#'`. Split into two imports instead: plain `String`, then a separately gated `ToString`.
- Also removed `LiveX3VmAdapter::transport()`, a `pub(crate)` accessor with no callers (clippy `dead_code` fires under `--all-targets`).
- Verification must cover **both** feature configurations for this class of change: `--features std` (clippy clean) and `--no-default-features` (still compiles), plus the std test suite (764) and a workspace check.

### Verified state (master 6219cdc6a)
- PRs merged: #203 (forge deps), #204 (bond accounting), #205 (rollback invariants), #206 (no cancelling master gates), #207 (std-feature lints).
- After #206, no run has been cancelled; heavy gates queue behind each other on the single runner.
- The nested `native-x3vm-live` job inside `production-gate` was still running the X3-native lifecycles on the runner at hand-off (queue: production-gate, Rust Clippy, x3vm-evm-live-lifecycle, x3vm-svm-live-lifecycle for the two most recent commits).

### Next task seed
- Read the first completed `Rust Clippy` run (master `bad90cc48` or `6219cdc6a`): the `--workspace --all-targets` command pulls dev-dependencies (e.g. `actix-codec`) and has never completed, so its real backlog is still unknown.
- Watch the queue trade-off introduced by #206: each push adds ~4 heavy 20-30 min runs to a single runner. If the backlog becomes a problem, narrow *which* commits run heavy gates rather than re-enabling cancellation.

## 2026-09-18 (round 7) — persistent cargo target dir for the live gates (PR #208)

### Facts to remember
- **`actions/checkout` cleans the workspace at the start of every job**, so any build cache *inside* the checkout is destroyed between runs. That is why each live gate rebuilt the runtime from scratch (20-30 min per run). Fix: point `CARGO_TARGET_DIR` at a sibling of the checkout, `${{ github.workspace }}/../x3-cargo-target`, which survives the clean and is shared across jobs, workflows and commits.
- Scope matters: `x3vm-live-lifecycle` and both `x3vm-evm-live-lifecycle` jobs take it at job level; `x3vm-svm-live-lifecycle` takes it **only on the two cross-domain test steps**, because `cargo build-sbf` and the client build must keep their own target paths (the workflow asserts `programs/svm/x3_atomic_swap/target/deploy/x3_atomic_swap.so` exists).
- The SVM workflow's `rust-cache` step lists only `programs/svm/...` workspaces, so the root workspace (node + runtime + runtime WASM) was never cached there at all.
- A workflow run executes the workflow file **from the commit being run**, so runs queued for older commits keep the old (slow) definition; only runs for the merge commit and later benefit. The first such run is cold by construction.
- Master is `9d5a5127b`; PRs merged in this stretch: #203, #204, #205, #206, #207, #208.

### Next task seed
- Measure the before/after duration of a live-gate run that uses the shared target dir (compare with the 20-30 min cold runs) and confirm the directory actually persists between runs.
- Then read the first completed `Rust Clippy` run for the real workspace lint backlog.

## 2026-09-18 (round 8) — the release gate's own job had never run (PR #209)

### Facts to remember
- **`production-gate.yml`'s `gate` job — where `make guard`, `make test-all-pallets`, the srtool build and `make mainnet-check` live — had never executed.** It declares `needs: [native-x3vm-live, evm-live, svm-live]`, but only `native-x3vm-live` was ever scheduled, so the job was skipped for unmet needs and the run ended in `failure` with the real gate skipped.
- Root cause: **a workflow-level `concurrency:` block applies to a reusable-workflow call as well as to the standalone trigger.** `x3vm-evm-live-lifecycle.yml` and `x3vm-svm-live-lifecycle.yml` had one, and for the same commit the call collided with the standalone push run of the same workflow, so the call was never scheduled. `x3vm-live-lifecycle.yml` has no concurrency block and is the one call that always ran — a natural experiment that pinpointed it.
- Fix (PR #209, master `baa3fc387`): delete the `concurrency:` blocks from those two workflows. The single self-hosted runner serialises everything anyway, and #206 already established no-cancel semantics where they matter.
- Verification: production-gate run `35309656263` now schedules `native-x3vm-live`, `evm-live / real X3VM to Anvil lock claim lifecycle`, `evm-live / EVM HTLC live anvil lifecycle` and `svm-live / SVM HTLC live validator lifecycle` — the two `evm-live` and one `svm-live` jobs appear for the first time. Every prior run showed two jobs (one live job + skipped `gate`).
- `production-gate.yml` has **no `workflow_dispatch` trigger**, so the release gate cannot be run on demand (`gh workflow run production-gate.yml` returns 422). Worth adding.

### Next task seed
- Watch `production-gate` run `35309656263` through to `gate`: it is the first execution of `make guard`, `make test-all-pallets`, the srtool reproducibility build and `make mainnet-check` in CI. Expect real failures there and treat them as the highest-value backlog.
- Still unmeasured: the workspace clippy backlog (the local run keeps getting killed with the session; CI's `Rust Clippy` run will report it) and #208's timing benefit (needs a warm second run).

## 2026-09-18 (round 9) — the lint gate is satisfiable; the release gate is now dispatchable

### Facts to remember
- **The declared clippy gate passes locally** — my earlier worry about a large lint backlog was wrong. Correcting it matters because I had flagged it as a blocker on incomplete evidence. Measured on master `baa3fc387` with `SKIP_WASM_BUILD=1` (run escalated: cargo needed write access to `~/.cargo` to unpack dev-deps, which is why the earlier in-sandbox runs failed):
  1. `cargo clippy --workspace --all-targets -- -D warnings` → exit 0, 0 errors
  2. `-p x3-chain-runtime --all-targets --no-default-features --features std,mainnet-rc1` → PASS
  3. `-p x3-chain-node --all-targets --features mainnet-rc1` → PASS
  4. `-p x3-chain-node --all-targets --features mainnet-rc1,try-runtime` → PASS
  5. `-p x3-chain-node --all-targets --features mainnet-rc1,runtime-benchmarks` → PASS
  6. `-p x3-chain-node --all-targets --features mainnet-rc1,gpu-validator` → PASS
  These are the workflow's own six commands. The `--features std` failures fixed in #207 were the real lint debt.
- **`production-gate.yml` had no `workflow_dispatch` trigger** (`gh workflow run` returned HTTP 422). PR #210 added it; `gh workflow run production-gate.yml --ref master` now creates run `35310867732` (event `workflow_dispatch`). Dispatch is a non-PR event, so it routes to the self-hosted runner and keys its concurrency group per commit.
- Two diagnosis lessons reinforced: (a) an in-sandbox cargo failure that looks like "missing dependency" can just be a read-only `~/.cargo` — check the error chain before believing it; (b) `gh pr merge` can report "merge conflicts" on a PR that is `MERGEABLE` moments later — GitHub computes mergeability asynchronously, so re-check `gh pr view --json mergeable` before treating it as real.

### Verified state (master c3300a517)
- PRs merged in this stretch: #203, #204, #205, #206, #207, #208, #209, #210.
- Live gates all green in CI; release gate's job graph reachable; release gate dispatchable; lint matrix green locally.

### Next task seed
- Read `production-gate` run `35309656263` (push, all four live jobs scheduled) through to `gate`, and run `35310867732` (dispatch) — the first executions of `make guard`, `make test-all-pallets`, srtool reproducibility and `make mainnet-check`.
- Confirm `Rust Clippy` goes green in CI now that it can run and the code is clean.

## 2026-09-18 (round 10) — local CI made first-class (PR #211) + a second runner

### Facts to remember
- **A second runner instance is live**: `x3star2` (registered from `~/actions-runner-2`, labels `self-hosted,Linux,X64,x3`). The machine has **32 cores / 77 GB RAM**, so the bottleneck was the single runner, not capacity. Effect measured immediately: queue depth 10 → 7 and both runners busy (the new one picked up the queued `clippy` job within seconds of starting).
  - It is a **user process, not a service** (`setsid nohup ./run.sh &`), because installing a unit needs sudo which this environment lacks. Managing it: stop with `pkill -f "actions-runner-2/bin/Runner.Listener"`; restart after reboot with `cd ~/actions-runner-2 && setsid nohup ./run.sh >> runner.log 2>&1 &`; unregister with `./config.sh remove --token <removal-token>`. Installing it properly as a service needs `sudo ./svc.sh install`.
  - `${{ github.workspace }}/../x3-cargo-target` (from #208) resolves per runner, so each instance keeps its **own** warm target dir — parallel heavy builds do not fight over one cache.
- **`make guard` hung forever** — a step of `production-gate.yml`, so the release gate would have sat until its 90-minute timeout with no output. Cause: `scripts/agent_guard.py` decoded every tracked file as text and ran 43 regexes per line; the repo tracks 475 binary files (ParityDB tables) whose NUL-filled contents make an ambiguous pattern backtrack exponentially. Fixed in #211: skip files with a NUL in the first 8 KB (reading only the head), evaluate the 3 secret patterns first and the 40 allow-list patterns only on a hit, compile both lists once, and keep a 300s watchdog that **fails loudly naming the file** rather than hanging. Measured 4m03s → 27s; `make guard` ~20s.
- **`cargo fmt --all -- --check` failed on master** (123 diffs / 26 files) — fixed with a mechanical rustfmt sweep in #211.
- **`scripts/local-ci.sh`** now exists: one command that runs the gate suite, logs each gate under `.ai/runlogs/`, prints a summary table, and exits non-zero on any failure. `--live` adds the anvil/SVM contract gates, `--cross` adds the X3-native lifecycles, `--list` prints the gates. Verified: format 10s, guards 35s, readiness 17s, workspace check 121s all PASS; the run was interrupted during clippy by a session boundary.
- **Tracked junk found (not yet removed)**: `.rc4-runtime-upgrade-work/` — **475 files, 238 MB** of generated node chain data (ParityDB tables, binary) committed to the repo. It is what the guard choked on. Removal is destructive and needs an explicit decision.

### Verified state (master 4fa8b5cc5)
- PRs merged: #203-#211. Both runners online; queue draining in parallel.
- `make guard`, `cargo fmt --all -- --check`, `cargo check --workspace` and the readiness consistency check all pass locally on master.

### Next task seed
- Decide on `.rc4-runtime-upgrade-work/` (238 MB tracked generated data): remove + gitignore, or keep as rehearsal evidence.
- Watch `production-gate` (runs `35309656263` push, `35310867732` dispatch) reach `gate` — the first real execution of `make guard` (now ~20s instead of hanging), `make test-all-pallets`, srtool and `make mainnet-check`.

## 2026-09-18 (round 11) — local CI reachable from make; runner survives reboot

### Facts to remember
- `make local-ci` / `local-ci-live` / `local-ci-cross` / `local-ci-list` now wrap `scripts/local-ci.sh` (PR #213, master `792bf07c8`). Hosted CI is dead, so the local suite is the gate suite; it needed to be reachable from the repo's own entrypoint.
- **`x3star2` now survives reboots** via a crontab `@reboot` line (`@reboot cd $HOME/actions-runner-2 && setsid nohup ./run.sh >> runner.log 2>&1 &`), because installing a proper systemd unit needs sudo this environment lacks. Reversible with `crontab -e`/`crontab -r`; the proper long-term fix is `sudo ./svc.sh install`.
- Step-level visibility matters: each `functions.exec_command` runs in its own process namespace, so `ps` in one session **cannot see** processes started by another (this is also why "the runner is dead" was wrong earlier). Inside a session you can see its own children; across sessions only files are shared. For long jobs, start them detached with `PYTHONUNBUFFERED=1` and a log file, then poll the file.
- `make mainnet-check` is heavy: its `check_build` step runs `cargo build --release -p x3-chain-node`, which alone exceeds 12 minutes cold. A 900s timeout kills it mid-build (exit 124) — that is not a gate failure, just an under-budgeted timeout. Run it detached.
- `gh pr merge` intermittently returns "Pull Request has merge conflicts" for a PR that `gh pr view --json mergeable` reports as `MERGEABLE` seconds later — GitHub computes mergeability asynchronously; retry after a short pause.

### Verified state (master 792bf07c8)
- PRs merged: #203-#213. Both runners online and busy; queue hovers around 7 while long gates run.
- Local gate status on master: `make guard` ~20s PASS, `cargo fmt --all -- --check` PASS, `cargo check --workspace` PASS, readiness consistency PASS, clippy matrix 6/6 PASS.

### Next task seed
- Read `.ai/runlogs/mainnet-check-detached.log` (detached `make mainnet-check`) through sections 2-6; it is the first local execution of the release gate's own script.
- Decide on `.rc4-runtime-upgrade-work/` (475 files / 238 MB tracked generated chain data).

## 2026-09-18 (round 12) — in-flight halt contract covered (PR #214)

### Facts to remember
- The halt guard in `pallet-x3-atomic-kernel` only gates `submit_atomic_bundle`, so the *other half* of the contract was untested: a halt must not freeze work already in flight or trap its bond. `economic_halt_does_not_block_inflight_bundle_assignment` now asserts assignment still succeeds (bundle reaches `Executing` with the executor recorded), a new submission is still refused with `EconomicHaltActive`, and the in-flight bundle can still be rolled back with `reserved_balance == 0`.
- `atomic_kernel.required_tests` now lists **eight** real test functions; `scripts/check-readiness-consistency.sh` verifies each name resolves (PASS).
- Verified: `cargo test -p pallet-x3-atomic-kernel` → 88 passed / 0 failed; pallet clippy → exit 0.
- Capacity trend after adding the second runner: **queue 10 → 7 → 4** while both runners stay busy. The `make mainnet-check` release build is progressing (runtime rlib at 00:31, node rlib at 00:32).

### Next task seed
- Read `.ai/runlogs/mainnet-check-detached.log` sections 2-6 (build → chain-spec → test suites → srtool prereq → secret hygiene) and fix what it reports.
- Then close the "gate has never executed" gap in CI by watching `production-gate` runs `35309656263` (push) and `35310867732` (dispatch) reach their `gate` job.

## 2026-09-18 (round 13) — **`make mainnet-check` PASSES locally** (PR #215)

### Facts to remember
- **The release gate's own script now passes end to end on this machine**: `✅ mainnet_release_gate: PASS` plus readiness consistency PASS, **18 checks, 0 failures**. Sections: required docs ✓; release build of `x3-chain-node` **and** the runtime WASM (`target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm`) ✓; both `x3-local3-current-{plain,raw}.json` genesis specs valid + `production_config()` present ✓; critical suites (x3-chain-runtime, pallet-x3-supply-ledger, x3-packet-standard, x3-bridge, x3-fees, pallet-x3-slash) ✓; srtool + docker prereqs ✓; forbidden-secret scan ✓.
- The only blocker was a **missing local dependency**: `srtool` was not installed and section 5 fails loudly ("Without srtool, WASM builds are non-deterministic. Mainnet genesis artifacts MUST be reproducible."). Install the same pinned revision CI uses:
  `cargo install --locked --git https://github.com/chevdor/srtool-cli --rev 0485b5507a0b63cf7699376f15d9cb5c849bbcd0 srtool-cli` → `~/.cargo/bin/srtool`.
- Cost profile: the gate's release build is ~20 min cold and seconds warm; section 4 re-runs six real suites (fast once their release/test artefacts are warm). Opt-in as `make local-ci-release` / `scripts/local-ci.sh --release` (or `--all`) rather than part of the fast path.
- Queue trend with two runners: **10 → 7 → 4** while both stay busy. The CI release-gate runs (`35309656263` push, `35310867732` dispatch) are still queued behind their own live-lifecycle jobs; the CI `gate` job installs srtool itself, so section 5 was never its blocker.

### Verified state (master 1d8965916)
- PRs merged: #203-#215. `make mainnet-check` PASS (local, 18 checks). `make guard` ~20s PASS. `cargo fmt --all -- --check` PASS. clippy matrix 6/6 PASS. Kernel `atomic_kernel` 88 tests, eight registered in the registry.

### Next task seed
- Watch `production-gate` runs `35309656263` / `35310867732` reach `gate` for the CI-side equivalent (it additionally runs `make test-all-pallets`, `cargo build --release -p x3-chain-node --features mainnet-rc1`, `./scripts/run-srtool.sh build` and `make test-node-build`).
- Then attack the registry blockers: `triforge_runtime` (no migration dry-run across the six `construct_runtime!` variants), `btc_fortress_gateway` (SIM_TESTNET only), `launch_gate`/`repo_scanner_agent` scores.

## 2026-09-18 (round 14) — automated migration dry-run per runtime variant (PR #217)

### Facts to remember
- The `try-runtime` CLI is **not** in the pinned SDK (documented in `try-runtime-upgrade.yml`), and `make test-runtime-upgrade` is a stub that only builds. The dry-run is therefore **in-process**: `runtime_upgrade_rehearsal` (`runtime/src/lib.rs`, `#[cfg(all(test, feature = "std"))]`) calls `<AllPalletsWithSystem as OnRuntimeUpgrade>::on_runtime_upgrade()` **and** `<Migrations as OnRuntimeUpgrade>::on_runtime_upgrade()` — exactly what `Executive::on_runtime_upgrade` runs — and asserts the total weight fits in a block.
- One test covers all variants because the variant comes from features: `scripts/check-runtime-variants.sh` runs it with `--no-default-features --features std,tuples-96[,<variant>]` for `full`, `dev`, `dev+frontier`, `frontier`, `mainnet-rc1`, `testnet`.
- Verified results: **full PASS (42s), frontier PASS (88s), mainnet-rc1 PASS (66s), testnet PASS (39s); dev FAIL, dev+frontier FAIL.** The two dev variants' runtime test target **does not compile at all** — their modules are `#[cfg(all(test, feature = "dev"))]`, so nothing else ever compiled them, and the mock has drifted from the SDK (`pallet_balances::Config` no longer has `MaxHolds`; the mock's `Test` type is out of scope). One error fixed here (missing `assert_ok!`/`assert_noop!` imports in `runtime/src/fraud_proofs/pallet.rs`); the rest is a recorded repair task, not guessed at.
- **`runtime/src/tests.rs` is an orphan file** — no `mod tests;` anywhere — so its 267 lines, including five tests with **empty bodies**, have never compiled or run. Do not assume a file in `src/` is part of the crate; check for its `mod` declaration.
- SDK API traps worth remembering: `build_storage()` comes from the *deprecated* `GenesisBuild` trait, the modern `BuildGenesisConfig` exposes only `build()`, and the generated `RuntimeGenesisConfig` implements neither in a way that yields storage — after four failed attempts the working base was `sp_io::TestExternalities::default()` plus `frame_system::Pallet::set_block_number(1)`.
- Process note: I amended the commit message before opening the PR because it claimed a `testnet` PASS that had not finished yet. Do not put results in a commit message that the run has not produced.

### Verified state (master 5e233e510)
- PRs merged: #203-#217. `make mainnet-check` PASS (local). Runtime dry-run: 4 of 6 variants PASS, dev variants unverified (repair task). CI queue ~4 with both runners busy; release-gate runs `35309656263` / `35310867732` still queued.

### Next task seed
- Repair the dev-variant runtime test target (update the fraud-proof mock to the current `pallet_balances::Config`, fix `Test` scoping) so `dev`/`dev+frontier` can run the dry-run; then the blocker can move to the remaining limitation (empty storage vs a real state snapshot).

## 2026-09-18 (round 15) — dev variants repaired; **6/6 runtime dry-runs pass**

### Facts to remember
- `scripts/check-runtime-variants.sh` now reports **all six variants PASS** (full 49s, dev 13s, dev+frontier 11s, frontier 29s, mainnet-rc1 16s, testnet 11s). Reaching that required repairing the dev variants' test target, which had **never compiled**: the module is `#[cfg(all(test, feature = "dev"))]`, so nothing else ever built it.
- The repair in `runtime/src/fraud_proofs/pallet.rs`: (1) `impl … for Test` never matched — `construct_runtime!` declares `Runtime`, a half-finished rename producing ~100 trait-bound errors; renamed 16 sites. (2) `pallet_balances::Config` drift: dropped `MaxHolds`, added `RuntimeFreezeReason` + `DoneSlashHandler`. (3) `GenesisConfig` needs `dev_accounts: None`. (4) `BlockHashCount` must be `Get<u32>` → `ConstU32<250>`. (5) missing `assert_ok!`/`assert_noop!`. (6) **eleven tests read the binding they were defining** (`make_valid_proof(.., disputed.scheduler_commitment)`) → replaced with a named `correct_commitment()` helper; same value, explicit intent.
- Result: the dev suite runs **80 tests** and dev+frontier **84 tests**, all passing. The fraud-proof tests were sound; they simply could not compile. `triforge_runtime` score 65 → 70 with the blocker rewritten (only the empty-storage-vs-real-snapshot limitation remains).
- `cargo` is a rustup shim: `timeout 900 cargo …` fails with `failed to run command 'cargo': No such file or directory` even though `which cargo` resolves. Run cargo directly (or via `bash -lc`) instead of wrapping it in `timeout`.
- **I made a merge mistake (recorded so it does not repeat):** `gh pr create` returned **#219** while I had the number 218 in my head from an earlier guess, and `gh pr merge 218 --subject "…(#218)"` therefore merged **another agent's open PR** (`fix/bridge-evm-transfer-content-verification`, commit `5a6af9e3e`) and stamped the merge commit with my subject line. Always take the number from the `gh pr create` output — or re-check `gh pr view <n> --json headRefName` — immediately before merging.
  - Damage bound afterwards: that PR is a genuine security fix (both EVM verifiers defaulted `require_erc20_transfer = false`, so `verify_evm_transfer_proof` accepted *any* successfully-included Ethereum transaction as proof for an arbitrary bridge request; it now checks asset/amount/receiver and returns `X3_EVM_TRANSFER_EVENT_MISMATCH`). I ran the gate it belongs to: `cargo test --workspace` in `x3-lang/` → **439 passed / 0 failed**. Verified, but merged before verification rather than after.

### Verified state (master 504fe41d0)
- PRs merged by me: #203-#217, #219 (plus #218 by accident). Runtime dry-run 6/6. Dev suites 80/84 tests pass. x3-lang 439 tests pass.

### Next task seed
- Close the last `triforge_runtime` gap: run the rehearsal against a real state snapshot instead of empty storage.
- Check openclaw-agent's PR queue before merging anything: #218 landed by accident because it was open; their branch may have more.

## 2026-09-18 (round 16) — parallel agent's PR verified and landed deliberately; merge-number discipline

### Facts to remember
- Open-PR inventory at the time: **#216** (parallel agent, `fix/trading-core-v1-mainnet-and-audit-blindness`) and **#193** (their homepage/reqwest branch). Both authors show as the same GitHub account, so `author.login` cannot distinguish agents — check the *branch* and the body's provenance wording instead.
- #216 landed **properly**: tested the exact head SHA `0b379c4b4` on its own branch first — `cargo test --workspace` in `x3-lang/` → **441 passed / 0 failed**, clippy → exit 0 — then merged; master's merge commit `c2693dd6d` lists `0b379c4b4` as a parent (verified, not assumed). It fixes `x3c audit`'s blindness to trading-core-v1's `Item::AtomicTrade`/`Item::TradeRiskPolicy` and mainnet-mode compilation.
- **Merge-number discipline**: capture the PR number from `gh pr create`'s own output, confirm the branch with `gh pr view <n> --json headRefName`, merge, then check the merge commit's parents include the SHA that was tested. Done that way for #220.
- Master after this round: `5fb9f1af5`. PR #220 added `scripts/local-ci.sh --variants` / `make local-ci-variants` so the six-variant dry-run is reachable from the local suite (opt-in: it compiles the runtime six times).
- Local gate state: `scripts/check-runtime-variants.sh` all six PASS; x3-lang 441 tests; runtime dev/dev+frontier 80/84; `make mainnet-check` PASS.

### Next task seed
- Only #193 remains open (parallel agent's reqwest-0.12 / homepage branch) — leave alone unless asked.
- Remaining registry work: rehearsal empty-storage fidelity, `btc_fortress_gateway` (SIM_TESTNET only), `launch_gate`/`repo_scanner_agent` scores; `.rc4-runtime-upgrade-work/` (238 MB tracked generated data) still awaits a decision.

## 2026-09-18 (round 17) — stub removed; the orphaned runtime tests are dead *and* broken

### Facts to remember
- `make test-runtime-upgrade` was a stub ("try-runtime requires live chain"). It now runs the real six-variant dry-run (PR #221, master `be7303fee`): `full PASS 14s, dev 22s, dev+frontier 41s, frontier 20s, mainnet-rc1 41s, testnet 26s`.
- **`runtime/src/tests.rs` (267 lines, 14 tests) is dead *and* broken.** Wiring it in (`#[cfg(all(test, feature = "std"))] mod tests;`) produces **102 errors**: `mock::Runtime` does not satisfy `pallet_x3_settlement_engine::Config` (14), `pallet_x3_jury_anchor::Config` (14) or `pallet_x3_atomic_kernel::Config` (12), plus `event_metadata` bound failures — the same half-finished-rename rot as the fraud-proof mock, but on three pallets instead of one. I reverted the wiring (it would have broken the runtime test target) and left the file untouched. **Decision needed**: repair it (a real task) or delete it as dead code. Five of its tests have empty bodies.
- Verification discipline that worked twice now: bring a suspect test harness up to a compiling state *in a branch*, measure the error count, and only then decide repair vs. retreat — rather than starting a repair blind. #216's verification and this orphan probe both followed that shape.

### Verified state (master be7303fee)
- PRs merged by me this stretch: #203-#217, #219, #220, #221 (plus #218 accidentally, verified after).
- Green locally: runtime variant dry-runs 6/6; x3-lang 441 tests; dev/dev+frontier 80/84; `make mainnet-check` PASS; `make guard` ~20s.

### Next task seed
- Decide `runtime/src/tests.rs`: repair (mock for three pallets) or delete; five empty-bodied tests either way.
- CI-side release gate still unrun (runs `35309656263` push, `35310867732` dispatch) — the last structural unknown.

## 2026-09-18 (round 18) — rehearsal added to the release gate; srtool disappeared and why

### Facts to remember
- **Caught the CI release gate running for real**: run `35309656263`'s nested jobs show `native-x3vm-live` **completed/success**, `svm-live` in progress, `evm-live` queued — the four-job graph now executes, which is what #209 fixed.
- `scripts/mainnet_release_gate.py` now has a **section 5, "Runtime upgrade rehearsal"** (PR #222, master `d37c67371`): it runs `scripts/check-runtime-variants.sh`, so a runtime upgrade that panics or cannot fit a block blocks a release. Sections renumbered (6 = reproducible-build prereqs, 7 = forbidden secrets). Confirmed in a full gate run: `✓ migration dry-run passes for every construct_runtime! variant`.
- **`which srtool` is a false-negative source**: `cargo install` drops the binary in `~/.cargo/bin`, which is not on `PATH` for detached/non-interactive runs. The check now falls back to `~/.cargo/bin/srtool` and says where it found it.
- **srtool vanished between rounds.** It was installed (and seen by the gate) earlier; later `~/.cargo/bin` held only rustup shims and `~/.cargo/git/checkouts/srtool-cli-*` was gone. Probe result: a foreground write to `~/.cargo` **does** persist across invocations, so the difference is that the earlier install ran *detached* (`setsid nohup … &`) — reinstalling in the foreground persisted and `srtool --version` reports `srtool-cli 0.13.2`. Practical rule: **install tools in the foreground**, then verify with a *fresh* invocation before trusting them.
- Local gate state after this round: release gate includes the rehearsal; srtool present and found; runtime dry-runs 6/6; x3-lang 441; `make guard` ~20s.

### Next task seed
- Read `.ai/runlogs/mainnet-check-final.log` for the first full gate verdict that includes the rehearsal section.
- Still open: `runtime/src/tests.rs` (repair or delete), `.rc4-runtime-upgrade-work/` (238 MB tracked generated data), the CI-side `gate` job's first result.

## 2026-09-18 (round 19) — release gate PASS with rehearsal; PR #223 verified and merged

### Facts to remember
- **`make mainnet-check` now passes with all seven sections** (`✅ mainnet_release_gate: PASS`), the new section 5 being the per-variant migration dry-run (PR #222, master `d37c67371`).
- **PR #223** (parallel agent, `feat/trading-core-bridge-integration`, 19 files / +1274) verified on its own branch before merging: x3-lang `cargo test --workspace` → **468 passed / 0 failed** (was 441), clippy → exit 0, head `fcc0d8f78` confirmed as the merge commit's parent. Merge procedure held: number from `gh pr create`/`list`, branch confirmed, parent checked.
- CI release gate run `35309656263`: `native-x3vm-live` **success**, `svm-live` **success**, `evm-live` (2 jobs) queued — the `gate` job runs once all four finish. That is the last structural unknown outstanding.
- Two decisions remain the user's: `runtime/src/tests.rs` (dead + 102 compile errors when wired; 5 empty-bodied tests) and `.rc4-runtime-upgrade-work/` (475 files / 238 MB tracked generated chain data).

### Verified state (master d4fbdba74)
- x3-lang: 468 tests. Runtime: 6/6 variant dry-runs; dev/dev+frontier 80/84. Release gate PASS locally. `make guard` ~20s.
- PRs merged by me across the session: #203-#217, #219, #220, #221, #222, plus verified landings of the parallel agent's #216 and #223 (#218 landed accidentally and was verified after the fact).

### Next task seed
- Read the CI `gate` job's first verdict when the evm-live jobs finish.
- Then: rehearsal state fidelity, Dependabot high alerts, and the two pending decisions.

## 2026-09-18 (round 20) — dead libp2p deps removed: **high Dependabot alerts 12 → 4**

### Facts to remember
- `crates/x3-gpu-validator-swarm` and `crates/x3-turbine` each declared `libp2p = "0.53"` and **used it nowhere** (`grep -rl libp2p --include=*.rs` → zero). Deleting the two declarations removed the whole vulnerable generation from the graph: `libp2p-gossipsub 0.46.1` (2 high), `yamux 0.12.1` (high), `libp2p-mdns 0.45.1 → hickory-proto 0.24.4` (high). `Cargo.lock` lost 367 lines and now carries only `libp2p 0.54.1` (the substrate generation). **The fix was deletion, not an upgrade** (PR #224, master `5a2245398`).
- Verified: `SKIP_WASM_BUILD=1 cargo check --workspace` exit 0; `cargo test -p x3-gpu-validator-swarm` PASS (incl. 1k/10k TPS stress tests).
- **High alerts: 12 → 4** (medium 45, low 18 with `per_page=100`; the earlier "medium 18" reading was an un-paginated partial). Remaining high, all still in the root lockfile but reachable only via other features/targets — i.e. genuine upgrade work, not dead declarations: `hickory-proto` (no fixed version), `libp2p-quic 0.11.1`, `rustls-webpki 0.101.7`, `yamux` (the 0.12.1 entry). Dependabot had not re-scanned at hand-off (alert `updated_at` still 2026-06-07 / 09-15), so the count may fall further.
- `crates/x3-turbine` is **not a workspace member and cannot build standalone** ("current package believes it's in a workspace when it's not") — another orphaned artifact, sibling to `runtime/src/tests.rs`.
- Cargo gotchas hit while verifying: after changing dependencies, `--offline` can no longer resolve (`no matching package named 'Inflector'`) because the lock must be re-resolved against the index — run once online; and `cargo` is a rustup shim that is sometimes absent from PATH in these invocations, so use the absolute path `$HOME/.cargo/bin/cargo` when a command mysteriously reports "command not found".

### Next task seed
- Remaining high alerts need deliberate family bumps: `rustls-webpki` (0.101 line via rustls 0.21 / reqwest 0.11 — the parallel agent's PR #193 branch is the reqwest-0.12 work), `libp2p-quic`/`yamux` (libp2p 0.54 → newer), `hickory-proto` (no fix; drop mdns or wait).

### Precise origins of the 4 remaining high alerts (lockfile analysis, master `5a2245398`)

| Alert | Pulled in by | What unblocking it requires |
| --- | --- | --- |
| `yamux` (vulnerable 0.12.1) | `libp2p 0.54.1`, `libp2p-yamux 0.46.0`, `litep2p 0.13.3` | libp2p ≥0.56 — i.e. moving the **polkadot-sdk `stable2512` pin** (sc-network 0.55.2), not a local bump |
| `libp2p-quic 0.11.1` | `libp2p 0.54.1` | same SDK pin (fix 0.13.1 needs libp2p ≥0.56), or dropping the quic feature sc-network enables |
| `rustls-webpki 0.101.7` | `rustls 0.21.12` via `reqwest 0.11.27` / `hyper-rustls 0.24.2` / `tungstenite 0.20.1` / `libp2p-tls 0.5.0` | move off the rustls 0.21 line (reqwest 0.12 + friends) — the parallel agent's PR #193 branch is exactly this work. The 0.23 line already carries the fixed 0.103.15 |
| `hickory-proto 0.24.4 / 0.25.2` | `hickory-resolver 0.24.4`, `0.25.2` via `libp2p-mdns 0.46.0` | **no fixed version exists upstream** — needs mdns removed or an upstream release |

Conclusion to carry forward: 8 of the 12 high alerts were removable by deleting dead declarations; the remaining 4 are constrained by the SDK pin, an in-progress dependency-family move, or a missing upstream fix. Do not "fix" them with isolated bumps — this family has to move together (the same rule that bit the ark-* family earlier).

## 2026-09-18 (round 21) — the CI release gate's first verdict, and my #208 regression it found

### Facts to remember
- **Run `35309656263` gave the release gate its first real verdict**: `native-x3vm-live` success, `evm-live / real X3VM to Anvil lock claim lifecycle` success, `svm-live` success, **`evm-live / EVM HTLC live anvil lifecycle` failure**, `gate: skipped` → run failure. So the `needs:` graph is now reached and the one blocker is identified rather than hidden.
- **The failure was my regression from #208.** That PR set `CARGO_TARGET_DIR` to a shared directory beside the checkout (good for node builds) but `X3-contracts/evm/test-live-lifecycle.sh` asserted the broadcaster at `$REPO_ROOT/target/release/`, so it reported *"not built"* for a binary cargo had just built elsewhere. Fixed in PR #225 (master `5eeb174b1`) with `${CARGO_TARGET_DIR:-$REPO_ROOT/target}/release/x3-evm-broadcast`, verified **both ways**: 11 passed / 0 failed with the variable set (the CI case) and unset.
- It went unnoticed because the one earlier passing EVM run used the pre-#208 workflow file (a run executes the file from its own commit) and the release-gate run that would have exercised it sat queued behind four live jobs. The SVM workflow escaped the same bug only because #208 scoped the variable to its cross-domain steps.
- **General rule, now thrice-learned**: when a change moves *where* an artefact lands, every consumer that hard-codes the old location is a latent break. Search for the path (`grep -rn "target/release"` etc.) before landing such a change.

### Next task seed
- The push for #225 triggers a fresh `production-gate` run; its EVM contract job should now pass, which means the **`gate` job runs for the first time ever** (`make guard`, `make test-all-pallets`, `cargo build --release -p x3-chain-node --features mainnet-rc1`, `./scripts/run-srtool.sh build`, `make test-node-build`, `make mainnet-check`). Read its verdict next.

### Artefact-path audit (same round)

Only **three** workflows export `CARGO_TARGET_DIR` (`x3vm-live-lifecycle`, `x3vm-evm-live-lifecycle`, `x3vm-svm-live-lifecycle`), and only two of them invoke scripts:

| Workflow | Script | Status |
| --- | --- | --- |
| `x3vm-live-lifecycle` | none (cargo test only) | safe |
| `x3vm-evm-live-lifecycle` | `X3-contracts/evm/test-live-lifecycle.sh` | fixed in #225 |
| `x3vm-svm-live-lifecycle` | `programs/svm/x3_atomic_swap/test-live-lifecycle.sh` | hardened in #226 — its SBF/client builds now run with `env -u CARGO_TARGET_DIR`, verified 15 passed / 0 failed *with* the variable set |

Many other scripts hard-code `$ROOT/target/release/...` (e.g. `scripts/mainnet/*`, `run-frame-benchmarks.sh`, `run-chopsticks.sh`, `start-validator-easy.sh`), but none of them run under a workflow that exports the variable today. If that changes, they break the same way — check this table before exporting `CARGO_TARGET_DIR` anywhere new.

Master after this round: `8d774fbcd` (PRs #224, #225, #226).

### Rehearsal fidelity attempt (round 22) — stopped after 3 failures, note landed instead

- Tried to seed the rehearsal's externalities from the runtime's own genesis state so the upgrade hooks run against populated storage. Plan: `sp_genesis_builder::GenesisBuilder` + `frame_system::genesis_builder_helper::build_state`.
- **Three compile failures, in this order**: (1) `build_storage` does not exist on the generated `RuntimeGenesisConfig` (only the deprecated `GenesisBuild` provides it, and the generated type does not implement it); (2) same for `frame_system::GenesisConfig`; (3) `<Runtime as GenesisBuilder<crate::Block>>` is not satisfied — the runtime's impl carries a `<Block>` parameter while the SDK trait definition I read (`primitives/genesis-builder/src/lib.rs:98`) is **not** generic, and a "function takes 3 arguments but 1 was supplied" error followed. The discrepancy suggests the runtime resolves a different `GenesisBuilder` than the one in the checkout I grepped.
- Decision: keep the working empty-storage rehearsal and record the attempts **at the call site** (PR #227, master `eadd79da6`) rather than leave it blocked. `runtime_upgrade_rehearsal` re-verified ok after the revert.
- Next attempt should start by checking *which* `GenesisBuilder` the runtime actually links (`cargo tree -p x3-chain-runtime | grep genesis-builder` and the `sp-genesis-builder` version in the runtime's dep graph) before writing the call.

### Resolved in the same round — the fidelity gap is closed (PR #228, master `e3e20afa5`)

- **The insight**: `sp_genesis_builder::GenesisBuilder` is declared **inside `sp_api::decl_runtime_apis!`** (`primitives/genesis-builder/src/lib.rs:96`) — it is a *runtime API*, not a plain trait. That is why its impl is `GenesisBuilder<Block>`, why the trait looked non-generic, why direct calls failed with arity errors, and why `build_storage` never existed on `RuntimeGenesisConfig`.
- **The working call** goes through the plain helper functions the API delegates to:
  ```rust
  let json = frame_support::genesis_builder_helper::get_preset::<RuntimeGenesisConfig>(&None, |_| None)
      .expect("default genesis preset");
  frame_support::genesis_builder_helper::build_state::<RuntimeGenesisConfig>(json)
      .expect("genesis state builds");
  ```
  (`get_preset<GC: BuildGenesisConfig + Default>(&Option<PresetId>, impl FnOnce(&PresetId) -> Option<Vec<u8>>)`; `build_state<GC: BuildGenesisConfig>(Vec<u8>)`.)
- The rehearsal now seeds each variant's real genesis and then runs the `OnRuntimeUpgrade` hooks.
- Verified: **all six variants PASS against populated genesis state** (full 18s, dev 20s, dev+frontier 31s, frontier 17s, mainnet-rc1 4s, testnet 28s). Registry `triforge_runtime` 70 → 75; the remaining blocker is now the honest one — this rehearses an upgrade from *this* runtime's genesis, not from a previous runtime's state snapshot, and storage-version alignment is unchecked (what the absent try-runtime CLI would have given).
- General lesson: when a trait method "takes 3 arguments" or an impl has a parameter the trait definition lacks, suspect that the item is generated by a macro (runtime APIs, `construct_runtime!`, `decl_module!`) and look at the macro rather than the trait.

## 2026-09-18 (round 23) — cargo parallelism capped; the lint gate now queues instead of dying

### Facts to remember
- The push-triggered **`Rust Clippy` runs are queued** now (only the `pull_request` ones fail, on the hosted billing lock) — so the lint gate has a live path to a verdict for the first time. It has not completed yet; the fleet is the constraint.
- Measured cause: two self-hosted runners on one 32-core host, each letting cargo default to one job per core → **load 54-67**, i.e. thrashing. PR #229 sets `CARGO_BUILD_JOBS=12` on the heavy jobs (3 live workflows, `production-gate.gate`, `rust-clippy.clippy`), so two 12-way builds fit with headroom for the node/validator processes. Trade-off stated: per-job time may rise; two jobs finishing beats both crawling.
- Master after this round: `233d1f10e` (PRs #227, #228, #229).

### Next task seed
- Read the first completed push-triggered `Rust Clippy` run and the release-gate `gate` run once the queue drains; both are now legitimate verdicts rather than infrastructure noise.

## 2026-09-18 (round 24) — every live gate was running twice per push (PR #230)

### Facts to remember
- **Root cause of the queue backlog**: each of the three live workflows had a standalone `push` trigger *and* was called by `production-gate.yml` (`native-x3vm-live`, `evm-live`, `svm-live`) — so every push executed all four heavy jobs **twice** on the same two runners.
- Fix (master `a07deb252`): remove the standalone `push` triggers (keep `pull_request`, `workflow_dispatch`, `workflow_call`), and give `production-gate` a `paths-ignore` for docs/evidence paths (`**/*.md`, `docs/`, `.ai/`, `reports/`, `audit-artifacts/`, `memory/`, `stakeholder_comms/`).
- **Measured before/after on consecutive merges** — push-run sets:
  - `233d1f10e` (before): production-gate **+ x3vm-live-lifecycle + x3vm-evm-live-lifecycle + x3vm-svm-live-lifecycle** + hosted set = 10 runs.
  - `a07deb252` (after): production-gate + hosted set = 7 runs, no standalone live workflows.
  → ~4 fewer heavy job-runs per push, zero coverage loss (production-gate still calls and `needs:` them).
- Coverage argument that made this safe: production-gate has **no path filter**, so it runs on every push; the live workflows' own path filters were redundant with it.
- General lesson: a reusable workflow that is *also* independently triggered runs twice. Check for `workflow_call` + an overlapping `push`/`pull_request` trigger whenever CI load seems disproportionate to the commit count.

### Preparation for the next triforge gap (cross-version rehearsal) — analysis only, nothing pushed

- `x3-chain-node export-state` **exists** ("Export full runtime state at a given block into a snapshot file"), so rehearsing an upgrade against a *real previous state* is feasible in principle: export a snapshot, load it into externalities, run the hooks.
- Loading it needs `sp_runtime::Storage` (the export is a key→value map, the shape `TestExternalities::new(storage)` takes) rather than the genesis-preset path used today, because a state export is not a `GenesisConfig`.
- Cost to weigh before doing it: the snapshot must be **committed** for the test to be reproducible, and this repo already suffers from 238 MB of tracked generated chain data. A smaller first step that catches the most common upgrade bug without any snapshot is a **storage-version alignment check** — assert each critical pallet's `on_chain_storage_version() == Pallet::current_storage_version()` after the upgrade hooks run (what try-runtime's checks did).
- Polling note (same round): GitHub's run/job `in_progress` status lagged badly again — the queue showed 6 with *no* run in progress while both runners were demonstrably busy (`Worker_*.log` mtimes seconds old, `busy=true`). Authoritative signal remains the runner logs, not the API's run list.

### Capacity finding (same round)
- With two self-hosted runners executing heavy Rust gates plus a local `make mainnet-check` release build, the machine's **load average reached 54-67 on 32 cores** — heavily oversubscribed. Conclusion for the local CI: **more runners is not the lever; less rebuilding is.** Keep the fleet at two and invest in cache reuse (#208's per-runner persistent `CARGO_TARGET_DIR`), scoped gate triggers, and shrunk release builds. Consider measuring with `uptime` before adding capacity.

## 2026-09-18 (round 25) — the local CI is now the CI of record

### Facts to remember
- **Hosted CI reality, measured by the new gate**: 42 workflows parsed, **30 are hosted-only** (they cannot produce a verdict on this account — instant, step-less runs), 11 run on self-hosted labels. `scripts/check_ci_workflow_refs.py --parity` prints the inventory and is now the authoritative list.
- `scripts/local-ci.sh` **already existed in master** (196 lines, `2b3a9c579`/`eff1291cc`/`a85bd2a31`), and the only thing referencing it was the Makefile. It ran every gate sequentially, had no machine-readable output, and had no gate for the CI wiring itself. It was **extended, not replaced**: the flag interface (`--live`, `--cross`, `--release`, `--variants`, `--all`, `--list`) is unchanged, and the new layer adds `--jobs`, `--cargo-jobs`, `--only`, `--skip`, `--changed-from`, `--pre-push`, `--dry-run`, `--fail-fast`.
- **Parallel scheduling with real exit codes**: each gate runs in its own process, writes `local-ci-<ts>-<slug>.{log,status,secs}`, and the parent reads the status back — so parallelism cannot swallow a failure (the H13 lesson). `--jobs 3` + `CARGO_BUILD_JOBS=10` is the default; `--jobs 1` reproduces the old sequential behaviour for triage.
- **New gates**: `script syntax` (parses 302 shell + 506 Python + 5 JS files, ~5s, no execution), `workflow wiring`, `test integrity diff`, `make gate exit codes`. Measured fast-set wall clock with `--jobs 4`: 17s where the light gates used to run ~35s sequentially.
- `test_cheat_guard.py` now takes `--base REF` (diff `REF...HEAD` + working tree). It previously scanned only the **index**, so in CI — where nothing is staged — it passed vacuously. A missing base ref exits 2 instead of silently scanning nothing.
- `.githooks/pre-push` delegates to `scripts/local-ci.sh --pre-push`: fast set always, `--live` when the diff touches `X3-contracts/{evm,svm}/**`, `--variants` when it touches `runtime/**`, `--release` + `--variants` when the target ref is `master`/`main`. Escape hatches are loud: `X3_LOCAL_CI_SKIP_ALL=1` exits 3 and prints that nothing was verified; `X3_LOCAL_CI_PREPUSH_MODE` passes any local-ci flags through.
- `docs/local-ci.md` documents the gates, the flags, the artefact layout, and the hosted/self-hosted asymmetry.

### Defects the new gates found, and this change fixes
1. **`master` did not pass `cargo fmt --all -- --check`.** `runtime/src/fraud_proofs/pallet.rs` (introduced with `73e45292f`) carried an unsorted import group and an over-long line. Fixed by `cargo fmt --all`; no other file changed. The earlier "fmt PASS" claim was measured in the *other* lineage — `d3db02250` (parallel agent) is **not** descended from `a07deb252`, and it carries a different version of that same file.
2. **Seven tracked scripts did not parse**, so every step invoking them died at parse time:
   - `scripts/k8s-deploy.sh` — missing closing `"` on the indexer curl hint (line 169).
   - `tests/test_monitor.sh` and its byte-identical copies `tests_phase4/test_monitor.sh`, `tests_core/test_monitor.sh` — `local msg="${3:-Values don't match}"`: a `'` inside `${...}` inside `"..."` is a **bash syntax error**, not a style issue. Rewritten as `${3:-}` plus a default.
   - `tests/e2e/stop_test_environment.sh` and `tests_phase4/e2e/stop_test_environment.sh` — truncated mid-string at EOF; completed the warning text and the `if/fi`.
   - `swarm_infrastructure/skills_adapter.py` — a mangled `try: import requests / except:` block had swallowed a duplicate of `ollama_generate`'s body, so the module **could not be imported at all**. Restored the intended `requests = None` fallback that `ollama_generate` probes. Verified: module imports, `load_skills('/nonexistent') == {}`, and `ollama_generate` raises its own `RuntimeError` instead of `NameError`.
3. **`.github/workflows/test-integrity.yml` was doubly dead**: it invoked `scripts_infrastructure/enforce_x3_rules.py`, which has never existed in this tree, and it was wired to `branches: [main]` while the default branch is `master`. Rewritten to run the real guards (`scripts/test_cheat_guard.py --base origin/<base_ref>`, `scripts_infrastructure/pr_supervisor.py`, `make check-make-gates`) on `[self-hosted, Linux, X64, x3]`.

### Dead ends / hazards
- **`apply_patch` "Add File" overwrote an existing tracked file** (`scripts/local-ci.sh`) and reported success with an `A` marker. The original had to be recovered with `git show HEAD:scripts/local-ci.sh`. Always `git ls-files <path>` before adding a file here.
- `git log --all -- <path>` returning commits is **not** evidence the file exists at HEAD — check `git show HEAD:<path>`.
- Two lineages with different content for the same path coexisting is now a *demonstrated* hazard, not a theoretical one. A proof note must name the tree (and the SHA) it was measured in.

### Next task seed
- Route the remaining self-hosted workflow jobs through `make local-ci ARGS="--only <slug>"` so a workflow step and a local gate cannot drift apart.
- `tests_phase4/` and `tests_core/` are byte-identical copies of `tests/` (same md5 for the files checked); decide whether to dedupe — they produced three copies of the same broken script.
- `srtool` is missing from `~/.cargo/bin` again (the rustup reinstall on 2026-09-17 wiped it), so the release gate's reproducibility section fails locally; reinstall the pinned 0.13.2 build or make that section a documented warning.

### Landed (PR #231, master `8262b2145`, merge parents `a07deb252` + `50cc169b3`)

- Three commits: `6a1e51ccc` (parallel scheduler, new gates, summary json, hook, docs), `c4b65ce8b` (the seven unparseable scripts + dead workflow + rustfmt), `50cc169b3` (BLOCKED classification for environment-blocked gates).
- **Full fast-set run on the branch** (`--jobs 3 --cargo-jobs 8`, stamp `20260918T092259Z`): **15 PASS / 2 red**. Red = `test node` (real, pre-existing) and `test cross-vm-coordinator` (network fetch); both now have issues:
  - **#232** — `x3-chain-node`'s own dev chain spec fails to boot: the genesis blob carries the full variant's pallet set (`depinMarketplace`, `evolutionCore`, `x3Oracle`, `x3DappHub`, `x3FlashLoan`, `privateExecution`) while the rejecting runtime knows the smaller set; test `service::runtime_bridge_client_tests::full_node_http_rpc_...` fails at `node/src/service.rs:2702`, 49 passed / 1 failed. Leading hypothesis: the node's runtime features and the runtime's wasm-builder feature set diverge, so `x3_chain_genesis` and the embedded WASM come from different `construct_runtime!` variants.
  - **#233** — `crates/cross-vm-coordinator` is outside the workspace `members`, so its gate needs the network (`--offline` fails on `environmental` via `sp-externalities`).
- Verified on merged master: `./scripts/local-ci.sh --only format-check,script-syntax,workflow-wiring,test-integrity-diff,make-gate-exit-codes` → 5/5 PASS at `8262b2145`.
- **Operational note for the next push**: the pre-push hook runs the fast set, which is red until #232/#233 are fixed. The recorded (non-silent) way to push meanwhile is `X3_LOCAL_CI_SKIP=test-node,test-cross-vm-coordinator git push` — the skip is printed and written into the run summary. Add that recipe to `docs/local-ci.md` in the follow-up.
- The `production-gate` `gate` job self-installs the srtool CLI (`cargo install --locked --git https://github.com/chevdor/srtool-cli --rev 0485b5507a...`) before `./scripts/run-srtool.sh build`, so the local `srtool` binary being missing does not block the runner — it only makes `make mainnet-check` red locally.

## 2026-09-18 (round 26) — #232 fixed: the node was booting a runtime from another feature set

### Root cause (proven, not inferred)
`runtime/build.rs` embedded `target/<profile>/wbuild/x3-chain-runtime/x3_chain_runtime.wasm` whenever `SKIP_WASM_BUILD` was set. **That path is shared by every feature set**, so a blob built for one variant was embedded into a build of another:

```
$ sed -n '1,8p' target/debug/wbuild/x3-chain-runtime/Cargo.toml     # manifest the blob was built from
[dependencies.wasm-project]
default-features = false
features = ["native-real-vm-adapters", "mainnet-rc1", "tuples-96"]
```

The node then decoded its own chain spec against that runtime and rejected it (`unknown field depinMarketplace, expected one of … x3Custody`), which is the #232 panic at `node/src/service.rs:2702`. `SKIP_WASM_BUILD=1` is used by `workspace check`, `check-runtime-variants.sh`, the `scripts/mainnet/*` gate scripts and (before this round) the `test node` gate — so the hazard was reachable from several directions, and in a release build it means shipping a node whose embedded runtime does not match its own chain spec.

### Fix (PR #235, master `54d62514a`)
- `runtime/build.rs` writes a `<wasm>.features` sidecar with the feature key the blob was built for. The key is **derived generically** from `CARGO_FEATURE_*` minus `std`/`default` — the same list `substrate-wasm-builder` passes to the nested build (verified against the generated wbuild `Cargo.toml`), so a future variant feature is covered without touching this code. (Note: the outer key is NOT identical to the nested feature list — the outer set also carries implicit optional-dep features such as `pallet-sudo` and `native-real-vm-adapters`. That is fine: both sides compute it the same way, so it is a faithful discriminator.)
- `SKIP_WASM_BUILD` embeds the cache **only** on a key match; otherwise it writes the `None` stub plus a warning naming both sets, and the call site fails with the existing actionable message. A wrong-variant runtime can no longer be embedded silently.
- The `test node` gate no longer forces `SKIP_WASM_BUILD` (the service tests boot a real node); `env -u SKIP_WASM_BUILD cargo test -p x3-chain-node` builds the runtime once and cargo caches it.
- PR #236 (master `adbdf2e2f`) lowercased the gate slugs, so `--only evm-contract-lifecycle` matches the acronym gate names.

### Verification
| check | before | after |
| --- | --- | --- |
| `cargo test -p x3-chain-node --lib` | 49 passed; 1 failed; 3 ignored | **50 passed; 0 failed; 3 ignored** |
| `./scripts/local-ci.sh --only test-node` | FAIL 149s | **PASS 100s** (130s on merged master) |
| targeted `full_node_http_rpc_…svm_rpc_reads_runtime_state` | panic at `service.rs:2702` | `ok` (85.68s) |
| `SKIP_WASM_BUILD=1` + mismatched cache | silent wrong-runtime boot | stub + `cached runtime WASM was built for features [mainnet-rc1] but this build is [full]` |
| `--live` EVM/SVM gates (regression check for the build.rs change) | — | **PASS 71s / 119s**, EVM 11 passed 0 failed, SVM 15 passed 0 failed |

### Next task seed
- The only remaining red gate is #233 (`crates/cross-vm-coordinator` outside the workspace → the gate needs the network and reports BLOCKED). Options are in the issue: add it to `members`, vendor a lockfile, or drop it as a duplicate.
- Run `./scripts/local-ci.sh --cross` (the X3-native `x3vm_live_lifecycle --ignored` suite) — it has not been re-run since the embedded-runtime change.
- The `test node` gate now costs a runtime WASM build (~2.5 min) on any runtime change; that is deliberate (the service tests need a runtime matching their own genesis) and cargo caches it afterwards.

## 2026-09-18 (round 27) — upgrade storage-version alignment, and 15 fictional registry citations

Landed as PR #243 (master `65b9f6a9b`, merge parents `adbdf2e2f` + `45eec71c5`).

### The rehearsal never checked storage-version alignment
`runtime_upgrade_rehearsal` only asserted the hooks fit in a block. A pallet left at a stale on-chain storage version after an upgrade is the classic silent mainnet bug (reads keep succeeding against a schema the code no longer expects), and the absent `try-runtime` CLI was the only thing that would have caught it.

- Added `runtime_upgrade_rehearsal_storage_versions` to `runtime/src/lib.rs`: after `AllPalletsWithSystem::on_runtime_upgrade()` + the `Migrations` tuple, every pallet in the **intersection of all five `construct_runtime!` variants** that declares an in-code storage version must have the same version on chain.
- API notes for the next agent: use `frame_support::traits::GetStorageVersion::{in_code_storage_version, on_chain_storage_version}` and `StorageVersion::get::<P>()`; `StorageVersion`'s inner `u16` is private, so decode it (`u16::decode(&mut &sv.encode()[..])`) — and `StorageVersion`/`NoStorageVersionSet` are different types, which is why the check needs a small `DeclaredVersion` trait with one impl each (pallets with no declared version are skipped by construction). `Encode`/`Decode` must be imported explicitly.
- **Coverage floor + teeth**: the test fails if fewer than 5 pallets were inspected, and `runtime_upgrade_rehearsal_storage_version_check_has_teeth` writes a deliberately wrong on-chain version and proves the check reports it. Do not remove either; without them this could silently become a no-op.
- Verified: `full PASS 9s | dev 17s | dev+frontier 18s | frontier 23s | mainnet-rc1 18s | testnet 15s` on merged master; the full variant reports **21 pallets declare a version, 21 aligned**.

### `check-readiness-consistency.sh` was blind to 12 of 15 registry entries
`required_tests` parsing set the in-array flag and cleared it on the *same line*, so single-line arrays — the form used by 12 of the 15 entries — were never checked. Fixed the parser (accumulate the array body, flush at `]`), and the gate immediately produced **15 fictional citations**: readiness scores justified by test names that exist nowhere in the repo.

| feature | previously cited (nonexistent) | now cites (real) |
| --- | --- | --- |
| `triforge_runtime` | `runtime_upgrade_rehearsal` (real fn, but `crate_or_service` pointed at `pallets/evolution-core`) | points at `runtime`, cites both rehearsal tests |
| `atomic_gateway` | `audit_gate_enabled`, `revoke_disables_gateway` | `enabling_external_bridges_requires_documented_audit_gate`, `revoking_bridge_audit_gate_disables_external_bridges` (in `pallets/x3-cross-vm-router`) |
| `axe` | `axe_create_pool`, `axe_swap`, `axe_fee_accounting` | `create_pool_works`, `swap_works`, `test_amm_calculations` |
| `x3_forge` | `forge_create_token_guarded`, `forge_requires_sentinel_score` | `test_01_create_fixed_supply_token_emits_event`, `test_sentinel_frozen_mint_authority_blocks_mint` |
| `atomic_lock` | `lock_liquidity`, `unlock_after_schedule`, `early_unlock_rejected` | repointed to `pallets/x3-lp-locker`; `lock_lp_creates_lock`, `unlock_lp_removes_lock_after_expiry`, `unlock_lp_rejects_before_expiry` |
| `x3_reactor` | `benchmark_job_submits`, `benchmark_result_publishes` | `create_and_compare_reports`, `test_compile_optimized` + blocker that no job-submit test exists |
| `tauri_os` | `dead_buttons_report`, `tauri_wiring_report` (report artifacts) | `required_tests = []` + blocker that `apps/tauri-os` has no automated test |

`triforge_runtime` 75 → 80 (storage-version alignment now checked); its blocker list names only the remaining gap. `feature_matrix.py check` still passes (145 features) — note its rule "tested >= 80 requires required_tests or test_evidence", which is why the score stayed at 80 rather than higher.

### Environment notes
- `pytest` in PATH is Python 3.10 and cannot import `tomllib`, so `tests/test_feature_matrix.py` fails collection here — a pre-existing environment gap, not a code failure. Use `python3 scripts/feature_matrix.py check` (Python 3.14) instead; it passes. The local CI does not run pytest at all, which is worth adding.
- `cargo test -p x3-readiness --lib` → 7 passed after the registry edits.

### Next task seed
- Cross-version rehearsal: export a previous runtime's state (`x3-chain-node export-state`) and run the hooks against it (the remaining `triforge_runtime` gap).
- Add a `pytest` gate to `scripts/local-ci.sh` (with an interpreter that has `tomllib`), so the Python suites are not invisible locally.
- #233 is still the only red gate in the local CI.

## 2026-09-18 (round 28) — the migration run-path is rehearsed, and CI stops showing red for jobs that cannot run

### Migration run-path rehearsal (PR #259, master `1f3ccf997`)
`runtime_upgrade_rehearsal_storage_versions` (round 27) only proved the *end* state is consistent — it passes even when a pallet never had to move. Added `runtime_upgrade_rehearsal_migrations_advance_behind_versions`:

- the four pallets whose migration struct is registered in the `Migrations` tuple (`AtlasKernel`, `Treasury`, `AgentMemory`, `AgentAccounts` — the tuple is `pallet_x3_kernel::migrations::Migration`, `pallet_treasury::…`, `pallet_agent_memory::…`, `pallet_agent_accounts::…`, each of which bumps a behind version to 1) are each set to storage version 0;
- **the rollback is asserted before the hooks run**, so the test cannot pass vacuously;
- `AllPalletsWithSystem::on_runtime_upgrade()` + `Migrations` run, and every pallet must end at its declared version; the test asserts it covered 4 pallets.

It fails if a migration is dropped from the tuple or a pallet's declared version moves past what its migration writes. All six variants PASS (`full 20s | dev 12s | dev+frontier 19s | frontier 19s | mainnet-rc1 20s | testnet 12s` on the branch; `9/10/10/10/89/14s` warm on merged master). `scripts/mainnet_release_gate.py` section 5 already shells out to `scripts/check-runtime-variants.sh`, so the release gate covers it with no extra wiring. Registry `[triforge_runtime].required_tests` now names all three rehearsal tests.

### CI: triggers that can never execute (PR #260, master `ac9e6a592`)
Hard evidence from run `35334984277`: the four `production-gate` jobs completed as **failure with `runner_name: ""` and `steps: 0` in ~2s**, and `gh api .../actions/jobs/<id> --jq .labels` returned `["ubuntu-latest"]`. They are routed to hosted for PR events by the `github.event_name == 'pull_request' ? 'ubuntu-latest' : self-hosted` ternary, and this account's hosted minutes are locked — so every PR carried guaranteed-red checks that proved nothing.

- Removed the `pull_request` trigger from `rust-clippy.yml`, `production-gate.yml`, and all three `x3vm-*-live-lifecycle.yml`. The three live workflows stay reachable through `production-gate`'s `workflow_call` on every push (their only real verdicts) plus `workflow_dispatch`.
- **Do NOT "fix" this by routing PRs to the self-hosted runner.** `rust-clippy.yml` documents the decision: this repo is public, and checking out an untrusted fork PR and running `cargo build`/proc-macros executes arbitrary code on `x3star1`. Keep the split.
- The `runs-on` ternaries are deliberately left in place as a safety net if a PR trigger is ever re-added.
- `docs/local-ci.md` now records the sharper inventory: **26 other workflows still auto-run on hosted-only runners** (semgrep, trivy, osv-scan, formal-verification, economic-attack-tests, proof-gates, release-hardening, frame-benchmarking, zombienet-integration, x3-desktop-ci, the deploy workflows, …) with the three options (route to runner / make dispatch-only / delete duplicates). Until a decision is made, read their red as "could not run".

Verification: `python3 scripts/check_ci_workflow_refs.py --parity` → 0 missing refs, 0 unreachable workflows, 0 trigger gaps; `./scripts/local-ci.sh --only workflow-wiring,script-syntax` → PASS.

### Environment note
`gh pr create --body "$(cat <<'EOF' ...)"` with backticks in the body hung on the interactive editor and mangled the body (the heredoc was not treated as quoted). Use `--body-file <path>` for anything containing backticks.

## 2026-09-18 (round 29) — first fully green full-set run; one self-inflicted regression found and fixed

### The regression (PR #261, master `0d028cd12`)
`runtime/build.rs` is a build script, so every clippy configuration compiles and lints it. The helpers I added in #235 took `&PathBuf`, which `clippy::ptr_arg` rejects under `-D warnings`:

```
error: writing `&PathBuf` instead of `&Path` involves a new object where a slice will do
  --> runtime/build.rs:30:23      = note: `-D clippy::ptr-arg` implied by `-D warnings`
```

`make clippy` and the self-hosted `Rust Clippy` workflow were red on master as a result. Fixed by taking `&Path` in the four helpers (`std::path::{Path, PathBuf}`; deref coercion keeps call sites unchanged) and verified with all three previously-red gates: `clippy workspace 211s`, `clippy runtime rc1 147s`, `clippy node rc1 282s`.

**Process lesson (now in `docs/local-ci.md`):** #235 was verified with `--only <light subset>`, which is for triage, not sign-off. Any change under `runtime/` or in a build script must be verified with the whole fast set (`--pre-push` or a plain run) before merging. The pre-push hook runs the fast set precisely so this cannot slip through again — but only if the gate is actually invoked.

### First fully green full-set run (master `0d028cd12`, stamp `20260918T105444Z`)
All 18 gates PASS:

```
format check 6s | agent guards 31s | make gate exit codes 0s | script syntax 7s |
workflow wiring 2s | test integrity diff 0s | readiness consistency 8s |
workspace check 31s | clippy workspace 171s | clippy runtime rc1 281s |
clippy node rc1 396s | test x3-lang 263s | test x3-lang python 4s |
test atomic-kernel 129s | test atomic-swap std 9s | test settlement-engine 13s |
test node 160s | test cross-vm-coordinator 141s
```

- `test node` (was the #232 panic) passes in 160s including the runtime WASM build.
- `test cross-vm-coordinator` **passes** (141s) in a run with network access — confirming #233's diagnosis: the gate is environment-blocked, not a code defect. Without network it reports BLOCKED and still fails the run.
- `test cross-vm-coordinator` also took 363s in the previous (sandboxed, escalated) run; the gate is timing-sensitive because it fetches its own dependency graph.

### Next task seed
- The full fast set is now green on master; keep it that way by running it before every merge that touches `runtime/`, build scripts, `Cargo.toml`, or workflows.
- Historical-state upgrade rehearsal (the only remaining `triforge_runtime` gap).
- #233: the crate needs the network; decide workspace membership vs vendoring vs dedupe.

## 2026-09-18 (round 30) — the release gate runs end to end here, and #233 is closed

### `make mainnet-check` PASSES on this box (first time)
Blocked all session by the missing srtool binary (a rustup reinstall wiped it). Installed the exact revision the gate job uses:

```bash
cargo install --locked --git https://github.com/chevdor/srtool-cli --rev 0485b5507a0b63cf7699376f15d9cb5c849bbcd0 srtool-cli
# -> srtool-cli v0.13.2 at ~/.cargo/bin/srtool
```

Then `make mainnet-check` (log kept at `.ai/runlogs/mainnet-check-20260918T1113Z.log`) → **PASS**, all seven sections:

| # | section | result |
| --- | --- | --- |
| 1 | required documentation | ✓ |
| 2 | build validation | node binary + `x3_chain_runtime.compact.compressed.wasm` present |
| 3 | chain-spec/genesis artifacts | both current specs valid; `production_config()` present |
| 4 | critical suites | runtime, supply-ledger, packet-standard, bridge, fees, x3-slash |
| 5 | runtime upgrade rehearsal | every `construct_runtime!` variant |
| 6 | reproducible-build prereqs | srtool ✓, docker ✓ (29.1.3), no `SKIP_WASM_BUILD` override |
| 7 | forbidden secrets scan | ✓ |

plus `check-readiness-consistency.sh` → PASS. Docker already had `paritytech/srtool:1.93.0` (4.57GB) pulled, so a future `./scripts/run-srtool.sh build` will not need that download.

### #233 closed — the coordinator gate is deterministic and offline (PR #263, master `ed37b0f9d`)
`crates/cross-vm-coordinator` is deliberately outside the workspace (its own lockfile/graph), so cargo re-resolved it against crates.io/github every run; without network the gate reported BLOCKED, and `--offline` alone failed because there was no lockfile to resolve against.

- Committed `crates/cross-vm-coordinator/Cargo.lock` (`cargo generate-lockfile` + `cargo fetch --locked`).
- Local gate is now `cargo test --offline --locked --manifest-path crates/cross-vm-coordinator/Cargo.toml` → **PASS 36s** offline in-sandbox, `42 passed` (lib) + `112 passed` (integration), down from 141-363s with network.
- `valkey-coordinator.yml` and `distributed-atomic-chaos.yml` now pass `--locked` so a dependency bump cannot silently change what they build.
- The crate stays out of the workspace on purpose: moving it in would unify features across the whole graph — a separate decision with its own blast radius. The two workflows that drive it are hosted-only, i.e. my local gate is still the only place it runs.

### Consolidated verdict: 18/18 PASS on master `ed37b0f9d` (stamp `20260918T112013Z`)
No red gates remain in the fast set. Per-gate: `format check 8s | agent guards 17s | make gate exit codes 0s | script syntax 5s | workflow wiring 1s | test integrity diff 0s | readiness consistency 5s | workspace check 12s | clippy workspace 71s | clippy runtime rc1 107s | clippy node rc1 180s | test x3-lang 128s | test x3-lang python 3s | test atomic-kernel 80s | test atomic-swap std 7s | test settlement-engine 8s | test node 148s | test cross-vm-coordinator 129s`.

### Next task seed
- Historical-state upgrade rehearsal (the last `triforge_runtime` gap): `x3-chain-node export-state` → `TestExternalities` → hooks.
- Optional now: `./scripts/run-srtool.sh build` to produce the reproducible-build report (`srtool-cli` and the 1.93.0 image are both present).
- The three push `production-gate` runs for recent master commits are still queued behind the two runners; the local CI is the effective gate until that backlog drains.

## 2026-09-18 (round 31) — `cargo test --workspace` is now a gate, and it found a test that wedges CI

### The deep tier (PR #264, master `54bf14937`)
AGENTS.md lists `cargo test --workspace` as required proof; the local CI only ran a curated subset. Added **`--deep`** (also implied by `--all`, `make local-ci-deep`) running `env -u SKIP_WASM_BUILD cargo test --workspace`.

Baseline evidence on master before the fix: `cargo test --workspace` → **5,068 passed, 0 failed, 49 ignored, 358 test binaries**.

### The hang it exposed
The first `--deep` run wedged: `crates/x3-gpu-validator-swarm/tests/stress_harness.rs::stress_test_10k_tps` ran 32+ minutes at load 23 (CPU-burning, not deadlocked) and had to be killed (`kill` on PID of the test binary + its cargo parent — the per-command PID namespace hides it from other execs, but *escalated* commands share the host namespace, so `pgrep -fa` finds it).

**Root cause — the rate limiter was 256× off:**
```rust
let task_interval = Duration::from_micros(1_000_000 / tasks_per_second);  // one *task* (400us)
for _ in 0..batch_size { tokio::spawn(...) }                              // ...but 256 tasks
if elapsed < task_interval { sleep(task_interval - elapsed).await }
```
So a 10k TPS config spawned ~2.5M tasks/second for 5s, each pushing into one contended `Mutex<Vec<f64>>`. Whether it finished or thrashed depended on machine load — it passed in one run and hung in another.

**Fixes:** rate-limit the batch (`batch_interval = batch_size / tasks_per_second`); bound the submitter joins with `tokio::time::timeout(duration + 30s)` so a stuck submitter panics instead of hanging the run; and give `stress_test_10k_tps` real assertions (it previously asserted **nothing** — it could only hang, never fail): `tasks_submitted >= 10_000`, `tasks_completed > 0`, `tasks_failed == 0`.

**Verification:** `Submitted: 50176, Completed: 50176, Failed: 0` for 5s at 10k TPS (previously millions); suite 5 passed in 32.56s / 32.30s / 32.60s across three runs; `./scripts/local-ci.sh --deep --only test-workspace` → **PASS 530s** (it never completed before).

### srtool vanishes for real (operational note)
After installing srtool in round 30 and running `make mainnet-check` successfully, the binary was gone again at 06:31 while `~/.cargo/bin` mtime was *older* than the install (01:58). A probe file written to `~/.cargo/bin` **did** persist across commands, so writes outside the workspace are durable — something on this box re-creates that directory (the same pattern the earlier handoff recorded as "srtool vanished"). Consequences:

- Treat a working `srtool` as **per-session**; run `make srtool-install` (new target, same pinned rev as the CI gate job) before `--release`, and expect the prereq line to say `srtool=MISSING` otherwise.
- The local CI now prints that exact command instead of a bare note, and `docs/local-ci.md` has a Preconditions section.
- The round-30 `make mainnet-check` PASS is real but was measured while the binary existed; the runner's `gate` job installs its own srtool each run, so CI is unaffected.

### Next task seed
- Re-run `--deep` (or `--all`) as the release-candidate check once other work settles; it now completes.
- Historical-state upgrade rehearsal (last `triforge_runtime` gap).
- `tests/stress_with_real_time_metrics.rs` and the other swarm stress binaries passed but are timing-sensitive; consider whether they belong in the default deep tier or a separate `--soak` tier.

## 2026-09-18 (round 32) — the documented CRITICAL verifier blocker is closed (fail closed instead of accept-anything)

### Finding
`docs/reports/SECURITY_BLOCKERS.md` had always recorded it: "All 5 verification router strategies return `accepted: true` unconditionally… if `ExternalBridgesEnabled` is set by governance, these stubs accept any proof." The implementations confirmed it — `ValidatorQuorumVerifier` and `SolanaFinalizedVerifier` accepted any non-empty payload, and the legacy `EvmReceiptVerifier` accepted 64 arbitrary bytes. The crate's own header even documents the rule they broke ("TestOnly verifier … MUST NOT compile in production builds", "Unsupported verifier MUST fail closed") and already carried a `compile_error!` guard for `test-verifier + production`.

The repo also already had the acceptance test: `audit-artifacts/mainnet-readiness/2026-09-05-6a24d8cf-audit/audit-harness/proof` (depends on the router with `features = ["production"]`). It **failed 3/3 before** and **passes 3/3 after**:
```
before: one-byte unsigned proof accepted by production quorum verifier / … Solana verifier / 64 arbitrary bytes accepted by legacy verifier
after:  test result: ok. 3 passed; 0 failed
```

### Changes (PR #265, master `376d9649e`)
- `VerificationError::NotImplemented` ("no verifier implemented for this strategy: failing closed"); `EvmReceiptVerifier`, `ValidatorQuorumVerifier`, `SolanaFinalizedVerifier` return it by default. Permissive behaviour only with `test-verifier` (never with `production`). `X3InternalVerifier` keeps its pass-through with an internal-only note.
- **Relayer EVM path**: deleted the fabricated attestation (`ValidatorId("relayer-main")`, `signature: vec![1]`, `weight: 100` vs a hardcoded 67 threshold) that made the quorum check unconditionally true. The receipt proof verified just above is what satisfies the requirement.
- **Relayer SVM path**: quorum identity was the vector *index* (`svm-validator-{idx}`), so one validator repeated N times satisfied an N-of-M quorum. Identity is now `hex::encode(validator_pubkey)`; a repeat is `DuplicateValidator`. New test `safety_pipeline_rejects_repeated_svm_signer`.
- **Attestation primitive**: `AttestationSet` never compared `statement_hash`, so attestations for unrelated statements counted toward quorum — added `AttestationError::StatementMismatch` + tests (mismatch, empty signature, duplicate weight).
- **Wiring**: the gateway pallet and `tests/e2e` drive plumbing with structural proofs, so `test-verifier` is a *dev*-dependency there (production builds never see it). Two new local-CI gates keep both postures honest: `test verification router` (default = fail closed) and `test verification router test-verifier`.

### Verification
`x3-verification-router` 11 passed (default) / 13 passed (`--features test-verifier`) · `x3-validator-attestation` 5 passed · `x3-relayer` 35+5 passed · `pallet-x3-crosschain-gateway` 49 passed · `e2e gateway_integration_test` 10 passed · audit harness 3 passed (was 0/3) · fast set 19/20 (see below) · **deep workspace suite PASS 635s — 5,072 passed, 0 failed, 49 ignored, 358 binaries**.

### Follow-ups filed
- **#266** — implement the real Solana finalized-proof verifier (Ed25519 attestations vs a validator set). It is now *required* for the SVM relay path to work at all; note the payload contract needs the signed message (`BLAKE2b-256(slot || blockhash)`) to be reconstructible from the envelope, so it also needs a format/version decision.
- **#267** — `BitcoinSpvVerifier` parses `vault_threshold` / `vault_total_signers` (its docs claim SPV deposits must be backed by that many vault signers) but `verify()` never reads them: either enforce or delete the fields.

### Operations
- The box's cargo **registry cache lost `wasm-instrument` mid-session**, so `test cross-vm-coordinator` (which runs `--offline --locked`) failed once. Same class as the srtool/bin disappearance: something rewrites `~/.cargo`. `scripts/local-ci.sh` now classifies the cold-cache signature (`you're using offline mode`) as BLOCKED with the exact `cargo fetch --locked --manifest-path …` command, and the gate passes again after re-fetching.
- `tests/e2e/cross_vm_real_chain_test.rs` fails **in-sandbox** because the node cannot bind `0.0.0.0:30333` ("Operation not permitted"); it passes in escalated runs. Do not read those failures as product defects.

### Next task seed
- #266 is the highest-value security item now: without it the SVM bridge is deliberately closed.
- Re-run `--all` (fast + live + cross + release + deep) once #266/#267 settle.
- The fast set is 19/20 with the coordinator gate dependent on a warm cargo cache; consider pre-fetching in the pre-push hook.

## 2026-09-18 (round 33) — real Solana attestation verification (the other half of #266)

### What the verifier does now (PR #268, master `230e2d05a`)
The relayer has always signed `BLAKE2b-256(slot_le || blockhash)` (`Submitter::sign_proof_payload`), but it handed the verifier `blockhash || (pubkey || sig)*` — **no slot, no version, no count** — so no verifier could reconstruct the signed message. New payload contract (`SOLANA_FINALIZED_FORMAT_V1`, documented at the constant):

```
u8 version(=1) | u64 slot LE | [32] blockhash | u32 count LE | count * ([32] pubkey || [64] sig)
message = BLAKE2b-256(slot_le || blockhash)
```

`SolanaFinalizedVerifier::new(validators, threshold)` verifies each attestation with `verify_strict`, counts only authorized keys, counts each at most once, and requires the threshold. New errors: `UnsupportedProofFormat`, `NoAuthorizedValidators`, `InsufficientValidSignatures`. Crypto deps are `no_std`-safe: `blake2` and `ed25519-dalek` both with `default-features = false`.

**Design lesson (learned the hard way in this PR):** the workspace test build unifies `test-verifier` through the gateway pallet's dev-dependency, so a *test-verifier* branch that skips the real check silently changes what the relayer's tests exercise. `test-verifier` now relaxes only the **unconfigured** case (no validator set), and a configured verifier always verifies for real in every feature configuration. The first version of the PR passed `cargo test -p x3-relayer` standalone and failed the deep workspace run — only the deep gate caught it.

### Evidence
| check | result |
| --- | --- |
| `cargo test -p x3-verification-router` | 18 passed (fail-closed posture) |
| `... --features test-verifier` / `--features production` | 20 / 18 passed |
| `cargo test -p x3-relayer` | 37 + 5 passed (incl. `relayer_svm_payload_verifies_through_the_real_verifier`) |
| pallet / e2e gateway | 49 / 10 passed |
| audit harness (`production`) | 3 passed |
| `scripts/check-runtime-variants.sh` | all six variants PASS |
| `cargo build -p x3-chain-runtime` (wasm) | PASS — caught a `no_std` `Vec` slip first |
| local CI `--deep` | PASS 653s — 5,080 passed, 0 failed, 49 ignored, 358 binaries |

The cross-side test asserts the relayer's `blake2b_simd` digest equals the router's `blake2` digest — if those ever diverge, every signature silently fails, so it is now pinned.

### Still open on #266
Nothing supplies the validator set: `build_router` constructs `SolanaFinalizedVerifier::empty()`, so every Solana deposit route still fails closed. The comment on #266 lists the remaining work (storage + admin setter + genesis for the authorized set, `build_router` using it, a pallet-level positive test, and the slot-vs-cluster keying decision).

### Environment (worse this round)
At 07:36 local the box **removed `cargo` itself** (`~/.cargo/bin` was emptied and re-created at 07:38), so twenty gates "failed" in 0s with `cargo: command not found`; a later wipe took the registry cache again. `scripts/local-ci.sh` now resolves a full toolchain from `~/.rustup/toolchains/*/bin` when `cargo` is absent from PATH, exits 2 with a clear message if none exists, and the coordinator gate runs `cargo fetch --locked` before its offline test. Nothing else on this box is durable across ~30 minutes — treat every tool install as per-session.

### Next task seed
- #266 step 1-3: governance-controlled SVM validator set + `build_router` wiring + an on-chain positive test.
- #267: enforce or delete `BitcoinSpvVerifier`'s unused vault-signer policy.
- `--all` release-candidate run once the above settle.

## 2026-09-18 (round 34) — #266 closed: the SVM bridge path is complete end to end

PR #271 (master `257754e95`) adds the governance-controlled validator set the real verifier needed.

### What landed
- `pallet-x3-crosschain-gateway`: `SvmValidatorSet { threshold, validators: BoundedVec<[u8;32], ConstU32<64>> }` per `ExternalChainId`, plus `set_svm_validators` / `clear_svm_validators` (`GovernanceOrigin` only). Rejects empty sets, zero/too-large thresholds and duplicate keys (`EmptySvmValidatorSet`, `InvalidSvmThreshold`, `DuplicateSvmValidator`); emits `SvmValidatorsSet` / `SvmValidatorsCleared`.
- `build_router` builds `SolanaFinalizedVerifier::new(set.validators, set.threshold)` when a set exists, `empty()` otherwise.
- **`test-verifier` no longer relaxes Solana at all.** The pallet's clearing test proved the escape hatch wrong: clearing the set made proofs pass again under the test feature — "disabled validation looks enabled". Now fail-closed is unconditional; a test that wants a Solana proof accepted must sign it with an authorized key. The pallet's old plumbing test and the two e2e dispatch tests were updated accordingly (the e2e ones now assert *routing*: `NoAuthorizedValidators` = the SVM verifier handled it, `MissingVerifier` = an EVM-only router refused it).
- Added the last missing acceptance case, `solana_tampered_signature_rejected`.

### Evidence
| check | result |
| --- | --- |
| `cargo test -p pallet-x3-crosschain-gateway` | 53 passed |
| `cargo test -p x3-verification-router` | 18 / 19 / 18 (default / test-verifier / production) |
| `cargo test -p x3-relayer` | 37 + 5 passed |
| e2e gateway | 10 passed |
| audit harness (`production`) | 3 passed |
| runtime WASM build + six variants | PASS |
| local CI fast set / `--deep` | **20/20 PASS** / PASS 902s, 0 failures |

### Notes for whoever configures a network
- The validator set is set by governance at runtime; genesis does not seed it. Order of operations: `set_svm_validators(chain, keys, threshold)` **before** enabling a Solana route for that chain.
- `clear_svm_validators(chain)` is a one-transaction halt for a cluster: every Solana proof for the chain then fails closed.
- Threshold is per chain (stored with the set), not per route, so it cannot drift from the key set it belongs to.

### Open issues at handoff
- **#267** — `BitcoinSpvVerifier` parses `vault_threshold`/`vault_total_signers` but never enforces them.
- **#110** — "Complete X3 Foundry production wiring and verification" (pre-existing, not mine).
- Not filed but known: the 26 hosted-only auto-running workflows still need a routing decision, and the historical-state upgrade rehearsal remains the last `triforge_runtime` gap.

### Next task seed
- #267 (small, self-contained: enforce or delete the Bitcoin vault-signer policy).
- Then a `--all` release-candidate run (fast + live + cross + release + deep) as the consolidated pre-release verdict.

## 2026-09-18 (round 35) — #267 closed by deleting an unenforceable claim (+ a new finding)

PR #273 (master `3f1176bce`). `BitcoinSpvVerifier` documented `vault_threshold` /
`vault_total_signers` as an enforced policy and `verify()` never read them.

### Why the fields were deleted rather than "enforced"
Enforcing would have meant counting approvals, and **`BtcVault::add_signer_approval` stores the signature bytes without verifying them** (it checks only that the signer is authorized and not already counted). A 3-of-5 threshold over unverified bytes is the same fake-quorum pattern removed from the verification router, so implementing it would have made the claim false in a worse way. The Bitcoin SPV checks that *are* real (sha256d header linkage, merkle proof, confirmations) are unchanged; `BitcoinSpvVerifier::with_vault_config` now adopts only the confirmation policy, and `from_vault_defaults()` stays.

- New finding filed as **#272** (HIGH) with acceptance criteria; `add_signer_approval`'s parameter is renamed `unverified_signature_bytes` and its docs state the requirement.
- New test `bitcoin_spv_below_confirmation_threshold_rejected` keeps the confirmation policy pinned.

### The ledger was updated, not rewritten
`docs/reports/SECURITY_BLOCKERS.md` keeps the 2026-06-10 audit text and gains a dated status update: finding 1/5 (stub verifiers + gating) resolved per strategy, with the acceptance harness that failed 3/3 before and passes 3/3 now; findings 2 (`MockChainAdapter`) and 3/4 (3,078 `unwrap` / 104 `panic!`) explicitly still open; #267 recorded; #272 added.

### Also landed: a stale lockfile edge from #271
#271 added `ed25519-dalek` as a dev-dependency of `pallet-x3-crosschain-gateway` but the root `Cargo.lock` edge was not committed with it — a `--locked` build on master would have reported the lockfile as stale. Included in #273 (one line) and verified with `cargo metadata --locked` (exit 0).

### Evidence
| check | result |
| --- | --- |
| `cargo test -p x3-verification-router` | 18 / 19 / 18 (default / test-verifier / production) |
| `cargo test -p x3-bitcoin-vault` | 19 passed |
| `cargo test -p pallet-x3-crosschain-gateway` | 53 passed |
| audit harness (`production`) | 3 passed |
| `cargo metadata --locked` | exit 0 |
| local CI fast set | **20/20 PASS** |
| local CI `--deep` | PASS 771s — **5,085 passed, 0 failed, 49 ignored, 358 binaries** |

### Open issues at handoff
- **#272** — vault approval signatures are stored unverified (Bitcoin analogue of the Solana fix).
- **#110** — pre-existing "Complete X3 Foundry production wiring and verification".
- Known, unfiled: the 26 hosted-only auto-running workflows need a routing decision; historical-state upgrade rehearsal is the last `triforge_runtime` gap; `MockChainAdapter` is still compiled in all builds (finding 2 of the ledger).

### Next task seed
- `--all` release-candidate run (fast + live + cross + release + deep) for a consolidated pre-release verdict.
- Then either #272 or the `MockChainAdapter` gating (finding 2) — both are self-contained security items.

## 2026-09-18 (round 36) — the membership change exposed an untested crate; batch runner added

### What happened (PR #277, master `b78d4b1c3`)
The deep suite on the enlarged workspace (from round 35's #275) failed on the **first** run: 3,696 passed, 1 failed —
`x3-evolution::simulator::tests::test_simulation` unwrapped `SimulationFailed("strategy simulation not yet implemented; X3 VM integration pending")`.

**Cause was mine, indirectly, and it is the exact blind spot #274 describes:** `cross-chain-position-manager` path-depends on `crates/x3-evolution`, so adding the position manager to `members` made `x3-evolution` an **automatic workspace member** (path deps of members are members). Its test suites ran for the first time ever. The test was aspirational: `execute_strategy` is a deliberate fail-closed stub (empty bytecode → `Hold`, otherwise that error). The test now pins the real contract (a constructible chromosome must produce the documented error, never a fabricated result), and `Chromosome::from_bytecode` rejects empty input, so the `Hold` branch is unreachable from a constructed chromosome — noted at the call site.

Evidence: `cargo test -p x3-evolution --lib` 38 passed; **deep suite PASS 711s — 368 binaries (was 358), 5,258 passed, 0 failed, 49 ignored**; `clippy workspace` PASS 303s on the enlarged workspace.

### Batch runner (the "build five things, hit the runner once" flow)
`scripts/batch-runner.sh` + `make batch-runner ARGS="--branches a,b,c,d,e"` (or `--ready` for the open PRs):

1. `batch/<utc-stamp>` off `origin/master` in a scratch worktree under `/tmp` (never touches master);
2. merges up to five branches, aborting with the offending branch on conflict;
3. runs the local gate set on the union (`--deep` adds `cargo test --workspace`);
4. pushes the batch branch and dispatches `production-gate.yml` on it — one runner queue slot covers all five, and the queued run is printed.

`--dry-run` (verified end-to-end), `--no-dispatch`, `--keep`, `--max`. It never merges the batch branch itself.

### Standing backlog at this point (for the next agent)
- Open issues: **#272** (vault approval signatures unverified), **#274** (59 crates in the members/exclude gap), **#110** (Foundry wiring, pre-existing).
- `.ai/workspace-membership-baseline.txt`: 59 crates whose tests run nowhere — burn down in batches with the new batch runner.
- Ledger findings still open: `unwrap()` (3,078) and `panic!()` (104) counts.
- Other tracked items: `--all` release-candidate run; 26 hosted-only workflows decision; historical-state upgrade rehearsal; `srtool` + `run-srtool.sh build` report; dedupe `tests_phase4`/`tests_core`; orphan `runtime/src/tests.rs`; dependency advisories; `--soak` tier.

## 2026-09-18 (round 37) — burn-down batch 1: 6 crates fixed, checker made honest (PR #279, master `bc97aad09`)

### The checker was measuring the wrong thing
`scripts/check-workspace-membership.py` compared paths against the `members`/`exclude` **arrays**. A crate that is a path dependency of a member is a member in cargo's eyes without appearing there — that is how `x3-evolution` and `custody-service` joined in round 36. It now reads the real member set from `cargo metadata --no-deps` (arrays only as a fallback). Same tree, honest number: **47 in the gap, not 59**.

### Batch 1 result (baseline 47 → 41)
| crate | tests that now run | work needed |
| --- | --- | --- |
| `quantum-swarm` | 69 | none |
| `x3-constitution` | 9 | none |
| `private-mempool` | 25 | 2 clippy lints |
| `x3-cli` | 4 | none |
| `orchestra` | — | **not taken**: no crate root at all (17 module files, no `src/lib.rs`), four modules import a non-existent `crate::audit`, plus a type error in `scrap.rs`. Restoring means inventing an audit subsystem — left in the baseline with the evidence. |

Adding those pulled 14 more crates in as automatic members (`custody-service`, `cross-chain-gpu-validator`, `x3-evolution`, `x3-sdk`, the `pallet-x3-*` set, `tps-tracker`, `x3-metrics-tracker`, …); their lints are fixed in the same commit (`x3-sdk`: `same_item_push`, 5 × `needless_borrow`, an index loop; `private-mempool`: index loop + redundant closure). **107 tests run that ran nowhere before.**

### Filed
- **#278** — `private-mempool::reconstruct_secret` XORs shares byte-wise while its comment promises Shamir/Lagrange ("Lagrange coefficients needed in production"); a privacy feature advertising a threshold guarantee it does not provide.
- `orchestra` needs a restore-from-lineage-or-delete decision (evidence in the PR body).

### The batch flow, used for real
Dispatched `production-gate` **once** on the batch branch (`gh workflow run production-gate.yml --ref fix/workspace-membership-batch-1` → run `35378800542`, queued behind 3 other runs). That is exactly what `scripts/batch-runner.sh` automates: N changes → one integration branch → one runner queue slot. The run was still queued at handoff; read it next.

### Next task seed
- **Batch 2**: next five off `.ai/workspace-membership-baseline.txt` (41 left) — check `x3-appzone-factory`, `x3-atomic-client`, `x3-bench`, `x3-bot`, `x3-canonical-truth` first, same triage: member / exclude+self-contained / delete.
- The ledger's `unwrap`/`panic` findings and #272 remain the security-flavoured backlog.

## 2026-09-18 (round 38) — burn-down batch 2: 5 crates, 63 tests that ran nowhere (PR #280, master `576f6927f`)

Baseline **41 → 36**. Per crate:

| crate | tests | work |
| --- | --- | --- |
| `x3-appzone-factory` | 25 | 9 of 25 failed on the first run — fixtures wrote into a `src/` they never created (7 sites), two fixtures used `[app]`/`[pallets]` shapes the structs do not parse, one omitted `pallets.toml`, and `test_deploy_config_creation` called `generate_deploy_config` on an empty dir. Added a shared `write_app_zone_fixture` + `write_src_lib`. Also added the missing `templates/basic/` (the factory requires `<templates_dir>/<template>`; the templates were never committed, so "create a zone" could not work at all). |
| `x3-atomic-client` | 10 | `DeclaredAccess` derives `Default`; one justified `too_many_arguments` allow (9 params mirror the on-chain `BundleLeg`). |
| `x3-canonical-truth` | 27 | doc backticks; a test helper takes values by value (allow with a reason). |
| `x3-bench` | 4 | none — and these are the two tests `FEATURE_REGISTRY.toml`'s `x3_reactor` cites, so that registry evidence is executed now. |
| `x3-bot` | — | binary crate; compiles, no tests. |

**One real defect, not a lint:** `initialize_app_zone` had `.replace("substrate = \"4.0\"", "substrate = \"4.0\"")` — a dependency "upgrade" that rewrote the string with itself. Removed with a comment (bumping it needs the release's target version; inventing one would silently change every generated zone). `copy_template_files` also lost an unused `&self` in its recursion.

Follow-up noted in the PR: `AppZoneFactory::templates_dir` defaults to the literal `"templates"` (cwd-relative) rather than `CARGO_MANIFEST_DIR`.

Evidence: 63 tests pass; fmt / clippy (`-D warnings`) / workspace check / membership / script syntax all PASS; membership gate reports 36 in the gap, 0 new, 0 stale.

**Runner:** dispatched `production-gate` once on the batch branch — run `35380824857` (queued behind several master runs). Batch 1's dispatch (`35378800542`) was still queued at handoff too; the fleet is saturated, which is exactly why the batch flow exists.

### Next task seed
- **Batch 3**: next five from the 36 (candidates seen so far: `x3-canonical-truth` is done; look at `x3-cvn-*`, `x3-dns-server`, `x3-drive-*`, `x3-evm-integration`, `x3-foundry-*`).
- Read both dispatched runs when the queue drains; batch branches are safe to keep until then.

## 2026-09-18 (round 39) — burn-down batch 3: stale expectations, and a crate with two API generations (PR #282, master `74d8678bc`)

Baseline **36 → 32** (cumulative 47 → 32 across batches 1–3; **221 tests now run that ran nowhere before**).

| crate | tests | work |
| --- | --- | --- |
| `x3-parallel-executor` | 21 | three tests pinned **placeholders the code had outgrown** (see below) |
| `x3-readiness-report` | 15 | `all_pass_is_ready` set only the original four gates; readiness now requires nine (five RC-1 gates were added later) |
| `x3-runtime-params` | 12 | unused `serde` import, `field_reassign_with_default`, derivable `Default`, and `cargo fmt` across the crate (never formatted) |
| `x3-rpc-policy` | 3 | eleven `doc_markdown` backticks (lib + tests) |
| `x3-swap-router` | — | **not taken** — see #281 |

### The stale-expectation pattern (worth remembering)
These tests failed *because the code got better*:

- `test_access_list_builder` asserted an **empty** access list (*"Access list should be empty for now (simplified implementation)"*) while the builder now maps `0x01`→read, `0x02`→write, `0x03`→both. The test now asserts the real per-instruction keys.
- `test_execution_result_operations` asserted `final_state_hash() == [0; 32]` while `commit_batch` recalculates a BLAKE2 hash of the committed results. It now asserts non-zero, stable across calls, and **identical for identical result sets** (determinism beats a magic value).
- `test_multiple_conflicts` expected 3 conflicts while `check_conflict` documents "one representative key per pair" (so 2). It now asserts the two pairs plus "every reported key is one of the conflicting keys".

**Rule to reuse:** when a crate is first built, expect its tests to encode the *old* placeholder behaviour. Read the implementation before "fixing" the test; three times out of three here the implementation was right.

### Filed: #281 — `crates/x3-swap-router` holds two API generations
`lib.rs` declares **no modules** while `src/` holds 986 lines of them (`fee_calculator`, `routing`, `quote_engine`, `optimization`, `slippage_control`, `atomic_execution`, `gas_optimization`, `mev_protection`, plus an orphaned `tests.rs`) — none compiled, linted or tested. They cannot simply be declared: they target `SwapParams` / `SwapRouterError` / `VmType` while today's `lib.rs` has `SwapRoute` / `RouterError` / `AiRouteOptimizer`. Its 550-line integration test file never imports the crate and asserts tautologies about its own locals. Left in the baseline pending a decision.

### Runner
Third dispatch: run `35382252143` on `fix/workspace-membership-batch-3`. **All three batch dispatches (35378800542, 35380824857, 35382252143) were still queued at handoff** — master is moving fast from the parallel agent, and every push to master queues a `production-gate`, so the queue is deep. Batch branches are safe to keep until their runs drain.

### Next task seed
- **Batch 4**: next five from the 32. Good candidates: `x3-evm-integration`, `x3-dns-server`, `x3-indexer`, `x3-treasury`/`pallets/x3-governance` (check for duplication with the member pallets first), `pallet-x3-control`.
- Read the three batch runs when they drain; if any fails, its branch is still on origin for triage.

## 2026-09-18 (round 40) — burn-down batch 4: a crate that never compiled (PR #284, master `365a8f58a`)

Baseline **32 → 28**. Cumulative across batches 1–4: **47 → 28 crates, 232 tests that now run and ran nowhere before.**

| crate | tests | work |
| --- | --- | --- |
| `x3-gulfstream` | 7 | five compile errors + three lints (below) |
| `x3-turbine` | 9 | one lint + two dead computations in a test |
| `x3-evolution-core` | 1 | none |
| `x3-launch-validator` | 1 | none |

### `x3-gulfstream` had never compiled
- `forwarder.rs` and `mempool.rs` used `GulfstreamError` without importing it (only `GulfstreamResult` was in scope).
- A **duplicate `impl Default for GulfstreamConfig`** in `lib.rs` filled every field except one via `..Default::default()` *inside the impl itself* — E0119 and an infinite recursion waiting to happen. Deleted; `config.rs` has the canonical one.
- `LruCache::new(config.dedup_cache_size)` needs `NonZero<usize>`; a configured 0 now maps to the smallest cache.
- Borrow-after-move in `mempool.rs` (priority read after the entry was moved into the map).
- A `std::sync::RwLock` guard held **across an `await`** in `Forwarder::stop` — the classic deadlock/`!Send` hazard; the guard is dropped before awaiting now.
- Note for a follow-up: neither `Gulfstream` nor `Forwarder` consults its stored `config` yet (the mempool does). Accessors exist so it is at least readable.

### Blocked, filed as #283
`pallets/pallet-x3-control` (pulls a second Substrate git source, `paritytech/substrate @ polkadot-v1.0.0`, `sp-io ^7.0.0`), `pallets/x3-governance` (`wasmi` 0.5 vs the workspace's patched `parity-wasm` adding `Instruction::Bulk`, plus a `[[test]]` target whose file does not exist), `crates/x3-wallet-cli` (inherits `sp-keyring`, absent from `[workspace.dependencies]`).

### Runner queue (operational)
Four batch dispatches are queued — `35378800542` (batch 1), `35380824857` (batch 2), `35382252143` (batch 3), `35383758598` (batch 4) — behind a backlog of master runs from the parallel agent. That is the batch flow working as intended (4 queue slots for 19 crates of change instead of 19 slots), but **the fleet is saturated**, so verdicts will lag well behind the merges. Read them before trusting the release-gate path.

### Next task seed
- **Batch 5**: next five from the 28 (`x3-dns-server`, `x3-marketplace`, `x3-mobile-sdk`, `x3-pq`, `x3-sidecar`, `x3-solvency-sidecar`, `x3-evolution-core` is done, `chronos-flash`, `confidential-gpu`, `dream-mining`, `gpu-sig-verifier`, `import-queue-wrapper`, `loom-concurrency`, `swarm-media`, `voice-to-x3`, `x3-chain-onboarding`, `apotheosis-tx`, `dylint-determinism`, `x3-swarm-*`, `x3-treasury`, `x3-lsp`, `x3-indexer`, `x3-turbine` done…). Remember: `cargo check -p` cannot select a non-member, so add first, then probe.
- Work the four queued runner verdicts when they drain.

## 2026-09-19 (round 41) — x3-swap-router: two generations resolved, deleted the fake one (PR #312, closes #281)

By the time this round started, batches 5-9 and several other burn-downs had
already landed on master by other concurrent agents (this log fell behind —
`git log` and open PRs are more current than this file's "Next task seed"
entries above; check both before trusting this as the latest state).

`crates/x3-swap-router` (issue #274's baseline, issue #281's "two
incompatible generations, pending a decision") turned out to be: a no_std
`lib.rs` with a **fake stub** (`BasicSwapRouter::execute_route` hardcoded
`Ok(U256::from(1000))` regardless of input, "1:1 for demo") that **nothing
else in the repo referenced** (grepped `pallets/`, `runtime/`, every
`Cargo.toml` — zero hits), sitting next to 768 lines across 8 unwired
modules (`fee_calculator`/`routing`/`quote_engine`/`optimization`/
`slippage_control`/`atomic_execution`/`gas_optimization`/`mev_protection`)
implementing a real, complete, already-tested cross-VM (X3VM/EVM/SVM) swap
pipeline that referenced `SwapParams`/`VmType`/`SwapRouterError` — types
defined nowhere in the crate. `tests.rs` (218 lines, never wired via `mod`)
gave the exact field shape needed to reconstruct those three types.

**Decision:** delete the fake stub (zero blast radius, confirmed unused),
make the real pipeline the crate's actual API. Declared the 8 modules,
defined the 3 missing root types from `tests.rs`'s own usage, fixed one
real bug found along the way (`quote_engine.rs` imported `alloc::vec::Vec`
— not a valid path in this crate; `alloc` was never declared — fixed to
use the in-scope `std::Vec`), dropped 4 now-unused deps
(`codec`/`scale-info`/`sp-io`/`sp-runtime`), added `serde` + a `tokio`
dev-dep (`tests.rs` uses `#[tokio::test]`) — both already workspace-pinned.
Added the crate to `members`, removed it from
`.ai/workspace-membership-baseline.txt`.

First-ever `cargo test -p x3-swap-router` run: **37 passed** (12 unit + 3
mev-protection inline + 25 proptest) — this crate's tests had never once
executed before. First-ever clippy run surfaced 6 real `-D warnings`
findings, one of which was a **genuinely tautological property test**
(`prop_lower_fee_path_preferred` returned `true` in every branch of its
own if/else-if/else) — matches issue #281's "asserts tautologies about its
own locals" complaint almost exactly, just in a different function than
the one the issue named. Rewrote it to actually assert fee ordering
matches bps ordering rather than collapsing it to silence the linter.

### Note for whoever reads this next
- `crates/x3-swap-router/tests/prop_swap_math.rs` (550 lines, still
  present) genuinely never imports the crate — its properties are all
  generic arithmetic invariants (fee ⇐ input, monotonicity, etc.) that
  don't call any real `x3-swap-router` function. Left as-is; wiring it to
  the real `FeeCalculator`/`RouteOptimizer` etc. would be a separate,
  larger task, not part of resolving #281's "two generations" collision.
- `.ai/memory/agent-memory.md` is **untracked** (not in git) — it only
  lives in whichever worktree's checkout happens to have it. Other
  worktrees (`/tmp/x3-*`) won't see this entry unless someone copies it
  over; the main worktree at `/home/lojak/Desktop/xxxstar-main` is
  apparently the canonical copy other sessions have been reading.

## 2026-09-19 — x3-lang round 51: byte-stream walkers, finality depth, five agent branches merged

### Facts to discoverable-by-inspection
- `origin/master` is `52c3173ba` at the end of this session (it was `52f79a4b9` at the start). The x3-lang subtree lives at `x3-lang/` inside the repo; the working worktree for x3-lang work is `/tmp/x3lang-merge` (branch `wip/x3lang-objectives-20260918`), and `/tmp/merge-gate` is the scratch worktree used for branch merges (detached, then `fix/settlement-proof-set-gate`).
- The pinned toolchain is 1.90.0; `/tmp/x3lang-cargo.sh <args>` runs it by absolute path (rustup shims get wiped on this box). Warm target dirs: `/tmp/x3-b5-target` (root workspace, 93 GB), `/tmp/x3lang-target-merge` (x3-lang).
- Every cargo invocation needs `require_escalated`: the crates.io cache is wiped between commands, so an unprivileged run fails with `Could not resolve host: index.crates.io`.
- The compiler-stream byte format: instructions start at offset 1 (version byte at 0) and then at multiples of four; a fixed frame is **three** bytes of content (`[opcode][flags][operand_lo]`, the operand's high byte is the pad) except `REQUIRE`, which is **four** (`[opcode][flags][threshold u16]`). `spec/opcodes.rs` is the shared table (`fixed_frame_content_len`, `fixed_frame_operand`, `is_payload_opcode`, `opcode_name`) and is `include!`d by both the compiler and the VM.
- Guard kinds backed by a declaration now include `finality` (`finality_policy { chain <c> requirement <mode> blocks <n> }`). Still unbacked: `risk`, `mainnet_safe`, `audit_gate`.
- `x3c` subcommands that exist and are now tested through the binary: `graph`, `optimize`, `fusion`, `explain`, `check`, `build`, `run`, `audit`.
- pytest lives at `/home/lojak/.local/bin/pytest` (the default `python3` has no pytest module). The x3-lang Python harness (`cli.py`) reads only 2 of 23 examples — a documented drift (TICKET-060), not a regression.

### Decisions made this session
- Fixed the walkers, not the writer: the emitter's layout is self-consistent (pad to the next absolute multiple of four after every instruction), and all the readers were the wrong side. Changing the writer to align the first instruction to 4 would have silently mis-walked every previously emitted artifact.
- `finality` depth rule direction: a guard **below** the declared depth is refused (it would pass at a depth the program's own policy says is not final); a stricter guard is allowed. The declaration is the program's requirement, not a fact about the chain.
- Rejected carrying `blocks N` in the `FinalityExplicit` expression string: a value nothing parses is a claim nothing can check (TICKET-059 instead).
- Merged all five of the other agent's live branches into master, and **did not** merge `fix/x3lang-frame-classification`: its `verify_solver_bond_declared` is an earlier draft of what master already has in a better form (shared `require_guards` enumeration plus the floor-direction check), so merging it would regress master and duplicate the check.
- A hand-resolved `Cargo.lock` is not resolved until cargo agrees: the swap-router merge's lock was ambiguous (two `sp-std` versions, bare name) and `cargo metadata --locked` exited 101 on it. Always run a `--locked` cargo command after resolving a lock conflict.

### Canonical paths chosen
- `x3-lang/compiler/src/semantic.rs` + `compiler/src/lib.rs::ast_level_errors` is where guard-versus-declaration checks go (AST-level, not IR-level), and `compiler/tests/test_guard_declarations.rs` is where their tests go.
- `crates/x3-tools/src/bin/x3c.rs` is the CLI; a compiler module is not a feature until a subcommand runs it and a test in `crates/x3-tools/tests/cli.rs` exercises the binary.

### Known blockers
- The JS suites added by `fix/js-sdk-test-gate` need `npm ci` (network); the gate reports BLOCKED offline by design. Not verified here.
- `srtool` is missing on this box, so `--release` / `make mainnet-check` cannot run its reproducibility section.
- The six non-parsing corpus examples (TICKET-013/014) still fail `check`; nothing in this round changed that.

### Dead ends to avoid
- Counting trace lines with `line.contains("0x")`: the disassembler's version banner (`; x3-lang bytecode v0x01`) matches, so the count is off by one in the direction that looks like a pass. Count the second column.
- `git show --stat --oneline <branch>` shows only the tip commit; use a range (`origin/master..origin/<branch>`) to see all of a branch's commits.
- `git merge-tree <base> <ours> <theirs>` on git 2.34 reports "changed in both" for files that merge cleanly; confirm with a real `git merge --no-commit` in a scratch worktree.
- `str.replace` in a mechanical patch silently no-ops when the anchor is missing, and `python3 - <<PY ... replace(anchor, new)` can hit the *wrong* occurrence (it edited the payload branch instead of the fixed branch once). Always assert the anchor and verify the spot afterwards.

### Next task seed
- TICKET-058 (IF/LOOP records cannot be walked) is the next byte-format item; it is unreachable from the corpus until the six non-parsing examples are adjudicated (TICKET-013/014).
- TICKET-060 (the Python harness reads 2 of 23 examples) is the largest single "second implementation" gap.
- TICKET-061 (proof-supplied block number in the settlement engine) is the highest-severity open finding, and needs a proof-type change rather than a patch.

## 2026-09-19 — x3-lang round 52 (continuation): six more landed

### Facts
- `origin/master` moved to `253cd4cd6` this round: `95a952901` (the last three guard kinds), `ec83c8503` (receipt quote window), `40e8ef44b` (branch refusal), `1c665b5b1` (proof obligation is a mainnet requirement), `4d557483b` (destination payout is not a claim), `253cd4cd6` (conformance cases state the diagnostic they expect).
- x3-lang workspace: 72 suites, 831 tests, 0 failed; clippy `-D warnings` clean; fmt clean; pytest 14; corpus sweep 17/23 check, 17/23 build, **17/23 warning-free** (the corpus no longer emits a single warning), 17/23 run.
- `if`/`loop` in an intent body are refused at *parse* time ("unexpected clause in intent body: KwIf"); they only reach lowering through an `execute { … }` block in a `strategy` (or another statement-accepting body). Measured on a strategy with `if 1 > 0`: build wrote 320 bytes, explain printed the condition as opcodes, run failed `X3_VERIFY_FAILED: OutOfBounds(292)`. Now refused by the IR verifier *and* the emitter, so `check` and `build` agree and no artifact is written.
- `frontier`: guard kinds are all decided or refused (the rule lives in `verify_guard_kinds_are_checkable`, renamed); `verify_proof_requirements` takes the compilation mode; `Condition` has no new variants; the receipt `format_version` is 2 because it now carries `legs`.

### Decisions
- `mainnet_safe` is a *request for the mainnet checks*, not a claim about a mode: the pass runs them when the build is for mainnet or the program asks. Refusing it in dev would make it unwritable where programs live; recording it without running them would be the defect this whole family removes.
- `audit_gate` is refused rather than given an invented `audit` declaration: there is no ledger to check an audit against, so a declaration would be a fake.
- The proof obligation and the "risk" bound are decided by *mode/ceiling direction* respectively, matching the pattern the rest of the language already uses (`verify_mainnet_safe`).
- TICKET-046 (clause words as lexer keywords) was deliberately **not** attempted: it turns ~17 common words (`to`, `amount`, `balance`, `path`, …) into keywords, which forbids them as identifiers anywhere — a corpus-wide parsing risk for little feature value. It stays open with its analysis.
- TICKET-013/014 (the five Solidity-dialect examples) is an owner decision (retire to `examples/legacy/` or implement the roadmap's asset-native `contract`); not touched.

### Dead ends / traps confirmed
- `cargo test` output piped through `grep` in a long-running command can come back empty (buffering): log to a file and grep the file instead.
- Two `apply_patch` calls in this round were silent no-ops because the "changed" line was byte-identical to the "new" line. Always include a neighbouring line that actually changes.
- `git push origin HEAD:master` fails after master moves: `git fetch && git rebase origin/master` first (the x3-lang worktree is `wip/x3lang-objectives-20260918`).

## 2026-09-19 — x3-lang round 53: settlement proof height + the critical finding behind it

### Facts
- `origin/master` at `fa478f2b9`: `1a59b8293` (a proof states the chain height it is about — TICKET-061) and `fa478f2b9` (the chain refuses an external proof nothing binds to its header — TICKET-063 posture).
- `SettlementProof` (pallets/x3-settlement-engine/src/types.rs) has `chain_height: Option<u64>`, stated rather than derived. Both EVM/SVM verify sites refuse a proof without it; `submit_proof` refuses it for every chain; the BTC path additionally refuses a stated height that disagrees with the header it carries.
- `Config::AllowUnboundExternalProofs: Get<bool>` is the posture switch. **The chain runtime sets it `false`** (`spec_version` 11), so EVM/SVM settlement proofs are refused outright until the receipt-to-header binding exists; the pallet's test runtime sets `true` (mock validator) so its lifecycle tests run.
- `pallet-x3-settlement-engine/src/mock.rs::RecordingCrossChainValidator` records the height it was asked about in a thread-local (`VALIDATOR_CALLS`) — that is how "the engine checks the stated height" is testable, since the previous mock accepted anything.
- Building/testing the runtime crate (`cargo test -p x3-chain-runtime --lib <filter>`) takes ~5-10 minutes even warm; the runtime artifacts are in `/tmp/x3-b5-target`.
- **TICKET-063 (critical, funds)**: `verify_settlement_evm_header` compares against `LastEvmHeader`, a single `StorageValue`, and the `merkle_proof` entries are read as the *roots*, never walked as a path — so a proof that copies the latest header's public fields passes for any structurally valid receipt. Full write-up: `.ai/reports/settlement-engine-invented-evidence-20260919.md` finding 3.

### Decisions
- The height fix is plumbing, not protection: before it, the attacker additionally had to grind `tx_hash[0..8]` to the header's block number (2^64 over a receipt they choose) — an accident of the derivation, not a designed barrier. It is a prerequisite for lookup-by-height.
- Fail closed rather than leave the acceptance implicit: a config item that the chain must state, defaulting to refusal, with the reason in the diagnostic. The alternative (leaving it implicit until the MPT check lands) keeps a live forgery path in the node.
- Did **not** attempt a partial MPT verifier: a half-check would be worse than a refusal, and there is no way to validate one against real Ethereum data on this box.

### Next task seed
- TICKET-063 step 1: `merkle_proof` must carry RLP node bytes (a data-contract change: fuzz target, benches, tests, mock) and the EVM path must walk them to the header's receipts root. Step 2: store headers by height. Step 3: carry asset/amount/recipient in the proof and compare to the intent.
- The same "receipt read as roots" shape should be checked in `crates/external-chains` (the other agent's `fix/settlement-proofs-fail-closed` is already in that area).

## 2026-09-19 — x3-lang round 54: the declared finality depth in the artifact

### Facts
- `origin/master` at `b5e7056c9`: TICKET-059 closed — `ir::Condition::FinalityPolicy { name, requirement, blocks }` replaced the rendered `"strict finalized"` string, the emitter writes the depth into the `REQUIRE` operand, and `emit_x3ir`'s disassembler prints a fixed frame's operand for `REQUIRE` (`REQUIRE static 32`, `REQUIRE ge 1`).
- `finality_policy { blocks 0 }` and a depth above `u16::MAX` are now parse errors: the artifact's zero operand means "no depth stated", so a zero depth cannot mean the same number.
- x3-lang: 72 suites, 833 tests, 0 failed; clippy/fmt clean; pytest 14; corpus sweep 17/23 check, 17/23 build, 17/23 warning-free, 17/23 run.
- Master this turn also has `1a59b8293` (the proof states its chain height) and `fa478f2b9` (the chain refuses an unbound external proof; runtime `spec_version` 11).

### Decisions
- The depth travels as a typed IR value rather than in a string, because a number nothing parses is a claim nothing can check (TICKET-044's lesson). The ticket had already rejected the string extension for that reason.
- `blocks 0` is refused rather than treated as "no depth": one operand value must not mean two things.
- The runtime posture for TICKET-063 is a `Get<bool>` config item the chain must state, not a hidden default: the test runtime sets it true (mock validator) and the chain false, so the dangerous path is visible in the runtime's config rather than implied.

### Dead ends / traps
- Bumping `runtime/src/lib.rs`'s `spec_version` is expected for a pallet-config change; nothing in the repo pins the value (checked: the only other `spec_version` literal in a patch file is an unrelated fixture at 100).
- `cargo test -p x3-chain-runtime` rebuilds the runtime crate (~5 min even warm); run it once per runtime change, filtered.

## 2026-09-19 — x3-lang round 55: coded economic diagnostics

### Facts
- `origin/master` at `f25be0d3b`: `DiagnosticCode` gained `AssetTypeMismatch` (`X3E2107`) and `UnresolvedEconomicEffect` (`X3E4021`), `CompilerDiagnostic::into_error()` is the single "CODE: message" renderer (the IR verifier's conversion uses it), and the trading/strategy effects-and-guarantees checks plus the swap/bridge asset-mismatch sites report under those codes.
- The asset-mismatch check in `trading_semantic.rs` cannot be triggered by source text: `parse_trade_swap`/`parse_trade_bridge` set `from_asset = input.asset.clone()`. It is a guard for other front ends (the intent bridge on `salvage/x3lang-intent-bridge` builds ASTs from JSON), and its test hand-builds the AST.
- x3-lang: 72 suites, 834 tests, 0 failed; clippy/fmt clean; pytest 14; corpus sweep 17/23 check, 17/23 build, 17/23 warning-free, 17/23 run.
- AST types for a hand-built program: `x3_lang_ast::{AmountExpr, AssetDecl, AssetId, AtomicTradeDecl, ChainRef, Item, Program, TradeStmt}`, `x3_lang_common::{Span, Spanned, Symbol}`; `AmountExpr::literal(value, symbol)` is the easy constructor; `analyze_trading(&program, mode)` is the public entry point that returns `Err(Vec<X3Error>)`.

### Decisions
- Codes use the spec's numbers (PHASE 52: `X3E-2107`, unresolved economic effect) in the catalogue's existing four-digit spelling, rather than renumbering the existing codes — the conformance manifest pins `X3E0001` and tooling may already key on it.
- Severity is left as the accumulator's channel (errors vs warnings) rather than added as a field: `X3Error` lives in `x3-common` and cannot carry a compiler-crate type without moving `DiagnosticCode` down a layer, which is a bigger change than the ticket asks for. Recorded as remaining.

## 2026-09-19 — round 56: the canonical EVM receipt verifier was dead, and the canonical path for TICKET-063

### Facts
- `origin/master` at `ebc22fa47`. The user merged several PRs during this turn (notably `salvage/x3lang-intent-bridge` #304, so `x3-lang/compiler/src/intent_bridge.rs` is now on master).
- **Four** EVM receipt implementations exist: `crates/x3-verification-router/src/evm_receipt.rs` (real MPT walk, `no_std`+`alloc`+`tiny_keccak`, wired to the relayer — the canonical one), `x3-lang/vm/src/bridge.rs` (a real MPT walk, wired to x3-lang programs, tested with a fixture whose key is `[0x01]` for index 1), `pallets/x3-settlement-engine` (roots-only; TICKET-063), `crates/x3-crosschain-intent/src/proof/evm.rs` (a receipt parser claiming verification, no callers; TICKET-065).
- The canonical verifier had **four** defects until `ebc22fa47`, each alone enough to reject every real proof: `rlp([index])` key instead of `rlp(index)`; header fields read at the abbreviated-list indices (1/2/3 instead of 3/5/6); `decode_u64` left-aligning short big-endian values (100 → 0x6400000000000000, so the confirmations check saturated to 0); and `None` passed as the leaf value where the walk ends in `Some(stored) == value`.
- `verify_merkle_patricia_proof(root, key, value: Option<&[u8]>, proof_rlp)` is the reusable primitive; `receipt_trie_key`/`rlp_index_key` build the key; `encode_proof_payload` builds the wire format; `EvmMerkleRoots` (cross-chain-validator) already stores a root per height.
- The stored "canonical header" is a value an **authorized submitter** asserts (documented in the pallet); `validate_evm_header` only checks the submitted proof's leaves recompute to the claimed root.

### Decisions
- Fixed the canonical verifier rather than porting the VM's walk into the pallet: one implementation, and the pallet can depend on it (`no_std`). The VM's copy stays for now (a separate workspace) and TICKET-066 should consider sharing it.
- Added the verifier's *first positive test*, built from the standard convention rather than the crate's own helper: a test that used the helper on both sides would have agreed with the key bug.
- Left TICKET-065 (the claiming-but-not-verifying third implementation) as the next bounded item rather than folding it into this turn.

### Next task seed
- TICKET-063 step 1: add trie nodes + receipt index to `SettlementProof`, and have the EVM path call `verify_merkle_patricia_proof` against the header's `merkle_root` (stating in the docs that the field must be the receipts root); then flip `AllowUnboundExternalProofs` off and delete the refusal.
- TICKET-065: make `x3-crosschain-intent/proof/evm.rs` delegate (or delete it) and correct `proof/mod.rs`'s wiring claim.
- TICKET-066 (to file): decide whether `x3-lang/vm/src/bridge.rs` should use the router's MPT verifier instead of its own copy.

## 2026-09-19 — round 57: the EVM settlement path settles on an inclusion proof

### Facts
- `origin/master` at `af64a0769` (the user pushed several other PRs during the turn; mine rebased on top).
- `SettlementProof` now carries `receipt_index: Option<u32>` and `trie_proof: Option<BoundedVec<u8, ConstU32<2048>>>` (an RLP list of node byte strings). `verify_evm_receipt_proof` walks the receipt to the declared receipts root via `x3_verification_router::evm_receipt::verify_merkle_patricia_proof`; a missing path, index, or a failed walk returns `Ok(false)`.
- The pallet depends on `x3-verification-router` (`default-features = false`, path dep; that crate is in the root workspace's *exclude* list, which is the normal shape for path-only deps). The pallet's dev-dependencies gained `rlp = "0.5"` so fixtures can build real trie proofs.
- The refusal flag is `AllowUnboundSvmProofs` now (was `AllowUnboundExternalProofs`): EVM does not read it; SVM refuses before inspecting the proof, so even a minimal SVM fixture reaches the descriptive error.
- Test-fixture convention worth reusing: `receipt_trie(receipt_rlp, index) -> (H256, Vec<u8>)` in `pallets/x3-settlement-engine/src/tests.rs` builds the standard single-leaf trie (key `rlp(index)`, leaf `rlp([compact_path, receipt])`, root `keccak(leaf)`) — deliberately not via any helper the verifier shares.

### Decisions
- Delegated instead of porting: one MPT implementation in the workspace, used by the relayer and now the settlement path. The x3-lang VM keeps its own copy (separate workspace) — TICKET-066 should revisit that.
- Kept the single-header comparison (`LastEvmHeader`) rather than switching to `EvmMerkleRoots`: with the walk in place the stricter check is the safer one, and per-height lookup is a liveness improvement (settling older blocks), not a security fix.

### Next task seed
- SVM settlement binding (TICKET-063's remaining half): a bank-hash/account-state proof, or the attestation shape the router's `SolanaFinalizedVerifier` implements — decide which belongs on the settlement path.
- TICKET-065: `x3-crosschain-intent/proof/evm.rs` still claims verification (now that the canonical one works, delegating is a small change).
- TICKET-066: should `x3-lang/vm/src/bridge.rs` share `x3-verification-router`'s MPT verifier?

## 2026-09-19 — round 58: fresh phase audit + PHASE 47 provenance

### Facts
- `origin/master` at `00d6bbb8e` (PHASE 47 landed; the settlement work from the previous turns is `af64a0769`).
- The phase ledger `.ai/reports/x3lang-phase-ledger-20260918.md` was **stale** (written before five rounds): it called 14/15/16/21/23–26/33/34/53-graph ABSENT and 17/40/41/52 PARTIAL, all of which are implemented now. A fresh audit section was appended with per-row evidence, and it is the map to work from.
- Genuinely missing: 9 (hedge), 10 (liquidation), 11 (rebalance), 20 (flash collateral — deliberately out of scope per its own text), 22 (cross-domain netting, partial via fusion), 29 (opportunity packets), 30 (execution lanes), 35 (cost model), 36 (static profitability), 37–39 (arb IR/hyperarb/CEX-DEX), 47 (provenance — now DONE), 50 (benchmarks), 51 (GPU path, partial: the opcode exists).
- PHASE 47 implementation: `crates/x3-tools/src/provenance.rs` (module of the `x3c` binary via `#[path = "../provenance.rs"]`), `x3c build --provenance <path>`, tests in the module (3) and `crates/x3-tools/tests/cli.rs` (1). Fields are measured or `unknown` + a note; the commit is observed by the CLI (`git rev-parse HEAD` in the source's directory) and passed into the library.
- x3-lang: 838 tests, 72 suites, 0 failed; clippy/fmt clean; pytest 14; corpus sweep 17/23 check, 17/23 build, 17/23 warning-free, 17/23 run.

### Decisions
- Audited against the *code* rather than trusting the old ledger, and recorded the audit as evidence — the objective's completion claim depends on this map, not on the corpus sweep.
- For PHASE 47, the provenance document is a sidecar rather than bytes in the artifact: the bytecode format stays fixed (no version bump, no migration) and any tool can read the record.
- Chose provenance over the cost model (35/36) for this turn because it needs no host evidence and no cross-crate table move; the cost model remains the biggest *language* feature gap and is the next candidate.

### Next task seed
- PHASE 35 + 36 (compiler cost model + static route profitability): the VM's gas table lives in `vm/src`, the compiler cannot see it — decide whether the table moves to a shared crate or the estimate lives in `x3-tools` (which sees both), then report per-component figures with their basis, and warn when a declared minimum profit is below the estimated unavoidable cost.
- Then 9–11 (hedge/liquidation/rebalance), 29–30 (packets/lanes), 37–39 (arb surfaces) are each a project; the objectives ledger lists them.

## 2026-09-19 — round 59: PHASE 35 cost model (one table for the charge and the estimate)

### Facts
- `origin/master` at `cf0d83b2d`. The user pushed again mid-turn (`dc4ff55fa`); my commit rebased on top.
- The gas table moved: `spec/opcodes.rs::base_gas_cost(opcode)` is now the one table (the VM's `gas_cost_for_opcode` delegates). It prices codes with the catalogue's constants plus two stragglers (`0x0A`, `0x70`) that no opcode in the catalogue has ever had — kept because the move must not change what the VM charges.
- `compiler/src/emitter.rs` gained `instructions(bytecode) -> Vec<StreamInstruction>` and `metadata_records(bytecode)`; `disassemble` is now written on top of both, so the reader, the estimate and `first_instruction_offset` share one framing rule.
- `compiler/src/cost.rs::estimate_artifact(&bytecode)` → `CostEstimate { instruction_count, base_weight, payload_bytes, host_facing, proof_bytes, artifact_bytes, per_opcode, not_estimated }` + `report()` with the basis of each figure in brackets. `HOST_FACING` names the opcodes that leave the VM.
- `x3c estimate <file>` prints that report. Tests: `compiler/tests/test_cost.rs` (weight recomputed independently; unestimated components named), `crates/x3-tools/tests/cli.rs::cli_estimate_reports_the_cost_of_the_artifact_it_would_write` (an extra guard weighs more).
- x3-lang: 847 tests, 72 suites, 0 failed; clippy/fmt clean; sweep 17/23 in all four columns; pytest 16 (the merged intent-bridge work added two).

### Decisions
- The estimate is computed from the *emitted bytes* rather than from the IR: the bytes carry the opcodes the VM will see, so no IR→opcode mapping has to be written twice (the emitter's arms are the mapping).
- Refused to print numbers for EVM gas / SVM compute / latency / fees: each needs a table or a quote the compiler lacks, and a figure nobody can check is worse than an admitted gap (the rule the mainnet checks follow).
- Fixed a pre-existing `clippy -D warnings` failure in `compiler/src/intent_bridge.rs` (a redundant guard from the merged intent-bridge work): the workspace's clippy gate was red on master.

### Next task seed
- PHASE 36 (static route profitability): with `estimate_artifact` in place, compare a declared minimum-profit floor against *declared* costs (venue `fee_bps` applied to the amounts the route states, plus declared ceilings) and warn with the figures it summed; the spec warns against pretending a static estimate guarantees runtime profitability, so it must be a warning that names its basis.
- Then the remaining missing phases: 9–11, 29–30, 37–39, 50, 51 (each a project), 22 (netting), 20 (deliberately out of scope).

## 2026-09-19 — round 60: PHASE 36 static route profitability

### Facts
- `origin/master` at `d5ac67852` (master had moved to 42b7fb056 during the turn; my commit rebased on top).
- `compiler/src/profitability.rs`: `analyse(program) -> Verdict` (`CannotSatisfy` / `Satisfiable` / `NotAnalysed`) and `warnings(program)`; wired into `run_pre_emission_layers_with_context`, so `x3c check` prints it and `--deny-warnings` refuses.
- The comparison: `require profit >= N` (a *floor*; a ceiling is not read as one) versus `Σ min_output × venue.fee_bps / 10_000` per leg, in the intent's delivered asset unless the guard names one. Assets are never converted; mismatches report NotAnalysed with the reason.
- Two AST traps found while writing it: a `route { … }` block is lowered to `Statement::Atomic`, so any scan of legs must walk nested blocks; and the parser renders `swap <venue> …` as `LiteralExpr::String`, not an identifier.
- x3-lang: 853 tests, 72 suites, 0 failed; clippy/fmt clean; sweep 17/23 in all four columns (the new warning fires on no example); pytest 16.

### Decisions
- A warning, not an error, and the message says it is "a comparison of declarations, not a quote" — the phase itself warns against reading a static estimate as a runtime guarantee.
- Kept the comparison *within* declared data (no prices, no venue quotes) so every figure in the diagnostic is one a reader can point at in the source.
- The corpus sweep is the check that this new warning does not fire on programs that are fine: it stayed 17/23 warning-free.

### Next task seed
- Remaining missing phases: 9 (hedge), 10 (liquidation), 11 (rebalance), 29 (opportunity packets), 30 (execution lanes), 37–39 (arb IR / hyperarb / CEX-DEX), 50 (benchmarks), 51 (GPU path), 22 (cross-domain netting), 20 (deliberately out of scope).
- The trading path's version of PHASE 36 (declared `max_gas`/`max_flash_fee` versus `minimum_net_profit`, same asset rule) is not implemented — the `risk policy` declarations carry the figures, so it is bounded.

## 2026-09-19 — round 61: PHASE 36 soundness

### Facts
- `origin/master` at `ab40f68fa`: the profitability check now compares the floor with **the net at the declared minimum** (`min_output − amount − declared fees`), not with the fee alone. The fee-only version would have warned about a route whose gross absorbs the fee — a false positive in code written an hour earlier.
- The step's input amount lives in `Statement::Swap`'s `route` field (the parser stores `amount <expr>` there; the lowering takes the amount from the matching `from` endpoint instead). Filed as TICKET-067 (rename/one-source).
- Legs that cross assets are skipped: spent and returned are then different assets and prices are host facts.
- x3-lang: 855 tests, 72 suites, 0 failed; clippy/fmt clean; sweep 17/23 in all four columns; pytest 16.

### Decisions
- When a check I ship turns out to be unsound, the next turn fixes the check rather than adding coverage on top of it: a warning that fires on a correct program is worse than no warning.
- Tests named for the *property* that would have been violated (`a_fee_larger_than_the_floor_is_not_enough_to_warn`) so the soundness argument is executable.

### Next task seed
- TICKET-067 (the misleading `route` field): bounded but touches the AST, parser, lowering, formatter and the intent bridge — do it with a `serde(default)` and a lowering test.
- PHASE 36's trading-path half: `risk policy` states ceilings (`max_gas`, `max_flash_fee`), not floors, so no sound unavoidable cost can be derived — the honest outcome is a documented *NotAnalysed* reason rather than a check.
- Remaining missing phases: 9–11, 22, 29–30, 37–39, 50, 51.

## 2026-09-19 — round 62: TICKET-067, the swap step's amount field

### Facts
- `origin/master` at `9be554705`. `Statement::Swap`'s `route` field is now `amount`, with `#[serde(default, alias = "route")]` for stored ASTs; the three readers (lowering `input_amount`, formatter, profitability) were already treating it as the amount.
- `compiler/tests/test_swap_amount.rs` pins the sources: the step's `amount 1_000 min_output 2_000` reaches the swap instruction, the endpoint's `amount 500` reaches the lock, formatting round-trips to identical bytecode, and a statement serialized with the old field name still loads.
- The body-level (function/strategy) swap parser had `let route = if arrow { advance; None } else { None }; let _route_expr = route;` — now it consumes the arrow with a comment naming the endpoint as the amount's source.
- x3-lang: 859 tests, 72 suites, 0 failed; clippy/fmt clean; sweep 17/23 in all four columns; pytest 16.

### Decisions
- The rename is a *data contract* change, so it carries a serde alias rather than breaking stored ASTs; the test builds the renamed JSON from the real AST so it cannot pass against a shape the parser never writes.
- Kept the change to the one quantity and its readers: no other AST field was renamed in the same commit, so a bisect would point at this cause.

### Next task seed
- PHASE 36's trading-path half (document why ceilings are not floors), then the remaining missing phases: 9–11, 22, 29–30, 37–39, 50, 51.
- The settlement thread's next bounded item remains TICKET-063's SVM half and TICKET-065.

## 2026-09-19 — round 63: PHASE 36's trading path is described, not skipped

### Facts
- `origin/master` at `fe60d8582`: `profitability::analyse` now distinguishes "no intent to analyse" from "trading programs declare ceilings, not a cost floor" — the trading grammar has no declared lower bound on cost, so no sound refusal can be produced there, and the reason says what would be needed instead.
- x3-lang: 860 tests, 72 suites, 0 failed; clippy/fmt clean; sweep 17/23 in all four columns; pytest 16.

### Decisions
- PHASE 36 is complete as far as soundness allows: the intent path is checked (net at the declared minimum), the trading path is *decided* with a reason rather than silently skipped or given a check that cannot be sound.
- Same principle as the mainnet checks: a quantity that no declaration backs is refused (or not compared), never approximated.

### Next task seed
- Remaining missing phases: 9 (hedge), 10 (liquidation), 11 (rebalance), 22 (cross-domain netting), 29 (opportunity packets), 30 (execution lanes), 37–39 (arb IR / hyperarb / CEX-DEX), 50 (benchmarks), 51 (GPU path). Each is a project; 20 is deliberately out of scope.
- Settlement thread: TICKET-063's SVM half, TICKET-065 (the claiming EVM verifier).

## 2026-09-19 — round 64: PHASE 9 (atomic hedge) — exposure decided, execution refused

### Facts
- `origin/master` at `7e7360c98` (the user pushed during the turn; my commit rebased on top).
- Surface: `atomic_hedge { buy <n> <ASSET> spot; short equivalent <ASSET> perp; require delta <= <pct>; }` — a new `Item::AtomicHedge(AtomicHedgeDecl)` with `HedgeLeg { side, quantity: Amount|Equivalent, asset, venue: Spot|Perp }`, parsed by `parse_atomic_hedge_item`; the asset may be `chain.ASSET` or bare (chain `unknown`), and two spellings of the same asset are two different references.
- Reasoning: `compiler/src/hedge.rs` — `delta_bps = |long−short| × 10_000 / long`; refuses one leg, two assets, two `equivalent` legs, a missing bound, and a delta above the bound (with the figures). Wired into `ast_level_errors`, so it runs before lowering.
- The net travels in `Operation::Hedge { asset, long, short, delta_bps, delta_bound_bps }`; the **IR verifier refuses it** (no perp venue adapter) and the emitter refuses too (`emit_x3ir` is public), so `check` and `build` agree and no artifact is produced (TICKET-068 for the adapter).
- `require` inside the hedge block arrives as the keyword `KwRequire`, not an Ident — the block parser matches the token.
- Renamed `semantic::slippage_bps_from_expr` → `bound_bps_from_expr` (the unit rule is shared by slippage bounds and the delta bound).
- x3-lang: 867 tests, 72 suites, 0 failed; clippy/fmt clean; sweep 17/23 in all four columns; pytest 16.

### Decisions
- The phase's "verifier must reason about directional exposure" is implemented fully; the phase does *not* ask for perp execution, and pretending it (an IR op the VM cannot dispatch) would be the fake-adapter defect. Refusing at the IR verifier and the emitter is the honest end.
- `equivalent` resolves to the other side's written size, with two `equivalent` legs refused (nothing to be equivalent to) rather than defaulting to zero.

### Next task seed
- PHASE 10 (atomic liquidation) and PHASE 11 (portfolio rebalancing) are the next surface features; both can follow this shape (surface + a *decided* property + an honest execution verdict).
- TICKET-068 needs a perp venue adapter before any hedge artifact can exist.

## 2026-09-19 — round 65: PHASE 10 (atomic liquidation)

### Facts
- `origin/master` at `89e120484` (the user pushed during the turn; my commit rebased on top).
- Surface: `atomic_liquidation { liquidate <n> <ASSET> of <ref>; receive <n> <ASSET> collateral; swap <n> <A> -> <B> min_output <n>; repay <n> <ASSET>; require net_profit >= <n> <ASSET>; }` — a new `Item::AtomicLiquidation(AtomicLiquidationDecl)`; every amount is required (a verifier asked to check repayment without knowing what was advanced has nothing to check) and the position reference is carried verbatim (`borrower.position`).
- Reasoning: `compiler/src/liquidation.rs::ledger` — the swap's `min_output` must cover `repay`; the swap must convert *all* the collateral; the collateral must be what the swap spends; the swap must produce the repaid asset; the floor must be in the repaid asset and ≤ `min_output − repaid`; zero amounts refused. Errors name the figures.
- `Operation::Liquidation` carries the ledger; the IR verifier refuses it (no lending-protocol adapter) and the emitter refuses too → `check` and `build` agree, no artifact (TICKET-069).
- Parse traps: inside the block, `swap` is `KwSwap` and `require` is `KwRequire` (keywords, not identifiers); and a dotted reference needs an explicit loop (`name.push('.')` + the next ident).
- New parser helper `expect_uint(what)` for clauses that need a number the compiler can evaluate.
- x3-lang: 875 tests, 72 suites, 0 failed; clippy/fmt clean; sweep 17/23 in all four columns; pytest 16.

### Decisions
- Same shape as PHASE 9 deliberately: a surface whose *decided* property is the phase's stated verifier duty, plus an honest execution verdict instead of a fake adapter.
- The spec's sketch writes `liquidate borrower.position;` with no amount; the surface requires amounts and says why in the AST doc — an unbacked check is the defect the earlier rounds kept finding.

### Next task seed
- PHASE 11 (portfolio rebalancing: `rebalance portfolio { weights; minimize { … }; atomic; }`) is the next surface; its *decided* property is that the weights sum to 100% and that the plan's legs can reach them, which needs the same per-asset discipline.
- TICKET-068/069 (perp venue and lending-protocol adapters) would let the decided hedges and liquidations actually run.

## 2026-09-19 — PHASE 29 (opportunity packets)

### Facts
- New: `x3-lang/vm/src/opportunity_packet.rs` (+ `vm/tests/opportunity_packets.rs`), module wired in `vm/src/lib.rs`. `OpportunityPacket` carries `strategy_id`, `artifact_hash`, `state_roots`, `route: compiler::opportunity::Opportunity` (reused, not re-modelled), `required_capital`/`max_capital`, `expected_output`, `minimum_profit`, `maximum_fee`, `maximum_slippage_bps`, `deadline_blocks`, `proof_requirements`, `execution_commitment`, `packet_hash`, `signature`.
- The five phase adjectives are mechanisms, not labels: version checked against `OPPORTUNITY_PACKET_VERSION` and carried in both hash domains; `packet_hash = SHA256(PACKET_DOMAIN ‖ bincode(packet with hash+signature cleared))`; determinism via `BTreeMap`/`BTreeSet` + bincode (test builds the same packet in two insertion orders); ed25519 signature over the packet hash verified only against a caller-supplied trusted map; `OpportunityPacketLedger::admit` refuses a repeated packet hash.
- Decided properties: inverted capital window, unreachable profit floor *after the packet's own worst-case fee* (`expected_output − required_capital − maximum_fee ≥ minimum_profit`, all `checked_*`), route liquidity below `max_capital` (reusing the graph's `min_liquidity` meaning), route fee at max capital above `maximum_fee` (with an overflow variant), route slippage above `maximum_slippage_bps` or a ceiling above `MAX_SLIPPAGE_BPS = 10_000`, empty/mismatched route, unnamed venue, no/zero state root, zero artifact hash, empty strategy id, unnamed proof requirement, commitment and hash mismatches, expiry, missing/untrusted/invalid signature.
- Deliberately not decided (TICKET-071): whether the opportunity is *real* — state-root freshness, venue prices, strategy-commitment linkage. An empty `proof_requirements` set means "no proof attached" and is allowed; the proof vocabulary belongs to hosts/adapters.
- `execution_commitment` excludes `strategy_id` on purpose: the packet sells its execution and hides the strategy. `validate_commitments` checks the execution commitment *first*, so a term edit reports that commitment and a `strategy_id` edit reports the packet hash.
- CLI: `x3c packet inspect` (prints as read, recomputes nothing) and `x3c packet verify --block N --trusted <key_id>=<hex>` (repeatable trusted keys, required — a key carried inside the packet is not a check).

### Decisions
- `signature: Vec<u8>`, not `[u8; 64]`: serde implements neither `Serialize` nor `Deserialize` for arrays longer than 32, so `[u8; 64]` fails to derive (four compile errors). This is why `ReceiptAttestation` uses `Vec<u8>`; the length is checked with `try_into` before use.
- Reused `compiler::opportunity::Opportunity` as the packet's route rather than defining a packet-local leg type: the phase-14 graph already decides venues/assets/fee/slippage/liquidity, and the packet's job is to carry a *decided* route.
- No `x3c packet build` and no signer in the CLI: generating a packet from a program is the unimplemented generator chain (PHASE 37's `TICKET-070`), and a dev-key signer would be a fixture pretending to be a solver.

### Process notes (expensive to rediscover)
- A peer agent added `Item::Arb` to `x3-ast` while `compiler/src/formatter.rs` had no arm for it, so the whole workspace failed to compile for ~5 minutes (E0004) and neither agent could run tests. Their fix landed at 10:54. **Unblock pattern:** `git archive HEAD <paths> | tar -x -C /tmp/tree`, copy your own files in, run the proof there. Also copy `rust-toolchain.toml` and the root `scripts/` + `docs/x3-lang/fixtures/` needed by the bridge tests, or the run is contaminated.
- **Toolchain:** the repo pins 1.90.0 in `rust-toolchain.toml`. Outside the repo root the machine default (1.98.1) is used and clippy then reports five lints in *pristine* compiler code. Clippy results from outside the repo root are not evidence about this repository.
- Bare `rustfmt <files>` ignores `x3-lang/rustfmt.toml` (formats to 100 columns); `cargo fmt -p <pkg>` honours max_width 120.
- Editing a packet term trips `ExecutionCommitmentMismatch` before `HashMismatch`; three tests initially asserted the wrong one.

### Proof
- `cargo check --workspace` → exit 0; `cargo test --workspace --no-fail-fast` → 0 failed (97 vm lib incl. ~20 packet unit tests, 4 `opportunity_packets` integration tests, 37 cli, 8 cli_integration); `cargo clippy --workspace --all-targets -- -D warnings` → exit 0; `cargo fmt --all -- --check` → exit 0; `.venv/bin/python -m pytest -q x3-lang/tests` → 16 passed; fake-code scan over the new files → no matches. Note `.venv/bin/python` (not `python3`, and not `python`).
- Logs: `.ai/runlogs/x3lang-phase29-packets-tests.log`, `-main.log`, `-packets.log`. Report: `.ai/reports/x3lang-phase29-packets-20260919.md`.

### Next task seed
- **PHASE 30 (dedicated execution lanes)** is the immediate consumer: lane classes (standard / trading / atomic cross-domain / liquidation / settlement) admitted through this packet ledger, with a deterministic, documented, auditable scheduling policy and no unfair ordering.
- `TICKET-071` — host evidence for packet realism (state-root freshness, venue price attestation, strategy-commitment linkage).

### Ticket-number correction (2026-09-19, after the fact)
The PHASE 29 entry above refers to "TICKET-071" for host evidence / packet realism and to
"PHASE 37's TICKET-070" for the arb generator. Those numbers were placeholders written
before other agents claimed them in the shared ledger, and both now point elsewhere:
- **TICKET-070** = a rebalance's target portfolio has no transaction graph (PHASE 11).
- **TICKET-071** = a netting book reduces to a residual nothing can settle (PHASE 22).
- **TICKET-072** = `Lock.from` is the payer to one producer and the payee to another.
- **TICKET-073** = an arb scope has no pipeline that turns it into legs (PHASE 37).
- **TICKET-074** = an opportunity packet cannot say whether the opportunity is real
  (PHASE 29) — this is the entry the PHASE 29 note above meant.
Lesson: claim a ticket number by appending to `.ai/reports/x3lang-tickets-20260918.md`
*before* referring to it from memory, or the reference drifts.

## 2026-09-19 — PHASE 22 (cross-domain netting) and PHASE 37 (arbitrage IR)

### Facts
- **PHASE 22** landed at `99c122b5b`. Surface: `netting <name> { consent <party>; <debtor> owes
  <n> <chain.ASSET> to <creditor>; }` (`Item::Netting` → `Operation::Netting`). Analysis in
  `compiler/src/netting.rs`. Obligations are offset **one `(domain, asset)` group at a time**;
  `Book::preserves_net_positions()` is checked inside `netting::book`, so a run that would move a
  party's position fails the compilation rather than the test suite. CLI: `x3c netting` (a *separate*
  subcommand, deliberately — `x3c check` refuses a book at the IR layer because a party is a symbol,
  not an account, so folding the report into `x3c fusion` would have made it unreachable).
- **PHASE 37** landed at `db536ab4c`. Surface: `arb <name> { discover { chains/max_hops/liquidity_min }
  capital { flash/max } execution { atomic/parallel/private } risk { min_profit/max_slippage/
  max_total_fee/deadline } }` (`Item::Arb` → `Operation::Arb`). Analysis in `compiler/src/arb.rs`.
  `arb::STAGES` maps the phase's 7 pipeline stages onto the 5 modules that implement them and names the
  2 that do not ("Execution Plan", "Atomic Settlement"); `missing_stages()` is quoted in the
  verifier/emitter refusal, and a test asserts every named module file exists on disk.

### Decisions
- **A declared bound with nothing enforcing it is a compilation error, not a comment.** `arb`
  answers `risk.min_profit` against the program's own `require profit >= …` guards via
  `arb::enforcement` (three refusal shapes: no guard, a guard permitting less, a guard whose bound
  is not in bps). This is PHASE 15's own "label nothing acts on" defect made fatal.
- **No placeholder fields.** `ArbPolicy` originally carried an `enforcement` field that `policy()`
  left as `Unguarded` for `verify()` to fill in. My own test caught it: the field lied whenever it
  was read directly. The field was removed and `enforcement(program, floor)` is the only door.
- **Require the unit on a bps clause whose name does not say bps.** `min_profit = 20` is ambiguous
  (bps or USDC), so `parse_arb_bps` requires the `bps` unit while `risk_policy`'s `max_slippage_bps`
  does not need one. The phase's own `20bps` spelling works: the lexer joins `20bps` into one
  identifier, so the reader splits it.
- Netting never offsets unlike assets (a price claim, as `hedge.rs` refuses) or unlike ledgers (a
  bridge's claim), and never a party that did not consent. Refused pairs are *named* in the report.

### Process notes (expensive to rediscover)
- **`Item` has no catch-all in `formatter.rs`.** Adding a variant to `Item` breaks the whole
  workspace until the formatter arm exists — a peer agent lost ~5 minutes to my in-flight PHASE 37
  edit for exactly this reason (E0004). Add the AST variant, the formatter arm and the lowering arm
  in one pass.
- **Indented blocks in a module doc comment are doctests.** The PHASE 22 module doc's grammar sketch
  failed `cargo test` with "expected one of `!` or `::`, found `book_a`". Fence a grammar sketch as
  ```` ```text ````.
- **`\`+newline inside a string literal keeps the next line's indentation when a tool builds the
  literal.** Python line-continuation collapses the newline but *not* the spaces, so a message
  patched that way bakes a 22-space run into the diagnostic. Rust's own `\`+newline does strip it, so
  heredoc-written code is fine and script-patched code is not. Two of my messages needed repairing;
  the CLI test now greps the rendered refusal for `"  "`. Pre-existing instance at
  `compiler/src/parser.rs:1719` (license-field message) — not mine, not fixed.
- **Rust does not concatenate adjacent string literals.** `format!("a" "b")` is a compile error;
  merge into one literal or use `concat!`.
- `atomic` is a **keyword token**. Inside a block, read a clause name with `peek_word()` (which maps
  `Tok::KwAtomic` → `"atomic"`), never `expect_ident`. `true`/`false` are `Tok::KwTrue`/`KwFalse`,
  which `peek_word()` does *not* cover, so a flag reader must match those explicitly.
- Two pre-existing duplicate ticket numbers in the ledger: 002, 013, 027, 045.
- **Sub-agent message delivery is unreliable here.** Three agents were spawned with full briefs and
  reported "no task in my inbox"; a later re-send reached two of them but not the third. Budget for
  doing the work yourself.

### Next task seed
- **PHASE 38 (hyperarb)** and **PHASE 39 (CEX/DEX intents)** are the remaining arbitrage phases.
  `arb_ir` has been assigned 38 with the `arb.rs` pattern as the model. 39 is about honesty: a CEX
  leg must declare which of {trusted adapter, escrow, pre-funded account, attested execution,
  compensating action} it relies on, and an `atomic` claim without an enforceable settlement
  semantic must be refused. Check `VenueDecl`/`VenueKind` first — there is already a venue model to
  extend rather than a second one to create.
- **TICKET-073** is the arb pipeline (the largest, most valuable follow-up); **TICKET-071** the
  netting settler; **TICKET-070** the rebalance graph; **TICKET-074** packet realism.
- **TICKET-065** is a verified critical defect (`crates/x3-crosschain-intent/src/proof/evm.rs`
  advertises MPT verification with no trust anchor; `compute_receipt_trie_root` ignores its
  `_index`/`_total_receipts`; `proof/mod.rs:14` claims a `ProofVerifier` wiring that exists
  nowhere). `crates/x3-verification-router/src/evm_receipt.rs` has the real verifier. Assigned.

## 2026-09-19 — PHASE 29 (opportunity packets) merged, and two duplicate-work findings

### Facts
- **PHASE 29** is on master at `920a44775`. It had been sitting **uncommitted in the
  canonical working tree** (`/home/lojak/Desktop/xxxstar-main/x3-lang/...`) rather than in a
  scratch clone, so it was invisible to `git log` and at risk of being lost. It was preserved
  onto `wip/x3lang-preserve-packets-and-arbitrage-20260919` first and then cherry-picked onto
  master with only its non-duplicative parts (vm module + tests + `packet` CLI + CLI tests).
- **A second PHASE 37 exists.** `compiler/src/arbitrage.rs` (unrebased, on the same
  preservation branch) is an independent implementation of the same phase as master's
  `compiler/src/arb.rs`. TICKET-076 decides which survives; TICKET-077 records that the
  branch is archival until rebased. Both pass their own tests.
- Master after this session's merges: `99c122b5b` (22) → `db536ab4c` (37) → `44a12ba94` (39)
  → `920a44775` (29).

### Process notes (expensive to rediscover)
- **Check the canonical working tree before assuming work is lost or unstarted.** Other
  agents write directly into `/home/lojak/Desktop/xxxstar-main` instead of a clone; `git
  status --short -- x3-lang` there is the only place their in-flight work is visible. Preserve
  it with `git diff -- x3-lang > patch` **plus a copy of each untracked file** — `git diff`
  does not include untracked files, and there were four.
- **`git clone --shared` sets `origin` to the local source path, not the GitHub URL.** A
  `git push origin <branch>` then pushes into the canonical repo's refs (and `HEAD -> master`
  is rejected with "branch is currently checked out" because master is checked out there).
  Always `git remote set-url origin https://github.com/Cyptopimpinainteazy/xxxstar.git` in a
  scratch clone before pushing.
- Cherry-picking one agent's work out of a mixed uncommitted tree: split the per-file diff
  into hunks, keep only the hunks whose *added* lines mention the feature, then for a test
  file whose context lines have drifted, take the `+` lines and append them instead of
  applying the hunk.
- The canonical repo's local `master` is far behind `origin/master`; `git log` there is not a
  statement about what has landed.

### Next task seed
- **TICKET-076** is the highest-value open item: reconcile the two `arb` implementations into
  one surface carrying both sets of checks (graph-grounded `OpportunityConstraints`
  validation from `arbitrage.rs`, guard-enforcement and the stage mapping from `arb.rs`).
- Rows still missing: **30** (execution lanes), **38** (hyperarb), **50** (p50/p95/p99
  benchmarks), **51** (GPU path + CPU/GPU equality).
- **TICKET-065** (the dead EVM "verifier") is a verified critical defect and is assigned.

## 2026-09-19 — the duplicate PHASE 37 is resolved: an `arb` scope is judged against the graph

### Facts
- `fb426988e` on master closes TICKET-076 and TICKET-077. `compiler/src/arb.rs` is the single
  `arb` surface. The other implementation's best idea — judge the clauses against
  `opportunity.rs` rather than beside it — is now `arb::venue_standings(program, policy)`:
  every declared venue is judged against the declaration's own bounds using the graph's own
  numbers (`fee_bps`, `liquidity`, `slippage_bps`, and the chain the venue settles on), and
  `arb::graph_grounding` refuses a scope no venue survives, naming **every venue and the
  bound that removed it**. A program that declares a scope and no venue is refused too.
- A venue is in scope when the chain it settles on **or** a chain either of its assets lives
  on is declared — a cross-domain venue should not need three lines to be visible.
- One surviving venue is enough. The weakest claim the graph can support is that one declared
  venue survives every bound as a one-hop candidate.
- Deliberately **not** carried over from `arbitrage.rs`: it allowed
  `capital { flash = enabled }` when a declared flash venue covered the ceiling. PHASE 20
  forbids shipping flash collateral before a formal safety proof, so allowing it would permit
  a claim the phase forbids. The surviving `arb` still refuses flash outright.

### Process notes (expensive to rediscover)
- **Adding a program-level check to `arb::verify` silently invalidated 26 tests and one CLI
  test.** They built `arb` programs with no `venue` declarations, and `x3c check` then stops
  at the AST layer with the new refusal before reaching the pipeline refusal those tests were
  about. Fix: give the shared test fixture venue declarations (`VENUES` in `test_arb.rs`) and
  give the CLI fixtures the same. When you add a check, grep for the sources it will reject.
- `u128`'s `Display` does not group digits, so a diagnostic says `500000`, not `500_000`. A
  test asserting the underscored form fails. Match the number as the formatter renders it.
- `verify` accumulated its checks with `match policy(decl) { Ok(decided) => match enforcement… }`
  and I needed to add a third check; a bare `if let Ok(decided) = &decided` after the `match`
  does not compile because the match's value is discarded. Bind the result first and
  `continue` on the error arm.

### Next task seed
- Rows still missing: **30** (execution lanes), **38** (hyperarb), **50** (p50/p95/p99
  benchmarks), **51** (GPU path + CPU/GPU equality). 20 is out of scope by its own text.
- **TICKET-065** (the dead EVM "verifier" with no trust anchor) remains a verified critical
  defect and is the highest-value open item now that 076/077 are closed.
- The `arb` pipeline generator is **TICKET-073**; `arb::STAGES` already names the two stages
  with no implementation, so a generator starts from that list.
## 2026-09-19 — root agent: stale-base audit, PHASE 37 duplicate withdrawn, PHASE 29 landed

### Facts
- `origin/master` was three commits ahead of the local clone's `master` for most of the session:
  `99c122b5b` (PHASE 22 netting), `db536ab4c` (PHASE 37 arb), `44a12ba94` (PHASE 39 off-chain venue
  settlement). Local `master` was `0a68cb883`. **Check `git log origin/master` before starting any
  phase** — `git log` in the local clone is not a statement about what has landed.
- PHASE 29 (opportunity packets) was finished by its agent but left uncommitted in the main tree. It is
  now committed (`71d8e26e4`) and rebased onto `origin/master`; local `master` is `920a44775`.
  The single rebase conflict was `crates/x3-tools/tests/cli.rs`, where PHASE 22's netting tests and the
  packet tests were both inserted at the same location — resolution: keep both blocks, delete the
  markers. `x3c.rs` and `vm/src/lib.rs` auto-merged.
- Integration proof on the rebased tree: 981 tests pass / 0 fail across the x3-lang workspace, clippy
  `--workspace --all-targets -D warnings` clean, fmt clean, 16 pytest, example sweep 17/23.
- `wip/x3lang-arb-graph-filter-20260919` (`bdc83ba7f`, based on `44a12ba94`) adds the missing half of
  PHASE 37: `arb::search_constraints` + `arb::admitted_venues` run the declared scope through the
  canonical filter (`reject_reason` / `path_reject_reason`) and refuse a scope whose own bounds admit
  no venue; `OpportunityConstraints::allowed_chains` + `RejectionReason::ChainNotAllowed` make
  `chains = [...]` a set the search enforces rather than a count. The deadline is deliberately not
  compared (blocks vs milliseconds; the block conversion rounds permissively for a filter).

### Decisions
- A duplicate phase is withdrawn rather than landed, even when the local implementation is good: two
  `arb` surfaces in one compiler is the defect the duplicate-work gate names. The withdrawn patch and
  files are kept at `/tmp/root-withdrawn-phase37/` and its one additive idea was re-implemented on the
  landed code.
- Another agent's finished-but-uncommitted work is committed on its behalf when the author's turn has
  ended and it says the commit is the root's call; commit exactly the paths that work touched, never
  `git add -A` in a shared tree.

### Process notes (expensive to rediscover)
- `git commit`/`git rebase`/`git worktree add` need escalation in this environment (`.git` is
  read-only in the sandbox); `git diff`/`git show`/`git log` do not.
- `git apply -R <patch>` is the clean way to withdraw your own uncommitted work: save the diff first,
  reverse-apply it, delete the new files, and the tree is exactly as it was.
- The `arb` surface landed as `arb <name> { discover { … } … }` with `Operation::Arb`, not the
  unnamed `arb { … }` of the spec sketch, and `policy()` refuses `flash = enabled` (PHASE 20) and
  `private = true` (no private path exists). Read `compiler/src/arb.rs` before touching PHASE 38.

### Next task seed
- Land `wip/x3lang-arb-graph-filter-20260919` (trivial rebase onto `920a44775`), then **TICKET-065**
  (the dead EVM verifier: delete or delegate to `x3-verification-router`), then the remaining rows
  **30** (execution lanes), **38** (hyperarb), **50**/**51** (benchmarks, GPU path).

## 2026-09-19 — PHASE 38 (hyperarb) landed, and a silent-drop trap in lowering

### Facts
- `bdb8c40e8` on master: `hyperarb <name> { capital = <n> <ASSET>; parallel { route_x =
  evaluate(<target>); … } choose <criterion>; hedge volatility; settle_across_domains;
  require net_profit >= <n>bps; }`. Analysis in `compiler/src/hyperarb.rs`; AST
  `Item::Hyperarb(HyperarbDecl)`; IR `Operation::Hyperarb`; CLI-visible through `x3c check`
  (refusal) and `x3c lower` (the decided plan).
- Every leg is **resolved**: a target names a declared venue, a chain a venue settles on or
  an asset lives on, or a venue's `domain`. Anything else is refused with the lists of what
  the program does declare. The plan records *what each leg resolved to* (`the venue
  'uniswap_v3'`), so a reader does not re-resolve it.
- Refused because the clause would be a label: `capital = flash(…)` (PHASE 20 forbids flash
  collateral before a formal safety proof; the flash form is parsed so the refusal quotes the
  amount), `hedge volatility` with no `atomic_hedge` in the program, `settle_across_domains`
  over one domain. Also: fewer than two legs, a repeated leg, zero capital, a zero floor, and
  a program with no venue at all.
- `Item::Hyperarb` is lowered **explicitly**. Two stages (`Execution Plan`, `Atomic
  Settlement`) are cited from `arb::missing_stages()`, so PHASE 37 and 38 refuse with one
  wording.

### Decisions
- `hyperarb` reuses PHASE 37's stage mapping rather than restating it: the two phases need the
  same pipeline, and a second list of missing stages would drift.
- The spec's `evaluate(EVM_PATH)` sketch is read as "name something the program declares", and
  the refusal lists the venues/chains/domains so a typo is diagnosable. There is no free-text
  path vocabulary in this language and inventing one would be a second routing model.

### Process notes (expensive to rediscover)
- **`lowering.rs:835` is `_ => {}`** with the comment "other items generate no operations".
  It is deliberate and it is a trap: a new `Item` variant is silently omitted from the IR, so
  `x3c lower`/`build` succeed and the declaration is simply absent from the artifact. PHASE 38
  would have shipped a `hyperarb` that lowered to nothing. TICKET-078 asks for the catch-all
  to become an explicit list so a new variant fails to compile.
- **`x3c fmt` corrupted an `atomic_hedge`.** The formatter emitted `require delta <= 1 bps;`
  and the delta guard accepted only `0.01%` or a bare count, so `bps` was an unexpected token
  and the formatted program would not parse. Fixed in `bdb8c40e8`: the formatter emits the
  documented count and the guard accepts the unit (all three spellings = 1 bp). Lesson: a
  formatter is only correct if its output re-parses; `test_formatter_roundtrip.rs` did not
  cover the hedge.
- `AtomicHedgeDecl` has **no name** — it is `{ legs, delta_bound_bps }`. A check that wants to
  refer to "the hedge" has to refer to its bound, not a name.
- `Tok` has `Ge`/`Gt`/`Le`/`Lt`, not `GtEq`/`LtEq`. And `require` is `Tok::KwRequire`, so a
  clause starting with it must be matched on the keyword, not the word.

### Next task seed
- Rows still missing: **30** (execution lanes), **50** (p50/p95/p99 benchmarks), **51** (GPU
  path + CPU/GPU equality). 20 is out of scope by its own text.
- **TICKET-073** is now the single generator both PHASE 37 and PHASE 38 wait on: `arb::STAGES`
  names the two missing stages, and `Operation::{Arb, Hyperarb}` both refuse over them.
- **TICKET-065** (the dead EVM "verifier") is still the highest-value non-phase item.
- **TICKET-078** is a cheap, real robustness win in `lowering.rs`.

## 2026-09-19 — PHASE 30 (execution lanes)

### Facts
- `ed70c526e` on master: `compiler/src/lanes.rs` + `x3c lanes`. Five lanes (`liquidation`,
  `atomic_cross_domain`, `trading`, `settlement`, `standard`) decided by `lanes::classify(ir)`
  from the lowered `Operation`s.
- The classification is a **precedence** (`lanes::PRIORITY`, most constrained first), because a
  program is often several things at once. Two cases are decided from the operations rather
  than a label: a `Swap` whose two chains differ is cross-domain with no `Bridge` operation, and
  a `ParallelPlan` whose legs share one domain is *not* cross-domain.
- The scheduling policy is three rules: arrival order within a lane; `PRIORITY` across lanes;
  and no participant-controlled input at all. `Queued` is `{name, lane}` and `schedule` takes
  nothing else, deliberately — a market in lane position is the unfair mechanism PHASE 30
  forbids, and having no parameter for one is the structural guarantee. A test destructures
  `Queued` so adding such a field is a visible decision.
- `classify_program` returns `Result<Lane, String>` per declaration; `x3c lanes` prints
  `NOT CLASSIFIED — <reason>` for one that does not lower.

### Decisions
- **A declaration that does not lower gets no lane.** The first version mapped a lowering
  failure to `Lane::Standard`, which would have made "standard" mean both "does nothing that
  needs a lane" and "could not be read". My own test caught it (a fixture with invalid
  `atomic_swap` syntax came back as `standard` rather than reporting that it did not lower).
- Nothing attaches a lane to a real queue: the VM has no scheduler object, so the module says
  the classification and ordering *are* the policy and an executor with a queue can use it. It
  does not claim to be wired in.

### Process notes
- `Operation::Liquidation` has a `profit_floor: Option<u128>` field — constructing it in a test
  needs the whole field list; `cargo check` names the missing one.
- `atomic_swap` in a fixture needs two *different* declared chains, or lowering fails with
  "source chain 'unknown' is not a known chain prefix". Intents are a much easier fixture for
  lane tests: `intent x { from … to … }` with no `route` lowers to Lock+Release (settlement), a
  same-chain `route { swap … }` to trading, and a cross-chain swap to cross-domain.
- Borrowing a `&str` out of `schedule(&work).iter()` needs the `Vec` bound to a `let` first.

### Next task seed
- **Only PHASE 50 (p50/p95/p99 benchmarks) and PHASE 51 (GPU path + CPU/GPU bit-equality)
  remain.** 20 is out of scope by its own text. Both are assigned to an agent that has not
  delivered; the repo state to start from is master at `ed70c526e`.
- **TICKET-073** is the generator PHASE 37 and PHASE 38 both wait on. **TICKET-065** (the dead
  EVM verifier) is the highest-value non-phase item. **TICKET-078** (lowering's `_ => {}`
  silently dropping unknown items) is a cheap robustness win.

## 2026-09-19 — PHASE 50 (perf targets) and PHASE 51 (GPU), and the phase list is closed

### Facts
- **PHASE 50** `2ff7777dc`: `crates/x3-tools/src/bin/x3bench.rs` measures the ten operations
  the phase names, against the *function that does each*, reporting p50/p95/p99 by nearest
  rank over sorted samples. Artifact: `.ai/reports/x3lang-benchmarks-20260919.md` (a verbatim
  run). No new dependency — the percentiles are computed in the binary so a reader can check
  the method.
  **The finding: parsing dominates by ~2 orders of magnitude** (~150µs p50 vs single-digit µs
  for graph/optimizer/verifier). `receipt verification` is second at ~31µs.
- **PHASE 51** `79856066c`: `vm/src/gpu.rs`. Six candidates classified *in code* with reasons:
  critical = route candidate scoring, signature verification, hash batches, graph scoring;
  not critical = simulation batches, opportunity filtering. `gpu_backend_probe()` calls
  `UnconfiguredBridge.gpu_dispatch` and reports the host's own error
  (`X3_BACKEND_REQUIRED: production GPU dispatch backend is not configured`).
  `run_equality` computes the CPU bytes and returns `Equality::Unproven` carrying that
  message — `Proven` is unrepresentable on one backend, which is the structural guarantee
  against the silent CPU fallback the phase forbids. `x3c gpu` prints it.
- **Every phase row now has an implementation except 20**, whose own text forbids shipping
  before a formal safety proof. Several rows remain PARTIAL in depth, and seven
  (9, 10, 11, 20, 22, 37, 39 — plus 38's legs) are *decided with execution refused*, which is
  the honest end of a phase whose runtime support does not exist.

### Process notes
- The benchmark harness's first run reported **nine of ten**: its scheduling fixture built two
  legs from the same operations and `dag::plan` correctly refused them as a `Cycle`. The legs
  had to be genuinely disjoint (different chains/assets). Because the harness prints
  `NOT MEASURED` with a reason and exits non-zero, the hole was visible instead of silent.
- `OptimizationReport`'s field is `chosen`, not `selected`. `dag::RaceError` implements
  `Debug`, not `Display`. `Operation::Emit`'s `data` is a `HashMap<String,String>`, not a
  `Vec`. `Operation::Liquidation` has a `profit_floor: Option<u128>`.
- The objective constraint grammar is `<field> <= <value>[unit];` inside `constraints { … }`
  (e.g. `hops <= 3; fees <= 20bps;`), not `max_hops 3`.
- `GpuDispatch` is **not** a VM kernel: it is `BridgeAdapter::gpu_dispatch`, a host call. Two
  adapters (`UnconfiguredBridge`, and the production one) already refuse it, which is what
  makes the probe honest rather than a hardcoded string.
- The GPU feature flag is `VMState.allowed_features: BTreeSet<u8>` set by
  `Operation::FeatureAllow` (only `FEATURE_INTENT_FUSION` exists today). A `gpu` feature flag
  was deliberately *not* added: a flag for a path nothing can take is itself a fake.

### Next task seed
- The phase list is closed; what remains is **depth**, and the tickets name it:
  **TICKET-073** (the generator that turns an `arb`/`hyperarb` scope into legs — the single
  biggest remaining piece, and both operations refuse over it), **TICKET-065** (the dead EVM
  verifier: a verified critical defect), **TICKET-071** (the netting residual settler),
  **TICKET-070** (the rebalance graph), **TICKET-068/069** (perp and lending adapters),
  **TICKET-074** (packet realism), **TICKET-075** (the venue guarantee in the artifact),
  **TICKET-078** (lowering's `_ => {}` silently dropping unknown items).
- The `arb::STAGES` list is now the canonical statement of what the arbitrage pipeline is
  missing: **Execution Plan** and **Atomic Settlement**.

## 2026-09-19 — PHASE 37 is executable: an `arb` scope lowers to a plan

### Facts
- `8e53a3e84`: `arb::plan(program, decl) -> ArbPlan` owns the two stages the stage table used
  to report as missing. An `arb` now lowers to
  `AtomicBegin, [AtomicChoice], MultiHopSwap, RouteFallback, Require(profit), Require(slippage), AtomicEnd`
  and `x3c check` / `build` / `explain` / `run` all succeed. `arb::missing_stages()` is empty.
- **The generator plans a route, not a profitable cycle, and that is forced.** The opportunity
  graph holds venue attributes (fee, slippage, liquidity, latency, finality, risk) and **no
  prices** — the language already refuses `maximize profit` for an objective for that reason.
  So the profit floor is a **runtime guard** and the per-hop outputs belong to the host
  (`MultiHopSwap` takes only the input amount, which the declaration states). Nothing is
  invented to fill either gap.
- New `ChoiceCriterion::LowestDeclaredFee` + `CHOICE_CRITERION_LOWEST_DECLARED_FEE = 2`. The
  word *declared* is load-bearing: the sum is over venue `fee_bps`, never over output. A
  source-level `atomic_choice` is refused the criterion (`verify_atomic_choice_decls`) because
  a path body names hops, not venues, so it has no declared fee to sum.
- `Operation::Arb` was **deleted**: the scope's job was to filter and the filter's result is
  the cycle, so a marker operation was a second copy of the answer. `Operation::MultiHopSwap`
  took its place in the `lanes` classification (trading).
- A `hyperarb`'s refusal now says its *own* generator is missing rather than blaming a shared
  stage (TICKET-079).

### Process notes (expensive to rediscover)
- **`open(p,'a')` followed by `open(p,'w').write(s)` truncates the append.** I lost a whole
  appended block that way and had to re-apply it. Read, compose the full text, then write once.
- A line-based test replacement that starts at the `fn` line (because the loop stops *at* the
  preceding `#[test]`) duplicates the attribute and warns `duplicate_macro_attributes`. Either
  include the attribute in the replaced range or drop it from the replacement.
- `graph.edges_from(asset)` hands back edges whose `attributes.domain` is the venue's *domain*,
  not its chain, so a chain-scope filter cannot read the edge. Build the admitted-venue name set
  from the declarations with the same `venue_in_scope` rule `venue_standings` uses — one rule,
  used twice, so the standings and the search cannot disagree.
- `rustfmt` reflows long `assert!` calls across lines, so exact-text anchors for tests break on
  the second run. Anchor on function boundaries instead.
- `Operation::MultiHopSwap` is emitted as opcode `0x91` and disassembles with its path and
  amount; `x3c explain` is the way to prove a plan is really in the artifact.

### Next task seed
- **TICKET-079** is the natural next piece and the last "decided but refuses" arbitrage phase:
  `hyperarb::plan` emitting a `ParallelPlan` over per-leg operation sets via
  `dag::leg_from_operations` + `dag::plan` (both already exist and are used by PHASE 16).
- Then the remaining depth tickets: 065 (dead EVM verifier), 070 (rebalance graph),
  071 (netting settler), 068/069 (perp and lending adapters), 074/075, 078 (lowering catch-all).

## 2026-09-19 — PHASE 22 settles, and two safety checks that contradicted themselves

### Facts
- `fa12e602d`: a netting book binds accounts (`account <party> = <address>;`) and lowers to one
  atomic route per residual transfer — `AtomicBegin, Lock(from=debtor), Release(to=creditor),
  AtomicEnd` — and `check`/`build`/`explain`/`run` all succeed with **no warnings**. Verified
  artifact: LOCK 120 from 0xA1 → RELEASE to 0xB1, LOCK 80 from 0xC1 → RELEASE to 0xB1, matching
  the net positions (alice −120, carol −80, bob +200). `netting::settlement` + `netting::accounts`.
- **Two existing checks had to be corrected, and both corrections make them *more* precise:**
  1. `no_double_claim` counted `Release`s **program-wide** while its own description said "twice
     for the same lock". Its sibling `no_double_refund` had already been fixed for exactly this
     (with a comment about two-legged swaps), and the `release_lock` helper for naming a claim's
     lock already existed. Now counted per route and per lock; the existing test that pins a real
     double claim (two releases of the same asset) still passes.
  2. `verify_refund_path_exists` demanded an explicit `OnFail`/`OnTimeout` Refund for any `Lock`.
     That was **contradictory, not strict**: `no_refund_after_claim` refuses a handler refunding an
     escrow the same route claims, so a same-asset lock-and-release — the only shape a netting
     transfer can take — was unexpressible. It now also accepts the **atomic rollback** for a lock
     whose escrow the same route claims, grounded in `vm/src/executor.rs` (AtomicBegin snapshots;
     a failed route truncates `asset_ops`). `Bridge`/`Swap` still require an explicit handler: a
     rollback restores this VM's state and cannot reach another chain. Four tests pin both ways.

### Process notes (expensive to rediscover)
- **Do not write `\\` at the end of a Rust string line in a heredoc.** I did, five times in
  `netting.rs`; Rust then emits a literal backslash plus a newline plus the next line's
  indentation into the diagnostic. The rendered message in a test failure showed it plainly:
  `...owed \` + newline + 22 spaces. Grep `' \\\\$'` in files you appended to with a heredoc.
- `empty_ir()` lives inside `semantic.rs`'s private `mod tests`, so a second test module in the
  same file cannot use it. Build `X3IR { operations, metadata: ProgramMetadata { … } }` directly.
- The reference shape for a *cross-asset* settlement is `atomic_swap`'s: Lock(eth.USDC),
  Release(sol.SOL), two OnTimeout refunds. It dodges `no_refund_after_claim` only because its
  refunded asset (sol.SOL) was never locked. A same-asset settlement has no such dodge — hence
  the check fix rather than a shape change.
- Current state: master `fa12e602d`, 1048 passed / 0 failed, clippy and fmt clean, sweep 17/23,
  pytest 16.

### Next task seed
- **TICKET-079** (hyperarb's generator) is the last "decides but refuses" arbitrage phase, and the
  reading is now decided: the legs are **candidate routes** (the phase's own pipeline says
  "Candidate Routes → Filter"), so the artifact is `AtomicChoice` over the legs plus the chosen
  one's operations. `choose highest_net_output` must be refused at the AST layer (no prices in
  the graph), `fewest_hops`/`lowest_declared_fee` are computable, ties go to the earliest leg.
- **TICKET-080** is the root fix that would let a whole book settle as one unit.
- Remaining depth: **TICKET-065** (the dead EVM verifier — verified critical), **TICKET-070**
  (the rebalance graph, gated on prices/holdings), **TICKET-068/069** (perp and lending
  adapters), **TICKET-074/075**, **TICKET-078** (lowering's catch-all).

## 2026-09-19 (later) — PHASE 38 plans and runs, and two cross-layer gaps it exposed

### Facts
- `89a15ccc5`: `hyperarb::plan` selects one candidate leg and emits
  `AtomicBegin, AtomicChoice, MultiHopSwap, RouteFallback, Require, AtomicEnd`. Verified artifact:
  `ATOMIC_CHOICE 2:2:1`, `MULTI_HOP_SWAP path ["ethereum.USDC","solana.USDC"] amount 25000000`,
  `ROUTE_FALLBACK usdc_to_sol`; `x3c run: ok`.
- **The reading is decided: the legs are candidate routes.** The phase's own pipeline says
  "Candidate Routes → Filter → …", and `choose` only means anything among alternatives. So the
  artifact is `AtomicChoice` + the chosen leg's operations. The diagram's concurrency is the
  *evaluation's*.
- **A compiler/VM disagreement the new criterion exposed:** `vm/src/verifier.rs` and
  `vm/src/executor.rs` each whitelisted only criteria 0 and 1, so the first artifact carrying
  `CHOICE_CRITERION_LOWEST_DECLARED_FEE` was rejected with `X3_VERIFY_FAILED: InvalidOperand` and
  would not run. `spec/opcodes.rs` is the single source for the vocabulary; both layers now read
  it. **Lesson: adding an opcode-level constant is a two-layer change — grep the VM for every
  whitelist that names the old members of the set.**
- Three consistency rules the plan needed, each a refusal: a leg must move the *capital's*
  asset; a leg's resolved venues must agree on an asset pair; and the program must carry a
  `require slippage <= <n>` because `verify_slippage_explicit` refuses any artifact with a swap
  leg and no bound (a hyperarb states only a profit floor).
- `settle_across_domains` moved from `analyse` (union of all legs) to `plan` (the selected
  route's path) — under the candidate-routes reading the union was the wrong subject.

### Process notes (expensive to rediscover)
- **Deleting code by "find the arm, delete to the first `},`" breaks neighbours.** I did that
  twice (emitter, verify) and both times removed the *next* arm's opener or left a duplicate
  delimiter. Read the surrounding arms before deleting, and re-read the region after.
- `split` is not double-ended, so a reversal built from `split(..).rev().collect()` does not
  compile; build the reversed fixture explicitly.
- Two `const` items with the same name in an appended test block shadow each other with E0428.
  Check names before appending.
- `empty_ir()` is private to `semantic.rs`'s `mod tests`; a second test module needs its own
  `X3IR { operations, metadata }`.
- Current state: master `89a15ccc5`, 1056 passed / 0 failed, clippy and fmt clean, sweep 17/23,
  pytest 16.

### Next task seed
- **TICKET-065** is now the highest-value open item: a verified critical defect (a module that
  advertises Merkle Patricia Trie verification with no trust anchor; its `compute_receipt_trie_root`
  ignores its own `_index`/`_total_receipts`, and `proof/mod.rs` claims a wiring that exists
  nowhere). `crates/x3-verification-router/src/evm_receipt.rs` has the real verifier.
- Then **TICKET-068/069** (perp and lending-protocol adapters) — they are what keep PHASE 9 and 10
  decided-but-refusing — and **TICKET-070** (the rebalance graph, gated on holdings/prices),
  **TICKET-080** (a `Release` that names its lock, which would let a book settle as one unit),
  **TICKET-078** (lowering's catch-all), **TICKET-074/075**.

## 2026-09-19 — correcting a claim: economic guards are records, not tests

### Fact (and a correction to two earlier entries)
- The emitter says it outright: `Operation::Require` is "always STATIC today … a guard would have
  to find its quantity in `r0`, and no instruction puts it there". Only the **nonce** guard is
  emitted as a comparison, because `NONCE_UNUSED` leaves its quantity in `r0` just before it.
- So the `require profit >= 20bps` floors that `arb::plan` (`8e53a3e84`) and `hyperarb::plan`
  (`89a15ccc5`) put in their plans are **recorded in the artifact and re-checkable**, and
  **nothing evaluates them at run time**. My reports on those two turns called them runtime
  guards; that was wrong, and the ledger rows are corrected.
- `TICKET-027` owns this and now carries the design (below).

### Design built, verified, then reverted — and why
Built and measured end to end: a reply-tag convention (`CAPABILITY_REPLY_MEASURED_TAG` with a
unit and a 16-byte value), two measured comparison modes that fail closed with
`X3_GUARD_UNMEASURED`, `DryRunBridge::with_measurement(..)`, and
`x3c run --measured-profit-bps/--measured-slippage-bps` (refusing a half-stated measurement).
Reverted because with enforcement on **ten of the twenty-three corpus examples fail at run
time** — `simple_swap`, `atomic_choice`, `flagship_b52`, `intent_fusion`, `mainnet_safe_swap`,
`multi_leg_route`, `objective_routing`, `parallel_dag`, `route_fallback`, `strategy_module` —
every one of them at a guard written *before* the trade it names. Those guards are
**pre-conditions**: the compiler checks them against the venues and the policy, and the trade
below is what the program then does. Reading them as post-conditions changes the meaning of ten
working programs in one commit, which is a language decision rather than a bug fix.

### Process notes
- **A whitelist that names the members of a set has to be updated with the set.** Two places
  this session: the VM's verifier *and* executor for `CHOICE_CRITERION_*` (`89a15ccc5`), and the
  VM's verifier for `REQUIRE_COMPARE_*`. Both rejected a new member as `InvalidOperand` before the
  honest refusal could happen. When adding a constant to `spec/opcodes.rs`, grep the VM for every
  `matches!` that names its siblings.
- **A struct-literal change is a wide change**: turning `DryRunBridge` from a unit struct into
  one with a field needed a `Default`, and every bare `DryRunBridge` use had to be updated.
- **Never push before clippy.** I pushed `3bac66b49` with clippy failing (twelve unreachable
  patterns and three non-exhaustive variants), then had to push `b0f409b0e` to fix it. The
  explicit list was written from a file scan instead of from the compiler; three `cargo check`
  iterations would have been faster than the mistake.
- **`git checkout -- <files>` cleanly reverts an uncommitted experiment**, which is the right way
  to abandon a design without leaving debris. Verified by re-running the suite (1056/0 before and
  after).

### Next task seed
- master is `b0f409b0e`, 1056 passed / 0 failed, clippy and fmt clean, sweep 17/23, pytest 16.
- Open and aligned: **TICKET-065** (the dead EVM verifier — verified critical), **TICKET-068/069**
  (perp and lending adapters, which keep PHASE 9 and 10 from executing), **TICKET-070** (the
  rebalance graph), **TICKET-080** (a `Release` that names its lock), **TICKET-075**, and
  **TICKET-027**'s enforcement path (needs the pre-/post-condition decision first).

## 2026-09-19 — a plan's economic floor is now enforced (`31522c24e`)

### Facts
- `Operation::Require` gained `measured: bool` (`#[serde(default)]`). A guard a **program**
  writes is `measured: false` — a constraint the compiler checks against declarations, emitted
  STATIC as before. A guard `arb::plan`/`hyperarb::plan` emits after the trade is `measured:
  true` and is **enforced** against a measurement the host reports.
- The four outcomes, all verified through the binary: unmeasured → `X3_GUARD_UNMEASURED`;
  clearing → `x3c run: ok`; below the floor → `X3_PROFIT_BELOW_FLOOR: realised 5bps, requires at
  least 20bps`; above the ceiling → `X3_SLIPPAGE_ABOVE_CEILING: realised 90bps, allows at most
  8bps`.
- Wire form: a reply is a **sequence** of 18-byte records — `CAPABILITY_REPLY_MEASURED_TAG`,
  unit (`MEASURED_UNIT_PROFIT_BPS`/`_SLIPPAGE_BPS`), 16-byte little-endian bps — because one
  trade answers both a profit floor and a slippage ceiling. `read_measured_replies` takes the
  sequence; `read_measured_reply` refuses a reply with more than one record.
- `DryRunBridge::with_measurement(..)` + `Default`; `VM::report_measurement(profit, slippage)`
  replaces the adapter (the measurement belongs to the host, not to VM state); `x3c run
  --measured-profit-bps/--measured-slippage-bps`, refusing a half-stated measurement.

### Process notes (expensive to rediscover)
- **`ON_FAIL` takes its handler target from `r0`** — the emitter writes operand 0, the executor
  reads `registers[ra]`, and so a guard failure dispatched through it jumps to whatever pc `r0`
  held: my first version aborted with `InvalidOpcode(115)` instead of naming the shortfall. A
  measured floor now *refuses* instead of dispatching. TICKET-058's "explicit branch target in
  the record" is what the handler mechanism needs for its own sake.
- **Two-layer whitelists again**: the VM's verifier named only `REQUIRE_COMPARE_STATIC` and
  `_GE`, so the first artifact carrying a measured mode failed as `InvalidOperand`. Same failure
  as `CHOICE_CRITERION_*` in `89a15ccc5`. When adding a constant to `spec/opcodes.rs`, grep the
  VM for every `matches!` naming its siblings.
- Enforcing **every** economic guard fails 10 of 23 corpus examples at guards written before the
  trade they name — so the distinction has to be in the IR (`measured`) rather than inferred by
  the executor. I built the blanket version first, measured that, and reverted it.
- `u128::from(threshold)` where `threshold` is already `u128` is a clippy `-D warnings` error.
- Current state: master `31522c24e`, 1063 passed / 0 failed, clippy and fmt clean, sweep 17/23,
  pytest 16.

### Next task seed
- **TICKET-058** now has a second, independent reason to exist: the `ON_FAIL` handler target.
- Still open and aligned: **TICKET-065** (dead EVM verifier), **TICKET-068/069** (perp and
  lending adapters — PHASE 9/10 execution), **TICKET-070** (rebalance graph), **TICKET-080**
  (`Release` naming its lock), **TICKET-075**, and the language decision in TICKET-027 about
  whether a *program's* economic guard should also be a post-condition.

## 2026-09-19 — PHASE 9 executes: a hedge lowers to venue orders (`c2eac6911`)

### Facts
- New `spec::opcodes::VENUE_ORDER = 0x9D` + `CapabilityPayload::VenueOrder { action, subject,
  asset, quantity }` + `Operation::VenueOrder` + `BridgeAdapter::venue_order`. A hedge lowers to
  `AtomicBegin, VenueOrder(spot_buy), VenueOrder(perp_short), Require(delta <= bound), AtomicEnd`
  and `check`/`build`/`explain`/`run` all succeed (108 bytes for the balanced fixture).
- `hedge::orders(decl)` resolves each leg's size the way `hedge::exposure` resolves the net, so a
  leg written `equivalent` takes the other side's number and the plan cannot disagree with the
  delta that was checked.
- Why not `Operation::Call`: its `CALL_HOST` dispatch goes to `BridgeAdapter::svm_call`, so a perp
  short on Ethereum would arrive at a host as an SVM call. A venue order says what it is.
- Still open (TICKET-068): the *runtime* does not report the delta back, so a venue that filled
  something else is not caught. Needs a measured delta = a fourth comparison mode, and
  `REQUIRE_COMPARE_MASK` is 2 bits with all four values taken — bits 5-7 of the flags byte are
  free and are the natural place for a unit code.

### Process notes (expensive to rediscover)
- **The capability opcode set is named in three places**, and adding a member needs all three:
  `is_payload_opcode` (`spec/opcodes.rs`), the opcode name table, and the executor's dispatch
  **range** `GPU_DISPATCH..=SUB_EXEC`. Missing the third gave `InvalidOpcode(157)` at run time.
  The repo's `every_payload_opcode_is_recognised_by_the_executor` test exists to catch exactly
  that drift and it did. Same class as the `CHOICE_CRITERION_*` and `REQUIRE_COMPARE_*` pairs.
- **My `drop_arm` helper deleted 279 lines of `verify.rs`** — it searched forward for the first
  line equal to `        }` and the arm it was removing did not end there, so it ate every
  remaining arm. Recovery was `git checkout -- <file>` and a precise line-range edit. Do not use
  a search-based helper to delete code; compute the exact range and assert what is at both ends.
- A `format!` with a literal `\u{1f}` inside a python-written Rust string needs the braces
  *unescaped* in the Rust source (`\u{1f}`), not doubled — `\u{{1f}}` is a Rust error.
- `ir::RequireKind::Custom` takes a `String`; `ast::RequireKind::Custom` takes a `Symbol`. Easy
  to mix up when a lowering arm builds one to feed the other.
- Current state: master `c2eac6911`, 1065 passed / 0 failed, clippy and fmt clean, sweep 17/23,
  pytest 16.

### Next task seed
- **PHASE 10 (liquidation) is now one step closer**: `VenueOrder` already carries a `subject` for a
  position reference and the action vocabulary can take `liquidate`/`receive_collateral`, so the
  same shape applies. `liquidation::ledger` already decides the figures.
- Then TICKET-068's remainder (measured delta), TICKET-058 (`ON_FAIL`'s handler target from `r0`),
  TICKET-065 (dead EVM verifier), TICKET-070, TICKET-080, TICKET-075.

## 2026-09-19 — PHASE 10 executes: a liquidation lowers to its calls and its conversion

### Facts
- `9a5438ef5`: `Item::AtomicLiquidation` lowers to `AtomicBegin, VenueOrder(liquidate,
  subject=position), VenueOrder(receive_collateral, subject=position), Swap(declared amounts,
  dex: None), Require(profit floor), AtomicEnd`. `check`/`build`/`explain`/`run` all succeed and
  the plan is in the artifact. `Operation::Liquidation` was deleted, like the hedge's and the
  arb's markers before it.
- **The conversion is a `Swap`, not a route** — the declaration states the input and the
  `min_output`, so no price is needed. That is the difference from arbitrage, where the amounts
  had to be the host's. `dex: None` because the program names no market.
- **Its profit floor is a *constraint*, not a measured guard**, and the reason is worth keeping:
  a plan's measured floors follow a *host call* whose reply carries a measurement, and a `Swap`
  is an asset-op *record* — `apply_asset_payload` sets registers and never reaches the host — so
  no reply could carry one. The floor derives from the declared `min_output` against the
  repayment, which `liquidation::verify` already checks. Enforcing the realised net needs the
  swap path to report an output (TICKET-069).
- `LiquidationLedger` gained `collateral_parts()`/`debt_parts()` so the conversion's assets can
  be named for the `Swap` without a second `chain.ASSET` parse.
- The lane model now reads the `VenueOrder` **action**: `liquidate`/`receive_collateral` →
  Liquidation lane, every other order → Trading. The lane is still a property of what the
  program does, and the action is in the artifact.

### Process notes
- Clippy caught a duplicate `Operation::VenueOrder` arm in `lanes.rs` (the action-based arm
  matched all orders, making the later one unreachable) — *before* I pushed, because I ran the
  gates first this time. That is the lesson from the previous turn applied.
- `apply_asset_payload` is the reason a `Swap` cannot produce a measurement: worth remembering
  when a plan's floor looks measurable and is not.
- Current state: master `9a5438ef5`, 1065 passed / 0 failed, clippy and fmt clean, sweep 17/23,
  pytest 16.

### Next task seed
- **Only PHASE 11 (rebalance) still decides without executing**, and it needs current holdings
  and prices, which the graph does not hold (TICKET-070) — so it may be the honest end for that
  phase rather than a missing piece of work.
- Then: TICKET-068/069 remainders (a measured delta/net, needing a wider comparison-mode field),
  TICKET-058 (`ON_FAIL`'s handler target from `r0`), TICKET-065 (dead EVM verifier),
  TICKET-080, TICKET-075, TICKET-074.

## 2026-09-19 — PHASE 11 executes: a rebalance carries its target (`9955201ec`)

### Facts
- `REBALANCE_TARGET = 0x9E` + `CapabilityPayload::RebalanceTarget { portfolio, weights:
  Vec<(String,u32)>, criterion }` + `BridgeAdapter::rebalance_target`. A `rebalance` lowers to
  that instruction, `check`/`build`/`explain`/`run` all succeed, and the opcode set was updated
  in all **four** places at once (`is_payload_opcode`, the name table, the framing-cost table,
  the executor's dispatch range) — the lesson from `c2eac6911` applied rather than rediscovered.
- **The trades are not generated, and that is the honest end rather than missing work**: every
  trade depends on the account's *current* portfolio, and a compiler has no state. The phase's
  own sentence says the graph generation is "eventually". TICKET-070 records the two ways to
  close it (a program states its holdings; or a compile-time interface for them), both language
  decisions.
- With this, **no phase of the 56 refuses any more except 20**, which its own text keeps
  out of scope until a formal safety proof exists.

### Process notes
- Adding a capability opcode touches four tables in `spec/opcodes.rs` **plus** the executor's
  dispatch *range* in `vm/src/executor.rs` (five places). The repo's
  `every_payload_opcode_is_recognised_by_the_executor` test catches the range; the name table
  is caught by `opcode_name` assertions in the same test module.
- Three `useless conversion` clippy errors this session came from `u32::from(*percent)` and
  `u128::from(threshold)` where the value was already that type. Cheap to fix and worth running
  clippy before pushing, which is now the habit after the `3bac66b49` mistake.
- Current state: master `9955201ec`, 1066 passed / 0 failed, clippy and fmt clean, sweep 17/23,
  pytest 16.

## 2026-09-19 — PHASE 45: every artifact binds its versions, and a mismatch is refused (`1d7d2f6a7`)

### Facts
- `META_VERSIONS = 0x12`, a fixed 11-byte header record: language, compiler, IR, VM, policy.
  `spec::opcodes::version_binding(bytes)` reads it; `vm/src/verifier.rs::verify_version_binding`
  refuses a mismatch on language/IR/VM with `VerifyError::VersionMismatch { field, bound,
  supported }` (its own variant, with a `Display` that renders `X3_VERSION_MISMATCH: …`), and
  refuses a *compiler stream with no binding at all*. `x3c explain` prints
  `meta.versions = language 1 compiler 1 IR 1 VM 1 policy 1`.
- **One walker now**: `spec::opcodes::metadata_record(bytes, pc) -> Option<(len, label, rendered)>`
  replaces the five places that parsed the metadata set (the verifier's skip, the executor's,
  the disassembler's, the trading decoder's, and the version reader). Adding the record broke six
  tests because three of the five read its bytes as instructions — the same "a set named in N
  places drifts in N-1 of them" defect the opcode set had twice. The refactor also deleted two
  now-dead copies of the old walkers.

### Process notes (expensive to rediscover)
- **`git rebase` and `git cherry-pick` need `user.email`, and this clone has none.** Both failed
  with "unable to auto-detect email address", and a *failed* rebase leaves HEAD at the new base
  with my changes in the working tree — which then looked like the commit had been "dropped".
  Set `git config user.email` in every scratch clone before the first rebase. `git reflog` found
  the lost commit object, and the changes were recoverable from the working tree each time.
- **A stale `.git/rebase-merge` directory silently no-ops a rebase** ("I am stopping in case you
  still have something valuable there"), and `git rebase --abort` then rolls back to the state
  *before* that interrupted rebase — which can discard a commit made in between. Check
  `git log --oneline -1 origin/master` equals HEAD after every rebase rather than trusting the
  exit status.
- Two other agents are pushing to `master` continuously (PHASE 11's push raced one, PHASE 45's
  raced two). Always `git fetch && git rebase origin/master` immediately before the push, and
  re-verify the tree after the rebase.
- Current state: master `1d7d2f6a7`, 1068 passed / 0 failed, clippy and fmt clean, sweep 17/23,
  pytest 16. All four of this session's phase landings (9, 10, 11, 45) are confirmed present on
  master by grepping the pushed content.

### Next task seed
- **PHASE 43** is the one phase whose requirement is verified-unmet: the named fixed-point types
  (`Decimal<18>`, `Bps`, `Rate`, `Price`, `Ratio`) and the item-by-item audit list. That is
  bounded and additive (`crates/x3-common` is the natural home).
- Then the depth rows: **42** (an explicit determinism audit of the consensus paths), **48/49**
  (the adversarial matrix and the property list item by item), **54** (`simulate --state`),
  **39**'s artifact provenance (TICKET-075).
- And the open tickets: **TICKET-065** (the dead EVM verifier — critical), **TICKET-058**
  (`ON_FAIL`'s handler target from `r0`), **TICKET-027** (a program's own economic guard),
  **TICKET-068/069** (the measured halves), **TICKET-080**, **TICKET-074**.

## 2026-09-19 — PHASE 43: the fixed-point vocabulary (`1a9274900`)

### Facts
- `crates/x3-common/src/fixed.rs` is the home: `Bps`, `Decimal<const SCALE: u8>`,
  `Ratio`, `Rate`, `Price`, `RoundingMode`, `pow10`, `MAX_SCALE = 18`. Every
  operation is `checked_*` -> `None`; the mantissa is **unsigned**, which is what
  turns "underflow" from a check into a property.
- `RoundingMode` **moved out of `crates/x3-ast/src/trading.rs` and into `fixed.rs`**;
  the AST re-exports it. One type, so a direction written in a declaration and a
  direction obeyed by a conversion cannot drift. The AST test asserts the identity
  at *compile time* (pass the AST value to a function taking the arithmetic one),
  which is stronger than a `TypeId` comparison.
- The audit list is answered by test, not prose: 12 tests in
  `crates/x3-common/tests/fixed_math.rs`, one per phase item, plus the identity test.
  **Writing them found two real defects in the module itself** — see below.
- `hedge.rs::delta_bps` now computes through `Ratio::of`/`to_bps`. The truncation
  point is unchanged (`floor(floor(x)/n) == floor(x/n)` for positive ints), so the
  existing 1_000 bps assertion still holds. An unrepresentable notional reports
  `u128::MAX`, which is fail-closed because every declared bound is below it.
- The bare `10_000` basis-point ceiling is now `Bps::WHOLE` / `is_within_whole()`
  in `arb.rs`, `semantic.rs`, `trading_semantic.rs`, `trading_verify.rs`,
  `verify.rs`, `objective.rs`, `strategy.rs`. Diagnostic strings were kept
  **byte-identical** on purpose — `test_arb.rs` matches on `"the whole trade"`.
- Verified first half of the phase rather than assuming it:
  `rg -n '\bf(32|64)\b' compiler/src vm/src crates/*/src | grep -v '^[^:]*:[0-9]*: *//'`
  gives exactly 7 lines — the Solana `warmup_cooldown_rate` wire field plus six
  renderings of the language's own `f32`/`f64` literal *suffixes*. All remaining
  `f64` mentions are comments recording removed paths.

### Defects the new tests found (the lesson: write the audit tests *first*)
- `Ratio::of` scaled **both** sides to 18 digits and then divided — i.e. it
  multiplied two 18-digit mantissas — so `Ratio::of(1_000, 999)` returned `None`.
  A ratio type that refuses every amount the language writes is decoration. Fix is
  one scaling: `output × 10^18 / input`. The same double scaling was in
  `Ratio::of_amount` and `Rate::over`; both are now the integer form.
- `Price::convert` had the same product and so refused any amount above ~227 units.
  Fix cancels the powers of ten the two sides share (the ratio's trailing zeros,
  then the amount's) before forming the product — exact, so still one rounding.
  `pow10_ext` was added because a two-asset conversion composes scales past
  `MAX_SCALE` (up to `10^36`, which still fits `u128`).

### Process notes (expensive to rediscover)
- **A stale `.git/rebase-merge` no-ops `git rebase` and is not evidence of pending
  work.** Hit again this round ("It seems that there is already a rebase-merge
  directory"). Before clearing it, prove nothing is lost: the dir's `orig-head` is
  the *pre-rebase* hash, so `git diff --stat <orig-head> <rewritten-hash>` should
  show only what the new base changed. Here `244027cdba` vs `1d7d2f6a2` differed
  only in the two files `a10db8885` touched. Then `rm -fr .git/rebase-merge` and
  rebase.
- Rebase target moves constantly (master advanced to `4ac1d4994` mid-round). The
  safe push is: `git fetch`, assert `git rev-parse origin/master == git rev-parse
  HEAD^`, then `git push origin HEAD:master`. Assert it rather than eyeballing.
- `HEAD` was **detached** in this clone from a previous session's interrupted
  rebase. `git status --short` does not tell you this; `git branch --show-current`
  returning empty does. `git push origin HEAD:master` works regardless.
- `cargo test --workspace 2>&1 | grep -E '^test result' | awk -F'[ ;]' '{p+=$4; f+=$6} END {print p" / "f}'`
  is the one-liner that turns the battery into a single figure. 1081 / 0 this round.
- House formatting is **not** default rustfmt `max_width`; it is wider than 100. Use
  `cargo fmt --all` and then check `git diff --stat` to confirm it only touched your
  files, rather than hand-wrapping and fighting `fmt --check`.

### Next task seed
- **TICKET-081** (new): `compiler/src/trading_semantic.rs::decimal_to_base_units`
  re-implements the rounding rule the new `Decimal` owns, so the rule has two homes
  and its tests are in only one. A differential test over a boundary table is the
  cheap first half; the ticket says what the unification needs.
- Then the depth rows: **42** (explicit determinism audit), **48/49** (adversarial
  matrix and property list item by item), **54** (`simulate --state`), **39**'s
  artifact provenance (TICKET-075).
- And the open tickets: **TICKET-065** (the dead EVM verifier — critical, still
  unaddressed, and the only one of the four EVM receipt paths with no callers),
  **TICKET-058**, **TICKET-027**, **TICKET-068/069**, **TICKET-074**, **TICKET-080**.

## 2026-09-19 — TICKET-065: the EVM "verifier" now verifies (`bebbe55cf`)

### Facts
- `crates/x3-crosschain-intent/src/proof/evm.rs` advertised Merkle Patricia receipt
  verification and had no trust anchor: `compute_receipt_trie_root` took
  `_index`/`_total_receipts` and used neither, returning `keccak256(rlp(receipt))` —
  the hash of the caller's own bytes. `block_hash` was copied and never read;
  `tx_hash` was `[0u8;32]`; `confirmations` a hardcoded `1`; and
  `TrieRootMismatch` / `ReceiptHashMismatch` / `InsufficientConfirmations` /
  `InvalidReceiptIndex` were **declared and never constructed**. The tests pinned it
  (`verify_valid_receipt` succeeded against `block_hash = [0xab; 32]`).
- Nothing outside the crate calls it. `grep -rn 'EvmReceiptProof|verify_evm_receipt_proof'`
  outside `crates/x3-crosschain-intent` returns only
  `x3_verification_router::VerificationStrategy::EvmReceiptProof`, an unrelated enum
  variant. So the false claims had no victim yet — the reason to fix it, not a reason
  not to.
- The canonical verifier is
  `x3_verification_router::evm_receipt::verify_merkle_patricia_proof(root, key, value, proof)`
  where `proof` is an **RLP list of node byte strings**, and the key must come from
  `evm_receipt::receipt_trie_key(index)` (`rlp(index)`, index 0 → `0x80`). Use the
  router's key helper rather than re-deriving it: TICKET-064 was exactly that key
  being wrong, and a second crate can re-derive the same bug.
- Fix is delegation plus honesty: the fields a trie walk does not establish are
  `Option`s, `with_header_attestation` records a chain view as an *attestation*, and
  `require_confirmations` refuses with `NoHeaderAttestation` rather than reading a
  default of 1 as one confirmation.
- Test construction that actually proves the delegation (copy it):
  `key = rlp(index)`; `node = rlp([compact_leaf_path(nibbles(key)), receipt_rlp])`;
  `root = keccak256(node)`; `proof = rlp([rlp(node)])`. Build it from the **standard**
  encoding, never from anything the verifier exposes. Green positive test = the
  delegation works end to end; four new negative tests (tampered node, wrong index,
  a root holding a *different* receipt, empty proof list) could not be expressed at
  all before.
- `keccak256("")` = `c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470`.
  `sha3::Keccak256` is Ethereum's keccak256; `sha3::Sha3_256` is FIPS-202 and is a
  different hash. A known-answer test on the empty string is the cheap way to pin
  which one a module is using — the old test here asserted only "not all zeros"
  against a `Verified:` comment whose value was the digest of nothing.

### Process notes (expensive to rediscover)
- **A `Verified:` comment is not evidence.** This file carried a fabricated digest
  and a fabricated "does NOT trust the relayer" claim for its whole life because the
  test next to it asserted something weaker than the comment. When a doc states a
  property, the test has to be able to *fail* when that property is false.
- A dead-code warning is a finding. `rlp_encode_list` carried `#[allow(dead_code)]`
  and `rlp_encode_bytes` started warning the moment the fabricated root was deleted —
  which is how the file announced that its RLP encoders were test-only.
- The root workspace builds a single crate offline in ~45s (`cargo check -p <crate>
  --offline`) but `pallet-x3-settlement-engine` takes ~2m16s from cold. Both are
  feasible; the x3-lang workspace tests are the slow ones (~90s).
- `cargo check -p x3-crosschain-intent --no-default-features` fails with **244
  errors** (pre-existing, TICKET-082). Moving `hex`'s `alloc` feature to an
  unconditional `features = ["alloc"]` takes it to 231 — measure before assuming a
  feature-gating fix is one line.
- Rebase-and-push that worked twice this session: `git fetch`, assert
  `git rev-parse origin/master == git rev-parse HEAD^`, then
  `git push origin HEAD:master`. Master moved under both commits; the assert caught
  it and the rebase was clean both times. `HEAD` is **detached** in this clone
  (`git branch --show-current` is empty) — that is fine for `HEAD:master`.

### Next task seed
- **TICKET-082** (new): `x3-crosschain-intent` has no working `no_std` path; decide
  whether to fix the features or drop the claim.
- **TICKET-063 step 1**: `pallets/x3-settlement-engine` reads `merkle_proof[0..2]` as
  roots and never walks a path. The verifier it should adopt
  (`x3-verification-router`'s) is now already a dependency of a crate the pallet
  depends on, which shortens that step.
- x3-lang depth rows still open: **42** (explicit determinism audit), **48/49**
  (adversarial matrix, property list item by item), **54** (`simulate --state`),
  **39**'s artifact provenance (TICKET-075), **TICKET-081** (the second home of the
  decimal-literal rounding rule), **TICKET-058**, **TICKET-068/069**, **TICKET-074**,
  **TICKET-080**.

## 2026-09-19 — PHASE 54: simulation against a state snapshot (`7c78f0633`)

### Facts
- `x3c simulate <artifact> --state <snapshot.json> [--explain]`. The state and the
  arithmetic live in **`x3-lang/crates/x3-tools/src/simulation.rs`**, included by
  `bin/x3c.rs` with `#[path = "../simulation.rs"] mod simulation;` — the same wiring
  `provenance.rs` uses. There is no `lib.rs` in `x3-tools`; a shared module is a
  `#[path]` include, **not** a `pub mod` in a crate root.
- The snapshot states the *observations* (route chains + venues, `capital`, `gross`,
  `fees` in one asset, optional `slippage_bps`); the report **derives** net
  (`gross − capital − fees`, checked), net in bps (`Ratio::of(capital, net).to_bps(Down)`
  — PHASE 43's vocabulary), and the minimum required (`Bps::of_floor` of the
  **artifact's** floor applied to the snapshot's capital). The floor is read from the
  artifact's instructions; a caller who could state the requirement could make any run
  pass.
- `--state` and `--measured-profit-bps`/`--measured-slippage-bps` are the same fact
  stated two ways; stating both is refused. `--explain` requires `--state`.
- Serde's `deny_unknown_fields` on every snapshot struct: a misspelled field is a state
  the host meant to state and this build cannot see.
- Honest negatives, both of which a cheaper version would have got wrong: an artifact
  with no profit floor reports **`NO FLOOR STATED`**, not `PASS`; an artifact with no
  approved venue list reports the venues were **NOT** checked.

### The defect class, hit a third time
- **`REQUIRE`'s flags byte carries the comparison mode in bits 0-1 and the guard's own
  operator in bits 2-4.** `spec/opcodes.rs` had `require_flags` (writer) and
  `require_guard_operator` (reader) but **no reader for the comparison mode**, so
  `vm/src/executor.rs` and `compiler/src/emitter.rs` each wrote `flags & REQUIRE_COMPARE_MASK`
  inline, and the new command compared the raw byte — so it found **no** measured guard
  in an artifact that carried two (`Minimum required: none` for an artifact with a 20bps
  floor and an 8bps ceiling). The flags byte for measured-profit + `>=` is `2 | (4<<2) = 18`,
  so `flags == 2` never matched.
- Fixed by adding the missing reader `require_comparison(flags)` and using it in all
  three places, plus a round-trip test over every mode × operator pair.
- **Rule to carry forward**: when a bitfield has a writer and a reader for one field but
  not the other, the missing reader is a defect waiting for its third caller. Grep for
  `& SOME_MASK` next to any `spec/opcodes.rs` constant — an inline mask is the smell.

### Process notes (expensive to rediscover)
- **The dead-code warning is a design reviewer.** `SimulationError::ZeroCapital` was
  never constructed because `Ratio::of(0, net)` returns the same `None` it uses for an
  out-of-range value, so a snapshot with `"amount": 0` was reported as "too large to
  represent". The compiler said `variant is never constructed`; the fix was to check
  zero capital by name *before* the ratio.
- **A hand-rolled digit grouper**: inserting a comma where `(index - len % 3) % 3 == 0`
  underflows for a length below the remainder. Count from the left instead:
  `index != 0 && (len - index) % 3 == 0`.
- The `arb` fixture that reaches the measured guards is the one in
  `crates/x3-tools/tests/cli.rs::cli_plans_an_arb_scope_builds_it_and_runs_it` — an
  `intent` with `require profit`/`require slippage` **plus** two `venue` decls **plus**
  an `arb` scope. `examples/arb*.x3` cannot be used: they are in a dead dialect
  (TICKET-083).
- `cargo clippy -p <crate> --all-targets -- -D warnings` is much faster than the
  workspace-wide run and catches test-only lints (`cloned_ref_to_slice_refs` this
  round). Run it on the crate while iterating, the workspace before pushing.

### Next task seed
- **TICKET-083**: 6 of 23 `examples/*.x3` do not check (five in a dead `contract { fn }`
  dialect, one using the guard name `proof` where the language says `proof_complete`).
  The sweep's `17/23` has been read as a baseline for several rounds; there is no gate,
  so nothing notices. A `scripts/` gate over `examples/*.x3` is the cheap half.
- **TICKET-084**: `x3c check` prints `lowering failed: Parser error: …` for a syntax
  error — the wrong stage, which sends a reader to the wrong pass.
- Then the depth rows still open: **42** (an explicit determinism audit — PHASE 54 added
  a determinism test for the report, but the audit is not written), **48/49** (adversarial
  matrix and property list item by item), **39**'s artifact provenance (TICKET-075),
  **TICKET-081** (one home for the decimal-literal rounding rule), **TICKET-082**
  (`x3-crosschain-intent` has no working `no_std` path), **TICKET-063** step 1,
  **TICKET-058**, **TICKET-068/069**, **TICKET-074**, **TICKET-080**.

## 2026-09-19 — PHASE 42: the prohibitions as gates (`e56d1dbb7`)

### Facts
- `compiler/tests/test_determinism_audit.rs` gates three things over `compiler/src`,
  `vm/src`, `crates/{x3-ast,x3-common,x3-lexer}/src`:
  1. **no clock, no random source, no spawned thread — absolutely, no allowlist.**
     Nine words: `SystemTime`, `Instant::now`, `rand::`, `thread_rng`, `from_entropy`,
     `OsRng`, `rand_core`, `std::thread::spawn`, `rayon::`. The tree has **zero** of
     them, which is why an allowlist would be worse than nothing. The test also asserts
     it read >20 source files, so a wrong path cannot pass vacuously.
  2. **the same source compiles to byte-identical artifacts eight times** — the
     property itself, not an inference from a scan (a scan cannot see a clock reached
     through a dependency).
  3. the word-boundary helper against its own false positive.
- **`contains("rand::")` matches `Operand::Reg(r)`.** A `contains`-based audit
  "found" two violations in a comment about register operands and none of the real
  thing. Anything that scans source for identifiers needs a boundary check, and the
  boundary check needs a test with the exact string that fooled the naive version.
- **The register allocator is a record, not a pass.** `regalloc::patch_operation` is an
  *empty function*; `rewrite_operations` carries
  `#[expect(dead_code, reason = "v0.2 register allocator placeholder")]`;
  `compile_program_with_regalloc` is called **only from its own tests** (`x3c build`
  uses `compile_program`); and nothing reads `register_assignments`/`spill_slots` —
  `vm/src` never mentions them. So the artifact is byte-identical to the plain path,
  which is now **measured** by
  `the_regalloc_entry_point_emits_the_same_bytes_as_the_plain_one`. That test is a
  **ratchet**: it fails the moment the rewrite is wired, which is when the doc has to
  change too (TICKET-085).
- PHASE 42's row is **PARTIAL**, deliberately: the unordered-map item is *surveyed*
  (68 `HashMap`/`HashSet` mentions across 14 files; the only iteration is
  `temp_to_reg.values().any(…)`, an existence test) but not classified site by site —
  TICKET-086. A survey is not an audit, and a round number read as a baseline is the
  habit these gates exist to break.

### Process notes (expensive to rediscover)
- **`#[expect(dead_code, reason = "…placeholder")]` in a diff is a finding.** It is the
  tree telling you an item is unused and the author knowing it. Grep for `expect(dead_code`
  and for `allow(dead_code)` when auditing reachability; both were carrying a real gap here.
- `include_str!` paths in `compiler/tests/*.rs` are relative to the test file, so
  `include_str!("fixtures/trading_core_v1.x3")`. Only that one fixture in
  `compiler/tests/fixtures/` parses under a current build — the others are deliberately
  invalid or stale (TICKET-083's shape at another scale).
- A test file that walks the tree needs `CARGO_MANIFEST_DIR` plus relative `../vm/src`
  paths; `x3-lang`'s crates are siblings of `compiler/`, not children.
- `rg -o '<alternatives>'` is the fast way to find *which* alternative matched when a
  pattern hits an unexpected line. It is how `rand::` inside `Operand::Reg` was found.

### Next task seed
- **TICKET-086** (high): classify all 68 hash-map sites, fix any ordering-sensitive one,
  then make the classification a two-directional gate. This is what closes PHASE 42.
- **TICKET-085**: wire the register allocator or delete it; either way "placeholder"
  leaves the tree.
- **TICKET-083 / TICKET-084**: 6 of 23 `examples/*.x3` do not check (no gate notices),
  and `x3c check` calls a syntax error a "lowering failed".
- Then the remaining depth: **48/49** (adversarial matrix and property list item by
  item), **39**'s artifact provenance (TICKET-075), **TICKET-081** (one home for the
  decimal-literal rounding rule), **TICKET-082**, **TICKET-063** step 1, **TICKET-058**,
  **TICKET-068/069**, **TICKET-074**, **TICKET-080**.

## 2026-09-19 — PHASE 42 closed: the artifact was not reproducible (`cb72223a4`)

### The defect (this is the one to remember)
- **`Operation::Emit { data: HashMap<String, String> }` reached the artifact's bytes.**
  `emitter.rs:310` writes `format!("{}:{:?}", name, data)`, `Debug` for a `HashMap`
  prints in **iteration order**, and `RandomState` is seeded **per map instance** — the
  thread-local seed is initialised once and bumped on every `RandomState::new()`. So two
  maps built from the same entries in the same process iterate differently.
- **Measured before the fix: 12 identical compiles of one program produced 6 distinct
  artifacts.** After: 24 compiles → 1 artifact, payload `{"arg0": …, "arg1": …, "arg2": …}`.
- `Operation::RouteScore { weights }` had it too, via `weights.iter().collect()` into the
  payload vector, and `verify_route_score` iterates the same map to name every overweight
  key — so the *diagnostic* could differ as well.
- Fix: both IR fields are `BTreeMap`. The rendered shape is unchanged (`{:?}` on a sorted
  map is the same `{"k": "v", …}`), so only the order moves.
- Any `HashMap` in an IR type that the emitter renders or serialises is this bug. Grep for
  `HashMap` in `ir.rs` and for `{:?}` in `emitter.rs`.

### The test that should have caught it
`property_tests.rs` had asserted "IR emission must be deterministic" for a `RouteScore`
**with one weight**. A one-entry map has no order to disagree about, so the assertion was
true and said nothing. It now generates three. **Same failure mode as** the
`verify_valid_receipt` that asserted success against `block_hash = [0xab; 32]`
(TICKET-065) and the `keccak256` test that asserted "not all zeros": *a test whose input
cannot express the property it names.* When a test is about ordering, determinism, or an
absence, check that its input can actually exhibit the thing.

### House rules from this round
- **A `HashMap` field on a `pub` IR type is a trap even with no callers.** The census
  finished by fixing `SourceMap::files()` (returned `self.files.values()`, i.e. map order)
  to sort by `file_id`, because it had no callers — which is exactly when a trap is
  cheapest to remove. `regalloc.rs`'s `pub HashMap` assignment tables were left for
  TICKET-085 deliberately, and TICKET-085 now says why.
- **A survey is not an audit.** Round 56 classified the map census as "surveyed", which
  was the right word; doing the per-site work then found a live defect the survey had
  cleared. When a row is PARTIAL because something is "surveyed", finishing it is what
  finds the bug.
- **Don't build a type-scan gate for ordering.** A file-level `HashMap` allowlist would
  miss a new ordering-sensitive *use* in an already-listed file (which is how this
  survived), and a per-site regex has the `contains("rand::")` ↔ `Operand::Reg`
  false-positive problem. The byte-identity tests bite on any non-determinism including
  one no scan can see; that is where a gate belongs.
- Reproducible ≠ canonical. `FxHashMap` has a fixed seed so its iteration order *is*
  reproducible across runs, but it is still not an order the program chose.

### Next task seed
- **TICKET-083 / TICKET-084** (cheap, mechanical, both need no design): 6 of 23
  `examples/*.x3` do not check and nothing notices; `x3c check` reports a syntax error as
  `lowering failed`.
- **TICKET-085**: wire the register allocator or delete it; either way "placeholder"
  leaves the tree and the two `pub HashMap`s resolve.
- **PHASE 48/49** (the last depth rows): the adversarial matrix and the property list item
  by item — 48 adds reorg / nonce-conflict / restart cases the ledger says x3-lang lacks.
- Then: **TICKET-075** (venue guarantee → artifact), **TICKET-081** (one home for the
  decimal-literal rounding rule), **TICKET-082**, **TICKET-063** step 1, **TICKET-058**,
  **TICKET-068/069**, **TICKET-074**, **TICKET-080**.

## 2026-09-19 — PHASE 49: the eight named invariants (`769636207`)

### Facts
- The eight invariants are in **`vm/tests/trading_properties.rs`** (seven of them, reusing
  its `Host` harness — do **not** grow a second one) and **`vm/src/bridge.rs`'s test
  module** (invariant 7, where the header-proof fixtures live). The module doc of
  `trading_properties.rs` is the checklist.
- The accounting identity that makes invariants 1, 3 and 4 checkable:
  `credit` → `credits += a`, `net_deltas += a`; `debit` → `debits += a`,
  `net_deltas -= a`; `accrue_cost` → `costs += a`, `net_deltas -= a`, and appends to
  `cost_ledger`. Hence **`net_deltas == credits − debits − costs`** per asset, and
  **`balance − profit.net == costs`**.
- `TradingVm::profit(asset)` **assembles** `Profit::from_ledger(credits, debits, 0, costs)`
  and then reconciles it against `net_deltas`, refusing with
  `ProfitReconciliationMismatch`. So a test asserting "a passing floor implies
  `net_deltas >= floor`" is near-tautological. The version with force asserts against
  `balances` and `costs`, which different calls maintain.
- `verify_evm_header_proof`'s anchor: (a) `header_hash == keccak256(rlp_header)`
  (`X3_EVM_HEADER_HASH_MISMATCH`), (b) `header_hash == trusted` (`X3_EVM_HEADER_NOT_TRUSTED`).
  Both are reachable from a test **without any fixture** by generating bytes and hashing
  them — the property is about the anchor, not about a real block.
- Capability enforcement is `CapabilityManifest { providers, venues, bridges }`, checked at
  `trading.rs:688/726/812` with `UnknownCapability`.

### Process notes (expensive to rediscover)
- **`i128: From<u128>` does not exist.** Use `i128::try_from(..)`; and bound the
  proptest generator to values that fit (`0u128..1_000_000`) rather than `u128::ANY`,
  or the property becomes about the conversion.
- **A `///` doc comment on a `proptest! { … }` invocation is an `unused_doc_comments`
  warning**, which `-D warnings` turns into an error. Put the prose in a `//` comment
  before the macro and a short `///` on the generated `fn` inside it.
- Inside `vm/src/…` test modules the `proptest` macro is **not** in scope: add
  `use proptest::{prop_assert_eq, proptest};` and `use proptest::prelude::any;`.
- `BridgeError.code` is already a `&str`; `.as_str()` on it is the unstable `str_as_str`.
- The arb/trading fixture that reaches every economic path is
  `vm/tests/trading_properties.rs::operations()` — debt of 1_000_000 USDC through two
  swaps and back — plus `operations_with(|ops| …)` for one edit. A run commits iff the
  host's output is ≥ 1_000_001, so ~90% of the generated output space exercises the
  success branch.

### Next task seed
- **PHASE 48** is the last depth row: the 30-case hostile matrix. The existing negative
  tests cover single-process bad inputs (wrong asset/chain, stale quote, low profit,
  deadline, tamper). What the ledger says x3-lang lacks is everything a **restart**
  implies: reorg simulation, nonce conflict, restart during execution, process kill,
  RPC outage, partial domain outage, dropped/duplicate transaction, ledger corruption.
  Those need state that outlives the process, which is a design question before it is a
  test question — start by asking what the artifact + receipt pair is enough to re-derive
  (PHASE 45's version binding and TICKET-017's receipt legs are the existing answers).
- Then: **TICKET-083/084** (stale examples, misattributed error stage), **TICKET-085**
  (wire or delete the register allocator), **TICKET-075**, **TICKET-081**, **TICKET-082**,
  **TICKET-063** step 1, **TICKET-058**.
- PHASE 20 stays absent by its own instruction; PHASE 52's `X3E-2107` exemplar is
  TICKET-021.

## 2026-09-19 — PHASE 48 measured: 23 of 30 covered, the gap is durability

### Facts
- The matrix with per-case hit counts is `.ai/reports/x3lang-phase48-matrix-20260919.md`.
  **23 of the phase's 30 cases have tests that assert a refusal**; the uncovered six are
  one family — `restart during execution` (0 hits), `process kill` (0), `partial domain
  outage` (1), `RPC outage` (1), `reorg simulation` (2), `ledger corruption` (2).
  `dropped transaction` is unverified because the counts are too noisy to call.
- **Why the six cannot be written yet** (verified, not inferred): `TradingState` derives
  `Debug, Clone, Default, PartialEq, Eq` and **not** `Serialize`/`Deserialize`;
  `TradingVm` has no persistence; the atomic rollback snapshot is
  `VMState::atomic_snapshot: Option<VmSnapshot>` — memory only (note: that field is on
  **`VMState`** in `vm/src/x3_lang_vm.rs`, not on `TradingVm`; easy to conflate).
  There is nothing to restart *from* → TICKET-087, a design question first.
- Two existing partial answers to build on: PHASE 45's version binding, and TICKET-017's
  receipt legs (quote windows in the receipt so a restarted verifier re-derives the age).

### Process notes
- **rg alternation is `a|b`, not `a\|b`.** A first pass with `\|` returned 0 for every
  alternation and would have "proved" that asset/chain/signer tests do not exist. In a
  double-quoted bash string pass the bare `|`.
- **A hit count is a pointer, not proof.** `rg -c … | awk -F: '{s+=$2}'` counts matching
  *lines*, is noisy for generic words (`dropped` = 30 mostly unrelated), and says nothing
  about boundaries (176 slippage hits ≠ the ceiling tested at the boundary). The matrix
  file states this so a later reader does not read it as coverage proof.

### Next task seed
- **TICKET-087** (high): decide and write down which state outlives a process — the open
  atomic plan, the trading journal, the receipt, the finality view — then the six
  durability cases become tests. "Nothing is persisted, so an interrupted plan is not
  executed" is an acceptable answer **if stated**; the fail-closed reading is fine, the
  unstated one is not.
- **TICKET-083/084** (stale examples; `check` calls a syntax error "lowering failed") —
  both mechanical, both still open.
- **TICKET-085** (wire or delete the register allocator; resolves the last `pub HashMap`
  trap), **TICKET-075** (venue guarantee → artifact), **TICKET-081**, **TICKET-082**,
  **TICKET-063** step 1, **TICKET-058**.
- Phase status now: 54 phases complete in depth; **48** partial (the six above);
  **20** absent by its own text; **39** done with provenance outstanding (TICKET-075);
  **52** done with the `X3E-2107` exemplar outstanding (TICKET-021).

## 2026-09-19 — TICKET-087 closed: what survives a run (`c4b3a1621`)

### The decision
- **Nothing survives a run.** Per candidate, in
  `.ai/reports/x3lang-ticket087-decision-20260919.md`: the atomic plan is a region of the
  artifact (re-executing reaches the same state — PHASE 42); the trading journal is derived
  and `TradingVm::profit` already refuses with `ProfitReconciliationMismatch` when the
  assembled figure disagrees with `net_deltas`; the receipt is already durable (PHASE 45's
  version binding + TICKET-017's legs); the finality view is the caller's, because a cached
  view is a stale view.
- **Consequence, stated:** an interrupted atomic plan is **not resumed and not executed**.
  Fail-closed, and legitimate — a plan that did not commit has no receipt, and a receipt is
  the only thing a settlement layer can act on.

### The defect the question found
- **`ProductionBridgeAdapter`'s storage was process-global.** `storage_op` and `lifecycle`
  reached `static Mutex<HashMap<…>>` behind `storage_map()`/`lifecycle_states()`, so the
  storage a program's `storage_store`/`storage_load` uses belonged to the **process**: two
  adapters shared it and `COUNT` grew across runs. Reachable from the language —
  `Operation::StorageOp` (`lowering.rs:1386`,`1634`) → `CapabilityPayload::StorageOp`
  (`emitter.rs:871`) → `bridge.storage_op` (`executor.rs:1150`).
- Fixed by making both fields on the adapter (`BTreeMap`, per round 57's rule). A caller
  wanting persistence reuses one adapter — a call-site decision, not a process property.
- **Before/after run, not described:** `bridge::tests::two_adapters_do_not_share_host_storage`
  FAILED against the restored global and passes after. Always run the reproduction both
  ways; a test that has never failed is not yet evidence.

### Process notes (expensive to rediscover)
- **A keyword count is a way to find candidates, not a way to decide coverage.** Round 58
  marked `ledger corruption` uncovered on 2 hits; `vm/tests/trading_receipts.rs` covers it
  with **six** tests. The matrix now carries test *names* for that row. Any row marked
  covered is marked because a test was read.
- **`x3-crosschain-intent` is a member of the ROOT workspace, not of `x3-lang`.** Running
  `cargo check -p x3-crosschain-intent --no-default-features` from `x3-lang/` prints
  `cannot specify features for packages outside of workspace` — **one line that looks like
  a near-fix and is an invocation error measuring nothing.** Run it from `/tmp/x3lang-p29`.
- **Adding a path dependency is a feature-unification decision.** `bebbe55cf` (TICKET-065)
  added `x3-verification-router = { path = … }`, which carried the router's default `std`
  feature into `pallet-x3-settlement-engine`'s **wasm** graph and broke the embedded
  runtime; PR #361 fixed it with `default-features = false` +
  `std = […, "x3-verification-router/std"]`. In a workspace with a `wasm32v1-none` member,
  `{ path = "…" }` is not a local decision.
- `TradingOperation::Bridge` is `{ via, from: AssetKey, to: AssetKey, input: ValueRef,
  receiver: String }` — there are **two** `Bridge` shapes in `compiler/src/ir.rs` (109 and
  729) and the trading one is the second.
- Naming a local binding `host` inside a test **shadows the `host(output)` fixture fn**;
  the next `host(…)` call becomes "expected function, found Host". Use `first_host`,
  `poor_host`, …
- TICKET-082 re-measured after PR #361 touched the same file: **244 errors, unchanged** —
  it fixed a different leak. Re-measure a ticket whose file another agent edited; do not
  assume either way.

### Next task seed
- **TICKET-083 / TICKET-084** — mechanical, no design: 6 of 23 `examples/*.x3` do not check
  and nothing notices; `x3c check` reports a syntax error as `lowering failed`.
- **TICKET-085** — wire the register allocator or delete it; either way "placeholder"
  leaves the tree and the two `pub HashMap`s resolve.
- Then **TICKET-075** (venue guarantee → artifact), **TICKET-081**, **TICKET-082**,
  **TICKET-063** step 1, **TICKET-058**, **TICKET-068/069**, **TICKET-074**, **TICKET-080**.
- PHASE 48 has **one** unverified row left: `dropped transaction` (keyword count too noisy).
- Unverified-by-nobody's-fault: the bridge `lifecycle` map is write-only (`kind 0` returns
  its argument instead of the recorded state) — a bridge-semantics decision, not a bug fix.

## 2026-09-19 — `47e944662`: a hole under an example (TICKET-088, TICKET-084, TICKET-089)

### TICKET-088 — a route swap that states no `amount` lowered to zero
- `parser::fill_route_bridge_amounts` filled a `Statement::Bridge` from the intent's source
  endpoint, but its match arm named only `Bridge` **and so did the function** — so a swap in
  the same shape wrote `input_amount: 0` and was refused two passes later with
  "swap input_amount must be greater than zero". The parser's comment on a body-level swap
  claims both follow the same rule; that comment was false for swaps.
- Fixed: the pass handles both, and is renamed **`fill_route_step_amounts`** (it was named
  for the statement it happened to handle first, which is why the gap lived).
- Minimal reproduction: take `examples/timeout_refund_minimal.x3` (known good, one `bridge`
  step with no amount) and replace that step with a `swap` step with no amount.
- **The half that was a second defect**: the fill only reaches a step whose `from` is the
  source asset, so a *later* leg still lowered to `0`. Lowering now refuses it with the
  reason ("the compiler cannot infer one … write `amount <n>`"), which is the principle the
  comment above already states for a fractional amount. **Accept set unchanged** — a zero
  amount was refused later anyway — and the evidence for that is that no workspace test
  needed adjusting.
- `Statement::Swap`'s amount is `Option<Expression>`; `Statement::Bridge`'s is `Expression`.
  That asymmetry is why the parser can fill a bridge and cannot fill a swap.

### TICKET-084 — the failing stage is named
- `X3Error` gained `stage()` / `staged()` in `crates/x3-common/src/error.rs` (next to
  `span()`, exhaustive, so a new variant cannot lack one). Two call sites in `x3c.rs`
  hard-coded `"lowering failed"`; they now ask the error. **The inner message already named
  the right stage every time — the prefix is what a grep finds.**
- Generalisable: when a message has an outer stage prefix and an inner kind, they can drift;
  derive one from the other.

### TICKET-089 — a whole-number percent guard eats the next clause (NEW, high)
- Measured from the known-good `timeout_refund_minimal.x3` with one guard inserted before
  its `timeout`:
```
require slippage <= 50      OK
require slippage <= 0.5%    OK
require slippage <= 1%      FAIL  unexpected clause in intent body: Ident("45s")
require slippage <= 2%      FAIL  unexpected clause in intent body: Ident("45s")
require delta <= 0.01%      OK    (parses; then a semantic error, unrelated)
```
- **Cause**: `70%` is `Int` then `Tok::Percent`, while `0.5%` is one `Percentage` literal
  (the parser says so itself at `parse_whole_percent_bps`). A `require` guard's value goes
  through the general expression parser, so `1 %` becomes a binary **modulo** looking for a
  right-hand operand and the next clause is consumed. A percent guard written **last** in a
  body works — which is where every passing example and fixture happens to put it.
- Risk beyond the error path: `require slippage <= 1 % 50` would parse as a modulo, silently.
- Do **not** fix by removing `%` from the operator table; fix the guard's value path the way
  `parse_whole_percent_bps` and the rebalance-weight parser already do.

### TICKET-083 corrected
`examples/arb_solana_eth.x3` is **not** dead-dialect syntax — it is current dialect needing:
the guard renamed to `proof_complete` **with a proof named**, a `proofs required { … }` block,
base units for `min_output` (it carries no asset, so a fractional literal is refused), and
explicit amounts on the later route legs. The other five are genuinely the old
`contract … { fn … }` dialect.

### Process notes
- `require proof_complete` must **name** the proof for an intent declaration, and the name
  must be declared in `proofs required { … }`.
- A `proofs required { … }` block must be at **file scope**; inside an intent body the parser
  refuses it by design.
- `min_output` takes base units: a fractional literal is refused with a message pointing at
  base units. Working examples all use integers.
- `TradingOperation::Bridge` (ir.rs:729) is `{ via, from, to, input: ValueRef, receiver }`;
  the *other* `Bridge` at ir.rs:109 is a different shape.
- **Commit-message fidelity is worth checking.** I wrote that the pass "gets a name-shaped doc
  comment" before making that true; the honest fix was to do the rename rather than weaken the
  message, and to `--amend` (safe because nothing was pushed yet).

### Next task seed
- **TICKET-089** first: the highest-value parser defect found — a natural spelling silently
  mis-parses and can mean a modulo where a bound was intended. Table test over guard kinds ×
  spellings (`50`, `50%`, `1%`, `0.5%`, `0.01%`) plus a modulo-still-parses test.
- **TICKET-083**: retire the five old-dialect examples with a README, fix
  `arb_solana_eth.x3` per the note above, and add the gate so the count stops being manual.
- **TICKET-085**, then TICKET-075/081/082, TICKET-063 step 1, TICKET-058, TICKET-068/069,
  TICKET-074, TICKET-080. PHASE 48 has one unverified row (`dropped transaction`).

## 2026-09-19 — TICKET-089 closed: a percent that was a modulo (`4d5afba72`)

### Facts
- `require slippage <= 1%` did not parse; `0.5%` did. **Cause**: the literal reader
  implements only `Int.Dot.Int.Percent`. `1%` is `Tok::Int` then `Tok::Percent`, so the
  `%` reached the expression parser, was read as the **modulo** operator, and consumed
  the next token as its right-hand operand — in a guard, the next clause. Hence
  `unexpected clause in intent body: Ident("45s")`, naming a **correct** line.
- It survived because a percent guard written **last** in a body has no following clause
  to swallow, which is where every passing example and fixture puts its own.
- **Where the fix is**: `parser.rs::parse_guard_bound` — a two-token lookahead used for a
  guard's right-hand side. NOT the literal reader: implementing `Int.Percent` there broke
  `test_parser_coverage.rs::expression_arithmetic`, whose `let x = 1 + 2 * 3 - 4 / 5 % 6;`
  is a modulo between integer literals. The literal reader's comment now says which form
  it implements and that the omission is deliberate.
- **Boundary, now pinned** by
  `a_percent_in_a_guard_bound_is_a_percentage_and_a_modulo_everywhere_else`: a modulo
  between non-literals is untouched; `5 % 6` in a `let` is untouched; inside a guard's
  *bound* `Int %` is a percentage, so a modulo there needs parentheses. The narrowing is
  deliberate — the spelling it replaces made a bound whose value was the text
  `5 Percent 6`.

### The test that was passing for the wrong reason (fourth instance this session)
- `test_slippage_units.rs::the_mainnet_ceiling_reads_the_same_unit_as_the_guards` asserted
  `("require slippage <= 5%", "500", /* accepted */ false)` and **passed because the guard
  failed to parse** — `compile_with_mode` returned `Err` and the assertion on `is_ok()` was
  satisfied by a syntax error, not by the ceiling. A second row (`501` vs a `501` ceiling)
  was wrong the same way. The check refuses only what is *above* (`bound > policy`), so
  equal is inside.
- The family so far: `verify_valid_receipt` succeeding against `block_hash = [0xab; 32]`;
  a `keccak256` test asserting only "not all zeros"; a determinism property generating a
  **one-entry** map; and now a ceiling assertion satisfied by a parse error. **Rule**: when
  a test asserts an outcome, ask whether the input can reach the *reason* the test names —
  an `is_ok()`/`is_err()` assertion is satisfied by any error for any reason.

### Process notes
- **A comment promising behaviour the code does not implement is a trap for the fixer, not
  just for the reader.** `// Check for percentage literal: Int.Dot.Int.Percent or
  Int.Percent` sent me to the wrong fix site; the test that caught it was one I had not
  read. When correcting such a comment, say which form is implemented **and that the
  omission is deliberate**, or the next reader will "fix" it again.
- `parse_require_guard` uses `self.parse_expr()` for a comparison's right-hand side. A
  bound-specific entry point is the place for bound-specific spelling.
- `Tok::Float` + `Percent` was already handled in the literal reader; only `Tok::Int` +
  `Percent` was missing, which is why the split was whole vs fractional.
- `master` moved twice during this round (`47e944662` → `790a1cc8d` PR #362, a mainnet
  gate that rebuilds the runtime and compares the hash). Both rebases were clean and the
  base did not touch `x3-lang`. The reproducible-build theme from this session's earlier
  rounds is now visible in other agents' gates.

### Next task seed
- **TICKET-083**: retire the five dead-dialect examples with a README, fix
  `examples/arb_solana_eth.x3` per the recorded recipe, and add the gate over
  `examples/*.x3` so the count stops being a manual observation.
- **TICKET-085**: wire the register allocator or delete it (the last `pub HashMap` trap and
  the word "placeholder" both go).
- Then TICKET-075 (venue guarantee → artifact), TICKET-081, TICKET-082, TICKET-063 step 1,
  TICKET-058, TICKET-068/069, TICKET-074, TICKET-080. PHASE 48 has one unverified row.

## 2026-09-19 — TICKET-083 closed: 19 of 19 examples, and two defects under them (`6f6bc0463`)

### The sweep baseline moved — record it, or it reads as a regression
```
before  files=23 check=17 build=17 warning-free=17 run-artifact=17
after   files=19 check=19 build=19 warning-free=19 run-artifact=18
```
`files` fell because five examples were retired; `run` is 18 because `arb_scope.x3` refuses
a bare `x3c run` **by design** (its floors are measured guards, a dry run has no host
measurement). Both are explained in the round report and the ticket.

### Facts
- `examples/legacy/` holds the five dead-dialect files (`arb`, `flash`, `jit_lp`,
  `mev_smooth`, `x3_coin_layer`) with a README giving, per file, why it is **not**
  rewritten — `flash.x3` cannot be (PHASE 20 is absent by its own text), `jit_lp`/`mev_smooth`
  have no feature to translate to, `x3_coin_layer` calls itself `(Pseudo)`. `sweep2.sh`
  globs `examples/*.x3`, so `legacy/` is out of it by construction.
- `examples/arb_scope.x3` is new and is the only example of PHASE 37's `arb` + `venue`
  form. A bare `x3c run` on it refuses with `X3_GUARD_UNMEASURED`; it settles with
  `--measured-profit-bps`/`--measured-slippage-bps` or `simulate --state`.
- `x3c fmt` **writes in place** — `x3c fmt <file>` reformats the file; there is no stdout
  mode. Use `--check` to test idempotence. (I learned this by redirecting its output and
  finding a two-line file.)
- The formatter moves comments to declaration boundaries, so a comment written inside a
  declaration ends up above it or at the end of the file. An example meant to be a document
  should put its prose in a **file header**, before the first declaration, where the
  formatter leaves it.

### TICKET-090 — the formatter corrupted every percent literal
- `formatter.rs`'s literal match ended in `_ => self.write("/* literal */")`, so
  `LiteralExpr::Percentage` became a **comment** — source the parser rejects. Reachable
  from the phase's own example: `require slippage <= 0.5%`.
- Fixed: `Percentage` is rendered (`value.as_str()`, which already carries the `%`). The
  other five (`RawString`, `ByteString`, `Char`, `Byte`, `Size`) have **no parser arm** that
  produces them, so they cannot come from source; the fallback says so and carries
  `debug_assert!(false, …)`.
- Found by asking whether an example was formatter-stable, which is a question worth asking
  of any file the tooling writes.

### TICKET-091 — two parsers for one language
- The Python surface (`cli.py`, `typechecker.py`, `registry.py`) drifted from the compiler
  **three times in one round**, each a spelling the compiler or the *formatter* accepts and
  it did not: `proof_complete`, `finality.<chain>`, and `refund <asset>` (the default
  receiver is `sender`). All three are read now.
- **The gate written against the ideal assertion failed for 13 of 19 examples**, and the
  reasons are scope: it needs the first item to be `intent` (seven examples start with
  `finality_policy` or `risk_policy`), validates address *shape* (40 hex chars) so `0xA1` is
  refused, knows nine guard kinds of eighteen, and **crashes with `list index out of range`
  on four files** — a parser that crashes on valid input has no error surface at all.
- So the shipped gate (`tests/test_surface_drift.py`) is scoped to the examples the suite
  itself uses, **derived from the suite rather than listed** so a new fixture joins
  automatically. TICKET-091 records the rest as a decision.
- **Lesson**: writing a gate against the behaviour you want, when that behaviour is not
  there, produces a failing test rather than a gate. Write it against the boundary that
  exists and ticket the gap.

### Process notes
- A gate over a directory needs the **too-few-files guard** (`> 10`) or a moved directory
  makes it vacuous. Used in three places now: the determinism audit, the examples gate, and
  the surface-drift gate.
- `--deny-warnings` in a gate over examples is what caught `on_fail refund` for a lock the
  `timeout` already refunds (the `no_double_refund` invariant). An example that warns shows a
  reader a program the compiler complains about.
- Python tests that `subprocess.run(..., check=True)` hide the child's stderr; run the child
  directly to see why it failed.
- `git mv` in this clone needs escalation; `cat > file` in the workspace does not.

### Next task seed
- **TICKET-091**: decide the Python surface's scope, state it in its module doc, make every
  refusal a named error (four files currently raise `IndexError`), and shape the gate to the
  answer.
- **TICKET-085**: wire or delete the register allocator — the last `pub HashMap` trap and the
  word "placeholder" both go.
- Then TICKET-075, TICKET-081, TICKET-082, TICKET-063 step 1, TICKET-058, TICKET-068/069,
  TICKET-074, TICKET-080. PHASE 48 has one unverified row (`dropped transaction`).

## 2026-09-19 — TICKET-091 landed: the surface that had no error surface (`97984a30d`)

### Facts
- **`_intent_line_index` returns `len(lines)`** when a file's every top-level item is a
  declaration it skips — which is the whole file for the five examples whose subjects are
  bare `atomic_choice` / `objective` / `parallel` / `strategy` blocks. `parse_file` then did
  `lines[start]` and raised `IndexError`. Fixed: `X3_PARSE_NO_INTENT`, with a line and a
  message that says what the file is. Its docstring already said the caller refuses what it
  does not recognise; the caller did not.
- The neighbouring refusal said `expected intent <name> {` **at the offending line**, sending
  a reader after a typo in a construct that is correct in the language. Now it says the file
  declares no intent and quotes the line.
- **`runner.py` had no error surface**: a clean `X3ParseError` escaped as a traceback. `run()`
  now returns `{status: error, errors:[…]}` (the shape `typechecker.errors_to_json` uses) and
  `main()` exits 1 on it.
- **The handler belongs in `run()`, not `main()`** — and the reason is not obvious: `cli` is
  loaded **per call** by `load_module`, so a second load yields a *different*
  `X3ParseError` class and an `except cli.X3ParseError` in `main()` would not match the one
  the parser raised. `main()` just got `NameError: name 'cli' is not defined`.
- Measured with codes: of 19 examples, **3 readable**, 16 refused by name —
  `X3_PARSE_NO_INTENT` × 10 (scope, now stated), `X3_PARSE_REQUIRE` × 3 (guard kind it does
  not know, e.g. `route_score`, where the compiler would accept it), `X3_PARSE_RECEIVER` × 2
  (stricter address shape; the compiler accepts `0xA1`), `X3_PARSE_OPERATION` × 1
  (`fallback`).
- The scope is now in `cli.py`'s module doc: one `intent` per file, nine kinds of eighteen, a
  stricter address shape, and "nothing here may raise anything but `X3ParseError`".

### Process notes
- **Round 62's figure was wrong: "thirteen of nineteen" should have been sixteen.** The list
  the count came from had sixteen entries. Corrected in place in both the report and the
  ticket, with the correction stated rather than quietly fixed — a number in a report is what
  a later reader quotes, and this session has already had a figure (`17/23`) read as a
  baseline.
- **A gate that derives its inputs from the suite's source will find a test's *negative*
  fixtures too.** The new runner test used `examples/parallel_dag.x3` as an unreadable
  example; the drift gate (which requires the suite's examples to be readable) pulled it in
  and failed — correctly. A test whose subject is "a file with no intent" should write its
  own fixture. It does now.
- Two gates, each shaped to what is true: one over **all** examples asserting *nothing
  crashes* (true regardless of scope), one over the suite's fixtures asserting readability.
  The first needs no scope argument; the second pins the drifts fixed in round 62.
- **Relaxing a receiver-address check is a safety decision, not a formatting one.** Two
  examples are refused for a short `0xA1`-style receiver the compiler accepts; the surface is
  stricter on purpose and the drift is recorded rather than "fixed" by loosening a check.

### Next task seed
- **TICKET-091 (rest)**: decide whether the surface should read more — `X3_PARSE_REQUIRE` × 3
  and `X3_PARSE_RECEIVER` × 2 are drift (files the compiler accepts and the surface refuses).
- **TICKET-085**: wire or delete the register allocator; the last `pub HashMap` trap and the
  word "placeholder" both go.
- Then TICKET-075, TICKET-081, TICKET-082, TICKET-063 step 1, TICKET-058, TICKET-068/069,
  TICKET-074, TICKET-080. PHASE 48 has one unverified row (`dropped transaction`).

## 2026-09-19 — TICKET-085 closed: the register allocator is deleted (`b8ea24aec`)

### The decision, and why it is not "we gave up"
- **The ISA has no registers to allocate.** The `Operation` set is economic records —
  `Lock`, `Swap`, `Bridge`, `Require`, `MultiHopSwap`, … There is **no `Operation::Add`, no
  `Operand::Reg`, and no def/use slot on any variant.** So `compute_live_ranges` computed
  ranges over temporary ids nothing declares, and `patch_operation` had nothing to rewrite.
- The VM's register file exists for one purpose: `REQUIRE` reads `r0`. Operands carry
  thresholds and constants, not register numbers.
- Wiring would mean building a register machine under a language that has one, plus an
  artifact-format change to carry operand slots — a language redesign, and nothing in the
  56 phases asks for it.
- Deleted: `compiler/src/regalloc.rs`, `pub mod`, the two re-exports, both entry points, and
  `mod regalloc_wiring_tests` (including PHASE 42's byte-identity ratchet, which did its job —
  it pinned the pass as a record, and this is the other documented outcome).
- **Provably invisible to the artifact**: five examples built with the compiler from before
  and after, in `git worktree add /tmp/pre-regalloc HEAD`, are byte-identical
  (`arb_scope 97d93d56`, `arb_solana_eth 3171984c`, `flagship_b52 f36e7cd8`,
  `strategy_module 5e51e54a`, `trading_core_v1 bcebd01e`). Sweep unchanged at 19/19.
- Census re-measured: **50 mentions / 12 files** (was 68 / 14). The only map *iteration* the
  census found (`temp_to_reg.values().any(…)`) went with the file.

### Process notes, and one that nearly cost the round
- **Deleting by line range ate the wrong brace, twice.** The first attempt removed
  `verify_bytecode`'s closing `}` because the range *started* at a line I had never asserted —
  I had asserted the last line and the neighbours after it, not the first. The second attempt
  caught a wrong guess (line 41 is `pub mod risk;`) *before* deleting anything.
  **Rule**: assert the first line of every deleted range, the last line, and the neighbours
  on both sides — then delete bottom-up. "Compute the exact range" is not enough if the
  range's own endpoints are unverified.
- `git worktree add /tmp/pre-regalloc HEAD` is the clean way to build the *previous* tree
  while the working tree holds uncommitted deletions. Cheaper and safer than `stash`.
- A test that pins "X is a record, not a rewrite" is a **ratchet with two documented
  outcomes**: wire it, or delete it. Both are fine; leaving it is not.

### Next task seed
- **TICKET-091 (rest)**: decide whether the Python surface should read more — `X3_PARSE_REQUIRE`
  × 3 and `X3_PARSE_RECEIVER` × 2 are drift (files the compiler accepts and the surface
  refuses). The receiver check is a safety decision; it was left alone on purpose.
- **TICKET-075** (venue guarantee → artifact), **TICKET-081** (the rounding rule's second
  home), **TICKET-082** (the intent crate's `no_std` path, 244 errors re-measured twice).
- Then TICKET-063 step 1, TICKET-058, TICKET-068/069, TICKET-074, TICKET-080. PHASE 48 has
  one unverified row (`dropped transaction`).

## 2026-09-19 — TICKET-081 closed: one rounding rule, two scales (`a24b10f3b`)

### The finding that decides the design
- The compiler's **`MAX_DECIMALS` is 38**; `fixed::MAX_SCALE` is **18**. Measured:
  `decimal_to_base_units("1.0000000000000000001", 19, Exact)` is `Ok(10000000000000000001)`
  while `Decimal::<18>::from_parts(1, "0000000000000000001")` is `None`.
- So the obvious unification — delegate the compiler's path to `Decimal` — is **impossible**:
  an 18-scale `u128` mantissa cannot be widened to 38 without giving up almost all of its
  range. It is a property of the representation, not an omission. The ticket's criteria did
  not anticipate it.
- The boundary is now asserted in the tree
  (`test_amount_conversion_agreement.rs::an_asset_finer_than_the_fixed_scale_is_the_compilers_alone`).

### What was unified
- **The decision, not the arithmetic.** `fixed::apply_rounding(quotient, discarded, rounding)`
  is the one place the three directions are decided; `div_rounded` calls it and so does
  `decimal_to_base_units`. Also `fixed_math.rs::the_rounding_decision_is_one_function`
  (including `Up` at `u128::MAX` refusing rather than wrapping).
- **The `Exact` refusal stays BEFORE the arithmetic** so a literal that is both lossy *and*
  too large reports the loss, not the overflow. Moving the shared rule to the end of the
  function would have swapped that; the tidier version is the wrong one.

### The differential test as the pattern
- `compiler/tests/test_amount_conversion_agreement.rs`: 13 literals × 6 decimals ×
  3 directions through both paths, asserting `Ok(v) ↔ Some(v)` / `Err(PrecisionLoss) ↔ None`,
  and **counting the comparisons** so a table that compared nothing cannot pass.
- **Write it first.** It is the safety net for any refactor that follows, and it is what
  proves behaviour preservation — green against two independent implementations, still green
  after one became a caller. The sweep unchanged at 19/19 is the same fact at the artifact
  level.

### A criterion that held after all
- **`10u128.checked_pow` appears exactly once** in `compiler/src`, `crates/x3-common/src`,
  `vm/src` — the compiler's line 463 — because `fixed` computes powers through `pow10`'s
  match table. Round 63's guess that it could not hold was wrong, and measuring is what said
  so.

### Next task seed
- **TICKET-091 (rest)**: the Python surface's scope — `X3_PARSE_REQUIRE` × 3 (a guard kind it
  does not know) and `X3_PARSE_RECEIVER` × 2 (a stricter address shape) are drift: files the
  compiler accepts and the surface refuses. The receiver check is a safety decision.
- **TICKET-075** (venue guarantee → artifact), **TICKET-082** (the intent crate's `no_std`
  path, 244 errors re-measured twice).
- Then TICKET-063 step 1, TICKET-058, TICKET-068/069, TICKET-074, TICKET-080. PHASE 48 has one
  unverified row (`dropped transaction`).

## 2026-09-19 — TICKET-082 fixed, and a critical fail-open underneath (`6d6dedca6`)

### The critical finding (TICKET-092)
- **`verify_svm_validator_quorum` counted a validator's stake without verifying its
  signature** in the configuration the *runtime ships*: the Ed25519 check was
  `#[cfg(any(test, feature = "std"))]` **with no `else`**, so a build without `std` and not
  under `test` — `pallet-x3-settlement-engine` depends on the crate with
  `default-features = false`, and that is the wasm graph — skipped verification and ran
  `signed_stake += stake` anyway. A proof naming any validator and carrying any 64 bytes
  cleared the threshold.
- The compiler had been pointing at it: `warning: unused variable: signature` **at that
  loop**, in the no-std build. **An unused variable in security code is a finding, not a
  nit** — it means the value the check needs was not used.
- Fixed by deleting the gate: `ed25519-dalek` is `no_std` with `alloc`, the feature set the
  crate already declares, so verification runs in both configurations.
- Found only *because* the no_std build started compiling: with 244 errors nobody could reach
  the path. **Fixing a build can expose what the build was hiding.**
- Swept for the same shape repo-wide: every other `#[cfg(any(test, feature = …))]` gates a
  *mock* on `test`/`dev-mock`/`test-utils`, which is correct. This was the only `cfg` gate on
  a verification.

### The test that hid it (fifth instance of one pattern)
- `verify_quorum_requires_threshold_met` asserted `result.is_err() || result.is_ok()` —
  **true for every value** — under a message saying "signature verification should be
  attempted". Under `std` it was `Err`, under the gated build `Ok`, and the assertion took
  both. Now `Err(InvalidSignature { .. })`, plus a real forgery test (a real key, a real
  validator, a signature over the **wrong message** whose stake alone clears a 1/3 threshold —
  so nothing but verification can refuse it) and the honest signature reaching the threshold
  so the fixture cannot be the reason it passes.

### TICKET-082 itself
- The crate was **already** `#![cfg_attr(not(feature = "std"), no_std)]` with
  `extern crate alloc`; its `std` feature only turned on `serde/std`, `hex/alloc`, `sha2/std`
  and the router's `std`. The first two are needed by every path: **230 × `String:
  serde::Deserialize` unsatisfied** and **13 × `cannot find function hex::encode`**.
  Declared where needed now: `serde` gains `alloc`, `hex` gains `alloc`, and `std` keeps the
  rest. **244 → 0 errors.**
- Measure it from the **root** workspace: from `x3-lang/` cargo prints
  `cannot specify features for packages outside of workspace` — one line that looks like a
  near-fix and measures nothing.

### Next task seed
- **TICKET-063** is now the largest item of this class: `pallets/x3-settlement-engine` reads
  `merkle_proof[0..2]` as roots and never walks a path — a receipt "proof" that proves
  nothing, in the same subsystem this round found a forgery in.
- Then **TICKET-091 (rest)** (the Python surface's scope), **TICKET-075** (venue guarantee →
  artifact), TICKET-058, TICKET-068/069, TICKET-074, TICKET-080. PHASE 48 has one unverified
  row.

## 2026-09-19 — TICKET-063 (b) closed: two structures over the same 32 bytes (`922fbde16`)

### The finding, which is why the requirement is not merely a name
- **`pallet-cross-chain-validator` and `pallet-x3-settlement-engine` validate two different
  structures over the same 32 bytes.**
  - the validator pallet checks `merkle_root_of(proof_to_leaves(proof)) == merkle_root` — a
    **flat Merkle tree** over 32-byte leaves. It has **no RLP, no keccak and no header
    anywhere**; `block_hash` and `state_root` are assertions checked only for being non-zero.
    Its model is an *authorized-relayer header store*, which its module doc says.
  - the settlement pallet walks an **MPT** receipt proof — `rlp(index)` key, node path,
    receipt leaf — against that same stored root.
- So a submitter who attests a block's leaf-Merkle root stores a valid root and makes that
  block **impossible to settle**, with nothing complaining: the shape is valid, the root is
  stored, only the walk refuses. **Liveness, not theft** — hence stated rather than enforced,
  because there is nothing to enforce against.
- Stated at four points: the validator module doc (a new section), `validate_evm_header`,
  `EvmHeaderInfo::merkle_root`, and `verify_evm_receipt_proof` on the consuming side.

### Facts worth keeping
- The chain **fails closed** on unbound SVM proofs: `AllowUnboundSvmProofs = false` in
  `runtime/src/lib.rs`; the pallet's mock sets it `true` because the mock stands in for a
  binding validator. So TICKET-063 (a) is contained, not live.
- The settlement pallet's test runtime does **not** include `pallet-cross-chain-validator`,
  and the pallet is **not a Cargo dependency** of it — so a cross-pallet test needs the chain
  runtime's setup (`cargo test -p x3-chain-runtime --lib settlement_proof` is the existing
  pattern). TICKET-063 (d) is recorded for that reason.
- A per-height header lookup (TICKET-063 (c)) would **loosen** a check that is currently the
  stricter of the two. Liveness improvement, not security — do not take it without a
  requirement.

### Process notes
- **FRAME pallets are expensive to build**: `cargo check -p pallet-cross-chain-validator`
  took 1m35s cold. Budget for it; the x3-lang workspace is ~15s by comparison.
- A doc-only commit still deserves a `cargo check` (intra-doc links) and the edited pallet's
  test suite. 19 tests was 45s.
- When updating a long ticket entry with python, anchor on a *short unique* fragment. A
  3-line anchor that spans a line-wrap difference fails the assert — and because I assert
  before writing, nothing was written, which is the behaviour I want.

### Next task seed
- **TICKET-063 (a)** — the SVM binding (an account-state / bank-hash proof the workspace does
  not have), and **(d)** — the runtime-level test that makes (b) executable.
- **TICKET-091 (rest)** — the Python surface's scope.
- Then TICKET-075, TICKET-058, TICKET-068/069, TICKET-074, TICKET-080. PHASE 48 has one
  unverified row (`dropped transaction`).

## 2026-09-19 — the no-default-features class is gated (`3230566e4`)

### The structural hole this closes
- `cargo check --workspace` builds every crate **with its default features**, so a dependency
  feature the code cannot do without — gated behind `std` for no reason — is invisible until
  something turns the feature off. The runtime sets `default-features = false` on **64 local
  crates**, so "off" is the configuration the chain ships. That is how TICKET-082 (244
  errors) and TICKET-092 (an Ed25519 check `cfg`-gated away, no `else`) both stayed hidden.

### The gate: `scripts/check-no-default-features.sh`
- Derived list: greps for the `no_std` posture (`#![no_std]` or
  `#![cfg_attr(not(feature = "std"), no_std)]`), maps file → package, keeps workspace members.
- **One crate per cargo invocation, and that is the whole point.** The first version passed
  every crate to a single invocation and **passed with TICKET-082 deliberately reintroduced**,
  because cargo unifies features across a graph. Measured: isolated intent crate = 14 errors;
  via its pallet = 0; all 64 in one invocation = 0.
- Uses a crate's own `alloc` feature when it declares one: `x3-common` fails
  `--no-default-features` (2 × `String: serde::Serialize`) and passes with `--features alloc`,
  so that is its configuration. Passing the feature a crate declares beats failing it.
- `GATES_VARIANTS` (opt-in): **~13 min cold, ~2 min warm** — one cargo invocation per crate.
  A gate whose cost is unknown is a gate that gets removed the first time it is slow.

### The finding (TICKET-093)
- **5 of 87** crates declare `no_std` and cannot build that way: `x3-chain-runtime` (1 error,
  E0599), `x3-external-chains` (184), `x3-gateway-risk-engine` (7), `x3-liquidity-core` (3),
  `x3-sdk` (216). None declares an `alloc` feature, so `--no-default-features` is their only
  non-std configuration.
- They sit on a `KNOWN_UNBUILDABLE` list **with measured counts**, so the gate fails on
  anything not on it and the list cannot grow silently. The summary prints the known count
  every run so it does not become a baseline. **Both directions verified**: green with the
  list, and exit 1 naming `x3-sdk` as new drift when one entry is removed.

### Process notes
- **`cargo -p` selects by *package* name, not by the dependency key.** `pallet-svm-runtime =
  { package = "pallet-svm", … }` in `runtime/Cargo.toml` produced `-p pallet-svm-runtime`,
  which matched nothing — the gate's first run failed on exactly that. Read the `package`
  field.
- **A heredoc whose body contains the same delimiter terminates early.** Writing a python
  script that itself contained `python3 - <<'PY'` … `PY` broke the outer heredoc. Use a
  distinct outer delimiter.
- FRAME pallets are slow to build: a single `cargo check -p pallet-cross-chain-validator`
  cold was 1m35s; 87 isolated checks cold were ~13 min.
- When a new gate goes red, either fix the finding or pin it **with reasons and counts** and
  make the gate fail on drift — a red gate gets skipped, and a skipped gate protects nothing.

### Next task seed
- **TICKET-093**: bring the five off the list — `x3-chain-runtime` is one error, the other two
  are ~200 each.
- **TICKET-063 (a)/(d)**: the SVM binding, and the runtime-level test that makes the
  receipts-root requirement executable.
- **TICKET-091 (rest)**: the Python surface's scope. Then TICKET-075, TICKET-058,
  TICKET-068/069, TICKET-074, TICKET-080. PHASE 48 has one unverified row.

## 2026-09-19 — two crates off the no_std list, and PHASE 43's scope (`984b9aed6`)

### `x3-chain-runtime`: `tuples-96` was in `default`
- `construct_runtime!` builds a tuple of the pallets and this runtime exceeds the default
  limit, so `frame-support/tuples-96` is required for the crate to compile **at all** — and
  it was in the crate's `default`, so `--no-default-features` turned it off. It is on the
  dependency now. **A feature that has to be on for the build to succeed is not a feature.**
- **Measurement correction**: my first reading said "1 error" because I ran `cargo check`
  **without `SKIP_WASM_BUILD=1`** — the "error" was the runtime's build script, not the crate.
  In the gate's environment it has 8. **Measure in the gate's own environment.**

### `x3-liquidity-core`: an anti-rug score in `f64` (and PHASE 43's real scope)
- Three float uses in `anti_rug.rs`: two supply shares and
  `ln(lock_duration) * 20 / ln(30 days)`. `ln` is not in `core` (hence the build failure) and
  a platform's `ln` is the platform's — **a score built from a float logarithm varies with
  where it ran.** Fixed with integer ratios and `ilog2(d) * 20 / ilog2(30 days)`, which is the
  *same ratio* (`ln a / ln b == log2 a / log2 b`) with no float.
- Behaviour change, stated: the duration sub-score can differ by at most 1 point of 20; the
  percentages are now exact (the float form was lossy above 2^53). Pinned ladder
  `1s→0, 1h→10, 1d→15, 7d→18, 30d→20` and a reachable maximum of 100.
- `anti_rug.rs` had **no test module** — that is how a float survived in a security-adjacent
  score. Writing tests found an arithmetic error in *my own expectation* (96 for 7 days; the
  code says 98). **Pin values, do not describe them.**
- **PHASE 43's audit scanned `x3-lang/{compiler,vm,crates/*}/src` only.** Its claim holds in
  that scope; this crate is outside it. Repo-wide:
  `rg -l 'as f64|as f32|: f64|: f32|\.ln\(\)|\.sqrt\(\)|\.powf\(' crates pallets runtime
  services apps` → **551 files**, unclassified. Not all consensus-sensitive (simulators,
  monitors, dashboards, benchmarks legitimately use floats), but
  `x3-staking-analytics/{reward_calculator,slash_tracker,staking_ledger,validator_stats}.rs`,
  `x3-swap-router/atomic_execution.rs` and `x3-gpu-validator-swarm/cpu_validator.rs` are the
  ones whose names say a balance or a reward is computed. **TICKET-094.**

### Process notes
- `cargo check -p <crate>` alone is often *not* the environment a gate uses; check the env
  (`SKIP_WASM_BUILD`, features) before quoting a count.
- The gate's known list went 5 → 3, and removing entries is the verification: green now
  *requires* those crates to pass.
- Clippy caught an unnecessary `as u32` cast in the new code — run it on the changed crate
  before committing; the workspace-wide clippy is far slower.

### Next task seed
- **TICKET-094** (high): classify the 551 float sites, convert the ones that reach a
  balance/reward/slash/settlement decision, and gate the consensus-sensitive set.
- **TICKET-093**: the three remaining on the list.
- **TICKET-063 (a)/(d)**, **TICKET-091 (rest)**, TICKET-075, TICKET-058, TICKET-068/069,
  TICKET-074, TICKET-080. PHASE 48 has one unverified row.

## 2026-09-19 — a payment rate that binary could not hold (`10f0262e6`)

### The defect, and it is the concrete form of PHASE 43
- `x3-gpu-validator-swarm/src/payment.rs::calculate_reward` applied its rates as
  `(verification_bonus * 1000.0) as u128`. **A cast of a float product truncates**: `1.001` is
  `1.0009999999999999…` in binary, so `1.001 * 1000.0` is `1000.9999999999999` → `1000`, and
  the bonus was applied as `1.000`. **A provider was paid one part in a thousand short.**
- Measured over `0.000..2.000`: **12 of 2001** rates lose that unit (`1.001`, `1.003`,
  `1.005`, … and the same series above 1) — every one a rate an operator could name.
- Fixed by making the rate an integer in thousandths (`verification_bonus_per_mille: 1_500`).
  `a_rate_that_binary_cannot_hold_is_not_lost` pins 1000 × 1.001 = 1001, **verified both
  ways**: it fails against the old expression (1000 vs 1001) and passes with the integer.
- **`python3` is the fast oracle for this class**: Python floats are IEEE-754 doubles, exactly
  as Rust's `f64` are, so `int(x * 1000.0)` enumerates the same truncations `as u128` does.

### The classification half (TICKET-094's criteria, applied)
- **`x3-staking-analytics` is analytics, not a balance path.** Evidence, not impression:
  `estimate_reward` is called only inside its own crate; the crate's only dependent is
  `x3-rpc` (a query surface); `annual_backing_reward` / `annual_commission_from_backing` /
  `nominator_net_reward` are consumed by nothing but each other. Recorded so the next pass
  does not re-open it.
- `slash_tracker.rs` reads `Utc::now()`. In a *score* that is a statistic; had it reached a
  decision it would be PHASE 42's problem. Noted, not acted on.

### Process notes
- **A simulation that does not compile produces no test result, and an absence is not a
  verdict.** My first attempt to restore the float behaviour used `f64::from(u64)`, which does
  not exist, so the crate did not compile — and grepping the output for `^test result|left|
  right|panicked` found nothing, which looks exactly like a pass under a different filter.
  Capture the **whole** output when simulating, and check the simulation compiled.
- **Grep the *live* expression, not just the name.** After the fix, `grep 'verification_bonus
  \* 1000.0'` still matched once — in the doc comment explaining the bug. Check whether a
  remaining hit is code or prose before calling it a leak.
- `cargo clippy -p <crate> --lib -- -D warnings` was 50s; the crate's tests were ~35s each for
  its integration suites.

### Next task seed
- **TICKET-094**: 550 sites. Next most amount-shaped: `x3-swap-router/src/atomic_execution.rs`,
  `x3-dex/src/liquidity_mining.rs`, then the `x3-staking-analytics` remainder.
- **TICKET-093**: three crates on the `no_std` gate's known list.
- **TICKET-063 (a)/(d)**, **TICKET-091 (rest)**, TICKET-075, TICKET-058, TICKET-068/069,
  TICKET-074, TICKET-080. PHASE 48 has one unverified row.

## 2026-09-19 — the swap executor reported a success it never executed (`f7007ee4d`)

### The finding
- `crates/x3-swap-router/src/atomic_execution.rs::execute_swap_bundle` returned `Ok` with a
  record assembled from **constants**: `execution_id: H256::zero()` (the same id every call),
  `gas_used` = the gas *limit*, `execution_time_ms: 10`, `success = !hops.is_empty() ||
  amount_in > 0` (always true), and `slippage_achieved` echoing the caller's *declared* bound.
  **No swap was executed**, on `pub` API, in a module named for executing atomic swaps. The
  source said so: "Minimal deterministic execution record; replace with real executor
  integration".
- **Why nothing caught it**: the only caller in the repository is one test, which asserted the
  fabricated success (`assert!(res.success); assert!(res.gas_used > U256::zero());`), and **no
  crate depends on `x3-swap-router`** — verified by grep. A no-op path blessed by the one test
  that touched it.
- Fixed: refused with `SwapRouterError::NoExecutorConfigured`, a variant of its own because a
  missing executor is a **configuration fact** and `ExecutionFailed` is a **market outcome** —
  a caller has to tell them apart. Test renamed to
  `executor_refuses_a_bundle_instead_of_fabricating_a_success`, **verified both ways**.
- The float: `slippage_achieved: f64` (`params.slippage_bps as f64 / 10_000.0`) → `slippage_bps:
  u16`. An execution record carries no float (PHASE 43), and echoing the caller's bound under
  the name "achieved" is a measurement not taken. **Width matches the source field** —
  `SlippageProtectedParams::slippage_bps` is `u16`; my first attempt typed it `u32` and the
  compiler caught it.
- Recorded, not removed: `pub struct SwapBundle;` and `enum ExecutionStatus` — declared,
  re-exported, used nowhere.

### Process notes
- **A `pub` API with one caller that is its own test is the signature of a blessed
  placeholder.** When a module's name says it does X, check that X happens; the test may be
  asserting the fabrication rather than the work.
- **A simulation that does not compile yields no test result — and here the compiler found
  what a grep could not**: my first restore of the fabricated `Ok` failed to compile on a type
  mismatch, which told me the *field type was wrong*, not the simulation.
- **Grep the live expression, not the name**: after the fix `grep f64` still matches once, in
  the doc comment explaining the old field. Check code versus prose before calling it a leak.

### Next task seed
- **TICKET-094**: 549 float sites. Rather than reading crates one at a time, the next useful
  step is to *rank* them — files that name money (`amount`, `balance`, `reward`, `slash`,
  `payout`, `settle`) **and** contain a float site — so the classification has a worklist
  instead of a number.
- **TICKET-093**: three crates on the gate's known list.
- **TICKET-063 (a)/(d)**, **TICKET-091 (rest)**, TICKET-075, TICKET-058, TICKET-068/069,
  TICKET-074, TICKET-080. PHASE 48 has one unverified row.

---

## Round 72 — 2026-09-19 — `crates/x3-dex` AMM arithmetic, and `x3-lang` `NaN`

Base `f7007ee4d`. Landed: `14a450368` (format gate), `5ae419458` (TICKET-094, x3-dex),
`5ccd725c3` (TICKET-095, x3-lang). Working clone `/tmp/x3lang-p29`, toolchain always
`/tmp/x3lang-cargo.sh` (pins 1.90.0). Master moved 3 times during the round; assert
`HEAD~1 == origin/master` before every `git push origin HEAD:master`.

### TICKET-094, third stop — `crates/x3-dex/src/amm_pools.rs`
- **The worklist undercounted the file: 6 lines counted, 9 functions priced.** Reading it end
  to end found `preview_swap` too. A line-counting ranking is a *starting* order, not a scope.
  The fix therefore includes a test that pins the two implementations together
  (`preview_swap_agrees_with_swap`), because two hand-written copies of one formula is how they
  drift and the enumeration is what missed it.
- The arithmetic: `10^24` is not an `f64` (`float(10**24) == 999999999999999983222784`), so a
  pool at that size was quoted against reserves it does not hold. At reserves `10^24`, a `10^18`
  swap, 30bp: exact `996999005991991025`, float `996999005991991040` — **15 units, and upward,
  in a floor operation**.
- The shape of the fix is now the house style for this ticket: **one private helper per formula,
  not the formula inlined nine times.** `mul_div` (256-bit `U256` intermediate so the product
  cannot wrap the `u128` it computes in), `net_of_fee`, `swap_out`, `lp_for_deposit`.
  `add_liquidity_calculate`'s ratio comparison became a cross-multiplication in `U256`, because
  dividing two floats and comparing quotients is the same defect wearing a different hat.
- **A backup file goes stale within the same session.** `/tmp/amm-fixed.rs` was 48 lines behind
  the working copy when this round resumed — the file had been edited again after the copy was
  taken. Diff the worktree against `HEAD` and against the backup; never trust the copy.

### TICKET-095 — an amount with no `float` representation ran to completion as `NaN`
- `numeric.parse_decimal` checked `Decimal.is_finite` and **all nine callers narrow to `float`
  on the next line**, and `float()` returns `inf` rather than raising. `1e400` is finite as a
  `Decimal` and `inf` one line later. Result: `exit 0`, `status: rolled_back`,
  `"expected_profit_usd": NaN`, `"estimated_slippage_usd": Infinity` — a `json.dumps` document
  carrying two tokens RFC 8259 does not define, i.e. **the runner's own output stopped being
  JSON and nothing raised at any point**. Fix at the choke point, not at the nine readers.
- `float(Decimal(...))` does **not** raise `OverflowError`; it returns `inf`. The `isfinite`
  check is the load-bearing one. Verified boundary: `1.7976931348623157e308` accepted, `1e309`
  refused.
- **A refusal message must name the rule that was broken.** `1e400` *is* positive and *is*
  numeric, so `"amount must be positive numeric"` sent its reader to the sign and the spelling.
  The reason now travels: `f'amount must be positive numeric ({exc})'`.
- The null-`requires`/`policies` half is **contract consistency, not a reachable crash**:
  `cli.parse_file` initializes both keys, so no `.x3` file can produce a null. Said so in the
  ticket, the commit and the report rather than dressing it as a live defect.

### Process notes
- **A red gate is a defect and it is cheap to prove**: `cargo fmt --all -- --check` at
  `f7007ee4d` fails in 3 files this same line of work committed. `git status` showing only the
  in-flight file is what proves the other three are committed *as-is* — that is the assertion,
  not the fmt output alone.
- **Duplicate work happened and the safe-failure reading matters**: `openclaw-agent` fixed the
  same three format files independently at 21:48 (`6f91aa696`) as `14a450368`. Byte-identical
  content, two commits. Check `git diff <mine>:path origin/master:path` before calling a rebase
  conflict a conflict.
- **`salvage/*` branches are worth reading**: `salvage/x3lang-intent-bridge` (`fbc095420`) had a
  real, unmerged fix for the same two TICKET-095 defects. Two independent agents reaching the
  same finding raises confidence; the branch itself should not be merged now — its content is
  in, at a better layer.
- **`git archive HEAD x3-lang | tar -x -C /tmp/before`** is the cheap way to get a true "before"
  tree for a before/after measurement without a worktree or a stash.

### Next task seed
- **TICKET-094 re-measured at 513 files** (551 at the ticket's opening). Re-derived worklist =
  **84 lines in 36 files**: `x3-staking-analytics/src/reward_calculator.rs` (10, already
  classified analytics — do not re-open), `chronos-flash/src/predictor.rs` (6),
  `x3-staking-analytics/src/staking_simulator.rs` (6),
  `apps/x3-desktop/…/funding_war_plan_commands.rs` (5),
  `pallets/x3-dex/fuzz/fuzz_targets/fuzz_remove_liquidity.rs` (4).
  **Highest consequence unread: `crates/x3-oracle/src/pyth_oracle.rs`** — Pyth publishes
  mantissa+exponent integers and a price is the input to every money decision.
- The ticket's **acceptance criterion is still unmet**: a *classification* of all 513 files as
  converted-or-legitimate, not a pile of conversions. That artifact is the remaining work.
- PHASE 48 (one unverified row, `dropped transaction`), PHASE 20 (absent by its own text),
  TICKET-075, TICKET-021.

---

## Round 73 — 2026-09-19 — PHASE 48 closed out, PHASE 39's guarantee reaches the artifact

Base `6fb649ec4`. Landed: `0ddbb26ad` (PHASE 48 row 24), `1f976974a` (TICKET-075 / PHASE 39).

### The classification the last round ordered — reachability before line counts
- `cargo tree -i <crate> --workspace` for each of the 18 crates on TICKET-094's 84-line
  worklist: **30 of the 84 lines are in crates nothing depends on** (`quantum-swarm` 9,
  `chronos-flash` 8 — the worklist's #2 and #3 — plus `x3-foundry-core`,
  `cross-chain-position-manager`, `swarm-media`, `voice-to-x3`, `x3-marketplace`), and
  `crates/x3-oracle` is **not a workspace member at all**, so `cargo build --workspace` never
  compiles it and no gate sees it.
- **The entry that looked most consequential was the least.** `pyth_oracle.rs` is a price feed,
  which is why it topped a consequence ranking — but `get_price_decimal` /
  `get_price_change_pct` have no call sites outside the file and `is_price_anomaly` has **none
  at all, including its own tests**. Converting those floats is polish on code the build does not
  compile. → TICKET-096: decide membership or deletion *before* touching it.
- **A ranking is a reading order, not a scope.** A count over money words elects dead code;
  sort by reachability first.
- `x3-cli` reports "no dependents" but is a `bin` — an entrypoint. `cargo tree -i` alone does not
  answer reachability; the target kinds do.

### PHASE 48 row 24, `dropped transaction` — a contract nobody had written down
- A dropped transaction is a venue that **answers**: it accepted and broadcast, and reports the
  state it started from. `TradingVm::check_commitment` compares that against
  `CapabilityManifest::state_commitment`, so the row depends entirely on **which state the
  manifest declares** — and that field had no doc comment.
- **My first reading was backwards, and the 2×2 caught it.** Anchored at the pre-state the
  runtime *commits* the dropped leg and *refuses* the landed one; I nearly filed that as the
  defect. Enumerate the whole matrix (two booleans, four rows, one command) before writing the
  finding.
- **Proving a test is load-bearing is cheap**: neuter the guard (`check_commitment` → `Ok(())`),
  watch the test fail, restore. Do it for any test whose subject is a check.

### TICKET-075 — a declaration that reached nothing
- PHASE 39's `settlement` was enforced at compile time and dropped: `Item::VenueDecl` lowered to
  **no operation**, so the artifact could not say whether a leg was atomic or compensating. The
  skip arm's own comment claimed the attributes "reach the artifact through the plan a route
  produced" — true for fee/slippage/liquidity, false for the guarantee.
- The mechanism to copy is TICKET-059's (finality depth): **a declaration lowers to an operation
  that carries its values.** New inline record at opcode `0x58`,
  `[opcode][u16 len][venue:shape]`, registered in the one `is_payload_opcode` table both the
  disassembler and the VM verifier walk by. `0x58` was free in the `0x5x` declaration block,
  beside `STRATEGY_LICENSE`.
- **A new opcode's real work is the boundary table, not the encoder.** `spec/opcodes.rs` is
  `include!`d by both `compiler/src/lib.rs` and `vm/src/lib.rs`, so it is the one definition they
  share — and an opcode missing from `is_payload_opcode` makes a reader walk payload bytes as
  instructions. Per that file, this failure mode "has surfaced four times in this format".

### Two self-corrections, both found by tests rather than by reading
- **The executor's validation was unreachable.** `VM::execute` is `verify_and_execute`, it is the
  only caller of `execute_unverified`, and nothing else in the crate calls that. A check in the
  executor is a rule no reachable path exercises; the test said so — it expected the executor's
  refusal code and got `X3_VERIFY_FAILED: InvalidOperand(180)`. The rule lives in
  `verifier::verify` now, one enforcement point, and the executor arm only advances the pc.
  **Before validating in the executor, ask which entrypoint reaches it without the verifier.**
- **The VM and the compiler disagreed about a blank venue name**: the verifier used
  `venue.is_empty()`, the compiler uses `trim().is_empty()`, so an all-spaces name passed the VM
  and failed the compiler. When two layers validate one field, diff the predicates.

### Process notes
- **When a test's constant is wrong the implementation is often right.** `1_012_000_000_000_000`
  had an extra zero; the code returned the exact `120` for the inputs actually written. Read the
  `left`/`right` before touching the fix.
- **Do not claim a difference you have not exhibited.** For the arb spread I searched 800k random
  pairs plus a constructed family for an admission-boundary flip, found none, and wrote the test
  to say it is a wrong number and not a wrong decision. The constructed family did find 195
  one-basis-point errors — which is what got claimed.
- **`include!`d files are the one-definition surface here**: `spec/opcodes.rs` (compiler + VM) and
  `x3-lang-common` (every crate, the AST included). Prefer them over a new dependency edge; the
  VM reaches the compiler's `SettlementGuarantee` through the re-export it already has.

### Next task seed
- **TICKET-097 (new)** — the artifact version byte does not move when the opcode set does, so a
  reader predating an opcode **misparses** instead of refusing it. Latent today (no consumer
  outside x3-lang), live the moment a `.x3b` outlives its runtime. Decide the policy, add the gate.
- **TICKET-021** — the remaining trading diagnostics carry no code; severity is the accumulator's
  channel rather than a field.
- **TICKET-094** — the 513-file classification, now with worklist reachability measured.
- **TICKET-096** — `crates/x3-oracle`'s workspace membership.
- Ledger: **55 of 56 phases complete in depth; PHASE 20 absent by its own text.** PHASE 48 is
  complete against its own 30 cases as of `0ddbb26ad`; PHASE 39 is complete as of `1f976974a`.

---

## Round 74 — 2026-09-20 — a decidable `if` is folded and runs (TICKET-058, feature half)

Landed `e8aa4ee90` on base `1f976974a`.

- **The fix did not need a new opcode.** `Condition::True`/`False` already existed, so folding a
  decidable condition and having the emitter write the *taken branch inline* solves the whole
  problem the refusal was working around: the instructions land at the stream's own absolute
  boundaries, so every reader can walk them, and no `IF` record exists to be unwalkable. **Check
  whether the vocabulary already exists before adding to the format.**
- The two halves are now: decidable → folded and emitted; undecidable → still refused by both the
  verifier and `emit_x3ir` with `X3E0501` naming "not decidable at compile time".
- **Measured proof that the right branch ran**, not just that something built: branches of
  different lengths, so the artifact differs —
  `if 1 > 0` (true) → **8** instruction records, `if 1 > 2` (false) → **9**, one apart, matching
  one guard vs two. Counting them the way `x3c explain` lists them (one record per line) is
  reliable; `x3c build`'s "N ops" is a different unit for the same artifact (292 bytes / 73 ops vs
  8 records) — **do not mix the two counts**, and say which one a pinned number is.
- **Fail-closed arithmetic**: the folder uses `checked_*`, so `1 / 0`, `1 % 0`, `u128::MAX * 2`,
  `u128::MAX + 1` and `1 - 2` are refused rather than decided. A branch decided from a wrapped
  number is decided *wrongly* and nothing downstream can tell. Five edges, all tested.
- **An existing test whose premise my change invalidated was updated, not weakened.**
  `cli_refuses_a_branch_no_reader_could_follow_instead_of_writing_one` used `if 1 > 0` — now
  decidable — so its fixture moved to `if steps > 0` and it still tests exactly what it tested
  (refusal reaches `check` *and* `build`, no artifact left). The new test is the other half of the
  rule; the pair is what stops either from passing vacuously. **When a test fails because the
  feature now exists, move the test back onto the case that is still refused — do not delete it.**
- Two blockers found by writing tests, both mine: I iterated an `Option<&[T]>` with `for` (the
  `True` arm was wrapped in `Some`), and a `while` guard turned out to be computed and discarded —
  → **TICKET-098**: `Operation::Loop` has no condition field, so the IR's loop does not say what it
  loops on; latent (Loop is refused everywhere) but `x3c lower` prints it.
- Also observed, not a defect: `swap` is not a statement inside a nested block (only at
  `execute`/`route` level), and the semantic checks do not see guards nested inside a branch. Both
  are separate matters from this change.

### Next task seed
- **TICKET-058 remaining**: an undecidable `if` needs expression codegen into a register, and the
  compiler emits **no arithmetic at all** — that is a register machine plus a code generator, and
  `loop` needs a jump target in stream coordinates on top. A project, not a patch.
- **TICKET-098** — give `Operation::Loop` its condition.
- **TICKET-021**, **TICKET-094**, **TICKET-096**, **TICKET-097** as before.
- Needle state: x3-lang `cargo test --workspace` **1136 passed / 0 failed**; clippy + fmt clean;
  pytest 21; sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 75 — 2026-09-20 — the hedge's delta is a post-condition (TICKET-068 CLOSED)

Landed `cfedbbe43` on base `e8aa4ee90`. First of the three "host adapters" that stand between the
phase list and the features running end to end.

### What made it cheap
- **The format already had the concept.** The measured reply carries an explicit *unit byte*, and
  the comparison-mode field was full — but bits 5-7 of the `REQUIRE` flags byte were free. So
  `MEASURED_UNIT_DELTA_BPS` (reply unit) plus a **unit code in the flag bits** needed no new opcode,
  no new record and no version move. The profit's code is **zero**, which is what every artifact
  written before the field existed carries, so old artifacts keep their meaning.
  **Before inventing a format extension, check the fields that are already there.**
- The pattern to copy is PHASE 37's: a *measured* guard refuses with `X3_GUARD_UNMEASURED` when no
  host reported a number, so the failure is fail-closed by construction.

### Two defects avoided, both found by reading rather than by a test
- **`lowering.rs` holds two `Require` blocks with byte-identical shape** — the liquidation floor and
  the hedge bound. My patch hit the *first* (liquidation) and flipped it to `measured: true`, while
  the hedge stayed `false`. Found by running `x3c lower` and reading `"measured": false` where I
  expected true. **Anchor edits on a field that distinguishes the site** (I used `subject:`), or
  assert the neighbours — this is the same class as the earlier "deleting by range ate a closing
  brace".
- **A delta guard shares the profit's mode**, so the simulation's floor reader read it as
  `profit_floor_bps` — a floor compared against a delta. Now refused with the reason (TICKET-099).
  **When a new value shares an enum variant with an old one, every reader that switches on the
  variant is a site to check.** `grep` for the mode constant found it; nothing else would have.

### Process notes
- Pin the *relationship* a test asserts, not just a number: the branch-folding test asserts 8-vs-9
  records; this one asserts `REQUIRE measured delta 1` in the disassembly and the two refusals with
  their figures.
- The disassembler printed `REQUIRE ? 1` for **any** measured guard before this. A reader that says
  "something was measured" and not what is barely a reader — naming it was two lines.
- `x3c explain` and `x3c build` report different counts for the same artifact (records vs encoded
  instructions). Say which one a pinned number is.

### Where 98 and 100 stand
- **The three adapters**: 068 **closed** here. **069 → TICKET-100**: a liquidation's conversion is
  an `Operation::Swap`, which `execute_asset_opcode` resolves *locally* and never puts to a host, so
  no reply can carry what was seized; three ways to close it are written out with a recommendation
  (make the conversion a `VenueOrder` — the narrowest that keeps one number for one quantity), and
  the CLI's "state both measurements or neither" rule needs relaxing for a program with only a
  profit floor. **070**: the remaining half is a *language* input (a program stating its current
  holdings), not compiler work, and the ticket already says so.
- Needle: x3-lang `cargo test --workspace` **1142 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 76 — 2026-09-20 — TICKET-072 was not a defect (a ticket can be wrong)

Landed `cc6ea6eb2` on base `cfedbbe43`. **A written ticket's characterisation is a hypothesis, not
evidence** — this one survived several rounds as a medium-high "funds semantics" bug and is not a bug.

- The claim: three sites disagree about `Lock.from` — the intent parser stores the *payee*, the
  atomic-swap path stores the payer. The facts: an intent's endpoints each name the account on
  **their own** chain (`typechecker.py` checks `from.receiver` against `from.chain`), so the from
  endpoint's account *is* the payer. Measured on two different addresses: `Lock{from: 0xA1}` (the
  from side's account) and `Release{to: 0xA2}` (the to side's). All three sites agree.
- **The third "site" was a match arm's shape, not a semantics.** `verifier.rs` binds
  `Lock { from }` with `Mint { to: from }` to share one non-emptiness check. Reading a shared binding
  as a semantic claim is how a ticket invents a defect. **When a ticket cites a match arm as
  evidence, read what the arm is *for*.**
- What was worth keeping: the same quantity has two spellings (an address, and the keyword
  `"sender"`), which is now on the payload field's doc instead of being inferable only from the two
  writers. Fixed by documentation plus a test that uses **two different addresses**, because a test
  with one address for both sides cannot tell the sides apart — the "a test whose input cannot
  express the property it names" trap.
- Cost of being wrong in the other direction: had I "fixed" it, I would have moved a host's debit
  from the source-side account to the destination's — the mis-debit the ticket feared, introduced by
  the fix.

### Next task seed
- **TICKET-100** (liquidation's conversion cannot report what it realised) still needs the language
  decision; **TICKET-070** needs a program to state its holdings. Both written out with options.
- Then `TICKET-080`, `TICKET-093`, `TICKET-094`, `TICKET-096`…`TICKET-099`, `TICKET-021`, and the
  three `FIXABLE_NOW`.
- Needle: x3-lang `cargo test --workspace` **1144 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep 19/19/19/18.

---

## Round 77 — 2026-09-20 — a liquidation's floor is a post-condition (TICKET-069 + TICKET-100 CLOSED)

Landed `ce8961990` on base `cc6ea6eb2`. **Second of the three host adapters.** 068 (hedge) closed in
`cfedbbe43`; 070 (rebalance) remains, and it is a language decision rather than compiler work.

- **My own recommendation on the ticket was wrong, and the reason is worth keeping**: option 1
  ("lower the conversion as a `VenueOrder`") would have **dropped the declared `min_output`** — a
  venue order carries an action, a subject, an asset and a quantity, and no minimum. Check what a
  record's *fields* can hold before proposing to route a quantity through it; the plan-shape change
  looked free and would have replaced a constraint with a weaker one while calling it a
  post-condition.
- **The cheap option was right, and my objection to it was wrong.** Option 3 — report the net on the
  reply to the venue orders that already reach the host — needed no new unit, no new vocabulary and
  no format change. My objection ("the net becomes the venue's claim where the plan computed one
  locally") is answered by the hedge: exposure is computed at compile time *and* the delta measured
  at run time. **Two figures for one quantity is the pattern, as long as the one that decides
  settlement is the measured one.** I recorded the correction on the ticket rather than quietly
  taking the cheaper path.
- A `REQUIRE` inside an `atomic_liquidation` block only accepts `net_profit`; a `require slippage`
  for the conversion's ceiling has to come from an `intent` block in the same program
  (`semantic::require_guards` reads program-level guards). Cost an iteration to find.
- **A CLI convenience rule can be load-bearing for a feature**: "state both measurements or neither"
  was right for a plan and made a liquidation unable to state the one quantity it is bounded by.
  Relaxing it did not remove the check it performed — a guard whose quantity is unstated refuses and
  now names the guard and the quantity, which the old message did not. When relaxing a rule, say
  which *property* it was protecting and show that property still holds.
- Process: the CLI's `run` keeps three booleans of measurement; a single-arm `match` left behind by
  the relaxation is a clippy error ("could be written as a `let`"), and clippy caught it before the
  push.

### Next task seed
- **TICKET-070** — the rebalance target has no transaction graph: needs the language to let a program
  state its current holdings (a host then prices them). Two shapes are written out on the ticket.
- Then `TICKET-080` (a `Release` does not name its lock), `TICKET-093`, `TICKET-094`, `TICKET-096`,
  `TICKET-097`, `TICKET-098`, `TICKET-099`, `TICKET-021`, and the three `FIXABLE_NOW`.
- Needle: x3-lang `cargo test --workspace` **1144 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 78 — 2026-09-20 — a rebalance states what it holds (TICKET-070 CLOSED)

Landed `89749ad29` on base `ce8961990`. **All three host adapters are now closed**: 068 (hedge,
`cfedbbe43`), 069/100 (liquidation, `ce8961990`), 070 (rebalance, `89749ad29`).

- The gap was an *input*, not effort: every trade to a target portfolio depends on where it starts,
  and a compiler has no state. The declaration may state it now (`holds { chain.ASSET = <n>; }`) and
  it travels beside the weights, so a host that prices both ends can compute the trades.
- **"Stated none" is a different fact from "holds nothing"**, and the artifact carries both as an
  empty list — so the distinction lives in the program, and the test asserts both readings rather
  than only that a field exists. Same discipline as `settlement none` and the empty-shape field.
- **A new payload field goes at the END.** `RebalanceTarget` gained `holdings` appended, so a record
  written before it exists ends early and is **refused as short** rather than misread — the same
  fail-closed direction as the venue-settlement record, and the reason TICKET-097 (the version byte)
  still matters.
- `x3c fmt` had to learn the clause or reformatting would silently delete the input the trades need
  (TICKET-090's defect, other direction). Any new syntactic clause needs a formatter round-trip test
  *and* an assertion that the formatter does not invent the clause for a program that omitted it.
- The formatter faithfully rewrites `BTC = 40%` as `unknown.BTC = 40%`, because that is how the
  weight line parses (no chain). Pre-existing; worth knowing before writing an assertion that reads
  a chain-qualified name in a record — it is evidence the name came from somewhere *else*.
- Residual stated, not folded in: the compiler still does not generate the graph, because weights are
  value-shares and holdings are amounts — the trades need prices. The host prices; the artifact gives
  it both ends. That is the `arb.rs`/`hyperarb.rs` division of labour and the ticket's own wording.

### Next task seed
- **The register machine + code generator** for dynamic `if`/`loop` (TICKET-058's remainder) is the
  one genuine *project* left: an undecidable condition needs a value in a register and the compiler
  emits no arithmetic at all.
- Then `TICKET-080`, `TICKET-093`, `TICKET-094`, `TICKET-096`, `TICKET-097`, `TICKET-098`,
  `TICKET-099`, `TICKET-021`, and the three `FIXABLE_NOW`.
- Needle: x3-lang `cargo test --workspace` **1150 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

## 2026-09-20 — cross-VM execution is further along than the last estimate said, and now reproducible

The user asked "i thought we were way further along with cross chain execution". The answer, measured
rather than remembered, is **yes for the three VMs X3 actually transacts with, no for everything else**.

### What is real, and proven today (master `ce8961990` + PR #375)

All four cross-domain lifecycles were re-run on this box on 2026-09-20 against real chains. Each one
boots its own chain; none mocks chain state:

| test (`node/tests/`) | boots | result |
| --- | --- | --- |
| `real_x3vm_evm_lock_claim_atomic_lifecycle` | anvil + deployed AtlasHTLC + x3-chain-node | 51.9s PASS |
| `real_x3vm_evm_timeout_refund_atomic_lifecycle` | same | 67.1s PASS |
| `real_x3vm_svm_lock_claim_atomic_lifecycle` | solana-test-validator + SBF program + x3-svm-broadcast | 76.2s PASS |
| `real_x3vm_svm_timeout_refund_atomic_lifecycle` | same | 79.4s PASS |

The on-chain side of that is `pallets/x3-settlement-engine` (3311 lines): `create_intent`, `lock_escrow`,
`submit_proof`, `submit_cross_domain_proof_set`, `claim_settlement`, `refund_settlement`, bonds and
slashing. It is wired into the runtime as `X3SettlementEngine` and the tests drive it through real
signed extrinsics, so this is execution, not description.

### The correction that matters

**`--cross` did not run these four tests.** It listed the two cross-domain lifecycles in `--list` and
ran only the X3-native one; the ignore attributes were accurate ("needs a chain") but nothing supplied
the chain. The pass evidence for the cross-domain leg lived in `.ai/runlogs/*.log`, which is
**gitignored** (`*.log`), plus CI history. So the honest previous statement was "it passed in a past CI
run", which is not the same as "it passes now on your machine".

PR #375 fixes that: `scripts/cross-domain-evm-gate.sh` and `scripts/cross-domain-svm-gate.sh` boot what
the tests need, both are in `GATES_CROSS`, and the `describe_all` heredoc that described them as
unrunnable notes is gone. Merged as `3b64b948b`. Fast gates on that branch: 5 PASS / 0 FAIL.

### What is NOT real (do not quote these as cross-chain execution)

- **13 VM adapters are simulated and say so.** `crates/x3-atomic-swap/src/{near,ton,cairo_vm,plutus,
  move_vm,soroban,fuel,cosmwasm,bitcoin,substrate,polkadot_ink,wasm_l1,zkvm}_htlc.rs` fabricate tx ids
  via `mock_tx_id` and carry `raw_proof: "mock proof"`; `X3VmAdapter::is_simulated()` defaults to `true`
  in `adapter.rs`, and only the live modules (`x3vm_live`, `evm_live`, `btc_live`, `x3vm_native`,
  `x3vm_node`) exist to override it. ~12k lines of interface, not transport.
- **L2 message passing refuses by design.** `crates/external-chains` reads a real chain (block number,
  balances, receipts via `evm_rpc.rs`) but `send_message`, `receive_messages`, `initiate_transfer` and
  `finalize_transfer` on Base/Arbitrum/Polygon/Avalanche return "cannot decode `SentMessage` /
  `L2ToL1Tx` / `StateSync` / `TeleporterMessageReceived` yet; refusing rather than reporting empty".
  Fail-closed and correct; also zero cross-chain message delivery.
- **`X3-contracts/svm/programs/x3_htlc` has no caller.** `FEATURE_REGISTRY.toml` records it at
  readiness 30, "not wired into any live cross-VM route".

### Numbers to reuse

- `crates/x3-atomic-swap` 35,307 lines total; the live/provable slice is ~3.0k of it.
- Register: 19 entries — 7 `LIVE_TESTNET`, 10 `GUARDED_TESTNET`, 2 `SIM_TESTNET`; `x3_htlc` 30,
  `btc_fortress_gateway` 25, `atomic_gateway` 65, `atomic_router` 88.

### Next task seed

1. Same treatment for the two **live contract** gates already in `--live`: they exist, so the sharper
   question is whether `--cross` is in the pre-push hook or only opt-in.
2. `crates/external-chains` log decoding is the single biggest missing cross-chain *transport*; each
   refusal names the exact event it cannot decode, so the work is enumerable, not vague.
3. The 13 simulated adapters: either a real transport or move them behind an explicit
   "interface-only, never routed" register mode, so the registry stops implying fourteen chains.

## 2026-09-20 (later) — the mainnet genesis path was a placeholder; now it builds, boots and is gated

Objective is still "mainnet ready". Three PRs landed this session; master after them is `a45d7debf`.

### The finding that mattered

**The mainnet genesis had never been produced, and could not be.** `production_config()` in
`node/src/chain_spec.rs` is env-gated and refuse-closed (needs `X3_PRODUCTION_AUTHORITIES`,
endowed/council/treasury accounts, non-zero EVM+SVM escrow addresses, and refuses dev seeds). Three
things around it were broken at once:

1. `x3-chain-node keys generate|insert|list|verify` were **placeholders** — they printed "In a full
   implementation, this would use sp_core crypto / For now, show a placeholder" and exited **0**. That
   is the only key tooling the node ships, and `X3_PRODUCTION_AUTHORITIES` is built from its output, so
   the operator got an apology instead of a key.
2. `scripts/mainnet/generate_mainnet_chain_spec.sh` called `build-spec --chain production` with **no
   env** (only-fail) and redirected straight to the output path, leaving two empty
   `x3-mainnet-{plain,raw}.json` files behind.
3. The startup banner went to **stdout** via `println!`, and `build-spec` writes the spec to stdout —
   so every generated spec began `{\n\n<ESC>[1;35m` and `json.load` refused it.

And the release gate could not have caught any of it: stage 3 only parsed the local3 specs and grepped
`chain_spec.rs` for the string `production_config`.

### What landed

- **#376** real `keys` CLI (sr25519 aura/imonline, ed25519 grandpa; `--seed` or generated-and-printed
  secret; `verify` exits non-zero on mismatch; `insert`/`list` on a real `sc_keystore::LocalKeystore`
  at `<base-path>/chains/<chain-id>/keystore`), 5 unit tests pinning `//Alice` -> `5GrwvaEF…` /
  `5FA9nQDV…`; fixed generator (refuses up front, names missing vars, refuses no-bootnodes, builds to
  temp files, asserts Live/x3_chain_production, prints sha256); banner moved to stderr; new
  `scripts/mainnet/production_genesis_gate.sh`; release-gate **stage 3b**.
- **#377** `mainnet_release_gate.py::check_build()` **never built anything** — it printed "✓ binary
  found" if `target/release/x3-chain-node` existed and skipped the build, so stages 2b/2c/3/3b ran
  against whatever binary was on disk. It now always builds and prints size+sha256.
- **#375** (earlier in the session) `--cross` really runs the X3VM<->EVM and X3VM<->SVM lifecycles.

### Evidence to reuse

- Production genesis gate: 3 fixture authorities -> Live spec (17.2 MB) -> three validators boot on it,
  validator 1's `system_localPeerId` **equals** the peer id in the spec's bootNodes, all three agree at
  a finalized height (observed 29, 102, 124 across runs).
- **Full release gate PASS** (first run with stage 3b and a self-built binary): chain runs 0->60
  finalized 55; 3 validators agree at height 34; production genesis agrees at 29; six runtime/pallet
  suites pass; six runtime variants migrate; srtool compact `0x69f795b7…` / compressed `0x3140bf85…`
  match the record; no secrets.
- `cargo test -p x3-chain-node` = 93 passed / 0 failed (8 ignored integration targets);
  `cargo clippy -p x3-chain-node --all-targets --features mainnet-rc1 -- -D warnings` clean.

### Traps worth remembering

- **Rebuild before trusting a CLI test.** The stale release binary made the new stage look broken; the
  real defect was the gate never compiling. Print digests in gate output.
- `cargo +toolchain` fails when the toolchain *bin dir* is first on PATH; use `env PATH="$HOME/.cargo/bin:$PATH"`.
- The x3-gov worktree at `/tmp/x3-gov` carries a warm target at `/tmp/x3-gov-target`
  (`WASM_BUILD_WORKSPACE_HINT=/tmp/x3-gov`). It still has **uncommitted** local3 chain-spec
  regeneration (`chain-specs/*.json`) and `scripts/local-node-smoke.sh` / `scripts/local-ci.sh` edits
  from an earlier pass; do not switch branches there without dealing with them.
- Node boot on the 17 MB plain production spec takes ~40 s before RPC answers; budget timeouts >= 180 s.

### Next task seed

1. **Operator install path is broken**: `scripts/install-validator.sh` downloads
   `releases/download/<tag>/x3-chain-node` + `x3-mainnet-raw.json`, but the repo's only release
   (`v0.4.0-rc.1`) is a **draft** and has no assets, and the script continues without checksum
   verification when the `.sha256` is missing. The canonical runbook (`launch-gates/VALIDATOR_ONBOARDING_RUNBOOK.md`)
   tells operators to `cargo build --release` instead. Make the installer accept a locally built
   binary + a generated spec, verify hashes strictly, and fail loudly.
2. Stage 3b runs the genesis gate but nothing runs `genesis_lint.sh` against a *real* generated spec
   (`chain-specs/x3-mainnet-*.json` are still absent because they need real keys).
3. `crates/external-chains` L2 message decoding; the 13 simulated adapters; `x3_htlc` unwired.

### 2026-09-20 addendum — #378: the inspect CLI

- `x3-chain-node inspect assets` printed a **hardcoded** registry ("0: X3 / 1: ETH / 2: SOL /
  3: USDC") under a heading that reads like chain state. It survived because the dev genesis happens
  to register exactly those four ids. It now queries `x3_getAssetMetadata` over ids 0..=31 and prints
  the range it covered (the runtime has no enumerate-all API; adding one needs a runtime-hash
  re-attestation, so that is a follow-up ticket, not a silent bound).
- Every `inspect` arm printed `--- … Failed ---` and then `return Ok(())`: a dead node or a missing
  method exited **0**. Now non-zero. A `null` from the chain is data and still exits 0.
- Verified on a live dev node: assets 0..3 queried, account balances for 1/2/3 printed, `//Alice`
  (outside the documented "SS58 or hex" contract) exits 1, dead endpoint exits 1.
- Master after this session's four merges: `b3f79bdf3`. `node/src/command.rs` on master has **no**
  placeholder markers left (the one remaining "In a full implementation" string is the comment
  explaining what was removed).

## 2026-09-20 (third pass) — the release bar is green end to end; master `205adcf24`

Ten PRs landed this session: **#375–#386**. The two that matter most:

### #386 — no panic is reachable from a block hook

Two `on_finalize` sites converted a constant with
`b"bond_expiry".to_vec().try_into().expect("bond_expiry fits in bounded reason")`
(`pallets/x3-slash/src/lib.rs:576`, `pallets/pallet-x3-agent-registry/src/lib.rs:608`). They cannot
fail today, but a panic in `on_finalize` stops block production for every validator. Replaced with
`BoundedVec::truncate_from(...)` (infallible). The ratchet's `runtime_hooks_allowed` is now **empty**.

Runtime re-attested (two srtool builds of `5775b1587` agree):

    compact    8,424,386 bytes  setCode 0x733d6faa…  BLAKE2 0x47acc821…
    compressed 1,442,745 bytes  setCode 0x9d7e04a4…  BLAKE2 0x380527fa…

**Full release gate PASS, all 13 stages** — 3c shipped-genesis boot (h8), 3b production (h30),
3d testnet (h29), 4b panic ratchet (0/0/2269), 6b srtool rebuild matches the new record.

### Trap: `update-runtime-hashes.sh`'s replacement list is incomplete

It prints old→new pairs for `runtime-wasm-reproducibility.md`, but not the prose sentence "current
values are the `0x…` pair", and the byte counts it quoted in that doc were stale by a few hundred
bytes independently of the hash change. After a re-attestation, grep the doc for every old value
(including truncated `0x…` forms) rather than trusting the list.

### The pipeline that now exists (all merged)

    3   chain-spec artifacts (+ format assertion: plain needs genesis.runtimeGenesis, raw needs genesis.raw)
    3c  the SHIPPED genesis boots 3 validators (local-network-smoke.sh + X3_NETWORK_SMOKE_CHAIN_SPEC)
    3b  production genesis builds/boots/finalizes  (scripts/mainnet/production_genesis_gate.sh)
    3d  testnet    genesis builds/boots/finalizes  (same script, X3_GENESIS_CHAIN=testnet)
    2d  validator install path (9 cases: accepts 3, refuses 6, writes nothing in --check)
    4b  panic ratchet (scripts/audit/panic_unwrap_scan.py + docs/reports/panic-unwrap-baseline.json)

Fixture generation for 2d/3b/3d: `scripts/mainnet/make-fixture-live-spec.sh <out> [port] [chain]`
writes `fixture.json` {spec, seeds, peers, bootnodes, authorities}; the boot gate uses the seeds as
`X3_DEV_SEED` per validator and asserts `system_localPeerId` equals the spec's bootnode peer id.

### What was verified about the shipped chain specs (evidence, not assumption)

- committed `x3-local3-plain.json` (before #383): **node exits** — `unknown variant 'runtime'`
- committed `x3-local3-current-raw.json`: loads, 3 validators finalize (h19)
- regenerated plain/raw (#383): finalize h11 / h47
- `deployment/chain-specs/fresh/x3-testnet-plain.json` (plain, Live): **loads**
- `deployment/chain-specs/x3-testnet-raw.json` (raw, Live): **rejected** — `Live chain spec requires
  at least one Aura authority`. Raw is for distribution; the node runs the PLAIN file.
- A solo node on a 3-authority spec authors but never finalizes. That is 1/3 < 2/3 GRANDPA, not a
  broken spec — testing a multi-authority genesis with one node is a false negative.

### Remaining, in priority order

1. **External L2 message delivery** (`crates/external-chains`): reads real chains, `send_message` /
   `receive_messages` / `finalize_transfer` still refuse by design. Biggest functional gap.
2. **Adversarial settlement suite**: double-claim, replay of `submit_cross_domain_proof_set`,
   claim-after-refund, forged proof set — the cross-domain path is proven happy-path only.
3. **Published release artifacts** (binary + sha256 + spec + SBOM) and a pinned Docker image; the
   installer supports the source-build route but `--from-release` refuses today.
4. **Public testnet infrastructure**: real authority keys, bootnode multiaddrs (committed raw testnet
   spec has `bootNodes: []`), RPC/indexer/faucet/explorer (scripts exist, unverified), monitoring,
   and a multi-validator soak (7-validator script never run here).
5. 2,269 production `unwrap()/expect()` sites outside block hooks, now ratcheted so they cannot grow.
6. External security audit — the one item no gate can substitute for.

---

## Round 79 — 2026-09-20 — a claim names its lock, so a book settles as one unit (TICKET-080 CLOSED)

Landed `2e341876e` on base `89749ad29` (rebased past a CI merge to `3b64b948b`).

- The blocker was a **pair of overlapping identities**: `semantic::release_lock` returned
  `(chain, asset)`, so a release named its claim by *asset*, and two transfers of one asset in a
  route were the same claim to every reader — which is why netting settled one route per transfer.
  A book nets *within* one asset, so the impossible case was the normal one.
- The fix is an **index**, not a field on the lock: `Release.claims: u32` = the position of its lock
  among its route's locks. A position is derivable from the route, so it cannot disagree with the
  locks it indexes — cheaper than a claim id on both variants and strictly sounder. **Prefer a
  derivable reference to a second stored one.**
- **Two rules share the identity, and only one needed widening.** `no_double_claim` counts claims
  per lock → needs the index. `no_refund_after_claim` and `escrows_claimed_in_their_own_route`
  reason about *escrows* (a refund is keyed by asset) → stay asset-level. The reader split into
  `release_lock` (escrow) and `claimed_lock` (index) rather than one helper with two meanings. The
  compiler found this: widening one function broke a comparison whose types had been telling the
  truth.
- **A check I wrote, tested, and reverted**: a range check on the claim index. It failed four
  cross-chain parallel-plan tests because this IR uses `Release` for *claims* **and** for *payouts*,
  and a payout claims no lock. Shipping it needed a distinction the IR does not carry. **Recorded in
  the code and as TICKET-101 rather than shipped**, and the reverted attempt's failure message is
  quoted there so the next reader does not re-derive it.
- **A mechanical edit script must know syntax from semantics.** My brace-matching inserter added
  `claims: 0,` to every `Release {` — including `match` patterns — then a "cleanup" pass removed 23
  of them and took two *correct* fields with it (they sat under a comment, which my heuristic read
  as a pattern context). Fixed by hand from the compiler's E0063 list. **Let the compiler enumerate
  the sites; a regex over braces cannot tell an initializer from a pattern.**
- Master moved mid-turn again (a CI merge to `3b64b948b`). Rebase was clean — the changed files
  (`docs/local-ci.md`, three `scripts/*`) did not overlap mine — and the battery was re-run on the
  rebased commit before pushing.

### Next task seed
- **TICKET-101** (new) — `Release` means two things and the IR does not say which; the range check
  and a cleaner `no_refund_after_claim` both wait on it.
- Then `TICKET-093`, `TICKET-094`, `TICKET-096`…`TICKET-099`, `TICKET-021`, `TICKET-046`,
  `TICKET-060`, `TICKET-062`, `TICKET-074`, the three `FIXABLE_NOW`, and the register-codegen
  project (`TICKET-058`'s remainder).
- Needle: x3-lang `cargo test --workspace` **1153 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 80 — 2026-09-20 — a release says which of its two acts it is (TICKET-101 CLOSED)

Landed `108785c62` on base `2e341876e`.

- **The reverted check was the evidence.** The range check abandoned in TICKET-080 failed because a
  payout was read as a claim on lock #0 — and that failure is what justified making the distinction
  explicit rather than guessing. **A reverted attempt is a finding; record it and come back.**
- **An explicit absence beats a sentinel**, again: `claims: Option<u32>` with a **tag byte** in the
  payload (`0` = claims nothing, `1` + index = a claim), and a tag the encoder never writes is its own
  `UnknownTag` error rather than a fall-through to the default. Same shape as `settlement none` and
  the empty-shape field.
- **Saying which act each site performs turned out to be the documentation.** Six sites, three acts:
  netting claims; an atomic swap's destination release, an intent's `to` endpoint and a bare
  `release` are payouts; a timeout refund's release is *neither* (it returns an escrow). Writing that
  table into the code is what makes the rules readable.
- **The distinction deleted code**: `locked_escrows` existed only to tell a payout from a claim, and
  with claims explicit it had no callers — removed rather than left as dead code, along with its
  careful doc comment whose reasoning now lives in the rule's own comment.
- **A heuristic classified three fixtures and got all three wrong**; the tests' *names* said which act
  each described (`..._pays_out_and_refunds_different_assets` is a payout; `invariant_no_double_claim_
  detects_violation` is the same lock claimed twice). **When a mechanical pass has to make a semantic
  choice, read the test names before the diffs.**
- `verify_ir` returns `Result<(), Vec<CompilerDiagnostic>>`; the *invariants* are a separate entry
  (`get_builtin_invariants()` → `rule.check_fn`). A hand-built IR that violates an invariant passes
  structural verification — the two layers are different questions.

### Next task seed
- **TICKET-001** (`FIXABLE_NOW`) — the `Release`/refund representation in intent lowering is the same
  family and is now cheaper to do properly.
- Then `TICKET-093`, `TICKET-094`, `TICKET-096`…`TICKET-099`, `TICKET-021`, `TICKET-046`,
  `TICKET-060`, `TICKET-062`, `TICKET-074`, the other two `FIXABLE_NOW`, and the register-codegen
  project (`TICKET-058`'s remainder).
- Needle: x3-lang `cargo test --workspace` **1156 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 81 — 2026-09-20 — `Release` says which of its three acts it is (TICKET-001 CLOSED)

Landed `706f0d02e` on base `108785c62`. The third and last act: TICKET-101 made claim-vs-payout
explicit, TICKET-001 added **refund**, which was still spelled as a payout even though a timeout
refund's concrete release is a different act from a destination endpoint's.

- **One definition, re-exported**: `ReleaseAct` lives in `x3-common` beside the payload that encodes
  it and is re-exported through the IR (`pub use x3_lang_common::capability::ReleaseAct;`), the way
  `SettlementGuarantee` and `ChoiceCriterion` are. The payload writes a tag byte per act and
  `UnknownTag` covers the rest. The IR is serde-serialized (`x3c lower`), so the enum needed the
  derives and `x3-common` needed `serde`.
- **Measured on a real example**: `examples/timeout_refund.x3` now shows `act: Payout` for the
  destination endpoint and `act: Refund` for what `ON_TIMEOUT` performs — previously identical
  records. That is the ambiguity the ticket named, visible in the artifact.
- **A rule that must count one of two records**: `refund_lock` matches the *handler*, not the
  concrete release, because a refund is one act described twice. Counting both would double-count
  every refund. It says so at the helper.
- **Mechanical cascades need their imports tidied**: my sweep added `ReleaseAct` imports to files
  that already had it from `crate::ir`, producing `E0252` duplicates and `ReleaseAct, ReleaseAct,`.
  Nine files needed a cleanup pass. **When a script adds imports across a tree, dedupe in the same
  pass** — the compiler's error list is the checklist, but only if you read it twice.
- A file whose imports are function-local (`vm/tests/integration.rs`) takes a file-level `use`
  gracefully; the script's top-level regex found nothing to anchor on and silently did nothing until
  the compiler complained.

### Next task seed
- Remaining narrow work: `TICKET-013`/`014` (`FIXABLE_NOW`, corpus), `TICKET-021` (diagnostic codes),
  `TICKET-046` (clause words as a lexical class), `TICKET-060` (Python harness reads 2 of 23),
  `TICKET-062`, `TICKET-074`, `TICKET-091~`, `TICKET-093`, `TICKET-094`, `TICKET-096`…`099`.
- The one project: register machine + code generator for dynamic `if`/`loop` (`TICKET-058`'s
  remainder).
- Needle: x3-lang `cargo test --workspace` **1157 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 82 — 2026-09-20 — every `.x3` the tooling walks is a program (TICKET-013, TICKET-014 CLOSED)

Landed `bedf5efe9` on base `706f0d02e`.

- **The gate is the deliverable.** `examples/*.x3` was gated and the harness reads *named* fixtures,
  so a `.x3` file anywhere else could be unparseable indefinitely. The new gate walks the whole tree
  (29 files) and requires `x3c check`; four directory names are skipped, each with a README saying why.
  **A corpus without a gate rots silently** — the two files the ticket named were joined by three more
  the moment anything looked.
- **TICKET-013 was not a feature.** The `contract` dialect is Solidity-shaped, is not one of the 56
  phases, and the spec says twice that the mission is not to build a Solidity-like language. Five
  legacy files were quarantined and their subjects documented; the ticket's FIXABLE_NOW reading came
  from a roadmap snippet rather than the spec. **Read the spec's own sentences before implementing a
  construct a ticket calls "intended".**
- **Two of TICKET-014's four files were stale when it was written** — both already check, and one's
  "not yet supported" self-label was already gone. Ticket lists age; verify each item.
- The three fixtures the gate found were broken in two independent ways: **syntax** (`sender` where
  the grammar says `receiver`; a two-clause timeout/on_fail where the language has one clause) and
  **rules** (a bridge with no nonce or proof obligations; a `require route_score` with no
  `risk_policy.min_route_score`; a `require finality.<chain>` with no `finality_policy`). Translating
  them meant satisfying rules added after they were written — which is why "fix the parse error" is
  never the whole job.
- A deliberately-invalid fixture belongs in a directory that says so, not beside the valid ones with
  a filename heuristic deciding. `fixtures/invalid/` now carries the README that says it.

### Next task seed
- Remaining narrow work: `TICKET-021` (diagnostic codes), `TICKET-046` (clause words as a lexical
  class), `TICKET-060` (Python harness reads 2 of 23), `TICKET-062`, `TICKET-074`, `TICKET-091~`,
  `TICKET-093`, `TICKET-094`, `TICKET-096`…`099`.
- The one project: register machine + code generator for dynamic `if`/`loop` (`TICKET-058`'s remainder).
- Needle: x3-lang `cargo test --workspace` **1158 passed / 0 failed**; clippy + fmt clean; pytest 21;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 83 — 2026-09-20 — the Python surface's boundary is a contract now (TICKET-060 CLOSED)

Landed `de690bd29` on base `bedf5efe9`.

- **Re-measure a ticket's numbers before acting on them.** The ticket says "2 of 23"; the tree has
  19 examples and the surface reads **3** of them, with **zero crashes** — the five `IndexError`s it
  names were fixed by TICKET-091 some rounds ago. The premise had aged in both directions.
- **A boundary stated in prose is not a contract.** `cli.py` already had a "## Scope, stated so it
  does not have to be inferred" section listing three boundaries; what was missing was the per-file
  assertion, so a change either way — a file that *starts* being read, or one that stops — was
  invisible until someone measured by hand. The table pins each file to `reads` or to its refusal
  code, and the code is part of the entry because a refusal for a different reason is a different
  boundary.
- **Prove each guard by mutating what it guards**: I moved one file between the two tables (fails
  naming the file and both expectations) and dropped a throwaway `.x3` into `examples/` (fails asking
  for its classification). Both mutations are quoted in the commit message. A gate whose failure mode
  has not been seen is a gate nobody knows works.
- **Closing on a third option is allowed if you say so.** The ticket offered grow-or-retire; the
  surface is a front-end whose scope was already a deliberate decision, so the honest closure is
  "stated and pinned", and the ticket says exactly that — including that the one genuinely-drifted
  boundary (address shape, `X3_PARSE_RECEIVER`) belongs to **TICKET-091**, which is open.

### Next task seed
- **TICKET-091** is now the pointed-at item: this surface refuses files the compiler accepts because
  its address shape is stricter. Its fix is in `cli.py`/`registry.py`.
- Then `TICKET-021` (diagnostic codes), `TICKET-046` (clause words), `TICKET-062`, `TICKET-074`,
  `TICKET-093`, `TICKET-094`, `TICKET-096`…`099`.
- The one project: register machine + code generator for dynamic `if`/`loop` (`TICKET-058`'s remainder).
- Needle: x3-lang `cargo test --workspace` **1158 passed / 0 failed**; pytest **22**; clippy + fmt
  clean; sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 84 — 2026-09-20 — the Python surface reads every example the compiler accepts (TICKET-091 CLOSED)

Landed `bea1bd07f` on base `01ae00325` (rebased past a node/mainnet merge; no overlap).

- **"Drift" was three separate things**, and the ticket named two of them: guard kinds, address
  shape, and a `fallback` route step. Measured before/after: **3 read → 9 read**, 16 refused → 10,
  and every remaining refusal is the *stated scope* (`NO_INTENT`: a file whose subject is a compiler
  program). No crashes either way.
- **A list maintained twice drifts.** The surface held the guard vocabulary in `registry.py` *and* in
  nine hand-written branches in `cli.py`, and the docstring named only the first as "the list". The
  branches are one rule now. **When a docstring says "the list is X", check that the refusal comes
  from X.**
- **A front-end should carry what it does not model.** `rust_intent_envelope` passes `requires` to the
  compiler verbatim, so refusing an unknown guard kind was the surface claiming authority over a
  vocabulary it does not own. The generic rule ("a comparison is `<kind> <op> <value>`") is also what
  stops the next kind from needing a branch.
- **A cross-language list needs a cross-language test.** `REQUIRE_KIND_NAMES` in Rust and
  `REQUIRE_KINDS` in Python cannot be kept equal by care, so the Python test reads the Rust array and
  compares — allowing exactly one documented alias, named so it cannot grow into a habit. Proven by
  dropping one name: `only there: ['route_score']`, and the accept/refuse table failed with it.
- The address rule: the compiler validates **no** address shape, so requiring forty hex was pure
  drift. The relaxation keeps the check's purpose (a string with no `0x` and no hex is still refused)
  — **narrow the rule to what the language requires, not to what looks tidy.**

### Next task seed
- Remaining: `TICKET-021` (diagnostic codes), `TICKET-046` (clause words as a lexical class),
  `TICKET-062`, `TICKET-074`, `TICKET-093`, `TICKET-094`, `TICKET-096`…`099`.
- The one project: register machine + code generator for dynamic `if`/`loop` (`TICKET-058`'s remainder).
- Needle: x3-lang `cargo test --workspace` **1158 passed / 0 failed**; pytest **23**; clippy + fmt
  clean; sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 85 — 2026-09-20 — a valueless guard stops at every clause (TICKET-046 PARTIAL)

Landed `4bed37b14` on base `bea1bd07f`. The ticket's **validation** is landed; its acceptance is not,
and the reason is a conflict rather than effort.

- **Where the words become identifiers** — one line:
  `TokenKind::Keyword(kw) => keyword_to_tok(kw).unwrap_or_else(|| Tok::Ident(kw.as_str()…))`. The
  lexer already knows `timeout` as `Keyword::Timeout`; the *parser* flattens it back, which is why
  `CLAUSE_WORDS` has to list it. **A fallback in a conversion is where a typed vocabulary turns back
  into strings** — worth looking for whenever a list duplicates a grammar.
- **The acceptance cannot be done as written**: `expect_ident` accepts only `Tok::Ident` and reads the
  field name after `.`; `debt.amount` is in two shipped examples; so making `amount` a keyword token
  breaks a program the compiler accepts. It needs `expect_ident` to accept keywords — the opposite of
  the acceptance's premise. Measured scope: 22 words, 36 arms, 42 conversion arms, 79 `Tok` variants.
  **Measure a refactor's blast radius before promising it**, and put the conflict in the ticket.
- The validation is now mechanical rather than hand-written: fixtures are *lists of clause lines* and
  the guard is placed before/after each, so coverage is `16 placements` rather than two anecdotes.
  Removing `amount` from the list fails it with the clause named AND fails the older test — two
  guards for the mistake the first pass made once.

### Next task seed
- **TICKET-046** needs its acceptance amended (a reserved-word decision) before the lexer half.
- Remaining narrow work: `TICKET-021`, `TICKET-062`, `TICKET-074`, `TICKET-093`, `TICKET-094`,
  `TICKET-096`…`099`. The one project: register machine + code generator (`TICKET-058`'s remainder).
- Needle: x3-lang `cargo test --workspace` **1161 passed / 0 failed**; pytest 23; clippy + fmt clean;
  sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

---

## Round 86 — 2026-09-20 — the known-unbuildable list is empty (TICKET-093 CLOSED)

Landed `7b54cadb8` on base `a45d7debf` (rebased past a mainnet release-gate merge).

- The gate `scripts/check-no-default-features.sh` derives every crate declaring a `no_std` posture
  and checks it alone. **Its known list is empty now**: 87/87 `ok`, exit 0. Before: 7, 184 and 216
  errors across three crates.
- **`sp-std` is not `alloc`.** At `default-features = false` it *stubs* `Vec` so the type name
  resolves while `String`/`format!`/`ToString` simply do not exist — which is why a crate can look
  half-no_std and fail on exactly those three. And `sp-std` **14 has no `alloc` feature at all**: I
  added `alloc = ["sp-std/alloc"]` and resolution refused it. `extern crate alloc` is the answer, and
  it is part of the distribution rather than a dependency to declare.
- **Removing a `cfg_attr(not(feature = "std"), …)` attribute is a no-op for every build that uses
  default features** — so "stops declaring a posture it does not have" is provably safe here, and the
  gate's own derivation (its list comes from that attribute) makes the false entry disappear with it.
  Two crates (184 and 216 errors of unresolved `std`) took that route rather than a rewrite.
- **Prove a pre-existing failure is pre-existing**: `git stash push -- <file>` and re-run the one
  test. Cheaper and stronger than arguing it cannot be related. `x3-sdk`'s WebSocket test fails
  identically without my change — it needs a socket the sandbox denies → **TICKET-102**, because
  `cargo test --workspace` is a command the repository's own rules require as proof and it cannot be
  green offline.
- A slice of a shell array by `index(')\n')` matched a `)` **inside a comment** and left the list
  half-emptied; `bash -n` caught it. Assert the *shape* of the region you are replacing, not a
  delimiter that occurs in prose.

### Next task seed
- **TICKET-102** — make `x3-sdk`'s WebSocket test loopback-based so `cargo test --workspace` can be
  green offline; it is a prerequisite for the repair loop the repo's rules assume.
- Then `TICKET-021`, `TICKET-062`, `TICKET-074`, `TICKET-094`, `TICKET-096`…`099`, and the one
  project: register machine + code generator (`TICKET-058`'s remainder).
- Needle: no-default-features gate 87/87 no known entries; root `cargo check --workspace` 0 errors;
  x3-lang `cargo test --workspace` 1161/0; pytest 23; sweep 19/19/19/18.

---

## Round 87 — 2026-09-20 — the WebSocket test runs on loopback (TICKET-102 CLOSED, TICKET-103 filed)

Landed `aa4ce2add` on base `7b54cadb8`.

- The defect: a unit test did `WsRpcClient::connect("ws://localhost:9944").await.unwrap()` — a node
  nobody had started — so it failed wherever a socket was unavailable. It binds `127.0.0.1:0`,
  accepts one connection and completes the handshake itself now. **44→45 passed, 0 failed**, proven
  load-bearing by mutating the constructor to refuse loopback.
- **The sandbox denies `bind("127.0.0.1", 0)` but escalation permits it.** Worth knowing: a
  loopback-based test is writable and *verifiable* here, just not from inside the sandbox. Check that
  before concluding a network-shaped test cannot be fixed.
- **Correct the attribution when the premise was wrong.** I filed 102 saying the *root*
  `cargo test --workspace` could not be green because of that line; the root run actually aborts
  earlier, at `e2e_tests`, and this suite is a different one. The crate-level defect was real and is
  fixed; the root blocker is now **TICKET-103**, stated with its two causes (a hardcoded default
  target dir, so `CARGO_TARGET_DIR` can never satisfy it; and a workspace member that needs a built
  node and a running chain, which `cargo test --workspace` runs but `local-ci.sh` does not).
- **`cargo test` stops at the first failing test binary**, so a workspace run's "total passed" is a
  count of the suites *before* the failure — 171 here — not of the workspace. A total from an aborted
  run must not be quoted as if it were the whole.

### Next task seed
- **TICKET-103** — make the e2e suite skip with a named reason and honour `CARGO_TARGET_DIR`, or make
  the two test commands agree about what "the workspace is green" means.
- Then `TICKET-021`, `TICKET-062`, `TICKET-074`, `TICKET-094`, `TICKET-096`…`099`, and the one
  project: register machine + code generator (`TICKET-058`'s remainder).
- Needle: no-default-features gate 87/87 no known entries; `cargo check --workspace` 0 errors;
  `cargo test -p x3-sdk` 45/0; x3-lang `cargo test --workspace` 1161/0; pytest 23; sweep 19/19/19/18.

---

## Round 88 — 2026-09-20 — the live-chain suites are asked for rather than assumed (TICKET-103 CLOSED)

Landed `a7df85f4c` on base `98d89b688` (rebased past a mainnet validator-install merge).

- **`required-features = []` gates nothing.** Three e2e targets carried it — one (`mainnet_rc1`)
  legitimately, two (`cross_vm_real_chain_test`, `live_internal_mainnet_e2e`) as a gate that was
  never a gate. Reading a manifest entry is not enough: ask whether the *set* is empty.
- **One panic reported, two suites.** Fixing the named suite moved the abort one target later and
  found the second. **After fixing a workspace-wide failure, re-run the whole command** — the next
  blocker is usually one step further on, and the fix is cheap while you are in the file.
- **`CARGO_TARGET_DIR` is part of a build's contract.** A test that searches only
  `<root>/target/…` cannot be satisfied by any build the CI actually does. Splitting
  `node_candidates(...)` from the environment read is what made the search order *testable* — the
  old preference test could only pass on a machine that already had a node, which is the "test whose
  input cannot express the property" trap in a new costume.
- **A "MANDATORY, does NOT skip" suite is right to refuse and wrong to be ungated.** The live suite
  keeps its refusal (it panics if run without a node) and is gated so the workspace command does not
  run it. The two are not in conflict: mandatory *within* a run, asked-for *between* runs.
- Verified after rebasing, as always. The approval reviewer returned no assessment payload once for a
  combined commit+push; splitting it into a commit and then a push went through — the rejection was
  the reviewer failing, not a risk finding.

### Next task seed
- The workspace run is at **3168 passed / 2 failed**, and both failures are the embedded-WASM
  prerequisite (`SKIP_WASM_BUILD` unset is required; the WASM build needs a network fetch of
  polkadot-sdk). Not verifiable here; `local-ci.sh`'s `test node` gate is the command of record.
- Then `TICKET-021`, `TICKET-062`, `TICKET-074`, `TICKET-094`, `TICKET-096`…`099`, and the one
  project: register machine + code generator (`TICKET-058`'s remainder).
- Needle: no-default-features gate 87/87; `cargo test -p e2e_tests` 33/0; `cargo test -p x3-sdk` 45/0;
  x3-lang 1161/0; pytest 23; sweep 19/19/19/18.

---

## Round 89 — 2026-09-20 — the trading path's diagnostics carry codes (TICKET-021 progress)

Landed `b2da8b14c` on base `a7df85f4c`. `trading_verify.rs`: 18 bare diagnostics → 18 coded, and the
uncoded helper **deleted** so the next site cannot choose it.

- **Two of the ticket's four class names exist and two do not.** It asks for `sequence`, `debt`,
  `venue`, `quote`; the module has `sequence` and `debt`, and the other two have no site —
  "venue" resolves to a risk-policy capability here and quote freshness is a VM check. **A code
  nothing emits is a catalogue entry with no diagnostic behind it**, so they were not added, and the
  ticket now says why rather than leaving the gap as a to-do.
- **Delete the old helper, do not sit it beside the new one.** `semantic_error` produced exactly the
  defect; leaving it in scope is an invitation.
- Classifying 18 sites by *line* with a script and asserting each line really held the call is what
  kept the pass honest — and the script exited *before writing* when two sites were unaccounted for
  (one was inside a doc comment, one a real site I had missed). **Abort before write when the
  enumeration is incomplete**, or a partial classification ships.
- The mutation that proves the new assertions: make the helper drop the code again → all four
  assertions fail with the bare message. Cheapest possible proof for "the code renders".
- Remaining, measured: `trading_semantic.rs` (~20 sites) and `strategy.rs` (~23) have the same
  uncoded-helper shape, and severity is still two vectors (`errors`/`warnings`) rather than a field.
  The bridge exists — `Diagnostic::from(X3Error)` — so the migration is incremental, one module at a
  time, merged at `ast_level_errors`.

### Next task seed
- Finish TICKET-021: the two remaining modules, then the severity field on `PreEmission`.
- Then `TICKET-062`, `TICKET-074`, `TICKET-094`, `TICKET-096`…`099`, and the one project: register
  machine + code generator (`TICKET-058`'s remainder).
- Needle: x3-lang `cargo test --workspace` **1161/0**; `test_trading_verifier` 29/0; pytest 23;
  clippy + fmt clean; sweep 19/19/19/18.

---

## Round 90 — 2026-09-20 — the last two trading modules are coded (TICKET-021)

Landed `4d1a22df0` on base `4873c5dcf`. `trading_semantic.rs` (22 sites) + `strategy.rs` (22) — the
two the previous round measured and left. Only the severity clause remains on the ticket.

- **The classes from the first module fitted the other two without new names.** That is the test of
  whether a class was named well: `RiskPolicyBound` absorbed the licence-share and slippage ceilings,
  `TradeDeclaration` the module-completeness checks, and the two debt/asset sites landed where the
  first module had already put them. **If a second call site needs a fifth name, the first four were
  probably wrong.**
- **Clippy is the delete-the-old-helper detector.** After converting the call sites, the old `fn err`
  in `strategy.rs` was unused and `-D warnings` said so — my scripted helper replacement had silently
  missed (its anchor did not match the file's `Span::DUMMY` spelling). A grep for call sites would not
  have found that; the linter did.
- **A string replacement that appends after `(` leaves trailing whitespace**, and rustfmt refuses to
  format a file containing it ("left behind trailing whitespace") rather than fixing it. Four lines in
  `trading_verify.rs` from the earlier round. Strip trailing whitespace on every line a script rewrote.
- Two more assertions (one per module), each proven load-bearing by mutating its helper to drop the
  code. `cargo test` stops at the first failing binary, so mutating *both* helpers showed only the
  first failure — mutate one at a time when the proof is per-module.

### Next task seed
- TICKET-021's last clause: severity as a field rather than the accumulator's channel
  (`ast_level_errors -> Vec<X3Error>`, `PreEmission { errors, warnings }`). The bridge is
  `Diagnostic::from(X3Error)`.
- Then `TICKET-062`, `TICKET-074`, `TICKET-094`, `TICKET-096`…`099`, and the one project: register
  machine + code generator (`TICKET-058`'s remainder).
- Needle: x3-lang **1161/0**; clippy + fmt clean; pytest 23; sweep 19/19/19/18.

---

## Round 91 — 2026-09-20 — three more modules coded, and a lossy conversion found (TICKET-021, TICKET-104)

Landed `7a8fdc261` on base `4d1a22df0`. `hedge.rs` (3 sites, `RiskPolicyBound`), `liquidation.rs` (1),
`rebalance.rs` (1), helpers deleted.

- **A helper's arity is a fact about its module, not a style.** `trading_verify.rs`'s takes a `Span`;
  these three carry `Span::DUMMY` because their diagnostics come from a declaration's own numbers.
  Reusing the other module's helper broke five calls in one compile — the compiler enumerated them
  immediately, which is the cheap way to learn it.
- **`CompilerDiagnostic` has six fields and `into_error` keeps two** (code, message; span from
  `primary_span`). Severity and secondary spans are dropped because `X3Error` cannot carry them.
  Faithful today — every trading-path diagnostic comes from `CompilerDiagnostic::error` — and a trap
  tomorrow: **there is no `CompilerDiagnostic::warning`**, so nothing currently suffers. Recorded as
  **TICKET-104** with both fixes offered (a `Diagnostic`-preserving conversion, or a refusal).
  **A lossy conversion with no current caller is not a defect yet; it is a trap, and it is worth a
  ticket precisely because nothing exercises it.**

### Next task seed
- TICKET-021's remainder, measured: `objective.rs` (~10 sites), `semantic.rs`'s `fn err` (~17
  AST-level verifiers), then severity-as-a-field in `VerifyOutcome`/`PreEmission`.
- **TICKET-104** — make a coded warning possible and stop the conversion dropping severity/spans.
- Then `TICKET-062`, `TICKET-074`, `TICKET-094`, `TICKET-096`…`099`, and the one project: register
  machine + code generator (`TICKET-058`'s remainder).
- Needle: x3-lang **1161/0**; clippy + fmt clean; pytest 23; sweep 19/19/19/18.

---

## Round 92 — 2026-09-20 — a loop says what it tests; the version byte becomes a function of the opcode set

Two tickets closed and pushed: `dd07b0bc5` (TICKET-098), `d7b6cba9e` (TICKET-097). Base for the
round was `7d70b128b`; `origin/master` is `d7b6cba9e`. **A first `git fetch` at the start of the
round showed `origin/master == 7a8fdc261`, i.e. everything from rounds 72–91 was already pushed.**

- **A `while` now carries its condition, and the class the compiler can decide is decided.**
  `Operation::Loop { max_iterations, condition, body }`; `lowering` folds the guard with the same
  `fold_condition` an `if` uses. `while 1 > 2 { … }` was *refused* before and now builds, with an
  artifact byte-identical to the same program without the loop (108 bytes, 27 ops, `cmp` IDENTICAL).
  `while true` is refused, not dropped: a folder that read every decision as a licence to delete a
  loop would silently delete a program's body.
- **`expression_to_string` rendered a binary operator with `{:?}`** — `while steps < 10` reached the
  IR and every diagnostic as `steps Lt 10`, a guard the program never wrote in a spelling nothing
  can parse back. `BinOp`/`UnOp` already implement `Display` with the language's symbols. If you add
  a diagnostic that must *name* a condition, use `Condition::describe()` (new, one renderer).
- **The version byte is now a function of the opcode set, gated by the compiler.**
  `spec/opcodes.rs` holds `OPCODE_SET: &[(u8, u8)]` (86 entries) and
  `const _: () = assert!(CURRENT_BYTECODE_VERSION == max_opcode_version(), …)`: registering an
  opcode at a version the writer does not write is `error[E0080]`. `is_defined_version` (framing)
  vs `is_supported_version` (compatibility) are asked in that order; both `has_compiler_header`
  copies use the first, so a version-2 artifact reaches the verifier to be refused instead of being
  walked as raw bytecode. The VM's range-based `valid_opcode` is **deleted**.
  Adding any opcode now means: register it in `OPCODE_SET` with a version, and if that version is 2,
  move `CURRENT_BYTECODE_VERSION` *and* `SUPPORTED_BYTECODE_VERSIONS` — otherwise the build fails.
  The source-walk gate `compiler/tests/test_bytecode_version_gate.rs` reads `spec/opcodes.rs` itself
  and exempts five non-opcode `u8` constants by name, with a test that fails on a stale exemption.
- **TICKET-058's remainder is scoped, not open**: the branch *mechanics* are sound (every
  instruction starts at a multiple of four; `IF` skips four-byte units, so a skip is
  `body_bytes / 4`), and what is missing is a value in a register — there is no immediate-load
  instruction and no arithmetic codegen. The only nameable runtime quantities are the measured ones
  (profit/slippage/delta bps), which the VM compares itself. Recorded as **TICKET-106**: a
  measured-branch instruction at version 2, `if profit >= X { A } else { B }` as two records with
  complementary modes (there is no `JMP`, so an else-skip cannot be emitted another way). The spec
  asks for **bounded** branches (PHASE 12 `atomic_choice`), which are implemented.
- Also recorded: **TICKET-105** (global-max vs per-artifact version — decide with the first v2
  opcode), **TICKET-107** (`base_gas_cost` says `0x0A` is "no instruction" while the executor
  executes it as `POW_RRR`), **TICKET-108** (`test_control_flow_e2e.rs` has a tautological
  assertion).
- **The 124 unmerged remote branches are not a merge backlog.** Measured by content (does the
  branch's version of each file it touches differ from `master`'s?): `archive/pr126-pre-master-rewrite-20260909`
  (66 non-patch-equivalent commits), `pr132-work` and `agents/pasted-text-processing` (82 of 86
  files), dependabot bumps and CI experiments — pre-master-rewrite snapshots. The ones whose content
  mattered were landed at the choke point already (TICKET-095's salvage example).
- **Harness note:** three agents spawned with `fork_turns: "none"` replied as if idle ("no task has
  come through yet") and did no work. A `followup_task` re-sending the task as an imperative
  restarted them. If a spawned agent answers a *question*, treat it as not started and re-send.

### Next task seed
- Merge the three worktree commits when their agents report (`w2`: TICKET-021 remainder +
  TICKET-104; `w3`: TICKET-099; `w4`: TICKET-074) — each rebases onto a moved `master`, and
  `crates/x3-tools/src/bin/x3c.rs` is shared between `w3` and `w4` (disjoint regions, so a
  file-level merge is the risk to check first).
- Then **TICKET-106** (the measured branch, which needs the version-2 decision of TICKET-105),
  TICKET-062, TICKET-094 (513 float sites unclassified), TICKET-096, plus 105/107/108.
- Needle: x3-lang `cargo test --workspace` **1172/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 93 — 2026-09-20 — nothing is dropped: statements, tests, a hedge's delta, a diagnostic's severity

Four more tickets landed and pushed: `4f322939d` (TICKET-109 + 108), `7e23e3738` (TICKET-099),
`97d7af4f7` (TICKET-104). Base was `d7b6cba9e`; `origin/master` is `97d7af4f7`. Master moved twice
mid-round (other agents: chain-specs/genesis, testnet spec builder) — always `git fetch` → rebase →
re-verify → push.

- **`lower_statement`'s catch-all (`_ => push(Nop)`) dropped statements invisibly.** `NOP` is four
  zero bytes: the compiler's walker skips it as padding and the VM's verifier breaks on it as
  end-of-stream. `tests/test_arithmetic.x3` reported `3 ops, no semantic errors` and its artifact was
  byte-identical to an empty program's. The match is now **exhaustive with no catch-all** (rustc
  enumerates missing arms), and `let`/`return`/`break`/`continue`/`for` each refuse by name. A bare
  `loop { }` is *not* refused in lowering: it lowers as `Loop { condition: True }` and takes
  `while true`'s refusal. **Corpus lesson**: three files passed the gate only because everything they
  were made of was dropped; they are in `tests/sketches/` now. If you add a statement variant, the
  compiler will tell you where it goes.
- **`PLAN.md` claimed `x3-lang/compiler/src/regalloc.rs` exists. It does not** (the linear-scan
  allocator is `crates/x3-opt/src/regalloc.rs`, another crate). Corrected, with the run of the
  environment: PLAN.md's ✅ items are not evidence.
- **A hedge's delta is a snapshot field now** (TICKET-099): `ArtifactFloors.delta_ceiling_bps` from
  the **unit code** (not the mode — the mode is the profit's), `SimulationSnapshot.delta_bps`
  (`#[serde(default)]`; `SNAPSHOT_VERSION` deliberately does not move, and the one-way compatibility
  is the fail-closed direction), `Verdict::DeltaAboveCeiling` folding a *non-failing* verdict into a
  failure — including `NoFloorStated`, which is a hedge's normal shape and the combination where
  leaving it alone would be a silent pass. The CLI passes the delta to the VM (it passed `None`).
- **TICKET-104: `VerifyOutcome.errors`/`.warnings` are BOTH `Vec<X3Error>`** — the ticket claimed the
  accumulator "already accepts" `Diagnostic`; it does not. The faithful conversion
  (`From<CompilerDiagnostic> for x3_common::Diagnostic`) is for richer consumers;
  `VerifyOutcome::push_diagnostic` picks the vector from the diagnostic's own severity field.
  `into_error` and the new `into_warning` each assert the severity they are for. **No production pass
  emits a warning yet** — the trap is closed, the first user is not invented (candidate: TICKET-110).
- **Delegation still does not work in this environment**: three agents spawned + re-tasked produced
  no work (two replied as if idle; the third read the canonical checkout instead of its worktree and
  reported it stale). Do not plan around sub-agents here — take the tickets yourself.

### Next task seed
- **TICKET-106** (measured branch at version 2) is the one real feature gap; TICKET-105 is the
  decision to take with it. TICKET-110 is the natural first user of the new warning.
- **TICKET-074** needs a *design* first: the route (`Opportunity`) carries no price, so a
  venue-price attestation needs a stated scaling before it can be honest.
- Then TICKET-021's remainder (`objective.rs`, `semantic.rs`'s `fn err`), TICKET-062 (lockfile
  `--locked` gate), TICKET-096 (`x3-oracle`), TICKET-094 (513 float sites).
- Needle: x3-lang `cargo test --workspace` **1188/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 94 — 2026-09-20 — a runtime branch on a measured quantity; the version byte per artifact

**TICKET-106 + TICKET-105 closed and pushed as `bb5737775`** (base `97d7af4f7`). This was TICKET-058's
remainder and the last construct the compiler refused by design. Master is `bb5737775`.

- **`IF_MEASURED` (`0x34`)** is a payload record `<unit>:<invert>:<threshold_bps>:<skip>`. A branch is
  **two records with the bodies between them** (this format has no unconditional jump: `CALL`/`RET`
  push and pop). A unit code names the quantity (0 profit, 1 delta, and 2 — newly named — for
  slippage, which had a mode of its own and needed no code). The **skip is written before the body**:
  every instruction starts on a four-byte boundary, so a body's byte length is the same at any aligned
  offset — no patching. `measured_comparison` in `vm/src/executor.rs` is the *one* evaluator, shared
  with `REQUIRE`: failure = refusal in a guard, fork in a branch; **unmeasured = refusal in both**.
- **The caller's figure must be seeded into `VMState`, not only the bridge.** `report_measurement` /
  `report_outcome` only seeded the bridge, so a stated measurement was invisible until the first call
  that *answers* with one (a venue order). That is what made the branch unreachable from a source
  program — check reachability before believing a feature is done.
- **The version byte is per artifact (TICKET-105)**: `CURRENT_BYTECODE_VERSION` is the writer's
  *ceiling* (compile-time gate intact), and `emit_x3ir` narrows the byte to the greatest version among
  the opcodes the artifact contains, read back with the reader's own framing. A program with no v2
  opcode writes `0x01`; one with `IF_MEASURED` writes `0x02`.
- **`is_reserved_version_byte` claims `0x01..=0x0F`.** Without it, a *future* version-3 artifact would
  have been walked as raw bytecode ("nothing defines version 3") — the exact misparse TICKET-097
  exists to forbid. If you add a version, add it to `SUPPORTED_BYTECODE_VERSIONS` and nothing else.
- **Comparing a version byte to `CURRENT_BYTECODE_VERSION` is now wrong** unless you mean "the ceiling":
  `decode_trading_program` did, and refused every trading artifact this build had just compiled (two
  tests caught it). Use `is_supported_version`.
- A measured branch chooses between *statements*, not between *plans*; `atomic_choice` is still the
  bounded form PHASE 12 asks for.

### Next task seed
- **TICKET-110** — `GasAdaptive`'s `Nop` bodies, and the natural first user of TICKET-104's warning.
- **TICKET-074** — the packet proof-requirement vocabulary needs its evidence contract designed first
  (the route carries no price, so a venue-price check needs a stated scaling).
- Then TICKET-107, TICKET-021's remainder, TICKET-062, TICKET-096, TICKET-094 (513 float sites).
- Needle: x3-lang `cargo test --workspace` **1197/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 94b — 2026-09-20 — `@gas_adaptive` stops writing a record it cannot honour

**TICKET-110 closed and pushed as `e05b42cfe`** (base `8fe6ae19f`; master moved again mid-round —
another agent is landing chain-spec/testnet work). Master is `e05b42cfe`.

- The annotation takes **no arguments**, and the artifact's record demands two **non-empty** bodies
  (`verify_ir`: "gas-adaptive branches must not be empty") — so the arm satisfied the rule with
  `vec![Nop]` on each side: a record claiming two paths, neither of which was one. It lowers to nothing
  now (no `GAS_ADAPTIVE` record), like the ten other modifiers the artifact has no form for. The opcode
  and the VM capability stay; a source surface for it needs syntax the annotation does not have.
- **The assertion the ticket asked for is a source scan** of `lowering.rs` in
  `test_refused_statements.rs`: no `Operation::Nop` and no `Operation::GasAdaptive {` construction
  survives there. A source scan because a `Nop` construction is legal Rust that compiles — its failure
  is a program that checks clean and runs as if a statement had not been written.
- **TICKET-111 records the wider question** rather than settling it for one annotation: eleven
  annotations lower to `{}` and nothing says so; one decision (report them via TICKET-104's warning, or
  state the silence as policy in one place) applies to all eleven.
- If you need a *reachable* first user for `CompilerDiagnostic::warning` / `push_diagnostic`, this is
  the candidate the ledger now names.

### Next task seed
- **TICKET-111** (the annotation policy, one decision + a test that enumerates the list), **TICKET-107**
  (`0x0A`'s cost-table comment), **TICKET-074** (packet evidence contract — design first), TICKET-021's
  remainder, TICKET-062, TICKET-096, TICKET-094.
- Needle: x3-lang `cargo test --workspace` **1198/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 94c — 2026-09-20 — a modifier with no artifact form is stated as policy or reported

**TICKET-111 closed and pushed as `5c41c6d7e`** (base `e05b42cfe`). Master is `5c41c6d7e`.

- **`compiler/src/annotations.rs` is the enumeration**: `DISPOSITIONS` has a row per annotation spelling
  with `Carried` / `Invisible(reason)` / `Reported(reason)`, and `warnings(program)` raises one warning
  per `Reported` row. Six silent annotations are *policy* (`no_heap`, `no_recursion`, `on_chain`,
  `off_chain`, `concurrent`, `payable`); `@gas_adaptive` is *reported* because it claims two gas paths
  the artifact would have to state and the annotation names no bodies.
- **TICKET-104's warning path has its first production user**: `CompilerDiagnostic::warning` filed via
  `VerifyOutcome::push_diagnostic`; `DeclarationHasNoArtifactForm` / `X3E4026` is the catalogue's first
  warning-severity code.
- **Two more silent drops were found by the mechanical test, not by reading**: `@upgrade_from` alone is
  dropped (`VersionMeta` is pushed only when a version was stated too — its row is `Reported` now, and
  the pair is asserted to carry both), and `@subscription` **cannot be written at all** (the lexer
  reserves the word for the `subscription <name>: <amount>, <period> { … }` item, so `expect_ident`
  refuses it and that arm in `annotation_from_name_args` was unreachable — 20 of 21 spellings parse).
  The dead arm is deleted; the parser now says what the word is for.
- **`--deny-warnings` now fails** for a program carrying `@gas_adaptive` or a lonely `@upgrade_from`.
  Nothing in the corpus uses either.
- If you add an annotation: it needs a row in `DISPOSITIONS`, or `test_annotation_policy` fails — and the
  third fact that test holds is that the lowerer's IR is **non-empty exactly** for the `Carried` rows,
  which is the check that catches a record with placeholder bodies.

### Next task seed
- **TICKET-107** (`0x0A`'s cost-table comment vs the executor), **TICKET-074** (packet evidence contract
  — design first), TICKET-021's remainder, TICKET-062, TICKET-096, TICKET-094 (513 float sites).
- Consider: the `subscription` **item** form is implemented and unreachable-from-docs; if it matters,
  an example would make it a tested surface.
- Needle: x3-lang `cargo test --workspace` **1205/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 94d — 2026-09-20 — the catalogue, the executor and the cost table agree

**TICKET-107 closed and pushed as `339779ff3`** (base `5c41c6d7e`). Master is `339779ff3`.

- **`POW` = `0x0A`** is named now: the executor has had an arm for it all along while the cost table
  called it "no instruction in this catalogue" *and charged 50 gas for it*. Constant, `OPCODE_SET`,
  `opcode_name`, and the executor's arms written `POW =>` rather than by value.
- **`0x70`'s gas row is deleted**: no instruction has ever had that value, and the comment admitting
  the row was kept "only because this is a move of the table" no longer applies. The invariant that
  buys: **every code the cost table prices is an opcode `OPCODE_SET` defines** (52 codes).
- **`x3c explain` printed `UNKNOWN` for `FEATURE_ALLOW` (`0x56`)** — emitted for every
  `allow <feature>` statement, three times in `examples/intent_fusion.x3`. The new gate found it, and
  `NOP`/`ADD`/`SUB` are named too. If you add an opcode: a constant, a `OPCODE_SET` row, and a
  `opcode_name` arm, or `test_bytecode_version_gate` fails.
- **A python rewrite truncated `test_bytecode_version_gate.rs`** (everything after the insertion point
  deleted). The *workspace total* dropped by three and a per-binary comparison named the file. Restore
  pattern: `git show HEAD:<path> > /tmp/x`, then append only the intended block. **Compare per-binary
  test counts before and after any scripted rewrite of a test file** — the total is the detector.

### Next task seed
- TICKET-074 (packet evidence contract — design first), TICKET-021's remainder (`objective.rs`,
  `semantic.rs`'s `fn err`), TICKET-062 (lockfile `--locked` gate), TICKET-096 (`x3-oracle`),
  TICKET-094 (513 float sites).
- Needle: x3-lang `cargo test --workspace` **1206/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 94e — 2026-09-20 — every semantic diagnostic states its class

**TICKET-021 closed and pushed as `551e32b23`** (base `339779ff3`). Master is `551e32b23`.

- **`semantic.rs`'s `fn err` and `objective.rs`'s now require a `DiagnosticCode`** (115 + 10 sites).
  The class is a parameter, not a helper per class: the class is a fact about each *site*, and a helper
  choosing one code for the file would be the uncoded helper under a new name. **The signature is the
  gate** — a new site cannot skip a class.
- **Two new codes, both with sites**: `X3E4027 GuardClaimUnbacked` (30 sites — the recurring "a guard
  whose claim no declaration backs" shape) and `X3E4028 MainnetConfigurationUnsafe` (13 — the `mainnet:`
  gates; the same program is fine on a testnet, so it is not `UnsafeIr`). Existing: `X3E4025
  TradeDeclaration` (41), `X3E0501 UnsafeIr` (24 for IR-level structural invariants), `X3E4026
  DeclarationHasNoArtifactForm` (1), `X3E0401 InvalidCrossChainRoute`/`X3E0101 UndefinedSymbol` for
  `validate_atomic_swap` and `check_safe_symbol`.
- **If you code a diagnostic: read the site, then choose.** The script mapping functions to classes
  worked for 106 sites; 9 needed per-site codes (`validate_atomic_swap` mixes unknown-chain,
  same-chain and unknown-hash; `check_safe_symbol` mixes an empty name with two safety limits). The
  script **aborted before writing twice** — once on a multi-line `format!(` whose message is not on the
  call line, once on a needle that ran onto the next line. Put the *verification* of each site's text
  before the write, not after.
- **Two tooling catches**: clippy caught a doc-comment **quote marker** (a doc line beginning `>= N`
  reads as a Markdown blockquote — reflow it), and rustfmt **refused to format** the file the insertion
  left trailing whitespace in. Strip trailing whitespace as part of the script.
- Assertions added one per class, on fixtures the test files already had; the guard one is proven
  load-bearing by mutation.

### Next task seed
- TICKET-074 (packet evidence contract — design first), TICKET-062 (lockfile `--locked` gate),
  TICKET-096 (`x3-oracle`), TICKET-094 (513 float sites), TICKET-046 (needs a user decision).
- Needle: x3-lang `cargo test --workspace` **1207/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 94f — 2026-09-20 — a packet's proof requirements are checked against what a host states

**TICKET-074 closed and pushed as `faf3115e8`** (base `551e32b23`). Master is `faf3115e8`.

- **The vocabulary is closed**: `state_root_freshness`, `venue_price_attestation`, `strategy_commitment`
  (`vm/src/opportunity_packet.rs::requirements`). `PacketEvidence` is the JSON a host states;
  `verify_packet_with_evidence` checks each declared requirement **against the packet's own terms** and
  returns `PacketVerification { checked }`. `verify_packet` stays the *form* check (admission has no
  evidence); the vocabulary is enforced in `validate_proof_requirements`, so every entry point refuses an
  unknown name.
- **The venue-price check uses the route's own figures** — per-venue liquidity vs `route.min_liquidity`,
  attested fees summed vs `route.fee_bps` — because a per-leg price would need per-leg amounts the
  packet does not carry, and a scaling chosen here would compare two different things (TICKET-068's
  defect one instruction over).
- **`--evidence <file>` is required when a packet declares requirements**: without it the CLI names them
  and refuses, rather than reporting "verified" for the packet's form. On success it prints
  `requirements checked: …`.
- **Two fixture traps, both caught by the fixtures themselves**: a packet's `proof_requirements` are part
  of the terms its `execution_commitment` covers, so editing them into an already-signed JSON refuses on
  `ExecutionCommitmentMismatch` — parameterize the fixture and sign *after* setting them; and a blind
  `replace("[]", …)` rewrites the route's arrays first.
- If you add a requirement: add it to `requirements::ALL` **and** a check in
  `verify_packet_with_evidence`, or the vocabulary check in `validate_proof_requirements` will refuse a
  name with no behaviour behind it (which is the direction to fail in).

### Next task seed
- TICKET-062 (lockfile `--locked` gate), TICKET-096 (`x3-oracle`), TICKET-094 (513 float sites),
  TICKET-046 (needs a user decision).
- Needle: x3-lang `cargo test --workspace` **1213/0**; clippy + fmt clean; sweep 19/19/19/18.

---

## Round 94g — 2026-09-20 — the lockfile gate, and how to prove a CI gate is wired

**TICKET-062 closed and pushed as `febd2382f`** (base `205adcf24`). Master is `febd2382f`.

- `scripts/local-ci.sh`'s `GATES_FAST` gains `"cargo lockfile locked:cargo metadata --locked
  --format-version 1"`. `cargo metadata --locked` runs offline here (3s) and exits 101 when a manifest
  needs a lock entry the lock lacks.
- **Prove a gate through the script, not by hand**: `scripts/local-ci.sh --only <slug>` where the slug
  is the name lowercased with spaces → dashes (`cargo lockfile locked` → `cargo-lockfile-locked`). The
  passing run and the failing run (with one manifest edited and restored) are both in the ledger entry.
- **A gate command must not name a local path.** `cargo`, not `/tmp/x3lang-cargo.sh` — the repo's CI
  provides the toolchain, and a shim path passes only where the shim exists.

### Next task seed
- TICKET-096 (`x3-oracle`: workspace member with a consumer, or delete), TICKET-094 (513 float sites),
  TICKET-046 (needs a user decision).
- Needle: x3-lang `cargo test --workspace` **1213/0**; clippy + fmt clean; sweep 19/19/19/18;
  `scripts/local-ci.sh --only cargo-lockfile-locked` PASS.

---

## Round 94h — 2026-09-20 — the invariant rules are errors, and the promotion found two false positives

**TICKET-002b closed and pushed as `8386a4ec5`** (base `febd2382f`). Master is `8386a4ec5`.

- **All six built-in invariant rules are errors now** (`verify_invariants_structured`), which is what the
  ticket's acceptance asked for. Promoting them found **two false positives the warnings had hidden**:
  1. `no_double_refund` counted one refund per lock across both exit paths — a `timeout … refund` is a
     *declaration* for the timeout/refund engine (the emitter writes `[ON_TIMEOUT][0]` and keeps
     `duration_blocks` in the IR) and an `on_fail refund` is the failure path's instruction, so the
     canonical bridged `parallel` leg is not one refund twice. Counted per `(lock, exit path)` now.
  2. `no_claim_after_refund` read any `Release` as a claim; `Release` is a claim **or** a payout
     (TICKET-101's `ReleaseAct`), and its sibling rule already read `release_lock`. Fixed the same way.
- **A warning that cannot be promoted is a rule that protects nothing** — that is the lesson worth
  keeping: both defects were invisible for as long as the rules were warnings, and one of them had been
  "fixed" in the sibling rule years-of-commits ago without the same fix being applied here.
- **When a rule's test disagrees with a real program, one of them is wrong — check which.** Two existing
  tests encoded the old (broader) rules; both were updated with *both* halves asserted (the violation
  reported, the legitimate pair not), so the narrowing is visible rather than looking like a relaxation.
  One in-module fixture had been asserting a rule on `ReleaseAct::Payout`, a shape the rule is not about.
- New `compiler/tests/test_invariant_rules.rs` is the shape to reuse for any rule set: a matrix asserting
  **each rule fails only for its own case**, a test that the case set names every rule exactly once, and
  a well-formed case that trips none.

### Next task seed
- TICKET-046 (needs a user decision), TICKET-094 (513 float sites), TICKET-096 (`x3-oracle`: needs a
  decision), and the deferred ones (016 needs host evidence, 018/022 are design decisions, 006/008/009
  are repo/process and two of them need user approval).
- Needle: x3-lang `cargo test --workspace` **1216/0**; clippy + fmt clean; sweep 19/19/19/18; corpus
  25 files, 0 failing.

---

## Round 94i — 2026-09-20 — exact bitcoin amounts, and a float gate on the consensus surface

**TICKET-094 progress pushed as `64899f004`** (base `8386a4ec5`). Master is `64899f004`.

- **The ticket's own count misled me twice**: "551 files with float arithmetic" is a *file list* from a
  wide pattern, and most of it is **vendored third-party source** (`apps/x3-desktop/src-tauri/tauri-vendor/`).
  The dangerous shape — a float cast to an integer — is **31 sites in 18 files** across
  `crates pallets runtime services`, excluding tests/benches/vendored. Classify by *shape*, not by the
  pattern that mentions `f64`.
- **The consensus surface is clean** (`pallets/*/src`, `runtime/src`: zero floats, zero float→int), and
  `scripts/check-no-float-in-consensus.py` now holds it — `// float-exemption: <reason>` is the marker for
  a line that is not a decision (the convention `x3-lang/vm/src/bridge.rs` already uses for Solana's
  wire field). Test-support files (`src/tests.rs`, `src/mock.rs`, `fuzz/`, `benches/`) are out of scope
  **by name**, because a pallet keeps its test module under `src/`.
- **A float→int cast in a bridge adapter is money**: `(0.29_f64 * 1e8) as u64` = 28_999_999 against the
  exact 29_000_000, and `unwrap_or(0.0)` on an amount means a UTXO worth nothing. Read the reply's own
  *text* (`btc_to_satoshi`), refuse anything finer than a satoshi, and refuse a missing amount naming the
  field.
- **My own test caught a bug in the fix**: `serde_json::json!(0.00000001)` renders as `1e-8`, so a
  parser that refused exponent forms would have refused *one satoshi*. JSON numbers may be written
  either way.
- **Money-shaped sites still to convert** (named in the ledger's 094 table): the marketplace 80/20 fee
  split, the rollback listener's `refund * 1.05`, the CLI's `amount * 0.995`, and the foundry crates'
  six-way revenue split (which is in a crate **nothing depends on** — TICKET-096's question).

### Next task seed
- The three conversions above, then the ~10 unclassified files (x3-evolution, x3-opt, orchestra,
  x3-sidecar, x3-integration, …) — read each before a verdict; the ticket's artifact is the
  classification.
- TICKET-096 (member with no consumer: x3-oracle, and now x3-foundry-core/revenue), TICKET-046 (decision).
- Needle: `scripts/local-ci.sh --only no-float-in-consensus,cargo-lockfile-locked` PASS;
  `cargo test -p x3-bridge-adapters` 29/0; x3-lang workspace unchanged at 1216/0.

---

## Round 94j — 2026-09-20 — the marketplace fee split is basis points (TICKET-094)

**Pushed as `5b139733c`** (base `64899f004`). Master is `5b139733c`.

- **A float share of a token amount loses money, and a small probe cannot see it.** `80% of 10^18 + 7`
  through `f64` is **5 short**; of `12345678901234567890` **168 short**. My first probe over totals
  1..=100_000 reported **zero** mismatches — small totals are exact in f64 — so I nearly recorded "no
  defect". **Always probe at the magnitudes the code is for** (token/wei amounts, 1e18..1e24), and treat
  a clean probe as evidence only when its input could have failed.
- `FeeSplit` is `{publisher_bps: u32, marketplace_bps: u32}` now, matching `x3-accounting-events`'s
  `FeeSplits` ("all entries sum to 10 000") — one convention, not a third. `validate` requires exactly
  10 000 with both non-zero, the share is `total * bps / 10_000` in `u128`, and the marketplace takes the
  remainder so the two always sum to the amount paid.
- **A `f64` percentage field in a money path is the shape to look for**, not the arithmetic on it: the
  foundry crates hold six of them (`x3-foundry-core/revenue.rs`) and lose up to 5 units per distribution
  for 95,000 of the first 100,000 totals — but that crate has **no dependents**, so it is recorded, not
  converted (TICKET-096's question).
- A doc-comment code fence with no language is compiled as Rust by rustdoc: the `text` marker is
  required for a block that is an example rather than a test.

### Next task seed
- Remaining 094 conversions: `x3-atomic-trade/src/rollback_listener.rs` (`refund * 1.05`),
  `x3-cli/src/commands/swap.rs` (`amount * 0.995`, maybe a display estimate), then the ~10 unclassified
  files (x3-evolution, x3-opt, orchestra, x3-sidecar, x3-integration, x3-accounting-events …).
- Then the foundry crates' reachability (TICKET-096's class), TICKET-046 (needs a decision).
- Needle: `cargo test -p x3-marketplace` 40/0; `-p x3-bridge-adapters` 29/0;
  `scripts/local-ci.sh --only no-float-in-consensus,cargo-lockfile-locked` PASS; x3-lang 1216/0.

---

## Round 94k — 2026-09-20 — four money paths out of `f64` (TICKET-094)

Four conversions pushed this turn, each with a measured before/after: `64899f004` (bitcoin adapter),
`5b139733c` (marketplace), `a02aa07f7` (atomic-trade), `93d762b5c` (CLI). Master is `93d762b5c`.

| file | measured defect | fix |
|---|---|---|
| `x3-bridge-adapters/bitcoin.rs` | `(0.29_f64 * 1e8) as u64` = 28_999_999; a missing amount was `unwrap_or(0.0)` = a UTXO worth nothing | parse the reply's decimal text; refuse sub-satoshi and missing |
| `x3-marketplace/fee_distribution.rs` | 80% of `10^18+7` **5 short**, of `1.23e19` **168 short** | `FeeSplit` in basis points; shares sum to the total |
| `x3-atomic-trade/rollback_listener.rs` | a 5% compensation **7** and **860** short | basis points, rounded **up** |
| `x3-cli/commands/swap.rs` | `"0.29"` → `289999999999999996` wei, and that value **executes** | exact decimal → wei; exact formatting; integer estimate |

- **Probe at the magnitudes the code is for.** My first marketplace probe (totals ≤ 100_000) said *no
  defect* because small totals are exact in `f64`; the same code is 5 units short at `10^18 + 7`. A
  clean probe is evidence only when its input could have failed.
- **Rounding direction is part of the contract.** A fee *split* must keep the remainder (the shares sum
  to the total); a *compensation* must round **up** ("at least 5%"); a *limit* must round up; a *quote*
  may floor. State it in the code, and assert the contract (never below, at most one unit above) rather
  than a figure — my first version of the compensation test asserted a floor and failed against a
  deliberate ceiling.
- **`f64` percentage fields are the shape to grep for**, not the arithmetic on them. Also: a doc-comment
  fence with no language is compiled as Rust by rustdoc (`text` is required); a `const` inserted before
  a `#[derive]` splits it from its struct.
- **Two decimal→units parsers now exist** (BTC 8 decimals, wei 18). Neither crate may depend on
  `x3-common` (Substrate/`no_std`) for a parser, and creating a crate for one is TICKET-096's question —
  take the decision if a third caller appears.

### Next task seed
- The ~10 unclassified 094 files: `x3-evolution/{fitness,simulator,mutation,crossover,population,lib}.rs`,
  `x3-opt/peephole_autogen.rs`, `orchestra/jury/{session,rotation}.rs`,
  `x3-sidecar/benchmark.rs`, `x3-integration/mini_x3.rs`, `x3-dex`, `x3-gpu-validator-swarm/{metrics,payment}.rs`.
  Read each before a verdict; GA/optimiser/benchmark code is *probably* not money, but "probably" is not
  a classification.
- Then TICKET-096 (three crates with no consumer), TICKET-046 (decision).
- Needle: `cargo test -p x3-cli -p x3-atomic-trade -p x3-marketplace -p x3-bridge-adapters` all green;
  `scripts/local-ci.sh --only no-float-in-consensus,cargo-lockfile-locked` PASS; x3-lang 1216/0.

---

## Round 94l — 2026-09-20 — the 094 classification is complete, and the gas limit rounds up

**Pushed `883078fbc`** (base `93d762b5c`); **TICKET-094 is now CLOSED** with the classification as its
artifact. Master is `883078fbc`. The ledger is at **97 closed, 4 deferred, 2 blocked, 1 open** — the one
open item is TICKET-046, which needs the user's decision.

- **The gas limit** (`x3-rpc/gas_estimation.rs`): `(total_gas as f64 * 1.25) as u64` gave a limit of **3**
  for `total_gas = 3` — no margin at all, and a limit one under is a transaction that runs out of gas.
  `gas_limit_with_margin` rounds up. Its *test* computed its own expectation with the same float shape,
  so it could not have caught it — a test that re-derives the expression it is testing is not a test.
- **The classification is complete for both shapes that can decide an amount**: a float that **becomes an
  integer** (39 sites in 21 files) and a float **comparison** on an amount-shaped name (10 sites in 4
  files). 5 files converted; 2 clusters (20 sites) are in crates that are **unreachable** (`x3-staking-analytics`
  is not a member and has no dependents; `x3-foundry-core` is a member with none) and are recorded rather
  than converted; the rest are search steps, counts, indices and metrics.
- **The measurement itself has a known gap**: `as_f64()? as i64` does not match a `\bf64\b` token search,
  so the script's 38 sites missed exactly one (an `F64 -> I64` *instruction* in `mini_x3`). Any worklist
  built by grep should be stated as "at least N" and re-measured with a second pattern.
- **Not claimed**: the ~2,700 float uses that neither become an integer nor gate one are classified as a
  category with a reason, not read one at a time. A per-site read of every float use in 512 files has not
  been done, and the ticket says so rather than implying otherwise.
- **Two traps repeated this round**: inserting a helper before an `impl` method's doc comment puts it
  *inside* the impl (the tests cannot see it) — the same shape as the `const` before a `#[derive]` last
  round. Anchor an insertion on a **module-level** item, or check with `grep -n '^fn'` afterwards.
- `cargo test -p x3-rpc` needs `SKIP_WASM_BUILD=1` (the runtime's WASM prerequisite wants a network fetch);
  the repo's own `local-ci` uses the same workaround.

### Next task seed
- **TICKET-046 is the only open item** and it needs the user: 22 clause words as keyword tokens breaks
  `debt.amount`, which the compiler accepts today.
- Deferred with reasons: 016 (needs host evidence), 018/022 (design decisions), 006/008/009 (repo/process;
  two need approval). Blocked: 008/009 need approval.
- Then the standing cross-checks: re-audit the 56 phases against the code, an example for the
  `subscription` item form, a measured branch over *plans*, and the full `scripts/local-ci.sh`.
- Needle: `CASH` — x3-lang 1216/0; `cargo test -p x3-rpc` 28/0; `-p x3-cli` 8/0; `-p x3-atomic-trade` 25/0;
  `-p x3-marketplace` 40/0; `-p x3-bridge-adapters` 29/0; `scripts/local-ci.sh --only
  no-float-in-consensus,cargo-lockfile-locked` PASS; sweep 19/19/19/18.

---

## Round 94m — 2026-09-20 — TICKET-046's decision request, and a second phase audit

No code this turn; two **verification** artifacts, which is what the ledger needed.

- **TICKET-046's conflict is structural, not effort**, and the options now have measured costs in the
  ledger entry: `require proof_complete <name>` and `require proof_complete` + `amount 500` are the same
  tokens up to the identifier, so no lexical rule separates them — only *which clauses the enclosing
  block allows*. (a) keep the union list + its 16-placement validation (what landed; a missing word fails
  two tests); (b) keyword tokens — 42 `keyword_to_tok` arms, 46 `Kw*` variants, 36 parser sites, **and
  every name position must then accept keywords** because `debt.amount` is a shipped field access; (c)
  give the guard the block's clause set and try-parse — 122 per-block comparisons to derive from, more
  correct than the union, and it satisfies the acceptance's purpose without touching the lexer. It is a
  request because the grammar's owner chooses.
- **A phase ledger's caveats go stale first**, because a caveat is a promise that something is refused
  until another ticket lands. The 2026-09-19 audit was a day old and five of its caveats had closed:
  phases 9 (hedge), 10 (liquidation), 11 (rebalance), 22 (netting), 29 (packets). Re-measured by *building
  and running* each construct, with the commands and their output recorded in
  `.ai/reports/x3lang-phase-ledger-20260918.md`'s audit #2.
- **Re-verify a ledger row by exercising it, not by reading it.** `x3c build` + `x3c run` for the hedge,
  rebalance and netting answered the caveat directly; the liquidation needed an **intent alongside it**
  (a liquidation states a profit floor and no ceiling, and `verify_slippage_explicit` wants the ceiling
  from the program) — which is a fact about the language, discovered by trying to build one.
- The remaining audit gap is stated rather than implied: the rows marked DONE by naming a test file are
  covered by today's 1216-test run and the sweep, but each row's artifact has not been re-read line by
  line against today's tree.

### Next task seed
- TICKET-046 needs the grammar owner's pick ((a) recommended now, (c) as a follow-up).
- The standing cross-checks: an example for the `subscription` item form, a measured branch over *plans*,
  the full `scripts/local-ci.sh`, and the canonical checkout fast-forward.
- Needle: x3-lang 1216/0; sweep 19/19/19/18; `scripts/local-ci.sh --only
  no-float-in-consensus,cargo-lockfile-locked` PASS; hedge/rebalance/netting build + run.

---

## Round 94n — 2026-09-20 — `x3c replay`, and the trading decoder it found

**Pushed `2419bc2fe`** (base `883078fbc`). Master is `2419bc2fe`. Ledger: **98 closed, 4 deferred,
2 blocked, 1 open**; PHASE 32's row moved from PARTIAL to DONE.

- **`x3c replay <artifact> <receipt>`** is PHASE 32's artifact-side half: it recomputes the
  domain-separated artifact hash `receipt execute` binds with (refusing another artifact *naming both
  hashes*), runs the receipt's own replay (`verify_receipt` → `verify_receipt_economics`), and prints the
  phase's nine claims as **checked or not-checked** — the honest half, since the phase's "inputs" and
  "state evidence" are exactly what an artifact + receipt do not carry.
- **What is deliberately *not* compared**: the artifact's floors against the receipt's figures. A receipt
  whose hash matches was produced by executing *that* artifact, so a floor it states was already enforced
  by the run; re-comparing would be a second, weaker copy. The floor reader belongs to the *simulation*,
  where the market is stated by a caller rather than produced by a run. I wrote the comparison first and
  removed it for this reason.
- **The command's test found a real defect** (`TICKET-112`): `decode_trading_program` walked the stream by
  hand (`pos += 1` per byte, every non-zero byte read as `[opcode][u16 len][payload]`) with no
  `is_payload_opcode`/`fixed_frame_content_len`/`align4` — so a fixed frame's flags and operand were read
  as a *length* and the walk landed inside a payload: `receipt execute examples/arb_scope.x3` →
  *"truncated instruction payload for opcode 0x65"*, where `0x65` is not an opcode at all. It walks with
  `instructions()` now. **This is the fifth instance of "a second walker with its own idea of a frame
  width"** in this format; the shared table exists for exactly this.
- **A test that shares the code's assumption proves nothing**: the CLI's `receipt execute` tests passed
  while `decode_trading_program` was broken, because none of their fixtures had the fixed-frame +
  payload mixture that desynchronises the hand walk. The arb fixture did.
- When a new command's *setup* fails, the failure may be the setup's or the pipeline's — here it was the
  pipeline's, and the setup (`receipt execute` on a non-trading program) is *supposed* to refuse.

### Next task seed
- TICKET-046 needs the grammar owner's pick ((a) keep the list + validation, recommended; (b) keywords,
  which breaks `debt.amount`; (c) per-block clause sets).
- TICKET-096 (three crates with no consumer), 008/009 (need approval), and the deferred 016/018/022.
- An example for the `subscription` item form; a measured branch over *plans*; the full `local-ci`.
- Needle: x3-lang **1218/0**; clippy + fmt clean; sweep 19/19/19/18; `x3c replay` on a receipt pair ok.

## 2026-09-20 — push/merge state audit (root agent)

**Facts discovered**

- GitHub `refs/heads/master` was `2419bc2fe`; the user's canonical checkout at
  `/home/lojak/Desktop/xxxstar-main` was `920a44775`, **144 commits behind**, on
  branch `master`, with one uncommitted tracked file (`.ai/merge-queue.md`,
  the round-72 section).
- The canonical checkout's `origin/master` remote-tracking ref was stale at
  `93d762b5c`. **Never answer "is it pushed?" from `origin/master` — use
  `git ls-remote origin refs/heads/master`.**
- `.ai/reports` (93 files), `.ai/memory` (1), `.ai/runlogs` (1),
  `.ai/wip-backups` (7) were untracked **and not gitignored** — 102 evidence
  files that only existed on one disk.
- 14 branches had tips ahead of their GitHub copies; only 2 could push as a
  fast-forward. The rest had diverged (`origin/<b>` held commits the local copy
  lacked, e.g. `finish/x3vm-live-transport-fix`: 20 remote commits not in master).
- Three `/tmp` worktrees had uncommitted tracked work: `/tmp/x3-ext`,
  `/tmp/x3-gov`, `/tmp/x3-kernel`. `git stash create` **fails (exit 1)** in all
  three; use `git diff HEAD --binary` instead.
- 51 patches across 8 branches are genuinely unapplied vs master
  (`git cherry -v origin/master <b>` → `+`), but all 8 are 2026-09-11/12
  lineages and their headline features are already on master in later form
  (proof_bundle.rs, secret_release.rs, `NativeX3NodeTransport` exports in
  `crates/x3-atomic-swap/src/lib.rs:106,112`, `x3-lang/vm/src/economic.rs`).

**Decisions made**

- Fast-forwarded the user's checkout after stashing their `merge-queue` edit;
  `stash pop` applied cleanly. Zero collisions were pre-checked by intersecting
  incoming added paths with `git ls-files --others`.
- Pushed the 14 diverged branch tips to `archive/local-20260920/<branch>`
  rather than force-pushing. Force was rejected as a category: it destroys
  remote commits to save refs that master already contains.
- Committed the 102-file evidence pile and the 3 WIP patches to master.
- Scanned everything for credentials before pushing to a remote.

**Canonical paths / commands**

- GitHub truth: `git ls-remote origin refs/heads/master`
- Collision pre-check before a long ff:
  `comm -12 <(git diff --name-only --diff-filter=A A B | sort) <(git ls-files --others --exclude-standard | sort)`
- Divergence: `git rev-list --left-right --count origin/<b>...<b>`
- Unapplied work: `git cherry -v origin/master <b> | grep '^+'`

**Next task seed**

- 4 of the 8 backlog branches are still unverified (only their headline features
  were checked): `feat/canonical-cross-domain-proof-bundle-20260911`,
  `feat/idempotent-cross-domain-coordinator-20260911`,
  `finish/x3vm-live-transport-fix`, `test/cross-domain-refund-recovery-20260911`.
  Verify each of their unapplied patches against master by content before
  declaring them archival.
- Dashboard `dist/assets` (~hundreds of untracked build outputs) should get a
  `.gitignore` entry rather than being committed.

## 2026-09-20 — `EMIT`/`CALL_HOST` records, and `x3c fmt` deleting annotations

**Facts discovered**

- `EMIT` (0x60) and `CALL_HOST` (0x61) were written by hand in
  `x3-lang/compiler/src/emitter.rs` as `format!("{name}:{args:?}")`, and
  `decode_capability_payload` (`x3-lang/crates/x3-common/src/capability.rs`,
  the file is under `x3-lang/`, not `crates/`) had no arm for either. The
  verifier's fallback at `validate_payload_opcode` decodes every unclaimed
  payload opcode through that function, so `emit` and every host call built and
  then failed with `X3_VERIFY_FAILED: InvalidOperand`. Before/after: 96 bytes
  and refused → 40 bytes and runs.
- Five producers reach `Operation::Call`: `@subscribe`, `@sponsor`, the
  `subscription` item, `diff(a, b)`, and the unclaimed-call fallback in
  `lowering.rs`. `@subscription` is **not** one of them — the parser refuses it.
- `Annotation::Subscription` is dead: no constructor anywhere, and the parser
  cannot produce it (TICKET-111). Deleted.
- `X3Formatter` had **zero** references to annotations: `x3c fmt` deleted every
  `@…` from a program, which changed its compiled artifact (172 → 108 bytes on
  the new example, dropping two `CALL_HOST` records). The corpus round-trip test
  never caught it because no corpus file carried an annotation.
- `format_subscription` dropped the period, and `Item::SubscriptionDecl`'s
  lowering dropped `period_blocks` too — declared cadence reached nothing.
- `x3-lang/tests/test_surface_drift.py` requires **every** `examples/*.x3` to be
  classified in `READS` or `REFUSES`; adding an example without a row fails the
  suite. An example with no `intent` is `REFUSES` with `X3_PARSE_NO_INTENT`.
- The root `crates/x3-{ast,common,compiler,lexer}` are separate chain-integration
  copies, not path-deps on `x3-lang/*`. A change in `x3-lang` does not reach the
  root workspace.

**Decisions made**

- Payload records over special-casing: both opcodes go through
  `emit_payload_op` and the one codec. The verifier refuses a record that names
  nothing; the executor's `dispatch_host_opcode` refuses both variants by name
  because the execution loop owns them.
- The formatter uses `annotations::spelling` (already the inverse of the parser's
  name map) rather than a second table.
- `@gas_adaptive`-style "annotation with no artifact form" stays as TICKET-111
  left it; only the unreachable variant was removed.

**Dead ends to avoid**

- `git stash create` fails (exit 1) in the `/tmp/x3-*` worktrees.
- The sandbox denies loopback: `bind("127.0.0.1", 0)` → EPERM and
  `connect 127.0.0.1` → EPERM. Four `scripts/local-ci.sh` gates are red for that
  reason alone (`nested workspaces`, `test node`, `js sdk tests`) plus
  `clippy runtime rc1`, which is a rustc 1.90/1.98 mixed `target/` cache.

**Next task seed**

- `x3c fmt` still has no spelling for anything it cannot place (comments move to
  declaration boundaries and it warns); annotations are now covered.
- A typed record per host operation (instead of `HostCall`'s positional strings)
  needs a new opcode and therefore a bytecode version bump (TICKET-097's gate).

## 2026-09-20 — clause words, and the identifier arms that can never run (TICKET-046 closed)

**Facts discovered**

- The lexer maps `swap`, `bridge`, `require`, `emit`, `use`, `mint`, `burn`, `lock`, `release` to
  keywords, and `keyword_to_tok` (`compiler/src/parser.rs`, ~line 6430) gives each a `Tok::Kw*`
  variant. **An arm of the form `Tok::Ident(ref s) if s == "<one of those>"` can never run.** Three
  clauses were written that way and were unreadable: `use <target> <config>` in an intent body,
  the terse `finality_policy { ethereum require finalized }`, and inline
  `rpc_quorum { source require 2_of_3 }`. Each was fixed by matching the `Tok::Kw*` token.
- `CLAUSE_WORDS` (the guard lookahead) had a stale entry: `balance` appears exactly once in
  parser.rs — the list entry itself. Nothing dispatches on it.
- A guard can only be followed by a clause in a body that also holds statements. Those bodies are
  the intent body and the `atomic swap` body. Route blocks are read by a step loop that refuses
  statements (`expected route operation (…)`), so route-step words (`fallback`, and the keywords
  `swap`/`bridge`) never need a list entry.
- The guard fixture mechanism (`compiler/tests/test_require_guards.rs`) is a list of clause lines;
  adding lines to it is what "reaching every clause word means writing a fixture per grammar"
  turned out to cost once the question was narrowed to guard-bearing bodies. 9 → 13 words.
- Writing a commit message with backticks inside a shell `-m "…"` expands them. Use a heredoc file
  or single quotes. `c7dfb461a`'s message lost three backticked fragments this way; the content
  (code, tests) is unaffected and the ledger/report carry the correct text.
- Seven more identifier-arms-for-keywords survive as dead duplicates (`TICKET-113`); the keyword
  twin does the work in each.

**Decisions made**

- TICKET-046: keep the union list (option (a)), because the acceptance's *purpose* — a guard that
  stops without a list — already holds for the nine words that are keywords, and (b) would require
  every name position to accept a keyword token (`debt.amount` is in two shipped examples).
  Option (c) stays the follow-up if drift is judged material; it is not, because both directions
  of drift now fail a test.
- Did **not** force-push to correct `c7dfb461a`'s commit message.

**Dead ends to avoid**

- `git stash create` exits 1 in the `/tmp/x3-*` worktrees.
- The sandbox denies loopback (`bind`/`connect 127.0.0.1` → EPERM), which is what makes the
  `nested workspaces`, `test node` and `js sdk tests` local-ci gates red.
- Grepping the parser for clause words by line number is unreliable (offsets shift); anchor on the
  function name or read the source in a test.

**Next task seed**

- TICKET-113: delete the seven dead duplicate arms (method in the ticket). Cheapest is to add a
  source-scanning test that fails for any `Tok::Ident(ref s) if s == w` where `w` is a lexer keyword
  with a `Tok::Kw*` mapping — that makes the class impossible to reintroduce.

**2026-09-20 — TICKET-113 closed (same pass)**

- The seven dead arms are deleted; `no_arm_matches_an_identifier_for_a_lexer_keyword`
  (`compiler/tests/test_keyword_clauses.rs`) now fails for any such arm. It reads the lexer's
  keyword table **and** `keyword_to_tok`: the dead words are the *intersection*, because `timeout`
  is a lexer keyword with no `Tok::Kw*` arm and therefore arrives as an identifier (my first test
  version wrongly flagged four `timeout` arms).
- The scanner reads the first quoted word after `Tok::Ident(ref`, so a dead half hidden behind a
  live one — `requirement || require` — is invisible to it. Found by reading; the test is a floor,
  not a proof of absence.
- `on_fail` was removed from `CLAUSE_WORDS`: it is a keyword, so the guard stops at it with no
  entry. The list is now 20 entries, all identifier-reaching.
- 1234 workspace tests, clippy/fmt clean, 23 python tests, sweep 20/20/20/19.

**2026-09-20 — TICKET-008 and TICKET-009 closed (they were BLOCKED on approval, and the blockers are gone)**

- TICKET-008 (move the main worktree off a 444-behind branch): the checkout is on `master` and
  `HEAD == origin/master` (`69e7e8849`). Done in the `83d354b1a` round: stash the one local edit
  (`.ai/merge-queue.md`) → `merge --ff-only` → `stash pop`, after intersecting incoming added paths
  with `git ls-files --others` to prove zero collisions. Nothing was lost.
- TICKET-009 (three uncommitted WIP piles): all three worktrees report `git status --porcelain`
  clean, and the piles are committed on `master` as patches under `.ai/wip-backups/` (the Sept-18
  three plus the `20260920-*` three found later). Not merged to their branches — the merge-queue
  adjudication still applies.

## 2026-09-20 — guards: the walk, and what the VM actually enforces

**Facts discovered**

- `semantic::require_guards` (`compiler/src/semantic.rs`) read only the **top level** of an intent
  body plus four declarations' `requires` lists. It did not descend into `if`/`while`/`for`/`loop`/
  `atomic`, did not read `Statement::RouteFallback.requires`, and did not look at functions, agents,
  gpu blocks, simulate/task/subscription bodies, choice paths or parallel legs. Thirteen checks read
  it, so a guard one block down was invisible to every one of them. Fixed in `570718bbb`.
- `Statement::RouteFallback { replacements, .. }` in lowering **dropped** the block's guards with
  `..` — `fallback { require profit >= 0 }` reached neither artifact nor runtime.
- A user-written economic guard lowered with `measured: false` → the emitter wrote
  `REQUIRE static 0`, and `static` is treated as satisfied by the executor. Two programs with
  different slippage ceilings compiled to **byte-identical artifacts**. Fixed in `36f2856e3`: the
  bound is converted with `semantic::slippage_bps_from_text` (bare number = basis points, `5%` = 500)
  and emitted as a measured guard.
- The spec (`pasted-text-1.txt` line 41-42) requires exactly this: "Native fee/slippage guards —
  Economic constraints enforced by the VM".
- Old artifacts are unaffected (`static 0` still reads as satisfied); no version bump needed. The
  version byte is a function of the opcode set; `POLICY_VERSION` is carried, not compared.
- The CLI already had `--measured-profit-bps` / `--measured-slippage-bps`; they are now required for
  a program's own economic guards too, not only for plan floors.

**Decisions made**

- Enforce rather than record (fail-closed): an unmeasured guard refuses with `X3_GUARD_UNMEASURED`.
  Sweep: run-with-a-stated-outcome stays 19/20, run-unmeasured is 8/20.
- `slippage >= n` and `profit <= n` stay static: the direction is not what the quantity means, and
  inverting it at the emitter would enforce something the program did not write.
- One existing test asserted the old design ("a program's own guard is a compile-time constraint and
  must still run unmeasured"); it now asserts both halves of the new one. Flagged rather than
  silently adjusted.

**Next task seed**

- TICKET-114: every non-economic guard kind still emits `REQUIRE static 0`, so its bound is checked
  and then dropped from the artifact. Needs a per-kind unit decision for the operand.
- `risk { max_total_fee_bps }` is compile-time only; no fee quantity is stateable.

**2026-09-20 — TICKET-114 closed, and the `unwrap_or(0)` hole it uncovered**

- The flags byte's bits 5-7 (the measured guard's unit code) now also name the quantity a *static*
  guard's figure counts: `GUARD_QUANTITY_{AMOUNT,SCORE,COUNT,BLOCKS}` = 3,4,5,6 in
  `spec/opcodes.rs`. Zero is "no figure carried" (backward compatible); the set is closed in the
  verifier (`is_known_guard_quantity`).
- `x3c explain` prints `REQUIRE static score 90` / `static amount 10000` / `static blocks 32` /
  `static count 3`.
- `static_guard_quantity` in `compiler/src/emitter.rs` picks the code per IR kind (RouteScore |
  RiskScore → score, SolverBond | BridgeLiquidity → amount, RelayerQuorum → count); the finality
  branch uses blocks. A figure that does not fit u16 is refused, not truncated.
- **The hole:** `verify_route_score_declared`, `verify_solver_bond_declared` and
  `verify_relayer_quorum_declared` read `guard.value.and_then(extract_int_from_expr).unwrap_or(0)` —
  so `require route_score >= min_score` passed every policy and recorded threshold 0. All three now
  refuse a bound that is not a number. Other checks (fallback bounds, atomic-swap output, finality
  depth) already refused or fell through to a mode word.
- Pattern worth keeping: `unwrap_or(0)` on a value the compiler *should* be able to read turns an
  unreadable input into the weakest possible claim. Grep for `.unwrap_or(0)` next to a parse.
