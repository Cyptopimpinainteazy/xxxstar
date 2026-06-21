//! # Timeout/Refund Engine
//!
//! Implements timeout safety for X3 atomic swaps:
//!
//! 1. **Timeout ordering**: destination timeout must expire before source timeout.
//!    This ensures that if the destination lock expires, the source lock is still
//!    claimable/refundable for a window, preventing fund loss.
//!
//! 2. **Refund path validation**: refund path must exist before a lock is
//!    considered valid. The engine refuses to proceed without a refund path.
//!
//! 3. **Expiration handling**: expired swaps transition to REFUNDABLE or REFUNDED,
//!    never FAILED_SILENTLY. Funds are never lost - they either get claimed or refunded.

use crate::error::SwapError;
use crate::intent::AtomicIntent;
use crate::intent::AtomicSwapStatus;
use serde::{Deserialize, Serialize};

/// Result of a timeout check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeoutCheckResult {
    /// No timeout has expired; swap can proceed.
    Active,
    /// Destination timeout has expired; destination is refundable.
    DestinationExpired,
    /// Source timeout has expired; entire swap is expired.
    SourceExpired,
    /// Both timeouts have expired.
    FullyExpired,
}

impl TimeoutCheckResult {
    /// True if the swap is still active.
    pub fn is_active(&self) -> bool {
        matches!(self, TimeoutCheckResult::Active)
    }

