//! # AtomicIntent - Shared atomic swap intent model.
//!
//! Defines the complete intent schema for X3 cross-VM atomic swaps using
//! HTLC-style hashlocks with chain-specific timelocks. Every field is
//! serializable, hash-stable, and enforced by the scoreboard.
//!
//! ## Schema
//!
//! - `intent_id` - unique identifier
//! - `source_chain` / `destination_chain` - ChainKind (eth, sol, btc, x3, etc.)
//! - `source_asset` / `destination_asset` - asset symbol strings
//! - `amount_in` / `min_amount_out` - base units, u128
//! - `receiver` - destination address
//! - `hashlock` - 32-byte hash of the preimage (blake2b/sha256)
//! - `source_timeout` / `destination_timeout` - unix timestamps or slot numbers
//! - `finality_requirements` - per-chain finality rules
//! - `refund_path` - recovery path if swap expires
//! - `route_mode` - routing strategy (DirectHtlc, SolverFill, MultiHop, ...)
//! - `max_slippage_bps` - maximum acceptable slippage in basis points
//! - `relayer_quorum_requirement` - minimum relayer signatures needed
//! - `status` - current lifecycle status

use crate::error::SwapError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Unique intent identifier.
pub type IntentId = u64;

/// Route mode for atomic swap execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteMode {
    DirectHtlc,
    SolverFill,
    MultiHop,
    IbcRoute,
    XcmRoute,
    ProofOnly,
    RefundOnly,
    Disabled,
}

impl RouteMode {
    pub fn name(&self) -> &'static str {
        match self {
            RouteMode::DirectHtlc => "direct_htlc",
            RouteMode::SolverFill => "solver_fill",
            RouteMode::MultiHop => "multi_hop",
            RouteMode::IbcRoute => "ibc_route",
            RouteMode::XcmRoute => "xcm_route",
            RouteMode::ProofOnly => "proof_only",
            RouteMode::RefundOnly => "refund_only",
            RouteMode::Disabled => "disabled",
        }
    }
}

/// Supported chain kinds for atomic swap operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChainKind {
    Ethereum,
    Solana,
    Bitcoin,
    X3,
    Base,
    Arbitrum,
    Optimism,
    Bsc,
    Polygon,
    Avalanche,
    Cosmos,
}

impl ChainKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainKind::Ethereum => "eth",
            ChainKind::Solana => "sol",
            ChainKind::Bitcoin => "btc",
            ChainKind::X3 => "x3",
            ChainKind::Base => "base",
            ChainKind::Arbitrum => "arb",
            ChainKind::Optimism => "op",
            ChainKind::Bsc => "bsc",
            ChainKind::Polygon => "poly",
            ChainKind::Avalanche => "avax",
            ChainKind::Cosmos => "cosmos",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "eth" | "ethereum" => Some(ChainKind::Ethereum),
            "sol" | "solana" => Some(ChainKind::Solana),
            "btc" | "bitcoin" => Some(ChainKind::Bitcoin),
            "x3" => Some(ChainKind::X3),
            "base" => Some(ChainKind::Base),
            "arb" | "arbitrum" => Some(ChainKind::Arbitrum),
            "op" | "optimism" => Some(ChainKind::Optimism),
            "bsc" => Some(ChainKind::Bsc),
            "poly" | "polygon" => Some(ChainKind::Polygon),
            "avax" | "avalanche" => Some(ChainKind::Avalanche),
            "cosmos" => Some(ChainKind::Cosmos),
            _ => None,
        }
    }

    /// Default safe confirmation count for this chain.
    pub fn default_safe_confirmations(&self) -> u32 {
        match self {
            ChainKind::Ethereum => 12,
            ChainKind::Bitcoin => 6,
            ChainKind::Solana => 0,
            ChainKind::X3 => 1,
            ChainKind::Base | ChainKind::Arbitrum | ChainKind::Optimism => 1,
            ChainKind::Bsc => 15,
            ChainKind::Polygon => 128,
            ChainKind::Avalanche => 1,
            ChainKind::Cosmos => 1,
        }
    }
}

/// Finality level required for a chain before proceeding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinalityLevel {
    /// Wait for N block confirmations.
    Confirmations(u32),
    /// Solana finalized commitment.
    Finalized,
    /// Solana confirmed commitment.
    Confirmed,
    /// BFT finality (X3/Tendermint).
    Bft,
}

/// Per-chain finality requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityRequirement {
    pub chain: ChainKind,
    pub level: FinalityLevel,
}

