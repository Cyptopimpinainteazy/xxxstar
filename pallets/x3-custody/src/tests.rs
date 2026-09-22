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
//! 16. (removed) set_key_rotation_schedule — the due-date now lives only in the
//!     validator-key registry, so there is no separate schedule to drift
//! 17. check_signer_authorized extrinsic returns Ok for active signer
//! 18. check_signer_authorized extrinsic returns Err for inactive signer
//! 19. ValidatorSigning role rejected for Operational tier (KeyRoleNotAllowedForTier)
//! 20. ValidatorSigning accepted for non-Operational tiers
//! 21. rotate_validator_key — next_due_at must be strictly in the future
//! 22. renew_validator_key — same-account happy path, due-date advances
//! 23. renew_validator_key — an already-elapsed next_due_at is refused
//! 24. renew_validator_key — unregistered account is refused
//! 25. renew_validator_key — inactive (rotated-away) account is refused
//! 26. renew_validator_key — repeated renewal never thrashes (each call strictly
//!     advances the due-date, unlike the old rotate_validator_key bug)

use crate::{
    mock::{new_test_ext, RuntimeOrigin, System, Test, X3Custody},
    pallet::{CustodyMap, SignerLimits, ValidatorKeyRegistry},
    AuthorizationTier, Error, KeyRole, SignerPolicy,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::BuildStorage;

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const DAVE: u64 = 4;
const EVE: u64 = 5;

const CHAIN_ID: u32 = 1;
const ASSET_ID: u32 = 100;

#[test]
fn test_genesis_seeds_validator_key_registry() {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("system genesis builds");
    crate::pallet::GenesisConfig::<Test> {
        initial_tier_thresholds: Vec::new(),
        initial_signer_limits: Vec::new(),
        initial_validator_keys: vec![(ALICE, 1000_u64).encode()],
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut storage)
    .expect("custody genesis assimilates");

    let mut ext: sp_io::TestExternalities = storage.into();
    ext.execute_with(|| {
        let record = ValidatorKeyRegistry::<Test>::get(ALICE).expect("record must be seeded");
        assert!(record.active);
        assert_eq!(record.rotation_due_at, 1000_u64);
    });
}

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
            X3Custody::deactivate_signer(RuntimeOrigin::root(), CHAIN_ID, ASSET_ID, BOB,),
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

        let record = ValidatorKeyRegistry::<Test>::get(ALICE).expect("record must be stored");
        assert!(record.active);
        assert_eq!(record.rotation_due_at, 1000_u64);
        assert!(matches!(record.role, KeyRole::ValidatorSigning));
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

        // Rotating after the old due-date, with an explicit new due-date —
        // this is exactly the case that used to thrash: old due_at=5000 is
        // already in the past by block 6000, and the old code copied it
        // onto the new key regardless.
        System::set_block_number(6000);
        assert_ok!(X3Custody::rotate_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            BOB,
            7000_u64,
        ));

        // Old key deactivated
        let old_record =
            ValidatorKeyRegistry::<Test>::get(ALICE).expect("old record must still exist");
        assert!(!old_record.active, "old key must be inactive");
        // New key active, on the caller-supplied due-date (not the old one)
        let new_record = ValidatorKeyRegistry::<Test>::get(BOB).expect("new record must be stored");
        assert!(new_record.active);
        assert_eq!(new_record.rotation_due_at, 7000_u64);
        assert!(
            new_record.rotation_due_at > System::block_number(),
            "the new key must not be immediately overdue"
        );
    });
}

// ── 7b. rotate_validator_key — next_due_at must be in the future ─────────────

#[test]
fn test_rotate_validator_key_rejects_past_due_date() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            5000_u64,
        ));
        System::set_block_number(6000);

        // next_due_at in the past
        assert_noop!(
            X3Custody::rotate_validator_key(RuntimeOrigin::root(), ALICE, BOB, 100_u64),
            Error::<Test>::RotationDueDateNotInFuture
        );
        // next_due_at == current block (not strictly in the future)
        assert_noop!(
            X3Custody::rotate_validator_key(RuntimeOrigin::root(), ALICE, BOB, 6000_u64),
            Error::<Test>::RotationDueDateNotInFuture
        );

        // Nothing was mutated by the refused calls.
        let record = ValidatorKeyRegistry::<Test>::get(ALICE).unwrap();
        assert!(
            record.active,
            "old key must remain active after a refused rotation"
        );
        assert!(ValidatorKeyRegistry::<Test>::get(BOB).is_none());
    });
}

