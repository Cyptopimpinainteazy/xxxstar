//! VM revert mechanism: atomic rollback of state, events, and balances.
//!
//! When a cross-VM Comit aborts (or a nested call fails), all state changes
//! made within that scope must be fully undone. This module ties together
//! the storage rollback, event buffer discard, and gas accounting reset.

/// Revert reason codes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevertReason {
    /// Explicit revert instruction in bytecode.
    ExplicitRevert,
    /// Gas exhausted during execution.
    GasExhausted,
    /// Invalid opcode or decode failure.
    InvalidInstruction,
    /// Stack overflow.
    StackOverflow,
    /// Division by zero.
    DivisionByZero,
    /// Cross-VM call failed in the counterpart VM.
    CrossVmCallFailed,
    /// Invariant check failed (e.g. supply conservation).
    InvariantViolation,
    /// Host rejected the execution (access control).
    HostRejected,
    /// Custom reason with encoded payload.
    Custom(Vec<u8>),
}

impl RevertReason {
    /// Whether this revert was user-initiated (not a VM fault).
    pub fn is_user_revert(&self) -> bool {
        matches!(self, RevertReason::ExplicitRevert | RevertReason::Custom(_))
    }

    /// Whether gas should be fully charged on this revert.
    pub fn charges_full_gas(&self) -> bool {
        matches!(
            self,
            RevertReason::GasExhausted
                | RevertReason::InvariantViolation
                | RevertReason::HostRejected
        )
    }
}

/// The result of a revert operation.
#[derive(Debug)]
pub struct RevertResult {
    pub reason: RevertReason,
    /// Gas consumed before the revert.
    pub gas_consumed: u64,
    /// Optional ABI-encoded revert data (from explicit reverts).
    pub revert_data: Option<Vec<u8>>,
}

/// Build a revert result for an explicit revert instruction.
pub fn explicit_revert(gas_consumed: u64, data: Vec<u8>) -> RevertResult {
    RevertResult {
        reason: RevertReason::ExplicitRevert,
        gas_consumed,
        revert_data: if data.is_empty() { None } else { Some(data) },
    }
}

/// Build a revert result for gas exhaustion.
pub fn gas_exhausted_revert(gas_limit: u64) -> RevertResult {
    RevertResult {
        reason: RevertReason::GasExhausted,
        gas_consumed: gas_limit,
        revert_data: None,
    }
}

/// Build a revert result for a cross-VM call failure.
pub fn cross_vm_failed_revert(gas_consumed: u64) -> RevertResult {
    RevertResult {
        reason: RevertReason::CrossVmCallFailed,
        gas_consumed,
        revert_data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_revert_is_user_revert() {
        let r = explicit_revert(100, vec![0xDE, 0xAD]);
        assert!(r.reason.is_user_revert());
        assert!(!r.reason.charges_full_gas());
        assert_eq!(r.revert_data, Some(vec![0xDE, 0xAD]));
    }

    #[test]
    fn test_explicit_revert_empty_data_is_none() {
        let r = explicit_revert(50, vec![]);
        assert!(r.revert_data.is_none());
    }

    #[test]
    fn test_gas_exhausted_charges_full_gas() {
        let r = gas_exhausted_revert(1_000_000);
        assert!(r.reason.charges_full_gas());
        assert!(!r.reason.is_user_revert());
        assert_eq!(r.gas_consumed, 1_000_000);
    }

    #[test]
    fn test_cross_vm_failed_not_user_revert() {
        let r = cross_vm_failed_revert(500);
        assert!(!r.reason.is_user_revert());
        assert!(!r.reason.charges_full_gas());
    }

    #[test]
    fn test_custom_is_user_revert() {
        let reason = RevertReason::Custom(vec![1, 2, 3]);
        assert!(reason.is_user_revert());
    }

    #[test]
    fn test_invariant_violation_charges_full_gas() {
        let reason = RevertReason::InvariantViolation;
        assert!(reason.charges_full_gas());
    }
}
