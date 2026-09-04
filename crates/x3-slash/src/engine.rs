//! Slashing engine — the core enforcement mechanism.

use crate::bond::BondManager;
use crate::error::SlashError;
use crate::record::SlashRecord;
use crate::types::*;
use x3_proof::hasher::DeterministicHasher;
use x3_proof::types::{AgentIdentity, BlockHeight, Hash256, IntentId};

/// The slashing engine. Enforces protocol rules automatically.
/// No humans. No voting. No mercy.
pub struct SlashingEngine {
    /// Bond manager for escrow operations.
    bond_manager: BondManager,
    /// Permanent record of all slashing events.
    record: SlashRecord,
    /// Next slash event ID.
    next_slash_id: u64,
    /// Configuration.
    _config: SlashConfig,
}

impl SlashingEngine {
    /// Create a new slashing engine.
    pub fn new(config: SlashConfig) -> Self {
        let bond_manager = BondManager::new(config.clone());
        Self {
            bond_manager,
            record: SlashRecord::new(),
            next_slash_id: 0,
            _config: config,
        }
    }

    /// Post a bond for an agent. Required before any execution.
    pub fn post_bond(
        &mut self,
        agent_id: AgentIdentity,
        amount: Amount,
        current_block: BlockHeight,
        intent_id: Option<IntentId>,
    ) -> Result<BondId, SlashError> {
        self.bond_manager
            .post_bond(agent_id, amount, current_block, intent_id)
    }

    /// Execute a slash. This is called by the court system or by
    /// automatic detection mechanisms.
    pub fn execute_slash(
        &mut self,
        agent_id: AgentIdentity,
        bond_id: BondId,
        reason: SlashReason,
        current_block: BlockHeight,
    ) -> Result<SlashEvent, SlashError> {
        // Determine severity from reason
        let severity = Self::determine_severity(&reason);

        // Execute the slash on the bond
        let amount_slashed = self.bond_manager.slash(bond_id, severity)?;

        // Create the slash event
        let mut event = SlashEvent {
            id: self.next_slash_id,
            agent_id,
            bond_id,
            reason,
            severity,
            amount_slashed,
            slashed_at: current_block,
            event_hash: [0u8; 32],
        };

        // Compute event hash
        event.event_hash = Self::hash_slash_event(&event);
        self.next_slash_id += 1;

        // Record permanently
        self.record.record(event.clone());

        Ok(event)
    }

    /// Release a bond after successful execution.
    pub fn release_bond(&mut self, bond_id: BondId) -> Result<Amount, SlashError> {
        self.bond_manager.release(bond_id)
    }

    /// Process expired bonds — called periodically (e.g., in on_initialize).
    pub fn process_block(&mut self, current_block: BlockHeight) -> Vec<SlashEvent> {
        let expired = self.bond_manager.process_expiries(current_block);
        let mut events = Vec::new();

        for bond_id in expired {
            if let Some(bond) = self.bond_manager.get(bond_id) {
                let agent_id = bond.agent_id.clone();
                let intent_id = bond.intent_id;
                let reason = if let Some(iid) = intent_id {
                    SlashReason::BondExpiry {
                        bond_id,
                        intent_id: iid,
                    }
                } else {
                    SlashReason::ExecutionTimeout {
                        intent_id: IntentId(0),
                        timeout_block: current_block,
                    }
                };

                if let Ok(event) = self.execute_slash(agent_id, bond_id, reason, current_block) {
                    events.push(event);
                }
            }
        }

        events
    }

    /// Slash for execution failure.
    pub fn slash_execution_failure(
        &mut self,
        agent_id: AgentIdentity,
        bond_id: BondId,
        proof_hash: Hash256,
        failure_at: u64,
        current_block: BlockHeight,
    ) -> Result<SlashEvent, SlashError> {
        self.execute_slash(
            agent_id,
            bond_id,
            SlashReason::ExecutionFailure {
                proof_hash,
                failure_at,
            },
            current_block,
        )
    }

