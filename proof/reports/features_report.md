# X3 FeatureBuiltProof Report

## Verdict
**BLOCKED**

## Summary
| Status | Count |
|---|---:|
| BUILT | 0 |
| PARTIAL | 1 |
| MISSING | 37 |
| UNWIRED | 12 |
| UNTESTED | 7 |
| WEAK | 2 |
| STALE | 0 |
| BLOCKED | 0 |
| REVOKED | 0 |

## Top Blockers
1. x3.accounts: 1 code files missing
2. x3.accounts: 1 negative tests missing
3. x3.accounts: 2 tests missing
4. x3.accounts: proof receipt missing
5. x3.asset_kernel: 1 code files missing
6. x3.asset_kernel: 2 wiring checks failed
7. x3.asset_kernel: 3 negative tests missing
8. x3.asset_kernel: 3 tests missing
9. x3.asset_kernel: proof receipt missing
10. x3.asset_mapping: 1 code files missing

## Top 50 Missing/Partial/Unwired/Untested Features
| Feature | Area | Criticality | Status | Blockers |
|---|---|---|---|---|
| x3.accounts | system | catastrophic | MISSING | 1 code files missing, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.asset_mapping | asset-kernel | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.asset_registry | asset-kernel | catastrophic | MISSING | 1 code files missing, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.balances | balances | catastrophic | MISSING | 1 code files missing, 2 tests missing, 2 negative tests missing, proof receipt missing |
| x3.canonical_supply | asset-kernel | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.contracts.evm_core | contracts | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.contracts.security | contracts | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.contracts.upgrade | contracts | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.external_locked_accounting | asset-kernel | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.flashloan | flashloan | catastrophic | MISSING | 1 code files missing, 2 tests missing, 2 negative tests missing, proof receipt missing |
| x3.mint_burn_controller | asset-kernel | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 2 negative tests missing, proof receipt missing |
| x3.pending_transfer_accounting | asset-kernel | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.supply_invariant_guard | asset-kernel | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.vm_isolation | cross-vm | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.vm_state_transition | x3vm | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3lang_bytecode_generator | x3lang | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3lang_ir | x3lang | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3lang_parser | x3lang | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3lang_typechecker | x3lang | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3vm_bytecode | x3vm | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3vm_cpu_gpu_parity | gpu | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3vm_revert | x3vm | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3vm_storage | x3vm | catastrophic | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.block_production | consensus | catastrophic | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.consensus | consensus | catastrophic | UNWIRED | 2 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.contracts.evm_svm_parity | contracts | catastrophic | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt stale |
| x3.contracts.svm_core | contracts | catastrophic | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.finality | consensus | catastrophic | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.genesis_config | node | catastrophic | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.proofforge_cli | proofforge | catastrophic | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3lang_compiler | x3lang | catastrophic | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.asset_kernel | asset-kernel | catastrophic | UNTESTED | 1 code files missing, 2 wiring checks failed, 3 tests missing, 3 negative tests missing, proof receipt missing |
| x3.bridge | bridge | catastrophic | UNTESTED | 2 tests missing, 2 negative tests missing, proof receipt missing |
| x3.cross_vm_router | cross-vm | catastrophic | UNTESTED | 3 tests missing, 1 negative tests missing, proof receipt missing |
| x3.evm_adapter | evm | catastrophic | UNTESTED | 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.runtime | runtime | catastrophic | UNTESTED | 3 tests missing, 1 negative tests missing, proof receipt missing |
| x3.svm_adapter | svm | catastrophic | UNTESTED | 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3vm | x3vm | catastrophic | UNTESTED | 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.contracts.deployment | contracts | critical | MISSING | 2 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.contracts.event_schema | contracts | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.evm_precompiles | evm | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.fee_market | transaction-payment | critical | MISSING | 1 code files missing, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.svm_syscalls | svm | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.validator_set | staking | critical | MISSING | 1 code files missing, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.vm_fallback | cross-vm | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.vm_metering | cross-vm | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3lang_abi_generator | x3lang | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3vm_events | x3vm | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.x3vm_gas_metering | x3vm | critical | MISSING | 1 code files missing, 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |
| x3.contracts.shared_specs | contracts | critical | UNWIRED | 1 wiring checks failed, 2 tests missing, 1 negative tests missing, proof receipt missing |

## Built Features
| Feature | Area | Status | Exact Files Proving Build |
|---|---|---|---|

## Partial Features
| Feature | Blockers | Next Command |
|---|---|---|
| x3.dex | 2 tests missing, 1 negative tests missing, proof receipt missing | cargo test -p pallet-x3-dex |

