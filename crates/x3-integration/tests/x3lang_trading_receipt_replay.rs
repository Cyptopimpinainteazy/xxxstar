//! Receipt and replay suite: a signed trading receipt is only accepted when a registered attestor
//! signed exactly these bytes **and** re-executing the artifact against the receipt's own
//! transcript re-derives every figure. Totals in the receipt are never trusted.

#![cfg(all(feature = "std", feature = "compile"))]

mod trading_support;

use trading_support::*;
use x3_x3_integration::trading::{
    execute_trade, replay_receipt, sign_receipt, AttestorKey, ReplayError, SignedTradingReceipt,
};
use x3bc_core::receipt::CostKind;

const GAS_LIMIT: u64 = 1_000_000;

fn honest() -> (Vec<u8>, SignedTradingReceipt) {
    let artifact = compile(ARB_SOURCE);
    let run = execute_trade(&artifact.bytes, &pre_state(), &anchor(), GAS_LIMIT).expect("runs");
    assert!(run.committed);
    let signed = sign_receipt(&run.receipt, &attestor());
    (artifact.bytes, signed)
}

#[test]
fn an_honest_receipt_replays() {
    let (bytes, signed) = honest();
    replay_receipt(&bytes, &signed, &registry()).expect("honest receipt");
}

#[test]
fn a_provisional_receipt_without_finality_replays() {
    let (bytes, signed) = honest();
    let mut receipt = signed.receipt;
    receipt.anchor.finality = None;
    let provisional = sign_receipt(&receipt, &attestor());

    replay_receipt(&bytes, &provisional, &registry())
        .expect("a provisional receipt need not include finality");
}

#[test]
fn a_resigned_execution_height_mismatch_is_refused() {
    let (bytes, signed) = honest();
    let mut receipt = signed.receipt;
    // Keep finality consistent with the anchor, isolating the contradiction
    // between the claimed execution block and the VM's actual pre-state block.
    receipt.anchor.block_number += 1;
    receipt
        .anchor
        .finality
        .as_mut()
        .expect("finalized fixture")
        .block_number = receipt.anchor.block_number;
    let forged = sign_receipt(&receipt, &attestor());

    assert!(matches!(
        replay_receipt(&bytes, &forged, &registry()),
        Err(ReplayError::Inconsistent("anchor block number"))
    ));
}

#[test]
fn a_resigned_finality_height_mismatch_is_refused() {
    let (bytes, signed) = honest();
    let mut receipt = signed.receipt;
    receipt
        .anchor
        .finality
        .as_mut()
        .expect("finalized fixture")
        .block_number += 1;
    let forged = sign_receipt(&receipt, &attestor());

    assert!(matches!(
        replay_receipt(&bytes, &forged, &registry()),
        Err(ReplayError::Inconsistent("finality block number"))
    ));
}

#[test]
fn a_large_non_profit_asset_balance_replays_without_signed_overflow() {
    // WETX is not the profit asset. Its opening balance fits u128 but approaches
    // i128::MAX; the first swap credits it past that boundary before the second
    // swap spends its output. Replay must use checked unsigned balances here.
    let artifact = compile(ARB_SOURCE);
    let mut pre = pre_state();
    pre.trader_balances.insert(WETX, i128::MAX as u128 - 10);
    let run = execute_trade(&artifact.bytes, &pre, &anchor(), GAS_LIMIT)
        .expect("the chain VM must run the trade");
    assert!(
        run.committed,
        "the production engine committed this valid trade"
    );
    let signed = sign_receipt(&run.receipt, &attestor());
    replay_receipt(&artifact.bytes, &signed, &registry())
        .expect("economic replay must not overflow for a valid u128 balance");
}

#[test]
fn a_modified_receipt_breaks_the_signature() {
    let (bytes, mut signed) = honest();
    signed.receipt.profit.net += 1;
    assert!(matches!(
        replay_receipt(&bytes, &signed, &registry()),
        Err(ReplayError::BadSignature)
    ));
}

