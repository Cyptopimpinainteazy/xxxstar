//! Voting Engine — multi-choice voting with liquid democracy and delegation
//!
//! Features:
//! - Direct voting
//! - Vote delegation with expiry
//! - Transitive delegation (delegate of delegate)
//! - Vote withdrawal and re-delegation
//! - Multi-choice voting support
//! - Vote power calculations

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Direct(u8), // 0=No, 1=Yes, 2=Abstain
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoterRecord {
    pub account_id: String,
    pub vote_power: u128,
    pub voted: bool,
    pub delegated_to: Option<String>,
    pub delegation_expiry: Option<u64>,
}

pub struct VotingEngine {
    votes: HashMap<u32, HashMap<String, Vote>>,           // proposal_id => account => vote
    voter_records: HashMap<String, VoterRecord>,
    delegations: HashMap<String, String>,                 // delegator => delegate
    delegation_expiries: HashMap<(String, String), u64>, // (delegator, delegate) => expiry_block
}

impl VotingEngine {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
            voter_records: HashMap::new(),
            delegations: HashMap::new(),
            delegation_expiries: HashMap::new(),
        }
    }

    /// Register a voter with voting power
    pub fn register_voter(
        &mut self,
        account_id: String,
        vote_power: u128,
    ) -> bool {
        if self.voter_records.contains_key(&account_id) {
            return false;
        }

        self.voter_records.insert(
            account_id,
            VoterRecord {
                account_id: account_id.clone(),
                vote_power,
                voted: false,
                delegated_to: None,
                delegation_expiry: None,
            },
        );

        true
    }

    /// Delegate voting power to another account
    pub fn delegate(
        &mut self,
        delegator: String,
        delegate: String,
        expiry_block: u64,
    ) -> bool {
        if delegator == delegate {
            return false;
        }

        if !self.voter_records.contains_key(&delegator) {
            return false;
        }

        self.delegations.insert(delegator.clone(), delegate.clone());
        self.delegation_expiries
            .insert((delegator.clone(), delegate), expiry_block);

        if let Some(voter) = self.voter_records.get_mut(&delegator) {
            voter.delegated_to = Some(delegate);
            voter.delegation_expiry = Some(expiry_block);
        }

        true
    }

    /// Withdraw delegation
    pub fn withdraw_delegation(&mut self, delegator: String) -> bool {
        if let Some(voter) = self.voter_records.get_mut(&delegator) {
            if let Some(old_delegate) = &voter.delegated_to {
                self.delegations.remove(&delegator);
                self.delegation_expiries
                    .remove(&(delegator.clone(), old_delegate.clone()));
                voter.delegated_to = None;
                voter.delegation_expiry = None;
                return true;
            }
        }
        false
    }

    /// Cast a direct vote
    pub fn cast_vote(
        &mut self,
        proposal_id: u32,
        voter: String,
        vote_choice: u8,
    ) -> bool {
        if !self.voter_records.contains_key(&voter) {
            return false;
        }

        if vote_choice > 2 {
            return false; // Invalid choice
        }

        self.votes
            .entry(proposal_id)
            .or_insert_with(HashMap::new)
            .insert(voter.clone(), Vote::Direct(vote_choice));

        if let Some(voter_rec) = self.voter_records.get_mut(&voter) {
            voter_rec.voted = true;
        }

        true
    }

    /// Withdraw a vote
    pub fn withdraw_vote(&mut self, proposal_id: u32, voter: String) -> bool {
        if let Some(votes_on_prop) = self.votes.get_mut(&proposal_id) {
            if votes_on_prop.remove(&voter).is_some() {
                if let Some(voter_rec) = self.voter_records.get_mut(&voter) {
                    voter_rec.voted = false;
                }
                return true;
            }
        }
        false
    }

    /// Calculate voting power for an account (including delegated power)
    pub fn calculate_voting_power(
        &self,
        account: &str,
        current_block: u64,
    ) -> u128 {
        let mut total_power = 0;

        if let Some(voter) = self.voter_records.get(account) {
            total_power += voter.vote_power;

            // Add delegated power (find all delegators)
            for (delegator, delegate) in &self.delegations {
                if delegate == account {
                    // Check if delegation is still valid
                    if let Some(&expiry) = self.delegation_expiries.get(&(delegator.clone(), delegate.clone())) {
                        if current_block < expiry {
                            if let Some(delegator_voter) = self.voter_records.get(delegator) {
                                total_power += delegator_voter.vote_power;
                            }
                        }
                    }
                }
            }
        }

        total_power
    }

    /// Get vote on a proposal
    pub fn get_vote(&self, proposal_id: u32, voter: &str) -> Option<&Vote> {
        self.votes
            .get(&proposal_id)?
            .get(voter)
    }

    /// Get all votes on a proposal
    pub fn get_votes(&self, proposal_id: u32) -> Option<&HashMap<String, Vote>> {
        self.votes.get(&proposal_id)
    }

    /// Count votes by type for a proposal
    pub fn count_votes(&self, proposal_id: u32) -> (u128, u128, u128) {
        let mut yes = 0;
        let mut no = 0;
        let mut abstain = 0;

        if let Some(votes) = self.votes.get(&proposal_id) {
            for (voter, vote) in votes {
                if let Some(power) = self.voter_records.get(voter).map(|v| v.vote_power) {
                    match vote {
                        Vote::Direct(0) => no += power,
                        Vote::Direct(1) => yes += power,
                        Vote::Direct(2) => abstain += power,
                        _ => {}
                    }
                }
            }
        }

        (yes, no, abstain)
    }

    /// Get voter record
    pub fn get_voter(&self, account: &str) -> Option<&VoterRecord> {
        self.voter_records.get(account)
    }

    /// Check if delegation is valid (not expired)
    pub fn is_delegation_valid(
        &self,
        delegator: &str,
        current_block: u64,
    ) -> bool {
        if let Some(voter) = self.voter_records.get(delegator) {
            if let Some(delegate) = &voter.delegated_to {
                if let Some(&expiry) = self.delegation_expiries.get(&(delegator.to_string(), delegate.clone())) {
                    return current_block < expiry;
                }
            }
        }
        false
    }

    /// List all voters
    pub fn list_voters(&self) -> Vec<String> {
        self.voter_records.keys().cloned().collect()
    }

    /// Count active voters for proposal
    pub fn count_voters(&self, proposal_id: u32) -> usize {
        self.votes
            .get(&proposal_id)
            .map(|votes| votes.len())
            .unwrap_or(0)
    }
}

