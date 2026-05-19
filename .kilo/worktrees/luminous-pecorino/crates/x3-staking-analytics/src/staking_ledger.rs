//! Staking Ledger — Core position tracking
//! 
//! Maintains comprehensive staking positions with unbonding support,
//! reward accumulation, and lifecycle management.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{Result, StakingError};

/// Staking position status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PositionStatus {
    /// Currently active and earning rewards
    Active,
    /// Locked for minimum staking period
    Locked,
    /// Unbonding in progress
    Unbonding,
    /// All funds claimed, position closed
    Claimed,
    /// Position exited (no balance remaining)
    Exited,
}

/// Unbonding phase tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondingPhase {
    pub started_era: u32,
    pub unlock_era: u32,
    pub amount: u128,
    pub claimed: bool,
}

/// Core staking position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingPosition {
    pub id: String,
    pub delegator: String,
    pub validator: String,
    pub active_balance: u128,
    pub locked_until: DateTime<Utc>,
    pub unbonding_phases: Vec<UnbondingPhase>,
    pub accumulated_rewards: u128,
    pub claimed_rewards: u128,
    pub status: PositionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub commission_rate: f64,
    pub unstake_fee: f64, // percentage (0.5 = 0.5%)
}

impl StakingPosition {
    /// Total balance including active and locked
    pub fn total_balance(&self) -> u128 {
        let unbonding_total: u128 = self.unbonding_phases
            .iter()
            .filter(|p| !p.claimed)
            .map(|p| p.amount)
            .sum();
        self.active_balance + unbonding_total
    }

    /// Claimable rewards after fee
    pub fn claimable_after_fee(&self) -> u128 {
        let fee_amount = (self.accumulated_rewards as f64 * self.commission_rate / 100.0) as u128;
        self.accumulated_rewards.saturating_sub(fee_amount)
    }

    /// Current position value (balance + unclaimed rewards)
    pub fn position_value(&self) -> u128 {
        self.total_balance() + self.accumulated_rewards
    }

    /// Percentage of balance in unbonding
    pub fn unbonding_percentage(&self) -> f64 {
        if self.total_balance() == 0 {
            return 0.0;
        }
        let unbonding: u128 = self.unbonding_phases
            .iter()
            .filter(|p| !p.claimed)
            .map(|p| p.amount)
            .sum();
        (unbonding as f64 / self.total_balance() as f64) * 100.0
    }

    /// Can unlock remaining balance
    pub fn can_unlock(&self) -> bool {
        matches!(self.status, PositionStatus::Active | PositionStatus::Locked)
    }

    /// Can claim unbonded funds
    pub fn claimable_unbonded(&self) -> Vec<&UnbondingPhase> {
        self.unbonding_phases.iter().filter(|p| p.claimed == false).collect()
    }
}

/// Staking Ledger — Maintains all delegator positions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingLedger {
    positions: HashMap<String, StakingPosition>,
    by_delegator: HashMap<String, Vec<String>>, // delegator → position IDs
    position_counter: u32,
}

impl StakingLedger {
    pub fn new() -> Self {
        StakingLedger {
            positions: HashMap::new(),
            by_delegator: HashMap::new(),
            position_counter: 0,
        }
    }

    /// Register new staking position
    pub fn stake(
        &mut self,
        delegator: &str,
        validator: &str,
        amount: u128,
        commission_rate: f64,
        unstake_fee: f64,
    ) -> Result<String> {
        if amount == 0 {
            return Err(StakingError::InsufficientBalance);
        }

        self.position_counter += 1;
        let position_id = format!("pos_{}", self.position_counter);

        let position = StakingPosition {
            id: position_id.clone(),
            delegator: delegator.to_string(),
            validator: validator.to_string(),
            active_balance: amount,
            locked_until: Utc::now() + chrono::Duration::days(28),
            unbonding_phases: vec![],
            accumulated_rewards: 0,
            claimed_rewards: 0,
            status: PositionStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission_rate,
            unstake_fee,
        };

        self.positions.insert(position_id.clone(), position);
        self.by_delegator
            .entry(delegator.to_string())
            .or_insert_with(Vec::new)
            .push(position_id.clone());

        Ok(position_id)
    }

