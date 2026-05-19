//! Deterministic hashing for X3 proofs.
//!
//! All hashing in X3 is deterministic and canonical. The same inputs
//! always produce the same hash on any machine. This is foundational
//! for replay-based verification.

use crate::types::*;
use sha2::{Digest, Sha256};

/// Deterministic hasher producing canonical 256-bit hashes.
pub struct DeterministicHasher {
    hasher: Sha256,
}

impl DeterministicHasher {
    /// Create a new hasher instance.
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
        }
    }

    /// Feed raw bytes into the hasher.
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Feed a u64 in canonical little-endian encoding.
    pub fn update_u64(&mut self, val: u64) {
        self.hasher.update(val.to_le_bytes());
    }

    /// Feed a u128 in canonical little-endian encoding.
    pub fn update_u128(&mut self, val: u128) {
        self.hasher.update(val.to_le_bytes());
    }

    /// Feed a length-prefixed byte slice.
    pub fn update_bytes(&mut self, data: &[u8]) {
        self.update_u64(data.len() as u64);
        self.hasher.update(data);
    }

    /// Feed an optional value — 0x00 for None, 0x01 + value for Some.
    pub fn update_option_bytes(&mut self, data: &Option<Vec<u8>>) {
        match data {
            None => self.hasher.update([0x00]),
            Some(v) => {
                self.hasher.update([0x01]);
                self.update_bytes(v);
            }
        }
    }

    /// Finalize and produce the 32-byte hash.
    pub fn finalize(self) -> Hash256 {
        let result = self.hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Hash a state diff canonically.
    pub fn hash_state_diff(diff: &StateDiff) -> Hash256 {
        let mut h = Self::new();
        h.update_bytes(&diff.key);
        h.update_option_bytes(&diff.old_value);
        h.update_option_bytes(&diff.new_value);
        h.finalize()
    }

    /// Hash an ordered list of state diffs.
    pub fn hash_state_diffs(diffs: &[StateDiff]) -> Hash256 {
        let mut h = Self::new();
        h.update_u64(diffs.len() as u64);
        for diff in diffs {
            let diff_hash = Self::hash_state_diff(diff);
            h.update(&diff_hash);
        }
        h.finalize()
    }

    /// Compute the canonical hash of an execution proof.
    /// The proof_hash field is NOT included in the hash — it IS the hash.
    pub fn hash_execution_proof(proof: &ExecutionProof) -> Hash256 {
        let mut h = Self::new();
        h.update_u64(proof.id);
        h.update_u64(proof.block_height);
        h.update(&proof.program_hash);
        h.update(&proof.pre_state_hash);
        h.update(&proof.post_state_hash);

        // Hash state diffs
        let diffs_hash = Self::hash_state_diffs(&proof.state_diffs);
        h.update(&diffs_hash);

        h.update_u64(proof.gas_consumed);
        h.update_u64(proof.fee_charged);
        h.update(&proof.agent_id.pubkey);
        h.update(&[proof.agent_id.ephemeral as u8]);

        // Optional intent ID
        match &proof.intent_id {
            None => h.update(&[0x00]),
            Some(id) => {
                h.update(&[0x01]);
                h.update_u128(id.0);
            }
        }

        h.finalize()
    }

    /// Hash a program (bytecode) for canonical identification.
    pub fn hash_program(bytecode: &[u8]) -> Hash256 {
        let mut h = Self::new();
        h.update_bytes(bytecode);
        h.finalize()
    }

    /// Hash a state root from a set of key-value pairs.
    /// Keys must be sorted for determinism.
    pub fn hash_state(kvs: &[(Vec<u8>, Vec<u8>)]) -> Hash256 {
        let mut h = Self::new();
        h.update_u64(kvs.len() as u64);
        for (k, v) in kvs {
            h.update_bytes(k);
            h.update_bytes(v);
        }
        h.finalize()
    }
}

impl Default for DeterministicHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_hashing() {
        let data = b"hello x3";
        let h1 = {
            let mut h = DeterministicHasher::new();
            h.update(data);
            h.finalize()
        };
        let h2 = {
            let mut h = DeterministicHasher::new();
            h.update(data);
            h.finalize()
        };
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_state_diff_hashing() {
        let diff = StateDiff {
            key: vec![1, 2, 3],
            old_value: Some(vec![10]),
            new_value: Some(vec![20]),
        };
        let h1 = DeterministicHasher::hash_state_diff(&diff);
        let h2 = DeterministicHasher::hash_state_diff(&diff);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_inputs_different_hashes() {
        let d1 = StateDiff {
            key: vec![1],
            old_value: None,
            new_value: Some(vec![1]),
        };
        let d2 = StateDiff {
            key: vec![2],
            old_value: None,
            new_value: Some(vec![1]),
        };
        assert_ne!(
            DeterministicHasher::hash_state_diff(&d1),
            DeterministicHasher::hash_state_diff(&d2)
        );
    }

    #[test]
    fn test_program_hash() {
        let program = vec![0x00, 0x01, 0x02, 0x03];
        let hash = DeterministicHasher::hash_program(&program);
        assert_ne!(hash, [0u8; 32]);
    }
}
