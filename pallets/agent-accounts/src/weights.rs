//! Weights for pallet-agent-accounts.
//!
//! DB-aware weights with proof sizes.  Re-run benchmarks on target hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_agent_accounts.
pub trait WeightInfo {
    fn register_agent() -> Weight;
    fn update_operator() -> Weight;
    fn update_permissions() -> Weight;
    fn update_quota() -> Weight;
    fn suspend_agent() -> Weight;
    fn reactivate_agent() -> Weight;
    fn terminate_agent() -> Weight;
    fn record_consumption() -> Weight;
    fn update_reputation() -> Weight;
    fn emit_action() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `AgentAccounts::Agents` (r:1 w:1), `Balances::Reserves` (r:1 w:1).
    fn register_agent() -> Weight {
        Weight::from_parts(62_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    /// Storage: `AgentAccounts::Agents` (r:1 w:1).
    fn update_operator() -> Weight {
        Weight::from_parts(42_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn update_permissions() -> Weight {
        Weight::from_parts(32_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn update_quota() -> Weight {
        Weight::from_parts(32_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn suspend_agent() -> Weight {
        Weight::from_parts(37_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn reactivate_agent() -> Weight {
        Weight::from_parts(37_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Storage: `AgentAccounts::Agents` (r:1 w:1), `Balances::Reserves` (r:1 w:1) — unreserve.
    fn terminate_agent() -> Weight {
        Weight::from_parts(52_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    /// Storage: `AgentAccounts::Agents` (r:1 w:1), `AgentAccounts::ConsumptionLog` (r:0 w:1).
    fn record_consumption() -> Weight {
        Weight::from_parts(27_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn update_reputation() -> Weight {
        Weight::from_parts(32_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Storage: `AgentAccounts::Agents` (r:1 w:0), `AgentAccounts::ActionLog` (r:0 w:1).
    fn emit_action() -> Weight {
        Weight::from_parts(22_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

impl WeightInfo for () {
    fn register_agent() -> Weight {
        Weight::from_parts(62_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn update_operator() -> Weight {
        Weight::from_parts(42_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn update_permissions() -> Weight {
        Weight::from_parts(32_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn update_quota() -> Weight {
        Weight::from_parts(32_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn suspend_agent() -> Weight {
        Weight::from_parts(37_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn reactivate_agent() -> Weight {
        Weight::from_parts(37_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn terminate_agent() -> Weight {
        Weight::from_parts(52_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn record_consumption() -> Weight {
        Weight::from_parts(27_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 2))
    }
    fn update_reputation() -> Weight {
        Weight::from_parts(32_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn emit_action() -> Weight {
        Weight::from_parts(22_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
}
