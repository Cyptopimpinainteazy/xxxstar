use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeStatus {
    Open,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofDispute {
    pub proof_id: [u8; 32],
    pub started_at: u64,
    pub close_after: u64,
    pub votes_accept: u64,
    pub votes_reject: u64,
    pub status: DisputeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisputeError {
    NotOpen,
    WindowNotElapsed,
    DuplicateVoter,
    InvalidWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeResult {
    pub status: DisputeStatus,
    pub votes_accept: u64,
    pub votes_reject: u64,
}

pub struct DisputeTracker {
    dispute: ProofDispute,
    voters: HashSet<String>,
}

impl DisputeTracker {
    pub fn new(
        proof_id: [u8; 32],
        started_at: u64,
        dispute_window: u64,
    ) -> Result<Self, DisputeError> {
        if dispute_window == 0 {
            return Err(DisputeError::InvalidWindow);
        }

        Ok(Self {
            dispute: ProofDispute {
                proof_id,
                started_at,
                close_after: started_at.saturating_add(dispute_window),
                votes_accept: 0,
                votes_reject: 0,
                status: DisputeStatus::Open,
            },
            voters: HashSet::new(),
        })
    }

    pub fn state(&self) -> &ProofDispute {
        &self.dispute
    }

    pub fn vote(
        &mut self,
        voter: impl Into<String>,
        accept_dispute: bool,
    ) -> Result<(), DisputeError> {
        if self.dispute.status != DisputeStatus::Open {
            return Err(DisputeError::NotOpen);
        }

        let voter = voter.into();
        if !self.voters.insert(voter) {
            return Err(DisputeError::DuplicateVoter);
        }

        if accept_dispute {
            self.dispute.votes_accept = self.dispute.votes_accept.saturating_add(1);
        } else {
            self.dispute.votes_reject = self.dispute.votes_reject.saturating_add(1);
        }

        Ok(())
    }

    pub fn close(
        &mut self,
        now: u64,
        accept_threshold: u64,
    ) -> Result<DisputeResult, DisputeError> {
        if self.dispute.status != DisputeStatus::Open {
            return Err(DisputeError::NotOpen);
        }
        if now < self.dispute.close_after {
            return Err(DisputeError::WindowNotElapsed);
        }

        self.dispute.status = if self.dispute.votes_accept >= accept_threshold {
            DisputeStatus::Accepted
        } else {
            DisputeStatus::Rejected
        };

        Ok(DisputeResult {
            status: self.dispute.status,
            votes_accept: self.dispute.votes_accept,
            votes_reject: self.dispute.votes_reject,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_window() {
        let result = DisputeTracker::new([1; 32], 100, 0);
        assert!(matches!(result, Err(DisputeError::InvalidWindow)));
    }

    #[test]
    fn duplicate_voter_is_rejected() {
        let mut tracker = DisputeTracker::new([2; 32], 10, 5).expect("must create tracker");
        tracker.vote("alice", true).expect("first vote succeeds");
        let second = tracker.vote("alice", false);
        assert!(matches!(second, Err(DisputeError::DuplicateVoter)));
    }

    #[test]
    fn cannot_close_before_window_elapsed() {
        let mut tracker = DisputeTracker::new([3; 32], 10, 20).expect("must create tracker");
        tracker.vote("alice", true).expect("vote recorded");
        let result = tracker.close(15, 1);
        assert!(matches!(result, Err(DisputeError::WindowNotElapsed)));
    }

    #[test]
    fn accepts_dispute_when_threshold_met() {
        let mut tracker = DisputeTracker::new([4; 32], 10, 2).expect("must create tracker");
        tracker.vote("alice", true).expect("vote recorded");
        tracker.vote("bob", true).expect("vote recorded");
        let result = tracker.close(12, 2).expect("must close");

        assert_eq!(result.status, DisputeStatus::Accepted);
        assert_eq!(tracker.state().status, DisputeStatus::Accepted);
    }

    #[test]
    fn rejects_dispute_when_threshold_not_met() {
        let mut tracker = DisputeTracker::new([5; 32], 10, 2).expect("must create tracker");
        tracker.vote("alice", false).expect("vote recorded");
        let result = tracker.close(12, 1).expect("must close");

        assert_eq!(result.status, DisputeStatus::Rejected);
        assert_eq!(result.votes_reject, 1);
    }
}
