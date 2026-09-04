/// Cross-VM Atomic Rollback Event Listener — Tracks failed trade batches and handles compensation/refunds
/// Monitors TradeBatchFailed events across VM boundaries and triggers rollback mechanisms
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TradeBatchFailure {
    pub batch_id: [u8; 32],
    pub initiator: [u8; 32],
    pub failed_leg: u32, // Which leg of the trade failed (0-indexed)
    pub failure_reason: FailureReason,
    pub trade_state: TradeState,
    pub partial_execution_value: u128,
    pub refund_amount: u128,
    pub rollback_status: RollbackStatus,
    pub timestamp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum FailureReason {
    InsufficientLiquidity,
    SlippageExceeded,
    Timeout,
    OracleStaleness,
    ChainBridgeConfirmation,
    InvalidCounterparty,
    SignatureFailure,
    Other,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum TradeState {
    Pending,
    PartiallyExecuted,
    FullyExecuted,
    Failed,
    RolledBack,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum RollbackStatus {
    PendingRollback,
    RollingBack,
    RollbackComplete,
    RollbackFailed,
    ManualIntervention,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct RollbackLog {
    pub batch_id: [u8; 32],
    pub completed_legs: Vec<u32>,
    pub refunds: Vec<(u32, u128)>, // (leg index, refund amount)
    pub compensation_issued: bool,
    pub compensation_amount: u128,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FailureNotification {
    pub notification_id: [u8; 32],
    pub batch_id: [u8; 32],
    pub recipient: [u8; 32],
    pub message: Vec<u8>,
    pub severity: SeverityLevel,
    pub is_read: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum SeverityLevel {
    Info,
    Warning,
    Critical,
}

pub struct RollbackEventListener;

impl RollbackEventListener {
    /// Create failure event record (called when TradeBatchFailed fires)
    pub fn record_failure(
        batch_id: [u8; 32],
        initiator: [u8; 32],
        failed_leg: u32,
        reason: FailureReason,
        partial_value: u128,
        current_timestamp: u64,
    ) -> Result<TradeBatchFailure, &'static str> {
        if batch_id == [0; 32] {
            return Err("Invalid batch ID");
        }
        if initiator == [0; 32] {
            return Err("Invalid initiator");
        }

        Ok(TradeBatchFailure {
            batch_id,
            initiator,
            failed_leg,
            failure_reason: reason,
            trade_state: TradeState::Failed,
            partial_execution_value: partial_value,
            refund_amount: partial_value, // Initial refund = partial execution value
            rollback_status: RollbackStatus::PendingRollback,
            timestamp: current_timestamp,
        })
    }

    /// Initiate rollback sequence for failed batch
    pub fn initiate_rollback(
        failure: &mut TradeBatchFailure,
        executed_legs: Vec<(u32, u128)>,
    ) -> Result<(), &'static str> {
        if failure.rollback_status != RollbackStatus::PendingRollback {
            return Err("Rollback already initiated");
        }

        failure.trade_state = TradeState::PartiallyExecuted;
        failure.rollback_status = RollbackStatus::RollingBack;

        // Refund uses executed leg values, not fixed placeholders.
        // If no executed-leg breakdown is available yet, fall back to partial_execution_value.
        let mut total_refund = 0u128;
        let mut seen_legs: Vec<u32> = Vec::new();

        for (leg_index, executed_value) in executed_legs {
            if seen_legs.iter().any(|seen| *seen == leg_index) {
                return Err("Duplicate executed leg index");
            }
            if executed_value == 0 {
                return Err("Executed leg value must be positive");
            }

            seen_legs.push(leg_index);
            total_refund = total_refund.saturating_add(executed_value);
        }

        if total_refund == 0 {
            total_refund = failure.partial_execution_value;
        }

        failure.partial_execution_value = total_refund;
        failure.refund_amount = total_refund;
        Ok(())
    }

    /// Complete rollback and mark batch as rolled back
    pub fn complete_rollback(
        failure: &mut TradeBatchFailure,
        compensation_factor: f64, // 1.0 = full refund, 1.05 = 5% compensation
    ) -> Result<(), &'static str> {
        if failure.rollback_status != RollbackStatus::RollingBack {
            return Err("Rollback not in progress");
        }

        let compensated_refund = (failure.refund_amount as f64 * compensation_factor) as u128;
        failure.refund_amount = compensated_refund;
        failure.rollback_status = RollbackStatus::RollbackComplete;
        failure.trade_state = TradeState::RolledBack;

        Ok(())
    }

    /// Query failure details for UI display
    pub fn get_failure_details(
        failure: &TradeBatchFailure,
    ) -> (FailureReason, TradeState, u128, RollbackStatus) {
        (
            failure.failure_reason.clone(),
            failure.trade_state.clone(),
            failure.refund_amount,
            failure.rollback_status.clone(),
        )
    }

    /// Create compensation mechanism for affected users
    pub fn issue_compensation(
        failure: &mut TradeBatchFailure,
        compensation_amount: u128,
    ) -> Result<u128, &'static str> {
        if failure.rollback_status != RollbackStatus::RollbackComplete {
            return Err("Cannot issue compensation before rollback completes");
        }
        if compensation_amount == 0 {
            return Err("Compensation must be positive");
        }

        let total_payout = failure.refund_amount.saturating_add(compensation_amount);
        Ok(total_payout)
    }

    /// Create user notification of failure
    pub fn create_notification(
        batch_id: [u8; 32],
        recipient: [u8; 32],
        reason: &FailureReason,
    ) -> Result<FailureNotification, &'static str> {
        if recipient == [0; 32] {
            return Err("Invalid recipient");
        }

        let severity = match reason {
            FailureReason::Other => SeverityLevel::Warning,
            FailureReason::SlippageExceeded => SeverityLevel::Info,
            _ => SeverityLevel::Critical,
        };

        let message = format!(
            "Trade batch {:?} failed due to {:?}. Rollback in progress.",
            &batch_id[0..8],
            reason
        );

        let notification_id = Self::generate_notification_id(&batch_id, &recipient);

        Ok(FailureNotification {
            notification_id,
            batch_id,
            recipient,
            message: message.into_bytes(),
            severity,
            is_read: false,
        })
    }

    /// Log rollback completion with refunds
    pub fn log_rollback(
        batch_id: [u8; 32],
        completed_legs: Vec<u32>,
        refunds: Vec<(u32, u128)>,
        compensation_issued: bool,
        compensation_amount: u128,
    ) -> Result<RollbackLog, &'static str> {
        if batch_id == [0; 32] {
            return Err("Invalid batch ID");
        }

        Ok(RollbackLog {
            batch_id,
            completed_legs,
            refunds,
            compensation_issued,
            compensation_amount,
        })
    }

    /// Check if failure requires manual intervention
    pub fn requires_manual_intervention(failure: &TradeBatchFailure) -> bool {
        matches!(
            failure.failure_reason,
            FailureReason::Other | FailureReason::SignatureFailure
        ) || matches!(failure.rollback_status, RollbackStatus::RollbackFailed)
    }

    /// Mark notification as read
    pub fn mark_notification_read(
        notification: &mut FailureNotification,
    ) -> Result<(), &'static str> {
        notification.is_read = true;
        Ok(())
    }

    /// Handle failure with automatic recovery
    pub fn auto_recover_failure(failure: &mut TradeBatchFailure) -> Result<bool, &'static str> {
        match failure.failure_reason {
            FailureReason::SlippageExceeded => {
                // Auto-refund if slippage exceeded
                failure.rollback_status = RollbackStatus::RollbackComplete;
                Ok(true)
            }
            FailureReason::InsufficientLiquidity => {
                // Flag for manual intervention
                failure.rollback_status = RollbackStatus::ManualIntervention;
                Ok(false)
            }
            _ => {
                failure.rollback_status = RollbackStatus::ManualIntervention;
                Ok(false)
            }
        }
    }

    /// Track partial execution value (how much of the trade completed before failure)
    pub fn update_partial_execution(
        failure: &mut TradeBatchFailure,
        partial_value: u128,
    ) -> Result<(), &'static str> {
        if partial_value > failure.partial_execution_value {
            failure.partial_execution_value = partial_value;
            failure.refund_amount = partial_value;
        }
        Ok(())
    }

    /// Get all failures for a user (for UI dashboard)
    pub fn get_user_failures(
        initiator: [u8; 32],
        failures: &[TradeBatchFailure],
    ) -> Vec<TradeBatchFailure> {
        failures
            .iter()
            .filter(|f| f.initiator == initiator)
            .cloned()
            .collect()
    }

    /// Generate deterministic notification ID
    fn generate_notification_id(batch_id: &[u8; 32], recipient: &[u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        for i in 0..32 {
            id[i] = batch_id[i] ^ recipient[i].wrapping_add(i as u8);
        }
        id
    }

    /// Backward-compatible rollback initiation when only completed leg indices are available.
    /// This path intentionally avoids placeholder increments and uses partial_execution_value.
    pub fn initiate_rollback_legacy(
        failure: &mut TradeBatchFailure,
        _completed_legs: Vec<u32>,
    ) -> Result<(), &'static str> {
        Self::initiate_rollback(failure, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_failure() {
        let failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::InsufficientLiquidity,
            100000,
            1000,
        )
        .unwrap();

        assert_eq!(failure.batch_id, [1; 32]);
        assert_eq!(failure.initiator, [2; 32]);
        assert_eq!(failure.rollback_status, RollbackStatus::PendingRollback);
    }

    #[test]
    fn test_initiate_rollback() {
        let mut failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::SlippageExceeded,
            100000,
            1000,
        )
        .unwrap();

        RollbackEventListener::initiate_rollback(&mut failure, vec![(0, 100000)]).unwrap();
        assert_eq!(failure.rollback_status, RollbackStatus::RollingBack);
        assert_eq!(failure.refund_amount, 100000);
    }

    #[test]
    fn test_complete_rollback() {
        let mut failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::SlippageExceeded,
            100000,
            1000,
        )
        .unwrap();

        RollbackEventListener::initiate_rollback(&mut failure, vec![]).unwrap();
        RollbackEventListener::complete_rollback(&mut failure, 1.05).unwrap();

        assert_eq!(failure.rollback_status, RollbackStatus::RollbackComplete);
        assert!(failure.refund_amount > 100000); // Compensated
    }

    #[test]
    fn test_get_failure_details() {
        let failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::Timeout,
            100000,
            1000,
        )
        .unwrap();

        let (reason, state, refund, status) = RollbackEventListener::get_failure_details(&failure);
        assert_eq!(refund, 100000);
        assert!(matches!(state, TradeState::Failed));
    }

    #[test]
    fn test_issue_compensation() {
        let mut failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::SlippageExceeded,
            100000,
            1000,
        )
        .unwrap();

        RollbackEventListener::initiate_rollback(&mut failure, vec![]).unwrap();
        RollbackEventListener::complete_rollback(&mut failure, 1.0).unwrap();

        let payout = RollbackEventListener::issue_compensation(&mut failure, 5000).unwrap();
        assert!(payout > 100000);
    }

    #[test]
    fn test_create_notification() {
        let notif = RollbackEventListener::create_notification(
            [1; 32],
            [2; 32],
            &FailureReason::SlippageExceeded,
        )
        .unwrap();

        assert_eq!(notif.recipient, [2; 32]);
        assert!(!notif.is_read);
    }

    #[test]
    fn test_log_rollback() {
        let log = RollbackEventListener::log_rollback(
            [1; 32],
            vec![0, 1],
            vec![(0, 50000), (1, 50000)],
            true,
            5000,
        )
        .unwrap();

        assert_eq!(log.completed_legs.len(), 2);
        assert_eq!(log.compensation_amount, 5000);
    }

    #[test]
    fn test_requires_manual_intervention() {
        let failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::Other,
            100000,
            1000,
        )
        .unwrap();

        assert!(RollbackEventListener::requires_manual_intervention(
            &failure
        ));
    }

    #[test]
    fn test_mark_notification_read() {
        let mut notif =
            RollbackEventListener::create_notification([1; 32], [2; 32], &FailureReason::Timeout)
                .unwrap();

        RollbackEventListener::mark_notification_read(&mut notif).unwrap();
        assert!(notif.is_read);
    }

    #[test]
    fn test_auto_recover_slippage() {
        let mut failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::SlippageExceeded,
            100000,
            1000,
        )
        .unwrap();

        let recovered = RollbackEventListener::auto_recover_failure(&mut failure).unwrap();
        assert!(recovered);
    }

    #[test]
    fn test_update_partial_execution() {
        let mut failure = RollbackEventListener::record_failure(
            [1; 32],
            [2; 32],
            1,
            FailureReason::Timeout,
            50000,
            1000,
        )
        .unwrap();

        RollbackEventListener::update_partial_execution(&mut failure, 75000).unwrap();
        assert_eq!(failure.partial_execution_value, 75000);
    }

    #[test]
    fn test_initiate_rollback_sums_executed_leg_values() {
        let mut failure = RollbackEventListener::record_failure(
            [9; 32],
            [8; 32],
            2,
            FailureReason::Timeout,
            50000,
            1000,
        )
        .unwrap();

        RollbackEventListener::initiate_rollback(
            &mut failure,
            vec![(0, 30000), (1, 20000), (3, 10000)],
        )
        .unwrap();

        assert_eq!(failure.refund_amount, 60000);
        assert_eq!(failure.partial_execution_value, 60000);
    }

    #[test]
    fn test_initiate_rollback_rejects_duplicate_leg_index() {
        let mut failure = RollbackEventListener::record_failure(
            [9; 32],
            [8; 32],
            2,
            FailureReason::Timeout,
            50000,
            1000,
        )
        .unwrap();

        let result =
            RollbackEventListener::initiate_rollback(&mut failure, vec![(0, 30000), (0, 20000)]);

        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_rollback_rejects_zero_executed_leg_value() {
        let mut failure = RollbackEventListener::record_failure(
            [9; 32],
            [8; 32],
            2,
            FailureReason::Timeout,
            50000,
            1000,
        )
        .unwrap();

        let result = RollbackEventListener::initiate_rollback(&mut failure, vec![(0, 0)]);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_user_failures() {
        let failures = vec![
            RollbackEventListener::record_failure(
                [1; 32],
                [2; 32],
                0,
                FailureReason::Timeout,
                100000,
                1000,
            )
            .unwrap(),
            RollbackEventListener::record_failure(
                [2; 32],
                [3; 32],
                1,
                FailureReason::SlippageExceeded,
                200000,
                2000,
            )
            .unwrap(),
            RollbackEventListener::record_failure(
                [3; 32],
                [2; 32],
                0,
                FailureReason::InsufficientLiquidity,
                150000,
                3000,
            )
            .unwrap(),
        ];

        let user_failures = RollbackEventListener::get_user_failures([2; 32], &failures);
        assert_eq!(user_failures.len(), 2);
    }
}