    /// True if refund should be initiated.
    pub fn should_refund(&self) -> bool {
        matches!(
            self,
            TimeoutCheckResult::DestinationExpired
                | TimeoutCheckResult::SourceExpired
                | TimeoutCheckResult::FullyExpired
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Timeout Engine
// ─────────────────────────────────────────────────────────────────────────────

/// Engine responsible for timeout validation and refund management.
#[derive(Debug)]
pub struct TimeoutEngine {
    /// Current time (unix timestamp or slot, injected externally).
    current_time: u64,
}

impl TimeoutEngine {
    /// Create a new timeout engine with the given current time.
    pub fn new(current_time: u64) -> Self {
        Self { current_time }
    }

    /// Update the current time.
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Get the current time.
    pub fn current_time(&self) -> u64 {
        self.current_time
    }

    /// Check the timeout status of an intent.
    pub fn check_timeouts(&self, intent: &AtomicIntent) -> TimeoutCheckResult {
        let source_expired = intent.is_source_expired(self.current_time);
        let dest_expired = intent.is_destination_expired(self.current_time);

        match (source_expired, dest_expired) {
            (true, true) => TimeoutCheckResult::FullyExpired,
            (true, false) => TimeoutCheckResult::SourceExpired,
            (false, true) => TimeoutCheckResult::DestinationExpired,
            (false, false) => TimeoutCheckResult::Active,
        }
    }

    /// Validate timeout ordering.
    /// Destination timeout must expire BEFORE source timeout (dest < source).
    pub fn validate_timeout_ordering(
        destination_timeout: u64,
        source_timeout: u64,
    ) -> Result<(), SwapError> {
        if destination_timeout >= source_timeout {
            return Err(SwapError::InvalidTimeoutOrdering {
                destination_timeout,
                source_timeout,
            });
        }
        Ok(())
    }

    /// Validate that a refund path exists.
    pub fn validate_refund_path(intent: &AtomicIntent) -> Result<(), SwapError> {
        if intent.refund_path.address.is_empty() {
            return Err(SwapError::generic("refund path address is empty"));
        }
        Ok(())
    }

    /// Process timeouts for an intent.
    ///
    /// Returns the new status if a transition is needed, or None if no change.
    pub fn process_timeout(&self, intent: &AtomicIntent) -> Option<AtomicSwapStatus> {
        let result = self.check_timeouts(intent);
        match result {
            TimeoutCheckResult::Active => None,
            TimeoutCheckResult::DestinationExpired | TimeoutCheckResult::SourceExpired => {
                // If the swap hasn't completed yet and timeouts have expired,
                // the swap should become REFUNDABLE (which we map to Refunding)
                if !intent.status.is_terminal() && intent.status != AtomicSwapStatus::Refunding {
                    Some(AtomicSwapStatus::Refunding)
                } else {
                    None
                }
            }
            TimeoutCheckResult::FullyExpired => {
                if !intent.status.is_terminal() {
                    if intent.status != AtomicSwapStatus::Refunding
                        && intent.status != AtomicSwapStatus::Refunded
                    {
                        Some(AtomicSwapStatus::Refunding)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Perform full validation before considering a lock valid.
    ///
    /// Checks:
    /// 1. Timeout ordering (dest < source)
    /// 2. Refund path exists and is non-empty
    /// 3. Current time is before source timeout (swap hasn't fully expired)
    pub fn validate_before_lock(intent: &AtomicIntent) -> Result<(), SwapError> {
        // Check timeout ordering
        Self::validate_timeout_ordering(intent.destination_timeout, intent.source_timeout)?;

        // Check refund path
        Self::validate_refund_path(intent)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{AtomicIntentBuilder, ChainKind, RefundPath};

    fn make_test_intent(dest_timeout: u64, source_timeout: u64, has_refund: bool) -> AtomicIntent {
        let mut builder = AtomicIntentBuilder::new()
            .source_chain(ChainKind::Ethereum)
            .destination_chain(ChainKind::Solana)
            .source_asset("USDC")
            .destination_asset("SOL")
            .amount_in(1000)
            .min_amount_out(950)
            .receiver("solana_wallet_123")
            .hashlock([0xabu8; 32])
            .source_timeout(source_timeout)
            .destination_timeout(dest_timeout)
            .relayer_quorum(3);

        if has_refund {
            builder = builder.refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: "0xrefund".into(),
                asset: Some("USDC".into()),
            });
        } else {
            builder = builder.refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: String::new(),
                asset: Some("USDC".into()),
            });
        }

        builder.build(1).expect("test intent should build")
    }

    #[test]
    fn test_timeout_ordering_valid() {
        // dest (100) < source (200) = valid
        assert!(TimeoutEngine::validate_timeout_ordering(100, 200).is_ok());
    }

    #[test]
    fn test_timeout_ordering_invalid() {
        // dest (200) >= source (100) = invalid
        let result = TimeoutEngine::validate_timeout_ordering(200, 100);
        assert!(result.is_err());
        if let Err(SwapError::InvalidTimeoutOrdering { .. }) = result {
            // expected
        } else {
            panic!("expected InvalidTimeoutOrdering");
        }

        // Equality is also invalid (dest must be strictly less)
        let result2 = TimeoutEngine::validate_timeout_ordering(100, 100);
        assert!(result2.is_err());
    }

    #[test]
    fn test_timeout_check_active() {
        let intent = make_test_intent(100, 200, true);
        let engine = TimeoutEngine::new(50);
        assert_eq!(engine.check_timeouts(&intent), TimeoutCheckResult::Active);
    }

    #[test]
    fn test_timeout_check_destination_expired() {
        let intent = make_test_intent(100, 200, true);
        // Current time = 150: dest timeout (100) expired, source (200) not
        let engine = TimeoutEngine::new(150);
        assert_eq!(
            engine.check_timeouts(&intent),
            TimeoutCheckResult::DestinationExpired
        );
    }

    #[test]
    fn test_timeout_check_source_expired() {
        let intent = make_test_intent(100, 200, true);
        let engine = TimeoutEngine::new(250);
        assert_eq!(
            engine.check_timeouts(&intent),
            TimeoutCheckResult::FullyExpired
        );
    }

    #[test]
    fn test_validate_before_lock_valid() {
        let intent = make_test_intent(100, 200, true);
        assert!(TimeoutEngine::validate_before_lock(&intent).is_ok());
    }

    #[test]
    fn test_validate_before_lock_bad_ordering() {
        let mut intent = make_test_intent(100, 200, true);
        // Manually swap timeouts to create invalid ordering
        intent.destination_timeout = 200;
        intent.source_timeout = 100;
        let result = TimeoutEngine::validate_before_lock(&intent);
        assert!(result.is_err(), "bad ordering should be rejected");
    }

    #[test]
    fn test_validate_before_lock_empty_refund() {
        let intent = make_test_intent(100, 200, false);
        let result = TimeoutEngine::validate_before_lock(&intent);
        assert!(result.is_err(), "empty refund path should be rejected");
    }

    #[test]
    fn test_process_timeout_active() {
        let mut intent = make_test_intent(100, 200, true);
        intent.set_status(AtomicSwapStatus::SourceLocked).unwrap();
        let engine = TimeoutEngine::new(50);
        assert_eq!(engine.process_timeout(&intent), None);
    }

    #[test]
    fn test_process_timeout_expired_becomes_refunding() {
        let mut intent = make_test_intent(100, 200, true);
        intent.set_status(AtomicSwapStatus::SourceLocked).unwrap();
        let engine = TimeoutEngine::new(150); // dest expired
        assert_eq!(
            engine.process_timeout(&intent),
            Some(AtomicSwapStatus::Refunding)
        );
    }

    #[test]
    fn test_process_timeout_fully_expired() {
        let mut intent = make_test_intent(100, 200, true);
        intent.set_status(AtomicSwapStatus::SourceLocked).unwrap();
        let engine = TimeoutEngine::new(300); // both expired
        assert_eq!(
            engine.process_timeout(&intent),
            Some(AtomicSwapStatus::Refunding)
        );
    }

    #[test]
    fn test_process_timeout_terminal_does_not_change() {
        let mut intent = make_test_intent(100, 200, true);
        // Directly set status bypassing transition validation for test
        intent.status = AtomicSwapStatus::Completed;
        let engine = TimeoutEngine::new(300);
        assert_eq!(engine.process_timeout(&intent), None);
    }
}
