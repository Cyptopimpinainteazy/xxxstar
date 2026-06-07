//! Weights for pallet-swarm.
//!
//! Placeholder weights - should be benchmarked for production use.

use frame_support::weights::{constants::RocksDbWeight, Weight};
use sp_std;

/// Weight functions for pallet-swarm.
pub trait WeightInfo {
    fn register_contributor() -> Weight;
    fn deregister_contributor() -> Weight;
    fn heartbeat() -> Weight;
    fn submit_task() -> Weight;
    fn claim_task() -> Weight;
    fn submit_result() -> Weight;
    fn start_jury_session() -> Weight;
    fn commit_vote() -> Weight;
    fn reveal_vote() -> Weight;
    fn finalize_session() -> Weight;
    fn cancel_task() -> Weight;
    fn update_config() -> Weight;
    fn slash_contributor() -> Weight;
}

/// Substrate weight implementation with placeholder values.
pub struct SubstrateWeight<T>(sp_std::marker::PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn register_contributor() -> Weight {
        Weight::from_parts(50_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(5, 5))
    }

    fn deregister_contributor() -> Weight {
        Weight::from_parts(40_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(3, 3))
    }

    fn heartbeat() -> Weight {
        Weight::from_parts(15_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }

    fn submit_task() -> Weight {
        Weight::from_parts(45_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(3, 4))
    }

    fn claim_task() -> Weight {
        Weight::from_parts(35_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(3, 2))
    }

    fn submit_result() -> Weight {
        Weight::from_parts(40_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(4, 3))
    }

    fn start_jury_session() -> Weight {
        Weight::from_parts(35_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(2, 3))
    }

    fn commit_vote() -> Weight {
        Weight::from_parts(25_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(2, 1))
    }

    fn reveal_vote() -> Weight {
        Weight::from_parts(30_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(3, 2))
    }

    fn finalize_session() -> Weight {
        Weight::from_parts(60_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(6, 6))
    }

    fn cancel_task() -> Weight {
        Weight::from_parts(25_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }

    fn update_config() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(0, 1))
    }

    fn slash_contributor() -> Weight {
        Weight::from_parts(45_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(3, 3))
    }
}
