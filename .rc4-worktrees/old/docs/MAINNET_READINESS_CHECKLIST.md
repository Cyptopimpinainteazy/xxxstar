# X3 Mainnet Readiness Checklist

## Purpose
This checklist captures the minimum launch readiness work for X3 Atomic Star to move from internal audit mode to a credible mainnet/canary readiness posture.

## Related public plan
- `docs/MAINNET_CANARY_PLAN.md` — public canary launch path and 30–90 day reveal plan
- `docs/MAINNET_GATE_PROOFS_PLAN.md` — repository-wide mainnet gate and proofs execution plan
- `docs/archive/status/` — historical archived status reports

## Priority 1 — Mainnet safety gate (P0)
- [ ] Resolve all open P0 mainnet-blocker PRs before any launch announcement.
  - Focus on node-killing panics, validator rotation/fork-safety fixes, and consensus stability.
- [ ] Burn down the open Rust vulnerability backlog, with critical/high dependency issues remediated or formally mitigated.
- [ ] Confirm the ProofForge runner is real and not stubbed for mainnet-proof gates.
  - `proof-forge/src/runners/formal_proofs.rs` should execute real tools, not stubs.
- [ ] Reconcile readiness documents so the internal status story reflects actual gate state.
  - Update `docs/CURRENT_MAINNET_STATUS.md`, `docs/MAINNET_READINESS_DELTA.md`, and `MAINNET_READINESS_PUSH_COMPLETE.md` together.
- [ ] Validate that `ExternalBridgesEnabled=false` in the mainnet genesis and that bridge enablement is gated by formal audit.
- [ ] Generate and review live mainnet chain specs:
  - `chain-specs/x3-mainnet-plain.json`
  - `chain-specs/x3-mainnet-raw.json`
- [ ] Confirm no dev authorities, no dev genesis accounts, and no test-only keys are present in mainnet genesis.
- [ ] Run the full runtime and node gate suite:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo check --workspace`
  - `cargo test --workspace --lib --tests -- --test-threads=1`
- [ ] Complete the panic/unwrap audit and publish it in `reports/panic_unwrap_audit.md`.
- [ ] Ensure all P0 mainnet gate failures are tracked and blocked in `proof/policies/release_gates.yml` and `proof/policies/todo_policy.yml`.

## Priority 2 — Canary testnet credibility (public P1)
- [ ] Freeze the public launch scope to RC-1 reality:
  - Atomic Kernel
  - Multi-VM runtime (X3Native/EVM/SVM)
  - Cross-VM router
  - One minimal DEX canary path
  - Bridge disabled-by-default until audit passed
- [ ] Publish a public canary readiness scoreboard with green/yellow/red statuses for enabled and intentionally disabled features.
- [ ] Deliver one reproducible atomic rollback demo.
  - A cross-VM user flow that fails cleanly and reverts all state.
- [ ] Deliver one reproducible GPU/CPU determinism proof.
  - Same transaction / same receipts on CPU and GPU.
- [ ] Publish a public explorer/status page or status board.
- [ ] Publish node + validator quickstart docs.
  - A single command local node path.
  - A single command canary validator bootstrap path.
- [ ] Package proof receipts for the core mainnet gate:
  - supply invariant proof
  - rollback/replay proof
  - bridge audit gate proof
  - ProofForge receipt integrity proof
- [ ] Validate DEX/launchpad safety with targeted tests and publish the results.
  - Swap rollback
  - liquidity lock accounting
  - anti-rug / sandwich resilience

## Priority 3 — Launch operations and messaging
- [ ] Keep post-quantum, AI, GPU acceleration, and external bridge features off the main launch claim until they are audited.
- [ ] Make the launch message: Proof before hype.
  - “Public rollback receipts, determinism receipts, canary benchmarks, and launch gates.”
- [ ] Publish a simple validator/onboarding story that a third party can reproduce.
- [ ] Ensure launch governance and emergency recovery procedures are documented and tested.
- [ ] Seal the public testnet story with a stable explorer/faucet/chain-spec package.

## References
- `docs/MAINNET_LAUNCH_CHECKLIST.md`
- `.x3/X3_MAINNET_GATES.md`
- `docs/CURRENT_MAINNET_STATUS.md`
- `docs/MAINNET_READINESS_DELTA.md`
- `stakeholder_comms/ENGINEERING_TEAM_ANNOUNCEMENT.md`
- `proof/policies/release_gates.yml`
- `proof/policies/todo_policy.yml`
- `proof/receipts/claims/`
- `chain-specs/x3-mainnet-plain.json`
- `chain-specs/x3-mainnet-raw.json`
- `reports/panic_unwrap_audit.md`
- `.swarm/state/task_queue.md`
