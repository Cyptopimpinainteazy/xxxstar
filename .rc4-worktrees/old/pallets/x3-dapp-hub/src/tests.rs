//! Tests for `pallet-x3-dapp-hub`.
//!
//! Coverage (20 tests):
//!  1.  register_dapp_works
//!  2.  register_dapp_policy_not_found_fails
//!  3.  register_dapp_max_dapps_per_developer_fails
//!  4.  approve_dapp_works
//!  5.  approve_dapp_not_found_fails
//!  6.  approve_dapp_non_governance_fails
//!  7.  reject_dapp_works
//!  8.  reject_dapp_non_governance_fails
//!  9.  suspend_dapp_works
//! 10.  suspend_dapp_non_governance_fails
//! 11.  record_revenue_works
//! 12.  record_revenue_correct_split
//! 13.  record_revenue_rejected_dapp_fails
//! 14.  record_revenue_suspended_dapp_fails
//! 15.  record_revenue_pending_dapp_fails
//! 16.  set_revenue_policy_works
//! 17.  set_revenue_policy_invalid_sum_fails
//! 18.  set_revenue_policy_non_governance_fails
//! 19.  withdraw_earnings_works
//! 20.  withdraw_earnings_insufficient_fails

use crate::{
    mock::{
        new_test_ext, setup_policy, DappHub, MaxDAppsPerDeveloper, RuntimeOrigin, System, Test,
    },
    pallet::{DApps, DeveloperDApps, DeveloperEarnings, NextDAppId, RevenuePolicies, TotalDApps},
    ApprovalStatus, Error, Event, PlacementTier,
};
use frame_support::{assert_noop, assert_ok};

// ── Constants for readability ─────────────────────────────────────────────────

const POLICY_ID: u32 = 1;
const CATEGORY_ID: u32 = 42;
const DEV: u64 = 1;
const OTHER: u64 = 2;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Register a dApp under `developer` using `POLICY_ID` / `CATEGORY_ID`.
/// Panics on failure.
fn register(developer: u64) -> u64 {
    let dapp_id = NextDAppId::<Test>::get();
    assert_ok!(DappHub::register_dapp(
        RuntimeOrigin::signed(developer),
        CATEGORY_ID,
        POLICY_ID,
    ));
    dapp_id
}

/// Register and then approve a dApp.
fn register_and_approve(developer: u64) -> u64 {
    let id = register(developer);
    assert_ok!(DappHub::approve_dapp(RuntimeOrigin::root(), id));
    id
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// 1. Happy-path registration.
#[test]
fn register_dapp_works() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);

        let dapp_id = register(DEV);

        let dapp = DApps::<Test>::get(dapp_id).expect("dapp must exist");
        assert_eq!(dapp.developer, DEV);
        assert_eq!(dapp.category_id, CATEGORY_ID);
        assert_eq!(dapp.revenue_policy_id, POLICY_ID);
        assert_eq!(dapp.approval_status, ApprovalStatus::Pending);
        assert_eq!(dapp.placement, PlacementTier::Standard);
        assert_eq!(TotalDApps::<Test>::get(), 1);
    });
}

/// 2. Registration fails when the referenced policy does not exist.
#[test]
fn register_dapp_policy_not_found_fails() {
    new_test_ext().execute_with(|| {
        // No policy stored — should fail.
        assert_noop!(
            DappHub::register_dapp(RuntimeOrigin::signed(DEV), CATEGORY_ID, 99),
            Error::<Test>::PolicyNotFound
        );
    });
}

/// 3. Registration fails once the developer has reached `MaxDAppsPerDeveloper`.
#[test]
fn register_dapp_max_dapps_per_developer_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);

        let limit = MaxDAppsPerDeveloper::get();
        for _ in 0..limit {
            register(DEV);
        }

        assert_noop!(
            DappHub::register_dapp(RuntimeOrigin::signed(DEV), CATEGORY_ID, POLICY_ID),
            Error::<Test>::MaxDAppsReached
        );
    });
}

/// 4. Governance can approve a pending dApp.
#[test]
fn approve_dapp_works() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register(DEV);

        assert_ok!(DappHub::approve_dapp(RuntimeOrigin::root(), id));

        let dapp = DApps::<Test>::get(id).unwrap();
        assert_eq!(dapp.approval_status, ApprovalStatus::Approved);

        System::assert_last_event(Event::DAppApproved { dapp_id: id }.into());
    });
}

