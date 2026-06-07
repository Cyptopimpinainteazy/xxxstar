//! Proof generation engine.
//!
//! The ProofEngine is attached to the VM during execution and records
//! every state transition, producing cryptographic proofs that can be
//! independently verified via deterministic replay.

use crate::chain::ProofChain;
use crate::error::ProofError;
use crate::hasher::DeterministicHasher;
use crate::types::*;

/// Configuration for the proof engine.
#[derive(Debug, Clone)]
pub struct ProofEngineConfig {
    /// Maximum number of state diffs per proof.
    pub max_diffs_per_proof: usize,
    /// Whether to compute intermediate state roots.
    pub compute_intermediate_roots: bool,
}

impl Default for ProofEngineConfig {
    fn default() -> Self {
        Self {
            max_diffs_per_proof: 10_000,
            compute_intermediate_roots: true,
        }
    }
}

/// The proof generation engine — attached to VM execution context.
pub struct ProofEngine {
    config: ProofEngineConfig,
    /// Current proof chain being built.
    chain: ProofChain,
    /// Current block height.
    block_height: BlockHeight,
    /// Current agent identity.
    agent_id: AgentIdentity,
    /// Current program hash.
    program_hash: Hash256,
    /// Accumulated state diffs for the current atomic step.
    pending_diffs: Vec<StateDiff>,
    /// Current state snapshot (sorted key-value pairs).
    state_snapshot: Vec<(Vec<u8>, Vec<u8>)>,
    /// Next proof ID.
    next_proof_id: ProofId,
    /// Whether we're inside an atomic block.
    in_atomic: bool,
    /// Pre-state hash captured at atomic begin.
    atomic_pre_state: Option<Hash256>,
}

impl ProofEngine {
    /// Create a new proof engine for a given execution context.
    pub fn new(
        config: ProofEngineConfig,
        block_height: BlockHeight,
        agent_id: AgentIdentity,
        program_hash: Hash256,
    ) -> Self {
        Self {
            config,
            chain: ProofChain::new(block_height, program_hash),
            block_height,
            agent_id,
            program_hash,
            pending_diffs: Vec::new(),
            state_snapshot: Vec::new(),
            next_proof_id: 0,
            in_atomic: false,
            atomic_pre_state: None,
        }
    }

    /// Record a state write operation.
    pub fn record_state_write(
        &mut self,
        key: Vec<u8>,
        old_value: Option<Vec<u8>>,
        new_value: Option<Vec<u8>>,
    ) -> Result<(), ProofError> {
        if self.pending_diffs.len() >= self.config.max_diffs_per_proof {
            return Err(ProofError::TooManyDiffs {
                max: self.config.max_diffs_per_proof,
            });
        }

        self.pending_diffs.push(StateDiff {
            key,
            old_value,
            new_value,
        });

        Ok(())
    }

    /// Begin an atomic execution block.
    /// All state diffs within will be grouped into a single proof.
    pub fn begin_atomic(&mut self) -> Result<(), ProofError> {
        if self.in_atomic {
            return Err(ProofError::NestedAtomic);
        }
        self.in_atomic = true;
        self.atomic_pre_state = Some(self.current_state_hash());
        self.pending_diffs.clear();
        Ok(())
    }

    /// Commit an atomic block, producing an ExecutionProof.
    pub fn commit_atomic(
        &mut self,
        gas_consumed: u64,
        fee_charged: u64,
        intent_id: Option<IntentId>,
    ) -> Result<ExecutionProof, ProofError> {
        if !self.in_atomic {
            return Err(ProofError::NotInAtomic);
        }

        let pre_state_hash = self.atomic_pre_state.take().unwrap();

        // Drain diffs into a local vec to avoid borrow conflict
        let diffs = std::mem::take(&mut self.pending_diffs);
        for diff in &diffs {
            self.apply_diff_to_snapshot(diff);
        }

        let post_state_hash = self.current_state_hash();

        let mut proof = ExecutionProof {
            id: self.next_proof_id,
            block_height: self.block_height,
            program_hash: self.program_hash,
            pre_state_hash,
            post_state_hash,
            state_diffs: diffs,
            gas_consumed,
            fee_charged,
            agent_id: self.agent_id.clone(),
            intent_id,
            proof_hash: [0u8; 32], // Computed below
        };

        proof.proof_hash = DeterministicHasher::hash_execution_proof(&proof);
        self.next_proof_id += 1;
        self.in_atomic = false;

        self.chain.append(proof.clone())?;

        Ok(proof)
    }

    /// Rollback an atomic block, discarding pending diffs.
    pub fn rollback_atomic(&mut self) -> Result<(), ProofError> {
        if !self.in_atomic {
            return Err(ProofError::NotInAtomic);
        }
        self.pending_diffs.clear();
        self.atomic_pre_state = None;
        self.in_atomic = false;
        Ok(())
    }

