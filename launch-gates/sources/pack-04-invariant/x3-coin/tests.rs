//! Tests for X3 Coin pallet
//!
//! This module contains comprehensive tests for the X3 Coin pallet including:
//! - Genesis configuration and initial allocations
//! - Cross-chain operations and replay protection
//! - Vesting schedules and bonus claims
//! - Deterministic serialization and invariants
//! - Integration tests with X3 Kernel

#![cfg(test)]

use super::*;
use crate::mock::{new_test_ext, RuntimeOrigin, Test};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

type X3Coin = Pallet<Test>;

// Test accounts
const TREASURY: u64 = 1;
const TEAM_MEMBER: u64 = 2;
const ECOSYSTEM_PARTNER: u64 = 3;
const LIQUIDITY_PROVIDER: u64 = 4;
const BONUS_CLAIMER: u64 = 5;
const CROSS_CHAIN_USER: u64 = 6;

fn finality_envelope(chain: u8, confirmations: u32, tail: &[u8]) -> Vec<u8> {
    let receipt_root = [0x11; 32];
    let header_hash = [0x22; 32];
    let light_client_root = [0x33; 32];

    let mut inclusion_preimage = Vec::with_capacity(1 + 32 + 32);
    inclusion_preimage.push(chain);
    inclusion_preimage.extend_from_slice(&receipt_root);
    inclusion_preimage.extend_from_slice(&light_client_root);
    let inclusion_commitment = sp_io::hashing::blake2_256(&inclusion_preimage);

    let mut header_preimage = Vec::with_capacity(1 + 32 + 32);
    header_preimage.push(chain);
    header_preimage.extend_from_slice(&header_hash);
    header_preimage.extend_from_slice(&light_client_root);
    let header_commitment = sp_io::hashing::blake2_256(&header_preimage);

    let mut proof = Vec::with_capacity(74 + 96 + tail.len());
    proof.extend_from_slice(b"X3PF");
    proof.push(chain);
    proof.push(1); // envelope version
    proof.extend_from_slice(&confirmations.to_le_bytes());
    proof.extend_from_slice(&inclusion_commitment);
    proof.extend_from_slice(&header_commitment);
    proof.extend_from_slice(&receipt_root);
    proof.extend_from_slice(&header_hash);
    proof.extend_from_slice(&light_client_root);
    proof.extend_from_slice(tail);
    proof
}

fn evm_finality_proof_data(tx_hash: H256, block_number: u64, confirmations: u32) -> Vec<u8> {
    let mut tx_preimage = Vec::with_capacity(5 + 32);
    tx_preimage.extend_from_slice(b"EVMTX");
    tx_preimage.extend_from_slice(tx_hash.as_bytes());
    let tx_commitment = sp_io::hashing::blake2_256(&tx_preimage);

    let mut tail = Vec::with_capacity(52);
    tail.extend_from_slice(&tx_commitment);
    tail.extend_from_slice(&block_number.to_le_bytes());
    tail.extend_from_slice(&block_number.to_le_bytes());
    tail.extend_from_slice(&0u32.to_le_bytes());

    finality_envelope(1, confirmations, &tail)
}

fn svm_finality_proof_data(signature: &[u8], block_number: u64, confirmations: u32) -> Vec<u8> {
    let mut sig_preimage = Vec::with_capacity(6 + signature.len());
    sig_preimage.extend_from_slice(b"SVMSIG");
    sig_preimage.extend_from_slice(signature);
    let sig_commitment = sp_io::hashing::blake2_256(&sig_preimage);

    let mut tail = Vec::with_capacity(41);
    tail.extend_from_slice(&sig_commitment);
    tail.extend_from_slice(&block_number.to_le_bytes());
    tail.push(1u8);

    finality_envelope(2, confirmations, &tail)
}

fn btc_finality_merkle_proof(txid: H256, block_height: u64, confirmations: u32) -> Vec<u8> {
    let mut tx_preimage = Vec::with_capacity(5 + 32);
    tx_preimage.extend_from_slice(b"BTCTX");
    tx_preimage.extend_from_slice(txid.as_bytes());
    let tx_commitment = sp_io::hashing::blake2_256(&tx_preimage);

    let mut tail = Vec::with_capacity(72);
    tail.extend_from_slice(&tx_commitment);
    tail.extend_from_slice(&block_height.to_le_bytes());
    tail.extend_from_slice(&[0xCC; 32]);

    finality_envelope(3, confirmations, &tail)
}

