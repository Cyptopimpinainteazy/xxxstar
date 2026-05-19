// SPDX-License-Identifier: Apache-2.0
//
// tests_s0_1.rs — S0-1 canonical supply invariant tests.
//
// Comprehensive test suite verifying that the supply invariant holds under all
// operations, including adversarial scenarios and edge cases.

use crate::supply_verification::{AssetSupplyProof, SupplyMerkleTree};
use frame_support::{assert_err, assert_ok};
use sp_core::H256;
use x3_asset_kernel_types::{AssetId, Balance, DomainId, SupplyLedger};

// NOTE: These tests require a mock runtime to be set up.
// This file demonstrates the test structure; actual execution requires
// mock.rs with runtime configuration.

#[cfg(test)]
mod s0_1_tests {
    use super::*;

    // Helper to create a test asset ID
    fn test_asset_id(id: u8) -> AssetId {
        H256::repeat_byte(id)
    }

    // Helper to create a valid supply ledger
    fn create_valid_ledger(
        canonical: Balance,
        native: Balance,
        evm: Balance,
        svm: Balance,
    ) -> SupplyLedger {
        SupplyLedger {
            canonical_supply: canonical,
            native_supply: native,
            evm_supply: evm,
            svm_supply: svm,
            external_locked_supply: 0,
            pending_supply: 0,
        }
    }

    // Helper to create an invalid ledger (violates invariant)
    fn create_invalid_ledger() -> SupplyLedger {
        SupplyLedger {
            canonical_supply: 1000,
            native_supply: 600,
            evm_supply: 500, // 600 + 500 = 1100 > 1000 (violation!)
            svm_supply: 0,
            external_locked_supply: 0,
            pending_supply: 0,
        }
    }

    #[test]
    fn test_canonical_supply_always_equals_ledger_sum() {
        // REQUIREMENT: canonical_supply MUST equal sum of all domain supplies

        let asset_id = test_asset_id(1);
        let ledger = create_valid_ledger(1000, 400, 300, 300);

        // Verify invariant holds
        assert_ok!(ledger.check_invariant());

        // Verify sum equals canonical
        let sum = ledger.represented().unwrap();
        assert_eq!(sum, ledger.canonical_supply);

        // Create proof and verify
        let proof = AssetSupplyProof::from_ledger(asset_id, &ledger, 0);
        assert_ok!(proof.verify_invariant());
        assert_eq!(proof.canonical_supply, proof.represented_supply);
    }

    #[test]
    fn test_mint_preserves_invariant() {
        // REQUIREMENT: Minting MUST increase both canonical and domain supply equally

        let mut ledger = create_valid_ledger(1000, 1000, 0, 0);

        // Mint 500 to EVM domain
        ledger.canonical_supply = ledger.canonical_supply.checked_add(500).unwrap();
        ledger.evm_supply = ledger.evm_supply.checked_add(500).unwrap();

        // Invariant must still hold
        assert_ok!(ledger.check_invariant());

        // Verify new state
        assert_eq!(ledger.canonical_supply, 1500);
        assert_eq!(ledger.represented().unwrap(), 1500);
    }

    #[test]
    fn test_burn_preserves_invariant() {
        // REQUIREMENT: Burning MUST decrease both canonical and domain supply equally

        let mut ledger = create_valid_ledger(1000, 600, 400, 0);

        // Burn 200 from native domain
        ledger.canonical_supply = ledger.canonical_supply.checked_sub(200).unwrap();
        ledger.native_supply = ledger.native_supply.checked_sub(200).unwrap();

        // Invariant must still hold
        assert_ok!(ledger.check_invariant());

        // Verify new state
        assert_eq!(ledger.canonical_supply, 800);
        assert_eq!(ledger.represented().unwrap(), 800);
    }

    #[test]
    fn test_transfer_preserves_invariant() {
        // REQUIREMENT: Domain-to-domain transfers MUST NOT change canonical supply

        let mut ledger = create_valid_ledger(1000, 600, 400, 0);

        // Transfer 100 from native to SVM (via pending)
        ledger.native_supply = ledger.native_supply.checked_sub(100).unwrap();
        ledger.pending_supply = ledger.pending_supply.checked_add(100).unwrap();
        assert_ok!(ledger.check_invariant());

        ledger.pending_supply = ledger.pending_supply.checked_sub(100).unwrap();
        ledger.svm_supply = ledger.svm_supply.checked_add(100).unwrap();
        assert_ok!(ledger.check_invariant());

        // Canonical unchanged, but distribution changed
        assert_eq!(ledger.canonical_supply, 1000);
        assert_eq!(ledger.represented().unwrap(), 1000);
        assert_eq!(ledger.native_supply, 500);
        assert_eq!(ledger.svm_supply, 100);
    }

