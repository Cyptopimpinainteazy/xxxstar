//! X3 Instruction Set for cross-chain intent execution.
//!
//! The compiler produces an ordered `Vec<X3Instruction>` from a
//! [`CrossChainIntent`]. Each instruction is an atomic, typed operation.
//! Instructions are the bridge between user intent and X3 runtime dispatch.
//!
//! ## Instruction Design Principles
//!
//! - Every instruction is self-contained (carries all parameters it needs).
//! - Instructions are append-only in a plan — no backpatching.
//! - Every bridge/lock/mint instruction has a corresponding proof instruction.
//! - The runtime must never skip a proof instruction.
//! - Instructions are serializable for storage, replay, and dispute evidence.
//!
//! ## Execution Plan Example (USDC → SOL bridge + swap)
//!
//! ```text
//! 1. ValidateWalletOwner { owner: "alice.eth", chain: Ethereum }
//! 2. CheckBalance        { asset: eth.USDC, owner: "alice.eth", required: 500 }
//! 3. QuoteBestRoute      { from: eth.USDC, to: sol.SOL, amount: 500 }
//! 4. LockAsset           { asset: eth.USDC, amount: 500, contract: "0x..." }
//! 5. WaitFinality        { chain: Ethereum, level: Confirmations(12) }
//! 6. VerifyProof         { label: "eth.lock_event", chain: Ethereum }
//! 7. CheckCanonicalSupply{ asset: x3.USDC.e }
//! 8. MintCanonical       { canonical: x3.USDC.e, amount: 500, to: "alice.x3" }
//! 9. ExecuteSwap         { from: x3.USDC.e, to: x3.SOL, slippage_bps: 50 }
//! 10. BridgeToDestination{ asset: x3.SOL, to: "alice.sol", chain: Solana }
//! 11. WaitFinality        { chain: Solana, level: Finalized }
//! 12. VerifyProof         { label: "sol.release_receipt", chain: Solana }
//! 13. EmitIntentReceipt   { intent_id: ..., verbose: true }
//! ```

use crate::types::{AssetRef, ChainKind, FinalityLevel};
use serde::{Deserialize, Serialize};

/// A single typed instruction in the cross-chain execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum X3Instruction {
    // ── Pre-flight ──────────────────────────────────────────────────────────

    /// Verify that `owner` controls the address on `chain`.
    /// Prevents unauthorized drain of source funds.
    ValidateWalletOwner {
        owner: String,
        chain: ChainKind,
    },

    /// Check that `owner` has at least `required` units of `asset`.
    CheckBalance {
        asset: AssetRef,
        owner: String,
        required: u128,
    },

    /// Request a route quote from the DEX/bridge router.
    /// Produces a route plan that subsequent instructions reference.
    QuoteBestRoute {
        from: AssetRef,
        to: AssetRef,
        amount: u128,
        objective: String,
    },

    /// Simulate the full execution plan. Produces fee estimate, slippage
    /// estimate, liquidity depth, and risk score. Execution is gated on
    /// simulation success when `require_route_simulated` is set.
    SimulateExecution {
        intent_id: u64,
    },

    // ── Asset lifecycle ─────────────────────────────────────────────────────

    /// Lock `amount` of `asset` in `contract` on the source chain.
    /// This is the first irreversible step — proof required before proceeding.
    LockAsset {
        asset: AssetRef,
        amount: u128,
        from_address: String,
        contract: String,
    },

    /// Wait until the source chain satisfies the required finality level.
    /// Execution blocks here until the finality oracle confirms.
    WaitFinality {
        chain: ChainKind,
        level: FinalityLevel,
        block_or_slot: Option<u64>,
    },

    /// Verify a proof requirement before proceeding.
    /// The runtime MUST NOT skip this instruction.
    VerifyProof {
        label: String,
        chain: ChainKind,
        /// The kind of proof (EventProof, MerkleProof, etc.) as a debug string.
        kind_tag: String,
    },

    /// Verify the canonical supply invariant:
    /// `total_wrapped <= total_locked_on_external_chains`.
    /// Halts the bridge if invariant is violated.
    CheckCanonicalSupply {
        wrapped_asset: AssetRef,
    },

    /// Mint `amount` of the canonical wrapped asset on X3.
    MintCanonical {
        canonical_asset: AssetRef,
        amount: u128,
        to: String,
        /// Reference to the proof that authorizes this mint.
        proof_label: String,
    },

    /// Execute a swap on a DEX/AMM.
    ExecuteSwap {
        from: AssetRef,
        to: AssetRef,
        amount_in: u128,
        min_amount_out: u128,
        slippage_bps: u32,
        venue: String,
    },

    /// Submit the destination chain release (bridge out from X3).
    BridgeToDestination {
        asset: AssetRef,
        amount: u128,
        to_address: String,
        dest_chain: ChainKind,
    },

    /// Burn canonical wrapped asset on X3 (initiates release on source chain).
    BurnCanonical {
        canonical_asset: AssetRef,
        amount: u128,
        from: String,
    },

    /// Release the asset on the destination chain.
    ReleaseDestination {
        asset: AssetRef,
        amount: u128,
        to_address: String,
    },

    // ── Verification ─────────────────────────────────────────────────────────

    /// Verify that the final receipt on the destination chain is valid.
    VerifyFinalReceipt {
        chain: ChainKind,
        expected_asset: AssetRef,
        expected_min_amount: u128,
        to_address: String,
    },

    // ── Completion ───────────────────────────────────────────────────────────

    /// Emit the final cross-chain intent receipt.
    /// Visible on the X3 explorer and stored on-chain.
    EmitIntentReceipt {
        intent_id: u64,
        verbose: bool,
    },

    // ── Failure handling ─────────────────────────────────────────────────────

    /// Register a timeout watchdog for this intent.
    /// The watchdog will trigger `on_timeout_action` if not cancelled.
    RegisterTimeoutWatchdog {
        intent_id: u64,
        timeout_secs: u64,
        on_timeout_action: TimeoutAction,
    },

    /// Execute the refund path. Called by the watchdog or on explicit failure.
    ExecuteRefund {
        intent_id: u64,
        action: RefundAction,
    },

    /// Quarantine intent funds for manual security council review.
    Quarantine {
        intent_id: u64,
        reason: String,
    },
}

