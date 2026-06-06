//! Weights for pallet-agent-memory.
//!
//! DB-aware weights with proof sizes.  Re-run benchmarks on target hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_agent_memory.
pub trait WeightInfo {
    fn initialize_memory() -> Weight;
    fn append_entry() -> Weight;
    fn append_batch() -> Weight;
    fn update_permissions() -> Weight;
    fn prune_memory() -> Weight;
    fn increase_deposit() -> Weight;
    fn withdraw_deposit() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `AgentMemory::MemoryState` (r:1 w:1), `Balances::Reserves` (r:1 w:1).
    fn initialize_memory() -> Weight {
        Weight::from_parts(53_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }

    /// Storage: `AgentMemory::MemoryState` (r:1 w:1), `AgentMemory::Entries` (r:0 w:1).
    fn append_entry() -> Weight {
        Weight::from_parts(42_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }

    /// Storage: `AgentMemory::MemoryState` (r:1 w:1), `AgentMemory::Entries` (r:0 w:3).
    fn append_batch() -> Weight {
        Weight::from_parts(95_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }

    /// Storage: `AgentMemory::MemoryState` (r:1 w:1).
    fn update_permissions() -> Weight {
        Weight::from_parts(32_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    /// Storage: `AgentMemory::MemoryState` (r:1 w:1), `AgentMemory::Entries` (r:2 w:2).
    fn prune_memory() -> Weight {
        Weight::from_parts(82_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }

    /// Storage: `AgentMemory::MemoryState` (r:1 w:1), `Balances::Reserves` (r:1 w:1).
    fn increase_deposit() -> Weight {
        Weight::from_parts(37_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }

    /// Storage: `AgentMemory::MemoryState` (r:1 w:1), `Balances::Reserves` (r:1 w:1).
    fn withdraw_deposit() -> Weight {
        Weight::from_parts(37_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
}

impl WeightInfo for () {
    fn initialize_memory() -> Weight {
        Weight::from_parts(53_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn append_entry() -> Weight {
        Weight::from_parts(42_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(1, 2))
    }
    fn append_batch() -> Weight {
        Weight::from_parts(95_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(1, 4))
    }
    fn update_permissions() -> Weight {
        Weight::from_parts(32_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn prune_memory() -> Weight {
        Weight::from_parts(82_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(3, 3))
    }
    fn increase_deposit() -> Weight {
        Weight::from_parts(37_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn withdraw_deposit() -> Weight {
        Weight::from_parts(37_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
}
