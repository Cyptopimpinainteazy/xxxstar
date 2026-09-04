//! Consensus Parameters Module

use serde::{Deserialize, Serialize};

/// Consensus parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusParams {
    /// Slot duration in ms
    pub slot_duration_ms: u64,
    /// Finality depth in slots
    pub finality_depth: u64,
    /// PoH ticks per slot
    pub poh_ticks_per_slot: u64,
    /// Maximum rollback depth
    pub max_rollback_depth: u64,
    /// Leader rotation slots
    pub leader_rotation_slots: u64,
    /// Minimum stake for validation
    pub min_stake: u64,
    /// Maximum validators
    pub max_validators: u32,
    /// Vote threshold
    pub vote_threshold: f64,
}

impl Default for ConsensusParams {
    fn default() -> Self {
        Self {
            slot_duration_ms: 400, // 400ms slots
            finality_depth: 32,   // ~12.8s finality
            poh_ticks_per_slot: 6400,
            max_rollback_depth: 2,
            leader_rotation_slots: 4,
            min_stake: 1_000_000, // 1M lamports
            max_validators: 1000,
            vote_threshold: 0.67, // 2/3 + epsilon
        }
    }
}

impl ConsensusParams {
    /// Low latency configuration
    pub fn low_latency() -> Self {
        Self {
            slot_duration_ms: 200,
            finality_depth: 16,
            poh_ticks_per_slot: 3200,
            ..Default::default()
        }
    }

    /// Validate consensus parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.slot_duration_ms == 0 {
            return Err("slot_duration_ms must be > 0".into());
        }
        
        if self.finality_depth == 0 {
            return Err("finality_depth must be > 0".into());
        }
        
        if self.vote_threshold <= 0.5 || self.vote_threshold > 1.0 {
            return Err("vote_threshold must be > 0.5 and <= 1.0".into());
        }
        
        Ok(())
    }
}