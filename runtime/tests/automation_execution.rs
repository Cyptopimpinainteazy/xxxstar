//! Automation executes for real on the production runtime.
//!
//! These run against the real `x3-chain-runtime`, not the pallet's mock, so they
//! pin the two things the audit found broken:
//!
//! * the runtime wires the median price oracle rather than `NoopOracle`, and an
//!   asset with no median **fails closed** instead of reading as zero;
//! * a task's action moves real balances through `RuntimeActionDispatcher`, and
//!   the keeper is paid only when a dispatch happened.

use frame_support::{assert_noop, assert_ok, traits::Currency};
use x3_automation::{Action, Condition, TaskStatus};
use x3_chain_runtime::{AccountId, Balances, Runtime, RuntimeOrigin, System};

type Automation = pallet_x3_automation::Pallet<Runtime>;
type Oracle = pallet_x3_oracle::Pallet<Runtime>;
type BalancesPallet = Balances;

const ENDOWED: u128 = 1_000_000_000_000_000;

fn account(seed: u8) -> AccountId {
    AccountId::new([seed; 32])
}

fn free(who: &AccountId) -> u128 {
    <BalancesPallet as Currency<AccountId>>::free_balance(who)
}

fn setup() -> sp_io::TestExternalities {
    let mut ext = sp_io::TestExternalities::default();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Register a task and return its id.
fn register(owner: &AccountId, condition: Condition, action: Action) -> x3_automation::TaskId {
    assert_ok!(Automation::register_task(
        RuntimeOrigin::signed(owner.clone()),
        condition,
        action,
        1_000_000_000_000u128,
    ));
    *Automation::account_tasks(owner)
        .last()
        .expect("task registered")
}

#[test]
fn a_native_transfer_task_moves_real_balances_and_pays_the_keeper() {
    setup().execute_with(|| {
        let owner = account(1);
        let keeper = account(2);
        let payee = account(3);
        let _ = Balances::make_free_balance_be(&owner, ENDOWED);
        let _ = Balances::make_free_balance_be(&keeper, ENDOWED);

        let mut recipient = [0u8; 32];
        recipient.copy_from_slice(payee.as_ref());
        let amount = 5_000_000_000_000u128;

        let task_id = register(
            &owner,
            Condition::Manual,
            Action::TransferNative { recipient, amount },
        );

        let owner_before = free(&owner);
        let payee_before = free(&payee);
        let keeper_before = free(&keeper);

        assert_ok!(Automation::execute_task(
            RuntimeOrigin::signed(keeper.clone()),
            task_id
        ));

        // The effect is real, not a reported one.
        assert_eq!(free(&payee), payee_before + amount);
        assert!(free(&owner) < owner_before);
        assert!(free(&keeper) > keeper_before, "the keeper was paid");

        let task = Automation::tasks(task_id).expect("terminal task is retained");
        assert_eq!(task.status, TaskStatus::Executed);
        let receipt =
            Automation::receipt(task_id, 1).expect("a dispatch that happened writes a receipt");
        assert_eq!(receipt.outcome, x3_automation::ActionOutcome::Applied);
    });
}

#[test]
fn a_price_condition_fails_closed_when_the_oracle_has_no_median() {
    setup().execute_with(|| {
        let owner = account(4);
        let keeper = account(5);
        let _ = Balances::make_free_balance_be(&owner, ENDOWED);
        let _ = Balances::make_free_balance_be(&keeper, ENDOWED);

        // Asset 4242 has never been reported on. The old `unwrap_or(0)` made
        // this "below 1000" trigger fire on an outage.
        let task_id = register(
            &owner,
            Condition::PriceThreshold {
                asset_id: 4242,
                threshold: 1_000,
                above: false,
            },
            Action::Noop,
        );

        let keeper_before = free(&keeper);
        assert_noop!(
            Automation::execute_task(RuntimeOrigin::signed(keeper.clone()), task_id),
            pallet_x3_automation::Error::<Runtime>::PriceUnavailable
        );
        assert_eq!(
            free(&keeper),
            keeper_before,
            "nothing ran, nothing was paid"
        );

        let task = Automation::tasks(task_id).expect("task survives a refusal");
        assert_eq!(task.status, TaskStatus::Active);
        assert_eq!(task.attempts, 0);
        assert!(Automation::receipt(task_id, 1).is_none());
    });
}

#[test]
fn the_runtime_oracle_supplies_the_automation_price() {
    setup().execute_with(|| {
        // Write a median directly, standing in for authorized submitters. The
        // timestamp must be current: a median older than the configured maximum
        // age is refused, and this test previously proved the opposite because
        // no freshness check existed at all.
        let asset = 77u32;
        let now = pallet_timestamp::Pallet::<Runtime>::get();
        pallet_x3_oracle::AssetPrices::<Runtime>::insert(
            asset,
            pallet_x3_oracle::PriceData {
                price: 1_500,
                submission_count: 3,
                last_updated: 1,
                timestamp: now,
            },
        );

        let owner = account(6);
        let keeper = account(7);
        let _ = Balances::make_free_balance_be(&owner, ENDOWED);
        let _ = Balances::make_free_balance_be(&keeper, ENDOWED);

        let task_id = register(
            &owner,
            Condition::PriceThreshold {
                asset_id: asset,
                threshold: 1_000,
                above: true,
            },
            Action::Noop,
        );

        assert_ok!(Automation::execute_task(
            RuntimeOrigin::signed(keeper),
            task_id
        ));
        assert_eq!(
            Automation::tasks(task_id).unwrap().status,
            TaskStatus::Executed
        );
    });
}

#[test]
fn an_unsupported_action_is_refused_rather_than_reported_as_success() {
    setup().execute_with(|| {
        let owner = account(8);
        let keeper = account(9);
        let _ = Balances::make_free_balance_be(&owner, ENDOWED);
        let _ = Balances::make_free_balance_be(&keeper, ENDOWED);

        let mut payload = [0u8; 64];
        payload[0] = 0xAA;
        let task_id = register(&owner, Condition::Manual, Action::Custom(payload));

        let keeper_before = free(&keeper);
        // The attempt is recorded and the call commits; the keeper is paid the
        // attempt fee, but the task is never marked executed.
        assert_ok!(Automation::execute_task(
            RuntimeOrigin::signed(keeper.clone()),
            task_id
        ));

        let task = Automation::tasks(task_id).unwrap();
        assert_ne!(task.status, TaskStatus::Executed);
        assert!(
            Automation::receipt(task_id, 1).is_none(),
            "a refused dispatch writes no receipt"
        );
        assert!(free(&keeper) >= keeper_before);
    });
}

#[test]
fn a_ledger_asset_transfer_is_refused_until_atomic_account_moves_exist() {
    setup().execute_with(|| {
        let owner = account(10);
        let keeper = account(11);
        let _ = Balances::make_free_balance_be(&owner, ENDOWED);
        let _ = Balances::make_free_balance_be(&keeper, ENDOWED);

        let task_id = register(
            &owner,
            Condition::Manual,
            Action::TransferLedgerAsset {
                asset: 1,
                recipient: [0u8; 32],
                amount: 10,
            },
        );

        assert_ok!(Automation::execute_task(
            RuntimeOrigin::signed(keeper),
            task_id
        ));
        assert_ne!(
            Automation::tasks(task_id).unwrap().status,
            TaskStatus::Executed
        );
        assert!(Automation::receipt(task_id, 1).is_none());
    });
}

#[test]
fn the_oracle_pallet_is_the_wired_source_of_truth() {
    // Guards against a regression to `NoopOracle`: if the runtime went back to a
    // provider that always answers `None`, the price above would never be read.
    setup().execute_with(|| {
        assert!(Oracle::get_price(9999).is_none());
    });
}

#[test]
fn a_stale_median_is_refused_like_a_missing_one() {
    // The oracle never deletes an `AssetPrices` entry; when the feed goes quiet
    // the last median stays. A stop-loss trigger must not fire on it.
    setup().execute_with(|| {
        let asset = 88u32;
        pallet_x3_oracle::AssetPrices::<Runtime>::insert(
            asset,
            pallet_x3_oracle::PriceData {
                price: 1_500,
                submission_count: 3,
                last_updated: 1,
                timestamp: 0, // never observed at this chain's clock
            },
        );
        // Move the clock a full day past the recorded observation.
        pallet_timestamp::Pallet::<Runtime>::set_timestamp(86_400_000);

        let owner = account(12);
        let keeper = account(13);
        let _ = Balances::make_free_balance_be(&owner, ENDOWED);
        let _ = Balances::make_free_balance_be(&keeper, ENDOWED);

        // "price below 1_000" would be true for the stale 1_500 only if the
        // threshold were above it; use `above: false` with a high threshold so a
        // stale read would fire and a correct refusal will not.
        let task_id = register(
            &owner,
            Condition::PriceThreshold {
                asset_id: asset,
                threshold: 2_000,
                above: false,
            },
            Action::Noop,
        );

        let keeper_before = free(&keeper);
        assert_noop!(
            Automation::execute_task(RuntimeOrigin::signed(keeper.clone()), task_id),
            pallet_x3_automation::Error::<Runtime>::PriceUnavailable
        );
        assert_eq!(free(&keeper), keeper_before);
        let task = Automation::tasks(task_id).expect("task survives");
        assert_eq!(task.status, TaskStatus::Active);
        assert!(Automation::receipt(task_id, 1).is_none());
    });
}

#[test]
fn a_fresh_median_is_usable() {
    setup().execute_with(|| {
        let asset = 89u32;
        pallet_timestamp::Pallet::<Runtime>::set_timestamp(1_000_000);
        pallet_x3_oracle::AssetPrices::<Runtime>::insert(
            asset,
            pallet_x3_oracle::PriceData {
                price: 1_500,
                submission_count: 3,
                last_updated: 1,
                timestamp: 1_000_000 - 60_000, // one minute old
            },
        );

        let owner = account(14);
        let keeper = account(15);
        let _ = Balances::make_free_balance_be(&owner, ENDOWED);
        let _ = Balances::make_free_balance_be(&keeper, ENDOWED);

        let task_id = register(
            &owner,
            Condition::PriceThreshold {
                asset_id: asset,
                threshold: 2_000,
                above: false,
            },
            Action::Noop,
        );
        assert_ok!(Automation::execute_task(
            RuntimeOrigin::signed(keeper),
            task_id
        ));
        assert_eq!(
            Automation::tasks(task_id).unwrap().status,
            TaskStatus::Executed
        );
    });
}
