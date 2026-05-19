/// veX3 Governance Engine — Vote-Escrow tokenomics for decentralized protocol governance
/// Users lock X3 tokens (1-4 years) to gain voting power and direct liquidity mining rewards
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct VeX3Lock {
    pub lock_id: [u8; 32],
    pub user: [u8; 32],
    pub amount_locked: u64,
    pub lock_start_block: u64,
    pub unlock_block: u64, // When lock expires
    pub lock_duration_days: u32,
    pub voting_power: u64, // amount_locked * lock_duration_ratio
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct Proposal {
    pub proposal_id: [u8; 32],
    pub proposer: [u8; 32],
    pub title: Vec<u8>,
    pub description: Vec<u8>,
    pub proposal_type: u8, // 0=parameter, 1=treasury, 2=feature
    pub start_block: u64,
    pub end_block: u64,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub abstain_votes: u64,
    pub status: u8, // 0=voting, 1=passed, 2=rejected, 3=executed
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct Vote {
    pub vote_id: [u8; 32],
    pub proposal_id: [u8; 32],
    pub voter: [u8; 32],
    pub voting_power: u64,
    pub vote_direction: u8, // 0=yes, 1=no, 2=abstain
    pub vote_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LiquidityMiningAllocation {
    pub allocation_id: [u8; 32],
    pub pool_id: [u8; 32],
    pub votes_received: u64,
    pub total_votes_cast: u64,
    pub allocation_percentage: u32, // bps (5000 = 50%)
    pub proposed_by: [u8; 32],
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct EscrowInfo {
    pub escrow_id: [u8; 32],
    pub total_locked: u64,
    pub total_voting_power: u64,
    pub active_locks: u32,
    pub last_update_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct GovernanceReward {
    pub reward_id: [u8; 32],
    pub user: [u8; 32],
    pub voting_power: u64,
    pub reward_amount: u64,
    pub claim_block: u64,
}

pub struct VeX3GovernanceEngine;

impl VeX3GovernanceEngine {
    const MIN_LOCK_DURATION_DAYS: u32 = 365; // 1 year minimum
    const MAX_LOCK_DURATION_DAYS: u32 = 1_460; // 4 years maximum
    const MIN_LOCK_AMOUNT: u64 = 1;
    const MAX_LOCK_AMOUNT: u64 = u64::MAX / 2;
    const VOTING_POWER_SCALE: u64 = 10_000; // 1 year = 2500 = 25% of amount, 4 years = 10000 = 100%
    const BLOCKS_PER_DAY: u64 = 28_800; // 3-second blocks
    const QUORUM_PERCENTAGE_BPS: u32 = 4_000; // 40% quorum
    const PASSING_THRESHOLD_BPS: u32 = 5_000; // 50% + 1 of voting quorum

    /// Lock X3 tokens for voting power
    pub fn lock_x3_tokens(
        user: [u8; 32],
        amount: u64,
        lock_duration_days: u32,
        current_block: u64,
    ) -> Result<VeX3Lock, &'static str> {
        if !(Self::MIN_LOCK_AMOUNT..=Self::MAX_LOCK_AMOUNT).contains(&amount) {
            return Err("Lock amount out of range");
        }

        if !(Self::MIN_LOCK_DURATION_DAYS..=Self::MAX_LOCK_DURATION_DAYS)
            .contains(&lock_duration_days)
        {
            return Err("Lock duration outside allowed range (1-4 years)");
        }

        // Calculate voting power: amount * (duration / max_duration)
        let voting_power =
            (amount as u128 * lock_duration_days as u128 * Self::VOTING_POWER_SCALE as u128
                / Self::MAX_LOCK_DURATION_DAYS as u128) as u64;

        let unlock_block = current_block + (lock_duration_days as u64 * Self::BLOCKS_PER_DAY);

        let lock = VeX3Lock {
            lock_id: Self::derive_lock_id(user, amount, current_block),
            user,
            amount_locked: amount,
            lock_start_block: current_block,
            unlock_block,
            lock_duration_days,
            voting_power,
            is_active: true,
        };

        Ok(lock)
    }

    /// Unlock X3 tokens after lock period expires
    pub fn unlock_x3_tokens(lock: &mut VeX3Lock, current_block: u64) -> Result<u64, &'static str> {
        if !lock.is_active {
            return Err("Lock not active");
        }

        if current_block < lock.unlock_block {
            return Err("Tokens still locked");
        }

        lock.is_active = false;
        Ok(lock.amount_locked)
    }

    /// Early unlock with penalty (e.g., 50% penalty)
    pub fn early_unlock(
        lock: &mut VeX3Lock,
        penalty_percentage: u32, // bps
    ) -> Result<u64, &'static str> {
        if !lock.is_active {
            return Err("Lock not active");
        }

        let penalty = (lock.amount_locked as u128 * penalty_percentage as u128 / 10_000) as u64;
        let received = lock.amount_locked.saturating_sub(penalty);

        lock.is_active = false;
        Ok(received)
    }

    /// Create a governance proposal
    pub fn create_proposal(
        proposer: [u8; 32],
        title: Vec<u8>,
        description: Vec<u8>,
        proposal_type: u8,
        voting_duration_blocks: u64,
        current_block: u64,
    ) -> Result<Proposal, &'static str> {
        if title.is_empty() || description.is_empty() {
            return Err("Title and description required");
        }

        if voting_duration_blocks == 0 || voting_duration_blocks > 1_000_000 {
            return Err("Invalid voting duration");
        }

        let proposal = Proposal {
            proposal_id: Self::derive_proposal_id(proposer, title.clone(), current_block),
            proposer,
            title,
            description,
            proposal_type,
            start_block: current_block,
            end_block: current_block + voting_duration_blocks,
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
            status: 0, // voting
        };

        Ok(proposal)
    }

    /// Cast vote on proposal
    pub fn cast_vote(
        proposal: &mut Proposal,
        voter: [u8; 32],
        voting_power: u64,
        vote_direction: u8, // 0=yes, 1=no, 2=abstain
        current_block: u64,
    ) -> Result<Vote, &'static str> {
        if current_block > proposal.end_block {
            return Err("Voting period ended");
        }

        if vote_direction > 2 {
            return Err("Invalid vote direction");
        }

        // Tally vote
        match vote_direction {
            0 => proposal.yes_votes = proposal.yes_votes.saturating_add(voting_power),
            1 => proposal.no_votes = proposal.no_votes.saturating_add(voting_power),
            2 => proposal.abstain_votes = proposal.abstain_votes.saturating_add(voting_power),
            _ => return Err("Invalid vote"),
        }

        let vote = Vote {
            vote_id: Self::derive_vote_id(proposal.proposal_id, voter),
            proposal_id: proposal.proposal_id,
            voter,
            voting_power,
            vote_direction,
            vote_block: current_block,
        };

        Ok(vote)
    }

    /// Finalize proposal (check if passed)
    pub fn finalize_proposal(
        proposal: &mut Proposal,
        total_voting_power: u64,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        if current_block <= proposal.end_block {
            return Err("Voting still ongoing");
        }

        let votes_cast = proposal
            .yes_votes
            .saturating_add(proposal.no_votes)
            .saturating_add(proposal.abstain_votes);

        // Check quorum
        if votes_cast == 0 {
            proposal.status = 2; // rejected
            return Ok(false);
        }

        let quorum_required =
            (total_voting_power as u128 * Self::QUORUM_PERCENTAGE_BPS as u128 / 10_000) as u64;

        if votes_cast < quorum_required {
            proposal.status = 2; // rejected (quorum not met)
            return Ok(false);
        }

        // Check passing threshold
        let threshold = (votes_cast as u128 * Self::PASSING_THRESHOLD_BPS as u128 / 10_000) as u64;

        if proposal.yes_votes > threshold {
            proposal.status = 1; // passed
            Ok(true)
        } else {
            proposal.status = 2; // rejected
            Ok(false)
        }
    }

    /// Set liquidity mining allocation based on votes
    pub fn set_lm_allocation(
        pool_id: [u8; 32],
        votes_received: u64,
        total_votes_cast: u64,
    ) -> Result<LiquidityMiningAllocation, &'static str> {
        if total_votes_cast == 0 {
            return Err("No votes cast");
        }

        let allocation_percentage =
            ((votes_received as u128 * 10_000) / total_votes_cast as u128) as u32;

        let allocation = LiquidityMiningAllocation {
            allocation_id: Self::derive_allocation_id(pool_id),
            pool_id,
            votes_received,
            total_votes_cast,
            allocation_percentage,
            proposed_by: [0; 32],
        };

        Ok(allocation)
    }

    /// Calculate voting power from lock
    pub fn calculate_voting_power(amount_locked: u64, lock_duration_days: u32) -> u64 {
        (amount_locked as u128 * lock_duration_days as u128 * Self::VOTING_POWER_SCALE as u128
            / Self::MAX_LOCK_DURATION_DAYS as u128) as u64
    }

    /// Calculate time remaining on lock
    pub fn calculate_time_remaining(lock: &VeX3Lock, current_block: u64) -> u64 {
        lock.unlock_block.saturating_sub(current_block)
    }

    /// Calculate time remaining as days
    pub fn calculate_time_remaining_days(remaining_blocks: u64) -> u32 {
        (remaining_blocks / Self::BLOCKS_PER_DAY) as u32
    }

    /// Distribute governance rewards (optional feature)
    pub fn distribute_governance_reward(
        user: [u8; 32],
        voting_power: u64,
        total_voting_power: u64,
        reward_pool: u64,
    ) -> Result<GovernanceReward, &'static str> {
        if total_voting_power == 0 {
            return Err("No voting power");
        }

        let reward_amount =
            ((voting_power as u128 * reward_pool as u128) / total_voting_power as u128) as u64;

        let reward = GovernanceReward {
            reward_id: Self::derive_reward_id(user),
            user,
            voting_power,
            reward_amount,
            claim_block: 0,
        };

        Ok(reward)
    }

    /// Get escrow state summary
    pub fn get_escrow_state(
        total_locked: u64,
        total_voting_power: u64,
        active_locks: u32,
        current_block: u64,
    ) -> EscrowInfo {
        EscrowInfo {
            escrow_id: Self::derive_escrow_id(),
            total_locked,
            total_voting_power,
            active_locks,
            last_update_block: current_block,
        }
    }

    /// Derive lock ID
    fn derive_lock_id(user: [u8; 32], amount: u64, nonce: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in user.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let amount_bytes = amount.to_le_bytes();
        for (i, byte) in amount_bytes.iter().enumerate() {
            id[i + 16] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().take(8).enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive proposal ID
    fn derive_proposal_id(proposer: [u8; 32], title: Vec<u8>, nonce: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in proposer.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().enumerate().take(8) {
            id[i + 16] = *byte;
        }
        if let Some(first8) = title.first() {
            id[24] = *first8;
        }
        id
    }

    /// Derive vote ID
    fn derive_vote_id(proposal_id: [u8; 32], voter: [u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in proposal_id.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        for (i, byte) in voter.iter().enumerate().take(16) {
            id[i + 16] = *byte;
        }
        id
    }

    /// Derive allocation ID
    fn derive_allocation_id(pool_id: [u8; 32]) -> [u8; 32] {
        pool_id // Simple: use pool ID as allocation ID
    }

    /// Derive reward ID
    fn derive_reward_id(user: [u8; 32]) -> [u8; 32] {
        user // Simple: use user address as reward ID
    }

    /// Derive escrow ID (constant)
    fn derive_escrow_id() -> [u8; 32] {
        [1u8; 32] // Constant escrow ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_x3_tokens() {
        let lock = VeX3GovernanceEngine::lock_x3_tokens(
            [1; 32], 100_000, 365, // 1 year
            100,
        )
        .unwrap();

        assert_eq!(lock.amount_locked, 100_000);
        assert!(lock.voting_power > 0);
    }

    #[test]
    fn test_unlock_x3_tokens() {
        let mut lock = VeX3GovernanceEngine::lock_x3_tokens([1; 32], 100_000, 365, 100).unwrap();

        // Simulate unlock after lock period
        let unlock_block = lock.unlock_block + 1;
        let unlocked = VeX3GovernanceEngine::unlock_x3_tokens(&mut lock, unlock_block).unwrap();

        assert_eq!(unlocked, 100_000);
        assert!(!lock.is_active);
    }

    #[test]
    fn test_calculate_voting_power() {
        let power = VeX3GovernanceEngine::calculate_voting_power(100_000, 730); // 2 years

        assert!(power > 0);
        assert!(power < 100_000); // Scaled down
    }

    #[test]
    fn test_create_proposal() {
        let proposal = VeX3GovernanceEngine::create_proposal(
            [1; 32],
            b"Increase mining rewards".to_vec(),
            b"Proposal to increase LM pool rewards by 50%".to_vec(),
            0,
            100_000,
            100,
        )
        .unwrap();

        assert!(!proposal.title.is_empty());
        assert_eq!(proposal.status, 0); // voting
    }

    #[test]
    fn test_cast_vote() {
        let mut proposal = VeX3GovernanceEngine::create_proposal(
            [1; 32],
            b"Test".to_vec(),
            b"Test proposal".to_vec(),
            0,
            100_000,
            100,
        )
        .unwrap();

        let vote = VeX3GovernanceEngine::cast_vote(
            &mut proposal,
            [2; 32],
            50_000,
            0, // yes
            200,
        )
        .unwrap();

        assert_eq!(proposal.yes_votes, 50_000);
        assert_eq!(vote.vote_direction, 0);
    }

    #[test]
    fn test_finalize_proposal() {
        let mut proposal = VeX3GovernanceEngine::create_proposal(
            [1; 32],
            b"Test".to_vec(),
            b"Test".to_vec(),
            0,
            100,
            100,
        )
        .unwrap();

        // Add votes
        proposal.yes_votes = 100_000;
        proposal.no_votes = 50_000;

        let passed = VeX3GovernanceEngine::finalize_proposal(
            &mut proposal,
            500_000, // Total voting power
            200,
        )
        .unwrap();

        assert!(passed); // Yes votes should pass
    }

    #[test]
    fn test_set_lm_allocation() {
        let allocation =
            VeX3GovernanceEngine::set_lm_allocation([1; 32], 100_000, 500_000).unwrap();

        assert_eq!(allocation.allocation_percentage, 2_000); // 20%
    }

    #[test]
    fn test_calculate_time_remaining() {
        let lock = VeX3GovernanceEngine::lock_x3_tokens([1; 32], 100_000, 365, 100).unwrap();

        let remaining = VeX3GovernanceEngine::calculate_time_remaining(&lock, 200);

        assert!(remaining > 0);
    }

    #[test]
    fn test_distribute_governance_reward() {
        let reward =
            VeX3GovernanceEngine::distribute_governance_reward([1; 32], 50_000, 500_000, 100_000)
                .unwrap();

        assert!(reward.reward_amount > 0);
    }

    #[test]
    fn test_early_unlock_with_penalty() {
        let mut lock = VeX3GovernanceEngine::lock_x3_tokens([1; 32], 100_000, 365, 100).unwrap();

        let received = VeX3GovernanceEngine::early_unlock(&mut lock, 5_000).unwrap(); // 50% penalty

        assert!(received < 100_000);
    }
}
