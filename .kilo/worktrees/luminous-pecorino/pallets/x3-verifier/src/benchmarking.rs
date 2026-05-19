//! Benchmarking module for pallet-x3-verifier.
//!
//! Provides benchmarks for the x3_crypto_sort extrinsic and other
//! verifier operations that benefit from sorted multi-key verification.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as VerifierPallet;
use frame_benchmarking::{benchmarks, whitelisted_caller, ImplBenchmarkTestSuite};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::traits::Bounded;
use sp_std::vec::Vec;

/// Helper to create a bounded signature for benchmarking
fn bounded_signature(size: usize) -> BoundedVec<u8, ConstU32<65>> {
    let mut sig = vec![0u8; size.min(65)];
    sig[0] = 0x30; // DER sequence tag
    sig[size.saturating_sub(1)] = 0x01; // Dummy byte
    BoundedVec::try_from(sig).unwrap_or_else(|_| {
        let mut fallback = vec![0u8; size.min(65)];
        fallback[0] = 0x30;
        BoundedVec::try_from(fallback).unwrap()
    })
}

#[benchmarks]
mod benchmarks {
    use super::*;

    /// Benchmark x3_crypto_sort with varying numbers of executor keys.
    ///
    /// Measures the cost of sorting executor keys and their corresponding
    /// signatures using insertion sort. This is called by `verify_multikey_proof`
    /// to ensure deterministic ordering before signature verification.
    ///
    /// The sort is O(n²) in the worst case but efficient for small n (bounded
    /// by `MaxStateChanges`), and ensures deterministic ordering to prevent
    /// replay attacks exploiting signature ordering ambiguity.
    #[benchmark]
    fn x3_crypto_sort_1() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let account: T::AccountId = whitelisted_caller();

        // Setup: Register an executor so key lookup succeeds
        Executors::<T>::insert(
            &account,
            ExecutorRecord {
                account: account.clone(),
                stake: T::MinExecutorStake::get(),
                jobs_completed: 0,
                jobs_failed: 0,
                total_rewards: 0u32.into(),
                active: true,
                reputation: 50,
            },
        );

        let mut executor_keys: Vec<T::AccountId> = Vec::new();
        let mut signatures: Vec<BoundedVec<u8, ConstU32<65>>> = Vec::new();

        // Single executor key
        executor_keys.push(account.clone());
        signatures.push(bounded_signature(65));

        #[extrinsic_call]
        x3_crypto_sort(
            RawOrigin::Signed(caller),
            executor_keys,
            signatures,
        );

        Ok(())
    }

    /// Benchmark x3_crypto_sort with 10 executor keys
    #[benchmark]
    fn x3_crypto_sort_10() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();

        let mut executor_keys: Vec<T::AccountId> = Vec::new();
        let mut signatures: Vec<BoundedVec<u8, ConstU32<65>>> = Vec::new();

        for i in 0..10 {
            let account = frame_benchmarking::account::<T::AccountId>("executor", i, 0);
            // Register each executor
            Executors::<T>::insert(
                &account,
                ExecutorRecord {
                    account: account.clone(),
                    stake: T::MinExecutorStake::get(),
                    jobs_completed: 0,
                    jobs_failed: 0,
                    total_rewards: 0u32.into(),
                    active: true,
                    reputation: 50,
                },
            );
            executor_keys.push(account);
            signatures.push(bounded_signature(65));
        }

        #[extrinsic_call]
        x3_crypto_sort(
            RawOrigin::Signed(caller),
            executor_keys,
            signatures,
        );

        Ok(())
    }

    /// Benchmark x3_crypto_sort with 50 executor keys
    #[benchmark]
    fn x3_crypto_sort_50() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();

        let mut executor_keys: Vec<T::AccountId> = Vec::new();
        let mut signatures: Vec<BoundedVec<u8, ConstU32<65>>> = Vec::new();

        for i in 0..50 {
            let account = frame_benchmarking::account::<T::AccountId>("executor", i, 0);
            Executors::<T>::insert(
                &account,
                ExecutorRecord {
                    account: account.clone(),
                    stake: T::MinExecutorStake::get(),
                    jobs_completed: 0,
                    jobs_failed: 0,
                    total_rewards: 0u32.into(),
                    active: true,
                    reputation: 50,
                },
            );
            executor_keys.push(account);
            signatures.push(bounded_signature(65));
        }

        #[extrinsic_call]
        x3_crypto_sort(
            RawOrigin::Signed(caller),
            executor_keys,
            signatures,
        );

        Ok(())
    }

    /// Benchmark x3_crypto_sort with maximum executor keys (MaxStateChanges)
    #[benchmark]
    fn x3_crypto_sort_max() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let max_keys = T::MaxStateChanges::get() as usize;

        let mut executor_keys: Vec<T::AccountId> = Vec::with_capacity(max_keys);
        let mut signatures: Vec<BoundedVec<u8, ConstU32<65>>> = Vec::with_capacity(max_keys);

        for i in 0..max_keys {
            let account = frame_benchmarking::account::<T::AccountId>("executor", i as u32, 0);
            Executors::<T>::insert(
                &account,
                ExecutorRecord {
                    account: account.clone(),
                    stake: T::MinExecutorStake::get(),
                    jobs_completed: 0,
                    jobs_failed: 0,
                    total_rewards: 0u32.into(),
                    active: true,
                    reputation: 50,
                },
            );
            executor_keys.push(account);
            signatures.push(bounded_signature(65));
        }

        #[extrinsic_call]
        x3_crypto_sort(
            RawOrigin::Signed(caller),
            executor_keys,
            signatures,
        );

        Ok(())
    }

    impl_benchmark_test_suite!(VerifierPallet, crate::mock::new_test_ext(), crate::mock::Test);
}