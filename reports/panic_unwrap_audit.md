# Panic Unwrap Audit

Generated: 2026-09-20T11:54:10Z by scripts/mainnet/panic_unwrap_audit.sh

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
| production | 2269 | 2269 |

files scanned: 1298

## Block-hook panics

None. No panic is reachable from a block hook.

## Production findings by file

-  254  crates/x3-atomic-swap/tests/atomic_swap_integration.rs
-  104  pallets/x3-settlement-engine/src/tests.rs
-   88  pallets/pallet-x3-agent-registry/src/tests.rs
-   83  node/tests/x3vm_svm_live.rs
-   80  pallets/x3-cross-vm-router/src/tests.rs
-   77  node/tests/x3vm_evm_live.rs
-   67  node/tests/x3vm_live_lifecycle.rs
-   67  crates/cross-vm-coordinator/src/tests.rs
-   56  crates/x3-atomic-swap/tests/atomic_swap_chaos.rs
-   55  pallets/agent-accounts/src/tests.rs
-   51  crates/cross-vm-coordinator/tests/valkey_live.rs
-   37  pallets/governance/src/tests.rs
-   35  pallets/pallet-x3-agent-registry/src/benchmarking.rs
-   32  pallets/treasury/src/tests.rs
-   30  pallets/x3-token-factory/src/tests.rs
-   29  pallets/x3-atomic-kernel/src/benchmarking.rs
-   27  pallets/x3-inventory/src/tests.rs
-   23  pallets/x3-kernel/src/tests.rs
-   23  pallets/agent-accounts/src/benchmarking.rs
-   23  crates/cross-vm-coordinator/tests/distributed_chaos.rs
-   22  pallets/agent-memory/src/tests.rs
-   21  pallets/pallet-x3-proof-carrying-agent/src/tests.rs
-   20  pallets/x3-solvency/src/integration_tests.rs
-   20  pallets/x3-atomic-kernel/src/tests.rs
-   19  runtime/tests/fraud_proofs_witness_v1.rs

## Verdict

PASS — no new panic/unwrap in consensus-critical code.
