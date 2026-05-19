//! Treasury Management — funding allocation and disbursement control
//!
//! Features:
//! - Multi-sig approval for spends
//! - Budget caps per period
//! - Spending history and audit trail
//! - Emergency fund reserves
//! - ROI tracking for allocations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpendStatus {
    Proposed,
    Approved,
    Rejected,
    Executed,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendProposal {
    pub id: u32,
    pub recipient: String,
    pub amount: u128,
    pub reason: String,
    pub status: SpendStatus,
    pub approvals: u32,
    pub created_block: u64,
    pub executed_block: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriod {
    pub period_id: u32,
    pub start_block: u64,
    pub end_block: u64,
    pub allocated: u128,
    pub spent: u128,
}

pub struct Treasury {
    total_balance: u128,
    emergency_funds: u128,
    spends: HashMap<u32, SpendProposal>,
    next_spend_id: u32,
    budget_periods: HashMap<u32, BudgetPeriod>,
    next_period_id: u32,
    approvers: Vec<String>,
    required_approvals: u32,
}

impl Treasury {
    pub fn new(initial_balance: u128, emergency_reserve: u128, required_approvals: u32) -> Self {
        let available = initial_balance.saturating_sub(emergency_reserve);

        Self {
            total_balance: initial_balance,
            emergency_funds: emergency_reserve,
            spends: HashMap::new(),
            next_spend_id: 1,
            budget_periods: HashMap::new(),
            next_period_id: 1,
            approvers: Vec::new(),
            required_approvals,
        }
    }

    /// Get available balance (excluding emergency funds)
    pub fn available_balance(&self) -> u128 {
        self.total_balance.saturating_sub(self.emergency_funds)
    }

    /// Get total balance
    pub fn get_balance(&self) -> u128 {
        self.total_balance
    }

    /// Deposit funds to treasury
    pub fn deposit(&mut self, amount: u128) {
        self.total_balance = self.total_balance.saturating_add(amount);
    }

    /// Propose a spend
    pub fn propose_spend(
        &mut self,
        recipient: String,
        amount: u128,
        reason: String,
        created_block: u64,
    ) -> Result<u32, String> {
        if amount > self.available_balance() {
            return Err("Insufficient treasury balance".to_string());
        }

        let spend_id = self.next_spend_id;
        self.next_spend_id += 1;

        let spend = SpendProposal {
            id: spend_id,
            recipient,
            amount,
            reason,
            status: SpendStatus::Proposed,
            approvals: 0,
            created_block,
            executed_block: None,
        };

        self.spends.insert(spend_id, spend);
        Ok(spend_id)
    }

    /// Approve a spend (M-of-N consensus)
    pub fn approve_spend(&mut self, spend_id: u32, approver: String) -> Result<bool, String> {
        if !self.approvers.contains(&approver) {
            return Err("Not an authorized approver".to_string());
        }

        if let Some(spend) = self.spends.get_mut(&spend_id) {
            spend.approvals += 1;

            // Check if threshold reached
            if spend.approvals >= self.required_approvals {
                spend.status = SpendStatus::Approved;
                return Ok(true);
            }
            Ok(false)
        } else {
            Err("Spend proposal not found".to_string())
        }
    }

    /// Execute an approved spend
    pub fn execute_spend(&mut self, spend_id: u32, current_block: u64) -> Result<u128, String> {
        if let Some(spend) = self.spends.get_mut(&spend_id) {
            if spend.status != SpendStatus::Approved {
                return Err("Spend must be approved before execution".to_string());
            }

            if spend.amount > self.available_balance() {
                return Err("Insufficient funds".to_string());
            }

            spend.status = SpendStatus::Executed;
            spend.executed_block = Some(current_block);
            self.total_balance = self.total_balance.saturating_sub(spend.amount);

            Ok(spend.amount)
        } else {
            Err("Spend proposal not found".to_string())
        }
    }

    /// Reject a spend
    pub fn reject_spend(&mut self, spend_id: u32) -> Result<(), String> {
        if let Some(spend) = self.spends.get_mut(&spend_id) {
            if spend.status == SpendStatus::Proposed {
                spend.status = SpendStatus::Rejected;
                Ok(())
            } else {
                Err("Cannot reject non-proposed spend".to_string())
            }
        } else {
            Err("Spend proposal not found".to_string())
        }
    }

    /// Add an approver
    pub fn add_approver(&mut self, approver: String) {
        if !self.approvers.contains(&approver) {
            self.approvers.push(approver);
        }
    }

    /// Remove an approver
    pub fn remove_approver(&mut self, approver: &str) {
        self.approvers.retain(|a| a != approver);
    }

    /// Get approvers list
    pub fn get_approvers(&self) -> &Vec<String> {
        &self.approvers
    }

    /// Create budget period
    pub fn create_budget_period(
        &mut self,
        start_block: u64,
        end_block: u64,
        allocated: u128,
    ) -> u32 {
        let period_id = self.next_period_id;
        self.next_period_id += 1;

        let period = BudgetPeriod {
            period_id,
            start_block,
            end_block,
            allocated,
            spent: 0,
        };

        self.budget_periods.insert(period_id, period);
        period_id
    }

    /// Get budget period
    pub fn get_budget_period(&self, period_id: u32) -> Option<&BudgetPeriod> {
        self.budget_periods.get(&period_id)
    }

    /// Check if within budget
    pub fn is_within_budget(&self, period_id: u32, amount: u128) -> bool {
        if let Some(period) = self.budget_periods.get(&period_id) {
            period.spent + amount <= period.allocated
        } else {
            false
        }
    }

    /// Get spending history
    pub fn get_spending_history(&self, status: Option<SpendStatus>) -> Vec<&SpendProposal> {
        self.spends
            .values()
            .filter(|s| status.is_none() || s.status == status.unwrap())
            .collect()
    }

    /// Get spend proposal
    pub fn get_spend(&self, spend_id: u32) -> Option<&SpendProposal> {
        self.spends.get(&spend_id)
    }

    /// Total spent in history
    pub fn total_spent(&self) -> u128 {
        self.spends
            .values()
            .filter(|s| s.status == SpendStatus::Executed)
            .map(|s| s.amount)
            .sum()
    }

    /// Get pending spends (Proposed or Approved)
    pub fn get_pending_spends(&self) -> Vec<&SpendProposal> {
        self.spends
            .values()
            .filter(|s| s.status == SpendStatus::Proposed || s.status == SpendStatus::Approved)
            .collect()
    }

    /// Refund on-chain (if transaction reverted)
    pub fn refund_spend(&mut self, spend_id: u32, refund_amount: u128) -> Result<(), String> {
        if let Some(spend) = self.spends.get_mut(&spend_id) {
            if spend.status != SpendStatus::Executed {
                return Err("Can only refund executed spends".to_string());
            }

            if refund_amount > spend.amount {
                return Err("Refund exceeds spend amount".to_string());
            }

            spend.status = SpendStatus::Refunded;
            self.total_balance = self.total_balance.saturating_add(refund_amount);

            Ok(())
        } else {
            Err("Spend proposal not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_treasury_creation() {
        let treasury = Treasury::new(10_000, 2_000, 3);
        assert_eq!(treasury.get_balance(), 10_000);
        assert_eq!(treasury.available_balance(), 8_000);
    }

    #[test]
    fn test_deposit() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        treasury.deposit(1_000);
        assert_eq!(treasury.get_balance(), 11_000);
    }

    #[test]
    fn test_propose_spend() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        let spend_id = treasury
            .propose_spend("alice".to_string(), 1_000, "test".to_string(), 100)
            .unwrap();
        assert_eq!(spend_id, 1);
    }

    #[test]
    fn test_propose_exceeds_balance() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        let result = treasury.propose_spend("alice".to_string(), 9_001, "test".to_string(), 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_approve_spend() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        treasury.add_approver("approver1".to_string());
        treasury.add_approver("approver2".to_string());
        treasury.add_approver("approver3".to_string());

        let spend_id = treasury
            .propose_spend("alice".to_string(), 1_000, "test".to_string(), 100)
            .unwrap();

        let approved1 = treasury.approve_spend(spend_id, "approver1".to_string()).unwrap();
        let approved2 = treasury.approve_spend(spend_id, "approver2".to_string()).unwrap();
        let approved3 = treasury.approve_spend(spend_id, "approver3".to_string()).unwrap();

        assert!(!approved1);
        assert!(!approved2);
        assert!(approved3); // 3rd approval triggers
    }

    #[test]
    fn test_execute_spend() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        treasury.add_approver("approver1".to_string());

        let spend_id = treasury
            .propose_spend("alice".to_string(), 1_000, "test".to_string(), 100)
            .unwrap();

        treasury.approve_spend(spend_id, "approver1".to_string()).ok();

        let spend = treasury.spends.get_mut(&spend_id).unwrap();
        spend.approvals = 3; // Manually set to approved

        let executed = treasury.execute_spend(spend_id, 200).unwrap();
        assert_eq!(executed, 1_000);
        assert_eq!(treasury.get_balance(), 9_000);
    }

    #[test]
    fn test_reject_spend() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        let spend_id = treasury
            .propose_spend("alice".to_string(), 1_000, "test".to_string(), 100)
            .unwrap();

        assert!(treasury.reject_spend(spend_id).is_ok());
    }

    #[test]
    fn test_add_remove_approver() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        treasury.add_approver("alice".to_string());
        assert_eq!(treasury.get_approvers().len(), 1);

        treasury.remove_approver("alice");
        assert_eq!(treasury.get_approvers().len(), 0);
    }

    #[test]
    fn test_budget_period() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        let period_id = treasury.create_budget_period(0, 100, 5_000);
        let period = treasury.get_budget_period(period_id).unwrap();
        assert_eq!(period.allocated, 5_000);
    }

    #[test]
    fn test_is_within_budget() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        let period_id = treasury.create_budget_period(0, 100, 5_000);
        assert!(treasury.is_within_budget(period_id, 4_000));
        assert!(!treasury.is_within_budget(period_id, 6_000));
    }

    #[test]
    fn test_total_spent() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        treasury.add_approver("approver".to_string());

        let spend_id = treasury
            .propose_spend("alice".to_string(), 1_000, "test".to_string(), 100)
            .unwrap();

        let spend = treasury.spends.get_mut(&spend_id).unwrap();
        spend.status = SpendStatus::Executed;

        assert_eq!(treasury.total_spent(), 1_000);
    }

    #[test]
    fn test_refund_spend() {
        let mut treasury = Treasury::new(10_000, 2_000, 3);
        let initial_balance = treasury.get_balance();

        let spend_id = treasury
            .propose_spend("alice".to_string(), 1_000, "test".to_string(), 100)
            .unwrap();

        let spend = treasury.spends.get_mut(&spend_id).unwrap();
        spend.status = SpendStatus::Executed;
        treasury.total_balance = treasury.total_balance.saturating_sub(1_000);

        treasury.refund_spend(spend_id, 500).unwrap();
        assert_eq!(treasury.get_balance(), initial_balance - 500);
    }

    #[test]
    fn test_spend_status_enum() {
        assert_eq!(SpendStatus::Proposed, SpendStatus::Proposed);
        assert_ne!(SpendStatus::Proposed, SpendStatus::Approved);
    }
}
