//! Benchmarking setup for pallet-x3-vrf

use super::*;
use crate::Pallet as VrfPallet;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn request_randomness() {
        let caller: T::AccountId = whitelisted_caller();
        let seed = vec![0u8; 32];
        let max_fee = T::BaseFee::get().saturating_mul(2u32.into());

        // Give caller enough balance
        let _ = T::Currency::make_free_balance_be(&caller, max_fee.saturating_mul(2u32.into()));

        #[extrinsic_call]
        request_randomness(RawOrigin::Signed(caller), seed, max_fee);
    }

    #[benchmark]
    fn fulfill_randomness() {
        let caller: T::AccountId = whitelisted_caller();
        let requester: T::AccountId = whitelisted_caller();
        let seed = vec![0u8; 32];
        let max_fee = T::BaseFee::get().saturating_mul(2u32.into());

        // Give requester enough balance
        let _ = T::Currency::make_free_balance_be(&requester, max_fee.saturating_mul(2u32.into()));

        // Create a request
        assert_ok!(VrfPallet::<T>::request_randomness(
            RawOrigin::Signed(requester).into(),
            seed.clone(),
            max_fee
        ));

        let request_id = VrfPallet::<T>::account_requests(requester)[0];

        #[extrinsic_call]
        fulfill_randomness(RawOrigin::Signed(caller), request_id);
    }

    #[benchmark]
    fn cancel_randomness() {
        let caller: T::AccountId = whitelisted_caller();
        let seed = vec![0u8; 32];
        let max_fee = T::BaseFee::get().saturating_mul(2u32.into());

        // Give caller enough balance
        let _ = T::Currency::make_free_balance_be(&caller, max_fee.saturating_mul(2u32.into()));

        // Create a request
        assert_ok!(VrfPallet::<T>::request_randomness(
            RawOrigin::Signed(caller).into(),
            seed,
            max_fee
        ));

        let request_id = VrfPallet::<T>::account_requests(caller)[0];

        #[extrinsic_call]
        cancel_randomness(RawOrigin::Signed(caller), request_id);
    }

    impl_benchmark_test_suite!(VrfPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
