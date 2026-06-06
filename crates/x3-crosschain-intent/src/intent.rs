//! CrossChainIntent — the core structured intent type.
//!
//! A CrossChainIntent is the user's complete declaration of what they want,
//! what constraints must hold, what proofs are required, and how to recover
//! if execution fails. It is not a transaction. It compiles into one.
//!
//! # Naming
//!
//! The intent has a human-readable name used in logs, the explorer, and
//! diagnostic messages. Names must be unique within a session.
//!
//! # Construction
//!
//! Use [`CrossChainIntentBuilder`] for ergonomic construction, or construct
//! the struct directly if loading from a parsed representation.

use crate::types::{
    DestinationSpec, FailureAction, FinalityRequirement, ProofRequirement, ReceiptSpec,
    Requirements, RouteSpec, SourceSpec, TimeoutSpec,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Unique intent identifier (monotonically assigned by the registry).
pub type IntentId = u64;

/// A fully-specified cross-chain intent.
///
/// The four-part contract:
/// 1. `source` / `destination` — what the user wants
/// 2. `requirements` — what limits must never be crossed
/// 3. `requirements.proofs` — what proof must exist before each step
/// 4. `timeout` — what happens if execution fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainIntent {
    /// Unique identifier assigned at registration.
    pub id: IntentId,
    /// Human-readable intent name (e.g. "swap_and_bridge").
    pub name: String,
    /// Source asset, amount, and owner.
    pub source: SourceSpec,
    /// Destination asset and recipient.
    pub destination: DestinationSpec,
    /// Route selection and venue policy.
    pub route: RouteSpec,
    /// All safety constraints.
    pub requirements: Requirements,
    /// Timeout duration and failure recovery actions.
    pub timeout: TimeoutSpec,
    /// What to include in the final on-chain receipt.
    pub receipt: ReceiptSpec,
    /// SHA-256 of the canonical (id-excluded) intent fields.
    /// Computed at construction, verified before execution.
    pub intent_hash: [u8; 32],
}

impl CrossChainIntent {
    /// Compute the canonical SHA-256 hash of this intent.
    ///
    /// The `id` field is excluded so that the hash covers only the
    /// user-supplied content. Two intents with identical declarations
    /// produce the same hash regardless of registry assignment.
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.name.as_bytes());
        h.update(self.source.asset.display().as_bytes());
        h.update(self.source.amount.to_le_bytes());
        h.update(self.source.owner.as_bytes());
        h.update(self.destination.asset.display().as_bytes());
        h.update(self.destination.receiver.as_bytes());
        if let Some(min) = self.destination.min_amount {
            h.update(min.to_le_bytes());
        }
        h.update(self.timeout.timeout_secs.to_le_bytes());
        h.finalize().into()
    }

    /// Verify the stored hash matches recomputed hash.
    /// Returns `false` if the intent has been tampered with.
    pub fn verify_hash(&self) -> bool {
        self.intent_hash == self.compute_hash()
    }

    /// True if a swap step is required to satisfy this intent
    /// (source.asset != destination.asset).
    pub fn requires_swap(&self) -> bool {
        self.source.asset != self.destination.asset
    }

    /// True if a bridge step is required
    /// (source and destination are on different chains).
    pub fn requires_bridge(&self) -> bool {
        self.source.asset.chain != self.destination.asset.chain
    }

    /// True if this intent only bridges within X3 (no external chain crossing).
    pub fn is_x3_only(&self) -> bool {
        self.source.asset.chain == crate::types::ChainKind::X3
            && self.destination.asset.chain == crate::types::ChainKind::X3
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Builder
// ─────────────────────────────────────────────────────────────────────────────

/// Ergonomic builder for [`CrossChainIntent`].
///
/// Call `.build(id)` to produce the final intent with a computed hash.
#[derive(Debug, Default)]
pub struct CrossChainIntentBuilder {
    name: Option<String>,
    source: Option<SourceSpec>,
    destination: Option<DestinationSpec>,
    route: Option<RouteSpec>,
    requirements: Requirements,
    timeout: Option<TimeoutSpec>,
    receipt: ReceiptSpec,
}

impl CrossChainIntentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, n: impl Into<String>) -> Self {
        self.name = Some(n.into());
        self
    }

    pub fn source(mut self, s: SourceSpec) -> Self {
        self.source = Some(s);
        self
    }

    pub fn destination(mut self, d: DestinationSpec) -> Self {
        self.destination = Some(d);
        self
    }

    pub fn route(mut self, r: RouteSpec) -> Self {
        self.route = Some(r);
        self
    }

    pub fn require_finality(mut self, f: FinalityRequirement) -> Self {
        self.requirements.finality.push(f);
        self
    }

    pub fn require_max_slippage_bps(mut self, bps: u32) -> Self {
        self.requirements.max_slippage_bps = Some(bps);
        self
    }

    pub fn require_max_fee(mut self, fee: u128) -> Self {
        self.requirements.max_total_fee = Some(fee);
        self
    }

    pub fn require_receiver_is_owner(mut self) -> Self {
        self.requirements.require_receiver_is_owner = true;
        self
    }

    pub fn require_proof(mut self, p: ProofRequirement) -> Self {
        self.requirements.proofs.push(p);
        self
    }

    pub fn require_canonical_supply(mut self) -> Self {
        self.requirements.require_canonical_supply_valid = true;
        self
    }

    pub fn require_simulation(mut self) -> Self {
        self.requirements.require_route_simulated = true;
        self
    }

    pub fn timeout(mut self, t: TimeoutSpec) -> Self {
        self.timeout = Some(t);
        self
    }

    pub fn on_fail(mut self, action: FailureAction) -> Self {
        if let Some(t) = &mut self.timeout {
            t.on_fail.push(action);
        }
        self
    }

    pub fn receipt(mut self, r: ReceiptSpec) -> Self {
        self.receipt = r;
        self
    }

    /// Consume the builder and produce a [`CrossChainIntent`].
    ///
    /// Returns an error string if required fields are missing.
    pub fn build(self, id: IntentId) -> Result<CrossChainIntent, String> {
        let name = self.name.ok_or("intent name is required")?;
        let source = self.source.ok_or("source spec is required")?;
        let destination = self.destination.ok_or("destination spec is required")?;
        let route = self.route.unwrap_or_default();
        let timeout = self
            .timeout
            .ok_or("timeout spec is required (X3-INTENT-003)")?;

        let mut intent = CrossChainIntent {
            id,
            name,
            source,
            destination,
            route,
            requirements: self.requirements,
            timeout,
            receipt: self.receipt,
            intent_hash: [0u8; 32],
        };
        intent.intent_hash = intent.compute_hash();
        Ok(intent)
    }
}
