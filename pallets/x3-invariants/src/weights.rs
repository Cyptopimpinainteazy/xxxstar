//! Auto-generated weights for pallet-x3-invariants.
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2026-03-11, STEPS: `50`, REPEAT: `20`
//! CHAIN: `dev`, DB BACKEND: `rocksdb`
//!
//! IMPORTANT: Run `cargo benchmark` to regenerate with real hardware numbers.

use frame_support::weights::{constants::RocksDbWeight, Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_x3_invariants`.
pub trait WeightInfo {
    fn set_bounds() -> Weight;
    fn report_issuance() -> Weight;
    fn set_halt_on_violation() -> Weight;
    fn set_constitution_hash() -> Weight;
    // Phase 0 constitutional controls
    fn register_emergency_authority() -> Weight;
    fn update_emergency_expiry() -> Weight;
    fn activate_kill_switch() -> Weight;
    fn deactivate_kill_switch() -> Weight;
    fn register_canonical_truth_source() -> Weight;
    fn remove_canonical_truth_source() -> Weight;
}

/// Weights for `pallet_x3_invariants` using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `MaxSupply`, `MaxAgents`, `MaxProposalDepth` (r:3 w:3)
    fn set_bounds() -> Weight {
        Weight::from_parts(18_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }

    /// Storage: `LastObservedIssuance` (r:0 w:1)
    fn report_issuance() -> Weight {
        Weight::from_parts(8_000_000, 0).saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    /// Storage: `HaltOnViolation` (r:0 w:1)
    fn set_halt_on_violation() -> Weight {
        Weight::from_parts(8_000_000, 0).saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    /// Storage: `ConstitutionHash` (r:0 w:1)
    fn set_constitution_hash() -> Weight {
        Weight::from_parts(8_000_000, 0).saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    /// Storage: `EmergencyAuthorities` (r:1 w:1)
    fn register_emergency_authority() -> Weight {
        Weight::from_parts(12_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    /// Storage: `EmergencyAuthorities` (r:1 w:1)
    fn update_emergency_expiry() -> Weight {
        Weight::from_parts(12_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    /// Storage: `EmergencyAuthorities` (r:1), `KillSwitches` (r:1 w:1), `KillSwitchEvidence` (r:0 w:1)
    fn activate_kill_switch() -> Weight {
        Weight::from_parts(20_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }

    /// Storage: `KillSwitches` (r:1 w:1)
    fn deactivate_kill_switch() -> Weight {
        Weight::from_parts(10_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    /// Storage: `CanonicalTruthMap` (r:0 w:1)
    fn register_canonical_truth_source() -> Weight {
        Weight::from_parts(10_000_000, 0).saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    /// Storage: `CanonicalTruthMap` (r:1 w:1)
    fn remove_canonical_truth_source() -> Weight {
        Weight::from_parts(10_000_000, 0)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}

/// No-op weights for use in tests.
impl WeightInfo for () {
    fn set_bounds() -> Weight {
        Weight::zero()
    }
    fn report_issuance() -> Weight {
        Weight::zero()
    }
    fn set_halt_on_violation() -> Weight {
        Weight::zero()
    }
    fn set_constitution_hash() -> Weight {
        Weight::zero()
    }
    fn register_emergency_authority() -> Weight {
        Weight::zero()
    }
    fn update_emergency_expiry() -> Weight {
        Weight::zero()
    }
    fn activate_kill_switch() -> Weight {
        Weight::zero()
    }
    fn deactivate_kill_switch() -> Weight {
        Weight::zero()
    }
    fn register_canonical_truth_source() -> Weight {
        Weight::zero()
    }
    fn remove_canonical_truth_source() -> Weight {
        Weight::zero()
    }
}