## Missing Features
| Feature | Missing Code |
|---|---|
| x3.validator_set | pallets/staking/src/lib.rs |
| x3.fee_market | pallets/transaction-payment/src/lib.rs |
| x3.accounts | pallets/system/src/lib.rs |
| x3.balances | pallets/balances/src/lib.rs |
| x3.canonical_supply | pallets/x3-kernel/src/supply.rs |
| x3.asset_registry | pallets/x3-kernel/src/registry.rs |
| x3.asset_mapping | pallets/x3-kernel/src/mapping.rs |
| x3.mint_burn_controller | pallets/x3-kernel/src/mint_burn.rs |
| x3.external_locked_accounting | pallets/x3-kernel/src/external_locked.rs |
| x3.pending_transfer_accounting | pallets/x3-kernel/src/pending_transfer.rs |
| x3.supply_invariant_guard | pallets/x3-kernel/src/invariant.rs |
| x3.x3vm_bytecode | crates/x3-vm/src/bytecode.rs |
| x3.x3vm_gas_metering | crates/x3-vm/src/gas.rs |
| x3.x3vm_storage | crates/x3-vm/src/storage.rs |
| x3.x3vm_events | crates/x3-vm/src/events.rs |
| x3.x3vm_revert | crates/x3-vm/src/revert.rs |
| x3.x3vm_cpu_gpu_parity | crates/x3-vm/src/gpu.rs |
| x3.evm_precompiles | crates/evm-integration/src/precompiles.rs |
| x3.svm_syscalls | crates/svm-integration/src/syscalls.rs |
| x3.vm_isolation | crates/x3-vm/src/isolation.rs |
| x3.vm_state_transition | crates/x3-vm/src/state.rs |
| x3.vm_fallback | pallets/x3-cross-vm-router/src/fallback.rs |
| x3.vm_metering | pallets/x3-cross-vm-router/src/metering.rs |
| x3.x3lang_parser | crates/x3-compiler/src/parser.rs |
| x3.x3lang_typechecker | crates/x3-compiler/src/typechecker.rs |
| x3.x3lang_ir | crates/x3-compiler/src/ir.rs |
| x3.x3lang_optimizer | crates/x3-compiler/src/optimizer.rs |
| x3.x3lang_bytecode_generator | crates/x3-compiler/src/codegen.rs |
| x3.x3lang_abi_generator | crates/x3-compiler/src/abi.rs |
| x3.x3lang_contract_templates | templates/src/ |
| x3.contracts.evm_core | X3-contracts/evm/contracts/core/ |
| x3.contracts.deployment | X3-contracts/evm/script/deploy/, X3-contracts/svm/migrations/ |
| x3.contracts.upgrade | X3-contracts/evm/script/upgrade/ |
| x3.contracts.event_schema | X3-contracts/shared/schemas/ |
| x3.contracts.security | X3-contracts/evm/contracts/security/ |
| x3.flashloan | pallets/x3-flashloan/src/lib.rs |
| x3.dashboard | dashboard/src/ |

