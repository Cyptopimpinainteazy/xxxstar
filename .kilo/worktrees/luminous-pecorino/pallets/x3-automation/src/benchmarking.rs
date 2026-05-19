//! Benchmarking setup for pallet-x3-automation

use super::*;
use crate::Pallet as AutomationPallet;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use x3_automation::{Action, Condition};

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_task() {
        let caller: T::AccountId = whitelisted_caller();
        let condition = Condition::BlockNumber(1000);
        let action = Action::Custom({ let mut a = [0u8; 64]; a[..3].copy_from_slice(&[1,2,3]); a });
        let max_fee = T::BaseRegistrationFee::get().saturating_mul(2u32.into());

        // Give caller enough balance
        let _ = T::Currency::make_free_balance_be(&caller, max_fee.saturating_mul(2u32.into()));

        #[extrinsic_call]
        register_task(RawOrigin::Signed(caller), condition, action, max_fee);
    }

    #[benchmark]
    fn cancel_task() {
        let caller: T::AccountId = whitelisted_caller();
        let condition = Condition::BlockNumber(1000);
        let action = Action::Custom({ let mut a = [0u8; 64]; a[..3].copy_from_slice(&[1,2,3]); a });
        let max_fee = T::BaseRegistrationFee::get().saturating_mul(2u32.into());

        // Give caller enough balance
        let _ = T::Currency::make_free_balance_be(&caller, max_fee.saturating_mul(2u32.into()));

        // Register a task
        assert_ok!(AutomationPallet::<T>::register_task(
            RawOrigin::Signed(caller.clone()).into(),
            condition,
            action,
            max_fee
        ));

        let task_id = AutomationPallet::<T>::account_tasks(caller.clone())[0];

        #[extrinsic_call]
        cancel_task(RawOrigin::Signed(caller), task_id);
    }

    #[benchmark]
    fn execute_task() {
        let caller: T::AccountId = whitelisted_caller();
        let keeper: T::AccountId = whitelisted_caller();
        let condition = Condition::BlockNumber(1); // Condition met immediately
        let action = Action::Custom({ let mut a = [0u8; 64]; a[..3].copy_from_slice(&[1,2,3]); a });
        let max_fee = T::BaseRegistrationFee::get().saturating_mul(2u32.into());

        // Give caller enough balance
        let _ = T::Currency::make_free_balance_be(&caller, max_fee.saturating_mul(2u32.into()));

        // Register a task
        assert_ok!(AutomationPallet::<T>::register_task(
            RawOrigin::Signed(caller).into(),
            condition,
            action,
            max_fee
        ));

        let task_id = AutomationPallet::<T>::account_tasks(caller)[0];

        #[extrinsic_call]
        execute_task(RawOrigin::Signed(keeper), task_id);
    }

    impl_benchmark_test_suite!(
        AutomationPallet,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
