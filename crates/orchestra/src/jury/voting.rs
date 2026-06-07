//! Anonymous voting — commit-reveal scheme enforcing Commandment V.
//!
//! Voting is binary: Yes / No.
//! Votes are blinded via commitment hashes until the reveal phase.
//! Only the final outcome is visible; individual votes are never exposed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A binary vote — Yes or No.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Yes,
    No,
}

impl std::fmt::Display for Vote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vote::Yes => write!(f, "Yes"),
            Vote::No => write!(f, "No"),
        }
    }
}

/// A committed vote — hash of (vote || nonce || identity_hash).
/// The actual vote is hidden until reveal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommittedVote {
    /// Voter's anonymous slot (not the actual agent ID).
    pub slot: u32,
    /// Blake3 commitment hash.
    pub commitment: [u8; 32],
    /// When the commitment was submitted.
    pub committed_at: DateTime<Utc>,
}

/// A revealed vote — produced during the (optional) reveal phase.
/// In standard operation, votes are NEVER revealed (anonymity preserved).
/// Reveal only happens during scrap yard audit of misaligned outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealedVote {
    /// The anonymous slot.
    pub slot: u32,
    /// The actual vote.
    pub vote: Vote,
    /// The nonce used in commitment.
    pub nonce: [u8; 16],
}

impl RevealedVote {
    /// Verify that this revealed vote matches a commitment.
    pub fn verify(&self, commitment: &[u8; 32], identity_hash: &[u8; 32]) -> bool {
        let mut data = Vec::new();
        data.push(match self.vote {
            Vote::Yes => 1u8,
            Vote::No => 0u8,
        });
        data.extend_from_slice(&self.nonce);
        data.extend_from_slice(identity_hash);
        let computed = blake3::hash(&data);
        computed.as_bytes() == commitment
    }
}

/// The ballot box for a single task within a jury session.
/// Collects commitments and (optionally) reveals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallotBox {
    /// Task ID being voted on.
    pub task_id: String,
    /// Collected commitments (indexed by anonymous slot).
    pub commitments: Vec<CommittedVote>,
    /// Expected number of voters.
    pub expected_voters: u32,
    /// Whether the commitment phase is closed.
    pub commitment_closed: bool,
    /// Revealed votes (only populated during audit; empty in normal operation).
    pub reveals: Vec<RevealedVote>,
    /// Final tally (populated after commitment phase closes).
    pub tally: Option<Tally>,
}

/// Vote tally — only the aggregate outcome, not individual votes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tally {
    /// Number of Yes votes.
    pub yes_count: u32,
    /// Number of No votes.
    pub no_count: u32,
    /// Total votes cast.
    pub total_cast: u32,
    /// Whether the task is approved (majority Yes).
    pub approved: bool,
}

impl BallotBox {
    /// Create a new ballot box for a task.
    pub fn new(task_id: String, expected_voters: u32) -> Self {
        Self {
            task_id,
            commitments: Vec::new(),
            expected_voters,
            commitment_closed: false,
            reveals: Vec::new(),
            tally: None,
        }
    }

    /// Submit a commitment (anonymous vote).
    pub fn submit_commitment(&mut self, commitment: CommittedVote) -> Result<(), VotingError> {
        if self.commitment_closed {
            return Err(VotingError::CommitmentPhaseClosed);
        }

        // Check for duplicate slot
        if self.commitments.iter().any(|c| c.slot == commitment.slot) {
            return Err(VotingError::DuplicateVote(commitment.slot));
        }

        self.commitments.push(commitment);

        // Auto-close when all expected voters have committed
        if self.commitments.len() as u32 >= self.expected_voters {
            self.commitment_closed = true;
        }

        Ok(())
    }

    /// Close the commitment phase early (e.g., timeout).
    pub fn close_commitments(&mut self) {
        self.commitment_closed = true;
    }

    /// Tally votes from revealed votes (audit-only path).
    /// In normal operation, we use `tally_from_sealed` instead.
    pub fn tally_from_reveals(&mut self) -> Option<Tally> {
        if self.reveals.is_empty() {
            return None;
        }

        let yes_count = self.reveals.iter().filter(|r| r.vote == Vote::Yes).count() as u32;
        let no_count = self.reveals.iter().filter(|r| r.vote == Vote::No).count() as u32;
        let total_cast = yes_count + no_count;

        let tally = Tally {
            yes_count,
            no_count,
            total_cast,
            approved: yes_count > no_count, // simple majority
        };

        self.tally = Some(tally);
        Some(tally)
    }

