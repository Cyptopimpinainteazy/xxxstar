//! Agent lifecycle — spawn, retire, scrap yard recycling.

use super::identity::{AgentId, AlignmentScore, OrchestraSection};
use super::off_chain::{OffChainAgent, OffChainRole};
use super::on_chain::OnChainAgent;
use crate::audit::AuditEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parameters for spawning a new agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnParams {
    /// Agent ID (assigned by the system).
    pub id: AgentId,
    /// Display name.
    pub display_name: String,
    /// Orchestra section.
    pub section: OrchestraSection,
    /// Initial alignment score (defaults to NEUTRAL if not set).
    pub initial_alignment: Option<AlignmentScore>,
    /// Whether this is an on-chain or off-chain spawn.
    pub spawn_domain: SpawnDomain,
    /// Off-chain role (only if spawning off-chain).
    pub off_chain_role: Option<OffChainRole>,
    /// Session ID (only if spawning for a specific jury session).
    pub session_id: Option<String>,
}

/// Where to spawn the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnDomain {
    OnChain,
    OffChain,
}

/// Events in an agent's lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    /// Agent spawned.
    Spawned {
        agent_id: AgentId,
        section: OrchestraSection,
        timestamp: DateTime<Utc>,
    },
    /// Agent rotated from on-chain to off-chain.
    RotatedToOffChain {
        agent_id: AgentId,
        session_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent returned from off-chain to on-chain.
    ReturnedToOnChain {
        agent_id: AgentId,
        session_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent slashed for Score violation.
    Slashed {
        agent_id: AgentId,
        reason: String,
        alignment_before: AlignmentScore,
        alignment_after: AlignmentScore,
        timestamp: DateTime<Utc>,
    },
    /// Agent retired to scrap yard.
    Retired {
        agent_id: AgentId,
        reason: String,
        final_alignment: AlignmentScore,
        tasks_completed: u64,
        timestamp: DateTime<Utc>,
    },
    /// Agent recycled from scrap yard (data used for training).
    Recycled {
        agent_id: AgentId,
        data_harvested: bool,
        timestamp: DateTime<Utc>,
    },
}

/// Manages the full lifecycle of agents in the Orchestra.
pub struct AgentLifecycle {
    /// Next available agent ID.
    next_id: AgentId,
    /// Event log.
    events: Vec<LifecycleEvent>,
}

impl AgentLifecycle {
    pub fn new(starting_id: AgentId) -> Self {
        Self {
            next_id: starting_id,
            events: Vec::new(),
        }
    }

    /// Allocate the next agent ID.
    fn allocate_id(&mut self) -> AgentId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Spawn a new on-chain agent.
    pub fn spawn_on_chain(
        &mut self,
        display_name: String,
        section: OrchestraSection,
    ) -> OnChainAgent {
        let id = self.allocate_id();
        let agent = OnChainAgent::new(id, display_name, section);

        self.events.push(LifecycleEvent::Spawned {
            agent_id: id,
            section,
            timestamp: Utc::now(),
        });

        agent
    }

    /// Spawn a new off-chain agent for a specific session.
    pub fn spawn_off_chain(
        &mut self,
        display_name: String,
        section: OrchestraSection,
        role: OffChainRole,
        session_id: Option<String>,
    ) -> OffChainAgent {
        let id = self.allocate_id();
        let mut agent = OffChainAgent::new(id, display_name, section, role);
        agent.session_id = session_id;

        self.events.push(LifecycleEvent::Spawned {
            agent_id: id,
            section,
            timestamp: Utc::now(),
        });

        agent
    }

    /// Spawn from parameters.
    pub fn spawn(&mut self, params: SpawnParams) -> SpawnResult {
        match params.spawn_domain {
            SpawnDomain::OnChain => {
                let mut agent = self.spawn_on_chain(params.display_name, params.section);
                if let Some(alignment) = params.initial_alignment {
                    agent.identity.alignment = alignment;
                }
                SpawnResult::OnChain(agent)
            }
            SpawnDomain::OffChain => {
                let role = params.off_chain_role.unwrap_or(OffChainRole::JuryMember);
                let mut agent =
                    self.spawn_off_chain(params.display_name, params.section, role, params.session_id);
                if let Some(alignment) = params.initial_alignment {
                    agent.identity.alignment = alignment;
                }
                SpawnResult::OffChain(agent)
            }
        }
    }

    /// Record a retirement event.
    pub fn retire_agent(
        &mut self,
        agent_id: AgentId,
        reason: String,
        final_alignment: AlignmentScore,
        tasks_completed: u64,
    ) {
        self.events.push(LifecycleEvent::Retired {
            agent_id,
            reason,
            final_alignment,
            tasks_completed,
            timestamp: Utc::now(),
        });
    }

    /// Record a rotation event (on-chain → off-chain).
    pub fn record_rotation_out(&mut self, agent_id: AgentId, session_id: String) {
        self.events.push(LifecycleEvent::RotatedToOffChain {
            agent_id,
            session_id,
            timestamp: Utc::now(),
        });
    }

    /// Record a return event (off-chain → on-chain).
    pub fn record_rotation_in(&mut self, agent_id: AgentId, session_id: String) {
        self.events.push(LifecycleEvent::ReturnedToOnChain {
            agent_id,
            session_id,
            timestamp: Utc::now(),
        });
    }

    /// Record a recycling event (scrap yard).
    pub fn record_recycled(&mut self, agent_id: AgentId, data_harvested: bool) {
        self.events.push(LifecycleEvent::Recycled {
            agent_id,
            data_harvested,
            timestamp: Utc::now(),
        });
    }

    /// Get all lifecycle events.
    pub fn events(&self) -> &[LifecycleEvent] {
        &self.events
    }

    /// Get events for a specific agent.
    pub fn agent_events(&self, agent_id: AgentId) -> Vec<&LifecycleEvent> {
        self.events
            .iter()
            .filter(|e| match e {
                LifecycleEvent::Spawned { agent_id: id, .. }
                | LifecycleEvent::RotatedToOffChain { agent_id: id, .. }
                | LifecycleEvent::ReturnedToOnChain { agent_id: id, .. }
                | LifecycleEvent::Slashed { agent_id: id, .. }
                | LifecycleEvent::Retired { agent_id: id, .. }
                | LifecycleEvent::Recycled { agent_id: id, .. } => *id == agent_id,
            })
            .collect()
    }
}

/// Result of spawning an agent.
pub enum SpawnResult {
    OnChain(OnChainAgent),
    OffChain(OffChainAgent),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_ids_increment() {
        let mut lifecycle = AgentLifecycle::new(100);
        let a1 = lifecycle.spawn_on_chain("agent-1".into(), OrchestraSection::Strings);
        let a2 = lifecycle.spawn_on_chain("agent-2".into(), OrchestraSection::Brass);
        assert_eq!(a1.identity.id, 100);
        assert_eq!(a2.identity.id, 101);
    }

    #[test]
    fn lifecycle_events_are_recorded() {
        let mut lifecycle = AgentLifecycle::new(1);
        let agent = lifecycle.spawn_on_chain("test".into(), OrchestraSection::Percussion);
        lifecycle.retire_agent(
            agent.identity.id,
            "misalignment".into(),
            agent.identity.alignment,
            0,
        );

        let events = lifecycle.agent_events(agent.identity.id);
        assert_eq!(events.len(), 2); // Spawned + Retired
    }
}