/// 5. Approving a non-existent dApp returns `DAppNotFound`.
#[test]
fn approve_dapp_not_found_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            DappHub::approve_dapp(RuntimeOrigin::root(), 9999),
            Error::<Test>::DAppNotFound
        );
    });
}

/// 6. A non-governance signed origin cannot approve a dApp.
#[test]
fn approve_dapp_non_governance_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register(DEV);

        assert_noop!(
            DappHub::approve_dapp(RuntimeOrigin::signed(OTHER), id),
            frame_system::Error::<Test>::CallFiltered
        );
    });
}

/// 7. Governance can reject a dApp.
#[test]
fn reject_dapp_works() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register(DEV);

        assert_ok!(DappHub::reject_dapp(RuntimeOrigin::root(), id));

        let dapp = DApps::<Test>::get(id).unwrap();
        assert_eq!(dapp.approval_status, ApprovalStatus::Rejected);
        System::assert_last_event(Event::DAppRejected { dapp_id: id }.into());
    });
}

/// 8. A non-governance signed origin cannot reject a dApp.
#[test]
fn reject_dapp_non_governance_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register(DEV);

        assert_noop!(
            DappHub::reject_dapp(RuntimeOrigin::signed(OTHER), id),
            frame_system::Error::<Test>::CallFiltered
        );
    });
}

/// 9. Governance can suspend an approved dApp.
#[test]
fn suspend_dapp_works() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register_and_approve(DEV);

        assert_ok!(DappHub::suspend_dapp(RuntimeOrigin::root(), id));

        let dapp = DApps::<Test>::get(id).unwrap();
        assert_eq!(dapp.approval_status, ApprovalStatus::Suspended);
        System::assert_last_event(Event::DAppSuspended { dapp_id: id }.into());
    });
}

/// 10. A non-governance signed origin cannot suspend a dApp.
#[test]
fn suspend_dapp_non_governance_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register_and_approve(DEV);

        assert_noop!(
            DappHub::suspend_dapp(RuntimeOrigin::signed(OTHER), id),
            frame_system::Error::<Test>::CallFiltered
        );
    });
}

/// 11. Revenue can be recorded for an approved dApp.
#[test]
fn record_revenue_works() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register_and_approve(DEV);

        assert_ok!(DappHub::record_revenue(
            RuntimeOrigin::root(),
            id,
            1_000_000
        ));

        let dapp = DApps::<Test>::get(id).unwrap();
        assert_eq!(dapp.total_revenue_collected, 1_000_000);
    });
}

/// 12. Developer share is computed correctly (policy: 70 % developer, 30 % treasury).
///
/// Gross = 10 000 000
/// Expected developer share = 10 000 000 × 7 000 / 10 000 = 7 000 000
/// Expected protocol share  = 10 000 000 - 7 000 000 = 3 000 000
#[test]
fn record_revenue_correct_split() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID); // 7000 bps developer, 3000 bps treasury
        let id = register_and_approve(DEV);

        let gross: u128 = 10_000_000;
        assert_ok!(DappHub::record_revenue(RuntimeOrigin::root(), id, gross));

        let earnings = DeveloperEarnings::<Test>::get(DEV);
        assert_eq!(
            earnings, 7_000_000,
            "developer must receive 70 % = 7 000 000"
        );

        let dapp = DApps::<Test>::get(id).unwrap();
        assert_eq!(dapp.total_developer_paid, 7_000_000);
        assert_eq!(dapp.total_revenue_collected, 10_000_000);

        // Verify the emitted event carries correct amounts.
        System::assert_last_event(
            Event::RevenueRecorded {
                dapp_id: id,
                gross_amount: 10_000_000,
                developer_share: 7_000_000,
                protocol_share: 3_000_000,
            }
            .into(),
        );
    });
}

/// 13. Recording revenue on a rejected dApp returns `DAppNotApproved`.
#[test]
fn record_revenue_rejected_dapp_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register(DEV);
        assert_ok!(DappHub::reject_dapp(RuntimeOrigin::root(), id));

        assert_noop!(
            DappHub::record_revenue(RuntimeOrigin::root(), id, 1_000),
            Error::<Test>::DAppNotApproved
        );
    });
}

/// 14. Recording revenue on a suspended dApp returns `DAppNotApproved`.
#[test]
fn record_revenue_suspended_dapp_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register_and_approve(DEV);
        assert_ok!(DappHub::suspend_dapp(RuntimeOrigin::root(), id));

        assert_noop!(
            DappHub::record_revenue(RuntimeOrigin::root(), id, 1_000),
            Error::<Test>::DAppNotApproved
        );
    });
}

