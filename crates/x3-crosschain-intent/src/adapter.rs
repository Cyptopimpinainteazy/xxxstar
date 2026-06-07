//! Adapter boundary between language compilers and the cross-chain
//! intent system.
//!
//! The x3-lang compiler lowers `.x3` source into an `X3IR` program.
//! The cross-chain intent system expects a [`CrossChainIntent`] as
//! the canonical contract. Rather than letting the x3-lang compiler
//! hand-roll a `CrossChainIntent` (and possibly diverge from the
//! intent crate's invariants), the boundary is this module:
//!
//! - The language compiler produces an [`IntentSpec`] — a
//!   language-agnostic, fully-declarative description of a single
//!   cross-chain intent.
//! - The cross-chain intent crate converts the [`IntentSpec`] into a
//!   [`CrossChainIntent`] using [`intent_spec_to_crosschain_intent`].
//! - The settlement engine converts a [`CrossChainIntent`] into a
//!   [`SettlementIntent`] (Substrate runtime) using its own
//!   `from_crosschain_intent` adapter (see
//!   `pallets/x3-settlement-engine/src/intent.rs`).
//!
//! This is the single contract the language compiler, intent crate,
//! and settlement engine all share. None of the three layers may
//! maintain a parallel state machine; they MUST adapt through this
//! module and its mirror in the settlement pallet.
//!
//! ## Why a separate `IntentSpec`?
//!
//! The x3-lang workspace compiles for the `no_std`-leaning chain
//! runtime, and depends on its own AST/IR. The x3-crosschain-intent
//! crate cannot depend on x3-lang-compiler without creating a
//! dependency cycle (the chain runtime depends on the intent crate
//! but the x3-lang compiler also wants to produce intents).
//!
//! The `IntentSpec` is a value-only struct that the language compiler
//! can construct without depending on the intent crate's types
//! beyond the small set of fields it needs. The intent crate then
//! consumes that spec. Both sides can be tested in isolation.

use crate::prelude::*;
use crate::types::{
    AssetRef, ChainKind, DestinationSpec, ReceiptSpec, ReceiverAuthorization, Requirements,
    RouteSpec, SourceSpec, TimeoutSpec,
};

/// Language-agnostic description of a single cross-chain intent.
///
/// The x3-lang compiler (or any other front-end) constructs an
/// `IntentSpec` from the user's source program. The intent crate
/// then lowers the spec into a fully-validated [`crate::CrossChainIntent`].
///
/// The fields here are the same user-controlled surface that
/// [`crate::canonical::encode_intent_canonical`] walks; a complete
/// spec produces a complete intent.
#[derive(Debug, Clone)]
pub struct IntentSpec {
    /// Human-readable intent name.
    pub name: String,
    /// Source asset, amount, owner, optional lock contract.
    pub source: SourceSpec,
    /// Destination asset, receiver, optional min amount.
    pub destination: DestinationSpec,
    /// Route objective, allowed venues, denied venues.
    pub route: RouteSpec,
    /// All safety requirements (slippage, fees, proofs, …).
    pub requirements: Requirements,
    /// Timeout and recovery actions.
    pub timeout: TimeoutSpec,
    /// What the on-chain receipt should expose.
    pub receipt: ReceiptSpec,
    /// Receiver authorization rule.
    pub receiver_authorization: ReceiverAuthorization,
}

impl IntentSpec {
    /// Build a spec with the strictest possible defaults: a
    /// 30-minute timeout with source refund, no simulation required,
    /// canonical supply check off, no allowed venues, and the
    /// "bridge.unknown" venue denied.
    pub fn new(name: impl Into<String>, source: SourceSpec, destination: DestinationSpec) -> Self {
        Self {
            name: name.into(),
            source,
            destination,
            route: RouteSpec::default(),
            requirements: Requirements::default(),
            timeout: TimeoutSpec::default_30m_source_refund(),
            receipt: ReceiptSpec::default(),
            receiver_authorization: ReceiverAuthorization::OwnerOnly,
        }
    }
}

