# X3 Drift Report

- Changed/staged/untracked paths: 371
- Docs: 87
- Code: 29
- Tests: 6
- Danger-zone paths: 49

## DANGER ZONE CHANGES
- crates/cross-vm-bridge/bridge/finality.rs
- crates/cross-vm-bridge/tests/finality_verification_tests.rs
- crates/cross-vm-coordinator/Cargo.toml
- crates/parallel-proposer/Cargo.toml
- crates/x3-atomic-client/Cargo.toml
- crates/x3-atomic-trade/Cargo.toml
- crates/x3-bridge-adapters/Cargo.toml
- crates/x3-bridge/Cargo.toml
- crates/x3-liquidity-core/Cargo.toml
- crates/x3-liquidity-core/src/anti_rug.rs
- crates/x3-liquidity-core/src/lib.rs
- crates/x3-vrf/Cargo.toml
- launch-gates/GENESIS_CEREMONY_RUNBOOK.md
- launch-gates/PROOF_EXECUTION_REPORT.md
- launch-gates/S0_BLOCKER_PRIORITIZATION.md
- launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-201255.md
- launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-203300.md
- node/Cargo.toml
- pallets/cross-chain-validator/Cargo.toml
- pallets/governance/src/lib.rs
- pallets/svm-runtime/src/lib.rs
- pallets/svm-runtime/src/weights.rs
- pallets/x3-account-registry/src/lib.rs
- pallets/x3-atomic-kernel/Cargo.toml
- pallets/x3-atomic-kernel/src/lib.rs
- pallets/x3-coin/src/lib.rs
- pallets/x3-consensus/Cargo.toml
- pallets/x3-consensus/src/lib.rs
- pallets/x3-cross-vm-router/Cargo.toml
- pallets/x3-da/Cargo.toml
- pallets/x3-jury-anchor/Cargo.toml
- pallets/x3-sequencer/Cargo.toml
- pallets/x3-settlement-engine/Cargo.toml
- pallets/x3-settlement-engine/src/atomic_lock.rs
- pallets/x3-settlement-engine/src/btc_gateway.rs
- pallets/x3-settlement-engine/src/collateral.rs
- pallets/x3-settlement-engine/src/escrow.rs
- pallets/x3-settlement-engine/src/finality.rs
- pallets/x3-settlement-engine/src/intent.rs
- pallets/x3-settlement-engine/src/invariants.rs
- pallets/x3-settlement-engine/src/lib.rs
- pallets/x3-settlement-engine/src/runtime_api.rs
- pallets/x3-settlement-engine/src/types.rs
- pallets/x3-slash/Cargo.toml
- runtime/Cargo.toml
- runtime/src/fraud_proofs/freeze.rs
- runtime/src/fraud_proofs/types.rs
- runtime/src/fraud_proofs/witness_v1.rs
- runtime/src/lib.rs

Required: audit note, tests, rollback plan, and risk register update.

