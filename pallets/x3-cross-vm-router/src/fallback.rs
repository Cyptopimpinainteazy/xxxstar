//! Cross-VM router fallback: handles failures in cross-VM execution.
//!
//! When a cross-VM call fails (EVM → X3VM or SVM → EVM), the fallback handler
//! determines whether to retry, abort and refund, or route to an alternative path.

use frame_support::pallet_prelude::*;

/// Destination VM for a cross-VM call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum TargetVm {
    X3Vm,
    Evm,
    Svm,
}

/// Reason a cross-VM call failed.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum CrossVmFailureReason {
    /// Target VM rejected the call (e.g. out of gas).
    Rejected,
    /// Target VM is not available (halted or offline).
    Unavailable,
    /// Invariant check failed after execution.
    InvariantViolation,
    /// The Comit timed out before the counterpart executed.
    Timeout,
    /// Replay protection triggered.
    ReplayDetected,
}

/// Fallback action to take after a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FallbackAction {
    /// Abort the entire Comit and refund gas.
    AbortAndRefund,
    /// Retry the call on the same VM.
    Retry { max_attempts: u8 },
    /// Route to an alternative VM.
    Reroute { alternative: TargetVm },
    /// Escalate to governance for manual resolution.
    EscalateToGovernance,
}

/// Cross-VM fallback policy configuration.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct FallbackPolicy {
    /// Maximum retry attempts before aborting.
    pub max_retries: u8,
    /// Whether to allow rerouting to alternative VMs.
    pub allow_reroute: bool,
    /// Whether unavailability should escalate to governance.
    pub escalate_on_unavailable: bool,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            allow_reroute: false,
            escalate_on_unavailable: false,
        }
    }
}

/// Determine the fallback action for a given failure.
pub fn determine_fallback(
    policy: &FallbackPolicy,
    reason: &CrossVmFailureReason,
    attempt: u8,
) -> FallbackAction {
    match reason {
        CrossVmFailureReason::Rejected => {
            if attempt < policy.max_retries {
                FallbackAction::Retry {
                    max_attempts: policy.max_retries,
                }
            } else {
                FallbackAction::AbortAndRefund
            }
        }
        CrossVmFailureReason::Unavailable => {
            if policy.escalate_on_unavailable {
                FallbackAction::EscalateToGovernance
            } else if policy.allow_reroute {
                FallbackAction::Reroute {
                    alternative: TargetVm::X3Vm,
                }
            } else {
                FallbackAction::AbortAndRefund
            }
        }
        CrossVmFailureReason::InvariantViolation => FallbackAction::AbortAndRefund,
        CrossVmFailureReason::Timeout => FallbackAction::AbortAndRefund,
        CrossVmFailureReason::ReplayDetected => FallbackAction::AbortAndRefund,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_policy() -> FallbackPolicy {
        FallbackPolicy::default()
    }

    #[test]
    fn test_rejected_retries_first() {
        let policy = default_policy();
        let action = determine_fallback(&policy, &CrossVmFailureReason::Rejected, 0);
        assert_eq!(action, FallbackAction::Retry { max_attempts: 2 });
    }

    #[test]
    fn test_rejected_aborts_after_max_retries() {
        let policy = default_policy();
        let action = determine_fallback(&policy, &CrossVmFailureReason::Rejected, 2);
        assert_eq!(action, FallbackAction::AbortAndRefund);
    }

    #[test]
    fn test_unavailable_aborts_by_default() {
        let policy = default_policy();
        let action = determine_fallback(&policy, &CrossVmFailureReason::Unavailable, 0);
        assert_eq!(action, FallbackAction::AbortAndRefund);
    }

    #[test]
    fn test_unavailable_escalates_to_governance() {
        let policy = FallbackPolicy {
            escalate_on_unavailable: true,
            ..Default::default()
        };
        let action = determine_fallback(&policy, &CrossVmFailureReason::Unavailable, 0);
        assert_eq!(action, FallbackAction::EscalateToGovernance);
    }

    #[test]
    fn test_invariant_violation_always_aborts() {
        let policy = FallbackPolicy {
            max_retries: 10,
            allow_reroute: true,
            ..Default::default()
        };
        let action = determine_fallback(&policy, &CrossVmFailureReason::InvariantViolation, 0);
        assert_eq!(action, FallbackAction::AbortAndRefund);
    }

    #[test]
    fn test_replay_detected_always_aborts() {
        let policy = FallbackPolicy {
            max_retries: 10,
            ..Default::default()
        };
        let action = determine_fallback(&policy, &CrossVmFailureReason::ReplayDetected, 0);
        assert_eq!(action, FallbackAction::AbortAndRefund);
    }

    #[test]
    fn test_reroute_when_allowed() {
        let policy = FallbackPolicy {
            allow_reroute: true,
            ..Default::default()
        };
        let action = determine_fallback(&policy, &CrossVmFailureReason::Unavailable, 0);
        assert!(matches!(action, FallbackAction::Reroute { .. }));
    }
}