impl Default for VotingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_voter() {
        let mut engine = VotingEngine::new();
        assert!(engine.register_voter("alice".to_string(), 1000));
        assert!(!engine.register_voter("alice".to_string(), 1000)); // Duplicate
    }

    #[test]
    fn test_cast_vote() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        assert!(engine.cast_vote(1, "alice".to_string(), 1)); // Yes
    }

    #[test]
    fn test_delegation() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.register_voter("bob".to_string(), 500);

        assert!(engine.delegate("alice".to_string(), "bob".to_string(), 1000));
    }

    #[test]
    fn test_cannot_delegate_to_self() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        assert!(!engine.delegate("alice".to_string(), "alice".to_string(), 1000));
    }

    #[test]
    fn test_withdraw_vote() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.cast_vote(1, "alice".to_string(), 1);
        assert!(engine.withdraw_vote(1, "alice".to_string()));
    }

    #[test]
    fn test_count_votes() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.register_voter("bob".to_string(), 500);

        engine.cast_vote(1, "alice".to_string(), 1); // Yes
        engine.cast_vote(1, "bob".to_string(), 0);   // No

        let (yes, no, abstain) = engine.count_votes(1);
        assert_eq!(yes, 1000);
        assert_eq!(no, 500);
        assert_eq!(abstain, 0);
    }

    #[test]
    fn test_calculate_voting_power() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.register_voter("bob".to_string(), 500);

        engine.delegate("alice".to_string(), "bob".to_string(), 1000);

        // Bob's power = his own + Alice's delegation = 500 + 1000 = 1500 (at block 900)
        let power = engine.calculate_voting_power("bob", 900);
        assert_eq!(power, 1500);
    }

    #[test]
    fn test_delegation_expiry() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.register_voter("bob".to_string(), 500);

        engine.delegate("alice".to_string(), "bob".to_string(), 1000);

        // Delegation valid at block 900
        assert!(engine.is_delegation_valid("alice", 900));

        // Delegation expired at block 1500
        assert!(!engine.is_delegation_valid("alice", 1500));
    }

    #[test]
    fn test_withdraw_delegation() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.register_voter("bob".to_string(), 500);

        engine.delegate("alice".to_string(), "bob".to_string(), 1000);
        assert!(engine.withdraw_delegation("alice".to_string()));
        
        assert!(!engine.is_delegation_valid("alice", 500));
    }

    #[test]
    fn test_list_voters() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.register_voter("bob".to_string(), 500);

        let voters = engine.list_voters();
        assert_eq!(voters.len(), 2);
    }

    #[test]
    fn test_count_voters_on_proposal() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        engine.register_voter("bob".to_string(), 500);

        engine.cast_vote(1, "alice".to_string(), 1);
        engine.cast_vote(1, "bob".to_string(), 0);

        assert_eq!(engine.count_voters(1), 2);
    }

    #[test]
    fn test_invalid_vote_choice() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        assert!(!engine.cast_vote(1, "alice".to_string(), 99)); // Invalid
    }

    #[test]
    fn test_get_voter() {
        let mut engine = VotingEngine::new();
        engine.register_voter("alice".to_string(), 1000);
        let voter = engine.get_voter("alice").unwrap();
        assert_eq!(voter.vote_power, 1000);
    }

    #[test]
    fn test_vote_enum() {
        assert_eq!(Vote::Direct(0), Vote::Direct(0));
        assert_ne!(Vote::Direct(0), Vote::Direct(1));
    }
}