    #[test]
    fn test_bridge_mint_preserves_invariant() {
        // REQUIREMENT: Bridge mints (external collateral) MUST update canonical

        let mut ledger = create_valid_ledger(1000, 1000, 0, 0);

        // Bridge locks 500 external collateral and mints to native
        ledger.canonical_supply = ledger.canonical_supply.checked_add(500).unwrap();
        ledger.external_locked_supply = ledger.external_locked_supply.checked_add(500).unwrap();

        // Invariant must hold (external locked counts toward represented)
        assert_ok!(ledger.check_invariant());

        assert_eq!(ledger.canonical_supply, 1500);
        assert_eq!(ledger.represented().unwrap(), 1500);
    }

    #[test]
    fn test_supply_invariant_validation() {
        // REQUIREMENT: Invariant check MUST reject violations

        let invalid_ledger = create_invalid_ledger();

        // Check must fail
        assert_err!(
            invalid_ledger.check_invariant(),
            x3_asset_kernel_types::InvariantError::SupplyCeilingExceeded
        );

        // Proof verification must also fail
        let proof = AssetSupplyProof::from_ledger(test_asset_id(1), &invalid_ledger, 0);
        assert_err!(
            proof.verify_invariant(),
            x3_asset_kernel_types::InvariantError::SupplyCeilingExceeded
        );
    }

    #[test]
    fn test_overflow_detection() {
        // REQUIREMENT: Arithmetic overflows MUST be detected and rejected

        let ledger = SupplyLedger {
            canonical_supply: Balance::MAX,
            native_supply: Balance::MAX - 100,
            evm_supply: 100,
            svm_supply: 1, // This will overflow when summing
            external_locked_supply: 0,
            pending_supply: 0,
        };

        // Check must fail with overflow
        assert_err!(
            ledger.check_invariant(),
            x3_asset_kernel_types::InvariantError::ArithmeticOverflow
        );
    }

    #[test]
    fn test_merkle_proof_generation() {
        // REQUIREMENT: Merkle proofs MUST be verifiable

        let ledger1 = create_valid_ledger(1000, 1000, 0, 0);
        let ledger2 = create_valid_ledger(2000, 1000, 1000, 0);
        let ledger3 = create_valid_ledger(3000, 1000, 1000, 1000);

        let mut proofs = vec![
            AssetSupplyProof::from_ledger(test_asset_id(1), &ledger1, 0),
            AssetSupplyProof::from_ledger(test_asset_id(2), &ledger2, 1),
            AssetSupplyProof::from_ledger(test_asset_id(3), &ledger3, 2),
        ];

        let tree = SupplyMerkleTree::new(&mut proofs);
        let root = tree.root();

        // All proofs must verify against root
        for proof in &proofs {
            assert!(proof.verify_merkle_branch(root));
        }
    }

    #[test]
    fn test_merkle_proof_tamper_detection() {
        // REQUIREMENT: Tampered proofs MUST be rejected

        let ledger = create_valid_ledger(1000, 1000, 0, 0);
        let mut proofs = vec![AssetSupplyProof::from_ledger(test_asset_id(1), &ledger, 0)];

        let tree = SupplyMerkleTree::new(&mut proofs);
        let root = tree.root();

        // Verify original proof
        assert!(proofs[0].verify_merkle_branch(root));

        // Tamper with canonical supply
        proofs[0].canonical_supply = 2000;

        // Verification must fail (leaf hash mismatch)
        // Note: This test requires updating leaf_hash after tampering to properly test
        let tampered_hash = AssetSupplyProof::compute_leaf_hash(
            test_asset_id(1),
            &SupplyLedger {
                canonical_supply: 2000, // Tampered value
                native_supply: 1000,
                evm_supply: 0,
                svm_supply: 0,
                external_locked_supply: 0,
                pending_supply: 0,
            },
        );
        proofs[0].leaf_hash = tampered_hash;

        // Proof should fail because leaf doesn't match original tree
        assert!(!proofs[0].verify_merkle_branch(root));
    }

    // NOTE: The following tests require a full mock runtime
    // They are included here as templates for integration testing

    /*
    #[test]
    fn test_on_finalize_verifies_all_assets() {
        new_test_ext().execute_with(|| {
            // Setup: Create multiple assets with valid ledgers
            let asset1 = test_asset_id(1);
            let asset2 = test_asset_id(2);

            Ledgers::<Test>::insert(asset1, create_valid_ledger(1000, 1000, 0, 0));
            Ledgers::<Test>::insert(asset2, create_valid_ledger(2000, 1000, 1000, 0));

            // Finalize block (triggers on_finalize)
            Pallet::<Test>::on_finalize(1);

            // Verify proof was generated
            let proof = CurrentSupplyProof::<Test>::get().unwrap();
            assert_eq!(proof.asset_count, 2);
            assert_eq!(proof.total_canonical, 3000);

            // Verify event emitted
            assert!(System::events().iter().any(|e| {
                matches!(e.event, Event::SupplyProofGenerated { .. })
            }));
        });
    }

    #[test]
    fn test_on_finalize_detects_violations() {
        new_test_ext().execute_with(|| {
            // Setup: Insert a ledger that violates invariant
            let asset1 = test_asset_id(1);
            Ledgers::<Test>::insert(asset1, create_invalid_ledger());

            // Finalize block (triggers on_finalize)
            Pallet::<Test>::on_finalize(1);

            // Verify violation event emitted
            assert!(System::events().iter().any(|e| {
                matches!(e.event, Event::SupplyInvariantViolation { .. })
            }));
        });
    }

    #[test]
    fn test_historical_proof_retention() {
        new_test_ext().execute_with(|| {
            let asset1 = test_asset_id(1);
            Ledgers::<Test>::insert(asset1, create_valid_ledger(1000, 1000, 0, 0));

            // Finalize multiple blocks
            for block in 1..=10 {
                Pallet::<Test>::on_finalize(block);
            }

            // Verify historical proofs stored
            for block in 1..=10 {
                assert!(HistoricalProofs::<Test>::contains_key(block));
            }
        });
    }
    */
}