    /// Tally from sealed commitments — each commitment is treated as a
    /// sealed envelope. The oracle (off-chain execution environment) provides
    /// the aggregate counts without revealing individual votes.
    ///
    /// This is the NORMAL path — individual votes are never exposed.
    pub fn tally_from_sealed(&mut self, yes_count: u32, no_count: u32) -> Tally {
        let total_cast = yes_count + no_count;
        let tally = Tally {
            yes_count,
            no_count,
            total_cast,
            approved: yes_count > no_count,
        };
        self.tally = Some(tally);
        tally
    }

    /// Check if voting is complete.
    pub fn is_complete(&self) -> bool {
        self.tally.is_some()
    }

    /// Get the outcome.
    pub fn outcome(&self) -> Option<bool> {
        self.tally.map(|t| t.approved)
    }
}

/// Voting errors.
#[derive(Debug, thiserror::Error)]
pub enum VotingError {
    #[error("Commitment phase is closed")]
    CommitmentPhaseClosed,
    #[error("Duplicate vote from slot {0}")]
    DuplicateVote(u32),
    #[error("Invalid commitment reveal")]
    InvalidReveal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ballot_box_collects_commitments() {
        let mut ballot = BallotBox::new("task-001".into(), 3);

        ballot
            .submit_commitment(CommittedVote {
                slot: 0,
                commitment: [1u8; 32],
                committed_at: Utc::now(),
            })
            .unwrap();

        ballot
            .submit_commitment(CommittedVote {
                slot: 1,
                commitment: [2u8; 32],
                committed_at: Utc::now(),
            })
            .unwrap();

        assert_eq!(ballot.commitments.len(), 2);
        assert!(!ballot.commitment_closed);
    }

    #[test]
    fn auto_close_on_full_votes() {
        let mut ballot = BallotBox::new("task-001".into(), 2);

        ballot
            .submit_commitment(CommittedVote {
                slot: 0,
                commitment: [1u8; 32],
                committed_at: Utc::now(),
            })
            .unwrap();
        ballot
            .submit_commitment(CommittedVote {
                slot: 1,
                commitment: [2u8; 32],
                committed_at: Utc::now(),
            })
            .unwrap();

        assert!(ballot.commitment_closed);
    }

    #[test]
    fn duplicate_vote_rejected() {
        let mut ballot = BallotBox::new("task-001".into(), 3);

        ballot
            .submit_commitment(CommittedVote {
                slot: 0,
                commitment: [1u8; 32],
                committed_at: Utc::now(),
            })
            .unwrap();

        let result = ballot.submit_commitment(CommittedVote {
            slot: 0,
            commitment: [2u8; 32],
            committed_at: Utc::now(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn sealed_tally_majority_rule() {
        let mut ballot = BallotBox::new("task-001".into(), 5);
        ballot.close_commitments();

        let tally = ballot.tally_from_sealed(3, 2);
        assert!(tally.approved);
        assert_eq!(tally.yes_count, 3);
        assert_eq!(tally.no_count, 2);
    }

    #[test]
    fn sealed_tally_rejection() {
        let mut ballot = BallotBox::new("task-001".into(), 5);
        ballot.close_commitments();

        let tally = ballot.tally_from_sealed(1, 4);
        assert!(!tally.approved);
    }

    #[test]
    fn revealed_vote_verification() {
        let identity_hash = [42u8; 32];
        let nonce: [u8; 16] = [7u8; 16];
        let vote = Vote::Yes;

        // Compute expected commitment
        let mut data = Vec::new();
        data.push(1u8); // Yes
        data.extend_from_slice(&nonce);
        data.extend_from_slice(&identity_hash);
        let commitment = *blake3::hash(&data).as_bytes();

        let revealed = RevealedVote {
            slot: 0,
            vote,
            nonce,
        };

        assert!(revealed.verify(&commitment, &identity_hash));

        // Wrong identity should fail
        let wrong_identity = [99u8; 32];
        assert!(!revealed.verify(&commitment, &wrong_identity));
    }
}
