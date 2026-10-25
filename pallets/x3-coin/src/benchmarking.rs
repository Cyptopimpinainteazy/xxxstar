use super::*;
use frame_benchmarking::v2::*;
use frame_benchmarking::{whitelisted_caller, BenchmarkError};
use frame_support::traits::EnsureOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn pay_validator_reward() -> Result<(), BenchmarkError> {
        let validator: T::AccountId = whitelisted_caller();
        let origin = T::ValidatorPayoutOrigin::try_successful_origin()
            .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        pay_validator_reward(
            origin,
            validator.clone(),
            X3_BASE_UNITS_PER_X3.saturated_into(),
        );

        assert_eq!(
            ValidatorReserveBalance::<T>::get(),
            (X3_VALIDATOR_SECURITY_RESERVE - X3_BASE_UNITS_PER_X3).saturated_into()
        );
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