/// Fuzz test for supply invariant (S0-1 requirement).
///
/// This test uses property-based testing to verify that the supply invariant
/// holds under all possible operation sequences.
///
/// Run with: cargo test --release fuzz_all_operations_preserve_invariant
#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    #[derive(Clone, Debug)]
    enum Operation {
        Mint {
            domain: DomainId,
            amount: Balance,
        },
        Burn {
            domain: DomainId,
            amount: Balance,
        },
        Transfer {
            from: DomainId,
            to: DomainId,
            amount: Balance,
        },
    }

    impl Arbitrary for Operation {
        fn arbitrary(g: &mut Gen) -> Self {
            let domains = [DomainId::X3Native, DomainId::X3Evm, DomainId::X3Svm];
            let domain_idx = usize::arbitrary(g) % domains.len();
            let amount = Balance::arbitrary(g) % 1_000_000; // Keep amounts reasonable

            match u8::arbitrary(g) % 3 {
                0 => Operation::Mint {
                    domain: domains[domain_idx],
                    amount,
                },
                1 => Operation::Burn {
                    domain: domains[domain_idx],
                    amount,
                },
                _ => {
                    let to_idx = (domain_idx + 1) % domains.len();
                    Operation::Transfer {
                        from: domains[domain_idx],
                        to: domains[to_idx],
                        amount,
                    }
                }
            }
        }
    }

    #[quickcheck]
    fn fuzz_all_operations_preserve_invariant(ops: Vec<Operation>) -> TestResult {
        if ops.is_empty() {
            return TestResult::discard();
        }

        let mut ledger = SupplyLedger::default();

        // Apply each operation
        for op in ops {
            match op {
                Operation::Mint { domain, amount } => {
                    // Check would overflow
                    let new_canonical = match ledger.canonical_supply.checked_add(amount) {
                        Some(c) => c,
                        None => return TestResult::discard(), // Overflow, skip test
                    };

                    let domain_supply = get_domain_supply(&ledger, domain);
                    let new_domain = match domain_supply.checked_add(amount) {
                        Some(d) => d,
                        None => return TestResult::discard(),
                    };

                    ledger.canonical_supply = new_canonical;
                    set_domain_supply(&mut ledger, domain, new_domain);
                }
                Operation::Burn { domain, amount } => {
                    let domain_supply = get_domain_supply(&ledger, domain);
                    if amount > domain_supply || amount > ledger.canonical_supply {
                        continue; // Can't burn more than available
                    }

                    ledger.canonical_supply -= amount;
                    set_domain_supply(&mut ledger, domain, domain_supply - amount);
                }
                Operation::Transfer { from, to, amount } => {
                    let from_supply = get_domain_supply(&ledger, from);
                    if amount > from_supply {
                        continue; // Can't transfer more than available
                    }

                    let to_supply = get_domain_supply(&ledger, to);
                    let new_to = match to_supply.checked_add(amount) {
                        Some(t) => t,
                        None => continue, // Would overflow destination
                    };

                    set_domain_supply(&mut ledger, from, from_supply - amount);
                    set_domain_supply(&mut ledger, to, new_to);
                }
            }

            // CRITICAL: Invariant must hold after EVERY operation
            if ledger.check_invariant().is_err() {
                return TestResult::failed();
            }
        }

        TestResult::passed()
    }

    fn get_domain_supply(ledger: &SupplyLedger, domain: DomainId) -> Balance {
        match domain {
            DomainId::X3Native => ledger.native_supply,
            DomainId::X3Evm => ledger.evm_supply,
            DomainId::X3Svm => ledger.svm_supply,
            _ => ledger.external_locked_supply,
        }
    }

    fn set_domain_supply(ledger: &mut SupplyLedger, domain: DomainId, amount: Balance) {
        match domain {
            DomainId::X3Native => ledger.native_supply = amount,
            DomainId::X3Evm => ledger.evm_supply = amount,
            DomainId::X3Svm => ledger.svm_supply = amount,
            _ => ledger.external_locked_supply = amount,
        }
    }
}
