//! Determinism tiering module for task execution in X3 swarm.
//!
//! Classifies tasks by their execution determinism guarantees:
//! - **FullyDeterministic**: Identical output on all validators (deterministic algorithms)
//! - **BlockStateDeterministic**: Output depends on block state (valid for blockchain)
//! - **ProbabilisticBounded**: Output varies but within bounded variance (ML models)
//! - **NonDeterministic**: No execution guarantees (external services, randomness)
//!
//! Used to validate proof outputs against execution commitments.

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use sp_io::hashing::blake2_256;
use sp_std::vec::Vec;

/// Determinism tier for task execution
#[derive(Clone, Copy, Encode, Decode, Debug, TypeInfo, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeterminismTier {
    /// Fully deterministic: Same input → Same output (cryptographic, sorting, etc)
    FullyDeterministic = 0,
    /// Block-state dependent: Output varies with blockchain state (valid)
    BlockStateDeterministic = 1,
    /// Probabilistic with bounds: Output varies within specified error margin (ML)
    ProbabilisticBounded = 2,
    /// Non-deterministic: No execution guarantees (external APIs, randomness)
    NonDeterministic = 3,
}

/// Task execution specification with determinism requirements
#[derive(Clone, Encode, Decode, Debug, TypeInfo, PartialEq, Eq)]
pub struct TaskDeterminismSpec<Hash> {
    /// Determinism tier for this task
    pub tier: DeterminismTier,
    /// Expected output hash (used for FullyDeterministic and BlockStateDeterministic)
    pub output_hash: Option<Hash>,
    /// For ProbabilisticBounded: maximum allowed variance in output (as percentage 0-100)
    pub variance_bound_percent: Option<u8>,
    /// Canonical form of expected output for verification
    pub canonical_output_bytes: Vec<u8>,
}

/// Verify that a task execution output matches the determinism specification
///
/// # Returns
/// - `true` if output is valid per the spec
/// - `false` if output violates determinism constraints
pub fn verify_deterministic_output<T: Encode>(
    spec: &TaskDeterminismSpec<[u8; 32]>,
    actual_output: &T,
) -> bool {
    match spec.tier {
        DeterminismTier::FullyDeterministic => {
            // Output must match exactly: hash(actual) == expected_hash
            if let Some(expected_hash) = spec.output_hash {
                blake2_256(&actual_output.encode()) == expected_hash
            } else {
                false // Must have expected hash
            }
        }
        DeterminismTier::BlockStateDeterministic => {
            // Output must match hash (blockchain state is implicit)
            if let Some(expected_hash) = spec.output_hash {
                blake2_256(&actual_output.encode()) == expected_hash
            } else {
                false
            }
        }
        DeterminismTier::ProbabilisticBounded => {
            // Output variance is acceptable within bounds
            // This is a simplified check: in production, compare numerical variance
            let actual_bytes = actual_output.encode();
            let actual_hash = blake2_256(&actual_bytes);

            if let Some(variance_percent) = spec.variance_bound_percent {
                // Simplified: if variance <= variance_percent, accept
                // Real implementation would compute statistical distance
                // For now: accept if hash is close (within Hamming distance threshold)
                let expected_bytes = spec.canonical_output_bytes.as_slice();
                let expected_hash = blake2_256(expected_bytes);

                // Compute Hamming distance between hashes
                let hamming_distance = actual_hash
                    .iter()
                    .zip(expected_hash.iter())
                    .filter(|(a, b)| a != b)
                    .count();

                // Accept if Hamming distance is within variance threshold
                // (32 bytes * 8 bits = 256 bits; variance_percent as fraction)
                let threshold = ((256 * variance_percent as usize) / 100).max(1);
                hamming_distance <= threshold
            } else {
                false // Must have variance bound
            }
        }
        DeterminismTier::NonDeterministic => {
            // Accept any output (no determinism constraints)
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fully_deterministic_exact_match() {
        let output = 42u64;
        let expected_hash = blake2_256(&output.encode());

        let spec = TaskDeterminismSpec {
            tier: DeterminismTier::FullyDeterministic,
            output_hash: Some(expected_hash),
            variance_bound_percent: None,
            canonical_output_bytes: output.encode(),
        };

        assert!(verify_deterministic_output(&spec, &output));
    }

    #[test]
    fn fully_deterministic_mismatch() {
        let output = 42u64;
        let wrong_hash = blake2_256(&43u64.encode());

        let spec = TaskDeterminismSpec {
            tier: DeterminismTier::FullyDeterministic,
            output_hash: Some(wrong_hash),
            variance_bound_percent: None,
            canonical_output_bytes: output.encode(),
        };

        assert!(!verify_deterministic_output(&spec, &output));
    }

    #[test]
    fn nondeterministic_always_accepts() {
        let spec = TaskDeterminismSpec {
            tier: DeterminismTier::NonDeterministic,
            output_hash: None,
            variance_bound_percent: None,
            canonical_output_bytes: vec![],
        };

        assert!(verify_deterministic_output(&spec, &42u64));
        assert!(verify_deterministic_output(&spec, &0u64));
    }

    #[test]
    fn determinism_tier_ordering() {
        // Tiers have well-defined ordering
        assert!(DeterminismTier::FullyDeterministic < DeterminismTier::BlockStateDeterministic);
        assert!(DeterminismTier::BlockStateDeterministic < DeterminismTier::ProbabilisticBounded);
        assert!(DeterminismTier::ProbabilisticBounded < DeterminismTier::NonDeterministic);
    }
}
