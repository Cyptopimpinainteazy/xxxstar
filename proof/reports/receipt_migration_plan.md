# Receipt Migration Plan

Generated: 2026-04-27T22:08:50Z

Legacy receipts require full re-proof to produce cryptographically bound ProofForge receipts.
This script does not fabricate proof data. It generates a re-proof action plan only.

## Legacy Receipts

| Claim ID | Legacy Verifier | Legacy Date | Re-proof Command |
|---|---|---|---|
| x3.asset_kernel.supply_conservation | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.asset_kernel.supply_conservation --strict |
| x3.atomic.one_terminal_state | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.atomic.one_terminal_state --strict |
| x3.atomic.rollback_safety | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.atomic.rollback_safety --strict |
| x3.bridge.finality_verification | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.bridge.finality_verification --strict |
| x3.bridge.replay_protection | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.bridge.replay_protection --strict |
| x3.contracts.evm_svm_parity | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.contracts.evm_svm_parity --strict |
| x3.evolution.no_regression | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.evolution.no_regression --strict |
| x3.flashloan.repay_or_revert | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.flashloan.repay_or_revert --strict |
| x3.funding.milestone_receipts | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.funding.milestone_receipts --strict |
| x3.governance.proof_gated_upgrade | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.governance.proof_gated_upgrade --strict |
| x3.gpu.cpu_gpu_parity | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.gpu.cpu_gpu_parity --strict |
| x3.onboarding.developer_first_value | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.onboarding.developer_first_value --strict |
| x3.proofforge.receipt_integrity | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.proofforge.receipt_integrity --strict |
| x3.x3lang.compiler_reproducibility | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.x3lang.compiler_reproducibility --strict |
| x3.x3vm.determinism | x3-proof-2400b15 | 2026-04-27 | cargo run -p proof-forge -- verify x3.x3vm.determinism --strict |

## Next Steps

1. Run each re-proof command in a clean workspace state.
2. Ensure commands regenerate structured receipts under proof/receipts/claims.
3. Run scripts/proof/verify-receipts.sh and resolve all failures.
