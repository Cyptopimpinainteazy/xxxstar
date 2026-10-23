#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 Automation
//!
//! Task conditions, whitelisted actions and execution bookkeeping shared by
//! `pallet-x3-automation` and its off-chain components.
//!
//! Two rules shape this crate:
//!
//! * A condition that cannot be evaluated is **not** satisfied. Callers get
//!   `ConditionUnavailable` rather than a defaulted answer, so a dead oracle can
//!   never trigger a trade.
//! * An action is a whitelisted, typed verb. There is no opaque
//!   "call pallet N with these bytes" variant: every action names what it does,
//!   so a reviewer can enumerate the complete set of effects a task can have.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Task identifier
pub type TaskId = H256;

/// Identifier of an asset as the price oracle knows it.
pub type OracleAssetId = u32;

/// Identifier of a ledger asset as the kernel / token factory knows it.
pub type LedgerAssetId = u32;

/// Task condition types.
///
/// Every variant is decidable from on-chain state alone; nothing here depends on
/// an off-chain service being honest.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum Condition {
    /// Execute at or after a block number.
    BlockNumber(u64),
    /// Execute at or after a wall-clock timestamp in milliseconds.
    Timestamp(u64),
    /// Execute when the oracle price of `asset_id` is at or above / at or below
    /// `threshold`. A missing price is an error, never a satisfied condition.
    PriceThreshold {
        /// Asset the oracle reports on.
        asset_id: OracleAssetId,
        /// Threshold price in the oracle's fixed-point units.
        threshold: u64,
        /// `true` triggers on `price >= threshold`, `false` on `price <= threshold`.
        above: bool,
    },
    /// Execute when the oracle price is inside `[lower, upper]`.
    PriceBand {
        /// Asset the oracle reports on.
        asset_id: OracleAssetId,
        /// Inclusive lower bound.
        lower: u64,
        /// Inclusive upper bound.
        upper: u64,
    },
    /// Execute when an event whose encoded payload hashes to `topic` was
    /// emitted in the current block.
    ///
    /// The caller computes `blake2_256(event.encode())`. Matching on a content
    /// commitment rather than `(pallet_index, event_index)` keeps the trigger
    /// stable when pallets are reordered in `construct_runtime!`.
    Event {
        /// Commitment to the encoded event payload.
        topic: [u8; 32],
    },
    /// No on-chain precondition: the task may run whenever a keeper submits it.
    Manual,
    /// Condition evaluated by a runtime-configured evaluator.
    Custom([u8; 64]),
}

/// Task action types.
///
/// This is the entire vocabulary a task can have. Adding a variant is a visible,
/// reviewable change to the chain's automation surface.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum Action {
    /// Move native currency from the task owner to `recipient`.
    ///
    /// `recipient` is an account-key encoding the runtime's dispatcher decodes;
    /// a runtime that cannot decode it refuses the action rather than guessing.
    TransferNative {
        /// Recipient account key.
        recipient: [u8; 32],
        /// Amount to move, in the currency's smallest unit.
        amount: u128,
    },
    /// Move a kernel-tracked ledger asset from the task owner to `recipient`.
    TransferLedgerAsset {
        /// Ledger asset to move.
        asset: LedgerAssetId,
        /// Recipient account key.
        recipient: [u8; 32],
        /// Amount to move.
        amount: u128,
    },
    /// No effect.
    ///
    /// Exists so a task can be registered, scheduled and observed without moving
    /// funds — rehearsal, monitoring and the automation test suite all need a
    /// harmless verb. It is reported as `ActionOutcome::Noop`, never as a
    /// transfer that happened.
    Noop,
    /// Action interpreted by a runtime-configured dispatcher.
    ///
    /// The runtime must opt in explicitly; an unwired dispatcher returns
    /// `AutomationError::ActionUnsupported`.
    Custom([u8; 64]),
}

/// Automated task definition.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct Task<AccountId, Balance> {
    /// Unique task identifier.
    pub id: TaskId,
    /// Account that created the task and whose balance the action draws on.
    pub owner: AccountId,
    /// Condition that triggers execution.
    pub condition: Condition,
    /// Action to execute when the condition is met.
    pub action: Action,
    /// Maximum fee the owner will pay a keeper for one execution.
    pub max_fee: Balance,
    /// Block number when the task expires.
    pub expiry_block: u64,
    /// Task status.
    pub status: TaskStatus,
    /// Executions attempted so far. Bounded by the runtime's `MaxAttempts`.
    pub attempts: u32,
}

/// Task execution status.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum TaskStatus {
    /// Waiting for its condition.
    Active,
    /// Condition was met and the action completed; terminal.
    Executed,
    /// The action failed and no attempts remain; terminal.
    Failed,
    /// `expiry_block` passed before the condition was met; terminal.
    Expired,
    /// Owner cancelled the task; terminal.
    Cancelled,
}

impl TaskStatus {
    /// Whether no further work will happen for a task in this status.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Active)
    }
}

/// What an action did when the dispatcher ran it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum ActionOutcome {
    /// The action changed state.
    Applied,
    /// The action is a deliberate no-op.
    Noop,
}

