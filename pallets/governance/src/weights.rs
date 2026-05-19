//! Weights for pallet-governance.
//!
//! These weights include realistic DB read/write costs and proof sizes derived from
//! storage item sizes.  Re-run `benchmark pallet` on target hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions for `pallet_governance`.
pub trait WeightInfo {
    fn submit_proposal() -> Weight;
    fn vote() -> Weight;
    fn delegate() -> Weight;
    fn undelegate() -> Weight;
    fn fast_track() -> Weight;
    fn cancel_proposal() -> Weight;
    fn finalize_proposal() -> Weight;
    fn unlock() -> Weight;
    fn update_config() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `Governance::Config` (r:1), `Governance::Proposals` (r:0 w:1),
    /// `Balances::Reserves` (r:1 w:1) — deposit reservation.
    fn submit_proposal() -> Weight {
        Weight::from_parts(52_000_000, 1_890)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }

    /// Storage: `Governance::Proposals` (r:1), `Governance::Voting` (r:1 w:1).
    fn vote() -> Weight {
        Weight::from_parts(42_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    /// Storage: `Governance::Delegations` (r:1 w:1), `Governance::VotingState` (r:1 w:1).
    fn delegate() -> Weight {
        Weight::from_parts(32_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }

    /// Storage: `Governance::Delegations` (r:1 w:1), `Governance::VotingState` (r:1 w:1).
    fn undelegate() -> Weight {
        Weight::from_parts(27_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }

    /// Storage: `Governance::Proposals` (r:1 w:1), `Governance::Config` (r:1).
    fn fast_track() -> Weight {
        Weight::from_parts(22_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    /// Storage: `Governance::Proposals` (r:1 w:1), `Balances::Reserves` (r:1 w:1) — unreserve.
    fn cancel_proposal() -> Weight {
        Weight::from_parts(37_000_000, 512)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }

    /// Storage: `Governance::Proposals` (r:1 w:1), `Governance::Voting` (r:1 w:1),
    /// `Balances::Reserves` (r:2 w:2) — unlock all deposits.
    fn finalize_proposal() -> Weight {
        Weight::from_parts(105_000_000, 2_048)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }

    /// Storage: `Governance::VotingState` (r:1 w:1).
    fn unlock() -> Weight {
        Weight::from_parts(32_000_000, 256)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    /// Storage: `Governance::Config` (r:0 w:1).
    fn update_config() -> Weight {
        Weight::from_parts(17_000_000, 256)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

/// Weights for unit tests — use fixed `RocksDbWeight` so tests do not require
/// a full runtime with DB weight configuration.
impl WeightInfo for () {
    fn submit_proposal() -> Weight {
        Weight::from_parts(52_000_000, 1_890)
            .saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn vote() -> Weight {
        Weight::from_parts(42_000_000, 512)
            .saturating_add(RocksDbWeight::get().reads_writes(2, 1))
    }
    fn delegate() -> Weight {
        Weight::from_parts(32_000_000, 512)
            .saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn undelegate() -> Weight {
        Weight::from_parts(27_000_000, 512)
            .saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn fast_track() -> Weight {
        Weight::from_parts(22_000_000, 512)
            .saturating_add(RocksDbWeight::get().reads_writes(2, 1))
    }
    fn cancel_proposal() -> Weight {
        Weight::from_parts(37_000_000, 512)
            .saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn finalize_proposal() -> Weight {
        Weight::from_parts(105_000_000, 2_048)
            .saturating_add(RocksDbWeight::get().reads_writes(4, 4))
    }
    fn unlock() -> Weight {
        Weight::from_parts(32_000_000, 256)
            .saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn update_config() -> Weight {
        Weight::from_parts(17_000_000, 256)
            .saturating_add(RocksDbWeight::get().writes(1))
    }
}
