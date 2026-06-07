# Atlas Sphere (X3) Two-Week Development Roadmap

## Objective
Deliver a secure, observable, and governable X3 chain stack over the next 14 days by focusing on the high-priority audit/test coverage work for the DEX + flashloan layers, locking down MEV/Chronos, aligning CI/CD/governance, and refreshing documentation/monitoring so the chain can ship safely with measurable guardrails.

## Week 1 (March 9–15)

1. **Audit the financial stack (x3-dex + x3-flashloan + agents)**
   - Assign the Security team to burn through `crates/x3-dex/src/{flash_loan,route_finder,atomic_…}` and `crates/x3-flashloan/src/{planner,pool,executor}` plus any off-chain agents that orchestrate flashloan flows (e.g., `crates/x3-agent/` or `crates/x3-gateway`).
   - Capture invariants (atomicity, no partial fills, balance integrity) in a short checklist, publish in `.codex/CONSENSUS_INVARIANTS.md`, and refer to that checklist in the new audit tickets so reviewers can verify compliance.
   - Have the audit owners document findings in `.codex/X3_AUDIT_PASS.md` as soon as they validate each module.

2. **Expand unit tests + negative path coverage**
   - For swap flows: add targeted unit/negative tests for every AMM/router path in `crates/x3-dex/src/{amm_pools,concentrated_liquidity,batch_swap_router}` so success/failure paths (invalid params, insufficient liquidity, deadline expiry) are covered.
   - For flashloans: extend `crates/x3-flashloan/src/{planner,pool,executor}` tests with edge cases (invalid borrowing curve, simultaneous repay failure, zero liquidity) and ensure invariants revert the entire state.
   - For `x3-swap-router`: create a dedicated `tests/` module that exercises `mev_protection::{SandwichProtection,MEVProtector}` and `AtomicSwapRouter::execute_atomic_swap` with mocked routes to check error handling.
   - For `gpu-swarm`: start by adding unit tests for `crates/gpu-swarm/src/{coordinator,node,scheduler,warden}` (e.g., `ThreatLevel` transitions, MEV discovery job behavior) plus a small integration test that spins up a `Coordinator` with 2 nodes using the `jobs/mev_discovery.rs` path to emulate MEV signals.

3. **Coverage tooling and CI guardrails**
   - Integrate `tarpaulin`/`grcov` runs into the main CI (see `.github/workflows/ci.yml`) so we can publish a coverage report per push. Leverage the commented metadata in `Cargo.toml` as a starting point for configuration.
   - Define “critical path” spec IDs for the DEX/flashloan/swap-router/gpu-swarm coverage, record them in `.codex/consensus_paths.txt`, and extend `scripts/x3_enforce.py` so the `x3-enforce` workflow blocks merges that touch those paths without fresh coverage artifacts (e.g., `target/tarpaulin/report.json`).
   - Add property/fuzz harnesses for the swap planner + flashloan planner (new `fuzz/` modules under each crate or a shared `crates/x3-enforce-fuzz/src`). The harnesses should use `cargo fuzz` or `cargo afl` and assert no panics, no state drift, and full rollback on invalid inputs.

4. **Immediate actions for Week 1**
   - Publish this roadmap so every owner knows their deliverables.
   - Create the first backlog tickets (audit, tests, coverage) with spec IDs for x3-enforce.
   - Start the `x3-swap-router` test scaffolding and register the new coverage target with the CI (even if it is a placeholder run that just prints `cargo test`).

## Week 2 (March 16–23)

1. **MEV/Chronos hardening**
   - Require multi-review approvals for any changes touching `crates/x3-swap-router/src/mev_protection/lib.rs` and `x3-mev` guard logic. Document each protection algorithm (e.g., SandwichProtection filtering, route scoring) in `docs/docs/reports/` so reviewers understand the invariants.
   - Treat `ChronosFlash` as a feature toggle: keep it disabled by default on mainnet until validation completes, add docs in `crates/chronos-flash/src/{oracle,config,timewarp}.rs` explaining the “100–400ms pre-execution” window, and note that running it requires explicit configuration in `docker-compose.production.yml`.
   - Build a lightweight MEV monitoring dashboard that consumes signals from `crates/gpu-swarm/src/jobs/mev_discovery.rs` + the Warden metrics in `crates/gpu-swarm/src/warden/{metrics.rs,governance.rs}`. The dashboard should track `ThreatLevel` (see `ThreatLevel::recommended_action`) and flashloan/gas anomalies and expose them via the Grafana definitions in `grafana-dashboards.yml`.

2. **Governance and Treasury activation**
   - Define runtime parameters (periods, thresholds, enactment delay) for the governance pallet located in `pallets/x3-governance`. Move away from `Sudo` in `pallets/x3-kernel` by drafting a migration plan (e.g., mimic `pallets/governance` + `pallets/treasury`).
   - Formalize treasury funding flows via `pallets/treasury`, update `docs/reports/X3_ATLAS_SPHERE_CODEBASE_ANALYSIS.md`, and write a short “public goods funding” playbook for the multi-sig/council features already in the governance pallet.
   - Kick off the “Web3 Court” spec (use `crates/x3-court` + `crates/x3-proof`): describe how on-chain identity, dispute resolution, and optional ZK proof validation (e.g., `crates/x3-proof` circuits) plug into the governance flow.