// ── 8. rotate_validator_key — old key not found ───────────────────────────────

#[test]
fn test_rotate_validator_key_old_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::rotate_validator_key(RuntimeOrigin::root(), ALICE, BOB, 100_u64),
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
            9000_u64,
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

// ── renew_validator_key: same-account routine rotation ────────────────────────

#[test]
fn test_renew_validator_key_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            100_u64,
        ));

        System::set_block_number(90);
        assert_ok!(X3Custody::renew_validator_key(
            RuntimeOrigin::signed(ALICE),
            ALICE,
            500_u64,
        ));

        // Same account — no new registry entry is created, and it is still active.
        let record = ValidatorKeyRegistry::<Test>::get(ALICE).expect("record must still exist");
        assert!(record.active, "renewal must not deactivate the account");
        assert_eq!(record.rotation_due_at, 500_u64);
        assert_eq!(record.registered_at, 90_u64);
        System::assert_has_event(
            crate::pallet::Event::<Test>::ValidatorKeyRenewed {
                account: ALICE,
                rotation_due_at: 500_u64,
            }
            .into(),
        );
    });
}

#[test]
fn test_renew_validator_key_already_elapsed_is_refused() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            100_u64,
        ));

        // The validator is already well past its due-date (block 200 > due 100) —
        // renewing to a due-date that is itself already elapsed must be refused,
        // not silently accepted.
        System::set_block_number(200);
        assert_noop!(
            X3Custody::renew_validator_key(RuntimeOrigin::signed(ALICE), ALICE, 150_u64),
            Error::<Test>::RotationDueDateNotInFuture
        );
        assert_noop!(
            X3Custody::renew_validator_key(RuntimeOrigin::signed(ALICE), ALICE, 200_u64),
            Error::<Test>::RotationDueDateNotInFuture
        );

        // The original (overdue) record is untouched by the refused calls.
        let record = ValidatorKeyRegistry::<Test>::get(ALICE).unwrap();
        assert_eq!(record.rotation_due_at, 100_u64);
    });
}

#[test]
fn test_renew_validator_key_unregistered_account_is_refused() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Custody::renew_validator_key(RuntimeOrigin::signed(ALICE), ALICE, 500_u64),
            Error::<Test>::SignerNotFound
        );
    });
}

#[test]
fn test_renew_validator_key_inactive_account_is_refused() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            100_u64,
        ));
        // Deactivate ALICE by rotating it away to BOB.
        System::set_block_number(50);
        assert_ok!(X3Custody::rotate_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            BOB,
            200_u64,
        ));

        assert_noop!(
            X3Custody::renew_validator_key(RuntimeOrigin::signed(ALICE), ALICE, 300_u64),
            Error::<Test>::SignerNotFound
        );
    });
}

#[test]
fn test_renew_validator_key_can_be_called_repeatedly_without_thrash() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3Custody::register_validator_key(
            RuntimeOrigin::root(),
            ALICE,
            100_u64,
        ));

        // Three successive renewals, each strictly advancing the due-date —
        // this is the routine "operator renews what is due" loop the old
        // rotate_validator_key thrashed on; renew_validator_key must not.
        for (now, next_due) in [(90_u64, 200_u64), (190_u64, 300_u64), (290_u64, 400_u64)] {
            System::set_block_number(now);
            assert_ok!(X3Custody::renew_validator_key(
                RuntimeOrigin::signed(ALICE),
                ALICE,
                next_due,
            ));
            let record = ValidatorKeyRegistry::<Test>::get(ALICE).unwrap();
            assert_eq!(record.rotation_due_at, next_due);
            assert!(record.rotation_due_at > System::block_number());
        }
    });
}
