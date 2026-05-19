# RC-1 Feature Debt and Incomplete Feature Backlog

This document captures the incomplete or roadmap features referenced by the RC-1 readiness audit and the current repository surface. It is intended to help the team start finishing the most important incomplete work.

## Incomplete RC-1 / post-RC-1 features

### 1. `x3-parallel-executor`
Path: `crates/x3-parallel-executor/`

Status:
- Crate exists with a stubbed executor, conflict detector, access list builder, and tests.
- The implementation is currently a simplified simulation of parallel execution.
- `pallets/x3-cross-vm-router/Cargo.toml` explicitly gates `parallel-executor` off RC-1.

Next work:
- Implement real access list analysis for IXL transactions.
- Replace simulated batch execution with task-safe parallel execution or a deterministic scheduler.
- Add integration tests validating serial equivalence for conflicting and non-conflicting transaction sets.
- Add observability metrics for wave count and conflict rates.

### 2. `x3-appzone-factory`
Path: `crates/x3-appzone-factory/`

Status:
- CLI tool scaffold exists with template creation, deploy, list, validate, and update flows.
- Current implementation is still largely placeholder: template copy, deploy config generation, and update logic need real integration.
- This feature is explicitly gated out of RC-1 via `appzone-factory`.

Next work:
- Complete template metadata and actual app zone scaffolding.
- Wire deploy logic to network-specific configuration and package publishing.
- Add validation coverage for app zone manifests and runtime wiring.
- Add CLI integration tests and a sample app zone artifact.

### 3. `external-gateway` / bridge gateway
Path: likely `crates/x3-crosschain-gateway/` and related bridge crates

Status:
- External bridge activation is explicitly blocked for RC-1.
- Code exists in the repo, but the runtime and bridge path are intentionally disabled by scope guards.

Next work:
- Build the external gateway flow separately from RC-1.
- Add bridge root verification, relayer proof path, and audit/gate hardening.
- Maintain the RC-1 kill switch until external gateway is independently certified.

### 4. `pq-experimental`
Path: `crates/x3-pq/` and `crates/quantum-crypto/`

Status:
- Post-quantum crypto crates exist in the workspace.
- Runtime-level RC-1 feature gate forbids PQ in the shipped product.

Next work:
- Harden the PQ cryptography path off-chain first.
- Add proof-of-concept integration tests for signature and key handling.
- Keep the RC-1 public launch scope clean until PQ is audited.

### 5. `advanced-dex`
Path: `crates/x3-dex/`, `crates/atomic-swap-orchestrator/`, `crates/chronos-flash/`

Status:
- Spot AMM and LP lock behavior exist in RC-1,
- Perps/options/flash loan extensions are intentionally deferred.

Next work:
- Add advanced DEX settlement and economic attack tests in a separate release track.
- Keep current RC-1 messaging focused on spot swap / LP lock only.

### 6. `ai-optimizer`
Path: `crates/contention-predictor/`, `crates/ai-*`, `crates/x3-optimizer/`

Status:
- AI optimizer/consensus routes are scoped out of mainnet RC-1.
- This is a future product track, not a current launch dependency.

Next work:
- Separate AI route optimization from consensus-critical launch code.
- Build it as an optional optimization layer after testnet stabilization.

### 7. `gpu-acceleration`
Path: `crates/cross-chain-gpu-validator/`, `crates/gpu-sig-verifier/`, `crates/gpu-swarm/`

Status:
- GPU validator acceleration and GPU-critical paths are gated off RC-1.
- Existing GPU docs should remain research/benchmarks-only.

Next work:
- Keep GPU code out of mainnet release unless a fully audited path exists.
- Focus on deterministic non-GPU validator flow for RC-1.

## Immediate starting point
The fastest concrete start is:

1. Harden the parallel executor stub into a real library with tests.
2. Harden the appzone factory CLI flow and sample templates.
3. Keep the RC-1 scope locked by preserving the current feature gate docs and compile-time guards.
4. Create a public-facing status summary that clearly distinguishes shipped RC-1 capabilities from gated roadmap items.

## Recommended first tasks
- [ ] Add `crates/x3-parallel-executor` integration tests for wave assignment and serial equivalence.
- [ ] Add `crates/x3-appzone-factory` sample template assets and CLI smoke tests.
- [ ] Add a `docs/RC1_FEATURE_DEBT.md` backlog (this file) to the repo.
- [ ] Link `docs/RC1_FEATURE_DEBT.md` from `docs/CURRENT_MAINNET_STATUS.md` and `docs/MAINNET_READINESS_DELTA.md`.
- [ ] Validate that `pallets/x3-cross-vm-router` scope guards still compile cleanly with `mainnet-rc1` active.

## Notes
This document is intentionally not a feature-completion promise. It is a starting backlog for the scoped RC-1 and post-RC-1 work referenced by the current audit.