/// Refund path defines how to recover funds if the swap expires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundPath {
    /// Chain to refund to.
    pub chain: ChainKind,
    /// Address to receive refund.
    pub address: String,
    /// Asset to refund (if different from source).
    pub asset: Option<String>,
}

/// Current lifecycle status of an atomic swap intent (16+ variants with
/// transition rules enforced by [`AtomicSwapStatus::can_transition_to`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtomicSwapStatus {
    // ── Pre-lock routing ───────────────────────────────────────────────
    /// Intent constructed but not yet active (alias: Created).
    Pending,
    /// Route has been quoted.
    RouteQuoted,
    /// Solver assigned to fill the swap.
    SolverAssigned,
    /// Relayers assigned to watch and attest.
    RelayersAssigned,

    // ── Locking ────────────────────────────────────────────────────────
    /// Source funds locked on source chain.
    SourceLocked,
    /// Destination funds locked on destination chain.
    DestinationLocked,
    /// Both source and destination are locked.
    BothLocked,

    // ── Claim path ─────────────────────────────────────────────────────
    /// Waiting for finality before revealing preimage.
    FinalityPending,
    /// Preimage revealed, claim window open.
    Claimable,
    /// Hashlock preimage has been revealed (alias for Claimable).
    PreimageRevealed,
    /// Claim submitted on one side.
    ClaimSubmitted,
    /// Swap completed successfully - funds claimed on destination.
    Claimed,
    /// Swap completed successfully (alias for Claimed).
    Completed,

    // ── Refund path ────────────────────────────────────────────────────
    /// Swap is refundable (timeout elapsed or other trigger).
    Refundable,
    /// Source side refundable after source timeout.
    RefundableSource,
    /// Destination side refundable after dest timeout.
    RefundableDestination,
    /// Swap expired and refund process initiated.
    Refunding,
    /// Swap fully refunded.
    Refunded,

    // ── Terminal / Error ───────────────────────────────────────────────
    /// Swap expired (timeouts elapsed, no action taken).
    Expired,
    /// Timeout elapsed on source with no lock (unsafe).
    ExpiredUnsafe,
    /// Dispute raised by a party.
    Disputed,
    /// Swap failed due to error.
    Failed,
    /// Swap blocked (manual intervention required).
    Blocked,
}

