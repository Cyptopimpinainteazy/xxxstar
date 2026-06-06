//! Invariant Enforcement Module
//!
//! NON-NEGOTIABLE rules that X3VM enforces:
//!
//! 1. No asset finalized unless ALL legs are provably complete
//! 2. No BTC release without X3 confirmation
//! 3. No cross-VM partial state
//! 4. All intents must resolve (finalize or refund)
//! 5. Timeouts ALWAYS favor user funds
//!
//! Violations result in:
//! - Halt settlement
//! - Slash operators (testnet)
//! - Block governance upgrades (if invariant checks fail)

use crate::types::{IntentState, InvariantViolationType, RefundReason};
use codec::{Decode, DecodeWithMemTracking, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{vec, vec::Vec};

/// Invariant check result
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, PartialEq, Eq)]
pub enum InvariantCheckResult {
    /// All invariants satisfied
    Pass,
    /// Invariant violation detected
    Fail(InvariantViolationType),
    /// Check could not be completed (missing data)
    Inconclusive,
}

/// Invariant checker configuration
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct InvariantConfig {
    /// Enable strict mode (halt on any violation)
    pub strict_mode: bool,
    /// Enable slashing for violations
    pub slashing_enabled: bool,
    /// Minimum confirmations for BTC
    pub min_btc_confirmations: u32,
    /// Maximum allowed settlement time (seconds)
    pub max_settlement_time: u64,
    /// Enable cross-VM reentrancy detection
    pub detect_reentrancy: bool,
}

impl Default for InvariantConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            slashing_enabled: false, // Disabled until mainnet
            min_btc_confirmations: 6,
            max_settlement_time: 86400, // 24 hours
            detect_reentrancy: true,
        }
    }
}

/// Settlement invariants enforcer
pub struct InvariantEnforcer {
    config: InvariantConfig,
}

impl InvariantEnforcer {
    pub fn new(config: InvariantConfig) -> Self {
        Self { config }
    }

    /// Check invariant: No partial execution
    ///
    /// A settlement MUST NOT be finalized if any leg is incomplete.
    pub fn check_no_partial_execution(
        &self,
        legs_total: u32,
        legs_claimed: u32,
        state: IntentState,
    ) -> InvariantCheckResult {
        if matches!(state, IntentState::Finalized) {
            if legs_claimed < legs_total {
                return InvariantCheckResult::Fail(InvariantViolationType::PartialExecution);
            }
        }
        InvariantCheckResult::Pass
    }

    /// Check invariant: No BTC release without X3 confirmation
    ///
    /// BTC can ONLY be released after X3 has:
    /// 1. Verified SPV proof
    /// 2. Confirmed sufficient depth
    /// 3. Verified counterpart legs are locked
    pub fn check_btc_release_requires_x3(
        &self,
        btc_released: bool,
        x3_confirmed: bool,
        btc_confirmations: u32,
    ) -> InvariantCheckResult {
        if btc_released {
            if !x3_confirmed {
                return InvariantCheckResult::Fail(
                    InvariantViolationType::BtcReleaseWithoutConfirmation,
                );
            }
            if btc_confirmations < self.config.min_btc_confirmations {
                return InvariantCheckResult::Fail(
                    InvariantViolationType::BtcReleaseWithoutConfirmation,
                );
            }
        }
        InvariantCheckResult::Pass
    }

    /// Check invariant: No cross-VM partial state
    ///
    /// If any VM execution starts, all VMs must either:
    /// - Complete successfully, OR
    /// - Roll back to pre-execution state
    pub fn check_no_cross_vm_partial_state(
        &self,
        evm_executed: bool,
        svm_executed: bool,
        x3_executed: bool,
        all_success: bool,
        all_reverted: bool,
    ) -> InvariantCheckResult {
        let any_executed = evm_executed || svm_executed || x3_executed;
        let all_executed = evm_executed && svm_executed && x3_executed;

        if any_executed && !all_executed {
            // Partial execution - must be reverting
            if !all_reverted {
                return InvariantCheckResult::Fail(InvariantViolationType::PartialExecution);
            }
        }

        if all_executed && !all_success && !all_reverted {
            // Mixed state - violation
            return InvariantCheckResult::Fail(InvariantViolationType::PartialExecution);
        }

        InvariantCheckResult::Pass
    }