    /// Emit a single-step proof outside an atomic block.
    pub fn emit_proof(
        &mut self,
        gas_consumed: u64,
        fee_charged: u64,
        intent_id: Option<IntentId>,
    ) -> Result<ExecutionProof, ProofError> {
        if self.in_atomic {
            return Err(ProofError::EmitDuringAtomic);
        }

        let pre_state_hash = self.current_state_hash();

        let diffs = std::mem::take(&mut self.pending_diffs);
        for diff in &diffs {
            self.apply_diff_to_snapshot(diff);
        }

        let post_state_hash = self.current_state_hash();

        let mut proof = ExecutionProof {
            id: self.next_proof_id,
            block_height: self.block_height,
            program_hash: self.program_hash,
            pre_state_hash,
            post_state_hash,
            state_diffs: diffs,
            gas_consumed,
            fee_charged,
            agent_id: self.agent_id.clone(),
            intent_id,
            proof_hash: [0u8; 32],
        };

        proof.proof_hash = DeterministicHasher::hash_execution_proof(&proof);
        self.next_proof_id += 1;

        self.chain.append(proof.clone())?;

        Ok(proof)
    }

    /// Finalize the proof chain and produce an execution receipt.
    pub fn finalize(self) -> Result<(ProofChain, ExecutionReceipt), ProofError> {
        if self.in_atomic {
            return Err(ProofError::UncommittedAtomic);
        }

        let chain_hash = self.chain.chain_hash();
        let total_gas = self.chain.total_gas();
        let total_fees = self.chain.total_fees();
        let final_state = self.current_state_hash();

        let receipt = ExecutionReceipt {
            proof_chain_hash: chain_hash,
            total_gas,
            total_fees,
            final_state_root: final_state,
            success: true,
            finalized_at: self.block_height,
            agent_id: self.agent_id,
        };

        Ok((self.chain, receipt))
    }

    /// Get the current state hash.
    fn current_state_hash(&self) -> Hash256 {
        DeterministicHasher::hash_state(&self.state_snapshot)
    }

    /// Apply a state diff to the internal snapshot.
    fn apply_diff_to_snapshot(&mut self, diff: &StateDiff) {
        match &diff.new_value {
            Some(val) => {
                // Upsert
                match self
                    .state_snapshot
                    .binary_search_by(|(k, _)| k.cmp(&diff.key))
                {
                    Ok(idx) => self.state_snapshot[idx].1 = val.clone(),
                    Err(idx) => self
                        .state_snapshot
                        .insert(idx, (diff.key.clone(), val.clone())),
                }
            }
            None => {
                // Delete
                if let Ok(idx) = self
                    .state_snapshot
                    .binary_search_by(|(k, _)| k.cmp(&diff.key))
                {
                    self.state_snapshot.remove(idx);
                }
            }
        }
    }

    /// Get a reference to the current proof chain.
    pub fn chain(&self) -> &ProofChain {
        &self.chain
    }

    /// Check if currently inside an atomic block.
    pub fn is_atomic(&self) -> bool {
        self.in_atomic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_agent() -> AgentIdentity {
        AgentIdentity {
            pubkey: [1u8; 32],
            ephemeral: false,
        }
    }

    #[test]
    fn test_basic_proof_generation() {
        let mut engine =
            ProofEngine::new(ProofEngineConfig::default(), 100, test_agent(), [0xAA; 32]);

        engine
            .record_state_write(vec![1, 2, 3], None, Some(vec![42]))
            .unwrap();

        let proof = engine.emit_proof(1000, 50, None).unwrap();
        assert_eq!(proof.id, 0);
        assert_eq!(proof.block_height, 100);
        assert_eq!(proof.gas_consumed, 1000);
        assert_eq!(proof.state_diffs.len(), 1);
        assert_ne!(proof.proof_hash, [0u8; 32]);
    }

    #[test]
    fn test_atomic_proof() {
        let mut engine =
            ProofEngine::new(ProofEngineConfig::default(), 100, test_agent(), [0xBB; 32]);

        engine.begin_atomic().unwrap();
        engine
            .record_state_write(vec![1], None, Some(vec![10]))
            .unwrap();
        engine
            .record_state_write(vec![2], None, Some(vec![20]))
            .unwrap();

        let proof = engine.commit_atomic(2000, 100, None).unwrap();
        assert_eq!(proof.state_diffs.len(), 2);
    }

    #[test]
    fn test_atomic_rollback() {
        let mut engine =
            ProofEngine::new(ProofEngineConfig::default(), 100, test_agent(), [0xCC; 32]);

        engine.begin_atomic().unwrap();
        engine
            .record_state_write(vec![1], None, Some(vec![10]))
            .unwrap();
        engine.rollback_atomic().unwrap();

        // After rollback, state should be unchanged
        let proof = engine.emit_proof(0, 0, None).unwrap();
        assert_eq!(proof.state_diffs.len(), 0);
    }

    #[test]
    fn test_finalize_receipt() {
        let mut engine =
            ProofEngine::new(ProofEngineConfig::default(), 100, test_agent(), [0xDD; 32]);

        engine
            .record_state_write(vec![1], None, Some(vec![10]))
            .unwrap();
        engine.emit_proof(1000, 50, None).unwrap();

        engine
            .record_state_write(vec![2], None, Some(vec![20]))
            .unwrap();
        engine.emit_proof(500, 25, None).unwrap();

        let (chain, receipt) = engine.finalize().unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(receipt.total_gas, 1500);
        assert_eq!(receipt.total_fees, 75);
        assert!(receipt.success);
    }
}