impl AtomicSwapStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AtomicSwapStatus::Claimed
                | AtomicSwapStatus::Completed
                | AtomicSwapStatus::Refunded
                | AtomicSwapStatus::Expired
                | AtomicSwapStatus::ExpiredUnsafe
                | AtomicSwapStatus::Failed
        )
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            AtomicSwapStatus::Pending => "Pending",
            AtomicSwapStatus::RouteQuoted => "Route Quoted",
            AtomicSwapStatus::SolverAssigned => "Solver Assigned",
            AtomicSwapStatus::RelayersAssigned => "Relayers Assigned",
            AtomicSwapStatus::SourceLocked => "Source Locked",
            AtomicSwapStatus::DestinationLocked => "Destination Locked",
            AtomicSwapStatus::BothLocked => "Both Locked",
            AtomicSwapStatus::FinalityPending => "Finality Pending",
            AtomicSwapStatus::Claimable => "Claimable",
            AtomicSwapStatus::PreimageRevealed => "Preimage Revealed",
            AtomicSwapStatus::ClaimSubmitted => "Claim Submitted",
            AtomicSwapStatus::Claimed => "Claimed",
            AtomicSwapStatus::Completed => "Completed",
            AtomicSwapStatus::Refundable => "Refundable",
            AtomicSwapStatus::RefundableSource => "Refundable Source",
            AtomicSwapStatus::RefundableDestination => "Refundable Destination",
            AtomicSwapStatus::Refunding => "Refunding",
            AtomicSwapStatus::Refunded => "Refunded",
            AtomicSwapStatus::Expired => "Expired",
            AtomicSwapStatus::ExpiredUnsafe => "Expired Unsafe",
            AtomicSwapStatus::Disputed => "Disputed",
            AtomicSwapStatus::Failed => "Failed",
            AtomicSwapStatus::Blocked => "Blocked",
        }
    }

    /// Return the allowed next states for each variant.
    pub fn valid_transitions(&self) -> &'static [AtomicSwapStatus] {
        use AtomicSwapStatus::*;
        match self {
            // Created (Pending) → RouteQuoted, SourceLocked, Expired, Failed
            Pending => &[RouteQuoted, SourceLocked, Expired, ExpiredUnsafe, Failed],
            // RouteQuoted → SolverAssigned, RelayersAssigned, Failed
            RouteQuoted => &[SolverAssigned, RelayersAssigned, Failed],
            // SolverAssigned → RelayersAssigned, Failed
            SolverAssigned => &[RelayersAssigned, Failed],
            // RelayersAssigned → SourceLocked, Failed
            RelayersAssigned => &[SourceLocked, Failed],
            // SourceLocked → DestinationLocked, BothLocked, RefundableSource, ExpiredUnsafe, Failed
            SourceLocked => &[
                DestinationLocked,
                BothLocked,
                RefundableSource,
                ExpiredUnsafe,
                Failed,
            ],
            // DestinationLocked → BothLocked, RefundableDestination, Failed
            DestinationLocked => &[BothLocked, RefundableDestination, Failed],
            // BothLocked → FinalityPending, RefundableDestination, Failed
            BothLocked => &[FinalityPending, RefundableDestination, Failed],
            // FinalityPending → Claimable, Failed
            FinalityPending => &[Claimable, Failed],
            // Claimable (and legacy PreimageRevealed) → Claimed, RefundableSource, Failed
            Claimable => &[Claimed, RefundableSource, Failed],
            PreimageRevealed => &[Claimed, RefundableSource, Failed],
            // ClaimSubmitted → Claimed, Failed
            ClaimSubmitted => &[Claimed, Failed],
            // Claimed - terminal (no outgoing transitions)
            Claimed | Completed => &[],
            // Refundable → Refunded, Failed
            Refundable => &[Refunded, Failed],
            // RefundableSource → Refunded, Failed
            RefundableSource => &[Refunded, Failed],
            // RefundableDestination → Refunded, Failed
            RefundableDestination => &[Refunded, Failed],
            // Refunding → Refunded, Failed
            Refunding => &[Refunded, Failed],
            // Refunded - terminal
            Refunded => &[],
            // Expired → Refundable, Failed
            Expired => &[Refundable, Failed],
            // ExpiredUnsafe - terminal
            ExpiredUnsafe => &[],
            // Disputed → Claimed, Refunded, Failed
            Disputed => &[Claimed, Refunded, Failed],
            // Failed - terminal
            Failed => &[],
            // Blocked - only exit is via explicit unblock (manually set)
            Blocked => &[],
        }
    }

    /// Check if a transition from `self` to `target` is valid.
    pub fn can_transition_to(&self, target: AtomicSwapStatus) -> bool {
        self.valid_transitions().contains(&target)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AtomicIntent - Core Schema
// ─────────────────────────────────────────────────────────────────────────────

/// Complete atomic swap intent with all fields required for cross-VM HTLC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicIntent {
    /// Unique intent identifier.
    pub intent_id: IntentId,
    /// Source chain (where funds originate).
    pub source_chain: ChainKind,
    /// Destination chain (where funds arrive).
    pub destination_chain: ChainKind,
    /// Source asset symbol (e.g. "USDC").
    pub source_asset: String,
    /// Destination asset symbol (e.g. "SOL").
    pub destination_asset: String,
    /// Amount in source asset base units.
    pub amount_in: u128,
    /// Minimum amount out in destination asset base units.
    pub min_amount_out: u128,
    /// Receiver address on destination chain.
    pub receiver: String,
    /// Hashlock: 32-byte blake2b or sha256 hash of the preimage.
    pub hashlock: [u8; 32],
    /// Source timeout: unix timestamp (or slot) after which source lock expires.
    pub source_timeout: u64,
    /// Destination timeout: unix timestamp (or slot) after which dest lock expires.
    pub destination_timeout: u64,
    /// Per-chain finality requirements.
    pub finality_requirements: Vec<FinalityRequirement>,
    /// Refund path for recovery on expiry.
    pub refund_path: RefundPath,
    /// Routing strategy for this swap.
    pub route_mode: RouteMode,
    /// Maximum acceptable slippage in basis points (1 bps = 0.01%).
    pub max_slippage_bps: u16,
    /// Minimum number of relayers that must attest.
    pub relayer_quorum_requirement: u32,
    /// Current lifecycle status.
    pub status: AtomicSwapStatus,
    /// SHA-256 hash of all intent fields (for integrity).
    pub intent_hash: [u8; 32],
}

