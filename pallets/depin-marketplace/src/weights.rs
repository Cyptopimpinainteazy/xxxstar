//! Weight info for DePIN marketplace pallet.
//!
//! DB-aware weights with proof sizes.  Re-run benchmarks on target hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions for the DePIN marketplace pallet.
pub trait WeightInfo {
    fn register_provider() -> Weight;
    fn deregister_provider() -> Weight;
    fn pause_provider() -> Weight;
    fn resume_provider() -> Weight;
    fn submit_order() -> Weight;
    fn accept_order() -> Weight;
    fn complete_job() -> Weight;
    fn report_job_failure() -> Weight;
    fn cancel_order() -> Weight;
    fn pause_marketplace() -> Weight;
    fn resume_marketplace() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `DepinMarketplace::Providers` (r:1 w:1), `Balances::Reserves` (r:1 w:1).
    fn register_provider() -> Weight {
        Weight::from_parts(52_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn deregister_provider() -> Weight {
        Weight::from_parts(42_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn pause_provider() -> Weight {
        Weight::from_parts(22_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn resume_provider() -> Weight {
        Weight::from_parts(22_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Storage: `DepinMarketplace::Orders` (r:1 w:1), `Balances::Reserves` (r:2 w:1),
    /// `DepinMarketplace::Providers` (r:1 w:0).
    fn submit_order() -> Weight {
        Weight::from_parts(62_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn accept_order() -> Weight {
        Weight::from_parts(72_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Storage: `DepinMarketplace::Orders` (r:1 w:1), `Balances` (r:2 w:2) — transfer.
    fn complete_job() -> Weight {
        Weight::from_parts(82_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn report_job_failure() -> Weight {
        Weight::from_parts(72_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn cancel_order() -> Weight {
        Weight::from_parts(42_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn pause_marketplace() -> Weight {
        Weight::from_parts(12_000_000, 128)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn resume_marketplace() -> Weight {
        Weight::from_parts(12_000_000, 128)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

impl WeightInfo for () {
    fn register_provider() -> Weight {
        Weight::from_parts(52_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn deregister_provider() -> Weight {
        Weight::from_parts(42_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn pause_provider() -> Weight {
        Weight::from_parts(22_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn resume_provider() -> Weight {
        Weight::from_parts(22_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn submit_order() -> Weight {
        Weight::from_parts(62_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(4, 2))
    }
    fn accept_order() -> Weight {
        Weight::from_parts(72_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 1))
    }
    fn complete_job() -> Weight {
        Weight::from_parts(82_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(3, 3))
    }
    fn report_job_failure() -> Weight {
        Weight::from_parts(72_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(3, 3))
    }
    fn cancel_order() -> Weight {
        Weight::from_parts(42_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn pause_marketplace() -> Weight {
        Weight::from_parts(12_000_000, 128).saturating_add(RocksDbWeight::get().writes(1))
    }
    fn resume_marketplace() -> Weight {
        Weight::from_parts(12_000_000, 128).saturating_add(RocksDbWeight::get().writes(1))
    }
}
