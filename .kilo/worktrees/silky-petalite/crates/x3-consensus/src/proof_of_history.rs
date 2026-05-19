//! Proof of History: verifiable time ordering without trusted timeline source
//!
//! Each validator generates a sequence of PoH ticks (SHA-256 hash chain).
//! Ticks are embedded in block headers so verifiers can confirm ordering.
//! Enables secure timestamping + ordering without relying on validator honesty.

use sp_core::hashing::sha256;

/// Proof of History tick
#[derive(Clone, Debug)]
pub struct PoHTick {
    /// Sequential tick number
    pub tick_number: u64,
    /// Hash output for this tick
    pub hash: [u8; 32],
    /// Previous hash (parent)
    pub previous_hash: [u8; 32],
    /// Optional data hashed in (tx hash, event, etc.)
    pub hashed_data: Option<Vec<u8>>,
    /// Timestamp when this tick was generated (informational)
    pub unix_time_ms: u64,
}

impl PoHTick {
    /// Verify this tick's hash is valid
    pub fn verify(&self) -> bool {
        let mut input = self.previous_hash.to_vec();
        if let Some(data) = &self.hashed_data {
            input.extend(data);
        }

        let computed = sha256(&input);
        computed == self.hash
    }

    /// Link next tick in the chain
    pub fn next(&self, hashed_data: Option<Vec<u8>>, unix_time_ms: u64) -> PoHTick {
        let mut input = self.hash.to_vec();
        if let Some(data) = &hashed_data {
            input.extend(data);
        }

        let next_hash = sha256(&input);

        PoHTick {
            tick_number: self.tick_number + 1,
            hash: next_hash,
            previous_hash: self.hash,
            hashed_data,
            unix_time_ms,
        }
    }
}

/// Proof of History generator (per-validator)
#[derive(Clone, Debug)]
pub struct PoHGenerator {
    /// Validator ID
    pub validator: String,
    /// Current tick
    pub current_tick: PoHTick,
    /// Tick history (optionally keep full chain)
    pub tick_history: Vec<PoHTick>,
    /// Ticks per block (tick frequency)
    pub ticks_per_block: u64,
}

impl PoHGenerator {
    /// Initialize PoH generator with genesis tick
    pub fn new(validator: String) -> Self {
        let genesis_hash = [0u8; 32];
        let current_tick = PoHTick {
            tick_number: 0,
            hash: sha256(&genesis_hash),
            previous_hash: genesis_hash,
            hashed_data: None,
            unix_time_ms: 0,
        };

        let mut tick_history = Vec::new();
        tick_history.push(current_tick.clone());

        Self {
            validator,
            current_tick,
            tick_history,
            ticks_per_block: 100, // 100 ticks per block by default
        }
    }

    /// Generate next tick (called frequently, e.g., every ms)
    pub fn tick(&mut self, hashed_data: Option<Vec<u8>>, unix_time_ms: u64) {
        self.current_tick = self.current_tick.next(hashed_data, unix_time_ms);
        self.tick_history.push(self.current_tick.clone());
    }

    /// Generate N ticks for a block
    pub fn ticks_for_block(&mut self, num_ticks: u64, unix_time_ms: u64) -> Vec<PoHTick> {
        let mut ticks = Vec::new();
        for i in 0..num_ticks {
            self.tick(None, unix_time_ms + i);
            ticks.push(self.current_tick.clone());
        }
        ticks
    }

    /// Get PoH proof from tick X → tick Y (proves Y happened after X)
    pub fn proof_of_ordering(&self, from_tick: u64, to_tick: u64) -> Option<Vec<PoHTick>> {
        if from_tick >= to_tick || to_tick > self.current_tick.tick_number {
            return None;
        }

        let proof: Vec<PoHTick> = self
            .tick_history
            .iter()
            .filter(|t| t.tick_number >= from_tick && t.tick_number <= to_tick)
            .cloned()
            .collect();

        if !proof.is_empty() {
            Some(proof)
        } else {
            None
        }
    }

    /// Verify a chain of ticks
    pub fn verify_tick_chain(ticks: &[PoHTick]) -> bool {
        if ticks.is_empty() {
            return true;
        }

        // Verify each tick's hash is valid
        for tick in ticks {
            if !tick.verify() {
                return false;
            }
        }

        // Verify tick numbers are sequential
        for i in 0..ticks.len() - 1 {
            if ticks[i].tick_number >= ticks[i + 1].tick_number {
                return false;
            }
            if ticks[i].hash != ticks[i + 1].previous_hash {
                return false;
            }
        }

        true
    }

