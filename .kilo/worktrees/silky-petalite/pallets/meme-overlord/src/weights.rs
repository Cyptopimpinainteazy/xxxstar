//! Weight info for Meme Overlord pallet.
//!
//! DB-aware weights with proof sizes.  Re-run benchmarks on target hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

pub trait WeightInfo {
    fn register_template() -> Weight;
    fn generate_meme() -> Weight;
    fn like_meme() -> Weight;
    fn share_meme() -> Weight;
    fn deactivate_template() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `MemeOverlord::Templates` (r:1 w:1).
    fn register_template() -> Weight {
        Weight::from_parts(32_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Storage: `MemeOverlord::Templates` (r:1 w:1), `MemeOverlord::Memes` (r:0 w:1).
    fn generate_meme() -> Weight {
        Weight::from_parts(52_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    /// Storage: `MemeOverlord::Memes` (r:1 w:1).
    fn like_meme() -> Weight {
        Weight::from_parts(17_000_000, 128)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Storage: `MemeOverlord::Memes` (r:1 w:1).
    fn share_meme() -> Weight {
        Weight::from_parts(17_000_000, 128)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Storage: `MemeOverlord::Templates` (r:1 w:1).
    fn deactivate_template() -> Weight {
        Weight::from_parts(22_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

impl WeightInfo for () {
    fn register_template() -> Weight {
        Weight::from_parts(32_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn generate_meme() -> Weight {
        Weight::from_parts(52_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(1, 2))
    }
    fn like_meme() -> Weight {
        Weight::from_parts(17_000_000, 128).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn share_meme() -> Weight {
        Weight::from_parts(17_000_000, 128).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn deactivate_template() -> Weight {
        Weight::from_parts(22_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
}