impl X3Instruction {
    /// True if this instruction requires a proof to be verified before it.
    /// Used by the compiler to ensure proof ordering is correct.
    pub fn requires_prior_proof(&self) -> bool {
        matches!(
            self,
            X3Instruction::MintCanonical { .. }
                | X3Instruction::ReleaseDestination { .. }
                | X3Instruction::BurnCanonical { .. }
        )
    }

    /// True if this instruction is reversible (refund path can undo it).
    pub fn is_reversible(&self) -> bool {
        matches!(
            self,
            X3Instruction::ValidateWalletOwner { .. }
                | X3Instruction::CheckBalance { .. }
                | X3Instruction::QuoteBestRoute { .. }
                | X3Instruction::SimulateExecution { .. }
                | X3Instruction::WaitFinality { .. }
                | X3Instruction::VerifyProof { .. }
                | X3Instruction::CheckCanonicalSupply { .. }
        )
    }

    /// Short human-readable label for explorer and diagnostics.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ValidateWalletOwner { .. } => "ValidateOwner",
            Self::CheckBalance { .. } => "CheckBalance",
            Self::QuoteBestRoute { .. } => "QuoteRoute",
            Self::SimulateExecution { .. } => "Simulate",
            Self::LockAsset { .. } => "LockAsset",
            Self::WaitFinality { .. } => "WaitFinality",
            Self::VerifyProof { .. } => "VerifyProof",
            Self::CheckCanonicalSupply { .. } => "CheckCanonicalSupply",
            Self::MintCanonical { .. } => "MintCanonical",
            Self::ExecuteSwap { .. } => "ExecuteSwap",
            Self::BridgeToDestination { .. } => "BridgeToDestination",
            Self::BurnCanonical { .. } => "BurnCanonical",
            Self::ReleaseDestination { .. } => "ReleaseDestination",
            Self::VerifyFinalReceipt { .. } => "VerifyFinalReceipt",
            Self::EmitIntentReceipt { .. } => "EmitReceipt",
            Self::RegisterTimeoutWatchdog { .. } => "RegisterWatchdog",
            Self::ExecuteRefund { .. } => "ExecuteRefund",
            Self::Quarantine { .. } => "Quarantine",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Supporting enums for failure-path instructions
// ─────────────────────────────────────────────────────────────────────────────

/// Action to take when the watchdog timer fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeoutAction {
    RefundSource,
    RefundX3 { asset: AssetRef, to: String },
    Quarantine,
}

/// Concrete refund action to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefundAction {
    ReturnToSource {
        asset: AssetRef,
        amount: u128,
        to: String,
    },
    ReturnX3Canonical {
        asset: AssetRef,
        amount: u128,
        to: String,
    },
    Quarantine,
}
