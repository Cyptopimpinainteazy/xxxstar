/// Liquidity Mining Rewards — Proportional X3 token emissions to LP providers
/// Incentivizes liquidity provision with transparent reward distribution
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LiquidityMiningReward {
    pub pool_id: [u8; 32],
    pub reward_token: u128,
    pub rewards_per_block: u64,
    pub total_rewards_distributed: u64,
    pub is_active: bool,
    pub start_block: u64,
    pub end_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LPRewardAccumulator {
    pub lp: [u8; 32],
    pub pool_id: [u8; 32],
    pub accumulated_rewards: u64,
    pub last_reward_block: u64,
    pub pending_rewards: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct RewardSnapshot {
    pub reward_per_lp_token: u128,
    pub block: u64,
    pub timestamp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct EpochRewards {
    pub epoch: u32,
    pub total_rewards: u64,
    pub participants: u32,
    pub average_reward_per_lp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LPInfo {
    pub lp: [u8; 32],
    pub total_lp_tokens: u64,
    pub stake_timestamp: u64,
    pub is_active: bool,
}

pub struct LiquidityMiningEngine;

impl LiquidityMiningEngine {
    const MIN_REWARDS_PER_BLOCK: u64 = 1;
    const MAX_REWARDS_PER_BLOCK: u64 = 1_000_000_000_000; // 1 trillion max

    /// Create a new liquidity mining campaign for a pool
    pub fn create_mining_reward(
        pool_id: [u8; 32],
        reward_token: u128,
        rewards_per_block: u64,
        duration_blocks: u64,
        start_block: u64,
    ) -> Result<LiquidityMiningReward, &'static str> {
        if !(Self::MIN_REWARDS_PER_BLOCK..=Self::MAX_REWARDS_PER_BLOCK).contains(&rewards_per_block)
        {
            return Err("Rewards per block out of range");
        }

        if duration_blocks == 0 {
            return Err("Duration must be > 0");
        }

        let campaign = LiquidityMiningReward {
            pool_id,
            reward_token,
            rewards_per_block,
            total_rewards_distributed: 0,
            is_active: true,
            start_block,
            end_block: start_block + duration_blocks,
        };

        Ok(campaign)
    }

    /// Calculate accumulated rewards for an LP since last claim
    pub fn calculate_pending_rewards(
        lp_stake: u64,
        total_pool_lp_tokens: u64,
        campaign: &LiquidityMiningReward,
        current_block: u64,
    ) -> Result<u64, &'static str> {
        if !campaign.is_active || current_block < campaign.start_block {
            return Ok(0);
        }

        if total_pool_lp_tokens == 0 {
            return Err("No liquidity in pool");
        }

        let blocks_elapsed =
            sp_std::cmp::min(current_block, campaign.end_block) - campaign.start_block;
        let total_block_rewards = campaign
            .rewards_per_block
            .saturating_mul(blocks_elapsed as u64);

        // Proportional reward: (lp_stake / total_lp) * total_rewards
        let reward = ((lp_stake as u128)
            .saturating_mul(total_block_rewards as u128)
            .saturating_div(total_pool_lp_tokens as u128)) as u64;

        Ok(reward)
    }

    /// Claim accumulated rewards
    pub fn claim_rewards(
        accumulator: &mut LPRewardAccumulator,
        lp_stake: u64,
        total_pool_lp_tokens: u64,
        campaign: &mut LiquidityMiningReward,
        current_block: u64,
    ) -> Result<u64, &'static str> {
        if lp_stake == 0 {
            return Err("No LP tokens to claim on");
        }

        let pending = Self::calculate_pending_rewards(
            lp_stake,
            total_pool_lp_tokens,
            campaign,
            current_block,
        )?;

        accumulator.pending_rewards = 0;
        accumulator.accumulated_rewards = accumulator.accumulated_rewards.saturating_add(pending);
        accumulator.last_reward_block = current_block;

        campaign.total_rewards_distributed =
            campaign.total_rewards_distributed.saturating_add(pending);

        Ok(pending)
    }

    /// Stake LP tokens in mining pool
    pub fn stake_lp_tokens(
        lp: [u8; 32],
        lp_tokens: u64,
        current_block: u64,
    ) -> Result<LPInfo, &'static str> {
        if lp_tokens == 0 {
            return Err("Must stake at least 1 LP token");
        }

        let info = LPInfo {
            lp,
            total_lp_tokens: lp_tokens,
            stake_timestamp: current_block,
            is_active: true,
        };

        Ok(info)
    }

    /// Unstake LP tokens (withdraw from mining pool)
    pub fn unstake_lp_tokens(lp_info: &mut LPInfo, amount: u64) -> Result<u64, &'static str> {
        if amount > lp_info.total_lp_tokens {
            return Err("Cannot unstake more than staked");
        }

        lp_info.total_lp_tokens -= amount;

        if lp_info.total_lp_tokens == 0 {
            lp_info.is_active = false;
        }

        Ok(lp_info.total_lp_tokens)
    }

    /// Calculate reward tier based on staking duration
    pub fn calculate_reward_multiplier(stake_blocks: u64) -> u32 {
        // Base 100 = 1x, increases with time
        // 0-1000 blocks: 1x
        // 1000-10000 blocks: 1.25x
        // 10000+ blocks: 1.5x
        if stake_blocks >= 10_000 {
            150
        } else if stake_blocks >= 1_000 {
            125
        } else {
            100
        }
    }

    /// Calculate average APY from rewards
    pub fn calculate_apy(
        rewards_per_block: u64,
        total_lp_stake: u64,
        blocks_per_year: u64,
    ) -> Result<u64, &'static str> {
        if total_lp_stake == 0 {
            return Err("No stake");
        }

        let annual_rewards = rewards_per_block.saturating_mul(blocks_per_year);
        let apy = (annual_rewards as u128 * 10_000 / total_lp_stake as u128) as u64;

        Ok(apy)
    }

    /// Get epoch-based reward summary
    pub fn calculate_epoch_rewards(
        epoch: u32,
        total_rewards: u64,
        participant_count: u32,
    ) -> Result<EpochRewards, &'static str> {
        if participant_count == 0 {
            return Err("No participants");
        }

        let avg_per_participant = total_rewards / (participant_count as u64);

        Ok(EpochRewards {
            epoch,
            total_rewards,
            participants: participant_count,
            average_reward_per_lp: avg_per_participant,
        })
    }

    /// Update mining campaign status (end or modify)
    pub fn end_mining_campaign(
        campaign: &mut LiquidityMiningReward,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        if !campaign.is_active {
            return Err("Campaign not active");
        }

        campaign.is_active = false;
        campaign.end_block = current_block;

        Ok(true)
    }

    /// Boost rewards for specific LP (governance-controlled)
    pub fn apply_reward_boost(
        base_reward: u64,
        boost_percentage: u32,
    ) -> Result<u64, &'static str> {
        if boost_percentage > 1_000 {
            return Err("Boost cannot exceed 10x");
        }

        let boosted = base_reward.saturating_mul((100 + boost_percentage) as u64) / 100;
        Ok(boosted)
    }

    /// Harvest all rewards without unstaking
    pub fn harvest_rewards(
        accumulator: &mut LPRewardAccumulator,
        harvestable_amount: u64,
    ) -> Result<u64, &'static str> {
        if harvestable_amount == 0 {
            return Err("No rewards to harvest");
        }

        let claimed = accumulator
            .accumulated_rewards
            .saturating_add(harvestable_amount);
        accumulator.accumulated_rewards = 0;
        accumulator.pending_rewards = 0;

        Ok(claimed)
    }

    /// Calculate lock-in reward bonus (higher rewards for longer staking)
    pub fn calculate_lock_bonus(amount: u64, lock_blocks: u64) -> u64 {
        // Bonus: (lock_blocks / 50000) * amount, capped at 2x
        let bonus_factor = sp_std::cmp::min((lock_blocks / 50_000) + 100, 200);
        (amount * bonus_factor) / 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mining_reward() {
        let campaign =
            LiquidityMiningEngine::create_mining_reward([0; 32], 1, 100_000, 10_000, 0).unwrap();

        assert_eq!(campaign.rewards_per_block, 100_000);
        assert_eq!(campaign.end_block, 10_000);
        assert!(campaign.is_active);
    }

    #[test]
    fn test_create_invalid_rewards() {
        let result = LiquidityMiningEngine::create_mining_reward([0; 32], 1, 0, 10_000, 0);

        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_pending_rewards() {
        let campaign =
            LiquidityMiningEngine::create_mining_reward([0; 32], 1, 100_000, 10_000, 0).unwrap();

        let pending = LiquidityMiningEngine::calculate_pending_rewards(
            1_000_000,  // lp stake
            10_000_000, // total pool stake
            &campaign, 500, // 500 blocks elapsed
        )
        .unwrap();

        assert!(pending > 0);
    }

    #[test]
    fn test_claim_rewards() {
        let campaign =
            &mut LiquidityMiningEngine::create_mining_reward([0; 32], 1, 100_000, 10_000, 0)
                .unwrap();

        let mut accumulator = LPRewardAccumulator {
            lp: [1; 32],
            pool_id: [0; 32],
            accumulated_rewards: 0,
            last_reward_block: 0,
            pending_rewards: 0,
        };

        let claimed = LiquidityMiningEngine::claim_rewards(
            &mut accumulator,
            1_000_000,
            10_000_000,
            campaign,
            500,
        )
        .unwrap();

        assert!(claimed > 0);
        assert_eq!(accumulator.last_reward_block, 500);
    }

    #[test]
    fn test_stake_lp_tokens() {
        let info = LiquidityMiningEngine::stake_lp_tokens([1; 32], 1_000_000, 0).unwrap();

        assert_eq!(info.total_lp_tokens, 1_000_000);
        assert!(info.is_active);
    }

    #[test]
    fn test_unstake_lp_tokens() {
        let mut info = LiquidityMiningEngine::stake_lp_tokens([1; 32], 1_000_000, 0).unwrap();

        let remaining = LiquidityMiningEngine::unstake_lp_tokens(&mut info, 500_000).unwrap();
        assert_eq!(remaining, 500_000);
        assert!(info.is_active);
    }

    #[test]
    fn test_calculate_reward_multiplier() {
        let mult_0 = LiquidityMiningEngine::calculate_reward_multiplier(500);
        assert_eq!(mult_0, 100); // 1x

        let mult_1 = LiquidityMiningEngine::calculate_reward_multiplier(5_000);
        assert_eq!(mult_1, 125); // 1.25x

        let mult_2 = LiquidityMiningEngine::calculate_reward_multiplier(15_000);
        assert_eq!(mult_2, 150); // 1.5x
    }

    #[test]
    fn test_calculate_apy() {
        let apy = LiquidityMiningEngine::calculate_apy(
            100_000, 10_000_000, 2_628_000, // blocks per year
        )
        .unwrap();

        assert!(apy > 0);
    }

    #[test]
    fn test_calculate_epoch_rewards() {
        let epoch = LiquidityMiningEngine::calculate_epoch_rewards(1, 10_000_000, 100).unwrap();

        assert_eq!(epoch.average_reward_per_lp, 100_000);
    }

    #[test]
    fn test_end_mining_campaign() {
        let mut campaign =
            LiquidityMiningEngine::create_mining_reward([0; 32], 1, 100_000, 10_000, 0).unwrap();

        LiquidityMiningEngine::end_mining_campaign(&mut campaign, 5_000).unwrap();
        assert!(!campaign.is_active);
    }

    #[test]
    fn test_apply_reward_boost() {
        let boosted = LiquidityMiningEngine::apply_reward_boost(100_000, 50).unwrap();
        assert_eq!(boosted, 150_000); // 1.5x
    }

    #[test]
    fn test_harvest_rewards() {
        let mut accumulator = LPRewardAccumulator {
            lp: [1; 32],
            pool_id: [0; 32],
            accumulated_rewards: 100_000,
            last_reward_block: 0,
            pending_rewards: 0,
        };

        let claimed = LiquidityMiningEngine::harvest_rewards(&mut accumulator, 50_000).unwrap();
        assert_eq!(claimed, 150_000);
        assert_eq!(accumulator.accumulated_rewards, 0);
    }

    #[test]
    fn test_calculate_lock_bonus() {
        let bonus = LiquidityMiningEngine::calculate_lock_bonus(100_000, 100_000);
        assert!(bonus > 100_000);
    }
}
