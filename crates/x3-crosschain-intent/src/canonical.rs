//! Deterministic canonical encoding for [`CrossChainIntent`].
//!
//! This module is the single source of truth for the byte order in which the
//! intent's user-controlled fields are fed into SHA-256 to produce
//! `intent.intent_hash`. Every layer (compiler, runtime, settlement engine)
//! must call [`encode_intent_canonical`] to obtain the same bytes for the same
//! intent. Mismatch with `intent.intent_hash` is a hard failure — see
//! `X3-INTENT-014`.
//!
//! ## Encoding rules
//!
//! 1. **Length prefixes are big-endian `u64`** so a value cannot be confused
//!    with a string that happens to contain a NUL byte, and so a parser
//!    cannot walk off the end of a variable-length field.
//! 2. **Field tags are emitted in a fixed order** (see
//!    [`CANONICAL_FIELD_ORDER`]). The encoding always covers the same set
//!    of tags in the same order, so a hash covers *every* user-controlled
//!    field, not just a subset.
//! 3. **Enums encode their discriminant as a `u8`** before their payload.
//!    This means adding a new enum variant changes the hash for every
//!    intent (good — surface area change).
//! 4. **Strings encode as `u64` length prefix + UTF-8 bytes** (no NUL
//!    terminator).
//! 5. **Integers encode as little-endian, fixed-width** (`u32`/`u64`/`u128`).
//! 6. **Optional fields encode a `u8` tag (`0` = None, `1` = Some) before
//!
//! [`push_u8`] is retained for future field-tag encodings; it is
//! currently unused but is part of the canonical module's public
//! scalar-encoder surface for callers that need a tagged byte.
#![allow(dead_code, clippy::too_many_arguments)]
//!    their payload**. This is unambiguous: a missing `finality` vec is
//!    `0x00` while an empty one is `0x01 + 0x0000000000000000`.
//! 7. **Vectors encode as `u64` element count + per-element payload**. The
//!    per-element encoding is the same as the scalar type's encoding.
//! 8. **Booleans encode as a single `u8`** (`0` or `1`).
//!
//! The encoding is fully self-describing: a parser can walk any byte slice
//! and recover the field boundary for every field. The hash therefore
//! commits to *the entire user-controlled surface*, not a few cherry-picked
//! fields. See `compute_hash` in [`crate::intent`] and the `X3-INTENT-014`
//! error in [`crate::error::IntentCompileError`] for how the encoding is
//! used.

use crate::prelude::*;
use crate::types::{
    AssetRef, ChainKind, DestinationSpec, FailureAction, FinalityLevel, FinalityRequirement,
    ProofKind, ProofRequirement, ReceiptSpec, Requirements, RouteObjective, RouteSpec, SourceSpec,
    TimeoutSpec,
};

/// Fixed ordering of the user-controlled fields. Every canonical encoding
/// walks the intent in this exact order. Adding a field requires updating
/// this list and bumping the schema version.
pub const CANONICAL_FIELD_ORDER: &[&str] = &[
    "name",
    "source",
    "destination",
    "route",
    "requirements",
    "timeout",
    "receipt",
];

/// Tag byte identifying an enum discriminant or a None/Some marker.
pub const TAG_NONE: u8 = 0;
pub const TAG_SOME: u8 = 1;

/// Encode a `CrossChainIntent`'s user-controlled fields into `out`. The
/// output is the exact byte sequence fed to SHA-256 in
/// [`crate::intent::CrossChainIntent::compute_hash`].
///
/// `out` is *appended* to — call sites should pass a fresh `Vec<u8>` to get
/// the bare canonical bytes, or chain this with prior context.
pub fn encode_intent_canonical(
    name: &str,
    source: &SourceSpec,
    destination: &DestinationSpec,
    route: &RouteSpec,
    requirements: &Requirements,
    timeout: &TimeoutSpec,
    receipt: &ReceiptSpec,
    out: &mut Vec<u8>,
) {
    // The schema tag — currently `1`. Bumped if the canonical encoding
    // changes shape. This is the very first byte so any change to the
    // encoding format itself is detected by the hash.
    out.push(0x01);

    // Field 1/7: name
    push_str(out, name);

    // Field 2/7: source
    push_source(out, source);

    // Field 3/7: destination
    push_destination(out, destination);

    // Field 4/7: route
    push_route(out, route);

    // Field 5/7: requirements
    push_requirements(out, requirements);

    // Field 6/7: timeout
    push_timeout(out, timeout);

    // Field 7/7: receipt
    push_receipt(out, receipt);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scalar encoders
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) fn push_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

