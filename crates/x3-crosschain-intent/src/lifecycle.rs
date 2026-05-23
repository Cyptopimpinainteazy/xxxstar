//! Cross-chain intent lifecycle state machine.
//!
//! Every intent starts at [`CrossChainIntentState::Draft`] and progresses
//! through a strictly ordered set of states. Failure branches always end at
//! terminal states. There is no going backward.
//!
//! ## State Diagram
//!
//! ```text
//! Draft
//!   ↓  sign()
//! Signed
//!   ↓  simulate()
//! Simulated
//!   ↓  accept()
//! Accepted
//!   ↓  lock_source()
//! SourceLocked
//!   ↓  confirm_source_finality()
//! SourceFinalized
//!   ↓  verify_proof()
//! ProofVerified
//!   ↓  mint_canonical()
//! CanonicalMinted
//!   ↓  execute_swap()           (skip if no swap)
//! SwapExecuted
//!   ↓  release_destination()
//! DestinationReleasePending
//!   ↓  confirm_destination()
//! Completed
//!
//! Failure branches (reachable from most active states):
//!   → FailedSimulation    (from Simulated)
//!   → Expired             (from any active state)
//!   → Refunding           (from any post-lock state)
//!   → Refunded            (terminal)
//!   → Quarantined         (terminal)
//!   → Disputed            (from Completed or any post-lock state)
//!   → Slashed             (from Disputed, validator fault)
//! ```

use crate::error::IntentValidationError;
use serde::{Deserialize, Serialize};

/// Every possible state an intent can occupy.
///
/// States are grouped logically:
/// - **Happy path** (0–10): normal progression from draft to completion
/// - **Failure states** (11–16): terminal or recovery states
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CrossChainIntentState {
    // ── Happy path ──────────────────────────────────────────────────────────
    /// Intent has been constructed but not yet signed by the user.
    Draft = 0,
    /// User has signed the high-level intent declaration.
    Signed = 1,
    /// Pre-execution simulation completed successfully (fees, routes, risk).
    Simulated = 2,
    /// Simulation passed; intent accepted for execution.
    Accepted = 3,
    /// Source asset has been locked on the source chain.
    SourceLocked = 4,
    /// Source chain finality requirement has been satisfied.
    SourceFinalized = 5,
    /// Required proofs have been collected and verified by X3.
    ProofVerified = 6,
    /// Canonical wrapped asset has been minted on X3.
    CanonicalMinted = 7,
    /// Swap step has been executed (or skipped if not required).
    SwapExecuted = 8,
    /// Destination release has been submitted; awaiting confirmation.
    DestinationReleasePending = 9,
    /// Intent fully completed — all steps verified, receipt emitted.
    Completed = 10,

    // ── Failure / recovery ──────────────────────────────────────────────────
    /// Simulation failed — intent cannot proceed (slippage, liquidity, fee cap).
    FailedSimulation = 11,
    /// Intent expired before completing (timeout elapsed).
    Expired = 12,
    /// Refund process is in progress.
    Refunding = 13,
    /// Intent has been fully refunded. Terminal state.
    Refunded = 14,
    /// Intent is under dispute (user filed challenge or validator fault).
    Disputed = 15,
    /// Validator was slashed as a result of dispute resolution. Terminal state.
    Slashed = 16,
    /// Funds held pending manual security council review. Terminal state.
    Quarantined = 17,
}

impl CrossChainIntentState {
    /// True if this is a terminal state — no further transitions are valid.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            CrossChainIntentState::Completed
                | CrossChainIntentState::FailedSimulation
                | CrossChainIntentState::Refunded
                | CrossChainIntentState::Slashed
                | CrossChainIntentState::Quarantined
        )
    }

    /// True if the intent still has funds at risk (post-lock, pre-completion).
    pub fn has_funds_at_risk(&self) -> bool {
        matches!(
            self,
            CrossChainIntentState::SourceLocked
                | CrossChainIntentState::SourceFinalized
                | CrossChainIntentState::ProofVerified
                | CrossChainIntentState::CanonicalMinted
                | CrossChainIntentState::SwapExecuted
                | CrossChainIntentState::DestinationReleasePending
                | CrossChainIntentState::Refunding
                | CrossChainIntentState::Disputed
                | CrossChainIntentState::Quarantined
        )
    }

    /// Human-readable label for explorer display.
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Signed => "Signed",
            Self::Simulated => "Simulated",
            Self::Accepted => "Accepted — queued for execution",
            Self::SourceLocked => "Source asset locked",
            Self::SourceFinalized => "Source finality reached",
            Self::ProofVerified => "Lock proof verified",
            Self::CanonicalMinted => "Canonical asset minted on X3",
            Self::SwapExecuted => "Swap executed",
            Self::DestinationReleasePending => "Destination release submitted",
            Self::Completed => "Intent completed",
            Self::FailedSimulation => "Simulation failed — not executed",
            Self::Expired => "Expired",
            Self::Refunding => "Refund in progress",
            Self::Refunded => "Refunded",
            Self::Disputed => "Under dispute",
            Self::Slashed => "Validator slashed",
            Self::Quarantined => "Quarantined — awaiting manual review",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// State machine
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime state tracker for a single [`CrossChainIntent`].
///
/// Enforces valid transitions. All transitions are append-only — history is
/// preserved for audit and dispute resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStateMachine {
    pub current: CrossChainIntentState,
    /// Complete transition history, oldest first.
    pub history: Vec<CrossChainIntentState>,
}

