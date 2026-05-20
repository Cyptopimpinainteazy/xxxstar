//! Error types for the cross-chain intent system.
//!
//! All errors carry diagnostic codes in the format `X3-INTENT-NNN`.
//! The codes are stable — they can be referenced in user-facing messages,
//! docs, and tooling without breaking across releases.
//!
//! ## Error Code Registry
//!
//! | Code             | Meaning                                                  |
//! |------------------|----------------------------------------------------------|
//! | X3-INTENT-001    | Missing finality requirement for a bridge operation      |
//! | X3-INTENT-002    | Missing proof requirement for a bridge/mint/release      |
//! | X3-INTENT-003    | Missing timeout specification                            |
//! | X3-INTENT-004    | Missing refund/recovery path                             |
//! | X3-INTENT-005    | Missing receiver validation                              |
//! | X3-INTENT-006    | Missing slippage guard for swap operation                |
//! | X3-INTENT-007    | Missing fee cap                                          |
//! | X3-INTENT-008    | Unknown or unsupported chain identifier                  |
//! | X3-INTENT-009    | Unknown or ambiguous asset ticker                        |
//! | X3-INTENT-010    | Unsafe or unverified bridge venue                        |
//! | X3-INTENT-011    | Missing canonical supply check for bridge mint           |
//! | X3-INTENT-012    | Ambiguous decimal precision for asset amount             |
//! | X3-INTENT-013    | Unbounded execution (no fee cap + no timeout = rejected) |