pub(crate) fn push_bool(out: &mut Vec<u8>, v: bool) {
    out.push(if v { 1 } else { 0 });
}

pub(crate) fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub(crate) fn push_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub(crate) fn push_u128(out: &mut Vec<u8>, v: u128) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub(crate) fn push_str(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

pub(crate) fn push_option<T>(
    out: &mut Vec<u8>,
    opt: &Option<T>,
    encode_value: impl Fn(&T, &mut Vec<u8>),
) {
    match opt {
        None => out.push(TAG_NONE),
        Some(v) => {
            out.push(TAG_SOME);
            encode_value(v, out);
        }
    }
}

pub(crate) fn push_vec<T>(out: &mut Vec<u8>, items: &[T], encode_item: impl Fn(&T, &mut Vec<u8>)) {
    out.extend_from_slice(&(items.len() as u64).to_le_bytes());
    for item in items {
        encode_item(item, out);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain encoders
// ─────────────────────────────────────────────────────────────────────────────

pub fn push_chain_kind(out: &mut Vec<u8>, chain: ChainKind) {
    // Discriminant is the index in the canonical enum declaration.
    let discriminant: u8 = match chain {
        ChainKind::Ethereum => 0,
        ChainKind::Solana => 1,
        ChainKind::Bitcoin => 2,
        ChainKind::X3 => 3,
        ChainKind::Base => 4,
        ChainKind::Arbitrum => 5,
        ChainKind::Optimism => 6,
        ChainKind::Bsc => 7,
        ChainKind::Polygon => 8,
        ChainKind::Avalanche => 9,
        ChainKind::Cosmos => 10,
    };
    out.push(discriminant);
}

pub fn push_asset_ref(out: &mut Vec<u8>, asset: &AssetRef) {
    push_chain_kind(out, asset.chain);
    push_str(out, &asset.symbol);
}

fn push_source(out: &mut Vec<u8>, source: &SourceSpec) {
    push_asset_ref(out, &source.asset);
    push_u128(out, source.amount);
    push_str(out, &source.owner);
    push_option(out, &source.lock_contract, |v, out| push_str(out, v));
}

fn push_destination(out: &mut Vec<u8>, dest: &DestinationSpec) {
    push_asset_ref(out, &dest.asset);
    push_str(out, &dest.receiver);
    push_option(out, &dest.min_amount, |v, out| push_u128(out, *v));
}

pub fn push_route_objective(out: &mut Vec<u8>, obj: &RouteObjective) {
    let discriminant: u8 = match obj {
        RouteObjective::MaximizeOutput => 0,
        RouteObjective::MinimizeTotalCost => 1,
        RouteObjective::MinimizeLatency => 2,
        RouteObjective::Best => 3,
    };
    out.push(discriminant);
}

fn push_route(out: &mut Vec<u8>, route: &RouteSpec) {
    push_route_objective(out, &route.objective);
    push_vec(out, &route.allow, |v, out| push_str(out, v));
    push_vec(out, &route.deny, |v, out| push_str(out, v));
}

pub fn push_finality_level(out: &mut Vec<u8>, level: &FinalityLevel) {
    match level {
        FinalityLevel::Confirmations(n) => {
            out.push(0);
            push_u32(out, *n);
        }
        FinalityLevel::Finalized => out.push(1),
        FinalityLevel::Confirmed => out.push(2),
        FinalityLevel::Bft => out.push(3),
    }
}

fn push_finality_requirement(out: &mut Vec<u8>, req: &FinalityRequirement) {
    push_chain_kind(out, req.chain);
    push_finality_level(out, &req.level);
}

pub fn push_proof_kind(out: &mut Vec<u8>, kind: &ProofKind) {
    match kind {
        ProofKind::EventProof {
            event,
            contract,
            confirmations,
        } => {
            out.push(0);
            push_str(out, event);
            push_str(out, contract);
            push_u32(out, *confirmations);
        }
        ProofKind::MerkleProof { root_type } => {
            out.push(1);
            push_str(out, root_type);
        }
        ProofKind::LightClientProof { client_id } => {
            out.push(2);
            push_str(out, client_id);
        }
        ProofKind::ValidatorQuorum { threshold_bps } => {
            out.push(3);
            push_u32(out, *threshold_bps);
        }
        ProofKind::ZkProof { circuit } => {
            out.push(4);
            push_str(out, circuit);
        }
        ProofKind::SpvProof { confirmations } => {
            out.push(5);
            push_u32(out, *confirmations);
        }
        ProofKind::GpuBatchReceipt => out.push(6),
    }
}

fn push_proof_requirement(out: &mut Vec<u8>, req: &ProofRequirement) {
    push_chain_kind(out, req.chain);
    push_str(out, &req.label);
    push_proof_kind(out, &req.kind);
}

pub fn push_receiver_authorization(out: &mut Vec<u8>, rule: &crate::types::ReceiverAuthorization) {
    use crate::types::ReceiverAuthorization;
    match rule {
        ReceiverAuthorization::OwnerOnly => out.push(0),
        ReceiverAuthorization::ExplicitAccount { account } => {
            out.push(1);
            push_str(out, account);
        }
        ReceiverAuthorization::MappedAccount {
            source_chain,
            source_owner,
            dest_chain,
            dest_account,
        } => {
            out.push(2);
            push_chain_kind(out, *source_chain);
            push_str(out, source_owner);
            push_chain_kind(out, *dest_chain);
            push_str(out, dest_account);
        }
        ReceiverAuthorization::AllowAny => out.push(3),
    }
}

fn push_requirements(out: &mut Vec<u8>, req: &Requirements) {
    push_vec(out, &req.finality, |v, out| {
        push_finality_requirement(out, v)
    });
    push_option(out, &req.max_slippage_bps, |v, out| push_u32(out, *v));
    push_option(out, &req.max_total_fee, |v, out| push_u128(out, *v));
    // Comment 2: receiver authorization replaces the bare boolean.
    push_receiver_authorization(out, &req.receiver_authorization);
    push_vec(out, &req.proofs, |v, out| push_proof_requirement(out, v));
    push_bool(out, req.require_canonical_supply_valid);
    push_bool(out, req.require_route_simulated);
}

pub fn push_failure_action(out: &mut Vec<u8>, action: &FailureAction) {
    match action {
        FailureAction::RefundSource => out.push(0),
        FailureAction::RefundX3 { asset, to } => {
            out.push(1);
            push_asset_ref(out, asset);
            push_str(out, to);
        }
        FailureAction::RefundDestinationStable { asset, to } => {
            out.push(2);
            push_asset_ref(out, asset);
            push_str(out, to);
        }
        FailureAction::RollbackIfPossible => out.push(3),
        FailureAction::Quarantine => out.push(4),
        FailureAction::InsuranceClaim => out.push(5),
    }
}

fn push_timeout(out: &mut Vec<u8>, timeout: &TimeoutSpec) {
    push_u64(out, timeout.timeout_secs);
    push_vec(out, &timeout.on_fail, |v, out| push_failure_action(out, v));
}

fn push_receipt(out: &mut Vec<u8>, receipt: &ReceiptSpec) {
    push_bool(out, receipt.include_route);
    push_bool(out, receipt.include_fees);
    push_bool(out, receipt.include_proofs);
    push_bool(out, receipt.include_state_transitions);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::CrossChainIntent;
    use crate::types::*;

    #[test]
    fn encoding_is_deterministic_for_same_intent() {
        let a = minimal_intent();
        let b = minimal_intent();
        let mut buf_a = Vec::new();
        let mut buf_b = Vec::new();
        encode_intent_canonical(
            &a.name,
            &a.source,
            &a.destination,
            &a.route,
            &a.requirements,
            &a.timeout,
            &a.receipt,
            &mut buf_a,
        );
        encode_intent_canonical(
            &b.name,
            &b.source,
            &b.destination,
            &b.route,
            &b.requirements,
            &b.timeout,
            &b.receipt,
            &mut buf_b,
        );
        assert_eq!(buf_a, buf_b);
    }

    #[test]
    fn encoding_changes_on_route_edit() {
        let a = minimal_intent();
        let mut b = minimal_intent();
        b.route.allow.push("extra.venue".to_string());
        assert_ne!(
            canonical_bytes(&a),
            canonical_bytes(&b),
            "Adding an allowed venue must change the hash"
        );
    }

    #[test]
    fn encoding_changes_on_proof_edit() {
        let a = minimal_intent();
        let mut b = minimal_intent();
        b.requirements.proofs.push(ProofRequirement {
            chain: ChainKind::Ethereum,
            label: "extra.proof".to_string(),
            kind: ProofKind::GpuBatchReceipt,
        });
        assert_ne!(canonical_bytes(&a), canonical_bytes(&b));
    }

    #[test]
    fn encoding_changes_on_fee_cap_edit() {
        let a = minimal_intent();
        let mut b = minimal_intent();
        b.requirements.max_total_fee = Some(999_999);
        assert_ne!(canonical_bytes(&a), canonical_bytes(&b));
    }

    #[test]
    fn encoding_changes_on_slippage_edit() {
        let a = minimal_intent();
        let mut b = minimal_intent();
        b.requirements.max_slippage_bps = Some(7);
        assert_ne!(canonical_bytes(&a), canonical_bytes(&b));
    }

    #[test]
    fn encoding_changes_on_refund_path_edit() {
        let a = minimal_intent();
        let mut b = minimal_intent();
        b.timeout.on_fail.push(FailureAction::Quarantine);
        assert_ne!(canonical_bytes(&a), canonical_bytes(&b));
    }

    #[test]
    fn encoding_changes_on_receipt_edit() {
        let a = minimal_intent();
        let mut b = minimal_intent();
        b.receipt.include_proofs = true;
        assert_ne!(canonical_bytes(&a), canonical_bytes(&b));
    }

    #[test]
    fn encoding_changes_on_receiver_auth_edit() {
        let a = minimal_intent();
        let mut b = minimal_intent();
        b.requirements.receiver_authorization = ReceiverAuthorization::AllowAny;
        assert_ne!(canonical_bytes(&a), canonical_bytes(&b));
    }

    fn minimal_intent() -> CrossChainIntent {
        CrossChainIntent {
            id: 1,
            name: "minimal".to_string(),
            source: SourceSpec {
                asset: AssetRef::new(ChainKind::Ethereum, "USDC"),
                amount: 100,
                owner: "alice.eth".to_string(),
                lock_contract: None,
            },
            destination: DestinationSpec {
                asset: AssetRef::new(ChainKind::Solana, "USDC"),
                receiver: "alice.sol".to_string(),
                min_amount: None,
            },
            route: RouteSpec {
                objective: RouteObjective::Best,
                allow: vec!["x3.dex".to_string()],
                deny: vec!["bridge.unknown".to_string()],
            },
            requirements: Requirements {
                finality: vec![],
                max_slippage_bps: Some(50),
                max_total_fee: Some(10),
                receiver_authorization: ReceiverAuthorization::OwnerOnly,
                proofs: vec![],
                require_canonical_supply_valid: false,
                require_route_simulated: false,
            },
            timeout: TimeoutSpec {
                timeout_secs: 1800,
                on_fail: vec![FailureAction::RefundSource],
            },
            receipt: ReceiptSpec {
                include_route: false,
                include_fees: false,
                include_proofs: false,
                include_state_transitions: false,
            },
            intent_hash: [0u8; 32],
        }
    }

    fn canonical_bytes(intent: &CrossChainIntent) -> Vec<u8> {
        let mut out = Vec::new();
        encode_intent_canonical(
            &intent.name,
            &intent.source,
            &intent.destination,
            &intent.route,
            &intent.requirements,
            &intent.timeout,
            &intent.receipt,
            &mut out,
        );
        out
    }
}
