# Panic Unwrap Audit

Generated: 2026-09-21T03:05:08Z by scripts/mainnet/panic_unwrap_audit.sh

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
| production | 1330 | 1330 |

files scanned: 1298

## Block-hook panics

None. No panic is reachable from a block hook.

## Production findings by file

-  254  crates/x3-atomic-swap/tests/atomic_swap_integration.rs
-   83  node/tests/x3vm_svm_live.rs
-   77  node/tests/x3vm_evm_live.rs
-   67  node/tests/x3vm_live_lifecycle.rs
-   56  crates/x3-atomic-swap/tests/atomic_swap_chaos.rs
-   51  crates/cross-vm-coordinator/tests/valkey_live.rs
-   35  pallets/pallet-x3-agent-registry/src/benchmarking.rs
-   29  pallets/x3-atomic-kernel/src/benchmarking.rs
-   23  pallets/agent-accounts/src/benchmarking.rs
-   23  crates/cross-vm-coordinator/tests/distributed_chaos.rs
-   19  runtime/tests/fraud_proofs_witness_v1.rs
-   19  crates/cross-vm-coordinator/src/persistence.rs
-   18  crates/x3-atomic-swap/tests/atlas_htlc_deploy_test.rs
-   17  pallets/governance/src/benchmarking.rs
-   17  crates/x3-bridge-adapters/src/lib.rs
-   17  crates/cross-chain-position-manager/tests/integration_tests.rs
-   16  runtime/src/lib.rs
-   16  crates/x3-compiler/src/parser.rs
-   16  crates/x3-gateway/tests/loom_mempool_concurrency.rs
-   14  pallets/treasury/src/benchmarking.rs
-   14  crates/quantum-swarm/src/quantum/circuit.rs
-   11  pallets/x3-inventory/src/benchmarking.rs
-   11  crates/x3-gpu-validator-swarm/tests/test_x3_validator.rs
-   11  crates/x3-vm/tests/gpu_integration.rs
-   10  crates/external-chains/src/evm_rpc.rs

## Verdict

PASS — no new panic/unwrap in consensus-critical code.