/// Convert an [`IntentSpec`] to a [`crate::CrossChainIntent`], assign
/// a fresh `intent_id`, and stamp the canonical hash.
///
/// This is the **adapter boundary** from the language compiler to
/// the intent crate. It is the single canonical way the language
/// layer produces an intent. Any other entry point (RPC, manual
/// JSON, deserialization from disk) should also flow through an
/// `IntentSpec` and this function so the entire system agrees on
/// one contract.
pub fn intent_spec_to_crosschain_intent(spec: IntentSpec, id: u64) -> crate::CrossChainIntent {
    use crate::CrossChainIntent;
    // Merge the receiver authorization into the requirements so the
    // rest of the compiler sees a single Requirements struct.
    let mut requirements = spec.requirements;
    requirements.receiver_authorization = spec.receiver_authorization;
    let mut intent = CrossChainIntent {
        id,
        name: spec.name,
        source: spec.source,
        destination: spec.destination,
        route: spec.route,
        requirements,
        timeout: spec.timeout,
        receipt: spec.receipt,
        intent_hash: [0u8; 32],
    };
    intent.recompute_and_store_hash();
    intent
}

/// Bridge the high-level [`AssetRef`] chain string into a
/// [`ChainKind`]. The compiler that produces the `IntentSpec` is
/// expected to use canonical chain names ("eth", "sol", "x3", "btc",
/// "base", "arb", "op", "bsc", "poly", "avax", "cosmos"); anything
/// else fails closed at the boundary rather than being silently
/// re-mapped to `ChainKind::X3`.
pub fn chain_kind_from_canonical(name: &str) -> Result<ChainKind, AdapterError> {
    ChainKind::parse(name).ok_or_else(|| AdapterError::UnknownChain {
        chain: name.to_string(),
    })
}

/// Errors produced by the adapter boundary. They are converted into
/// `IntentCompileError`s at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// The language compiler produced a chain name that this adapter
    /// does not recognize.
    UnknownChain { chain: String },
    /// The language compiler produced a negative or zero amount.
    NonPositiveAmount { amount: u128, asset: String },
    /// The language compiler produced an empty name.
    EmptyIntentName,
    /// The language compiler produced an empty owner or receiver.
    EmptyAddress { field: &'static str },
}