/// 15. Recording revenue on a pending (not-yet-approved) dApp returns `DAppNotApproved`.
#[test]
fn record_revenue_pending_dapp_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register(DEV); // still Pending

        assert_noop!(
            DappHub::record_revenue(RuntimeOrigin::root(), id, 1_000),
            Error::<Test>::DAppNotApproved
        );
    });
}

/// 16. Governance can store a valid revenue policy.
#[test]
fn set_revenue_policy_works() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);

        let policy = RevenuePolicies::<Test>::get(POLICY_ID).expect("policy must be stored");
        assert_eq!(policy.policy_id, POLICY_ID);
        assert_eq!(policy.entries_len, 2);
    });
}

/// 17. A policy whose entries do not sum to 10 000 bps is rejected.
#[test]
fn set_revenue_policy_invalid_sum_fails() {
    new_test_ext().execute_with(|| {
        use x3_revenue_sharing::{RevenueDestination, RevenueSplitEntry, RevenueSplitPolicy};

        fn empty() -> RevenueSplitEntry {
            RevenueSplitEntry {
                destination: RevenueDestination::Treasury,
                share_bps: 0,
            }
        }

        // 4000 + 5000 = 9000 — one basis point short
        let bad_policy = RevenueSplitPolicy {
            policy_id: 99,
            entries_len: 2,
            entries: [
                RevenueSplitEntry {
                    destination: RevenueDestination::Treasury,
                    share_bps: 4_000,
                },
                RevenueSplitEntry {
                    destination: RevenueDestination::DeveloperAccount,
                    share_bps: 5_000,
                },
                empty(),
                empty(),
                empty(),
                empty(),
                empty(),
                empty(),
            ],
        };

        assert_noop!(
            DappHub::set_revenue_policy(RuntimeOrigin::root(), 99, bad_policy),
            Error::<Test>::InvalidSplitPolicy
        );
    });
}

/// 18. A non-governance signed origin cannot set a revenue policy.
#[test]
fn set_revenue_policy_non_governance_fails() {
    new_test_ext().execute_with(|| {
        use x3_revenue_sharing::{RevenueDestination, RevenueSplitEntry, RevenueSplitPolicy};

        fn empty() -> RevenueSplitEntry {
            RevenueSplitEntry {
                destination: RevenueDestination::Treasury,
                share_bps: 0,
            }
        }

        let policy = RevenueSplitPolicy {
            policy_id: 1,
            entries_len: 1,
            entries: [
                RevenueSplitEntry {
                    destination: RevenueDestination::Treasury,
                    share_bps: 10_000,
                },
                empty(),
                empty(),
                empty(),
                empty(),
                empty(),
                empty(),
                empty(),
            ],
        };

        assert_noop!(
            DappHub::set_revenue_policy(RuntimeOrigin::signed(OTHER), 1, policy),
            frame_system::Error::<Test>::CallFiltered
        );
    });
}

/// 19. A developer can withdraw earnings that have been accrued.
#[test]
fn withdraw_earnings_works() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register_and_approve(DEV);

        // Record 10 M gross → 7 M developer earnings.
        assert_ok!(DappHub::record_revenue(
            RuntimeOrigin::root(),
            id,
            10_000_000
        ));
        assert_eq!(DeveloperEarnings::<Test>::get(DEV), 7_000_000);

        // Withdraw 3 M.
        assert_ok!(DappHub::withdraw_earnings(
            RuntimeOrigin::signed(DEV),
            3_000_000
        ));
        assert_eq!(DeveloperEarnings::<Test>::get(DEV), 4_000_000);

        System::assert_last_event(
            Event::EarningsWithdrawn {
                developer: DEV,
                amount: 3_000_000,
            }
            .into(),
        );
    });
}

/// 20. Withdrawing more than accrued earnings returns `InsufficientEarnings`.
#[test]
fn withdraw_earnings_insufficient_fails() {
    new_test_ext().execute_with(|| {
        setup_policy(POLICY_ID);
        let id = register_and_approve(DEV);

        assert_ok!(DappHub::record_revenue(
            RuntimeOrigin::root(),
            id,
            10_000_000
        ));
        // 7 M accrued; try to withdraw 8 M → must fail.
        assert_noop!(
            DappHub::withdraw_earnings(RuntimeOrigin::signed(DEV), 8_000_000),
            Error::<Test>::InsufficientEarnings
        );
    });
}