## Exact Commands Required To Prove Non-BUILT Features
| Feature | Status | Next Commands |
|---|---|---|
| x3.runtime | UNTESTED | cargo build -p x3-runtime ; cargo test -p x3-runtime ; x3-proof claim prove x3.runtime |
| x3.consensus | UNWIRED | cargo test consensus ; x3-proof claim prove x3.consensus |
| x3.finality | UNWIRED | cargo test finality ; x3-proof claim prove x3.finality |
| x3.validator_set | MISSING | cargo test -p pallet-staking ; x3-proof claim prove x3.validator_set |
| x3.block_production | UNWIRED | cargo test block_production ; x3-proof claim prove x3.block_production |
| x3.fee_market | MISSING | cargo test -p pallet-transaction-payment ; x3-proof claim prove x3.fee_market |
| x3.accounts | MISSING | cargo test -p frame-system ; x3-proof claim prove x3.accounts |
| x3.balances | MISSING | cargo test -p pallet-balances ; x3-proof claim prove x3.balances |
| x3.events | UNWIRED | cargo test events ; x3-proof claim prove x3.events |
| x3.errors | UNWIRED | cargo test errors ; x3-proof claim prove x3.errors |
| x3.genesis_config | UNWIRED | cargo test chain_spec ; x3-proof claim prove x3.genesis_config |
| x3.asset_kernel | UNTESTED | cargo test -p pallet-x3-kernel ; cargo test supply_conservation ; x3-proof claim prove x3.asset_kernel.supply_conservation ; x3-proof claim prove x3.asset_kernel |
| x3.canonical_supply | MISSING | cargo test canonical_supply ; x3-proof claim prove x3.canonical_supply |
| x3.asset_registry | MISSING | cargo test asset_registry ; x3-proof claim prove x3.asset_registry |
| x3.asset_mapping | MISSING | cargo test asset_mapping ; x3-proof claim prove x3.asset_mapping |
| x3.mint_burn_controller | MISSING | cargo test mint_burn ; x3-proof claim prove x3.mint_burn_controller |
| x3.external_locked_accounting | MISSING | cargo test external_locked ; x3-proof claim prove x3.external_locked_accounting |
| x3.pending_transfer_accounting | MISSING | cargo test pending_transfer ; x3-proof claim prove x3.pending_transfer_accounting |
| x3.supply_invariant_guard | MISSING | cargo test supply_invariant ; x3-proof claim prove x3.supply_invariant_guard |
| x3.x3vm | UNTESTED | cargo test -p x3-vm ; x3-proof claim prove x3.x3vm |
| x3.x3vm_bytecode | MISSING | cargo test bytecode ; x3-proof claim prove x3.x3vm_bytecode |
| x3.x3vm_gas_metering | MISSING | cargo test gas ; x3-proof claim prove x3.x3vm_gas_metering |
| x3.x3vm_storage | MISSING | cargo test storage ; x3-proof claim prove x3.x3vm_storage |
| x3.x3vm_events | MISSING | cargo test -p x3-vm events ; x3-proof claim prove x3.x3vm_events |
| x3.x3vm_revert | MISSING | cargo test revert ; x3-proof claim prove x3.x3vm_revert |
| x3.x3vm_cpu_gpu_parity | MISSING | cargo test cpu_gpu_parity ; x3-proof claim prove x3.x3vm_cpu_gpu_parity |
| x3.evm_adapter | UNTESTED | cargo test -p evm-integration ; x3-proof claim prove x3.evm_adapter |
| x3.svm_adapter | UNTESTED | cargo test -p svm-integration ; x3-proof claim prove x3.svm_adapter |
| x3.cross_vm_router | UNTESTED | cargo test -p pallet-x3-cross-vm-router ; x3-proof claim prove x3.cross_vm_router |
| x3.evm_precompiles | MISSING | cargo test precompiles ; x3-proof claim prove x3.evm_precompiles |
| x3.svm_syscalls | MISSING | cargo test syscalls ; x3-proof claim prove x3.svm_syscalls |
| x3.vm_isolation | MISSING | cargo test isolation ; x3-proof claim prove x3.vm_isolation |
| x3.vm_state_transition | MISSING | cargo test state_transition ; x3-proof claim prove x3.vm_state_transition |
| x3.vm_fallback | MISSING | cargo test fallback ; x3-proof claim prove x3.vm_fallback |
| x3.vm_metering | MISSING | cargo test metering ; x3-proof claim prove x3.vm_metering |
| x3.x3lang_parser | MISSING | cargo test -p x3-compiler parser ; x3-proof claim prove x3.x3lang_parser |
| x3.x3lang_typechecker | MISSING | cargo test typechecker ; x3-proof claim prove x3.x3lang_typechecker |
| x3.x3lang_compiler | UNWIRED | cargo test -p x3-compiler ; x3-proof claim prove x3.x3lang_compiler |
| x3.x3lang_ir | MISSING | cargo test ir ; x3-proof claim prove x3.x3lang_ir |
| x3.x3lang_optimizer | MISSING | cargo test optimizer ; x3-proof claim prove x3.x3lang_optimizer |
| x3.x3lang_bytecode_generator | MISSING | cargo test codegen ; x3-proof claim prove x3.x3lang_bytecode_generator |
| x3.x3lang_abi_generator | MISSING | cargo test abi ; x3-proof claim prove x3.x3lang_abi_generator |
| x3.x3lang_stdlib | UNWIRED | cargo test -p x3-stdlib ; x3-proof claim prove x3.x3lang_stdlib |
| x3.x3lang_contract_templates | MISSING | cargo test templates ; x3-proof claim prove x3.x3lang_contract_templates |
| x3.contracts.evm_core | MISSING | cd X3-contracts/evm && forge test ; x3-proof claim prove x3.contracts.evm_core |
| x3.contracts.svm_core | UNWIRED | cd X3-contracts/svm && anchor test ; x3-proof claim prove x3.contracts.svm_core |
| x3.contracts.shared_specs | UNWIRED | cargo test specs ; x3-proof claim prove x3.contracts.shared_specs |
| x3.contracts.evm_svm_parity | UNWIRED | cargo test parity |
| x3.contracts.deployment | MISSING | forge script deploy ; x3-proof claim prove x3.contracts.deployment |
| x3.contracts.upgrade | MISSING | forge script upgrade ; x3-proof claim prove x3.contracts.upgrade |
| x3.contracts.event_schema | MISSING | cargo test event_schema ; x3-proof claim prove x3.contracts.event_schema |
| x3.contracts.security | MISSING | forge test --match-path test/security ; x3-proof claim prove x3.contracts.security |
| x3.bridge | UNTESTED | cargo test -p x3-bridge ; x3-proof claim prove x3.bridge |
| x3.atomic_execution | WEAK | cargo test -p pallet-x3-atomic-kernel ; x3-proof claim prove x3.atomic_execution |
| x3.flashloan | MISSING | cargo test -p pallet-x3-flashloan ; x3-proof claim prove x3.flashloan |
| x3.dex | PARTIAL | cargo test -p pallet-x3-dex ; x3-proof claim prove x3.dex |
| x3.governance | WEAK | cargo test -p pallet-governance ; x3-proof claim prove x3.governance |
| x3.proofforge_cli | UNWIRED | cargo test -p proof-forge ; x3-proof claim prove x3.proofforge_cli |
| x3.dashboard | MISSING | npm test ; x3-proof claim prove x3.dashboard |

