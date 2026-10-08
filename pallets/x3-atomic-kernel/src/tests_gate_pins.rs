// SPDX-License-Identifier: Apache-2.0
//
// Mutation-gate pins for lib.rs.
//
// The first pallet-wide campaign left dozens of mutants alive in lib.rs, all of
// them in behavior the suite never executed — the off-chain worker, auto-expiry
// in `on_initialize`, `ValidateUnsigned`, the revert-failure counter, and the
// read-only getters — plus a few log-only conditions that no test can observe
// through state. The unpinnable log guards were folded into the adjacent
// unconditional log lines in lib.rs; everything else is pinned here by driving
// each path through its real entry point and asserting exact values.
//
// Writing these pins surfaced a real defect: `on_initialize` slashed the
// expiry penalty without ever unreserving the rest of the bond, and because
// `RolledBack` is terminal nothing could ever release it. The fix (unreserve
// before slash, mirroring `do_rollback_atomic_bundle`) is pinned by
// `on_initialize_expires_bundles_and_returns_the_unslashed_bond`.

use crate::mock::{
    new_test_ext, new_test_ext_offchain, vm_revert_failing, vm_revert_ok, AtomicKernel,
    MaxLegsPerBundle, RuntimeCall, RuntimeEvent, RuntimeOrigin, System, Test, UncheckedExtrinsic,
    ALICE, BOB, MIN_BOND,
};
use crate::proof::{BundleLeg, DeclaredAccess, PoaeProof, VmType};
use crate::vm_revert::{LegReceipt, StateDiff};
use crate::{
    BundleLegReceipts, BundleRollbackReason, BundleStatus, Bundles, Call, DeadlineIndex, Event,
    PoaeProofs,
};
use frame_support::{
    assert_ok, pallet_prelude::ValidateUnsigned, traits::Hooks, weights::Weight, BoundedVec,
};
use parity_scale_codec::{Decode, Encode};
use sp_core::offchain::StorageKind;
use sp_core::H256;
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
    TransactionValidityError, ValidTransaction,
};

type Balances = pallet_balances::Pallet<Test>;

// ── Fixtures ───────────────────────────────────────────────────────────────

fn one_leg(amount_in: u128) -> BoundedVec<BundleLeg, MaxLegsPerBundle> {
    BoundedVec::try_from(vec![BundleLeg {
        vm_type: VmType::X3,
        token_in: H256::repeat_byte(0x11),
        token_out: H256::repeat_byte(0x22),
        amount_in,
        min_amount_out: 900,
        deadline: 4_000_000_000,
        access: DeclaredAccess {
            reads: BoundedVec::try_from(vec![H256::repeat_byte(0x33)]).expect("reads fit"),
            writes: BoundedVec::try_from(vec![H256::repeat_byte(0x44)]).expect("writes fit"),
        },
    }])
    .expect("a single leg fits MaxLegsPerBundle")
}

/// Submit one bundle as ALICE on the current block; `amount_in` varies the legs
/// hash so two bundles in one test get distinct ids.
fn submit_one(amount_in: u128, deadline_blocks: u64, nonce: u64) -> H256 {
    let legs = one_leg(amount_in);
    let now = System::block_number();
    let legs_hash = H256(sp_io::hashing::sha2_256(&legs.encode()));
    let bundle_id = AtomicKernel::derive_bundle_id(&ALICE, now, legs_hash);
    assert_ok!(AtomicKernel::submit_atomic_bundle(
        RuntimeOrigin::signed(ALICE),
        legs,
        deadline_blocks,
        1,
        nonce,
    ));
    bundle_id
}

fn mark_executing(bundle_id: H256) {
    Bundles::<Test>::mutate(bundle_id, |maybe| {
        let record = maybe.as_mut().expect("bundle stored");
        record.status = BundleStatus::Executing;
        record.executor = Some(ALICE);
    });
}

fn receipt(leg_index: u32, executed: bool, bytes: Vec<u8>) -> LegReceipt {
    LegReceipt {
        leg_index,
        vm_type: VmType::X3,
        executed,
        state_diff: StateDiff::from(bytes),
        receipt_root: [0u8; 32],
        finalized_block: 0,
    }
}

fn set_receipts(bundle_id: H256, receipts: Vec<LegReceipt>) {
    BundleLegReceipts::<Test>::insert(
        bundle_id,
        BoundedVec::try_from(receipts).expect("receipts fit MaxLegsPerBundle"),
    );
}

