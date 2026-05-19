//! Unit tests for `pallet-x3-custody`.
//!
//! Coverage:
//! 1.  register_signer — happy path + is_signer_authorized
//! 2.  register_signer — duplicate returns SignerAlreadyRegistered
//! 3.  deactivate_signer — happy path blocks authorization
//! 4.  deactivate_signer — unknown signer returns SignerNotFound
//! 5.  register_validator_key — happy path
//! 6.  register_validator_key — conflict returns ValidatorKeyConflict
//! 7.  rotate_validator_key — old deactivated, new active
//! 8.  rotate_validator_key — unknown old_key returns SignerNotFound
//! 9.  set_tier_threshold + get_threshold helper
//! 10. meets_threshold helper (Strategic needs 2+ signers)
//! 11. non-GovernanceOrigin register_signer is rejected
//! 12. non-GovernanceOrigin set_tier_threshold is rejected
//! 13. non-GovernanceOrigin register_validator_key is rejected
//! 14. MaxSignersPerVault capacity is enforced
//! 15. set_signer_limit — OperatorOrigin succeeds
//! 16. set_key_rotation_schedule — OperatorOrigin succeeds
//! 17. check_signer_authorized extrinsic returns Ok for active signer
//! 18. check_signer_authorized extrinsic returns Err for inactive signer
//! 19. ValidatorSigning role rejected for Operational tier (KeyRoleNotAllowedForTier)
//! 20. ValidatorSigning accepted for non-Operational tiers

use crate::{
    mock::{new_test_ext, RuntimeOrigin, System, Test, X3Custody},
    pallet::{CustodyMap, KeyRotationSchedule, SignerLimits, ValidatorKeyRegistry},
    AuthorizationTier, Error, KeyRole, SignerPolicy,
};
use frame_support::{assert_noop, assert_ok};

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const DAVE: u64 = 4;
const EVE: u64 = 5;

const CHAIN_ID: u32 = 1;
const ASSET_ID: u32 = 100;

// ── 1. register_signer happy path ────────────────────────────────────────────

#[test]
fn test_register_signer_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Operational,
            KeyRole::TreasuryOperational,
        ));

        assert!(
            X3Custody::is_signer_authorized(
                CHAIN_ID,
                ASSET_ID,
                &ALICE,
                AuthorizationTier::Operational,
            ),
            "ALICE should be authorized after registration"
        );
    });
}

// ── 2. register_signer duplicate ─────────────────────────────────────────────

#[test]
fn test_register_signer_duplicate_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Operational,
            KeyRole::TreasuryOperational,
        ));

        assert_noop!(
            X3Custody::register_signer(
                RuntimeOrigin::root(),
                CHAIN_ID,
                ASSET_ID,
                ALICE,
                AuthorizationTier::Operational,
                KeyRole::TreasuryOperational,
            ),
            Error::<Test>::SignerAlreadyRegistered
        );
    });
}

// ── 3. deactivate_signer blocks authorization ─────────────────────────────────

#[test]
fn test_deactivate_signer_blocks_authorization() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Operational,
            KeyRole::TreasuryOperational,
        ));
        assert!(X3Custody::is_signer_authorized(
            CHAIN_ID,
            ASSET_ID,
            &ALICE,
            AuthorizationTier::Operational
        ));

        assert_ok!(X3Custody::deactivate_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
        ));

        assert!(
            !X3Custody::is_signer_authorized(
                CHAIN_ID,
                ASSET_ID,
                &ALICE,
                AuthorizationTier::Operational
            ),
            "ALICE should no longer be authorized after deactivation"
        );
    });
}

// ── 4. deactivate_signer — signer not found ───────────────────────────────────

#[test]
fn test_deactivate_signer_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::deactivate_signer(
                RuntimeOrigin::root(),
                CHAIN_ID,
                ASSET_ID,
                BOB,
            ),
            Error::<Test>::SignerNotFound
        );
    });
}

// ── 5. register_validator_key happy path ──────────────────────────────────────

#[test]
fn test_register_validator_key_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            1000_u64, // rotation_due_at block 1000
        ));

        let record = ValidatorKeyRegistry::<Test>::get(ALICE)
            .expect("record must be stored");
        assert!(record.active);
        assert_eq!(record.rotation_due_at, 1000_u64);
        assert!(matches!(record.role, KeyRole::ValidatorSigning));

        let schedule = KeyRotationSchedule::<Test>::get(ALICE);
        assert_eq!(schedule, Some(1000_u64));
    });
}

// ── 6. register_validator_key — conflict ─────────────────────────────────────

#[test]
fn test_register_validator_key_conflict_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            2000_u64,
        ));

        assert_noop!(
            X3Custody::register_validator_key(RuntimeOrigin::root(), ALICE, 3000_u64),
            Error::<Test>::ValidatorKeyConflict
        );
    });
}