## Exact Blockers Preventing Completion
| Feature | Status | Blockers |
|---|---|---|
| x3.runtime | UNTESTED | 3 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.consensus | UNWIRED | 2 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.finality | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.validator_set | MISSING | 1 code files missing ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.block_production | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.fee_market | MISSING | 1 code files missing ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.accounts | MISSING | 1 code files missing ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.balances | MISSING | 1 code files missing ; 2 tests missing ; 2 negative tests missing ; proof receipt missing |
| x3.events | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.errors | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.genesis_config | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.asset_kernel | UNTESTED | 1 code files missing ; 2 wiring checks failed ; 3 tests missing ; 3 negative tests missing ; proof receipt missing |
| x3.canonical_supply | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.asset_registry | MISSING | 1 code files missing ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.asset_mapping | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.mint_burn_controller | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 2 negative tests missing ; proof receipt missing |
| x3.external_locked_accounting | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.pending_transfer_accounting | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.supply_invariant_guard | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3vm | UNTESTED | 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3vm_bytecode | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3vm_gas_metering | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3vm_storage | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3vm_events | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3vm_revert | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3vm_cpu_gpu_parity | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.evm_adapter | UNTESTED | 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.svm_adapter | UNTESTED | 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.cross_vm_router | UNTESTED | 3 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.evm_precompiles | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.svm_syscalls | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.vm_isolation | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.vm_state_transition | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.vm_fallback | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.vm_metering | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_parser | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_typechecker | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_compiler | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_ir | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_optimizer | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_bytecode_generator | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_abi_generator | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_stdlib | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.x3lang_contract_templates | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.contracts.evm_core | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.contracts.svm_core | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.contracts.shared_specs | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.contracts.evm_svm_parity | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt stale |
| x3.contracts.deployment | MISSING | 2 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.contracts.upgrade | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.contracts.event_schema | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.contracts.security | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.bridge | UNTESTED | 2 tests missing ; 2 negative tests missing ; proof receipt missing |
| x3.atomic_execution | WEAK | 2 tests missing ; 2 negative tests missing ; proof receipt missing |
| x3.flashloan | MISSING | 1 code files missing ; 2 tests missing ; 2 negative tests missing ; proof receipt missing |
| x3.dex | PARTIAL | 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.governance | WEAK | 2 tests missing ; 2 negative tests missing ; proof receipt missing |
| x3.proofforge_cli | UNWIRED | 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |
| x3.dashboard | MISSING | 1 code files missing ; 1 wiring checks failed ; 2 tests missing ; 1 negative tests missing ; proof receipt missing |

