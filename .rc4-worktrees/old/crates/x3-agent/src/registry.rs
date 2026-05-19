//! Agent registry — permissionless registration and management.
//!
//! NO whitelists. NO admin overrides. Bond is the only requirement.

use crate::error::AgentError;
use crate::types::*;
use std::collections::HashMap;
use x3_proof::types::{AgentIdentity, BlockHeight};

/// The agent registry. Permissionless. Decentralized. NO admin keys.
pub struct AgentRegistry {
    /// All registered agents by pubkey.
    agents: HashMap<[u8; 32], AgentRecord>,
    /// Ephemeral → primary key mapping.
    ephemeral_map: HashMap<[u8; 32], [u8; 32]>,
    /// Event log.
    events: Vec<AgentEvent>,
    /// Configuration.
    config: AgentConfig,
}

impl AgentRegistry {
    /// Create a new agent registry.
    pub fn new(config: AgentConfig) -> Self {
        Self {
            agents: HashMap::new(),
            ephemeral_map: HashMap::new(),
            events: Vec::new(),
            config,
        }
    }

    /// Register a new agent. Permissionless — bond is the only requirement.
    pub fn register(
        &mut self,
        identity: AgentIdentity,
        bond_amount: u128,
        current_block: BlockHeight,
    ) -> Result<(), AgentError> {
        if identity.ephemeral {
            return Err(AgentError::CannotRegisterEphemeral);
        }

        if bond_amount < self.config.min_registration_bond {
            return Err(AgentError::InsufficientBond {
                required: self.config.min_registration_bond,
                provided: bond_amount,
            });
        }

        if self.agents.contains_key(&identity.pubkey) {
            return Err(AgentError::AlreadyRegistered(identity.pubkey));
        }

        let record = AgentRecord {
            identity: identity.clone(),
            registered_at: current_block,
            status: AgentStatus::Active,
            initial_bond: bond_amount,
            current_bond: bond_amount,
            ephemeral_keys: Vec::new(),
            stats: AgentStats {
                last_active_at: current_block,
                ..Default::default()
            },
            reputation: x3_fees::types::AgentReputation {
                successes: 0,
                failures: 0,
                slashes: 0,
                total_volume: 0,
                age_blocks: 0,
            },
            policy: AgentPolicy::default(),
            constraints: AgentConstraints::default(),
            proof_commitment: [0u8; 32],
            proof_verified_at: 0,
            executions_since_proof_refresh: 0,
        };

        self.agents.insert(identity.pubkey, record);
        self.events.push(AgentEvent::Registered {
            pubkey: identity.pubkey,
            bond: bond_amount,
            block: current_block,
        });

        Ok(())
    }

    /// Link an ephemeral key to a registered agent.
    pub fn link_ephemeral(
        &mut self,
        primary_pubkey: &[u8; 32],
        ephemeral_pubkey: [u8; 32],
        current_block: BlockHeight,
    ) -> Result<(), AgentError> {
        let agent = self
            .agents
            .get_mut(primary_pubkey)
            .ok_or(AgentError::NotRegistered(*primary_pubkey))?;

        if agent.status != AgentStatus::Active {
            return Err(AgentError::NotActive(*primary_pubkey));
        }

        if agent.ephemeral_keys.len() >= self.config.max_ephemeral_keys {
            return Err(AgentError::TooManyEphemeralKeys {
                max: self.config.max_ephemeral_keys,
            });
        }

        if self.ephemeral_map.contains_key(&ephemeral_pubkey) {
            return Err(AgentError::EphemeralKeyAlreadyLinked(ephemeral_pubkey));
        }

        agent.ephemeral_keys.push(ephemeral_pubkey);
        self.ephemeral_map.insert(ephemeral_pubkey, *primary_pubkey);

        self.events.push(AgentEvent::EphemeralKeyLinked {
            pubkey: *primary_pubkey,
            ephemeral: ephemeral_pubkey,
            block: current_block,
        });

        Ok(())
    }

