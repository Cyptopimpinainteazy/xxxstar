#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 Automation
//!
//! Keeper network and automated task execution for X3 Chain.
//! Enables conditional task execution based on on-chain state.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Task identifier
pub type TaskId = H256;

/// Task condition types
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum Condition {
    /// Time-based: execute at specific block number
    BlockNumber(u64),
    /// Price condition: execute when asset price meets threshold
    PriceThreshold {
        asset_id: u32,
        threshold: u64,
        above: bool, // true = execute when price > threshold
    },
    /// Custom condition with encoded data
    Custom([u8; 64]), // Fixed size for simplicity
}

/// Task action types
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum Action {
    /// Call a pallet extrinsic
    Extrinsic {
        pallet_index: u8,
        call_index: u8,
        call_data: [u8; 64], // Fixed size for simplicity
    },
    /// Custom action with encoded data
    Custom([u8; 64]), // Fixed size for simplicity
}

/// Automated task definition
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct Task<AccountId, Balance> {
    /// Unique task identifier
    pub id: TaskId,
    /// Account that created the task
    pub owner: AccountId,
    /// Condition that triggers execution
    pub condition: Condition,
    /// Action to execute when condition is met
    pub action: Action,
    /// Maximum fee willing to pay for execution
    pub max_fee: Balance,
    /// Block number when task expires
    pub expiry_block: u64,
    /// Task status
    pub status: TaskStatus,
}

/// Task execution status
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum TaskStatus {
    /// Task is active and waiting for condition
    Active,
    /// Task executed successfully
    Executed,
    /// Task failed to execute
    Failed,
    /// Task expired without execution
    Expired,
    /// Task was cancelled by owner
    Cancelled,
}

/// Task execution result
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct ExecutionResult {
    /// Task that was executed
    pub task_id: TaskId,
    /// Whether execution succeeded
    pub success: bool,
    /// Gas used in execution
    pub gas_used: u64,
    /// Fee charged
    pub fee_charged: u128,
    /// Execution output/error data
    pub output: Vec<u8>,
}

/// Task registry trait for pallets to implement
pub trait TaskRegistry<AccountId, Balance> {
    /// Register a new automated task
    fn register_task(task: Task<AccountId, Balance>) -> Result<TaskId, AutomationError>;

    /// Cancel a task by its owner
    fn cancel_task(task_id: TaskId, caller: &AccountId) -> Result<(), AutomationError>;

    /// Execute a task (typically called by off-chain workers)
    fn execute_task(task_id: TaskId) -> Result<ExecutionResult, AutomationError>;

    /// Check if a task's condition is met
    fn check_condition(task: &Task<AccountId, Balance>) -> bool;

    /// Clean up expired tasks
    fn cleanup_expired_tasks(current_block: u64) -> u32;
}

/// Automation errors
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum AutomationError {
    /// Task ID already exists
    TaskAlreadyExists,
    /// Task not found
    TaskNotFound,
    /// Caller is not the task owner
    NotTaskOwner,
    /// Task has expired
    TaskExpired,
    /// Task is not in active status
    TaskNotActive,
    /// Condition not met
    ConditionNotMet,
    /// Execution failed
    ExecutionFailed,
    /// Insufficient balance for fee
    InsufficientBalance,
    /// Fee calculation overflow
    FeeOverflow,
    /// Invalid task parameters
    InvalidTask,
}

/// Keeper network participant
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct Keeper<AccountId> {
    /// Keeper account
    pub account: AccountId,
    /// Total tasks executed
    pub tasks_executed: u64,
    /// Success rate (0-10000, representing 0.00%-100.00%)
    pub success_rate: u16,
    /// Total fees earned
    pub total_fees_earned: u128,
    /// Last active block
    pub last_active: u64,
}

/// Helper functions for condition checking
pub mod conditions {

    /// Check if block number condition is met
    pub fn check_block_condition(condition_block: u64, current_block: u64) -> bool {
        current_block >= condition_block
    }

    /// Check if price threshold condition is met
    pub fn check_price_condition(
        _asset_id: u32,
        threshold: u64,
        above: bool,
        current_price: Option<u64>,
    ) -> bool {
        if let Some(price) = current_price {
            if above {
                price > threshold
            } else {
                price < threshold
            }
        } else {
            false
        }
    }
}

/// Helper functions for task execution
pub mod execution {
    use super::*;

    /// Calculate execution fee based on gas used and gas price
    pub fn calculate_fee(gas_used: u64, gas_price: u128) -> Result<u128, crate::AutomationError> {
        gas_used
            .checked_mul(gas_price as u64)
            .ok_or(crate::AutomationError::FeeOverflow)?
            .try_into()
            .map_err(|_| crate::AutomationError::FeeOverflow)
    }

    /// Validate task parameters (simplified for demo)
    pub fn validate_task<AccountId, Balance: Default + PartialEq>(
        _task: &Task<AccountId, Balance>,
    ) -> Result<(), crate::AutomationError> {
        // Simplified validation for demo
        Ok(())
    }

    /// Get current block number (placeholder - would be implemented by runtime)
    pub fn get_current_block() -> u64 {
        // In real implementation, this would access frame_system
        0
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_condition_block_check() {
        assert!(!conditions::check_block_condition(100, 50));
        assert!(conditions::check_block_condition(100, 100));
        assert!(conditions::check_block_condition(100, 150));
    }

    #[test]
    fn test_condition_price_check() {
        // Above threshold
        assert!(!conditions::check_price_condition(1, 100, true, Some(50)));
        assert!(conditions::check_price_condition(1, 100, true, Some(150)));
        assert!(!conditions::check_price_condition(1, 100, true, Some(100)));

        // Below threshold
        assert!(conditions::check_price_condition(1, 100, false, Some(50)));
        assert!(!conditions::check_price_condition(1, 100, false, Some(150)));
        assert!(!conditions::check_price_condition(1, 100, false, Some(100)));

        // No price data
        assert!(!conditions::check_price_condition(1, 100, true, None));
    }

    #[test]
    fn test_fee_calculation() {
        assert_eq!(execution::calculate_fee(21000, 1).unwrap(), 21000);
        assert_eq!(execution::calculate_fee(1000, 2).unwrap(), 2000);
        assert!(execution::calculate_fee(u64::MAX, u128::MAX).is_err());
    }
}