    /// Slash for state divergence (detected by court replay).
    pub fn slash_state_divergence(
        &mut self,
        agent_id: AgentIdentity,
        bond_id: BondId,
        original_proof: Hash256,
        replay_proof: Hash256,
        divergence_at: usize,
        current_block: BlockHeight,
    ) -> Result<SlashEvent, SlashError> {
        self.execute_slash(
            agent_id,
            bond_id,
            SlashReason::StateDivergence {
                original_proof,
                replay_proof,
                divergence_at,
            },
            current_block,
        )
    }

    /// Slash for double execution.
    pub fn slash_double_execution(
        &mut self,
        agent_id: AgentIdentity,
        bond_id: BondId,
        intent_id: IntentId,
        first_proof: Hash256,
        second_proof: Hash256,
        current_block: BlockHeight,
    ) -> Result<SlashEvent, SlashError> {
        self.execute_slash(
            agent_id,
            bond_id,
            SlashReason::DoubleExecution {
                intent_id,
                first_proof,
                second_proof,
            },
            current_block,
        )
    }

    /// Get the slash record.
    pub fn record(&self) -> &SlashRecord {
        &self.record
    }

    /// Get the bond manager.
    pub fn bond_manager(&self) -> &BondManager {
        &self.bond_manager
    }

    /// Determine severity from slash reason. This is deterministic.
    fn determine_severity(reason: &SlashReason) -> SlashSeverity {
        match reason {
            SlashReason::ExecutionFailure { .. } => SlashSeverity::Moderate,
            SlashReason::StateDivergence { .. } => SlashSeverity::Critical,
            SlashReason::BondExpiry { .. } => SlashSeverity::Minor,
            SlashReason::InvalidProof { .. } => SlashSeverity::Critical,
            SlashReason::ExecutionTimeout { .. } => SlashSeverity::Moderate,
            SlashReason::DoubleExecution { .. } => SlashSeverity::Critical,
        }
    }

    /// Compute the deterministic hash of a slash event.
    fn hash_slash_event(event: &SlashEvent) -> Hash256 {
        let mut h = DeterministicHasher::new();
        h.update_u64(event.id);
        h.update(&event.agent_id.pubkey);
        h.update(&[event.agent_id.ephemeral as u8]);
        h.update_u64(event.bond_id.0);
        // Hash reason discriminant + data
        let reason_bytes = serde_json::to_vec(&event.reason).unwrap_or_default();
        h.update_bytes(&reason_bytes);
        h.update_u64(event.severity.slash_bps());
        h.update_u128(event.amount_slashed);
        h.update_u64(event.slashed_at);
        h.finalize()
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
    fn test_slash_execution_failure() {
        let mut engine = SlashingEngine::new(SlashConfig::default());
        let bond_id = engine
            .post_bond(test_agent(), 2_000_000, 100, None)
            .unwrap();

        let event = engine
            .slash_execution_failure(test_agent(), bond_id, [0xAA; 32], 42, 110)
            .unwrap();

        assert_eq!(event.severity, SlashSeverity::Moderate);
        assert_eq!(event.amount_slashed, 1_000_000); // 50%
        assert_ne!(event.event_hash, [0u8; 32]);
    }

    #[test]
    fn test_slash_state_divergence_is_critical() {
        let mut engine = SlashingEngine::new(SlashConfig::default());
        let bond_id = engine
            .post_bond(test_agent(), 2_000_000, 100, None)
            .unwrap();

        let event = engine
            .slash_state_divergence(test_agent(), bond_id, [0xAA; 32], [0xBB; 32], 5, 110)
            .unwrap();

        assert_eq!(event.severity, SlashSeverity::Critical);
        assert_eq!(event.amount_slashed, 2_000_000); // 100%
    }

    #[test]
    fn test_slash_record_is_permanent() {
        let mut engine = SlashingEngine::new(SlashConfig::default());
        let bond_id = engine
            .post_bond(test_agent(), 2_000_000, 100, None)
            .unwrap();

        engine
            .slash_execution_failure(test_agent(), bond_id, [0xAA; 32], 42, 110)
            .unwrap();

        let history = engine.record().get_agent_history(&test_agent().pubkey);
        assert_eq!(history.len(), 1);
    }
}