    /// Revoke an ephemeral key.
    pub fn revoke_ephemeral(
        &mut self,
        primary_pubkey: &[u8; 32],
        ephemeral_pubkey: &[u8; 32],
    ) -> Result<(), AgentError> {
        let agent = self
            .agents
            .get_mut(primary_pubkey)
            .ok_or(AgentError::NotRegistered(*primary_pubkey))?;

        agent.ephemeral_keys.retain(|k| k != ephemeral_pubkey);
        self.ephemeral_map.remove(ephemeral_pubkey);

        Ok(())
    }

    /// Record an execution result for an agent.
    pub fn record_execution(
        &mut self,
        pubkey: &[u8; 32],
        success: bool,
        volume: u128,
        fees_paid: u128,
        pnl: i128,
        current_block: BlockHeight,
    ) -> Result<(), AgentError> {
        let agent = self
            .agents
            .get_mut(pubkey)
            .ok_or(AgentError::NotRegistered(*pubkey))?;

        agent.stats.intents_submitted += 1;
        if success {
            agent.stats.intents_succeeded += 1;
            agent.reputation.successes += 1;
        } else {
            agent.stats.intents_failed += 1;
            agent.reputation.failures += 1;
        }
        agent.stats.total_volume += volume;
        agent.stats.total_fees_paid += fees_paid;
        agent.stats.total_profit += pnl;
        agent.stats.last_active_at = current_block;
        agent.reputation.total_volume += volume;
        agent.reputation.age_blocks = current_block.saturating_sub(agent.registered_at);

        self.events.push(AgentEvent::ExecutionRecorded {
            pubkey: *pubkey,
            success,
            volume,
            block: current_block,
        });

        Ok(())
    }

    /// Record a slashing event for an agent.
    pub fn record_slash(
        &mut self,
        pubkey: &[u8; 32],
        slash_amount: u128,
        reason: String,
        current_block: BlockHeight,
    ) -> Result<(), AgentError> {
        let agent = self
            .agents
            .get_mut(pubkey)
            .ok_or(AgentError::NotRegistered(*pubkey))?;

        agent.stats.slash_count += 1;
        agent.stats.total_slashed += slash_amount;
        agent.current_bond = agent.current_bond.saturating_sub(slash_amount);
        agent.reputation.slashes += 1;

        // Auto-deactivate if too many slashes
        if agent.stats.slash_count >= self.config.critical_slash_threshold {
            agent.status = AgentStatus::Deactivated;
            self.events.push(AgentEvent::Deactivated {
                pubkey: *pubkey,
                reason: format!(
                    "critical slash threshold reached ({})",
                    agent.stats.slash_count
                ),
                block: current_block,
            });
        }

        // Suspend if bond below minimum
        if agent.current_bond < self.config.min_active_bond && agent.status == AgentStatus::Active {
            agent.status = AgentStatus::Suspended;
        }

        self.events.push(AgentEvent::Slashed {
            pubkey: *pubkey,
            amount: slash_amount,
            reason,
            block: current_block,
        });

        Ok(())
    }

    /// Voluntarily deregister an agent. Returns remaining bond.
    pub fn deregister(
        &mut self,
        pubkey: &[u8; 32],
        current_block: BlockHeight,
    ) -> Result<u128, AgentError> {
        let agent = self
            .agents
            .get_mut(pubkey)
            .ok_or(AgentError::NotRegistered(*pubkey))?;

        if agent.status == AgentStatus::Deregistered {
            return Err(AgentError::AlreadyDeregistered(*pubkey));
        }

        let bond_returned = agent.current_bond;
        agent.status = AgentStatus::Deregistered;
        agent.current_bond = 0;

        // Remove ephemeral key mappings
        for key in &agent.ephemeral_keys {
            self.ephemeral_map.remove(key);
        }

        self.events.push(AgentEvent::Deregistered {
            pubkey: *pubkey,
            bond_returned,
            block: current_block,
        });

        Ok(bond_returned)
    }

