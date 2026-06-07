# Change: add-swarm-orchestra-platform

## Status

DRAFT

## Authors

GitHub Copilot

## Summary

Define the X3 swarm-orchestra platform as a bounded off-chain control plane built on the live validator, sidecar, gateway, and governance surfaces that already exist in the repository. This change replaces the earlier draft framing that treated the deprecated legacy swarm path as a production base.

## Motivation

The repository already contains several relevant but unevenly mature subsystems. Deterministic GPU validation is active in `x3-gpu-validator-swarm`. Benchmark execution and report publishing already exist in `x3-sidecar` and `x3-gateway`. Runtime governance exists on-chain. The older `gpu-swarm` crate and `pallet-swarm` are explicitly deprecated. The existing draft proposal blurred those boundaries and implied platform capabilities that the codebase does not currently provide, especially around human approval flow and court-style voting.

This update makes the proposal truthful to the current code. It sets the first production scope around three services: gateway, sidecar, and a new orchestra-control-plane. It also records two prerequisite safety fixes that must exist before any broader automation surface is added: authority startup must enforce the determinism gate, and governance durations must be derived from the runtime's 200ms block target rather than the obsolete 6-second timing assumptions.

## Design

The platform is structured as an interface layer, a gateway layer, a protocol layer, and a bounded service-and-control layer.

The protocol layer remains the X3 node and runtime. It provides dual-VM execution, deterministic validation, receipts, settlement, and on-chain governance. The production validator base is `x3-gpu-validator-swarm`, not `gpu-swarm`. The sidecar remains the benchmark and receipt execution daemon. The gateway remains the public GraphQL and REST facade for off-chain workflows.

The new orchestra-control-plane owns intent intake, approval routing, CRM vote windows, evidence bundles, and reward orchestration. It does not own consensus, replay adjudication, or direct validator execution. The `x3-court` crate remains the deterministic replay-and-slashing court for protocol disputes. Human approval and CRM voting are separate off-chain workflows that may anchor final outcomes on-chain when needed.

Workflows remain role-typed. GPUs may autonomously validate, replay, benchmark, and perform approved defensive analysis. Public content publication, sanctions, treasury-affecting actions, and new strategy activation remain human-approved. Gateway and orchestra APIs may stage, review, and record these actions, but they may not bypass runtime governance for actions that materially change chain state.

## Integration Points

- [docs/x3-swarm-orchestra/README.md](../../x3-swarm-orchestra/README.md)
- [docs/x3-swarm-orchestra/EXECUTIVE_SUMMARY.md](../../x3-swarm-orchestra/EXECUTIVE_SUMMARY.md)
- [x3-security-swarm/README.md](../../../x3-security-swarm/README.md)
- [crates/x3-gpu-validator-swarm/src/orchestrator.rs](../../../crates/x3-gpu-validator-swarm/src/orchestrator.rs)
- [crates/x3-gpu-validator-swarm/src/quarantine.rs](../../../crates/x3-gpu-validator-swarm/src/quarantine.rs)
- [crates/x3-sidecar/src/lib.rs](../../../crates/x3-sidecar/src/lib.rs)
- [crates/x3-sidecar/src/rpc.rs](../../../crates/x3-sidecar/src/rpc.rs)
- [crates/x3-gateway/src/main.rs](../../../crates/x3-gateway/src/main.rs)
- [crates/x3-gateway/src/rest.rs](../../../crates/x3-gateway/src/rest.rs)
- [crates/x3-gateway/src/graphql.rs](../../../crates/x3-gateway/src/graphql.rs)
- [crates/x3-court/src/court.rs](../../../crates/x3-court/src/court.rs)
- [runtime/src/lib.rs](../../../runtime/src/lib.rs)
- [node/src/service.rs](../../../node/src/service.rs)
- [tests/invariants/registry.toml](../../../tests/invariants/registry.toml)

## Invariants

- `SWARM-ORCH-001` — deterministic validator execution stays rooted in `x3-gpu-validator-swarm`; deprecated swarm crates are not part of the production validator path.
- `SWARM-ORCH-002` — autonomous GPU actions are limited to validation, replay, benchmarking, and approved defensive analysis.
- `SWARM-ORCH-003` — every externally visible off-chain action must map back to an intent, reviewer or vote window, execution path, and evidence bundle.
- `SWARM-ORCH-004` — content/media agents may only consume approved asset sources and may not publish without approval state.
- `SWARM-ORCH-005` — human approval flow and CRM voting are off-chain control-plane workflows; `x3-court` remains the deterministic replay/slashing court and is not the human approval board.

## Testing Strategy

Add workflow tests around provider onboarding, benchmark report publication, approval gating, vote-window closure, intent lineage, content-asset policy enforcement, and routing separation between validation and outward-facing automation. Add integration tests around sidecar-to-gateway benchmark publishing, orchestra evidence-bundle creation, and startup-gate enforcement for authority nodes.

## Rollout Plan

Start with code-truthful documentation and the two prerequisite safety fixes. Next, land Phase 1 for gateway, sidecar, and orchestra-control-plane without changing consensus behavior. Then wire approval and evidence lineage into a narrow internal-only workflow. Only after those controls exist should broader automation or outward publication surfaces be enabled.

## Risks and Mitigations

The largest risk is architecture sprawl without enforcement boundaries. The mitigation is to build on the active validator, sidecar, and gateway surfaces and keep deprecated swarm code out of the production path. A second risk is stale timing or startup assumptions leaking into governance and validator admission. The mitigation is to derive governance periods from the live block target and fail authority startup when the determinism gate fails. A third risk is mixing content, trading, and sanctions in one opaque automation layer. The mitigation is explicit service separation, approval-bound permissions, and evidence capture.

## Open Questions

- Which service should own the authoritative intent ledger for off-chain actions?
- Should the first approval layer live inside orchestra-control-plane or behind a narrower gateway-owned approval module?
- Which operator dashboard becomes the canonical control plane for approval, security, and performance views?