impl AtomicIntent {
    /// Compute the canonical hash of this intent (all fields except status and hash).
    pub fn compute_hash(&self) -> [u8; 32] {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(self.intent_id.to_le_bytes());
        hasher.update(self.source_chain.as_str());
        hasher.update(self.destination_chain.as_str());
        hasher.update(&self.source_asset);
        hasher.update(&self.destination_asset);
        hasher.update(self.amount_in.to_le_bytes());
        hasher.update(self.min_amount_out.to_le_bytes());
        hasher.update(&self.receiver);
        hasher.update(self.hashlock);
        hasher.update(self.source_timeout.to_le_bytes());
        hasher.update(self.destination_timeout.to_le_bytes());
        for fr in &self.finality_requirements {
            hasher.update(fr.chain.as_str());
            match fr.level {
                FinalityLevel::Confirmations(n) => {
                    hasher.update([0u8]);
                    hasher.update(n.to_le_bytes());
                }
                FinalityLevel::Finalized => hasher.update([1u8]),
                FinalityLevel::Confirmed => hasher.update([2u8]),
                FinalityLevel::Bft => hasher.update([3u8]),
            }
        }
        hasher.update(self.refund_path.chain.as_str());
        hasher.update(&self.refund_path.address);
        hasher.update(self.route_mode.name());
        hasher.update(self.max_slippage_bps.to_le_bytes());
        hasher.update(self.relayer_quorum_requirement.to_le_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Verify the stored hash matches recomputed hash.
    pub fn verify_hash(&self) -> bool {
        self.intent_hash == self.compute_hash()
    }

    /// Validate timeout ordering: destination timeout must expire before source timeout.
    pub fn validate_timeout_ordering(&self) -> Result<(), SwapError> {
        if self.destination_timeout >= self.source_timeout {
            return Err(SwapError::InvalidTimeoutOrdering {
                destination_timeout: self.destination_timeout,
                source_timeout: self.source_timeout,
            });
        }
        Ok(())
    }

    /// Validate that a hashlock preimage matches.
    pub fn verify_preimage(&self, preimage: &[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut computed = [0u8; 32];
        computed.copy_from_slice(&result);
        computed == self.hashlock
    }

    /// Check if source timeout has elapsed.
    pub fn is_source_expired(&self, current_time: u64) -> bool {
        current_time >= self.source_timeout
    }

    /// Check if destination timeout has elapsed.
    pub fn is_destination_expired(&self, current_time: u64) -> bool {
        current_time >= self.destination_timeout
    }

    /// Set status enforcing the transition guard table.
    /// Illegal transitions return `SwapError::InvalidTransition { from, to }`.
    pub fn set_status(&mut self, new_status: AtomicSwapStatus) -> Result<(), SwapError> {
        if self.status.is_terminal() {
            return Err(SwapError::AlreadyTerminal {
                status: self.status,
            });
        }
        if !self.status.can_transition_to(new_status) {
            return Err(SwapError::InvalidTransition {
                from: self.status,
                to: new_status,
            });
        }
        self.status = new_status;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Builder
// ─────────────────────────────────────────────────────────────────────────────

/// Ergonomic builder for [`AtomicIntent`].
#[derive(Debug)]
pub struct AtomicIntentBuilder {
    source_chain: Option<ChainKind>,
    destination_chain: Option<ChainKind>,
    source_asset: Option<String>,
    destination_asset: Option<String>,
    amount_in: Option<u128>,
    min_amount_out: Option<u128>,
    receiver: Option<String>,
    hashlock: Option<[u8; 32]>,
    source_timeout: Option<u64>,
    destination_timeout: Option<u64>,
    finality_requirements: Vec<FinalityRequirement>,
    refund_path: Option<RefundPath>,
    route_mode: RouteMode,
    max_slippage_bps: u16,
    relayer_quorum_requirement: u32,
}

impl Default for AtomicIntentBuilder {
    fn default() -> Self {
        Self {
            source_chain: None,
            destination_chain: None,
            source_asset: None,
            destination_asset: None,
            amount_in: None,
            min_amount_out: None,
            receiver: None,
            hashlock: None,
            source_timeout: None,
            destination_timeout: None,
            finality_requirements: Vec::new(),
            refund_path: None,
            route_mode: RouteMode::DirectHtlc,
            max_slippage_bps: 100, // 1% default
            relayer_quorum_requirement: 0,
        }
    }
}

impl AtomicIntentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn source_chain(mut self, c: ChainKind) -> Self {
        self.source_chain = Some(c);
        self
    }

    pub fn destination_chain(mut self, c: ChainKind) -> Self {
        self.destination_chain = Some(c);
        self
    }

    pub fn source_asset(mut self, a: impl Into<String>) -> Self {
        self.source_asset = Some(a.into());
        self
    }

    pub fn destination_asset(mut self, a: impl Into<String>) -> Self {
        self.destination_asset = Some(a.into());
        self
    }

    pub fn amount_in(mut self, a: u128) -> Self {
        self.amount_in = Some(a);
        self
    }

    pub fn min_amount_out(mut self, a: u128) -> Self {
        self.min_amount_out = Some(a);
        self
    }

    pub fn receiver(mut self, r: impl Into<String>) -> Self {
        self.receiver = Some(r.into());
        self
    }

    pub fn hashlock(mut self, h: [u8; 32]) -> Self {
        self.hashlock = Some(h);
        self
    }

    pub fn source_timeout(mut self, t: u64) -> Self {
        self.source_timeout = Some(t);
        self
    }

    pub fn destination_timeout(mut self, t: u64) -> Self {
        self.destination_timeout = Some(t);
        self
    }

    pub fn add_finality(mut self, f: FinalityRequirement) -> Self {
        self.finality_requirements.push(f);
        self
    }

    pub fn refund_path(mut self, r: RefundPath) -> Self {
        self.refund_path = Some(r);
        self
    }

    pub fn relayer_quorum(mut self, q: u32) -> Self {
        self.relayer_quorum_requirement = q;
        self
    }

    pub fn route_mode(mut self, m: RouteMode) -> Self {
        self.route_mode = m;
        self
    }

    pub fn max_slippage_bps(mut self, bps: u16) -> Self {
        self.max_slippage_bps = bps;
        self
    }

    /// Build the AtomicIntent, computing hash and validating timeouts.
    pub fn build(self, intent_id: IntentId) -> Result<AtomicIntent, SwapError> {
        let source_chain = self.source_chain.ok_or(SwapError::MissingField {
            field: "source_chain",
        })?;
        let destination_chain = self.destination_chain.ok_or(SwapError::MissingField {
            field: "destination_chain",
        })?;
        let source_asset = self.source_asset.ok_or(SwapError::MissingField {
            field: "source_asset",
        })?;
        let destination_asset = self.destination_asset.ok_or(SwapError::MissingField {
            field: "destination_asset",
        })?;
        let amount_in = self
            .amount_in
            .ok_or(SwapError::MissingField { field: "amount_in" })?;
        let min_amount_out = self.min_amount_out.ok_or(SwapError::MissingField {
            field: "min_amount_out",
        })?;
        let receiver = self
            .receiver
            .ok_or(SwapError::MissingField { field: "receiver" })?;
        let hashlock = self
            .hashlock
            .ok_or(SwapError::MissingField { field: "hashlock" })?;
        let source_timeout = self.source_timeout.ok_or(SwapError::MissingField {
            field: "source_timeout",
        })?;
        let destination_timeout = self.destination_timeout.ok_or(SwapError::MissingField {
            field: "destination_timeout",
        })?;
        let refund_path = self.refund_path.ok_or(SwapError::MissingField {
            field: "refund_path",
        })?;

        let mut intent = AtomicIntent {
            intent_id,
            source_chain,
            destination_chain,
            source_asset,
            destination_asset,
            amount_in,
            min_amount_out,
            receiver,
            hashlock,
            source_timeout,
            destination_timeout,
            finality_requirements: self.finality_requirements,
            refund_path,
            route_mode: self.route_mode,
            max_slippage_bps: self.max_slippage_bps,
            relayer_quorum_requirement: self.relayer_quorum_requirement,
            status: AtomicSwapStatus::Pending,
            intent_hash: [0u8; 32],
        };

        // Validate timeout ordering
        intent.validate_timeout_ordering()?;

        // Validate partial fill: amount_in must be >= min_amount_out
        // (economically impossible swap otherwise)
        if intent.amount_in < intent.min_amount_out {
            return Err(SwapError::PartialFillNotAllowed {
                amount_in: intent.amount_in,
                min_amount_out: intent.min_amount_out,
            });
        }

        // Ensure route mode is not Disabled for active intents
        if intent.route_mode == RouteMode::Disabled {
            return Err(SwapError::generic(
                "route_mode must not be Disabled for active intents",
            ));
        }

        intent.intent_hash = intent.compute_hash();
        Ok(intent)
    }
}
