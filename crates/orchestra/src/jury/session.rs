//! Jury session management — creation, lifecycle, task assignment.

use super::voting::BallotBox;
use crate::agent::identity::{AgentId, OrchestraSection};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a jury session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Minimum number of jury members.
    pub min_jury_size: u32,
    /// Maximum number of jury members.
    pub max_jury_size: u32,
    /// Maximum proportion from any single section (prevents bias).
    pub max_section_proportion: f64,
    /// Duration of the voting period.
    pub voting_duration_secs: u64,
    /// Whether to require confidence weighting.
    pub require_confidence: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            min_jury_size: 5,
            max_jury_size: 15,
            max_section_proportion: OrchestraSection::MAX_JURY_PROPORTION, // 0.4
            voting_duration_secs: 300, // 5 minutes
            require_confidence: false,
        }
    }
}

/// Status of a jury session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is being assembled (jury members being selected).
    Assembling,
    /// Session is active — jury members are reviewing and voting.
    Active,
    /// Voting period has ended; tallying results.
    Tallying,
    /// Session is complete; results published.
    Complete,
    /// Session expired without quorum.
    Expired,
}

/// Record of a jury member's participation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JuryMember {
    /// Agent ID (known to the system but NOT to other jury members).
    pub agent_id: AgentId,
    /// Anonymous slot number (visible during voting).
    pub slot: u32,
    /// Orchestra section (for proportion balancing).
    pub section: OrchestraSection,
    /// Whether this member was rotated from on-chain.
    pub rotated_from_on_chain: bool,
    /// Whether this member has submitted a commitment.
    pub has_voted: bool,
}

/// A jury session — manages a group of jurors voting on tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurySession {
    /// Unique session identifier.
    pub session_id: String,
    /// Configuration.
    pub config: SessionConfig,
    /// Current status.
    pub status: SessionStatus,
    /// Jury members (indexed by slot).
    pub members: Vec<JuryMember>,
    /// Ballot boxes for each task in this session.
    pub ballots: HashMap<String, BallotBox>,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When voting started (None if still assembling).
    pub voting_started_at: Option<DateTime<Utc>>,
    /// When the session completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Tasks assigned to this session.
    pub task_ids: Vec<String>,
}

impl JurySession {
    /// Create a new jury session.
    pub fn new(session_id: String, config: SessionConfig) -> Self {
        Self {
            session_id,
            config,
            status: SessionStatus::Assembling,
            members: Vec::new(),
            ballots: HashMap::new(),
            created_at: Utc::now(),
            voting_started_at: None,
            completed_at: None,
            task_ids: Vec::new(),
        }
    }

    /// Add a jury member to the session.
    /// Returns the assigned slot number, or an error if session is full or
    /// section proportion would be exceeded.
    pub fn add_member(
        &mut self,
        agent_id: AgentId,
        section: OrchestraSection,
        rotated_from_on_chain: bool,
    ) -> Result<u32, JuryError> {
        if self.status != SessionStatus::Assembling {
            return Err(JuryError::SessionNotAssembling);
        }

        if self.members.len() as u32 >= self.config.max_jury_size {
            return Err(JuryError::SessionFull);
        }

        // Check section proportion
        let section_count = self
            .members
            .iter()
            .filter(|m| m.section == section)
            .count() as f64;
        let new_total = self.members.len() as f64 + 1.0;
        if section_count + 1.0 > new_total * self.config.max_section_proportion {
            return Err(JuryError::SectionProportionExceeded(section));
        }

        // Check for duplicate
        if self.members.iter().any(|m| m.agent_id == agent_id) {
            return Err(JuryError::DuplicateMember(agent_id));
        }

        let slot = self.members.len() as u32;
        self.members.push(JuryMember {
            agent_id,
            slot,
            section,
            rotated_from_on_chain,
            has_voted: false,
        });

        Ok(slot)
    }

    /// Add a task to the session for review.
    pub fn add_task(&mut self, task_id: String) {
        let ballot = BallotBox::new(task_id.clone(), self.members.len() as u32);
        self.ballots.insert(task_id.clone(), ballot);
        self.task_ids.push(task_id);
    }

    /// Start the voting period (transition from Assembling → Active).
    pub fn start_voting(&mut self) -> Result<(), JuryError> {
        if self.status != SessionStatus::Assembling {
            return Err(JuryError::InvalidTransition(self.status, SessionStatus::Active));
        }

        if (self.members.len() as u32) < self.config.min_jury_size {
            return Err(JuryError::InsufficientJurors(
                self.members.len() as u32,
                self.config.min_jury_size,
            ));
        }

        if self.task_ids.is_empty() {
            return Err(JuryError::NoTasks);
        }

        self.status = SessionStatus::Active;
        self.voting_started_at = Some(Utc::now());
        Ok(())
    }