fn expect_valid(result: TransactionValidity) -> ValidTransaction {
    result.expect("expected a valid transaction")
}

fn expect_invalid(result: TransactionValidity, expected: InvalidTransaction) {
    match result {
        Err(TransactionValidityError::Invalid(actual)) => assert_eq!(actual, expected),
        other => panic!("expected Invalid({expected:?}), got {other:?}"),
    }
}

// ── offchain_worker ────────────────────────────────────────────────────────

fn ff_key(block: u64) -> Vec<u8> {
    let mut key = b"x3ff:".to_vec();
    key.extend_from_slice(&block.to_le_bytes());
    key
}

fn leg_key(bundle_id: H256, leg_index: u32) -> Vec<u8> {
    let mut key = b"x3leg:".to_vec();
    key.extend_from_slice(bundle_id.as_bytes());
    key.extend_from_slice(&leg_index.to_le_bytes());
    key
}

fn set_persistent(key: &[u8], value: &[u8]) {
    sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, key, value);
}

fn get_persistent(key: &[u8]) -> Option<Vec<u8>> {
    sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, key)
}

fn decode_call(bytes: &[u8]) -> RuntimeCall {
    UncheckedExtrinsic::decode(&mut &bytes[..])
        .expect("submitted extrinsics must decode")
        .function
}

fn is_anchor(call: &RuntimeCall, block: u64, cert: H256) -> bool {
    matches!(
        call,
        RuntimeCall::AtomicKernel(Call::record_flash_finality_anchor {
            block_num,
            cert: anchored,
        }) if *block_num == block && *anchored == cert
    )
}

fn is_leg_receipt(call: &RuntimeCall, bundle_id: H256, leg_index: u32) -> bool {
    matches!(
        call,
        RuntimeCall::AtomicKernel(Call::record_leg_execution_receipt {
            bundle_id: id,
            leg_index: index,
            ..
        }) if *id == bundle_id && *index == leg_index
    )
}

#[test]
fn ocw_anchors_a_flash_cert_and_submits_leg_receipts() {
    let (mut ext, submitted) = new_test_ext_offchain();
    ext.execute_with(|| {
        System::set_block_number(7);
        let bundle_id = submit_one(1_000, 50, 1);
        let key = leg_key(bundle_id, 0);
        let encoded = StateDiff::from(vec![0xAB]).encode();
        set_persistent(&key, &encoded);

        // No cert, bundle still Pending: nothing is submitted and the leg value
        // must survive untouched (the scan only reads Executing bundles).
        AtomicKernel::offchain_worker(7);
        assert_eq!(submitted().len(), 0);
        assert_eq!(get_persistent(&key), Some(encoded.clone()));

        // Cert for this block + Executing bundle: one anchor, one receipt.
        let cert = H256::repeat_byte(0x5A);
        set_persistent(&ff_key(7), cert.as_bytes());
        mark_executing(bundle_id);
        AtomicKernel::offchain_worker(7);

        let transactions = submitted();
        assert_eq!(
            transactions.len(),
            2,
            "one finality anchor + one leg receipt"
        );
        let calls: Vec<RuntimeCall> = transactions.iter().map(|tx| decode_call(tx)).collect();
        assert!(
            calls.iter().any(|call| is_anchor(call, 7, cert)),
            "the 32-byte cert must be anchored"
        );
        assert!(
            calls.iter().any(|call| is_leg_receipt(call, bundle_id, 0)),
            "the Executing bundle's leg receipt must be submitted"
        );
        assert_eq!(
            get_persistent(&key),
            None,
            "an accepted leg value is cleared"
        );
    });
}

#[test]
fn ocw_accepts_only_exactly_32_byte_nonzero_certs() {
    let (mut ext, submitted) = new_test_ext_offchain();
    ext.execute_with(|| {
        System::set_block_number(3);
        set_persistent(&ff_key(3), &[0x11u8; 31]);
        set_persistent(&ff_key(4), &[0x00u8; 32]);

        AtomicKernel::offchain_worker(3);
        AtomicKernel::offchain_worker(4);
        assert_eq!(
            submitted().len(),
            0,
            "short and zero certs are ignored, not anchored"
        );

        let cert = H256::repeat_byte(0x77);
        set_persistent(&ff_key(4), cert.as_bytes());
        set_persistent(&ff_key(5), &[0x22u8; 33]);
        AtomicKernel::offchain_worker(4);
        AtomicKernel::offchain_worker(5);

        let transactions = submitted();
        assert_eq!(transactions.len(), 1, "only the exact 32-byte cert anchors");
        assert!(is_anchor(&decode_call(&transactions[0]), 4, cert));
    });
}