// ── 7. rotate_validator_key happy path ───────────────────────────────────────

#[test]
fn test_rotate_validator_key_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            5000_u64,
        ));

        assert_ok!(X3Custody::rotate_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            BOB,
        ));

        // Old key deactivated
        let old_record = ValidatorKeyRegistry::<Test>::get(ALICE)
            .expect("old record must still exist");
        assert!(!old_record.active, "old key must be inactive");
        assert!(
            KeyRotationSchedule::<Test>::get(ALICE).is_none(),
            "old key schedule removed"
        );

        // New key active, inherits rotation_due_at
        let new_record = ValidatorKeyRegistry::<Test>::get(BOB)
            .expect("new record must be stored");
        assert!(new_record.active);
        assert_eq!(new_record.rotation_due_at, 5000_u64);
        assert_eq!(KeyRotationSchedule::<Test>::get(BOB), Some(5000_u64));
    });
}

// ── 8. rotate_validator_key — old key not found ───────────────────────────────

#[test]
fn test_rotate_validator_key_old_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::rotate_validator_key(RuntimeOrigin::root(), ALICE, BOB),
            Error::<Test>::SignerNotFound
        );
    });
}

// ── 9. set_tier_threshold + get_threshold helper ──────────────────────────────

#[test]
fn test_set_tier_threshold_and_get_threshold() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::set_tier_threshold(
            RuntimeOrigin::root(),
            AuthorizationTier::Strategic,
            2,
        ));

        let policy =
            X3Custody::get_threshold(AuthorizationTier::Strategic).expect("policy must exist");
        assert_eq!(policy.min_signers, 2);
        assert!(matches!(policy.tier, AuthorizationTier::Strategic));

        // Operational threshold not set; should return None
        assert!(X3Custody::get_threshold(AuthorizationTier::Operational).is_none());
    });
}

// ── 10. meets_threshold helper (Strategic requires 2 signers) ─────────────────

#[test]
fn test_meets_threshold_strategic() {
    new_test_ext().execute_with(|| {
        // Set Strategic to require 2 signers
        assert_ok!(X3Custody::set_tier_threshold(
            RuntimeOrigin::root(),
            AuthorizationTier::Strategic,
            2,
        ));

        // Register one Strategic signer — threshold not met
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Strategic,
            KeyRole::RelayerSigning,
        ));
        assert!(
            !X3Custody::meets_threshold(CHAIN_ID, ASSET_ID, AuthorizationTier::Strategic),
            "1 signer should not meet threshold of 2"
        );

        // Register second Strategic signer — threshold now met
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            BOB,
            AuthorizationTier::Strategic,
            KeyRole::RelayerSigning,
        ));
        assert!(
            X3Custody::meets_threshold(CHAIN_ID, ASSET_ID, AuthorizationTier::Strategic),
            "2 signers should meet threshold of 2"
        );
    });
}

// ── 11. non-GovernanceOrigin: register_signer rejected ───────────────────────

#[test]
fn test_non_governance_register_signer_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::register_signer(
                RuntimeOrigin::signed(ALICE), // not root
                CHAIN_ID,
                ASSET_ID,
                BOB,
                AuthorizationTier::Operational,
                KeyRole::TreasuryOperational,
            ),
            frame_support::error::BadOrigin
        );
    });
}

// ── 12. non-GovernanceOrigin: set_tier_threshold rejected ────────────────────

#[test]
fn test_non_governance_set_tier_threshold_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::set_tier_threshold(
                RuntimeOrigin::signed(ALICE),
                AuthorizationTier::Strategic,
                3,
            ),
            frame_support::error::BadOrigin
        );
    });
}

// ── 13. non-GovernanceOrigin: register_validator_key rejected ────────────────

#[test]
fn test_non_governance_register_validator_key_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::register_validator_key(RuntimeOrigin::signed(ALICE), BOB, 500_u64),
            frame_support::error::BadOrigin
        );
    });
}

// ── 14. MaxSignersPerVault capacity enforced ──────────────────────────────────

#[test]
fn test_max_signers_per_vault_enforced() {
    new_test_ext().execute_with(|| {
        // MaxSignersPerVault = 4 in mock
        let signers = [ALICE, BOB, CHARLIE, DAVE];
        for &s in &signers {
            assert_ok!(X3Custody::register_signer(
                RuntimeOrigin::root(),
                CHAIN_ID,
                ASSET_ID,
                s,
                AuthorizationTier::Operational,
                KeyRole::TreasuryOperational,
            ));
        }

        // 5th signer must be rejected
        assert_noop!(
            X3Custody::register_signer(
                RuntimeOrigin::root(),
                CHAIN_ID,
                ASSET_ID,
                EVE,
                AuthorizationTier::Operational,
                KeyRole::TreasuryOperational,
            ),
            Error::<Test>::MaxSignersReached
        );

        // Verify all 4 stored entries
        let entries = CustodyMap::<Test>::get(CHAIN_ID, ASSET_ID);
        assert_eq!(entries.len(), 4);
    });
}