    /// Get position by ID
    pub fn get_position(&self, position_id: &str) -> Result<StakingPosition> {
        self.positions
            .get(position_id)
            .cloned()
            .ok_or(StakingError::PositionNotFound)
    }

    /// Get all positions for delegator
    pub fn delegator_positions(&self, delegator: &str) -> Vec<StakingPosition> {
        self.by_delegator
            .get(delegator)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.positions.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Accrue rewards to position
    pub fn accrue_rewards(&mut self, position_id: &str, reward_amount: u128) -> Result<()> {
        let position = self
            .positions
            .get_mut(position_id)
            .ok_or(StakingError::PositionNotFound)?;

        position.accumulated_rewards = position.accumulated_rewards.saturating_add(reward_amount);
        position.updated_at = Utc::now();
        Ok(())
    }

    /// Start unstaking from active balance
    pub fn unbond(&mut self, position_id: &str, amount: u128) -> Result<()> {
        let position = self
            .positions
            .get_mut(position_id)
            .ok_or(StakingError::PositionNotFound)?;

        if !position.can_unlock() {
            return Err(StakingError::UnbondingInProgress);
        }

        if position.active_balance < amount {
            return Err(StakingError::InsufficientBalance);
        }

        position.active_balance -= amount;
        position.status = PositionStatus::Unbonding;

        // Create unbonding phase (28 era period)
        position.unbonding_phases.push(UnbondingPhase {
            started_era: get_current_era(),
            unlock_era: get_current_era() + 28,
            amount,
            claimed: false,
        });

        position.updated_at = Utc::now();
        Ok(())
    }

    /// Claim unbonded funds
    pub fn claim_unbonded(&mut self, position_id: &str) -> Result<u128> {
        let position = self
            .positions
            .get_mut(position_id)
            .ok_or(StakingError::PositionNotFound)?;

        let current_era = get_current_era();
        let mut claimed_total = 0;

        for phase in position.unbonding_phases.iter_mut() {
            if !phase.claimed && current_era >= phase.unlock_era {
                claimed_total += phase.amount;
                phase.claimed = true;
            }
        }

        if claimed_total > 0 {
            position.status = if position.active_balance == 0 {
                PositionStatus::Claimed
            } else {
                PositionStatus::Active
            };
            position.updated_at = Utc::now();
        }

        Ok(claimed_total)
    }

    /// Claim accumulated rewards
    pub fn claim_rewards(&mut self, position_id: &str) -> Result<u128> {
        let position = self
            .positions
            .get_mut(position_id)
            .ok_or(StakingError::PositionNotFound)?;

        let rewards_to_claim = position.accumulated_rewards;
        position.accumulated_rewards = 0;
        position.claimed_rewards += rewards_to_claim;
        position.updated_at = Utc::now();

        Ok(rewards_to_claim)
    }

    /// Get total staked amount across all positions
    pub fn total_staked(&self) -> u128 {
        self.positions.values().map(|p| p.active_balance).sum()
    }

    /// Get total accumulated rewards
    pub fn total_rewards(&self) -> u128 {
        self.positions.values().map(|p| p.accumulated_rewards).sum()
    }

    /// Get active positions count
    pub fn active_count(&self) -> u32 {
        self.positions
            .values()
            .filter(|p| matches!(p.status, PositionStatus::Active))
            .count() as u32
    }

    /// Get unbonding positions count
    pub fn unbonding_count(&self) -> u32 {
        self.positions
            .values()
            .filter(|p| matches!(p.status, PositionStatus::Unbonding))
            .count() as u32
    }

    /// Get delegator's total balance
    pub fn delegator_total_balance(&self, delegator: &str) -> u128 {
        self.delegator_positions(delegator)
            .iter()
            .map(|p| p.total_balance())
            .sum()
    }

    /// Get delegator's claimable rewards
    pub fn delegator_claimable_rewards(&self, delegator: &str) -> u128 {
        self.delegator_positions(delegator)
            .iter()
            .map(|p| p.accumulated_rewards)
            .sum()
    }

    /// Position count for delegator
    pub fn delegator_position_count(&self, delegator: &str) -> u32 {
        self.by_delegator
            .get(delegator)
            .map(|ids| ids.len() as u32)
            .unwrap_or(0)
    }
}

fn get_current_era() -> u32 {
    // Mock implementation - in real system gets from chain
    0
}

impl Default for StakingLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stake_creation() {
        let mut ledger = StakingLedger::new();
        let pos_id = ledger
            .stake("alice", "validator1", 1000, 10.0, 0.5)
            .unwrap();

