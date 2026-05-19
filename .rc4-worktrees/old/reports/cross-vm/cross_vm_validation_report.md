# Cross-VM Validation Report

Date: 2026-05-12
Branch: main
Scope: targeted cross-VM, atomic swap, cross-chain GPU validator, and live local RPC validation from the current workspace state.

## Verdict

PARTIAL PASS / RUNTIME BLOCKED

The local Python-level cross-VM and cross-chain atomic validation probes now pass. The live local validator network is healthy and can finalize a harmless `system.remark` extrinsic, but the currently runnable fallback binary does not expose the current atomic/cross-VM RPC surface. Rust pallet/package tests remain blocked before logic execution by Rust compiler/native dependency failures already seen during RC1 source-build work.

## Live Chain Probe

Command artifacts:

- `reports/cross-vm/cross-vm-harness-nosubmit.log`
- `reports/cross-vm/cross-vm-harness-submit.log`

Observed:

- `system_health` returned `peers: 2`, `isSyncing: False` on `http://127.0.0.1:9944`.
- `system.remark` submit path finalized successfully at block hash `0xdfbc148079b90e777fe4938e01aee7b7bb249529dac675796c0c767799462b1f`.
- Atomic/cross-VM RPC methods returned JSON-RPC `-32601 Method not found`:
  - `atlasKernel_getAssetMetadata`
  - `atomicTrade_simulate`
  - `atomicTrade_estimateCost`
  - `atomicTrade_getPriceData`
- Polkadot JS also reported missing/decorated RPCs including `x3_submitCrossVmTransaction`, `x3_submitSvmTransaction`, `x3_submitX3vmTransaction`, and atomic trade methods.

Interpretation: networking, accounts, WS/RPC, and finalization are working on the fallback binary. Current cross-VM runtime/RPC behavior cannot be exercised on this binary.

## Python Validation

Environment fixes applied locally:

- Installed `pytest`, `pynacl`, and `redis` into `.venv` so the Python probe layer could actually run.
- Fixed mirrored atomic cross-VM test coordinators to raise explicit `RuntimeError` for invalid settle-phase transitions instead of relying on `assert`.
- Updated mirrored cross-chain GPU invariant tests to match the current `CrossChainOrchestrator` compatibility API.
- Fixed `scripts/testnet/submit-remark.js` for repo ESM mode and finalization waiting.

Passing tests:

- `tests_core/p4_atomic_crossvm_testnet.py`: 61 passed.
- `tests_phase4/p4_atomic_crossvm_testnet.py`: 61 passed.
- `tests_core/p4_p5_crosschain_gpu_validator.py`: 78 passed.
- `tests_phase4/p4_p5_crosschain_gpu_validator.py`: 78 passed.
- `tests_core/cross_chain_gpu_validator/test_atomic_invariant.py`: 2 passed.
- `tests_phase4/cross_chain_gpu_validator/test_atomic_invariant.py`: 2 passed.

Total targeted Python assertions: 282 passed.

Additional checks:

- Python probe syntax compile passed.
- `node --check` passed for `scripts/testnet/submit-remark.js` and `scripts/testnet/subkey-js-shim.cjs`.
- `bash -n` passed for `scripts/mainnet/rc1_smoke_test.sh`.

## Rust Validation

Attempted targeted package tests:

- `pallet-x3-account-registry`
- `pallet-x3-asset-registry`
- `pallet-x3-atomic-kernel`
- `pallet-x3-cross-vm-router`
- `pallet-x3-settlement-engine`
- `x3-ixl`
- `x3-proof`
- `x3-vm`

Result: all targeted Rust test attempts exited before cross-VM logic execution due compiler/native dependency failures, consistent with the source-build blocker documented in the RC1 report.

Known failure classes from logs:

- `rustc` SIGSEGV / signal 11 during dependency compilation.
- `could not compile libsecp256k1`.
- `could not compile serde_core`.

Interpretation: these are build-chain blockers, not confirmed cross-VM logic failures.

## Test Inventory

Additional Rust cross-VM test surfaces were inventoried in `reports/cross-vm/rust-crossvm-test-inventory.log`, including:

- `tests/atomic/cross_vm_rollback_test.rs`
- `tests/e2e_settlement_atomic_kernel.rs`
- `tests/e2e/cross_vm_real_chain_test.rs`
- `tests/contracts/evm_svm_parity_test.rs`
- `integration-tests/cross-vm-pallet-test.rs`
- `integration-tests/cross-vm-atomic-test.rs`

## Remaining Blockers

1. Fresh source-built node/runtime is still required to validate current cross-VM pallets and RPCs end to end.
2. The fallback binary can prove consensus/finality and basic extrinsics, but not current atomic/cross-VM RPC semantics.
3. Rust cargo tests cannot currently reach logic execution because compiler/native dependency failures occur first.

## Next Order Of Operations

1. Stabilize the Rust build toolchain/native dependency path until `x3-chain-node` and targeted Rust packages compile reliably.
2. Boot a fresh local3 network from the current runtime, not the fallback binary.
3. Re-run the live harness and expect atomic/cross-VM RPCs to return structured success/error responses instead of `Method not found`.
4. Run the targeted Rust package tests and integration tests after the build blocker is cleared.
5. Promote the same validation flow into CI once deterministic locally.