    /// Check invariant: All intents must resolve
    ///
    /// Every intent MUST eventually reach either:
    /// - Finalized (all legs complete)
    /// - Refunded (timeout or explicit failure)
    ///
    /// No intent can be left in limbo.
    pub fn check_intent_resolution(
        &self,
        state: IntentState,
        created_at: u64,
        timeout: u64,
        current_time: u64,
    ) -> InvariantCheckResult {
        // If past timeout and not resolved, violation
        if current_time > timeout {
            if !matches!(state, IntentState::Finalized | IntentState::Refunded) {
                // Intent should have been auto-refunded
                return InvariantCheckResult::Fail(InvariantViolationType::TimeoutBypass);
            }
        }

        // If way past max time and still pending, something is wrong
        if current_time > created_at + self.config.max_settlement_time {
            if !matches!(state, IntentState::Finalized | IntentState::Refunded) {
                return InvariantCheckResult::Fail(InvariantViolationType::TimeoutBypass);
            }
        }

        InvariantCheckResult::Pass
    }

    /// Check invariant: Timeouts favor user funds
    ///
    /// On timeout:
    /// - Locked assets MUST be refundable
    /// - No party should lose funds due to timeout
    /// - Timeout order: Fast chain first, slow chain second
    pub fn check_timeout_favors_users(
        &self,
        timed_out: bool,
        maker_refundable: bool,
        taker_refundable: bool,
    ) -> InvariantCheckResult {
        if timed_out {
            if !maker_refundable || !taker_refundable {
                // Funds stuck - violation
                return InvariantCheckResult::Fail(InvariantViolationType::TimeoutBypass);
            }
        }
        InvariantCheckResult::Pass
    }

    /// Check for cross-VM reentrancy
    ///
    /// Detects if a VM execution triggered re-entry into another VM
    /// in a way that could manipulate state.
    pub fn check_reentrancy(&self, execution_trace: &[VmExecutionEvent]) -> InvariantCheckResult {
        if !self.config.detect_reentrancy {
            return InvariantCheckResult::Pass;
        }

        let mut call_stack: Vec<VmType> = Vec::new();

        for event in execution_trace {
            match event {
                VmExecutionEvent::Enter(vm) => {
                    // Check if re-entering same VM through another
                    if call_stack.contains(vm) && call_stack.len() > 1 {
                        return InvariantCheckResult::Fail(
                            InvariantViolationType::CrossVmReentrancy,
                        );
                    }
                    call_stack.push(*vm);
                }
                VmExecutionEvent::Exit(vm) => {
                    if call_stack.last() == Some(vm) {
                        call_stack.pop();
                    }
                }
            }
        }

        InvariantCheckResult::Pass
    }

    /// Run all invariant checks for a settlement
    pub fn check_all(
        &self,
        legs_total: u32,
        legs_claimed: u32,
        state: IntentState,
        btc_released: bool,
        x3_confirmed: bool,
        btc_confirmations: u32,
        created_at: u64,
        timeout: u64,
        current_time: u64,
    ) -> Vec<InvariantCheckResult> {
        vec![
            self.check_no_partial_execution(legs_total, legs_claimed, state),
            self.check_btc_release_requires_x3(btc_released, x3_confirmed, btc_confirmations),
            self.check_intent_resolution(state, created_at, timeout, current_time),
        ]
    }
}

/// VM execution event for reentrancy detection
#[derive(Clone, Copy, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, PartialEq, Eq)]
pub enum VmExecutionEvent {
    /// Entered VM execution
    Enter(VmType),
    /// Exited VM execution
    Exit(VmType),
}