## Burn-Down Order By Criticality
| Order | Feature | Criticality | Status | Required For |
|---:|---|---|---|---|
| 1 | x3.accounts | catastrophic | MISSING | mainnet |
| 2 | x3.asset_mapping | catastrophic | MISSING | mainnet |
| 3 | x3.asset_registry | catastrophic | MISSING | mainnet |
| 4 | x3.balances | catastrophic | MISSING | mainnet |
| 5 | x3.canonical_supply | catastrophic | MISSING | mainnet |
| 6 | x3.contracts.evm_core | catastrophic | MISSING | mainnet |
| 7 | x3.contracts.security | catastrophic | MISSING | mainnet |
| 8 | x3.contracts.upgrade | catastrophic | MISSING | mainnet |
| 9 | x3.external_locked_accounting | catastrophic | MISSING | mainnet |
| 10 | x3.flashloan | catastrophic | MISSING | mainnet |
| 11 | x3.mint_burn_controller | catastrophic | MISSING | mainnet |
| 12 | x3.pending_transfer_accounting | catastrophic | MISSING | mainnet |
| 13 | x3.supply_invariant_guard | catastrophic | MISSING | mainnet |
| 14 | x3.vm_isolation | catastrophic | MISSING | mainnet |
| 15 | x3.vm_state_transition | catastrophic | MISSING | mainnet |
| 16 | x3.x3lang_bytecode_generator | catastrophic | MISSING | mainnet |
| 17 | x3.x3lang_ir | catastrophic | MISSING | mainnet |
| 18 | x3.x3lang_parser | catastrophic | MISSING | mainnet |
| 19 | x3.x3lang_typechecker | catastrophic | MISSING | mainnet |
| 20 | x3.x3vm_bytecode | catastrophic | MISSING | mainnet |
| 21 | x3.x3vm_cpu_gpu_parity | catastrophic | MISSING | mainnet |
| 22 | x3.x3vm_revert | catastrophic | MISSING | mainnet |
| 23 | x3.x3vm_storage | catastrophic | MISSING | mainnet |
| 24 | x3.block_production | catastrophic | UNWIRED | mainnet |
| 25 | x3.consensus | catastrophic | UNWIRED | mainnet |
| 26 | x3.contracts.evm_svm_parity | catastrophic | UNWIRED | mainnet |
| 27 | x3.contracts.svm_core | catastrophic | UNWIRED | mainnet |
| 28 | x3.finality | catastrophic | UNWIRED | mainnet |
| 29 | x3.genesis_config | catastrophic | UNWIRED | mainnet |
| 30 | x3.proofforge_cli | catastrophic | UNWIRED | mainnet |
| 31 | x3.x3lang_compiler | catastrophic | UNWIRED | mainnet |
| 32 | x3.asset_kernel | catastrophic | UNTESTED | mainnet |
| 33 | x3.bridge | catastrophic | UNTESTED | mainnet |
| 34 | x3.cross_vm_router | catastrophic | UNTESTED | mainnet |
| 35 | x3.evm_adapter | catastrophic | UNTESTED | mainnet |
| 36 | x3.runtime | catastrophic | UNTESTED | mainnet |
| 37 | x3.svm_adapter | catastrophic | UNTESTED | mainnet |
| 38 | x3.x3vm | catastrophic | UNTESTED | mainnet |
| 39 | x3.atomic_execution | catastrophic | WEAK | mainnet |
| 40 | x3.governance | catastrophic | WEAK | mainnet |
| 41 | x3.contracts.deployment | critical | MISSING | mainnet |
| 42 | x3.contracts.event_schema | critical | MISSING | mainnet |
| 43 | x3.evm_precompiles | critical | MISSING | mainnet |
| 44 | x3.fee_market | critical | MISSING | mainnet |
| 45 | x3.svm_syscalls | critical | MISSING | mainnet |
| 46 | x3.validator_set | critical | MISSING | mainnet |
| 47 | x3.vm_fallback | critical | MISSING | mainnet |
| 48 | x3.vm_metering | critical | MISSING | mainnet |
| 49 | x3.x3lang_abi_generator | critical | MISSING | mainnet |
| 50 | x3.x3vm_events | critical | MISSING | mainnet |
| 51 | x3.x3vm_gas_metering | critical | MISSING | mainnet |
| 52 | x3.contracts.shared_specs | critical | UNWIRED | mainnet |
| 53 | x3.errors | critical | UNWIRED | mainnet |
| 54 | x3.events | critical | UNWIRED | mainnet |
| 55 | x3.x3lang_stdlib | critical | UNWIRED | mainnet |
| 56 | x3.dex | critical | PARTIAL | mainnet |
| 57 | x3.dashboard | high | MISSING | testnet |
| 58 | x3.x3lang_optimizer | high | MISSING | testnet |
| 59 | x3.x3lang_contract_templates | medium | MISSING | testnet |