        assert!(!pos_id.is_empty());
        assert_eq!(ledger.total_staked(), 1000);
    }

    #[test]
    fn test_position_balance_calculation() {
        let position = StakingPosition {
            id: "test".to_string(),
            delegator: "alice".to_string(),
            validator: "val1".to_string(),
            active_balance: 1000,
            locked_until: Utc::now(),
            unbonding_phases: vec![UnbondingPhase {
                started_era: 0,
                unlock_era: 28,
                amount: 200,
                claimed: false,
            }],
            accumulated_rewards: 50,
            claimed_rewards: 0,
            status: PositionStatus::Unbonding,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission_rate: 10.0,
            unstake_fee: 0.5,
        };

        assert_eq!(position.total_balance(), 1200);
        assert_eq!(position.position_value(), 1250);
        assert_eq!(position.unbonding_percentage(), 200.0 / 1200.0 * 100.0);
    }

    #[test]
    fn test_accrue_rewards() {
        let mut ledger = StakingLedger::new();
        let pos_id = ledger
            .stake("alice", "validator1", 1000, 10.0, 0.5)
            .unwrap();

        ledger.accrue_rewards(&pos_id, 100).unwrap();
        let position = ledger.get_position(&pos_id).unwrap();
        assert_eq!(position.accumulated_rewards, 100);
    }

    #[test]
    fn test_unbond() {
        let mut ledger = StakingLedger::new();
        let pos_id = ledger
            .stake("alice", "validator1", 1000, 10.0, 0.5)
            .unwrap();

        ledger.unbond(&pos_id, 300).unwrap();
        let position = ledger.get_position(&pos_id).unwrap();

        assert_eq!(position.active_balance, 700);
        assert_eq!(position.unbonding_phases.len(), 1);
        assert_eq!(position.unbonding_phases[0].amount, 300);
    }

    #[test]
    fn test_claim_rewards() {
        let mut ledger = StakingLedger::new();
        let pos_id = ledger
            .stake("alice", "validator1", 1000, 10.0, 0.5)
            .unwrap();

        ledger.accrue_rewards(&pos_id, 100).unwrap();
        let claimed = ledger.claim_rewards(&pos_id).unwrap();

        assert_eq!(claimed, 100);
        let position = ledger.get_position(&pos_id).unwrap();
        assert_eq!(position.accumulated_rewards, 0);
        assert_eq!(position.claimed_rewards, 100);
    }

    #[test]
    fn test_delegator_positions() {
        let mut ledger = StakingLedger::new();
        ledger.stake("alice", "validator1", 1000, 10.0, 0.5).unwrap();
        ledger.stake("alice", "validator2", 500, 10.0, 0.5).unwrap();
        ledger.stake("bob", "validator1", 2000, 10.0, 0.5).unwrap();

        let alice_positions = ledger.delegator_positions("alice");
        assert_eq!(alice_positions.len(), 2);
        assert_eq!(ledger.delegator_total_balance("alice"), 1500);

        let bob_positions = ledger.delegator_positions("bob");
        assert_eq!(bob_positions.len(), 1);
    }

