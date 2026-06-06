//! Stake delegation with auto-compounding rewards
//!
//! Validators and nominator pools auto-compound staking rewards every epoch
//! without requiring explicit re-stake transactions. Dramatically improves
//! long-term staker APY vs manual re-staking.

use std::collections::HashMap;
use sp_runtime::Permill;

/// Stake delegation configuration
#[derive(Clone, Debug)]
pub struct DelegationConfig {
    /// Auto-compound enabled for this delegation
    pub auto_compound: bool,
    /// Manual reward claim threshold (if not auto-compounding)
    pub claim_threshold: u128,
    /// Timestamp of last manual claim
    pub last_claimed: u64,
}

impl DelegationConfig {
    pub fn new_with_auto_compound() -> Self {
        Self {
            auto_compound: true,
            claim_threshold: 0,
            last_claimed: 0,
        }
    }

    pub fn new_manual() -> Self {
        Self {
            auto_compound: false,
            claim_threshold: 1_000,
            last_claimed: 0,
        }
    }
}

/// Per-nominator stake tracking with compounding
#[derive(Clone, Debug)]
pub struct NominatorStake {
    /// Nominator account ID
    pub nominator: String,
    /// Current staked amount
    pub staked: u128,
    /// Accumulated unclaimed rewards
    pub unclaimed_rewards: u128,
    /// Delegation configuration
    pub config: DelegationConfig,
    /// Compound events: (epoch, amount_added)
    pub compound_history: Vec<(u32, u128)>,
}

impl NominatorStake {
    /// Create new nominator with auto-compound enabled
    pub fn new(nominator: String, initial_stake: u128) -> Self {
        Self {
            nominator,
            staked: initial_stake,
            unclaimed_rewards: 0,
            config: DelegationConfig::new_with_auto_compound(),
            compound_history: Vec::new(),
        }
    }

    /// Record reward earned this epoch
    pub fn earn_reward(&mut self, reward: u128) {
        self.unclaimed_rewards = self.unclaimed_rewards.saturating_add(reward);
    }

    /// Auto-compound at epoch boundary (if enabled)
    pub fn auto_compound(&mut self, epoch: u32) -> u128 {
        if !self.config.auto_compound {
            return 0;
        }

        let to_compound = self.unclaimed_rewards;
        if to_compound > 0 {
            self.staked = self.staked.saturating_add(to_compound);
            self.unclaimed_rewards = 0;
            self.compound_history.push((epoch, to_compound));
            to_compound
        } else {
            0
        }
    }

    /// Manual claim of rewards
    pub fn manual_claim(&mut self) -> u128 {
        let amount = self.unclaimed_rewards;
        self.unclaimed_rewards = 0;
        amount
    }

    /// Calculate effective stake with compound factor
    pub fn effective_stake(&self, apy_bps: u32) -> u128 {
        // Simple: compound_history.len() represents number of compounding periods
        let periods = self.compound_history.len() as u32;
        let rate = Permill::from_parts(apy_bps / 52); // weekly compound approx

        // Simplified: (1 + r)^n compound
        let mut result = self.staked;
        for _ in 0..periods {
            let interest = (result as u64).saturating_mul(rate.deconstruct() as u64) / 1_000_000;
            result = result.saturating_add(interest as u128);
        }
        result
    }
}

/// Validator pool with recursive compounding
#[derive(Clone, Debug)]
pub struct ValidatorPool {
    /// Validator account ID
    pub validator: String,
    /// Total staked (nominator sum)
    pub total_staked: u128,
    /// Per-nominator delegations
    pub delegations: HashMap<String, NominatorStake>,
    /// Pool's own stake
    pub pool_stake: u128,
    /// Unclaimed pool rewards
    pub pool_rewards: u128,
}

impl ValidatorPool {
    /// Create validator pool
    pub fn new(validator: String, pool_stake: u128) -> Self {
        Self {
            validator,
            total_staked: pool_stake,
            delegations: HashMap::new(),
            pool_stake,
            pool_rewards: 0,
        }
    }

    /// Add nominator delegation
    pub fn add_delegation(&mut self, nominator: String, amount: u128) {
        let stake = NominatorStake::new(nominator.clone(), amount);
        self.delegations.insert(nominator, stake);
        self.total_staked = self.total_staked.saturating_add(amount);
    }