#[test]
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        // Check total supply
        assert_eq!(X3Coin::total_supply(), 2_000_000_000_000_000_000_000);

        // Check treasury allocation (20%)
        assert_eq!(X3Coin::treasury_balance(), 400_000_000_000_000_000_000);

        // Check bonus pool allocation (10%)
        assert_eq!(X3Coin::bonus_pool_balance(), 200_000_000_000_000_000_000);

        // Check team vesting schedule
        let team_schedule = X3Coin::team_vesting(&TEAM_MEMBER).unwrap();
        assert_eq!(team_schedule.total_amount, 300_000_000_000_000_000_000);
        assert_eq!(team_schedule.claimed, 0);
        assert_eq!(team_schedule.start_block, 0);
        assert_eq!(team_schedule.cliff_blocks, 7_884_000);
        assert_eq!(team_schedule.vesting_blocks, 15_768_000);

        // Check ecosystem allocation (no vesting)
        assert_eq!(
            pallet_x3_kernel::CanonicalLedger::<Test>::get(ECOSYSTEM_PARTNER, X3_ASSET_ID),
            500_000_000_000_000_000_000
        );

        // Check liquidity allocation (no vesting)
        assert_eq!(
            pallet_x3_kernel::CanonicalLedger::<Test>::get(LIQUIDITY_PROVIDER, X3_ASSET_ID),
            600_000_000_000_000_000_000
        );
    });
}

#[test]
fn team_vesting_claim_works() {
    new_test_ext().execute_with(|| {
        // Advance blocks to pass cliff period
        frame_system::Pallet::<Test>::set_block_number(8_000_000);

        // Claim vested amount
        assert_ok!(X3Coin::claim_team_vesting(RuntimeOrigin::signed(
            TEAM_MEMBER
        )));

        // Check that claim was transferred to canonical ledger
        let canonical_balance =
            pallet_x3_kernel::CanonicalLedger::<Test>::get(TEAM_MEMBER, X3_ASSET_ID);
        assert!(canonical_balance > 0);

        // Check vesting schedule was updated
        let schedule = X3Coin::team_vesting(&TEAM_MEMBER).unwrap();
        assert_eq!(schedule.claimed, canonical_balance);
    });
}

#[test]
fn team_vesting_cliff_not_reached() {
    new_test_ext().execute_with(|| {
        // Try to claim before cliff period
        assert_noop!(
            X3Coin::claim_team_vesting(RuntimeOrigin::signed(TEAM_MEMBER)),
            Error::<Test>::VestingCliffNotReached
        );
    });
}

#[test]
fn team_vesting_no_schedule() {
    new_test_ext().execute_with(|| {
        // Try to claim for account with no vesting schedule
        assert_noop!(
            X3Coin::claim_team_vesting(RuntimeOrigin::signed(CROSS_CHAIN_USER)),
            Error::<Test>::NoVestingSchedule
        );
    });
}

#[test]
fn bonus_claim_works() {
    new_test_ext().execute_with(|| {
        // Advance blocks to pass claim period
        frame_system::Pallet::<Test>::set_block_number(4_000_000);

        // Claim bonus
        assert_ok!(X3Coin::claim_bonus(RuntimeOrigin::signed(BONUS_CLAIMER)));

        // Check that bonus was transferred (10% of initial pool) to canonical ledger
        let bonus_amount = 200_000_000_000_000_000_000u128 / 10;
        assert_eq!(
            pallet_x3_kernel::CanonicalLedger::<Test>::get(BONUS_CLAIMER, X3_ASSET_ID),
            bonus_amount
        );

        // Check bonus pool was reduced
        assert_eq!(
            X3Coin::bonus_pool_balance(),
            200_000_000_000_000_000_000 - bonus_amount
        );

        // Check claim record was added
        let claims = X3Coin::bonus_claims(&BONUS_CLAIMER);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].amount, bonus_amount);
    });
}

