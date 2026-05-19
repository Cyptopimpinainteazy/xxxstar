//! Proof chain — an ordered, hash-linked sequence of execution proofs.

use crate::error::ProofError;
use crate::hasher::DeterministicHasher;
use crate::types::*;

/// An ordered chain of execution proofs forming a complete execution trace.
///
/// Each proof's hash is linked to the previous, forming a Merkle-like chain
/// that can be verified from any starting point.
#[derive(Debug, Clone)]
pub struct ProofChain {
    /// Block height of the entire chain.
    block_height: BlockHeight,
    /// Program hash this chain belongs to.
    program_hash: Hash256,
    /// Ordered proofs.
    proofs: Vec<ExecutionProof>,
    /// Running chain hash (hash of all proof hashes in order).
    running_hash: Hash256,
}

impl ProofChain {
    /// Create a new empty proof chain.
    pub fn new(block_height: BlockHeight, program_hash: Hash256) -> Self {
        // Initial chain hash is hash of block_height + program_hash
        let mut h = DeterministicHasher::new();
        h.update_u64(block_height);
        h.update(&program_hash);
        let initial_hash = h.finalize();

        Self {
            block_height,
            program_hash,
            proofs: Vec::new(),
            running_hash: initial_hash,
        }
    }

    /// Append a proof to the chain.
    pub fn append(&mut self, proof: ExecutionProof) -> Result<(), ProofError> {
        // Verify proof belongs to this chain
        if proof.block_height != self.block_height {
            return Err(ProofError::BlockHeightMismatch {
                expected: self.block_height,
                got: proof.block_height,
            });
        }
        if proof.program_hash != self.program_hash {
            return Err(ProofError::ProgramHashMismatch);
        }

        // Update running hash: H(prev_hash || proof_hash)
        let mut h = DeterministicHasher::new();
        h.update(&self.running_hash);
        h.update(&proof.proof_hash);
        self.running_hash = h.finalize();

        self.proofs.push(proof);
        Ok(())
    }

    /// Get the current chain hash.
    pub fn chain_hash(&self) -> Hash256 {
        self.running_hash
    }

    /// Get total gas consumed across all proofs.
    pub fn total_gas(&self) -> u64 {
        self.proofs.iter().map(|p| p.gas_consumed).sum()
    }

    /// Get total fees charged across all proofs.
    pub fn total_fees(&self) -> u64 {
        self.proofs.iter().map(|p| p.fee_charged).sum()
    }

    /// Get the number of proofs in the chain.
    pub fn len(&self) -> usize {
        self.proofs.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.proofs.is_empty()
    }

    /// Get all proofs in order.
    pub fn proofs(&self) -> &[ExecutionProof] {
        &self.proofs
    }

    /// Get a proof by index.
    pub fn get(&self, index: usize) -> Option<&ExecutionProof> {
        self.proofs.get(index)
    }

    /// Get the last proof in the chain.
    pub fn last(&self) -> Option<&ExecutionProof> {
        self.proofs.last()
    }

    /// Get the final state root (post_state_hash of last proof, or initial state).
    pub fn final_state_root(&self) -> Hash256 {
        self.proofs
            .last()
            .map(|p| p.post_state_hash)
            .unwrap_or([0u8; 32])
    }

    /// Verify the internal consistency of the chain.
    /// Checks: proof hashes are correct, chain links are valid,
    /// pre/post state continuity.
    pub fn verify_integrity(&self) -> Result<(), ProofError> {
        let mut expected_running = {
            let mut h = DeterministicHasher::new();
            h.update_u64(self.block_height);
            h.update(&self.program_hash);
            h.finalize()
        };

        for (i, proof) in self.proofs.iter().enumerate() {
            // Verify proof hash
            let computed = DeterministicHasher::hash_execution_proof(proof);
            if computed != proof.proof_hash {
                return Err(ProofError::InvalidProofHash { index: i });
            }

            // Verify chain continuity
            let mut h = DeterministicHasher::new();
            h.update(&expected_running);
            h.update(&proof.proof_hash);
            expected_running = h.finalize();

            // Verify state continuity (post[i-1] == pre[i])
            if i > 0 {
                let prev = &self.proofs[i - 1];
                if prev.post_state_hash != proof.pre_state_hash {
                    return Err(ProofError::StateDiscontinuity { proof_index: i });
                }
            }
        }

        if expected_running != self.running_hash {
            return Err(ProofError::ChainHashMismatch);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_chain() {
        let chain = ProofChain::new(100, [0xAA; 32]);
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
        assert_eq!(chain.total_gas(), 0);
    }

    #[test]
    fn test_chain_integrity() {
        let mut chain = ProofChain::new(100, [0xAA; 32]);

        let agent = AgentIdentity {
            pubkey: [1u8; 32],
            ephemeral: false,
        };

        let mut proof1 = ExecutionProof {
            id: 0,
            block_height: 100,
            program_hash: [0xAA; 32],
            pre_state_hash: [0u8; 32],
            post_state_hash: [1u8; 32],
            state_diffs: vec![],
            gas_consumed: 100,
            fee_charged: 10,
            agent_id: agent.clone(),
            intent_id: None,
            proof_hash: [0u8; 32],
        };
        proof1.proof_hash = DeterministicHasher::hash_execution_proof(&proof1);

        let mut proof2 = ExecutionProof {
            id: 1,
            block_height: 100,
            program_hash: [0xAA; 32],
            pre_state_hash: [1u8; 32],
            post_state_hash: [2u8; 32],
            state_diffs: vec![],
            gas_consumed: 200,
            fee_charged: 20,
            agent_id: agent,
            intent_id: None,
            proof_hash: [0u8; 32],
        };
        proof2.proof_hash = DeterministicHasher::hash_execution_proof(&proof2);

        chain.append(proof1).unwrap();
        chain.append(proof2).unwrap();

        assert_eq!(chain.len(), 2);
        assert_eq!(chain.total_gas(), 300);
        chain.verify_integrity().unwrap();
    }
}