    /// Distribute epoch rewards proportionally
    pub fn distribute_epoch_rewards(&mut self, total_reward: u128, epoch: u32) {
        if self.total_staked == 0 {
            return;
        }

        // Pool takes a cut (commission already applied at higher level)
        let pool_reward = (total_reward * 5) / 100; // 5% pool fee
        self.pool_rewards = self.pool_rewards.saturating_add(pool_reward);

        // Remaining distributed to delegators by stake proportion
        let nominator_reward = total_reward.saturating_sub(pool_reward);

        for delegation in self.delegations.values_mut() {
            let proportion = (delegation.staked as u128 * 1_000_000) / self.total_staked;
            let reward = (nominator_reward as u64).saturating_mul(proportion as u64) / 1_000_000;
            delegation.earn_reward(reward as u128);
            delegation.auto_compound(epoch);
        }

        // Pool auto-compounds too
        self.pool_stake = self.pool_stake.saturating_add(self.pool_rewards);
        self.pool_rewards = 0;
    }

    /// Get total pool effective value (with compounding effects)
    pub fn total_effective_stake(&self) -> u128 {
        let mut total = self.pool_stake;
        for delegation in self.delegations.values() {
            total = total.saturating_add(delegation.staked);
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nominator_auto_compound() {
        let mut nominator = NominatorStake::new("alice".to_string(), 1000);

        nominator.earn_reward(100);
        let compounded = nominator.auto_compound(1);

        assert_eq!(compounded, 100);
        assert_eq!(nominator.staked, 1100);
        assert_eq!(nominator.unclaimed_rewards, 0);
    }

    #[test]
    fn test_nominator_manual_claim() {
        let mut nominator = NominatorStake::new("bob".to_string(), 1000);

        nominator.config.auto_compound = false;
        nominator.earn_reward(150);

        let claimed = nominator.manual_claim();

        assert_eq!(claimed, 150);
        assert_eq!(nominator.unclaimed_rewards, 0);
    }

    #[test]
    fn test_validator_pool_proportional_distribution() {
        let mut pool = ValidatorPool::new("validator".to_string(), 1000);
        pool.add_delegation("alice".to_string(), 1000);
        pool.add_delegation("bob".to_string(), 2000);

        // 3000 total, distribute 300 reward
        pool.distribute_epoch_rewards(300, 1);

        // Pool takes 5% = 15, leaving 285
        // Alice gets 1000/3000 = 33.3% of 285 ≈ 95
        // Bob gets 2000/3000 = 66.6% of 285 ≈ 190

        let alice_stake = pool.delegations.get("alice").unwrap().staked;
        let bob_stake = pool.delegations.get("bob").unwrap().staked;

        assert!(alice_stake > 1000); // Got compound
        assert!(bob_stake > 2000); // Got compound
    }

    #[test]
    fn test_pool_effective_stake() {
        let mut pool = ValidatorPool::new("validator".to_string(), 1000);
        pool.add_delegation("alice".to_string(), 500);

        assert_eq!(pool.total_effective_stake(), 1500);
    }

    #[test]
    fn test_compound_history_tracking() {
        let mut nominator = NominatorStake::new("charlie".to_string(), 1000);

        for epoch in 0..3 {
            nominator.earn_reward(50);
            nominator.auto_compound(epoch);
        }

        assert_eq!(nominator.compound_history.len(), 3);
        assert!(nominator.staked > 1150); // All rewards compounded
    }

    #[test]
    fn test_zero_stake_safety() {
        let mut pool = ValidatorPool::new("validator".to_string(), 0);
        pool.distribute_epoch_rewards(100, 1); // Should not panic
        assert_eq!(pool.total_staked, 0);
    }

    #[test]
    fn test_delegator_stays_in_pool_after_compound() {
        let mut pool = ValidatorPool::new("validator".to_string(), 500);
        pool.add_delegation("dave".to_string(), 500);

        pool.distribute_epoch_rewards(100, 1);

        // Dave still in pool
        assert!(pool.delegations.contains_key("dave"));
        assert!(pool.delegations.get("dave").unwrap().staked > 500);
    }
}
