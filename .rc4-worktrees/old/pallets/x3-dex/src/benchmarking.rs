//! Benchmarking setup for pallet-x3-dex

use super::*;
use crate::Pallet as DEXPallet;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use x3_dex::amm_pools::TokenId;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_pool() {
        let caller: T::AccountId = whitelisted_caller();
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let fee_basis_points = 30;

        #[extrinsic_call]
        create_pool(
            RawOrigin::Signed(caller),
            token_a,
            token_b,
            fee_basis_points,
        );
    }

    #[benchmark]
    fn add_liquidity() {
        let caller: T::AccountId = whitelisted_caller();
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool first
        assert_ok!(DEXPallet::<T>::create_pool(
            RawOrigin::Signed(caller.clone()).into(),
            token_a.clone(),
            token_b.clone(),
            30
        ));

        #[extrinsic_call]
        add_liquidity(RawOrigin::Signed(caller), 0, 1000, 1000, 900, 900);
    }

    #[benchmark]
    fn remove_liquidity() {
        let caller: T::AccountId = whitelisted_caller();
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool and add liquidity first
        assert_ok!(DEXPallet::<T>::create_pool(
            RawOrigin::Signed(caller.clone()).into(),
            token_a.clone(),
            token_b.clone(),
            30
        ));
        assert_ok!(DEXPallet::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100000,
            100000,
            90000,
            90000
        ));

        // Get position ID (simplified - would need to track this)
        let position_id = 0;

        #[extrinsic_call]
        remove_liquidity(RawOrigin::Signed(caller), position_id, 50000, 45000, 45000);
    }

    #[benchmark]
    fn swap() {
        let caller: T::AccountId = whitelisted_caller();
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 0,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 1,
        };

        // Create pool and add liquidity first
        assert_ok!(DEXPallet::<T>::create_pool(
            RawOrigin::Signed(caller.clone()).into(),
            token_a.clone(),
            token_b.clone(),
            30
        ));
        assert_ok!(DEXPallet::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100000,
            100000,
            90000,
            90000
        ));

        #[extrinsic_call]
        swap(RawOrigin::Signed(caller), 0, token_a, 1000, 900);
    }

    impl_benchmark_test_suite!(DEXPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
