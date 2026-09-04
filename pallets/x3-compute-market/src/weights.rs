//! Weight stubs for pallet-x3-compute-market.
//!
//! Run `cargo benchmark` against representative hardware to populate concrete values.

use frame_support::weights::{constants::RocksDbWeight, Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn stake_as_provider() -> Weight;
    fn create_listing() -> Weight;
    fn pause_listing() -> Weight;
    fn resume_listing() -> Weight;
    fn rent_compute() -> Weight;
    fn complete_session() -> Weight;
    fn dispute_session() -> Weight;
    fn resolve_dispute() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn stake_as_provider() -> Weight {
        Weight::from_parts(10_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    fn create_listing() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }

    fn pause_listing() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    fn resume_listing() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    fn rent_compute() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }

    fn complete_session() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }

    fn dispute_session() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    fn resolve_dispute() -> Weight {
        Weight::from_parts(30_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
}

impl WeightInfo for () {
    fn stake_as_provider() -> Weight {
        Weight::zero()
    }
    fn create_listing() -> Weight {
        Weight::zero()
    }
    fn pause_listing() -> Weight {
        Weight::zero()
    }
    fn resume_listing() -> Weight {
        Weight::zero()
    }
    fn rent_compute() -> Weight {
        Weight::zero()
    }
    fn complete_session() -> Weight {
        Weight::zero()
    }
    fn dispute_session() -> Weight {
        Weight::zero()
    }
    fn resolve_dispute() -> Weight {
        Weight::zero()
    }
}
