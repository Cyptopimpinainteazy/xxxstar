//! Property tests for deterministic atomic-kernel value handling.
//!
//! These tests exercise production functions and wire types. They deliberately
//! do not simulate a supply ledger or rollback engine inside the test: doing so
//! would only prove that the simulation agrees with itself.

#![cfg(test)]

use pallet_x3_atomic_kernel::{
    finality_cert_from_block_hash,
    proof::{BundleLeg, DeclaredAccess, VmType},
    vm_revert::{LegReceipt, StateDiff},
};
use parity_scale_codec::{Decode, Encode};
use proptest::prelude::*;
use sp_core::H256;
use sp_runtime::traits::ConstU32;
use sp_std::vec::Vec;

fn vm_type() -> impl Strategy<Value = VmType> {
    prop_oneof![
        Just(VmType::Evm),
        Just(VmType::Svm),
        Just(VmType::X3),
        Just(VmType::Cross),
    ]
}

fn h256() -> impl Strategy<Value = H256> {
    any::<[u8; 32]>().prop_map(H256::from)
}

fn declared_access() -> impl Strategy<Value = DeclaredAccess> {
    (
        prop::collection::vec(h256(), 0..=64),
        prop::collection::vec(h256(), 0..=64),
    )
        .prop_map(|(reads, writes)| DeclaredAccess {
            reads: frame_support::BoundedVec::<H256, ConstU32<64>>::try_from(reads)
                .expect("the strategy respects the declared-access bound"),
            writes: frame_support::BoundedVec::<H256, ConstU32<64>>::try_from(writes)
                .expect("the strategy respects the declared-access bound"),
        })
}

proptest! {
    /// The finality certificate must be a pure function of the finalized block
    /// hash, so every node derives the same anchor for the same header.
    #[test]
    fn finality_certificate_is_deterministic(block_hash in any::<Vec<u8>>()) {
        let first = finality_cert_from_block_hash(&block_hash);
        let second = finality_cert_from_block_hash(&block_hash);
        prop_assert_eq!(first, second);
    }

    /// A one-bit header change must change the certificate. A collision here
    /// would let two finalized blocks share one PoAE anchor.
    #[test]
    fn finality_certificate_commits_to_the_block_hash(block_hash in any::<[u8; 32]>()) {
        let original = finality_cert_from_block_hash(&block_hash);
        let mut changed = block_hash;
        changed[31] ^= 1;
        let changed_cert = finality_cert_from_block_hash(&changed);
        prop_assert_ne!(original, changed_cert);
    }

    /// Every production bundle-leg field must survive SCALE encoding. A lossy
    /// round-trip would alter the atomic operation between validation and
    /// execution.
    #[test]
    fn bundle_leg_scale_roundtrip(
        vm in vm_type(),
        token_in in h256(),
        token_out in h256(),
        amount_in in any::<u128>(),
        min_amount_out in any::<u128>(),
        deadline in any::<u64>(),
        access in declared_access(),
    ) {
        let leg = BundleLeg {
            vm_type: vm,
            token_in,
            token_out,
            amount_in,
            min_amount_out,
            deadline,
            access,
        };

        let encoded = leg.encode();
        let decoded = BundleLeg::decode(&mut &encoded[..])
            .expect("an encoded production BundleLeg must decode");
        prop_assert_eq!(leg, decoded);
    }

    /// Leg receipts carry the state delta used by rollback. Their SCALE form
    /// must preserve every byte and metadata field exactly.
    #[test]
    fn leg_receipt_scale_roundtrip(
        leg_index in any::<u32>(),
        vm in vm_type(),
        executed in any::<bool>(),
        state_diff in any::<Vec<u8>>(),
        receipt_root in any::<[u8; 32]>(),
        finalized_block in any::<u64>(),
    ) {
        let receipt = LegReceipt {
            leg_index,
            vm_type: vm,
            executed,
            state_diff: StateDiff::from(state_diff),
            receipt_root,
            finalized_block,
        };

        let encoded = receipt.encode();
        let decoded = LegReceipt::decode(&mut &encoded[..])
            .expect("an encoded production LegReceipt must decode");
        prop_assert_eq!(receipt, decoded);
    }
}
