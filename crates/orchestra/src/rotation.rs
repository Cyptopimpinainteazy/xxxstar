//! Rotation coordinator — orchestrates on-chain ↔ off-chain jury duty.

use crate::agent::identity::{AgentDomain, AgentId, AlignmentScore};
use crate::agent::off_chain::{OffChainAgent, OffChainRole};
use crate::agent::on_chain::OnChainAgent;
use crate::jury::rotation::{JuryRotation, RotationConfig, RotationResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Rotation session summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationSession {
    /// Session ID.
    pub session_id: String,
    /// Selected agent IDs.
    pub selected: Vec<AgentId>,
    /// When the rotation was created.
    pub created_at: chrono::DateTime<Utc>,
    /// Seed used for selection.
    pub seed_hex: String,
}

/// Coordinator that manages rotation into jury sessions.
pub struct RotationCoordinator {
    rotation: JuryRotation,
}

impl RotationCoordinator {
    pub fn new(config: RotationConfig) -> Self {
        Self {
            rotation: JuryRotation::new(config),
        }
    }

    /// Select agents for a jury session and generate off-chain agents.
    pub fn rotate_to_jury(
        &self,
        agents: &mut [OnChainAgent],
        target_size: u32,
        seed: &[u8; 32],
        session_id: String,
    ) -> (RotationSession, Vec<OffChainAgent>) {
        let selection = self.rotation.select(agents, target_size, seed);

        let mut off_chain_agents = Vec::new();

        for agent in agents.iter_mut() {
            if selection.selected.contains(&agent.identity.id) {
                agent.rotate_to_jury();
                let off_chain = OffChainAgent::from_on_chain(
                    agent.identity.clone(),
                    OffChainRole::JuryMember,
                    session_id.clone(),
                );
                off_chain_agents.push(off_chain);
            }
        }

        let session = RotationSession {
            session_id,
            selected: selection.selected.clone(),
            created_at: Utc::now(),
            seed_hex: selection.seed_hex,
        };

        (session, off_chain_agents)
    }

    /// Return agents from jury duty back to on-chain status.
    pub fn return_from_jury(
        &self,
        agents: &mut [OnChainAgent],
        session: &RotationSession,
    ) -> Vec<AgentId> {
        let mut returned = Vec::new();

        for agent in agents.iter_mut() {
            if session.selected.contains(&agent.identity.id) {
                agent.return_from_jury();
                returned.push(agent.identity.id);
            }
        }

        returned
    }

    /// Produce a deterministic seed from chain entropy (block hash + timestamp).
    pub fn seed_from_chain(block_hash: &[u8], timestamp: u64) -> [u8; 32] {
        JuryRotation::generate_seed(block_hash, timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::identity::OrchestraSection;

    fn make_agents(n: usize) -> Vec<OnChainAgent> {
        (0..n)
            .map(|i| {
                let mut agent = OnChainAgent::new(
                    i as u32,
                    format!("agent-{}", i),
                    OrchestraSection::Strings,
                );
                agent.identity.alignment = AlignmentScore::new(150);
                agent
            })
            .collect()
    }

    #[test]
    fn rotation_creates_off_chain_agents() {
        let mut agents = make_agents(10);
        let coordinator = RotationCoordinator::new(Default::default());
        let seed = [1u8; 32];

        let (session, off_chain) = coordinator.rotate_to_jury(
            &mut agents,
            3,
            &seed,
            "session-001".into(),
        );

        assert_eq!(session.selected.len(), off_chain.len());
        assert_eq!(off_chain.len(), 3);
        assert!(off_chain
            .iter()
            .all(|a| a.identity.domain == AgentDomain::OffChain));
    }

    #[test]
    fn return_from_jury_restores_status() {
        let mut agents = make_agents(5);
        let coordinator = RotationCoordinator::new(Default::default());
        let seed = [2u8; 32];

        let (session, _off_chain) = coordinator.rotate_to_jury(
            &mut agents,
            2,
            &seed,
            "session-002".into(),
        );

        let returned = coordinator.return_from_jury(&mut agents, &session);
        assert_eq!(returned.len(), 2);
    }
}