    #[test]
    fn test_multiple_stakers() {
        let mut ledger = StakingLedger::new();
        ledger.stake("alice", "validator1", 1000, 10.0, 0.5).unwrap();
        ledger.stake("bob", "validator1", 2000, 10.0, 0.5).unwrap();
        ledger.stake("charlie", "validator2", 1500, 10.0, 0.5).unwrap();

        assert_eq!(ledger.total_staked(), 4500);
        assert_eq!(ledger.active_count(), 3);
    }

    #[test]
    fn test_cannot_stake_zero() {
        let mut ledger = StakingLedger::new();
        let result = ledger.stake("alice", "validator1", 0, 10.0, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_unbond_insufficient_balance() {
        let mut ledger = StakingLedger::new();
        let pos_id = ledger
            .stake("alice", "validator1", 100, 10.0, 0.5)
            .unwrap();

        let result = ledger.unbond(&pos_id, 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_claimable_after_fee() {
        let position = StakingPosition {
            id: "test".to_string(),
            delegator: "alice".to_string(),
            validator: "val1".to_string(),
            active_balance: 1000,
            locked_until: Utc::now(),
            unbonding_phases: vec![],
            accumulated_rewards: 1000,
            claimed_rewards: 0,
            status: PositionStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission_rate: 10.0, // 10% commission
            unstake_fee: 0.5,
        };

        let claimable = position.claimable_after_fee();
        assert_eq!(claimable, 900); // 1000 - (1000 * 10%) = 900
    }

    #[test]
    fn test_position_status_transitions() {
        let mut ledger = StakingLedger::new();
        let pos_id = ledger
            .stake("alice", "validator1", 1000, 10.0, 0.5)
            .unwrap();

        let pos = ledger.get_position(&pos_id).unwrap();
        assert_eq!(pos.status, PositionStatus::Active);

        ledger.unbond(&pos_id, 500).unwrap();
        let pos = ledger.get_position(&pos_id).unwrap();
        assert_eq!(pos.status, PositionStatus::Unbonding);
    }

    #[test]
    fn test_delegator_claimable_rewards() {
        let mut ledger = StakingLedger::new();
        let pos1 = ledger
            .stake("alice", "validator1", 1000, 10.0, 0.5)
            .unwrap();
        let pos2 = ledger
            .stake("alice", "validator2", 500, 10.0, 0.5)
            .unwrap();

        ledger.accrue_rewards(&pos1, 100).unwrap();
        ledger.accrue_rewards(&pos2, 50).unwrap();

        assert_eq!(ledger.delegator_claimable_rewards("alice"), 150);
    }

    #[test]
    fn test_position_value_with_rewards() {
        let position = StakingPosition {
            id: "test".to_string(),
            delegator: "alice".to_string(),
            validator: "val1".to_string(),
            active_balance: 1000,
            locked_until: Utc::now(),
            unbonding_phases: vec![],
            accumulated_rewards: 100,
            claimed_rewards: 0,
            status: PositionStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission_rate: 10.0,
            unstake_fee: 0.5,
        };

        assert_eq!(position.position_value(), 1100);
    }

    #[test]
    fn test_unbonding_phases() {
        let mut ledger = StakingLedger::new();
        let pos_id = ledger
            .stake("alice", "validator1", 1000, 10.0, 0.5)
            .unwrap();

        ledger.unbond(&pos_id, 300).unwrap();
        ledger.unbond(&pos_id, 200).unwrap();

        let position = ledger.get_position(&pos_id).unwrap();
        assert_eq!(position.unbonding_phases.len(), 2);
        assert_eq!(position.total_balance(), 1000); // 500 active + 300 + 200 unbonding
    }
}
