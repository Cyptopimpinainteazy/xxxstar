//! Auto-generated weight stubs for pallet-x3-auction.
//!
//! Run `cargo benchmark` against real hardware to populate concrete values.

use frame_support::weights::{constants::RocksDbWeight, Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn create_auction() -> Weight;
    fn place_bid() -> Weight;
    fn cancel_auction() -> Weight;
    fn settle_auction() -> Weight;
    fn extend_auction() -> Weight;
    fn force_cancel() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn create_auction() -> Weight {
        Weight::from_parts(25_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(4_u64))
    }

    fn place_bid() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn cancel_auction() -> Weight {
        Weight::from_parts(18_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn settle_auction() -> Weight {
        Weight::from_parts(22_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    fn extend_auction() -> Weight {
        Weight::from_parts(15_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    fn force_cancel() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
}

impl WeightInfo for () {
    fn create_auction() -> Weight { Weight::zero() }
    fn place_bid() -> Weight { Weight::zero() }
    fn cancel_auction() -> Weight { Weight::zero() }
    fn settle_auction() -> Weight { Weight::zero() }
    fn extend_auction() -> Weight { Weight::zero() }
    fn force_cancel() -> Weight { Weight::zero() }
}