#[test]
fn bonus_claim_max_claims_reached() {
    new_test_ext().execute_with(|| {
        // Advance blocks and claim max times
        frame_system::Pallet::<Test>::set_block_number(4_000_000);

        for _ in 0..10 {
            assert_ok!(X3Coin::claim_bonus(RuntimeOrigin::signed(BONUS_CLAIMER)));
            frame_system::Pallet::<Test>::set_block_number(
                frame_system::Pallet::<Test>::block_number() + 4_000_000,
            );
        }

        // Try to claim one more time (should fail)
        assert_noop!(
            X3Coin::claim_bonus(RuntimeOrigin::signed(BONUS_CLAIMER)),
            Error::<Test>::MaxBonusClaimsReached
        );
    });
}

#[test]
fn bonus_claim_period_not_expired() {
    new_test_ext().execute_with(|| {
        // Try to claim immediately (period not expired)
        assert_noop!(
            X3Coin::claim_bonus(RuntimeOrigin::signed(BONUS_CLAIMER)),
            Error::<Test>::BonusClaimPeriodExpired
        );
    });
}

#[test]
fn cross_chain_mint_works() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000; // 100 X3

        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        assert_ok!(X3Coin::mint(
            RuntimeOrigin::signed(TREASURY),
            target_account.clone(),
            amount,
            proof
        ));

        // Check treasury balance was reduced
        assert_eq!(
            X3Coin::treasury_balance(),
            400_000_000_000_000_000_000 - amount
        );

        // Check target account received tokens
        assert_eq!(
            pallet_x3_kernel::CanonicalLedger::<Test>::get(CROSS_CHAIN_USER, X3_ASSET_ID),
            amount
        );
    });
}

#[test]
fn cross_chain_burn_works() {
    new_test_ext().execute_with(|| {
        let source_account = CROSS_CHAIN_USER;
        let amount = 50_000_000_000_000_000; // 50 X3

        // First mint some tokens to the account
        let mint_proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        assert_ok!(X3Coin::mint(
            RuntimeOrigin::signed(TREASURY),
            source_account.encode(),
            amount * 2,
            mint_proof
        ));

        // Now burn some tokens
        let burn_proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(67890),
            block_number: 2000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(67890), 2000, 12),
        };

        assert_ok!(X3Coin::burn(
            RuntimeOrigin::signed(TREASURY),
            source_account.encode(),
            amount,
            burn_proof
        ));

        // Check treasury balance was increased
        assert_eq!(
            X3Coin::treasury_balance(),
            400_000_000_000_000_000_000 - amount
        );

        // Check source account canonical balance was reduced
        assert_eq!(
            pallet_x3_kernel::CanonicalLedger::<Test>::get(source_account, X3_ASSET_ID),
            amount
        );
    });
}

#[test]
fn cross_chain_replay_protection_works() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        // First mint should succeed
        assert_ok!(X3Coin::mint(
            RuntimeOrigin::signed(TREASURY),
            target_account.clone(),
            amount,
            proof.clone()
        ));

        // Second mint with same proof should fail (replay attack)
        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::ProofAlreadyUsed
        );
    });
}

#[test]
fn cross_chain_rejects_evm_zero_hash_proof() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let proof = X3Proof::EvmProof {
            tx_hash: H256::zero(),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::zero(), 1000, 12),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_rejects_svm_invalid_signature_proof() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let proof = X3Proof::SvmProof {
            signature: vec![0u8; 32],
            block_number: 1000,
            proof_data: svm_finality_proof_data(&vec![0u8; 32], 1000, 32),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_rejects_btc_invalid_merkle_branch() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let proof = X3Proof::BtcProof {
            txid: H256::from_low_u64_be(777),
            block_height: 120_000,
            merkle_proof: vec![1, 2, 3],
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_rejects_evm_low_confirmations() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(42),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(42), 1000, 2),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_rejects_svm_low_confirmations() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let signature = vec![1u8; 64];
        let proof = X3Proof::SvmProof {
            signature: signature.clone(),
            block_number: 1000,
            proof_data: svm_finality_proof_data(&signature, 1000, 3),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_rejects_btc_low_confirmations() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let txid = H256::from_low_u64_be(777);
        let proof = X3Proof::BtcProof {
            txid,
            block_height: 120_000,
            merkle_proof: btc_finality_merkle_proof(txid, 120_000, 1),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_insufficient_treasury_balance() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 500_000_000_000_000_000_000; // More than treasury has

        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InsufficientTreasuryBalance
        );
    });
}

