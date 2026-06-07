//! Proof verification engine.
//!
//! Verifies proof chains by deterministic replay. This is the foundation
//! of the X3 court system — all disputes are resolved by replaying the
//! original execution and comparing the results.

use crate::chain::ProofChain;
use crate::error::ProofError;
use crate::hasher::DeterministicHasher;
use crate::types::*;

/// Independent proof verifier — can verify any proof chain
/// without access to the original execution context.
pub struct ProofVerifier;

impl ProofVerifier {
    /// Verify a single execution proof's internal consistency.
    pub fn verify_proof(proof: &ExecutionProof) -> Result<(), ProofError> {
        let computed_hash = DeterministicHasher::hash_execution_proof(proof);
        if computed_hash != proof.proof_hash {
            return Err(ProofError::InvalidProofHash {
                index: proof.id as usize,
            });
        }

        // Verify state diffs are internally consistent
        // (no duplicate keys within a single proof)
        let mut seen_keys = std::collections::HashSet::new();
        for diff in &proof.state_diffs {
            if !seen_keys.insert(&diff.key) {
                return Err(ProofError::DuplicateKeyInDiffs);
            }
        }

        Ok(())
    }

    /// Verify a complete proof chain.
    pub fn verify_chain(chain: &ProofChain) -> Result<VerificationResult, ProofError> {
        // First verify structural integrity
        chain.verify_integrity()?;

        // Verify each proof individually
        for proof in chain.proofs() {
            Self::verify_proof(proof)?;
        }

        Ok(VerificationResult {
            valid: true,
            chain_hash: chain.chain_hash(),
            proof_count: chain.len(),
            total_gas: chain.total_gas(),
            total_fees: chain.total_fees(),
            final_state_root: chain.final_state_root(),
        })
    }

    /// Compare two proof chains to detect divergence.
    /// Used by the court system for dispute resolution.
    pub fn compare_chains(original: &ProofChain, replay: &ProofChain) -> ComparisonResult {
        let min_len = std::cmp::min(original.len(), replay.len());

        for i in 0..min_len {
            let orig = &original.proofs()[i];
            let rep = &replay.proofs()[i];

            // Compare state diffs
            if orig.state_diffs != rep.state_diffs {
                return ComparisonResult::Diverged {
                    at_proof: i,
                    reason: DivergenceReason::StateDiffMismatch,
                    original_hash: orig.proof_hash,
                    replay_hash: rep.proof_hash,
                };
            }

            // Compare post-state
            if orig.post_state_hash != rep.post_state_hash {
                return ComparisonResult::Diverged {
                    at_proof: i,
                    reason: DivergenceReason::PostStateMismatch,
                    original_hash: orig.proof_hash,
                    replay_hash: rep.proof_hash,
                };
            }

            // Compare gas (execution should be deterministic)
            if orig.gas_consumed != rep.gas_consumed {
                return ComparisonResult::Diverged {
                    at_proof: i,
                    reason: DivergenceReason::GasMismatch,
                    original_hash: orig.proof_hash,
                    replay_hash: rep.proof_hash,
                };
            }
        }

        if original.len() != replay.len() {
            return ComparisonResult::Diverged {
                at_proof: min_len,
                reason: DivergenceReason::LengthMismatch,
                original_hash: original.chain_hash(),
                replay_hash: replay.chain_hash(),
            };
        }

        ComparisonResult::Matched {
            chain_hash: original.chain_hash(),
        }
    }

    /// Verify a replay proof against the original.
    pub fn verify_replay(
        original: &ExecutionProof,
        replay: &ReplayProof,
    ) -> Result<bool, ProofError> {
        // Verify the replay references the correct original
        if replay.original_proof_hash != original.proof_hash {
            return Err(ProofError::ReplayTargetMismatch);
        }

        Ok(replay.matches)
    }
}

/// Result of proof chain verification.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub chain_hash: Hash256,
    pub proof_count: usize,
    pub total_gas: u64,
    pub total_fees: u64,
    pub final_state_root: Hash256,
}

/// Result of comparing two proof chains.
#[derive(Debug, Clone)]
pub enum ComparisonResult {
    /// Chains matched exactly.
    Matched { chain_hash: Hash256 },
    /// Chains diverged at a specific point.
    Diverged {
        at_proof: usize,
        reason: DivergenceReason,
        original_hash: Hash256,
        replay_hash: Hash256,
    },
}

/// Reason for chain divergence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivergenceReason {
    /// State diffs don't match.
    StateDiffMismatch,
    /// Post-state hashes don't match.
    PostStateMismatch,
    /// Gas consumption doesn't match (non-deterministic execution).
    GasMismatch,
    /// Different number of proofs in chain.
    LengthMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof(id: u64, pre: Hash256, post: Hash256) -> ExecutionProof {
        let agent = AgentIdentity {
            pubkey: [1u8; 32],
            ephemeral: false,
        };
        let mut proof = ExecutionProof {
            id,
            block_height: 100,
            program_hash: [0xAA; 32],
            pre_state_hash: pre,
            post_state_hash: post,
            state_diffs: vec![],
            gas_consumed: 100,
            fee_charged: 10,
            agent_id: agent,
            intent_id: None,
            proof_hash: [0u8; 32],
        };
        proof.proof_hash = DeterministicHasher::hash_execution_proof(&proof);
        proof
    }

    #[test]
    fn test_verify_single_proof() {
        let proof = make_proof(0, [0u8; 32], [1u8; 32]);
        ProofVerifier::verify_proof(&proof).unwrap();
    }

    #[test]
    fn test_tampered_proof_fails() {
        let mut proof = make_proof(0, [0u8; 32], [1u8; 32]);
        proof.gas_consumed = 999; // Tamper
        let result = ProofVerifier::verify_proof(&proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_chain_comparison_match() {
        let mut c1 = ProofChain::new(100, [0xAA; 32]);
        let mut c2 = ProofChain::new(100, [0xAA; 32]);

        let p = make_proof(0, [0u8; 32], [1u8; 32]);
        c1.append(p.clone()).unwrap();
        c2.append(p).unwrap();

        match ProofVerifier::compare_chains(&c1, &c2) {
            ComparisonResult::Matched { .. } => {}
            _ => panic!("chains should match"),
        }
    }
}