impl IntentStateMachine {
    /// Start a new state machine in the Draft state.
    pub fn new() -> Self {
        Self {
            current: CrossChainIntentState::Draft,
            history: vec![CrossChainIntentState::Draft],
        }
    }

    /// Attempt a state transition. Returns the new state on success.
    ///
    /// Callers must hold the lock on the intent registry before calling this
    /// in production code. The state machine itself does not manage locking.
    pub fn transition(
        &mut self,
        to: CrossChainIntentState,
    ) -> Result<CrossChainIntentState, IntentValidationError> {
        if self.current.is_terminal() {
            return Err(IntentValidationError::AlreadyTerminal {
                state: self.current,
            });
        }

        if !Self::is_valid_transition(self.current, to) {
            return Err(IntentValidationError::InvalidTransition {
                from: self.current,
                to,
            });
        }

        self.history.push(to);
        self.current = to;
        Ok(to)
    }

    /// Force a state to a failure/recovery terminal. Used by timeout watchdog
    /// and the security council. Does not validate happy-path ordering.
    pub fn force_failure(
        &mut self,
        to: CrossChainIntentState,
    ) -> Result<(), IntentValidationError> {
        let is_failure_state = matches!(
            to,
            CrossChainIntentState::FailedSimulation
                | CrossChainIntentState::Expired
                | CrossChainIntentState::Refunding
                | CrossChainIntentState::Refunded
                | CrossChainIntentState::Quarantined
                | CrossChainIntentState::Disputed
                | CrossChainIntentState::Slashed
        );

        if !is_failure_state {
            return Err(IntentValidationError::InvalidTransition {
                from: self.current,
                to,
            });
        }

        if self.current.is_terminal() && self.current != CrossChainIntentState::Disputed {
            return Err(IntentValidationError::AlreadyTerminal {
                state: self.current,
            });
        }

        self.history.push(to);
        self.current = to;
        Ok(())
    }

    /// Validate that the `from → to` transition is allowed.
    fn is_valid_transition(from: CrossChainIntentState, to: CrossChainIntentState) -> bool {
        use CrossChainIntentState::*;
        match (from, to) {
            // Happy path
            (Draft, Signed) => true,
            (Signed, Simulated) => true,
            (Signed, FailedSimulation) => true,
            (Simulated, Accepted) => true,
            (Simulated, FailedSimulation) => true,
            (Accepted, SourceLocked) => true,
            (SourceLocked, SourceFinalized) => true,
            (SourceFinalized, ProofVerified) => true,
            (ProofVerified, CanonicalMinted) => true,
            // SwapExecuted may be skipped (no-swap intents go CanonicalMinted → DestinationReleasePending)
            (CanonicalMinted, SwapExecuted) => true,
            (CanonicalMinted, DestinationReleasePending) => true,
            (SwapExecuted, DestinationReleasePending) => true,
            (DestinationReleasePending, Completed) => true,

            // Any active state → Expired (timeout watchdog)
            (s, Expired) if !s.is_terminal() => true,

            // Any post-lock state → Refunding → Refunded
            (s, Refunding) if s.has_funds_at_risk() => true,
            (Refunding, Refunded) => true,
            (Refunding, Quarantined) => true,

            // Dispute flow
            (s, Disputed) if s.has_funds_at_risk() || s == Completed => true,
            (Disputed, Slashed) => true,
            (Disputed, Refunded) => true,
            (Disputed, Quarantined) => true,

            // Quarantine can be reached from many places
            (s, Quarantined) if !s.is_terminal() => true,

            _ => false,
        }
    }
}

impl Default for IntentStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