#[test]
fn ocw_skips_empty_leg_values_and_clears_corrupt_diffs() {
    let (mut ext, submitted) = new_test_ext_offchain();
    ext.execute_with(|| {
        System::set_block_number(2);
        let good = submit_one(1_000, 50, 1);
        let empty = submit_one(2_000, 50, 2);
        let corrupt = submit_one(3_000, 50, 3);
        mark_executing(good);
        mark_executing(empty);
        mark_executing(corrupt);

        let good_key = leg_key(good, 0);
        let empty_key = leg_key(empty, 0);
        let corrupt_key = leg_key(corrupt, 0);
        let encoded = StateDiff::from(vec![0x01]).encode();
        set_persistent(&good_key, &encoded);
        set_persistent(&empty_key, b"");
        // 0xFF starts a four-byte SCALE compact length with only one byte left.
        set_persistent(&corrupt_key, &[0xFF, 0x01]);

        AtomicKernel::offchain_worker(2);

        let transactions = submitted();
        assert_eq!(
            transactions.len(),
            1,
            "only the decodable non-empty diff is submitted"
        );
        assert!(is_leg_receipt(&decode_call(&transactions[0]), good, 0));
        assert_eq!(
            get_persistent(&good_key),
            None,
            "the accepted key is cleared"
        );
        assert_eq!(
            get_persistent(&empty_key),
            Some(Vec::new()),
            "an empty value is skipped, not cleared"
        );
        assert_eq!(
            get_persistent(&corrupt_key),
            None,
            "a corrupt diff is cleared so the OCW does not retry it forever"
        );
    });
}

// ── ValidateUnsigned ───────────────────────────────────────────────────────

#[test]
fn validate_unsigned_pins_anchor_zero_cert_window_and_priority() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1_000);
        let anchor = |block_num: u64, cert: H256| Call::<Test>::record_flash_finality_anchor {
            block_num,
            cert,
        };

        expect_invalid(
            AtomicKernel::validate_unsigned(
                TransactionSource::External,
                &anchor(1_000, H256::zero()),
            ),
            InvalidTransaction::BadProof,
        );

        // Exactly the future edge (+5) is valid; one block past it is Future.
        let valid = expect_valid(AtomicKernel::validate_unsigned(
            TransactionSource::External,
            &anchor(1_005, H256::repeat_byte(0xAB)),
        ));
        assert_eq!(
            valid.priority,
            TransactionPriority::MAX / 8,
            "anchor priority is pinned"
        );
        expect_invalid(
            AtomicKernel::validate_unsigned(
                TransactionSource::External,
                &anchor(1_006, H256::repeat_byte(0xAB)),
            ),
            InvalidTransaction::Future,
        );

        // Exactly the staleness edge (-50) is valid; one block older is Stale.
        expect_valid(AtomicKernel::validate_unsigned(
            TransactionSource::External,
            &anchor(950, H256::repeat_byte(0xAB)),
        ));
        expect_invalid(
            AtomicKernel::validate_unsigned(
                TransactionSource::External,
                &anchor(949, H256::repeat_byte(0xAB)),
            ),
            InvalidTransaction::Stale,
        );
    });
}