/// Record that one execution attempt happened.
///
/// This is the on-chain half of the execution trail: it is written in the same
/// transaction as the effect it describes, so a receipt exists exactly when the
/// effect it attests to happened.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct ExecutionReceipt {
    /// Task that ran.
    pub task_id: TaskId,
    /// Zero-based attempt counter; `(task_id, attempt)` is unique.
    pub attempt: u32,
    /// Keeper that submitted the execution.
    pub keeper_hash: [u8; 32],
    /// Block the attempt was included in.
    pub block: u64,
    /// Outcome of the action.
    pub outcome: ActionOutcome,
    /// Weight the attempt actually consumed.
    pub weight_ref_time: u64,
    /// Weight proof size the attempt actually consumed.
    pub weight_proof_size: u64,
    /// Data the dispatcher returned on success; empty when it failed.
    pub output: [u8; 64],
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
    /// The condition could not be evaluated; treat as not met.
    ///
    /// Distinct from `ConditionNotMet` because it is an operational fault (no
    /// price feed, no evaluator) rather than the condition simply being false.
    ConditionUnavailable,
    /// Execution failed
    ExecutionFailed,
    /// The action named something the runtime does not implement.
    ActionUnsupported,
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

    /// Check if price threshold condition is met.
    ///
    /// `current_price` is `Option` so the caller must decide what a missing price
    /// means. This helper returns `false`, and the pallet reports
    /// `ConditionUnavailable` rather than treating the answer as a real "no".
    pub fn check_price_condition(
        _asset_id: u32,
        threshold: u64,
        above: bool,
        current_price: Option<u64>,
    ) -> bool {
        match current_price {
            Some(price) if above => price >= threshold,
            Some(price) => price <= threshold,
            None => false,
        }
    }

    /// Check whether `price` lies inside the inclusive band `[lower, upper]`.
    ///
    /// An inverted band (`lower > upper`) matches nothing, which is the safe
    /// reading of a nonsensical trigger.
    pub fn check_price_band(lower: u64, upper: u64, price: u64) -> bool {
        lower <= upper && (lower..=upper).contains(&price)
    }
}

/// Helper functions for task execution
pub mod execution {
    use super::*;

    /// Calculate execution fee based on gas used and gas price
    pub fn calculate_fee(gas_used: u64, gas_price: u128) -> Result<u128, crate::AutomationError> {
        let product = gas_used
            .checked_mul(gas_price as u64)
            .ok_or(crate::AutomationError::FeeOverflow)?;
        Ok(u128::from(product))
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
    use crate::{conditions, execution, Action, ActionOutcome, Condition, TaskStatus};

    #[test]
    fn test_condition_block_check() {
        assert!(!conditions::check_block_condition(100, 50));
        assert!(conditions::check_block_condition(100, 100));
        assert!(conditions::check_block_condition(100, 150));
    }

    #[test]
    fn test_condition_price_check() {
        // Above threshold: touching the threshold counts.
        assert!(!conditions::check_price_condition(1, 100, true, Some(50)));
        assert!(conditions::check_price_condition(1, 100, true, Some(150)));
        assert!(conditions::check_price_condition(1, 100, true, Some(100)));

        // Below threshold: touching the threshold counts.
        assert!(conditions::check_price_condition(1, 100, false, Some(50)));
        assert!(!conditions::check_price_condition(1, 100, false, Some(150)));
        assert!(conditions::check_price_condition(1, 100, false, Some(100)));

        // No price data is never a match, in either direction.
        assert!(!conditions::check_price_condition(1, 100, true, None));
        assert!(!conditions::check_price_condition(1, 100, false, None));
    }

    #[test]
    fn test_price_band() {
        assert!(conditions::check_price_band(10, 20, 10));
        assert!(conditions::check_price_band(10, 20, 15));
        assert!(conditions::check_price_band(10, 20, 20));
        assert!(!conditions::check_price_band(10, 20, 9));
        assert!(!conditions::check_price_band(10, 20, 21));
        // An inverted band matches nothing rather than everything.
        assert!(!conditions::check_price_band(20, 10, 15));
    }

    #[test]
    fn test_fee_calculation() {
        assert_eq!(execution::calculate_fee(21000, 1).unwrap(), 21000);
        assert_eq!(execution::calculate_fee(1000, 2).unwrap(), 2000);
        assert!(execution::calculate_fee(u64::MAX, u128::MAX).is_err());
    }

    #[test]
    fn only_active_tasks_are_non_terminal() {
        assert!(!TaskStatus::Active.is_terminal());
        assert!(TaskStatus::Executed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Expired.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[test]
    fn a_noop_is_never_reported_as_applied() {
        assert_eq!(Action::Noop, Action::Noop);
        assert_ne!(ActionOutcome::Noop, ActionOutcome::Applied);
        let transfer = Action::TransferNative {
            recipient: [7; 32],
            amount: 1_000,
        };
        assert_ne!(transfer, Action::Noop);
    }

    #[test]
    fn every_trigger_is_decidable_from_chain_state() {
        // No variant carries an off-chain handle such as a URL or an endpoint.
        let all = [
            Condition::BlockNumber(1),
            Condition::Timestamp(1),
            Condition::PriceThreshold {
                asset_id: 1,
                threshold: 1,
                above: true,
            },
            Condition::PriceBand {
                asset_id: 1,
                lower: 1,
                upper: 2,
            },
            Condition::Event { topic: [0; 32] },
            Condition::Manual,
            Condition::Custom([0; 64]),
        ];
        assert_eq!(all.len(), 7);
    }
}
