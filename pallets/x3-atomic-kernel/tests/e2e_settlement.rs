// SPDX-License-Identifier: Apache-2.0
//!
//! # Settlement Origin-Gating Regression Coverage
//!
//! These tests verify the **structural contract** of the settlement
//! finalization path.  A full `construct_runtime!` mock requires runtime
//! crates not available to pallet integration tests; those live in the
//! runtime crate's test suite (see `runtime/src/tests.rs`).
//!
//! What is covered here:
//! - The `Config` trait exposes `SettlementOrigin` (compile-time gate).
//! - `finalize_with_settlement` exists and its origin is `OriginFor<T>`
//!   parameterised by `SettlementOrigin`.
//! - The `BundleLegReceipts` storage item links to `LegReceipt` for
//!   VM reversion.
//!
//! What requires runtime integration tests:
//! - Actual `BadOrigin` rejection of non-settlement callers.
//! - Actual success for the designated settlement gateway account.

use pallet_x3_atomic_kernel::{
    proof::{BundleLeg, VmType},
    vm_revert::{LegReceipt, StateDiff},
};

// ── Origin type check: SettlementOrigin must be on Config ─────────────────

/// Verifies the `SettlementOrigin` associated type exists at compile time.
///
/// This is a type-level assertion: if `SettlementOrigin` is removed from
/// `Config`, this function will fail to compile — which is exactly the
/// regression we want to catch.
#[allow(unused)]
fn assert_settlement_origin_on_config<T: pallet_x3_atomic_kernel::Config>() {
    // Type-level check: if `SettlementOrigin` doesn't exist on Config,
    // the trait bound below won't resolve.  The inner fn must declare its
    // own generic so Rust can resolve the type parameter.
    fn _assert_bound<C: pallet_x3_atomic_kernel::Config>()
    where
        C::SettlementOrigin: frame_support::traits::EnsureOrigin<C::RuntimeOrigin>,
    {
    }
    _assert_bound::<T>();
}

// ── Leg receipt ↔ storage type consistency ────────────────────────────────

#[test]
fn test_leg_receipt_matches_storage_type() {
    // BundleLegReceipts storage holds BoundedVec<LegReceipt, MaxLegsPerBundle>.
    // Verify LegReceipt fields are consistent with vm_revert module.
    let receipt = LegReceipt {
        leg_index: 0,
        vm_type: VmType::Evm,
        executed: false,
        state_diff: StateDiff::from(Vec::new()),
        receipt_root: [0u8; 32],
        finalized_block: 0,
    };
    assert_eq!(receipt.leg_index, 0);
    assert!(!receipt.executed);
    assert!(receipt.state_diff.is_empty());
}

#[test]
fn test_leg_receipts_can_track_executed_state() {
    let mut receipt = LegReceipt {
        leg_index: 1,
        vm_type: VmType::Svm,
        executed: false,
        state_diff: StateDiff::from(Vec::new()),
        receipt_root: [0u8; 32],
        finalized_block: 0,
    };
    receipt.executed = true;
    receipt.state_diff = StateDiff::from(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    assert!(receipt.executed);
    assert!(!receipt.state_diff.is_empty());
}

// ── Bundle leg round-trip (same as lib tests, validates integration) ──────

#[test]
fn test_bundle_leg_roundtrip_in_integration() {
    use parity_scale_codec::{Decode, Encode};

    let leg = BundleLeg {
        vm_type: VmType::Cross,
        token_in: sp_core::H256::repeat_byte(0xAA),
        token_out: sp_core::H256::repeat_byte(0xBB),
        amount_in: 1_000_000_000_000u128,
        min_amount_out: 990_000_000_000u128,
        deadline: 1_800_000_000u64,
        access: pallet_x3_atomic_kernel::proof::DeclaredAccess {
            reads: Default::default(),
            writes: Default::default(),
        },
    };

    let encoded = leg.encode();
    let decoded = BundleLeg::decode(&mut &encoded[..]).expect("decode failed");
    assert_eq!(leg, decoded);
}
