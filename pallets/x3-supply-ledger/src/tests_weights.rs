// SPDX-License-Identifier: Apache-2.0
//
// tests_weights.rs — dispatch weights are part of the security surface.
//
// A zero weight means a free extrinsic. cargo-mutants replaced every `Weight` body — in both
// `SubstrateWeight` and the `()` compatibility impl — with `Default::default()` and the whole
// suite stayed green, because nothing asserted the one property that matters for every call:
// a dispatchable must never be weightless. This checks both impls.

use crate::mock::Test;
use crate::weights::{SubstrateWeight, WeightInfo};

/// No dispatch through `W` may be free. `Default::default()` is exactly that.
fn assert_all_weighted<W: WeightInfo>() {
    let weights = [
        ("mint_canonical", W::mint_canonical()),
        ("burn_canonical", W::burn_canonical()),
        (
            "set_invariant_violation_policy",
            W::set_invariant_violation_policy(),
        ),
        ("halt_transfers", W::halt_transfers()),
        ("resume_transfers", W::resume_transfers()),
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
