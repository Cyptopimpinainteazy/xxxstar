// SPDX-License-Identifier: Apache-2.0
//
// tests_weights.rs — dispatch weights are part of the security surface.
//
// The mutation campaign (2026-10-03) replaced every `Weight` body — in both
// `SubstrateWeight` and the `()` compatibility impl — with `Default::default()` and the
// whole suite stayed green, because nothing asserted the one property that matters for
// every call: a dispatchable must never be weightless. A zero weight is a free extrinsic.
// This checks both impls, mirroring `pallets/x3-atomic-kernel/src/tests_weights.rs`.

use crate::mock::Test;
use crate::weights::{SubstrateWeight, WeightInfo};

fn assert_all_weighted<W: WeightInfo>() {
    let weights = [
        ("create_intent", W::create_intent()),
        ("lock_escrow", W::lock_escrow()),
        ("claim_settlement", W::claim_settlement()),
        ("finalize_intent", W::finalize_intent()),
        ("refund_intent", W::refund_intent()),
        ("verify_btc_proof", W::verify_btc_proof()),
        ("update_btc_block_header", W::update_btc_block_header()),
        ("submit_btc_headers", W::submit_btc_headers()),
        ("anchor_btc_checkpoint", W::anchor_btc_checkpoint()),
        ("submit_external_proof", W::submit_external_proof()),
        ("create_bond", W::create_bond()),
        ("claim_bond", W::claim_bond()),
        ("update_finality_config", W::update_finality_config()),
        ("report_violation", W::report_violation()),
        ("settle_transfer", W::settle_transfer()),
        ("trigger_refund", W::trigger_refund()),
        ("submit_adaptor_signature", W::submit_adaptor_signature()),
        ("complete_adaptor_swap", W::complete_adaptor_swap()),
    ];
    for (name, weight) in weights {
        assert!(
            weight.ref_time() > 0,
            "{name} returned a zero weight: the call would be free"
        );
    }
}

#[test]
fn every_dispatch_weight_is_nonzero() {
    assert_all_weighted::<SubstrateWeight<Test>>();
    assert_all_weighted::<()>();
}
