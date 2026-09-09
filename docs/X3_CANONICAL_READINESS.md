# Canonical Readiness

**Generated from:** `FEATURE_REGISTRY.toml`
**Reviewed:** 2026-09-08
**Average:** 51.07% across 15 implemented features

This page is a readable snapshot. `FEATURE_REGISTRY.toml` remains authoritative. If a row differs from the registry, use the registry and correct this page.

| Feature | Mode | Score |
|---|---|---:|
| Atomic router | LIVE_TESTNET | 88% |
| AXE DEX | GUARDED_TESTNET | 75% |
| X3 Forge | GUARDED_TESTNET | 75% |
| Atomic lock | LIVE_TESTNET | 68% |
| Triforge runtime | GUARDED_TESTNET | 65% |
| Atomic gateway | GUARDED_TESTNET | 65% |
| Launch gate | LIVE_TESTNET | 55% |
| Wallet pallet | LIVE_TESTNET | 55% |
| X3 Sentinel | GUARDED_TESTNET | 50% |
| Atomic kernel | LIVE_TESTNET | 40% |
| X3 Reactor | LIVE_TESTNET | 40% |
| Bitcoin gateway | SIM_TESTNET | 25% |
| X3 Swarm Core | GUARDED_TESTNET | 25% |
| Repository scanner | LIVE_TESTNET | 25% |
| Tauri OS | GUARDED_TESTNET | 15% |

## Interpretation

- `LIVE_TESTNET` means implemented testnet code. It does not mean production-ready.
- `GUARDED_TESTNET` means the implementation is restricted by runtime, feature, governance, or operational gates.
- `SIM_TESTNET` means simulation or regtest evidence only.
- External bridge activation remains disabled.
- Internal cross-VM routing is tracked separately from production external cross-chain transfer.
- On 2026-09-09, GitHub Actions reruns began executing normal steps after the earlier billing lock was cleared. The workflow configuration cannot be presented as green until those runs pass.

See `GRANT_READINESS.md`, `docs/CROSS_CHAIN_READINESS.md`, and `docs/current/FAILURES_AND_TODOS.md` for evidence and blockers.
