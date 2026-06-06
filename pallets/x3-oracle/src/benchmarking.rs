//! Benchmarking setup for pallet-x3-oracle

use super::*;
use crate::Pallet as OraclePallet;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn authorize_oracle() {
        let caller: T::AccountId = whitelisted_caller();
        let account: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        authorize_oracle(RawOrigin::Root, account.clone());

        assert!(AuthorizedOracles::<T>::contains_key(&account));
    }

    #[benchmark]
    fn deauthorize_oracle() {
        let caller: T::AccountId = whitelisted_caller();
        let account: T::AccountId = whitelisted_caller();

        AuthorizedOracles::<T>::insert(&account, ());

        #[extrinsic_call]
        deauthorize_oracle(RawOrigin::Root, account.clone());

        assert!(!AuthorizedOracles::<T>::contains_key(&account));
    }

    #[benchmark]
    fn submit_price() {
        let caller: T::AccountId = whitelisted_caller();
        let account: T::AccountId = whitelisted_caller();

        AuthorizedOracles::<T>::insert(&account, ());

        #[extrinsic_call]
        submit_price(RawOrigin::Signed(account.clone()), 1, 1000);

        assert!(PriceSubmissions::<T>::contains_key(1, &account));
    }

    impl_benchmark_test_suite!(OraclePallet, crate::mock::new_test_ext(), crate::mock::Test);
}
