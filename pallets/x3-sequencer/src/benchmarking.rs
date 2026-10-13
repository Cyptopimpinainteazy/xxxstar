//! Benchmarking setup for `pallet-x3-sequencer`.
//!
//! `submit_transaction` charged 10,000 picoseconds while it reserved a per-byte sequencing fee,
//! bumped the global sequence and pushed into the pending batch — and it had never been measured:
//! the pallet declared a `runtime-benchmarks` feature with nothing behind it.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    /// A payload the chain's own cap accepts; the fee is charged per byte, so the measured call
    /// includes the fee transfer.
    const PAYLOAD_SIZE: u32 = 1_024;

    #[benchmark]
    fn submit_transaction() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 10_000_000u32.into());
        // The fee is paid to the treasury; on a fresh chain it has no account yet, and a dead
        // account refuses any deposit below the existential deposit.
        let _ = T::Currency::make_free_balance_be(
            &T::ProtocolTreasury::get(),
            T::Currency::minimum_balance(),
        );

        #[extrinsic_call]
        submit_transaction(
            RawOrigin::Signed(caller),
            H256::from_low_u64_be(1),
            PAYLOAD_SIZE,
            0,
        );

        assert_eq!(GlobalSequence::<T>::get(), 1);
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
