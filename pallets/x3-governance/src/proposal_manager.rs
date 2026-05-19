//! Proposal Manager — handles proposal lifecycle and state transitions
//!
//! Tracks: creation, voting, approval, execution, rejection

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalPhase {
    Created,
    VotingActive,
    VotingClosed,
    Approved,
    Executed,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalMetrics {
    pub id: u32,
    pub phase: ProposalPhase,
    pub votes_for: u128,
    pub votes_against: u128,
    pub votes_abstain: u128,
    pub participation_rate: f64,
    pub approval_rate: f64,
}

pub struct ProposalManager {
    proposals: HashMap<u32, ProposalMetrics>,
    next_id: u32,
}

impl ProposalManager {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create_proposal(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.proposals.insert(
            id,
            ProposalMetrics {
                id,
                phase: ProposalPhase::Created,
                votes_for: 0,
                votes_against: 0,
                votes_abstain: 0,
                participation_rate: 0.0,
                approval_rate: 0.0,
            },
        );

        id
    }

    pub fn get_proposal(&self, id: u32) -> Option<&ProposalMetrics> {
        self.proposals.get(&id)
    }

    pub fn update_phase(&mut self, id: u32, phase: ProposalPhase) -> bool {
        if let Some(proposal) = self.proposals.get_mut(&id) {
            proposal.phase = phase;
            return true;
        }
        false
    }

    pub fn add_vote(&mut self, id: u32, vote_type: &str, power: u128) -> bool {
        if let Some(proposal) = self.proposals.get_mut(&id) {
            match vote_type {
                "for" => proposal.votes_for += power,
                "against" => proposal.votes_against += power,
                "abstain" => proposal.votes_abstain += power,
                _ => return false,
            }

            // Recalculate rates
            let total = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;
            if total > 0 {
                proposal.participation_rate = (total as f64) / 1_000_000.0; // Normalized
                proposal.approval_rate =
                    (proposal.votes_for as f64) / (total as f64);
            }

            return true;
        }
        false
    }

    pub fn should_approve(&self, id: u32, threshold: f64) -> bool {
        if let Some(proposal) = self.proposals.get(&id) {
            return proposal.approval_rate >= threshold;
        }
        false
    }

    pub fn list_by_phase(&self, phase: ProposalPhase) -> Vec<u32> {
        self.proposals
            .values()
            .filter(|p| p.phase == phase)
            .map(|p| p.id)
            .collect()
    }

    pub fn count_total(&self) -> usize {
        self.proposals.len()
    }
}

impl Default for ProposalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_proposal() {
        let mut pm = ProposalManager::new();
        let id = pm.create_proposal();
        assert_eq!(id, 1);
    }

    #[test]
    fn test_multiple_proposals() {
        let mut pm = ProposalManager::new();
        let id1 = pm.create_proposal();
        let id2 = pm.create_proposal();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_get_proposal() {
        let mut pm = ProposalManager::new();
        let id = pm.create_proposal();
        let proposal = pm.get_proposal(id).unwrap();
        assert_eq!(proposal.phase, ProposalPhase::Created);
    }

    #[test]
    fn test_update_phase() {
        let mut pm = ProposalManager::new();
        let id = pm.create_proposal();
        pm.update_phase(id, ProposalPhase::VotingActive);
        assert_eq!(pm.get_proposal(id).unwrap().phase, ProposalPhase::VotingActive);
    }

    #[test]
    fn test_add_vote() {
        let mut pm = ProposalManager::new();
        let id = pm.create_proposal();
        pm.add_vote(id, "for", 1000);
        pm.add_vote(id, "against", 200);

        let proposal = pm.get_proposal(id).unwrap();
        assert_eq!(proposal.votes_for, 1000);
        assert_eq!(proposal.votes_against, 200);
    }

    #[test]
    fn test_approval_rate() {
        let mut pm = ProposalManager::new();
        let id = pm.create_proposal();
        pm.add_vote(id, "for", 700);
        pm.add_vote(id, "against", 300);

        let proposal = pm.get_proposal(id).unwrap();
        assert!(proposal.approval_rate > 0.69 && proposal.approval_rate < 0.71);
    }

    #[test]
    fn test_should_approve() {
        let mut pm = ProposalManager::new();
        let id = pm.create_proposal();
        pm.add_vote(id, "for", 666);
        pm.add_vote(id, "against", 333);

        assert!(pm.should_approve(id, 0.5));
        assert!(pm.should_approve(id, 0.6));
        assert!(!pm.should_approve(id, 0.8));
    }

    #[test]
    fn test_list_by_phase() {
        let mut pm = ProposalManager::new();
        let id1 = pm.create_proposal();
        let id2 = pm.create_proposal();

        pm.update_phase(id1, ProposalPhase::VotingActive);

        let active = pm.list_by_phase(ProposalPhase::VotingActive);
        assert_eq!(active.len(), 1);
        assert!(active.contains(&id1));
    }

    #[test]
    fn test_count_total() {
        let mut pm = ProposalManager::new();
        pm.create_proposal();
        pm.create_proposal();
        pm.create_proposal();
        assert_eq!(pm.count_total(), 3);
    }

    #[test]
    fn test_proposal_phase_enum() {
        assert_eq!(ProposalPhase::Created, ProposalPhase::Created);
        assert_ne!(ProposalPhase::Created, ProposalPhase::Approved);
    }

    #[test]
    fn test_nonexistent_proposal() {
        let pm = ProposalManager::new();
        assert!(pm.get_proposal(999).is_none());
    }

    #[test]
    fn test_update_nonexistent() {
        let mut pm = ProposalManager::new();
        let result = pm.update_phase(999, ProposalPhase::Approved);
        assert!(!result);
    }

    #[test]
    fn test_add_vote_nonexistent() {
        let mut pm = ProposalManager::new();
        let result = pm.add_vote(999, "for", 100);
        assert!(!result);
    }

    #[test]
    fn test_add_vote_invalid_type() {
        let mut pm = ProposalManager::new();
        let id = pm.create_proposal();
        let result = pm.add_vote(id, "invalid", 100);
        assert!(!result);
    }
}
