# MAINNET CANARY PLAN

## Purpose
This document defines the public canary/testnet launch path for X3 Atomic Star. It preserves the repo’s current internal RC-1 scope while turning real proof artifacts into a credible public reveal.

## Target launch narrative
> “X3 Atomic Star is launching an internal-atomic-kernel public canary: one canonical ledger, internal X3Native/EVM/SVM settlement, strict supply invariants, deterministic rollback/refund, audited bridge disablement, and public proof artifacts.”

## Canary launch scope
Keep the public launch scope tight and verifiable:
- Atomic Kernel
- Multi-VM runtime: X3Native + EVM + SVM
- Cross-VM router
- One minimal DEX canary path
- Bridges disabled by default until audit
- Proof-backed mainnet readiness board
- GPU/CPU determinism proof as a trust signal

## Stage 1: Canary launch prep (Days 1–30)
1. Freeze public claims to the RC-1 surface.
   - Remove or qualify broader claims in README/docs.
   - Stop marketing PQ, AI, GPU, external bridge, and advanced DEX as launch features.
2. Nail down P0 mainnet blockers.
   - Fix node panics, validator rotation, fork-safety, and open critical dependency issues.
   - Verify all `proof/policies/release_gates.yml` and `proof/policies/todo_policy.yml` blockers reflect reality.
3. Reconcile public status docs.
   - Update `docs/CURRENT_MAINNET_STATUS.md`, `docs/MAINNET_READINESS_DELTA.md`, `docs/MAINNET_LAUNCH_CHECKLIST.md`, and `docs/MAINNET_READINESS_CHECKLIST.md` together.
4. Build and publish minimal live artifacts.
   - Generate `chain-specs/x3-mainnet-plain.json` and `chain-specs/x3-mainnet-raw.json`.
   - Publish binary hashes for the current testnet artifact.
   - Publish public explorer/status board skeleton.
5. Produce three reproducible proofs.
   - Atomic rollback demo
   - Cross-VM swap demo
   - GPU/CPU determinism proof
6. Publish the readiness scoreboard.
   - Track features as green/yellow/red.
   - Explicitly mark out-of-scope items as staged.

## Stage 2: Public validation and stress testing (Days 31–60)
1. Open the canary network to external validator participants.
   - Publish a validator quickstart.
   - Provide a one-command local node and one-command validator bootstrap path.
2. Run a public MEV gauntlet on the canary network.
   - Share attacker harness and results.
   - Target sandwich, replay, timeout/rollback, and duplicate-settlement cases.
3. Publish raw benchmark artifacts.
   - Share TPS harness results, hardware notes, and logs.
   - Report determinism and validator health metrics.
4. Validate ops readiness.
   - Publish collapse/recovery runbooks.
   - Confirm bootstrap, chain restart, and emergency governance flow.

## Stage 3: Externalization and mainnet decision (Days 61–90)
1. Invite outside validators to run the canary from docs.
   - Gather feedback and reproduce any failures.
2. Publish an audit scope.
   - Clarify what is in-scope for the launch canary and what remains post-canary.
3. Consolidate the release candidate package.
   - Include chain spec, binary hashes, docs, proof receipts, and runbooks.
4. Re-assess the mainnet window.
   - Only announce a mainnet date after the P0 gate status meaningfully improves and external canary validator results are acceptable.

## Public no-go zones
Do not claim these until they are audited and stable:
- external bridge / gateway mainnet launch
- post-quantum signature or key-management production readiness
- advanced DEX or lending product launch
- AI optimizer or parallel-executor mainnet activation
- GPU-accelerated validator consensus as a launch guarantee

## Ownership and references
- Primary doc: `docs/MAINNET_READINESS_CHECKLIST.md`
- Internal gate reference: `.x3/X3_MAINNET_GATES.md`
- Launch checklist: `docs/MAINNET_LAUNCH_CHECKLIST.md`
- Proof gate artifacts: `proof/receipts/claims/`
- Explorer/status data: `web/mainnet-progress/data/*`
- Launch gate reports: `launch-gates/reports/*`

## Why this matters
The repo already contains deep technical scope. The fastest credible path is not louder marketing — it is disciplined proof and a small, verifiable public canary launch.
