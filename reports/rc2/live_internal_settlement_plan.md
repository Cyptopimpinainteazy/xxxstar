# X3 Atomic Star - RC2 Live Internal Settlement Plan

## Verdict

PLANNED / BLOCKED BY RC1 RUNTIME BINARY

RC2 is the live internal settlement proof. RC1 proves the chain can boot, peer, produce blocks, finalize, expose the expected runtime pallet surface, and keep external bridges disabled. RC2 proves the internal settlement path moves value correctly across X3Native, X3Evm, and X3Svm domains without enabling external bridge behavior.

## RC2 Goal

Prove six live internal settlement routes on a local/internal validator network:

| Route | Required Proof |
|---|---|
| X3Native -> X3Evm | debit source, credit destination, emit settlement event |
| X3Evm -> X3Native | debit source, credit destination, emit settlement event |
| X3Native -> X3Svm | debit source, credit destination, emit settlement event |
| X3Svm -> X3Native | debit source, credit destination, emit settlement event |
| X3Evm -> X3Svm | route through internal settlement only, no external bridge |
| X3Svm -> X3Evm | route through internal settlement only, no external bridge |

## Required Preconditions

- RC1 local3 passes on a fresh current `x3-chain-node` binary.
- Runtime metadata exposes `X3SupplyLedger`, `X3CrossVmRouter`, `X3AssetRegistry`, `X3AccountRegistry`, `X3AtomicKernel`, and `X3SettlementEngine`.
- One canonical X3 asset is registered and usable in all three internal domains.
- Test accounts are funded for all route sources and destinations.
- External bridge storage/config remains disabled.

## Required Evidence

| Proof | Status | Evidence Target |
|---|---:|---|
| Fresh node binary built | PENDING | `reports/rc2/node_build.log` |
| local3 boots from current runtime | PENDING | `reports/rc2/boot_local3.log` |
| Metadata exposes settlement pallets | PENDING | `reports/rc2/runtime_metadata.hex` |
| Canonical X3 asset registered | PENDING | `reports/rc2/asset_registry.json` |
| Initial balances captured | PENDING | `reports/rc2/balances_before.json` |
| Native -> EVM route settles | PENDING | `reports/rc2/route_native_to_evm.json` |
| EVM -> Native route settles | PENDING | `reports/rc2/route_evm_to_native.json` |
| Native -> SVM route settles | PENDING | `reports/rc2/route_native_to_svm.json` |
| SVM -> Native route settles | PENDING | `reports/rc2/route_svm_to_native.json` |
| EVM -> SVM route settles | PENDING | `reports/rc2/route_evm_to_svm.json` |
| SVM -> EVM route settles | PENDING | `reports/rc2/route_svm_to_evm.json` |
| Final balances reconcile | PENDING | `reports/rc2/balances_after.json` |
| Settlement events emitted | PENDING | `reports/rc2/events.json` |
| External bridges remain disabled | PENDING | `reports/rc2/external_bridges_disabled.json` |

## Pass Criteria

RC2 passes only if all six routes complete on-chain with finalized inclusion, expected balance deltas, expected settlement events, and no external bridge activation.

## Current Test Coverage Already Exercised

The Python-level cross-VM validation layer has been exercised and currently passes targeted probes:

- `tests_core/p4_atomic_crossvm_testnet.py`
- `tests_phase4/p4_atomic_crossvm_testnet.py`
- `tests_core/p4_p5_crosschain_gpu_validator.py`
- `tests_phase4/p4_p5_crosschain_gpu_validator.py`
- `tests_core/cross_chain_gpu_validator/test_atomic_invariant.py`
- `tests_phase4/cross_chain_gpu_validator/test_atomic_invariant.py`

The live local fallback binary can finalize a harmless `system.remark`, but it does not expose the current atomic/cross-VM RPC surface. RC2 therefore remains blocked until RC1 is green on a fresh current runtime binary.

## Next Implementation Step

After RC1 is green, add `scripts/mainnet/rc2_internal_settlement_smoke.sh` or an equivalent checked-in harness that:

1. captures initial balances,
2. submits each route,
3. waits for finalized inclusion,
4. captures events and final balances,
5. verifies exact deltas,
6. writes structured evidence to `reports/rc2/`, and
7. exits non-zero on any missing route, missing event, bad delta, or bridge-enabled state.
