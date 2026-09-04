//! Weights for pallet-x3-agent-registry.
//!
//! DB-aware weights with proof sizes. Re-run benchmarks on target hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_x3_agent_registry.
pub trait WeightInfo {
    fn register_agent() -> Weight;
    fn bind_atlas_id() -> Weight;
    fn update_operator() -> Weight;
    fn update_permissions() -> Weight;
    fn update_quota() -> Weight;
    fn suspend_agent() -> Weight;
    fn reactivate_agent() -> Weight;
    fn terminate_agent() -> Weight;
    fn register_policy() -> Weight;
    fn remove_blacklist() -> Weight;
    fn post_bond() -> Weight;
    fn release_bond() -> Weight;
    fn slash_bond() -> Weight;
    fn record_consumption() -> Weight;
    fn update_reputation() -> Weight;
    fn distribute_rewards() -> Weight;
    fn emit_action() -> Weight;
    fn set_proof_reward() -> Weight;
    fn fund_reward_pool() -> Weight;
    fn claim_rewards() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn register_agent() -> Weight {
        Weight::from_parts(72_000_000, 1024)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(8_u64))
    }
    fn bind_atlas_id() -> Weight {
        Weight::from_parts(35_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn update_operator() -> Weight {
        Weight::from_parts(42_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
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
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn reactivate_agent() -> Weight {
        Weight::from_parts(37_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn terminate_agent() -> Weight {
        Weight::from_parts(62_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn register_policy() -> Weight {
        Weight::from_parts(35_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn remove_blacklist() -> Weight {
        Weight::from_parts(20_000_000, 128)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn post_bond() -> Weight {
        Weight::from_parts(55_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
    fn release_bond() -> Weight {
        Weight::from_parts(45_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn slash_bond() -> Weight {
        Weight::from_parts(65_000_000, 1024)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(6_u64))
    }
    fn record_consumption() -> Weight {
        Weight::from_parts(37_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn update_reputation() -> Weight {
        Weight::from_parts(32_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn distribute_rewards() -> Weight {
        Weight::from_parts(52_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn emit_action() -> Weight {
        Weight::from_parts(22_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(0_u64))
    }
    fn set_proof_reward() -> Weight {
        Weight::from_parts(25_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn fund_reward_pool() -> Weight {
        Weight::from_parts(35_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn claim_rewards() -> Weight {
        Weight::from_parts(45_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
}

impl WeightInfo for () {
    fn register_agent() -> Weight {
        Weight::from_parts(72_000_000, 1024).saturating_add(RocksDbWeight::get().reads_writes(4, 8))
    }
    fn bind_atlas_id() -> Weight {
        Weight::from_parts(35_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn update_operator() -> Weight {
        Weight::from_parts(42_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn update_permissions() -> Weight {
        Weight::from_parts(32_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn update_quota() -> Weight {
        Weight::from_parts(32_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn suspend_agent() -> Weight {
        Weight::from_parts(37_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 2))
    }
    fn reactivate_agent() -> Weight {
        Weight::from_parts(37_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 2))
    }
    fn terminate_agent() -> Weight {
        Weight::from_parts(62_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(3, 6))
    }
    fn register_policy() -> Weight {
        Weight::from_parts(35_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(1, 2))
    }
    fn remove_blacklist() -> Weight {
        Weight::from_parts(20_000_000, 128).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn post_bond() -> Weight {
        Weight::from_parts(55_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(3, 4))
    }
    fn release_bond() -> Weight {
        Weight::from_parts(45_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 3))
    }
    fn slash_bond() -> Weight {
        Weight::from_parts(65_000_000, 1024).saturating_add(RocksDbWeight::get().reads_writes(4, 6))
    }
    fn record_consumption() -> Weight {
        Weight::from_parts(37_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(3, 2))
    }
    fn update_reputation() -> Weight {
        Weight::from_parts(32_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn distribute_rewards() -> Weight {
        Weight::from_parts(52_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(3, 3))
    }
    fn emit_action() -> Weight {
        Weight::from_parts(22_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 0))
    }
    fn set_proof_reward() -> Weight {
        Weight::from_parts(25_000_000, 256).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn fund_reward_pool() -> Weight {
        Weight::from_parts(35_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn claim_rewards() -> Weight {
        Weight::from_parts(45_000_000, 512).saturating_add(RocksDbWeight::get().reads_writes(3, 3))
    }
}