#[test]
fn a_re_signed_forged_profit_is_caught_by_recomputation() {
    // The attestor key is honest but the figures are not: the signature is valid over a receipt
    // whose profit the execution never produced. Only re-execution can catch this.
    let (bytes, mut signed) = honest();
    signed.receipt.profit.net += 1_000 * UNIT as i128;
    let forged = sign_receipt(&signed.receipt, &attestor());
    match replay_receipt(&bytes, &forged, &registry()) {
        Err(ReplayError::Diverged { field }) => assert_eq!(field, "profit"),
        other => panic!("expected divergence on profit, got {other:?}"),
    }
}

#[test]
fn a_re_signed_dropped_cost_is_caught_by_recomputation() {
    let (bytes, mut signed) = honest();
    signed
        .receipt
        .costs
        .retain(|cost| cost.kind != CostKind::RelayFee);
    let forged = sign_receipt(&signed.receipt, &attestor());
    match replay_receipt(&bytes, &forged, &registry()) {
        Err(ReplayError::Diverged { field }) => assert_eq!(field, "costs"),
        other => panic!("expected divergence on costs, got {other:?}"),
    }
}

#[test]
fn a_re_signed_altered_transcript_input_is_caught() {
    // Change the seeded pool depth the receipt claims the trade saw: replay from the altered input
    // produces different reserves and profit than the receipt records.
    let (bytes, mut signed) = honest();
    signed.receipt.pre_state.venues[0].reserve_b += 1_000 * UNIT;
    let forged = sign_receipt(&signed.receipt, &attestor());
    assert!(matches!(
        replay_receipt(&bytes, &forged, &registry()),
        Err(ReplayError::Diverged { .. })
    ));
}

#[test]
fn a_receipt_for_another_artifact_is_refused() {
    let (_, signed) = honest();
    let other = compile(&ARB_SOURCE.replace("net_profit >= 1 USDX", "net_profit >= 2 USDX"));
    assert!(matches!(
        replay_receipt(&other.bytes, &signed, &registry()),
        Err(ReplayError::ArtifactMismatch)
    ));
}

#[test]
fn an_unknown_attestor_is_refused() {
    let (bytes, signed) = honest();
    let stranger = AttestorKey::from_seed("x3-attestor-unknown", [9u8; 32]);
    let resigned = sign_receipt(&signed.receipt, &stranger);
    assert!(matches!(
        replay_receipt(&bytes, &resigned, &registry()),
        Err(ReplayError::UnknownAttestor)
    ));
}

#[test]
fn a_known_attestor_id_with_the_wrong_key_is_refused() {
    let (bytes, signed) = honest();
    // Same id as the registered attestor, different key.
    let impostor = AttestorKey::from_seed("x3-attestor-1", [9u8; 32]);
    let resigned = sign_receipt(&signed.receipt, &impostor);
    assert!(matches!(
        replay_receipt(&bytes, &resigned, &registry()),
        Err(ReplayError::UnknownAttestor) | Err(ReplayError::BadSignature)
    ));
}

#[test]
fn a_revoked_attestor_is_refused() {
    let (bytes, signed) = honest();
    let mut registry = registry();
    registry.revoke("x3-attestor-1");
    assert!(matches!(
        replay_receipt(&bytes, &signed, &registry),
        Err(ReplayError::UnknownAttestor)
    ));
}

#[test]
fn receipt_encoding_is_canonical_and_hash_bound() {
    let (_, signed) = honest();
    let a = x3bc_core::receipt::encode(&signed.receipt);
    let b = x3bc_core::receipt::encode(&signed.receipt.clone());
    assert_eq!(a, b, "canonical encoding is deterministic");
    let decoded = x3bc_core::receipt::decode(&a).expect("round trips");
    assert_eq!(decoded, signed.receipt);
    for cut in [0, 1, a.len() / 2, a.len() - 1] {
        assert!(
            x3bc_core::receipt::decode(&a[..cut]).is_err(),
            "truncation at {cut} refused"
        );
    }
}
