// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2024 Mizuki Nanzaki <info@mizuki.one> | X3 Public Chain

//! Weight info for pallet_x3_health

use frame_support::weights::{constants::RocksDbWeight, Weight};

/// Weights for pallet_x3_health using the Substrate node and recommended hardware
pub struct SubstrateWeight;

impl frame_support::weights::WeightInfo for SubstrateWeight {
    // Storage: X3Invariants InvariantChecks (r:1 w:0)
    // Storage: X3Health ComponentHealthStore (r:2 w:1)
    // Storage: X3Health HealthScores (r:1 w:0)
    // Storage: X3Health ActiveAlerts (r:1 w:1)
    fn report_health_check() -> Weight {
        Weight::from_parts(
            175_000_000, // execution time
            4096,        // proof size
        )
        .saturating_add(RocksDbWeight::get().reads(5))
        .saturating_add(RocksDbWeight::get().writes(2))
    }

    // Storage: X3Health ActiveAlerts (r:1 w:1)
    // Storage: X3Health HealthScores (r:1 w:0)
    fn resolve_alert() -> Weight {
        Weight::from_parts(
            125_000_000, // execution time
            1024,        // proof size
        )
        .saturating_add(RocksDbWeight::get().reads(2))
        .saturating_add(RocksDbWeight::get().writes(1))
    }

    // Storage: X3Health ComponentHealthStore (r:1 w:0)
    fn get_component_health() -> Weight {
        Weight::from_parts(
            75_000_000,  // execution time
            512,        // proof size
        )
        .saturating_add(RocksDbWeight::get().reads(1))
    }

    // Storage: X3Health SystemHealthScore (r:1 w:0)
    fn get_system_health_score() -> Weight {
        Weight::from_parts(
            50_000_000,  // execution time
            256,        // proof size
        )
        .saturating_add(RocksDbWeight::get().reads(1))
    }
}