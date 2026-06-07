//! Comprehensive tests for pallet-x3-wallet.
//!
//! Tests cover:
//! - Hardware wallet registration
//! - Multisig wallet creation
//! - Token transfers
//! - Biometric registration
//! - Error conditions

#![cfg(test)]

use crate::{mock::*, pallet::*};
use frame_support::{assert_noop, assert_ok, BoundedVec};

// ============================================================================
// Hardware Wallet Tests
// ============================================================================

#[test]
fn register_hardware_wallet_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let device_type = 1u8; // e.g., Ledger
        let device_model = b"Ledger Nano X".to_vec();
        let public_key = [1u8; 32];

        assert_ok!(X3Wallet::register_hardware_wallet(
            RuntimeOrigin::signed(ALICE),
            device_type,
            device_model.clone(),
            public_key,
        ));

        // Check event emitted
        System::assert_has_event(RuntimeEvent::X3Wallet(Event::HardwareWalletConnected {
            account: ALICE,
            device_type,
        }));
    });
}

#[test]
fn register_multiple_hardware_wallets_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register first wallet
        assert_ok!(X3Wallet::register_hardware_wallet(
            RuntimeOrigin::signed(ALICE),
            1,
            b"Ledger Nano X".to_vec(),
            [1u8; 32],
        ));

        // Register second wallet with different key
        assert_ok!(X3Wallet::register_hardware_wallet(
            RuntimeOrigin::signed(ALICE),
            2,
            b"Trezor Model T".to_vec(),
            [2u8; 32],
        ));
    });
}

// ============================================================================
// Multisig Wallet Tests
// ============================================================================

#[test]
fn create_multisig_wallet_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let signers: Vec<[u8; 32]> = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

        assert_ok!(X3Wallet::create_multisig_wallet(
            RuntimeOrigin::signed(ALICE),
            signers,
            2,    // 2-of-3 threshold
            3600, // 1 hour timelock
        ));

        System::assert_has_event(RuntimeEvent::X3Wallet(Event::MultisigWalletCreated {
            account: ALICE,
            threshold: 2,
        }));
    });
}

#[test]
fn create_multisig_wallet_fails_with_invalid_threshold() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let signers: Vec<[u8; 32]> = vec![[1u8; 32], [2u8; 32]];

        // Threshold 0 is invalid
        assert_noop!(
            X3Wallet::create_multisig_wallet(
                RuntimeOrigin::signed(ALICE),
                signers.clone(),
                0, // Invalid threshold
                3600,
            ),
            Error::<Test>::InvalidThreshold
        );

        // Threshold > number of signers is invalid
        assert_noop!(
            X3Wallet::create_multisig_wallet(
                RuntimeOrigin::signed(ALICE),
                signers,
                5, // Greater than 2 signers
                3600,
            ),
            Error::<Test>::InvalidThreshold
        );
    });
}

// ============================================================================
// Token Transfer Tests
// ============================================================================

#[test]
fn transfer_tokens_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let token_id = [42u8; 32];

        // First mint some tokens to ALICE
        assert_ok!(X3Wallet::mint_tokens(
            RuntimeOrigin::signed(ALICE), // admin
            token_id,
            ALICE,
            1000,
        ));

        // Transfer some tokens to BOB
        assert_ok!(X3Wallet::transfer_tokens(
            RuntimeOrigin::signed(ALICE),
            token_id,
            BOB,
            300,
        ));

        // Check balances
        assert_eq!(X3Wallet::get_token_balance(&ALICE, &token_id), 700);
        assert_eq!(X3Wallet::get_token_balance(&BOB, &token_id), 300);
    });
}

#[test]
fn transfer_tokens_fails_with_zero_amount() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let token_id = [42u8; 32];

        assert_noop!(
            X3Wallet::transfer_tokens(
                RuntimeOrigin::signed(ALICE),
                token_id,
                BOB,
                0, // Invalid amount
            ),
            Error::<Test>::InvalidAmount
        );
    });
}

#[test]
fn transfer_tokens_fails_with_insufficient_balance() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let token_id = [42u8; 32];

        // Mint 100 tokens to ALICE
        assert_ok!(X3Wallet::mint_tokens(
            RuntimeOrigin::signed(ALICE),
            token_id,
            ALICE,
            100,
        ));

        // Try to transfer more than balance
        assert_noop!(
            X3Wallet::transfer_tokens(
                RuntimeOrigin::signed(ALICE),
                token_id,
                BOB,
                200, // More than balance
            ),
            Error::<Test>::InsufficientBalance
        );
    });
}

// ============================================================================
// Biometric Registration Tests
// ============================================================================

#[test]
fn register_biometric_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let biometric_type = 1u8; // e.g., fingerprint
        let template_hash = [1u8; 32];
        let pin_hash = [2u8; 32];

        assert_ok!(X3Wallet::register_biometric(
            RuntimeOrigin::signed(ALICE),
            biometric_type,
            template_hash,
            pin_hash,
        ));

        // Check profile was created
        let profile =
            X3Wallet::get_biometric_profile(&ALICE).expect("biometric profile should exist");
        assert_eq!(profile.biometric_type, biometric_type);
        assert_eq!(profile.template_hash, template_hash);
        assert!(profile.is_enabled);

        System::assert_has_event(RuntimeEvent::X3Wallet(Event::BiometricProfileCreated {
            account: ALICE,
        }));
    });
}

// ============================================================================
// Recovery Tests
// ============================================================================

#[test]
fn initiate_recovery_fails_without_guardian() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Try to initiate recovery without having set up guardians
        assert_noop!(
            X3Wallet::initiate_recovery(
                RuntimeOrigin::signed(ALICE),
                [3u8; 32], // new owner
            ),
            Error::<Test>::WalletNotFound
        );
    });
}

// ============================================================================
// Token Minting Tests
// ============================================================================

#[test]
fn mint_tokens_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let token_id = [42u8; 32];

        assert_ok!(X3Wallet::mint_tokens(
            RuntimeOrigin::signed(ALICE),
            token_id,
            BOB,
            5000,
        ));

        assert_eq!(X3Wallet::get_token_balance(&BOB, &token_id), 5000);

        System::assert_has_event(RuntimeEvent::X3Wallet(Event::BalanceUpdated {
            account: BOB,
            token_id,
            amount: 5000,
        }));
    });
}

#[test]
fn mint_tokens_accumulates() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let token_id = [42u8; 32];

        assert_ok!(X3Wallet::mint_tokens(
            RuntimeOrigin::signed(ALICE),
            token_id,
            BOB,
            1000,
        ));

        assert_ok!(X3Wallet::mint_tokens(
            RuntimeOrigin::signed(ALICE),
            token_id,
            BOB,
            500,
        ));

        assert_eq!(X3Wallet::get_token_balance(&BOB, &token_id), 1500);
    });
}

// ============================================================================
// Storage Query Tests
// ============================================================================

#[test]
fn storage_queries_return_none_for_missing_data() {
    new_test_ext().execute_with(|| {
        assert!(X3Wallet::get_hardware_wallet(&ALICE, &[0u8; 32]).is_none());
        assert!(X3Wallet::get_multisig_wallet(&ALICE, &[0u8; 32]).is_none());
        assert!(X3Wallet::get_biometric_profile(&ALICE).is_none());
        assert!(X3Wallet::get_recovery_account(&ALICE).is_none());
        assert_eq!(X3Wallet::get_token_balance(&ALICE, &[0u8; 32]), 0);
    });
}
