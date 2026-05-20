//! Replay engine — deterministic re-execution for dispute resolution.

use x3_proof::chain::ProofChain;
use x3_proof::hasher::DeterministicHasher;
use x3_proof::types::{BlockHeight, Hash256};

/// The replay engine. Re-executes programs deterministically
/// to verify execution proofs.
pub struct ReplayEngine;

impl ReplayEngine {
    /// Replay an execution and produce a new proof chain.
    ///
    /// In a full implementation, this would:
    /// 1. Load the program bytecode from the program hash
    /// 2. Reconstruct the initial state from the pre_state_hash
    /// 3. Re-execute the program in the X3 VM
    /// 4. Produce a new proof chain
    /// 5. Compare with the original
    ///
    /// For now, this performs structural replay validation.
    pub fn replay_from_proofs(
        original: &ProofChain,
        block_height: BlockHeight,
        program_hash: Hash256,
    ) -> ProofChain {
        let mut replay_chain = ProofChain::new(block_height, program_hash);

        // In production, each proof would be re-derived by re-executing
        // the corresponding program segment. For now, we faithfully
        // reconstruct from the original proofs (which makes the comparison
        // always match — real divergence would come from actual re-execution).
        for original_proof in original.proofs() {
            let mut proof = original_proof.clone();
            proof.proof_hash = DeterministicHasher::hash_execution_proof(&proof);
            let _ = replay_chain.append(proof);
        }

        replay_chain
    }

    /// Validate that a proof chain is structurally sound.
    pub fn validate_chain_structure(chain: &ProofChain) -> bool {
        chain.verify_integrity().is_ok()
    }

    /// Verify that state diffs in a proof are consistent.
    pub fn verify_state_continuity(chain: &ProofChain) -> bool {
        let proofs = chain.proofs();
        for i in 1..proofs.len() {
            if proofs[i - 1].post_state_hash != proofs[i].pre_state_hash {
                return false;
            }
        }
        true
    }
}
