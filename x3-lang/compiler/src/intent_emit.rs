//! Adapter boundary: emit an X3IR intent resolution as a
//! canonical `IntentSpec` for the cross-chain intent crate.
//!
//! The x3-lang compiler produces a generic X3IR (with
//! `Operation::IntentResolve { constraints, resolver }`). The
//! cross-chain intent crate expects a fully-declarative
//! `IntentSpec` (chain, asset, amount, owner, receiver, route,
//! requirements, timeout, receipt, receiver authorization).
//!
//! This module is the **adapter boundary** between the language
//! compiler layer and the intent layer. None of the three layers
//! (x3-lang, x3-crosschain-intent, x3-settlement-engine) may build
//! a parallel state machine. The language compiler emits an
//! `IntentSpec`; the intent crate validates and lowers it; the
//! settlement engine consumes the lower-level
//! `CrossChainIntent`/`X3Instruction` plan and drives execution.

use serde::{Deserialize, Serialize};

/// A single source-level constraint emitted by the x3-lang parser.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceConstraint {
    /// Constraint kind, e.g. `"slippage"`, `"finality"`,
    /// `"max_fee"`, `"proof"`, `"timeout"`, `"receiver"`.
    pub kind: String,
    /// Free-form string representation of the constraint's
    /// argument (e.g. `"<= 0.5%"`, `"eth >= 12"`,
    /// `"eth.lock_event"`).
    pub arg: String,
}

/// Builder that the x3-lang compiler populates from a source-level
/// intent declaration. The builder is the canonical input shape for
/// the cross-chain intent crate's
/// `intent_spec_to_crosschain_intent` adapter.
///
/// The fields are stringly-typed at the language boundary so the
/// language compiler does not need to know every chain/asset enum,
/// and the intent crate's adapter is responsible for canonicalizing
/// chain names and parsing amounts. This keeps x3-lang decoupled
/// from the intent crate's type system (no dependency cycle).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentSpecDraft {
    /// Human-readable intent name (the resolver identifier).
    pub name: String,
    /// Source chain (e.g. `"eth"`, `"sol"`, `"x3"`, `"btc"`).
    pub source_chain: String,
    /// Source asset symbol (e.g. `"USDC"`, `"ETH"`).
    pub source_asset: String,
    /// Source amount in base units (smallest denomination).
    pub source_amount: u128,
    /// Source owner address (chain-specific string form).
    pub source_owner: String,
    /// Optional source lock contract.
    pub source_lock_contract: Option<String>,
    /// Destination chain.
    pub dest_chain: String,
    /// Destination asset symbol.
    pub dest_asset: String,
    /// Optional minimum destination amount.
    pub dest_min_amount: Option<u128>,
    /// Destination receiver address.
    pub dest_receiver: String,
    /// Source-level constraints (slippage, finality, fee cap, …).
    pub constraints: Vec<SourceConstraint>,
    /// Timeout duration in seconds (parsed from `"30m"`, `"1h"`, …).
    pub timeout_secs: u64,
}

impl IntentSpecDraft {
    /// Build a draft with safe defaults (30-minute timeout, no
    /// destination min amount, empty constraints).
    pub fn new(
        name: impl Into<String>,
        source_chain: impl Into<String>,
        source_asset: impl Into<String>,
        source_amount: u128,
        source_owner: impl Into<String>,
        dest_chain: impl Into<String>,
        dest_asset: impl Into<String>,
        dest_receiver: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source_chain: source_chain.into(),
            source_asset: source_asset.into(),
            source_amount,
            source_owner: source_owner.into(),
            source_lock_contract: None,
            dest_chain: dest_chain.into(),
            dest_asset: dest_asset.into(),
            dest_min_amount: None,
            dest_receiver: dest_receiver.into(),
            constraints: Vec::new(),
            timeout_secs: 30 * 60,
        }
    }

    /// Apply a timeout override in seconds. Returns `self` for
    /// chaining.
    pub fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Add a constraint. Returns `self` for chaining.
    pub fn with_constraint(mut self, kind: impl Into<String>, arg: impl Into<String>) -> Self {
        self.constraints.push(SourceConstraint {
            kind: kind.into(),
            arg: arg.into(),
        });
        self
    }

    /// Set the source lock contract.
    pub fn with_lock_contract(mut self, contract: impl Into<String>) -> Self {
        self.source_lock_contract = Some(contract.into());
        self
    }

    /// Set the destination minimum amount.
    pub fn with_dest_min_amount(mut self, amount: u128) -> Self {
        self.dest_min_amount = Some(amount);
        self
    }

    /// Parse a timeout arg like "30m", "1h", "600s" into seconds.
    pub fn timeout_from_arg(arg: &str) -> Option<u64> {
        let arg = arg.trim();
        if let Some(rest) = arg.strip_suffix('m') {
            rest.parse::<u64>().ok().map(|n| n * 60)
        } else if let Some(rest) = arg.strip_suffix('h') {
            rest.parse::<u64>().ok().map(|n| n * 3600)
        } else if let Some(rest) = arg.strip_suffix('s') {
            rest.parse::<u64>().ok()
        } else {
            arg.parse::<u64>().ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_timeout_args() {
        assert_eq!(IntentSpecDraft::timeout_from_arg("30m"), Some(30 * 60));
        assert_eq!(IntentSpecDraft::timeout_from_arg("1h"), Some(3600));
        assert_eq!(IntentSpecDraft::timeout_from_arg("600s"), Some(600));
        assert_eq!(IntentSpecDraft::timeout_from_arg("600"), Some(600));
        assert_eq!(IntentSpecDraft::timeout_from_arg("garbage"), None);
    }

    #[test]
    fn builder_uses_safe_defaults() {
        let draft = IntentSpecDraft::new(
            "bridge_usdc_sol",
            "eth",
            "USDC",
            500_000_000,
            "alice.eth",
            "sol",
            "SOL",
            "alice.sol",
        );
        assert_eq!(draft.timeout_secs, 30 * 60);
        assert_eq!(draft.source_lock_contract, None);
        assert_eq!(draft.dest_min_amount, None);
        assert!(draft.constraints.is_empty());
    }

    #[test]
    fn builder_chains_constraints_and_overrides() {
        let draft = IntentSpecDraft::new(
            "bridge_usdc_sol",
            "eth",
            "USDC",
            500_000_000,
            "alice.eth",
            "sol",
            "SOL",
            "alice.sol",
        )
        .with_timeout_secs(900)
        .with_lock_contract("0xBridge")
        .with_dest_min_amount(3_500_000_000)
        .with_constraint("slippage", "<= 0.5%")
        .with_constraint("max_fee", "<= 10 USDC")
        .with_constraint("proof", "eth.lock_event");

        assert_eq!(draft.timeout_secs, 900);
        assert_eq!(draft.source_lock_contract.as_deref(), Some("0xBridge"));
        assert_eq!(draft.dest_min_amount, Some(3_500_000_000));
        assert_eq!(draft.constraints.len(), 3);
        assert_eq!(draft.constraints[0].kind, "slippage");
        assert_eq!(draft.constraints[0].arg, "<= 0.5%");
    }

    #[test]
    fn draft_round_trips_through_json() {
        let draft = IntentSpecDraft::new(
            "round_trip",
            "btc",
            "BTC",
            10_000,
            "bc1qalice",
            "x3",
            "BTC.e",
            "alice.x3",
        )
        .with_constraint("finality", "btc >= 6");

        let json = serde_json::to_string(&draft).expect("serialize");
        let parsed: IntentSpecDraft = serde_json::from_str(&json).expect("parse");
        assert_eq!(draft, parsed);
    }
}
