# Panic Unwrap Audit

Generated: 2026-09-21T04:31:05Z by scripts/mainnet/panic_unwrap_audit.sh

## Classification

- **runtime-hook** — inside `on_initialize` / `on_finalize` / `offchain_worker`: a panic stops block production
- **pallet-call** — inside a `#[pallet::call]` extrinsic body: a panic fails the block that includes it
- **production** — every other line reachable in a production build

`#[cfg(test)]` items and commented-out lines are excluded: they do not
exist in the runtime or in a release node build.

## Counts

| class | current | baseline |
| --- | --- | --- |
| runtime-hook | 0 | 0 |
| pallet-call | 0 | 0 |
| production | 516 | 516 |

files scanned: 1299

## Block-hook panics

None. No panic is reachable from a block hook.

## Production findings by file

-   35  pallets/pallet-x3-agent-registry/src/benchmarking.rs
-   29  pallets/x3-atomic-kernel/src/benchmarking.rs
-   23  pallets/agent-accounts/src/benchmarking.rs
-   19  crates/cross-vm-coordinator/src/persistence.rs
-   17  pallets/governance/src/benchmarking.rs
-   17  crates/x3-bridge-adapters/src/lib.rs
-   16  runtime/src/lib.rs
-   16  crates/x3-compiler/src/parser.rs
-   14  pallets/treasury/src/benchmarking.rs
-   14  crates/quantum-swarm/src/quantum/circuit.rs
-   11  pallets/x3-inventory/src/benchmarking.rs
-   10  crates/external-chains/src/evm_rpc.rs
-   10  crates/x3-bot/src/telemetry.rs
-   10  crates/x3-rpc/src/wallet_service_rpc.rs
-   10  crates/cross-vm-coordinator/src/proof_vault.rs
-    9  crates/x3-dns-server/src/config.rs
-    8  crates/x3-mobile-sdk/src/biometric_auth_mobile.rs
-    8  crates/x3-backend/src/lower.rs
-    7  pallets/pallet-x3-proof-carrying-agent/src/benchmarking.rs
-    7  pallets/atomic-trade-engine/src/benchmarking.rs
-    6  crates/x3-gpu-validator-swarm/src/bin/x3_bench.rs
-    6  crates/cross-vm-coordinator/src/settlement_submission.rs
-    6  crates/cross-vm-coordinator/src/state_machine.rs
-    5  runtime/build.rs
-    5  crates/x3-gpu-validator-swarm/src/bin/x3_swarm_orchestrator.rs

## Verdict

PASS — no new panic/unwrap in consensus-critical code.