#[test]
fn validate_unsigned_pins_leg_receipt_guards_and_priority() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        let bundle_id = submit_one(1_000, 50, 1);
        let leg_receipt = |leg_index: u32| Call::<Test>::record_leg_execution_receipt {
            bundle_id,
            leg_index,
            state_diff: StateDiff::from(Vec::new()),
        };
        let validate =
            |call: &Call<Test>| AtomicKernel::validate_unsigned(TransactionSource::External, call);

        // Pending without an executor: refused.
        expect_invalid(validate(&leg_receipt(0)), InvalidTransaction::Stale);

        // Pending *with* an executor assigned: the conjunction must still refuse.
        Bundles::<Test>::mutate(bundle_id, |maybe| {
            maybe.as_mut().expect("stored").executor = Some(ALICE);
        });
        expect_invalid(validate(&leg_receipt(0)), InvalidTransaction::Stale);

        // Executing without an executor: refused.
        Bundles::<Test>::mutate(bundle_id, |maybe| {
            let record = maybe.as_mut().expect("stored");
            record.status = BundleStatus::Executing;
            record.executor = None;
        });
        expect_invalid(validate(&leg_receipt(0)), InvalidTransaction::Stale);

        // Executing with an executor: valid, at the leg-receipt priority.
        Bundles::<Test>::mutate(bundle_id, |maybe| {
            maybe.as_mut().expect("stored").executor = Some(ALICE);
        });
        let valid = expect_valid(validate(&leg_receipt(0)));
        assert_eq!(valid.priority, TransactionPriority::MAX / 2);

        // receipts.len() == 1 here, so index 1 is one past the end.
        expect_invalid(validate(&leg_receipt(1)), InvalidTransaction::Stale);
        expect_invalid(validate(&leg_receipt(999)), InvalidTransaction::Stale);

        // An already-executed leg cannot be recorded twice.
        let mut receipts = BundleLegReceipts::<Test>::get(bundle_id).to_vec();
        receipts[0].executed = true;
        set_receipts(bundle_id, receipts);
        expect_invalid(validate(&leg_receipt(0)), InvalidTransaction::Stale);

        // Unknown bundles are refused.
        let missing = Call::<Test>::record_leg_execution_receipt {
            bundle_id: H256::repeat_byte(0xEE),
            leg_index: 0,
            state_diff: StateDiff::from(Vec::new()),
        };
        expect_invalid(validate(&missing), InvalidTransaction::Stale);
    });
}

// ── do_revert_bundle_legs (through rollback) ───────────────────────────────

fn incomplete_revert_alerts(bundle_id: H256) -> Vec<(u32, u32)> {
    System::events()
        .iter()
        .filter_map(|record| match &record.event {
            RuntimeEvent::AtomicKernel(Event::IncompleteVmRevert {
                bundle_id: id,
                leg_count,
                failed_count,
            }) if *id == bundle_id => Some((*leg_count, *failed_count)),
            _ => None,
        })
        .collect()
}

#[test]
fn rollback_counts_only_executed_nonempty_revert_failures() {
    new_test_ext().execute_with(|| {
        let _failing = vm_revert_failing();
        System::set_block_number(5);
        let bundle_id = submit_one(1_000, 50, 1);
        set_receipts(
            bundle_id,
            vec![
                receipt(0, true, vec![0x01]),  // executed + diff, revert fails: counted
                receipt(1, true, vec![0x02]),  // executed + diff, revert fails: counted
                receipt(2, false, vec![0x03]), // not executed: skipped
                receipt(3, true, Vec::new()),  // executed but no diff: skipped
            ],
        );

        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            BundleRollbackReason::SubmitterCancelled,
        ));

        assert_eq!(
            incomplete_revert_alerts(bundle_id),
            vec![(4, 2)],
            "exactly one alert, reporting the two real revert failures"
        );
    });
}

#[test]
fn rollback_clean_reverts_emit_no_failure_alert() {
    new_test_ext().execute_with(|| {
        let _succeeding = vm_revert_ok();
        System::set_block_number(5);
        let bundle_id = submit_one(1_000, 50, 1);
        set_receipts(bundle_id, vec![receipt(0, true, vec![0x01])]);

        assert_ok!(AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            bundle_id,
            BundleRollbackReason::SubmitterCancelled,
        ));

        assert!(
            incomplete_revert_alerts(bundle_id).is_empty(),
            "a clean revert must not raise the stale-side-effects alarm"
        );
    });
}

// ── on_initialize ──────────────────────────────────────────────────────────

