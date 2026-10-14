//! Benchmarking setup for `pallet-x3-da`.
//!
//! Both dispatchables charged literals — 15,000 picoseconds to commit a blob and 10,000 to attest a
//! shard — behind a `runtime-benchmarks` feature that had nothing in it, so neither had been
//! measured.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    const BLOB_SIZE: u32 = 1_024;
    const BLOB_HASH: H256 = H256::repeat_byte(0xAB);

    #[benchmark]
    fn submit_blob_commitment() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 10_000_000u32.into());
        // The fee is paid to the treasury; on a fresh chain it has no account yet, and a dead
        // account refuses any deposit below the existential deposit.
        let _ = T::Currency::make_free_balance_be(
            &T::ProtocolTreasury::get(),
            T::Currency::minimum_balance(),
        );

        #[extrinsic_call]
        submit_blob_commitment(RawOrigin::Signed(caller), BLOB_HASH, BLOB_SIZE, None, None);

        assert!(Blobs::<T>::contains_key(BLOB_HASH));
        assert_eq!(TotalBytesCommitted::<T>::get(), BLOB_SIZE as u64);
        Ok(())
    }

    #[benchmark]
    fn submit_shard_proof() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&caller, 10_000_000u32.into());
        // The fee is paid to the treasury; on a fresh chain it has no account yet, and a dead
        // account refuses any deposit below the existential deposit.
        let _ = T::Currency::make_free_balance_be(
            &T::ProtocolTreasury::get(),
            T::Currency::minimum_balance(),
        );
        // A shard proof attests to a blob that has to exist; the commitment is the setup, made the
        // way the extrinsic makes it.
        Pallet::<T>::submit_blob_commitment(
            RawOrigin::Signed(caller.clone()).into(),
            BLOB_HASH,
            BLOB_SIZE,
            None,
            None,
        )?;

        #[extrinsic_call]
        submit_shard_proof(
            RawOrigin::Signed(caller),
            BLOB_HASH,
            0,
            H256::from_low_u64_be(7),
        );

        assert_eq!(ShardProofs::<T>::get(BLOB_HASH).len(), 1);
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