/// VM type identifier
#[derive(Clone, Copy, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, PartialEq, Eq)]
pub enum VmType {
    Evm,
    Svm,
    X3Vm,
}

/// Slashing parameters for invariant violations
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct SlashingParams {
    /// Base slash amount (in native tokens)
    pub base_slash: u128,
    /// Multiplier for critical violations
    pub critical_multiplier: u32,
    /// Slash destination (treasury)
    pub slash_destination: [u8; 32],
}

/// Report of invariant violation
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct ViolationReport {
    /// Intent ID
    pub intent_id: H256,
    /// Violation type
    pub violation_type: InvariantViolationType,
    /// Reporter account
    pub reporter: [u8; 32],
    /// Evidence (encoded)
    pub evidence: Vec<u8>,
    /// Block when reported
    pub block_number: u64,
    /// Verified by X3
    pub verified: bool,
}

/// Governance proposal invariant check
///
/// Before any governance proposal can execute, it must pass:
/// 1. Invariant simulation (no invariants broken by change)
/// 2. Settlement test suite (existing settlements not affected)
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct GovernanceInvariantCheck {
    /// Proposal ID
    pub proposal_id: H256,
    /// Invariant checks passed
    pub invariants_passed: bool,
    /// Settlement tests passed
    pub settlement_tests_passed: bool,
    /// Simulation result
    pub simulation_result: SimulationResult,
}

/// Governance simulation result
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub enum SimulationResult {
    /// Simulation passed
    Passed,
    /// Simulation failed with reason
    Failed(Vec<u8>),
    /// Simulation not yet run
    Pending,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_enforcer() -> InvariantEnforcer {
        InvariantEnforcer::new(InvariantConfig::default())
    }

    #[test]
    fn test_no_partial_execution_pass() {
        let enforcer = default_enforcer();
        let result = enforcer.check_no_partial_execution(2, 2, IntentState::Finalized);
        assert_eq!(result, InvariantCheckResult::Pass);
    }

    #[test]
    fn test_no_partial_execution_fail() {
        let enforcer = default_enforcer();
        let result = enforcer.check_no_partial_execution(2, 1, IntentState::Finalized);
        assert_eq!(
            result,
            InvariantCheckResult::Fail(InvariantViolationType::PartialExecution)
        );
    }

    #[test]
    fn test_btc_release_without_confirmation() {
        let enforcer = default_enforcer();
        let result = enforcer.check_btc_release_requires_x3(true, false, 6);
        assert_eq!(
            result,
            InvariantCheckResult::Fail(InvariantViolationType::BtcReleaseWithoutConfirmation)
        );
    }

    #[test]
    fn test_btc_release_insufficient_confirmations() {
        let enforcer = default_enforcer();
        let result = enforcer.check_btc_release_requires_x3(true, true, 3);
        assert_eq!(
            result,
            InvariantCheckResult::Fail(InvariantViolationType::BtcReleaseWithoutConfirmation)
        );
    }

    #[test]
    fn test_btc_release_valid() {
        let enforcer = default_enforcer();
        let result = enforcer.check_btc_release_requires_x3(true, true, 6);
        assert_eq!(result, InvariantCheckResult::Pass);
    }

    #[test]
    fn test_reentrancy_detection() {
        let enforcer = default_enforcer();

        // Normal execution: EVM -> SVM -> exit SVM -> exit EVM
        let normal_trace = vec![
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
            VmExecutionEvent::Exit(VmType::Svm),
            VmExecutionEvent::Exit(VmType::Evm),
        ];
        assert_eq!(
            enforcer.check_reentrancy(&normal_trace),
            InvariantCheckResult::Pass
        );

        // Reentrancy: EVM -> SVM -> EVM (reentering)
        let reentrant_trace = vec![
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
            VmExecutionEvent::Enter(VmType::Evm), // Reentrant!
        ];
        assert_eq!(
            enforcer.check_reentrancy(&reentrant_trace),
            InvariantCheckResult::Fail(InvariantViolationType::CrossVmReentrancy)
        );
    }
}