#[test]
fn cross_chain_insufficient_balance_for_burn() {
    new_test_ext().execute_with(|| {
        let source_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000; // 100 X3

        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        // Try to burn without having any tokens
        assert_noop!(
            X3Coin::burn(
                RuntimeOrigin::signed(TREASURY),
                source_account,
                amount,
                proof
            ),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn deterministic_operation_id_generation() {
    new_test_ext().execute_with(|| {
        let target_account = vec![1, 2, 3, 4];
        let amount = 100_000_000_000_000_000u128;
        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 10),
        };

        // Generate operation ID
        let operation_id = X3Coin::generate_operation_id(&target_account, amount, &proof);

        // Verify it's deterministic
        let operation_id_2 = X3Coin::generate_operation_id(&target_account, amount, &proof);
        assert_eq!(operation_id, operation_id_2);

        // Verify it's different for different inputs
        let different_amount = 200_000_000_000_000_000u128;
        let operation_id_3 =
            X3Coin::generate_operation_id(&target_account, different_amount, &proof);
        assert_ne!(operation_id, operation_id_3);
    });
}

#[test]
fn cross_chain_operation_serialization() {
    new_test_ext().execute_with(|| {
        let operation = CrossChainOperation::Mint {
            target_account: vec![1, 2, 3, 4],
            amount: 100_000_000_000_000_000,
            proof: X3Proof::EvmProof {
                tx_hash: H256::from_low_u64_be(12345),
                block_number: 1000,
                proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
            },
        };

        // Test encoding/decoding
        let encoded = operation.encode();
        let decoded = CrossChainOperation::decode(&mut &encoded[..]).unwrap();
        assert_eq!(operation, decoded);

        // Test that encoding is deterministic
        let encoded_2 = operation.encode();
        assert_eq!(encoded, encoded_2);
    });
}

#[test]
fn proof_serialization() {
    new_test_ext().execute_with(|| {
        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        // Test encoding/decoding
        let encoded = proof.encode();
        let decoded = X3Proof::decode(&mut &encoded[..]).unwrap();
        assert_eq!(proof, decoded);

        // Test that encoding is deterministic
        let encoded_2 = proof.encode();
        assert_eq!(encoded, encoded_2);
    });
}

#[test]
fn runtime_api_works() {
    new_test_ext().execute_with(|| {
        // Test total supply
        assert_eq!(X3Coin::get_total_supply(), 2_000_000_000_000_000_000_000);

        // Test treasury balance
        assert_eq!(X3Coin::get_treasury_balance(), 400_000_000_000_000_000_000);

        // Test bonus pool balance
        assert_eq!(
            X3Coin::get_bonus_pool_balance(),
            200_000_000_000_000_000_000
        );

        // Test vested amount (linear vesting begins from start block)
        assert_eq!(X3Coin::get_vested_amount(&TEAM_MEMBER), 19_025_875_190_258);

        // Test total bonus claims (should be 0 initially)
        assert_eq!(X3Coin::get_total_bonus_claims(&BONUS_CLAIMER), 0);
    });
}

#[test]
fn invariants_hold() {
    new_test_ext().execute_with(|| {
        // Invariant 1: Total supply should never change
        let initial_total = X3Coin::total_supply();

        // Perform various operations
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        assert_ok!(X3Coin::mint(
            RuntimeOrigin::signed(TREASURY),
            target_account,
            amount,
            proof
        ));

        // Total supply should remain the same
        assert_eq!(X3Coin::total_supply(), initial_total);

        // Invariant 2: Treasury balance + distributed tokens should equal initial treasury
        let initial_treasury = 400_000_000_000_000_000_000;
        let current_treasury = X3Coin::treasury_balance();
        let distributed = initial_treasury - current_treasury;

        // Check that distributed amount matches what was minted
        assert_eq!(distributed, amount);
    });
}

#[test]
fn integration_with_x3_kernel() {
    new_test_ext().execute_with(|| {
        let account = CROSS_CHAIN_USER;
        let amount = 100_000_000_000_000_000;

        // Use X3 Kernel to update canonical balance
        assert_ok!(pallet_x3_kernel::Pallet::<Test>::register_asset(
            RuntimeOrigin::root(),
            X3_ASSET_ID,
            b"X3".to_vec(),
            12,
        ));

        assert_ok!(pallet_x3_kernel::Pallet::<Test>::update_canonical_balance(
            RuntimeOrigin::root(),
            account,
            X3_ASSET_ID,
            amount,
            None
        ));

        // Check that canonical balance was updated
        assert_eq!(
            pallet_x3_kernel::CanonicalLedger::<Test>::get(account, X3_ASSET_ID),
            amount
        );

        // Check that X3 Coin can query the balance
        assert_eq!(X3Coin::get_vested_amount(&account), 0); // No vesting
    });
}

#[test]
fn stress_test_multiple_operations() {
    new_test_ext().execute_with(|| {
        let mut total_minted = 0u128;

        // Perform multiple mint operations
        for i in 0..10 {
            let target_account = (CROSS_CHAIN_USER + i).encode();
            let amount = 10_000_000_000_000_000; // 10 X3 each

            let proof = X3Proof::EvmProof {
                tx_hash: H256::from_low_u64_be((i + 1) as u64),
                block_number: 1000 + i as u64,
                proof_data: evm_finality_proof_data(
                    H256::from_low_u64_be((i + 1) as u64),
                    1000 + i as u64,
                    12,
                ),
            };

            assert_ok!(X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ));

            total_minted += amount;
        }

        // Check that all operations succeeded
        assert_eq!(total_minted, 100_000_000_000_000_000);

        // Check treasury balance was reduced correctly
        assert_eq!(
            X3Coin::treasury_balance(),
            400_000_000_000_000_000_000 - total_minted
        );
    });
}

#[test]
fn edge_cases() {
    new_test_ext().execute_with(|| {
        // Test with zero amount
        let target_account = CROSS_CHAIN_USER.encode();
        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        // Minting zero should succeed but not change balances
        assert_ok!(X3Coin::mint(
            RuntimeOrigin::signed(TREASURY),
            target_account,
            0,
            proof
        ));

        // Check balances unchanged
        assert_eq!(X3Coin::treasury_balance(), 400_000_000_000_000_000_000);
        assert_eq!(
            pallet_x3_kernel::CanonicalLedger::<Test>::get(CROSS_CHAIN_USER, X3_ASSET_ID),
            0
        );

        // Test with maximum amount
        let max_amount = u128::MAX;
        let target_account = CROSS_CHAIN_USER.encode();
        let proof = X3Proof::EvmProof {
            tx_hash: H256::from_low_u64_be(12345),
            block_number: 1000,
            proof_data: evm_finality_proof_data(H256::from_low_u64_be(12345), 1000, 12),
        };

        // This should fail due to insufficient treasury balance
        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                max_amount,
                proof
            ),
            Error::<Test>::InsufficientTreasuryBalance
        );
    });
}