#[test]
fn on_initialize_expires_bundles_and_returns_the_unslashed_bond() {
    new_test_ext().execute_with(|| {
        let _succeeding = vm_revert_ok();
        System::set_block_number(1);
        let free_at_start = Balances::free_balance(ALICE);
        let issuance_at_start = Balances::total_issuance();

        let pending = submit_one(1_000, 5, 1); // deadline = 6
        let executing = submit_one(2_000, 5, 2); // deadline = 6
        mark_executing(executing);
        assert_eq!(DeadlineIndex::<Test>::get(6).len(), 2);

        System::set_block_number(6);
        let weight = AtomicKernel::on_initialize(6);

        assert_eq!(
            weight,
            Weight::from_parts(20_000, 0),
            "10_000 weight units per processed bundle"
        );
        assert_eq!(
            Bundles::<Test>::get(pending).expect("stored").status,
            BundleStatus::RolledBack
        );
        assert_eq!(
            Bundles::<Test>::get(executing).expect("stored").status,
            BundleStatus::RolledBack
        );

        let penalty = MIN_BOND / 20;
        assert_eq!(
            Balances::free_balance(ALICE),
            free_at_start - 2 * penalty,
            "each submitter keeps the bond minus the 5% penalty"
        );
        assert_eq!(
            Balances::reserved_balance(ALICE),
            0,
            "an expired bond must not stay stranded in reserve"
        );
        assert_eq!(
            Balances::total_issuance(),
            issuance_at_start - 2 * penalty,
            "auto-expiry drops the slash imbalance (burn), unlike rollback's treasury credit"
        );
        assert!(
            DeadlineIndex::<Test>::get(6).is_empty(),
            "the processed deadline index is removed"
        );
        assert!(
            BundleLegReceipts::<Test>::get(pending).is_empty(),
            "leg receipts are cleaned up on expiry"
        );

        let rolled_back = System::events()
            .iter()
            .filter(|record| {
                matches!(
                    &record.event,
                    RuntimeEvent::AtomicKernel(Event::BundleRolledBack {
                        reason: BundleRollbackReason::DeadlineExceeded,
                        ..
                    })
                )
            })
            .count();
        assert_eq!(rolled_back, 2);
    });
}

#[test]
fn on_initialize_leaves_terminal_bundles_untouched() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let finalized = submit_one(1_000, 5, 1); // deadline = 6
        Bundles::<Test>::mutate(finalized, |maybe| {
            maybe.as_mut().expect("stored").status = BundleStatus::Finalized;
        });
        let free_before = Balances::free_balance(ALICE);

        System::set_block_number(6);
        let weight = AtomicKernel::on_initialize(6);

        assert_eq!(weight, Weight::from_parts(0, 0));
        assert_eq!(
            Bundles::<Test>::get(finalized).expect("stored").status,
            BundleStatus::Finalized
        );
        assert_eq!(Balances::free_balance(ALICE), free_before);
        assert_eq!(
            Balances::reserved_balance(ALICE),
            MIN_BOND,
            "a finalizable bond stays reserved"
        );
        assert!(
            DeadlineIndex::<Test>::get(6).is_empty(),
            "the deadline index is cleaned even when nothing is processed"
        );
    });
}

// ── Read-only getters ──────────────────────────────────────────────────────

#[test]
fn read_only_getters_report_exactly_what_is_stored() {
    new_test_ext().execute_with(|| {
        System::set_block_number(4);
        let bundle_id = submit_one(1_000, 50, 1);
        let legs_hash = Bundles::<Test>::get(bundle_id).expect("stored").legs_hash;

        assert_eq!(
            AtomicKernel::bundle_status(bundle_id),
            Some(BundleStatus::Pending)
        );
        assert_eq!(AtomicKernel::bundle_status(H256::repeat_byte(0xEE)), None);

        assert_eq!(
            AtomicKernel::find_bundle(&ALICE, legs_hash),
            Some((bundle_id, BundleStatus::Pending))
        );
        assert_eq!(
            AtomicKernel::find_bundle(&ALICE, H256::repeat_byte(0x99)),
            None,
            "the legs hash must match"
        );
        assert_eq!(
            AtomicKernel::find_bundle(&BOB, legs_hash),
            None,
            "the submitter must match"
        );

        let proof = PoaeProof {
            bundle_id,
            receipt_root: H256::repeat_byte(0xAA),
            finalized_block: 42,
            finality_cert: H256::repeat_byte(0xBB),
            legs_hash,
            leg_count: 1,
        };
        PoaeProofs::<Test>::insert(bundle_id, proof.clone());
        assert_eq!(AtomicKernel::get_poae_proof(bundle_id), Some(proof));
        assert_eq!(AtomicKernel::get_poae_proof(H256::repeat_byte(0xEE)), None);
    });
}