    /// Check if the voting period has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(started) = self.voting_started_at {
            let deadline =
                started + Duration::seconds(self.config.voting_duration_secs as i64);
            Utc::now() > deadline
        } else {
            false
        }
    }

    /// Get the ballot box for a task.
    pub fn get_ballot(&self, task_id: &str) -> Option<&BallotBox> {
        self.ballots.get(task_id)
    }

    /// Get a mutable ballot box.
    pub fn get_ballot_mut(&mut self, task_id: &str) -> Option<&mut BallotBox> {
        self.ballots.get_mut(task_id)
    }

    /// Mark a member as having voted (without revealing the vote).
    pub fn record_vote(&mut self, slot: u32) -> Result<(), JuryError> {
        let member = self
            .members
            .iter_mut()
            .find(|m| m.slot == slot)
            .ok_or(JuryError::UnknownSlot(slot))?;

        if member.has_voted {
            return Err(JuryError::AlreadyVoted(slot));
        }

        member.has_voted = true;
        Ok(())
    }

    /// Transition to tallying (close all ballot boxes).
    pub fn close_voting(&mut self) -> Result<(), JuryError> {
        if self.status != SessionStatus::Active {
            return Err(JuryError::InvalidTransition(
                self.status,
                SessionStatus::Tallying,
            ));
        }

        for ballot in self.ballots.values_mut() {
            ballot.close_commitments();
        }

        self.status = SessionStatus::Tallying;
        Ok(())
    }

    /// Complete the session after tallying.
    pub fn complete(&mut self) {
        self.status = SessionStatus::Complete;
        self.completed_at = Some(Utc::now());
    }

    /// Get voting statistics.
    pub fn voting_stats(&self) -> SessionVotingStats {
        let total_members = self.members.len() as u32;
        let voted = self.members.iter().filter(|m| m.has_voted).count() as u32;

        SessionVotingStats {
            total_members,
            voted,
            not_voted: total_members - voted,
            participation_rate: if total_members > 0 {
                voted as f64 / total_members as f64
            } else {
                0.0
            },
        }
    }
}

/// Session voting statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionVotingStats {
    pub total_members: u32,
    pub voted: u32,
    pub not_voted: u32,
    pub participation_rate: f64,
}

/// Jury session errors.
#[derive(Debug, thiserror::Error)]
pub enum JuryError {
    #[error("Session is not in Assembling state")]
    SessionNotAssembling,
    #[error("Session is full (max jury size reached)")]
    SessionFull,
    #[error("Section {0} proportion would exceed maximum")]
    SectionProportionExceeded(OrchestraSection),
    #[error("Agent {0} is already a member")]
    DuplicateMember(AgentId),
    #[error("Invalid state transition from {0:?} to {1:?}")]
    InvalidTransition(SessionStatus, SessionStatus),
    #[error("Insufficient jurors: {0}/{1} required")]
    InsufficientJurors(u32, u32),
    #[error("No tasks assigned to session")]
    NoTasks,
    #[error("Unknown slot {0}")]
    UnknownSlot(u32),
    #[error("Slot {0} has already voted")]
    AlreadyVoted(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_session() -> JurySession {
        JurySession::new(
            "session-001".into(),
            SessionConfig {
                min_jury_size: 3,
                max_jury_size: 7,
                ..Default::default()
            },
        )
    }

    #[test]
    fn assemble_jury() {
        let mut session = default_session();
        session.add_member(1, OrchestraSection::Strings, false).unwrap();
        session.add_member(2, OrchestraSection::Brass, true).unwrap();
        session.add_member(3, OrchestraSection::Percussion, false).unwrap();

        assert_eq!(session.members.len(), 3);
    }

    #[test]
    fn section_proportion_enforced() {
        let mut session = JurySession::new(
            "session-002".into(),
            SessionConfig {
                min_jury_size: 2,
                max_jury_size: 5,
                max_section_proportion: 0.4,
                ..Default::default()
            },
        );

        session.add_member(1, OrchestraSection::Strings, false).unwrap();
        // Second Strings would be 2/2 = 100% > 40%
        let result = session.add_member(2, OrchestraSection::Strings, false);
        assert!(result.is_err());
    }

    #[test]
    fn full_session_lifecycle() {
        let mut session = default_session();
        session.add_member(1, OrchestraSection::Strings, false).unwrap();
        session.add_member(2, OrchestraSection::Brass, true).unwrap();
        session.add_member(3, OrchestraSection::Percussion, false).unwrap();

        session.add_task("task-001".into());
        session.start_voting().unwrap();
        assert_eq!(session.status, SessionStatus::Active);

        // Record votes
        session.record_vote(0).unwrap();
        session.record_vote(1).unwrap();
        session.record_vote(2).unwrap();

        let stats = session.voting_stats();
        assert_eq!(stats.voted, 3);
        assert!((stats.participation_rate - 1.0).abs() < f64::EPSILON);

        session.close_voting().unwrap();
        assert_eq!(session.status, SessionStatus::Tallying);

        session.complete();
        assert_eq!(session.status, SessionStatus::Complete);
    }

    #[test]
    fn cannot_start_without_quorum() {
        let mut session = default_session();
        session.add_member(1, OrchestraSection::Strings, false).unwrap();
        session.add_task("task-001".into());

        let result = session.start_voting();
        assert!(result.is_err()); // only 1 of 3 minimum
    }
}
