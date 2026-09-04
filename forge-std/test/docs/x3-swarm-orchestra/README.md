# X3 Swarm Orchestra

This directory documents the X3 swarm-orchestra platform shape. It is not a standalone runnable orchestrator, and it should not be treated as proof that the whole platform already exists as one deployable service.

The production-relevant base in this repository is narrower than the earlier draft implied. Deterministic validator execution lives in `crates/x3-gpu-validator-swarm`. Benchmark execution and report publishing live in `crates/x3-sidecar` and `crates/x3-gateway`. Runtime governance and validator admission live in `runtime` and `node`. The older `crates/gpu-swarm` path and `pallets/swarm` are explicitly deprecated and retained for reference only.

## Current Scope

- Validator and benchmark automation: active and code-backed.
- Gateway-exposed workflow APIs: active and code-backed.
- Human approval, CRM voting, evidence bundling, and reward orchestration: planned control-plane work, not finished code.
- Court-style replay and slashing: active in `crates/x3-court`, but this is not the same as a human approval board.

## OpenSpec Status

The platform-level architecture and operating assumptions are captured in [docs/x3-swarm-orchestra/EXECUTIVE_SUMMARY.md](EXECUTIVE_SUMMARY.md). The authoritative change proposal and implementation tickets live under [docs/openspec/changes/add-swarm-orchestra-platform/proposal.md](../openspec/changes/add-swarm-orchestra-platform/proposal.md), [docs/openspec/changes/add-swarm-orchestra-platform/design.md](../openspec/changes/add-swarm-orchestra-platform/design.md), and [docs/openspec/changes/add-swarm-orchestra-platform/tasks.md](../openspec/changes/add-swarm-orchestra-platform/tasks.md).

## Safety Boundary

Before new automation surfaces are added, two safety conditions apply:

- Authority startup must enforce the determinism gate.
- Governance and approval durations must be derived from the live 200ms runtime block target.

That safety boundary keeps validator correctness, approval workflow, and outward-facing automation from being conflated into one opaque system.