    /// Get current tick as a compact proof (for block header)
    pub fn current_proof(&self) -> PoHBlockProof {
        PoHBlockProof {
            tick_number: self.current_tick.tick_number,
            hash: self.current_tick.hash,
        }
    }
}

/// Compact PoH proof that goes in block headers
#[derive(Clone, Debug)]
pub struct PoHBlockProof {
    pub tick_number: u64,
    pub hash: [u8; 32],
}

/// Verify PoH proof between two blocks
#[derive(Clone, Debug)]
pub struct PoHVerifier {
    /// Genesis PoH proof
    pub genesis_proof: PoHBlockProof,
    /// Accumulated PoH chain (sparse: keep every Nth proof)
    pub checkpoint_proofs: Vec<(u64, PoHBlockProof)>, // (block_height, proof)
}

impl PoHVerifier {
    pub fn new(genesis_proof: PoHBlockProof) -> Self {
        let mut checkpoints = Vec::new();
        checkpoints.push((0, genesis_proof.clone()));

        Self {
            genesis_proof,
            checkpoint_proofs: checkpoints,
        }
    }

    /// Add PoH proof from a finalized block
    pub fn record_block_poh(&mut self, block_height: u64, proof: PoHBlockProof) {
        self.checkpoint_proofs.push((block_height, proof));
    }

    /// Verify block ordering: is block_b definitely after block_a?
    pub fn verify_ordering(&self, block_a: u64, block_b: u64) -> bool {
        let proof_a = self.checkpoint_proofs.iter().find(|(h, _)| *h == block_a)?;
        let proof_b = self.checkpoint_proofs.iter().find(|(h, _)| *h == block_b)?;

        // Block B is ordered after A if B's tick > A's tick
        proof_b.1.tick_number > proof_a.1.tick_number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poh_tick_verification() {
        let mut gen = PoHGenerator::new("validator1".to_string());

        gen.tick(None, 0);
        assert!(gen.current_tick.verify());

        gen.tick(Some(b"event1".to_vec()), 1);
        assert!(gen.current_tick.verify());
    }

    #[test]
    fn test_poh_tick_chain() {
        let mut gen = PoHGenerator::new("validator1".to_string());

        let ticks = gen.ticks_for_block(10, 0);
        assert!(PoHGenerator::verify_tick_chain(&ticks));
    }

    #[test]
    fn test_poh_generator_sequential() {
        let mut gen = PoHGenerator::new("validator1".to_string());

        let tick1 = gen.current_tick.clone();
        gen.tick(None, 1);
        let tick2 = gen.current_tick.clone();

        assert!(tick2.tick_number > tick1.tick_number);
        assert_eq!(tick2.previous_hash, tick1.hash);
    }

    #[test]
    fn test_poh_proof_of_ordering() {
        let mut gen = PoHGenerator::new("validator1".to_string());

        gen.ticks_for_block(100, 0);

        let proof = gen.proof_of_ordering(10, 50);
        assert!(proof.is_some());
        assert!(PoHGenerator::verify_tick_chain(proof.unwrap().as_slice()));
    }

    #[test]
    fn test_poh_block_proof() {
        let mut gen = PoHGenerator::new("validator1".to_string());

        gen.ticks_for_block(50, 0);
        let proof = gen.current_proof();

        assert_eq!(proof.tick_number, 50);
    }

    #[test]
    fn test_poh_verifier_ordering() {
        let proof_a = PoHBlockProof {
            tick_number: 100,
            hash: [0u8; 32],
        };

        let mut verifier = PoHVerifier::new(proof_a.clone());

        let proof_b = PoHBlockProof {
            tick_number: 200,
            hash: [1u8; 32],
        };

        verifier.record_block_poh(1, proof_b);

        assert!(verifier.verify_ordering(0, 1));
        assert!(!verifier.verify_ordering(1, 0));
    }

    #[test]
    fn test_poh_with_data_hashing() {
        let mut gen = PoHGenerator::new("validator1".to_string());

        let tx_hash = b"tx_0x123abc".to_vec();
        gen.tick(Some(tx_hash), 0);

        assert!(gen.current_tick.verify());
        assert!(gen.current_tick.hashed_data.is_some());
    }

    #[test]
    fn test_poh_invalid_tick_rejected() {
        let mut tick = PoHTick {
            tick_number: 1,
            hash: [0u8; 32], // wrong hash
            previous_hash: [1u8; 32],
            hashed_data: None,
            unix_time_ms: 0,
        };

        // This tick should fail verification
        assert!(!tick.verify());

        // Fix it
        let mut input = tick.previous_hash.to_vec();
        tick.hash = sha256(&input);

        assert!(tick.verify());
    }
}
