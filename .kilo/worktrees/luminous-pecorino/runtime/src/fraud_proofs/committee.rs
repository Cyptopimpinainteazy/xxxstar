// runtime/src/fraud_proofs/committee.rs
//
// Deterministic VRF-style committee selection for fraud-proof re-execution.
//
// ## Design
// The validator set that must re-execute a disputed block is chosen by a
// deterministic, verifiable process so that:
//   1. Both the proposer and every observer can independently compute the
//      same committee from the same public inputs.
//   2. The proposer cannot predict the committee in advance (the seed is
//      derived from the disputed block hash posted on-chain).
//   3. The algorithm is `no_std`-compatible and uses only BTreeMap / sorted
//      slices — no HashMap, no floats, no randomness crate.
//
// ## Algorithm: keyed Fisher-Yates using blake2_256 expansion
// We expand the `seed` into an infinite stream of 32-byte blocks:
//     H_i = blake2_256( seed ++ i_as_le_bytes )
// Each block provides 4 bytes used as a little-endian u32 swap index.
// This is conceptually equivalent to a cryptographically-secure PRG keyed
// by the disputed-block hash and iterated to produce a permutation prefix
// of length `k`.
//
// INVARIANT: COMMITTEE-SELECT-001 — same (eligible, seed, k) always yields
// the same output (see tests below and tests/invariants/registry.toml).

#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_std::vec::Vec;

/// Default committee size (must re-execute to confirm a dispute).
pub const DEFAULT_COMMITTEE_SIZE: usize = 15;

/// Maximum committee size the runtime will honour.
pub const MAX_COMMITTEE_SIZE: usize = 64;

/// Select `k` validators from `eligible` using a deterministic permutation
/// seeded by `seed`.
///
/// # Panics
/// Never panics.
///
/// # Returns
/// A `Vec<AccountId>` of length `min(k, eligible.len())`, in selection order.
///
/// # Note
/// `eligible` MUST be pre-sorted by the caller to ensure every node derives
/// the same set.  The function itself does not sort to keep the interface
/// flexible (sort key is caller-defined).
pub fn select_committee<AccountId: Clone>(
    eligible: &[AccountId],
    seed: H256,
    k: usize,
) -> Vec<AccountId> {
    let n = eligible.len();
    let take = k.min(n).min(MAX_COMMITTEE_SIZE);

    if take == 0 {
        return Vec::new();
    }

    // Build a mutable index array [0, 1, 2, ..., n-1].
    let mut indices: Vec<usize> = (0..n).collect();

    // Deterministic Fisher-Yates prefix shuffle.
    // Each swap uses 4 bytes from blake2_256(seed ++ round_counter).
    let mut counter: u32 = 0;
    for i in 0..take {
        // How many valid swap targets remain in [i, n)?
        let remaining = n - i;
        let swap_idx = next_u32(&seed, &mut counter) as usize % remaining + i;
        indices.swap(i, swap_idx);
    }

    indices[..take]
        .iter()
        .map(|&idx| eligible[idx].clone())
        .collect()
}

/// Derive the next pseudo-random u32 from a running counter keyed on the seed.
///
/// We batch-produce pairs of 4-byte words from a single 32-byte hash output
/// to halve the number of hash invocations.  Counter is incremented by the
/// caller.
fn next_u32(seed: &H256, counter: &mut u32) -> u32 {
    // Each hash call gives us 8 u32s; use the word at position within the block.
    let word_within_block = *counter % 8;
    let block_index = *counter / 8;
    *counter += 1;

    // hash( seed || block_index_le )
    let mut preimage = [0u8; 36];
    preimage[..32].copy_from_slice(seed.as_bytes());
    preimage[32..].copy_from_slice(&block_index.to_le_bytes());
    let hash = blake2_256(&preimage);

    let offset = (word_within_block as usize) * 4;
    u32::from_le_bytes([
        hash[offset],
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ])
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors returned by committee-selection routines.
#[derive(Debug, PartialEq, Eq)]
pub enum CommitteeError {
    /// Requested committee larger than eligible set (soft — we clamp instead).
    #[allow(dead_code)]
    KExceedsEligible { k: usize, eligible: usize },
    /// Eligible set is empty — no committee can be formed.
    NoEligibleValidators,
}

/// Like `select_committee` but returns `Err` when the eligible set is empty.
pub fn try_select_committee<AccountId: Clone>(
    eligible: &[AccountId],
    seed: H256,
    k: usize,
) -> Result<Vec<AccountId>, CommitteeError> {
    if eligible.is_empty() {
        return Err(CommitteeError::NoEligibleValidators);
    }
    Ok(select_committee(eligible, seed, k))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_seed(b: u8) -> H256 {
        H256::from([b; 32])
    }

    /// COMMITTEE-SELECT-001: identical inputs → identical output
    #[test]
    fn deterministic_same_seed() {
        let validators: Vec<u32> = (0..50).collect();
        let seed = make_seed(0xAB);
        let a = select_committee(&validators, seed, DEFAULT_COMMITTEE_SIZE);
        let b = select_committee(&validators, seed, DEFAULT_COMMITTEE_SIZE);
        assert_eq!(a, b);
    }

    /// COMMITTEE-SELECT-002: different seeds → different committees (with overwhelming probability)
    #[test]
    fn different_seeds_different_committees() {
        let validators: Vec<u32> = (0..50).collect();
        let a = select_committee(&validators, make_seed(0x01), DEFAULT_COMMITTEE_SIZE);
        let b = select_committee(&validators, make_seed(0x02), DEFAULT_COMMITTEE_SIZE);
        assert_ne!(a, b);
    }

    /// COMMITTEE-SELECT-003: k ≥ n returns all validators exactly once
    #[test]
    fn k_larger_than_n_returns_all() {
        let validators: Vec<u32> = (0..5).collect();
        let result = select_committee(&validators, make_seed(0x07), 100);
        assert_eq!(result.len(), 5);
        // All indices present
        let mut sorted = result.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, validators);
    }

    /// COMMITTEE-SELECT-004: empty eligible → empty result
    #[test]
    fn empty_eligible_returns_empty() {
        let empty: Vec<u32> = Vec::new();
        let result = select_committee(&empty, make_seed(0xFF), 10);
        assert!(result.is_empty());
    }

    /// COMMITTEE-SELECT-005: no duplicates in output
    #[test]
    fn no_duplicates_in_selection() {
        let validators: Vec<u32> = (0..100).collect();
        let result = select_committee(&validators, make_seed(0x42), DEFAULT_COMMITTEE_SIZE);
        let mut dedup = result.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(dedup.len(), result.len());
    }

    /// COMMITTEE-SELECT-006: try_select returns Err on empty set
    #[test]
    fn try_select_returns_error_on_empty() {
        let empty: Vec<u32> = Vec::new();
        assert_eq!(
            try_select_committee(&empty, make_seed(0x01), 5),
            Err(CommitteeError::NoEligibleValidators)
        );
    }

    /// COMMITTEE-SELECT-007: output length equals min(k, n)
    #[test]
    fn output_length_is_min_k_n() {
        let validators: Vec<u32> = (0..30).collect();
        for k in [1, 5, 15, 30, 50] {
            let result = select_committee(&validators, make_seed(0x10), k);
            assert_eq!(result.len(), k.min(30));
        }
    }
}
