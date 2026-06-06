//! Benchmarking for DePIN marketplace pallet (placeholder).
//!
//! Run with: `cargo test -p pallet-depin-marketplace --features runtime-benchmarks`

use super::*;
use frame_benchmarking::v2::*;
use frame_support::{traits::Currency, BoundedVec};
use frame_system::RawOrigin;

use crate::types::{GpuSpecification, GpuTier, ProviderStatus, MAX_GPU_MODEL_LEN};

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_provider() {
        let caller: T::AccountId = whitelisted_caller();
        let min_stake = T::MinProviderStake::get();
        let _ = T::Currency::make_free_balance_be(&caller, min_stake.saturating_mul(2u32.into()));

        let gpu_specs = GpuSpecification {
            model: BoundedVec::<u8, frame_support::pallet_prelude::ConstU32<MAX_GPU_MODEL_LEN>>::try_from(
                b"NVIDIA A100 80GB".to_vec(),
            )
            .unwrap(),
            vram_mb: 80 * 1024,
            compute_units: 108,
            tier: GpuTier::Datacenter,
            tensor_cores: true,
            confidential_compute: false,
            benchmark_score: 10_000,
        };

        #[extrinsic_call]
        register_provider(RawOrigin::Signed(caller.clone()), gpu_specs, 100u32.into());

        let provider = Providers::<T>::get(&caller).expect("provider should exist");
        assert_eq!(provider.account, caller);
        assert_eq!(provider.status, ProviderStatus::Active);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