use crate::lifecycle::CrossChainIntentState;
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// Compile-time errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors produced by [`IntentCompiler`] when validating a [`CrossChainIntent`].
///
/// These are rejected before any execution begins. The user must fix their
/// intent declaration and resubmit.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum IntentCompileError {
    /// X3-INTENT-001: Bridge operation has no finality requirement for the source chain.
    ///
    /// ```text
    /// ERROR X3-INTENT-001:
    /// Bridge from 'eth.USDC' requires a finality requirement for chain 'eth'.
    ///
    /// Add:
    ///   require finality eth >= 12
    /// ```
    #[error(
        "X3-INTENT-001: Bridge from '{asset}' on chain '{chain}' requires a finality requirement.\n\
         Add: require finality {chain} >= {min_confirmations}"
    )]
    MissingFinality {
        chain: String,
        asset: String,
        min_confirmations: u32,
    },

    /// X3-INTENT-002: Bridge/mint/release step has no corresponding proof requirement.
    ///
    /// ```text
    /// ERROR X3-INTENT-002:
    /// Bridge step 'LockAsset' requires a proof requirement.
    ///
    /// Add:
    ///   require proof eth.lock_event
    /// ```
    #[error(
        "X3-INTENT-002: Bridge/mint step '{step}' requires a proof requirement for chain '{chain}'.\n\
         Add: require proof {chain}.lock_event"
    )]
    MissingProof { step: String, chain: String },

    /// X3-INTENT-003: No timeout specified.
    ///
    /// ```text
    /// ERROR X3-INTENT-003:
    /// Cross-chain intent requires a timeout specification.
    ///
    /// Add:
    ///   timeout 30m
    /// ```
    #[error(
        "X3-INTENT-003: Cross-chain intent requires a timeout.\n\
         Add: timeout 30m"
    )]
    MissingTimeout,

    /// X3-INTENT-004: No refund or recovery path specified.
    ///
    /// ```text
    /// ERROR X3-INTENT-004:
    /// Cross-chain bridge requires timeout and refund path.
    ///
    /// Add:
    ///   timeout 30m
    ///   on_timeout refund source
    /// ```
    #[error(
        "X3-INTENT-004: Cross-chain bridge requires a refund/recovery path.\n\
         Add: on_timeout refund source"
    )]
    MissingRefundPath,

    /// X3-INTENT-005: Receiver is not validated against wallet owner.
    #[error(
        "X3-INTENT-005: Receiver address is not validated against wallet owner.\n\
         Add: require receiver == wallet.owner"
    )]
    MissingReceiverValidation,

    /// X3-INTENT-006: Swap step has no slippage guard.
    ///
    /// ```text
    /// ERROR X3-INTENT-006:
    /// Swap from 'eth.USDC' to 'sol.SOL' requires a slippage guard.
    ///
    /// Add:
    ///   require slippage <= 0.5%
    /// ```
    #[error(
        "X3-INTENT-006: Swap from '{from}' to '{to}' requires a slippage guard.\n\
         Add: require slippage <= 0.5%"
    )]
    MissingSlippageGuard { from: String, to: String },

    /// X3-INTENT-007: No fee cap declared.
    ///
    /// ```text
    /// ERROR X3-INTENT-007:
    /// Intent has no maximum fee cap. Without a fee cap, the system may
    /// charge unbounded fees in adverse conditions.
    ///
    /// Add:
    ///   require max_fee <= 10 USDC
    /// ```
    #[error(
        "X3-INTENT-007: Intent has no maximum fee cap.\n\
         Add: require max_fee <= 10 {asset}"
    )]
    MissingFeeCap { asset: String },

    /// X3-INTENT-008: Chain identifier is not supported.
    #[error(
        "X3-INTENT-008: Unknown or unsupported chain identifier '{chain}'.\n\
         Supported: eth, sol, btc, x3, base, arb, op, bsc, poly, avax, cosmos"
    )]
    UnknownChain { chain: String },

    /// X3-INTENT-009: Asset ticker is unknown or ambiguous.
    #[error(
        "X3-INTENT-009: Unknown or ambiguous asset '{asset}' on chain '{chain}'.\n\
         Use the fully-qualified form: chain.SYMBOL (e.g. eth.USDC)"
    )]
    UnknownAsset { asset: String, chain: String },

    /// X3-INTENT-010: Unsafe or unverified bridge venue in the route.
    #[error(
        "X3-INTENT-010: Unsafe bridge venue '{venue}' in route.\n\
         Use only verified bridges or add: deny bridge.unknown"
    )]
    UnsafeRoute { venue: String },

    /// X3-INTENT-011: Bridge mint step missing canonical supply check.
    #[error(
        "X3-INTENT-011: Mint of canonical asset '{asset}' is missing a supply invariant check.\n\
         Add: require canonical_supply_valid"
    )]
    MissingCanonicalSupplyCheck { asset: String },

    /// X3-INTENT-012: Decimal precision is ambiguous for the specified amount.
    #[error(
        "X3-INTENT-012: Decimal precision for '{asset}' amount '{amount}' is ambiguous.\n\
         Specify amounts in base units (smallest denomination) or provide explicit decimals."
    )]
    AmbiguousDecimals { asset: String, amount: String },

    /// X3-INTENT-013: Intent is unbounded — no fee cap AND no timeout.
    #[error(
        "X3-INTENT-013: Intent has neither a fee cap nor a timeout. Unbounded execution is rejected.\n\
         Add both: require max_fee <= N  and  timeout 30m"
    )]
    UnboundedExecution,

    /// Finality level is below the chain's safe minimum.
    #[error(
        "Finality requirement for '{chain}' is {specified} confirmations, \
         which is below the safe minimum of {minimum}. Bridge goblins get fed here."
    )]
    InsufficientFinality {
        chain: String,
        specified: u32,
        minimum: u32,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime/validation errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors produced during intent lifecycle management and state machine transitions.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum IntentValidationError {
    /// Attempted to transition an intent that is already in a terminal state.
    #[error(
        "Intent is already in terminal state '{state:?}'. No further transitions are valid."
    )]
    AlreadyTerminal { state: CrossChainIntentState },

    /// The requested state transition is not valid from the current state.
    #[error(
        "Invalid intent state transition: {from:?} → {to:?}."
    )]
    InvalidTransition {
        from: CrossChainIntentState,
        to: CrossChainIntentState,
    },

    /// Intent hash mismatch — the stored hash does not match recomputed.
    #[error("Intent hash mismatch. Intent may have been tampered with.")]
    HashMismatch,

    /// Intent has expired before completing execution.
    #[error("Intent {intent_id} expired at block/time {expired_at}.")]
    Expired { intent_id: u64, expired_at: u64 },

    /// A required proof is missing at the time it was needed.
    #[error("Required proof '{label}' for chain '{chain}' is not present.")]
    MissingProofAtRuntime { label: String, chain: String },

    /// Slippage exceeded the user's declared limit.
    #[error(
        "Slippage {actual_bps}bps exceeds the declared limit of {limit_bps}bps. \
         Execution halted."
    )]
    SlippageExceeded { actual_bps: u32, limit_bps: u32 },

    /// Fee exceeded the user's declared cap.
    #[error(
        "Estimated fee {estimated} exceeds declared max_fee {cap}. \
         Execution halted."
    )]
    FeeCapExceeded { estimated: u128, cap: u128 },

    /// Canonical supply invariant was violated.
    #[error(
        "Canonical supply invariant violated for asset '{asset}': \
         wrapped {wrapped} > locked {locked}. Bridge halted."
    )]
    CanonicalSupplyViolation {
        asset: String,
        wrapped: u128,
        locked: u128,
    },
}
