//! Weights for pallet-treasury.
//!
//! DB-aware weights with proof sizes.  Re-run benchmarks on target hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_treasury.
pub trait WeightInfo {
    fn submit_proposal() -> Weight;
    fn approve_proposal() -> Weight;
    fn execute_proposal() -> Weight;
    fn reject_proposal() -> Weight;
    fn create_recurring_payment() -> Weight;
    fn cancel_recurring_payment() -> Weight;
    fn register_yield_strategy() -> Weight;
    fn execute_yield_strategy() -> Weight;
    fn report_yield_return() -> Weight;
    fn deactivate_yield_strategy() -> Weight;
    fn pause() -> Weight;
    fn unpause() -> Weight;
    fn update_signers() -> Weight;
    fn deposit() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn submit_proposal() -> Weight {
        Weight::from_parts(52_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn approve_proposal() -> Weight {
        Weight::from_parts(42_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn execute_proposal() -> Weight {
        Weight::from_parts(65_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn reject_proposal() -> Weight {
        Weight::from_parts(32_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn create_recurring_payment() -> Weight {
        Weight::from_parts(47_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn cancel_recurring_payment() -> Weight {
        Weight::from_parts(27_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn register_yield_strategy() -> Weight {
        Weight::from_parts(52_000_000, 512)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn execute_yield_strategy() -> Weight {
        Weight::from_parts(72_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn report_yield_return() -> Weight {
        Weight::from_parts(57_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn deactivate_yield_strategy() -> Weight {
        Weight::from_parts(27_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn pause() -> Weight {
        Weight::from_parts(22_000_000, 128)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn unpause() -> Weight {
        Weight::from_parts(22_000_000, 128)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn update_signers() -> Weight {
        Weight::from_parts(37_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn deposit() -> Weight {
        Weight::from_parts(42_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
}

impl WeightInfo for () {
    fn submit_proposal() -> Weight {
        Weight::from_parts(52_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(1, 2))
    }
    fn approve_proposal() -> Weight {
        Weight::from_parts(42_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 1))
    }
    fn execute_proposal() -> Weight {
        Weight::from_parts(65_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(4, 3))
    }
    fn reject_proposal() -> Weight {
        Weight::from_parts(32_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn create_recurring_payment() -> Weight {
        Weight::from_parts(47_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn cancel_recurring_payment() -> Weight {
        Weight::from_parts(27_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn register_yield_strategy() -> Weight {
        Weight::from_parts(52_000_000, 512).saturating_add(RocksDbWeight::get().writes(1))
    }
    fn execute_yield_strategy() -> Weight {
        Weight::from_parts(72_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn report_yield_return() -> Weight {
        Weight::from_parts(57_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn deactivate_yield_strategy() -> Weight {
        Weight::from_parts(27_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn pause() -> Weight {
        Weight::from_parts(22_000_000, 128).saturating_add(RocksDbWeight::get().writes(1))
    }
    fn unpause() -> Weight {
        Weight::from_parts(22_000_000, 128).saturating_add(RocksDbWeight::get().writes(1))
    }
    fn update_signers() -> Weight {
        Weight::from_parts(37_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn deposit() -> Weight {
        Weight::from_parts(42_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
}