impl core::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AdapterError::UnknownChain { chain } => {
                write!(f, "adapter: unknown chain '{chain}'")
            }
            AdapterError::NonPositiveAmount { amount, asset } => {
                write!(
                    f,
                    "adapter: non-positive amount {amount} for asset '{asset}'"
                )
            }
            AdapterError::EmptyIntentName => write!(f, "adapter: empty intent name"),
            AdapterError::EmptyAddress { field } => {
                write!(f, "adapter: empty {field} address")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AdapterError {}

/// Sanity-check an `IntentSpec` for the most basic safety properties
/// before it is fed into the canonical intent compiler. This is the
/// **adapter-level preflight** — it catches mistakes that should
/// never reach the safety checker (so the safety checker can focus
/// on the high-level rules).
pub fn validate_intent_spec(spec: &IntentSpec) -> Result<(), AdapterError> {
    if spec.name.is_empty() {
        return Err(AdapterError::EmptyIntentName);
    }
    if spec.source.owner.is_empty() {
        return Err(AdapterError::EmptyAddress {
            field: "source.owner",
        });
    }
    if spec.destination.receiver.is_empty() {
        return Err(AdapterError::EmptyAddress {
            field: "destination.receiver",
        });
    }
    if spec.source.amount == 0 {
        return Err(AdapterError::NonPositiveAmount {
            amount: 0,
            asset: spec.source.asset.display(),
        });
    }
    Ok(())
}

/// Build an [`AssetRef`] from a `(chain, symbol)` pair after
/// validating the chain. This is the canonical way the adapter
/// produces asset references so the `AssetRef` invariants (non-empty
/// symbol, recognized chain) hold at the boundary.
pub fn asset_ref_from_canonical(
    chain: &str,
    symbol: impl Into<String>,
) -> Result<AssetRef, AdapterError> {
    let kind = chain_kind_from_canonical(chain)?;
    let symbol = symbol.into();
    if symbol.is_empty() {
        return Err(AdapterError::EmptyAddress {
            field: "asset.symbol",
        });
    }
    Ok(AssetRef::new(kind, symbol))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FailureAction, FinalityRequirement, RouteObjective};

    fn basic_spec() -> IntentSpec {
        let source = SourceSpec {
            asset: AssetRef::new(ChainKind::Ethereum, "USDC"),
            amount: 500_000_000,
            owner: "alice.eth".to_string(),
            lock_contract: Some("0xBridge".to_string()),
        };
        let destination = DestinationSpec {
            asset: AssetRef::new(ChainKind::Solana, "SOL"),
            receiver: "alice.sol".to_string(),
            min_amount: Some(3_500_000_000),
        };
        let mut spec = IntentSpec::new("bridge_usdc_sol", source, destination);
        spec.route = RouteSpec {
            objective: RouteObjective::Best,
            allow: vec!["x3.dex".to_string()],
            deny: vec!["bridge.unknown".to_string()],
        };
        spec.requirements = Requirements {
            finality: vec![FinalityRequirement {
                chain: ChainKind::Ethereum,
                level: crate::types::FinalityLevel::Confirmations(12),
            }],
            max_slippage_bps: Some(50),
            max_total_fee: Some(10_000_000),
            ..Requirements::default()
        };
        spec.receiver_authorization = ReceiverAuthorization::MappedAccount {
            source_chain: ChainKind::Ethereum,
            source_owner: "alice.eth".to_string(),
            dest_chain: ChainKind::Solana,
            dest_account: "alice.sol".to_string(),
        };
        spec
    }

    #[test]
    fn adapter_produces_canonical_intent() {
        let spec = basic_spec();
        let intent = intent_spec_to_crosschain_intent(spec, 42);
        assert_eq!(intent.id, 42);
        assert_eq!(intent.name, "bridge_usdc_sol");
        assert!(
            intent.verify_hash(),
            "adapter must produce a hash that matches fields"
        );
    }

    #[test]
    fn adapter_rejects_empty_name() {
        let mut spec = basic_spec();
        spec.name = "".to_string();
        assert!(validate_intent_spec(&spec).is_err());
    }

    #[test]
    fn adapter_rejects_zero_amount() {
        let mut spec = basic_spec();
        spec.source.amount = 0;
        assert!(validate_intent_spec(&spec).is_err());
    }

    #[test]
    fn adapter_rejects_empty_owner() {
        let mut spec = basic_spec();
        spec.source.owner = "".to_string();
        assert!(validate_intent_spec(&spec).is_err());
    }

    #[test]
    fn adapter_rejects_empty_receiver() {
        let mut spec = basic_spec();
        spec.destination.receiver = "".to_string();
        assert!(validate_intent_spec(&spec).is_err());
    }

    #[test]
    fn adapter_preserves_receiver_authorization() {
        let spec = basic_spec();
        let intent = intent_spec_to_crosschain_intent(spec, 7);
        match &intent.requirements.receiver_authorization {
            ReceiverAuthorization::MappedAccount {
                source_chain,
                source_owner,
                dest_chain,
                dest_account,
            } => {
                assert_eq!(*source_chain, ChainKind::Ethereum);
                assert_eq!(source_owner, "alice.eth");
                assert_eq!(*dest_chain, ChainKind::Solana);
                assert_eq!(dest_account, "alice.sol");
            }
            other => panic!("expected MappedAccount, got {other:?}"),
        }
    }

    #[test]
    fn asset_ref_from_canonical_rejects_unknown_chain() {
        let err = asset_ref_from_canonical("bogus", "USDC").unwrap_err();
        assert!(matches!(err, AdapterError::UnknownChain { .. }));
    }

    #[test]
    fn asset_ref_from_canonical_rejects_empty_symbol() {
        let err = asset_ref_from_canonical("eth", "").unwrap_err();
        assert!(matches!(err, AdapterError::EmptyAddress { .. }));
    }

    #[test]
    fn failure_action_round_trip_for_timeout_defaults() {
        // The default timeout on a fresh `IntentSpec::new` is
        // 30m / RefundSource. The adapter must preserve that.
        let source = SourceSpec {
            asset: AssetRef::new(ChainKind::Ethereum, "USDC"),
            amount: 1,
            owner: "alice.eth".to_string(),
            lock_contract: None,
        };
        let destination = DestinationSpec {
            asset: AssetRef::new(ChainKind::X3, "USDC.e"),
            receiver: "alice.x3".to_string(),
            min_amount: None,
        };
        let spec = IntentSpec::new("test", source, destination);
        let intent = intent_spec_to_crosschain_intent(spec, 1);
        assert_eq!(intent.timeout.timeout_secs, 30 * 60);
        assert_eq!(
            intent.timeout.on_fail,
            vec![FailureAction::RefundSource, FailureAction::Quarantine]
        );
    }
}
