//! Determinism tiering module for task execution in X3 swarm.
//!
//! Classifies tasks by their execution determinism guarantees:
//! - **FullyDeterministic**: Identical output on all validators (deterministic algorithms)
//! - **BlockStateDeterministic**: Output depends on block state (valid for blockchain)
//! - **ProbabilisticBounded**: Output varies but within bounded variance (ML
//!   models). [`verify_deterministic_output`] **refuses** this tier: a
//!   byte-level comparison cannot establish a numeric variance bound, and the
//!   check that used to stand in for one accepted arbitrary outputs (see the
//!   comment on that arm)
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
            // Refused, not guessed.
            //
            // This used to accept an output when the Hamming distance between
            // `blake2_256(actual)` and `blake2_256(canonical)` was within
            // `variance_bound_percent` of 256 bits. A hash is an avalanche
            // function: two nearly identical numbers differ in about half of
            // their bits, and two unrelated outputs land anywhere. The check
            // therefore measured nothing about the output — at
            // `variance_bound_percent = 50` it accepted roughly half of all
            // byte strings — while reading like a numeric tolerance test.
            //
            // A bounded *numeric* variance needs the numeric contract (which
            // field of the output, in what units, with what tolerance). This
            // pallet has no such contract: it is generic over `T: Encode`. Use
            // `DeterminismTier::FullyDeterministic` with `output_hash` when a
            // byte-exact comparison is what is meant; until a typed comparator
            // exists, this tier refuses every output rather than accepting the
            // ones that happen to hash "close".
            false
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

    /// The probabilistic tier refuses, including when the output is exactly the
    /// expected one.
    ///
    /// The check it replaces compared the Hamming distance between
    /// `blake2_256(actual)` and `blake2_256(canonical)` with a threshold, which
    /// has nothing to do with numeric variance: at
    /// `variance_bound_percent = 50` the threshold was half of 256 bits, and two
    /// unrelated outputs land within it about half the time.
    #[test]
    fn probabilistic_tier_refuses_rather_than_measuring_hashes() {
        let expected = 1_000_000i64;
        let spec = TaskDeterminismSpec {
            tier: DeterminismTier::ProbabilisticBounded,
            output_hash: None,
            variance_bound_percent: Some(50),
            canonical_output_bytes: expected.encode(),
        };

        // The exact expected value, a value 1% away, and a wildly different one
        // are all refused: there is no comparator that could tell them apart.
        assert!(!verify_deterministic_output(&spec, &expected));
        assert!(!verify_deterministic_output(&spec, &(expected + 10_000)));
        assert!(!verify_deterministic_output(&spec, &i64::MIN));
    }

    /// A tier with no variance bound was already refused; keep that pinned.
    #[test]
    fn probabilistic_tier_without_a_bound_is_refused() {
        let spec = TaskDeterminismSpec {
            tier: DeterminismTier::ProbabilisticBounded,
            output_hash: None,
            variance_bound_percent: None,
            canonical_output_bytes: 42u64.encode(),
        };

        assert!(!verify_deterministic_output(&spec, &42u64));
    }

    /// The two byte-exact tiers keep working.
    #[test]
    fn byte_exact_tiers_still_decide_on_the_hash() {
        let output = 7u64;

        for tier in [
            DeterminismTier::FullyDeterministic,
            DeterminismTier::BlockStateDeterministic,
        ] {
            let matching = TaskDeterminismSpec {
                tier,
                output_hash: Some(blake2_256(&output.encode())),
                variance_bound_percent: None,
                canonical_output_bytes: output.encode(),
            };
            assert!(verify_deterministic_output(&matching, &output));

            let mut wrong = matching.clone();
            wrong.output_hash = Some(blake2_256(&8u64.encode()));
            assert!(!verify_deterministic_output(&wrong, &output));

            let mut missing = matching.clone();
            missing.output_hash = None;
            assert!(!verify_deterministic_output(&missing, &output));
        }
    }

    #[test]
    fn determinism_tier_ordering() {
        // Tiers have well-defined ordering
        assert!(DeterminismTier::FullyDeterministic < DeterminismTier::BlockStateDeterministic);
        assert!(DeterminismTier::BlockStateDeterministic < DeterminismTier::ProbabilisticBounded);
        assert!(DeterminismTier::ProbabilisticBounded < DeterminismTier::NonDeterministic);
    }
}