3. **CI/CD, deployment, and documentation**
   - Have the DevOps team harden each workflow (`.github/workflows/security-guardrails.yml`, `production-deploy.yml`, `mainnet-gating.yml`) with linting, secret scanning, and dependency checks. Replace any baked-in keys/URLs with env vars or templates from `deployment/` and gate mainnet pushes behind the `mainnet-gating` workflow.
   - Stand up a staging pipeline (mirror the mainnet config defined in `deployment/`, `k8s-deployment.yaml`, `docker-compose.production.yml`) so upgrades can be validated before hitting production.
   - Assign a Docs Owner to refresh `docs/root/README.md`, `docs/runbooks/getting-started/QUICK_START.md`, and the developer guides (e.g., `docs/runbooks/getting-started/README_SETUP.md`, `docs/reports/X3_ATLAS_SPHERE_CODEBASE_ANALYSIS.md`) so they mention the new `x3-*` names, updated setup steps, and matching Cargo version numbers.

4. **Monitoring & continuity**
   - Ops/SRE owns alerts for abnormal behavior (huge token transfers, mempool spikes, block time drift). Hook these alerts into the existing Grafana/Prometheus stack (`prometheus.yml`, `grafana-dashboards.yml`) and call out what the Warden should do when `ThreatLevel::is_emergency()` becomes true.
   - Use `crates/gpu-swarm/src/warden/signals.rs` + `monitoring/` to detect failing GPU nodes, inconsistent proofs, or repeated verification failures. Document an incident playbook (freeze agent keys, rollback, emergency override via `crates/gpu-swarm/src/warden/governance.rs::EmergencyOverride`).
   - Refresh `docs/runbooks/operations/docs/runbooks/operations/MONITORING_GUIDE.md` so it outlines drill frequency and “measure as we go” KPIs (200ms block target, zero stalls). Include the `WardenDecision` payload fields as part of the dashboard so the “ThreatLevel” can be tied to alerts.

## Ownership / Milestones

| Workstream | Owner | Key artifact | Milestone |
|------------|-------|--------------|-----------|
| Financial audits + tests | Security Team | `.codex/X3_AUDIT_PASS.md`, new tests under `crates/x3-dex`, `crates/x3-flashloan`, `crates/x3-swap-router`, `crates/gpu-swarm` | Block any merge touching critical paths without passing `x3-enforce` + coverage artifacts |
| MEV & Chronos guardrails | Trading/ML Engineers | Docs for `mev_protection`, `ChronosFlash` toggle config, MEV dashboard | Multi-review gate after each MEV change + disable ChronosFlash unless validated |
| Governance stack | Blockchain Core + Legal/Compliance | `pallets/x3-governance` runtime params, `pallets/treasury` funding playbook, `crates/x3-court` spec with ZK hooks | Migration off Sudo + treasury funding flow defined |
| CI/CD + Deployment | DevOps | Hardened `.github/workflows`, templated deployment scripts, staging pipeline | No mainnet deploy without security lints + staging validation run |
| Docs | Docs Owner | Updated `docs/root/README.md`, `docs/runbooks/getting-started/QUICK_START.md`, developer guides, architecture diagrams | All top-level docs reflect current dual-VM status + `x3-*` naming |
| Monitoring & Continuity | Ops/SRE | Alerting playbook, Grafana dashboards, `EmergencyOverride` drill docs | Regular drills + measurable KPIs (ThreatLevel mapping, block time drift) |

## Immediate Next Steps

1. Lock the Security team on the `x3-dex`/`x3-flashloan` audit scope, capture invariants in `.codex/CONSENSUS_INVARIANTS.md`, and log progress in `.codex/X3_AUDIT_PASS.md` so `x3-enforce` can gate merges.
2. Start building the missing `x3-swap-router` test module (mock routes + MEVProtector) and register the new coverage target in the toolchain (tarpaulin/grcov). Produce a short test report that can be plugged into `x3-enforce`.
3. Update `scripts/x3_enforce.py` or add a companion `scripts/x3_enforce_coverage.py` so the workflow knows which spec IDs require coverage artifacts; document the spec list in `.codex/consensus_paths.txt`.

## Measure-as-we-go

Keep track of these success indicators each day:
- Test/coverage: run `cargo test` + `cargo tarpaulin` for critical crates, confirm coverage thresholds in CI.
- MEV/Chronos: show a documented review and dashboard event for each ThreatLevel change.
- Governance/Docs: confirm new runtime params are reflected in `docs/reports/X3_ATLAS_SPHERE_CODEBASE_ANALYSIS.md` and `docs/root/README.md`.
- Monitoring: verify Grafana `ThreatLevel` chart updates when the Warden emits high severity alerts.

Document blockers immediately so the team can re-prioritize rather than rushing at the deadline.

## Spec IDs (critical paths for enforcement)

- SPEC-DEX-COV — `crates/x3-dex/**`
- SPEC-FLASH-COV — `crates/x3-flashloan/**`
- SPEC-SWAP-COV — `crates/x3-swap-router/**`
- SPEC-GPU-SWARM-COV — `crates/gpu-swarm/**`