// ── 15. set_signer_limit — OperatorOrigin (any signed) succeeds ──────────────

#[test]
fn test_set_signer_limit_works() {
    new_test_ext().execute_with(|| {
        let policy = SignerPolicy {
            max_single_op_amount: 1_000_000,
            max_daily_aggregate: 5_000_000,
            allowed_tiers: 0b0000_0011, // Operational + Strategic
        };

        assert_ok!(X3Custody::set_signer_limit(
            RuntimeOrigin::signed(ALICE), // OperatorOrigin = EnsureSigned
            BOB,
            policy.clone(),
        ));

        let stored = SignerLimits::<Test>::get(BOB).expect("policy must be stored");
        assert_eq!(stored.max_single_op_amount, 1_000_000);
        assert_eq!(stored.allowed_tiers, 0b0000_0011);
    });
}

// ── 16. set_key_rotation_schedule — OperatorOrigin succeeds ──────────────────

#[test]
fn test_set_key_rotation_schedule_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::set_key_rotation_schedule(
            RuntimeOrigin::signed(ALICE),
            BOB,
            9999_u64,
        ));

        assert_eq!(KeyRotationSchedule::<Test>::get(BOB), Some(9999_u64));
    });
}

// ── 17. check_signer_authorized extrinsic — Ok for active signer ─────────────

#[test]
fn test_check_signer_authorized_ok_for_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Operational,
            KeyRole::TreasuryOperational,
        ));

        assert_ok!(X3Custody::check_signer_authorized(
            RuntimeOrigin::signed(BOB), // any signed origin
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Operational,
        ));
    });
}

// ── 18. check_signer_authorized extrinsic — Err for non-existent signer ──────

#[test]
fn test_check_signer_authorized_err_for_missing() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::check_signer_authorized(
                RuntimeOrigin::signed(BOB),
                CHAIN_ID,
                ASSET_ID,
                ALICE,
                AuthorizationTier::Operational,
            ),
            Error::<Test>::SignerNotFound
        );
    });
}

// ── 19. ValidatorSigning + Operational tier is rejected ──────────────────────

#[test]
fn test_validator_signing_role_rejected_for_operational_tier() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::register_signer(
                RuntimeOrigin::root(),
                CHAIN_ID,
                ASSET_ID,
                ALICE,
                AuthorizationTier::Operational,
                KeyRole::ValidatorSigning, // must not be combined with Operational
            ),
            Error::<Test>::KeyRoleNotAllowedForTier
        );
    });
}

// ── 20. ValidatorSigning accepted for non-Operational tier ───────────────────

#[test]
fn test_validator_signing_accepted_for_strategic_tier() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Strategic, // not Operational — OK
            KeyRole::ValidatorSigning,
        ));

        assert!(X3Custody::is_signer_authorized(
            CHAIN_ID,
            ASSET_ID,
            &ALICE,
            AuthorizationTier::Strategic,
        ));
    });
}

// ── Bonus: events emitted by key operations ───────────────────────────────────

#[test]
fn test_key_rotated_event_emitted() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            8000_u64,
        ));

        assert_ok!(X3Custody::rotate_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            BOB,
        ));

        System::assert_has_event(
            crate::pallet::Event::<Test>::KeyRotated {
                old_key: ALICE,
                new_key: BOB,
            }
            .into(),
        );
    });
}

#[test]
fn test_tier_threshold_set_event_emitted() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::set_tier_threshold(
            RuntimeOrigin::root(),
            AuthorizationTier::Emergency,
            3,
        ));

        System::assert_has_event(
            crate::pallet::Event::<Test>::TierThresholdSet {
                tier: AuthorizationTier::Emergency,
                min_signers: 3,
            }
            .into(),
        );
    });
}

#[test]
fn test_signer_deactivated_event_emitted() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
            AuthorizationTier::Operational,
            KeyRole::TreasuryOperational,
        ));

        assert_ok!(X3Custody::deactivate_signer(
            RuntimeOrigin::root(),
            CHAIN_ID,
            ASSET_ID,
            ALICE,
        ));

        System::assert_has_event(
            crate::pallet::Event::<Test>::SignerDeactivated {
                chain_id: CHAIN_ID,
                asset_id: ASSET_ID,
                signer: ALICE,
            }
            .into(),
        );
    });
}