#[test]
fn cross_chain_rejects_evm_witness_tx_commitment_mismatch() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;

        let tx_hash = H256::from_low_u64_be(111);
        let mismatched_hash = H256::from_low_u64_be(222);

        let proof = X3Proof::EvmProof {
            tx_hash,
            block_number: 1000,
            proof_data: evm_finality_proof_data(mismatched_hash, 1000, 12),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_rejects_svm_witness_slot_mismatch() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;
        let signature = vec![9u8; 64];

        let proof = X3Proof::SvmProof {
            signature: signature.clone(),
            block_number: 2000,
            proof_data: svm_finality_proof_data(&signature, 1999, 32),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}

#[test]
fn cross_chain_rejects_btc_witness_height_mismatch() {
    new_test_ext().execute_with(|| {
        let target_account = CROSS_CHAIN_USER.encode();
        let amount = 100_000_000_000_000_000;
        let txid = H256::from_low_u64_be(333);

        let proof = X3Proof::BtcProof {
            txid,
            block_height: 500_000,
            merkle_proof: btc_finality_merkle_proof(txid, 499_999, 6),
        };

        assert_noop!(
            X3Coin::mint(
                RuntimeOrigin::signed(TREASURY),
                target_account,
                amount,
                proof
            ),
            Error::<Test>::InvalidProof
        );
    });
}
