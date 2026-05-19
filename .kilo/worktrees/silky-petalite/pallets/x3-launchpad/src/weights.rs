//! Auto-generated weight stubs for pallet-x3-launchpad.
//!
//! Run `cargo benchmark` against real hardware to populate concrete values.

use frame_support::weights::{constants::RocksDbWeight, Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn create_launch() -> Weight;
    fn contribute() -> Weight;
    fn finalize_launch() -> Weight;
    fn claim_refund() -> Weight;
    fn claim_allocation() -> Weight;
    fn cancel_launch() -> Weight;
    fn withdraw_raised_funds() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn create_launch() -> Weight {
        Weight::from_parts(28_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }

    fn contribute() -> Weight {
        Weight::from_parts(22_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn finalize_launch() -> Weight {
        Weight::from_parts(18_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn claim_refund() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn claim_allocation() -> Weight {
        Weight::from_parts(22_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn cancel_launch() -> Weight {
        Weight::from_parts(18_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn withdraw_raised_funds() -> Weight {
        Weight::from_parts(18_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }
}

impl WeightInfo for () {
    fn create_launch() -> Weight { Weight::zero() }
    fn contribute() -> Weight { Weight::zero() }
    fn finalize_launch() -> Weight { Weight::zero() }
    fn claim_refund() -> Weight { Weight::zero() }
    fn claim_allocation() -> Weight { Weight::zero() }
    fn cancel_launch() -> Weight { Weight::zero() }
    fn withdraw_raised_funds() -> Weight { Weight::zero() }
}
