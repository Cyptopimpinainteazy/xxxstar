// SPDX-License-Identifier: Apache-2.0
//
// tests_weights.rs — dispatch weights are part of the security surface.
//
// A zero weight means a free extrinsic. cargo-mutants replaced every `Weight` body — in both
// `SubstrateWeight` and the `()` compatibility impl — with `Default::default()` and the whole
// suite stayed green, because nothing asserted the one property that matters for every call:
// a dispatchable must never be weightless. `submit_atomic_bundle` additionally scales with the
// leg count, so a constant body would under-charge many-leg bundles. This checks both impls.

use crate::mock::Test;
use crate::weights::{SubstrateWeight, WeightInfo};

/// No dispatch through `W` may be free, and `submit_atomic_bundle` must stay non-decreasing in
/// the leg count: a constant body would under-charge bundles with many legs.
fn assert_all_weighted<W: WeightInfo>() {
    let weights = [
        ("submit_atomic_bundle(1)", W::submit_atomic_bundle(1)),
        ("finalize_atomic_bundle", W::finalize_atomic_bundle()),
        ("rollback_atomic_bundle", W::rollback_atomic_bundle()),
        ("assign_bundle_executor", W::assign_bundle_executor()),
        (
            "record_flash_finality_anchor",
            W::record_flash_finality_anchor(),
        ),
        (
            "record_leg_execution_receipt",
            W::record_leg_execution_receipt(),
        ),
    ];
    for (name, weight) in weights {
        assert!(
            weight.ref_time() > 0,
            "{name} returned a zero weight: the call would be free"
        );
    }
    let mut previous = 0;
    for legs in 1..=8u32 {
        let weight = W::submit_atomic_bundle(legs).ref_time();
        assert!(
            weight >= previous,
            "submit_atomic_bundle({legs}) weighs less than with fewer legs: the per-leg cost is gone"
        );
        previous = weight;
    }
}

#[test]
fn every_dispatch_weight_is_nonzero() {
    assert_all_weighted::<SubstrateWeight<Test>>();
    assert_all_weighted::<()>();
}