    /// Resolve an identity — maps ephemeral keys to primary keys.
    pub fn resolve_identity(&self, pubkey: &[u8; 32]) -> Option<&AgentRecord> {
        // Try direct lookup first
        if let Some(agent) = self.agents.get(pubkey) {
            return Some(agent);
        }
        // Try ephemeral mapping
        if let Some(primary) = self.ephemeral_map.get(pubkey) {
            return self.agents.get(primary);
        }
        None
    }

    /// Get an agent record by pubkey.
    pub fn get(&self, pubkey: &[u8; 32]) -> Option<&AgentRecord> {
        self.agents.get(pubkey)
    }

    /// Get all active agents.
    pub fn active_agents(&self) -> Vec<&AgentRecord> {
        self.agents
            .values()
            .filter(|a| a.status == AgentStatus::Active)
            .collect()
    }

    /// Get event history.
    pub fn events(&self) -> &[AgentEvent] {
        &self.events
    }

    /// Total registered agents (all statuses).
    pub fn total_count(&self) -> usize {
        self.agents.len()
    }

    /// Total active agents.
    pub fn active_count(&self) -> usize {
        self.agents
            .values()
            .filter(|a| a.status == AgentStatus::Active)
            .count()
    }

    /// Get the configuration.
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> AgentIdentity {
        AgentIdentity {
            pubkey: [1u8; 32],
            ephemeral: false,
        }
    }

    #[test]
    fn test_register_agent() {
        let mut reg = AgentRegistry::new(AgentConfig::default());
        reg.register(test_identity(), 10_000_000, 100).unwrap();
        assert_eq!(reg.total_count(), 1);
        assert_eq!(reg.active_count(), 1);
    }

    #[test]
    fn test_insufficient_bond_rejected() {
        let mut reg = AgentRegistry::new(AgentConfig::default());
        let result = reg.register(test_identity(), 100, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_ephemeral_key_linking() {
        let mut reg = AgentRegistry::new(AgentConfig::default());
        reg.register(test_identity(), 10_000_000, 100).unwrap();

        let ephemeral = [2u8; 32];
        reg.link_ephemeral(&test_identity().pubkey, ephemeral, 101)
            .unwrap();

        let resolved = reg.resolve_identity(&ephemeral).unwrap();
        assert_eq!(resolved.identity.pubkey, test_identity().pubkey);
    }

    #[test]
    fn test_slash_deactivation() {
        let mut reg = AgentRegistry::new(AgentConfig {
            critical_slash_threshold: 2,
            ..Default::default()
        });
        reg.register(test_identity(), 10_000_000, 100).unwrap();

        reg.record_slash(&test_identity().pubkey, 1_000_000, "test1".to_string(), 110)
            .unwrap();
        assert_eq!(
            reg.get(&test_identity().pubkey).unwrap().status,
            AgentStatus::Active
        );

        reg.record_slash(&test_identity().pubkey, 1_000_000, "test2".to_string(), 120)
            .unwrap();
        assert_eq!(
            reg.get(&test_identity().pubkey).unwrap().status,
            AgentStatus::Deactivated
        );
    }

    #[test]
    fn test_reputation_tracking() {
        let mut reg = AgentRegistry::new(AgentConfig::default());
        reg.register(test_identity(), 10_000_000, 100).unwrap();

        for i in 0..10 {
            reg.record_execution(&test_identity().pubkey, true, 100_000, 1000, 500, 100 + i)
                .unwrap();
        }

        let agent = reg.get(&test_identity().pubkey).unwrap();
        assert_eq!(agent.stats.intents_succeeded, 10);
        assert_eq!(agent.reputation.successes, 10);
    }

    #[test]
    fn test_deregistration() {
        let mut reg = AgentRegistry::new(AgentConfig::default());
        reg.register(test_identity(), 10_000_000, 100).unwrap();

        let returned = reg.deregister(&test_identity().pubkey, 200).unwrap();
        assert_eq!(returned, 10_000_000);
        assert_eq!(reg.active_count(), 0);
    }
}
