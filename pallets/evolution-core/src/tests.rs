//! Tests for the evolution-core pallet.
//!
//! This file used to hold a single `#[test] fn stub()` whose body explained it
//! existed "so formatting and builds can run" — a test that asserted nothing,
//! which is worse than no test module because it makes the pallet look covered.

use crate::pallet::EvolvableParams;
use sp_runtime::Percent;

/// The execution weights are percentages of the same budget: if they do not
/// sum to 100 the pallet silently mis-allocates execution between VMs.
#[test]
fn default_execution_weights_sum_to_one_hundred() {
    let params = EvolvableParams::default();
    let total = params.evm_weight().deconstruct() + params.svm_weight().deconstruct();
    assert_eq!(total, 100, "EVM + SVM weight must be 100%");
}

#[test]
fn weights_are_read_from_the_stored_percentages() {
    let params = EvolvableParams {
        evm_weight_pct: 70,
        svm_weight_pct: 30,
        ..EvolvableParams::default()
    };
    assert_eq!(params.evm_weight(), Percent::from_percent(70));
    assert_eq!(params.svm_weight(), Percent::from_percent(30));
}
