// SPDX-License-Identifier: Apache-2.0
//
// tests_gate_pins.rs — pins for the settlement-engine mutation campaign (2026-10-03).
//
// The campaign changed behaviour across the money path and the suite stayed green: the
// BTC merkle/reconstruction and target maths, the finality oracle, the invariant
// checker itself, the escrow address derivations, and the fail-closed bridge default.
// Each test here drives one entry point through the exact case the mutant would answer
// differently — boundary, sibling position, or branch — so the next regression of any
// of these is a red test instead of a survivor.

use crate::atomic_lock::{AtomicLock, EscrowAccount, LockPhase};
use crate::btc_gateway::{BtcAdaptorSignature, BtcHtlcParams, BtcReorgRisk, BtcSpvProof};
use crate::escrow::{CrossVmEscrow, SvmEscrowParams};
use crate::finality::{FinalityOracle, ReorgDetector, Urgency};
use crate::intent::IntentPlanner;
use crate::invariants::{
    InvariantCheckResult, InvariantConfig, InvariantEnforcer, VmExecutionEvent, VmType,
};
use crate::mock::{new_test_ext, RuntimeOrigin, Test, ALICE, BOB};
use crate::types::{
    AssetSpec, BtcBlockHeader, BtcHeaderMeta, ExternalChainId, IntentState, InvariantViolationType,
    TokenId,
};
use crate::{
    AtomicLockExpiryIndex, AtomicLocks, BtcHeaderMetaStore, BtcHeaders, IntentDeadlineIndex,
    IntentStates, Pallet, SettlementIntents,
};
use frame_support::{assert_ok, traits::Hooks, weights::Weight, BoundedVec};
use sp_core::H256;
use x3_atomic_swap::{CrossDomainOperation, CrossDomainProofSet, VmType as ProofVmType};

fn dsha(left: &[u8], right: &[u8]) -> H256 {
    let mut buf = Vec::with_capacity(left.len() + right.len());
    buf.extend_from_slice(left);
    buf.extend_from_slice(right);
    let first = sp_io::hashing::sha2_256(&buf);
    H256::from(sp_io::hashing::sha2_256(&first))
}

fn btc_header_with_root(merkle_root: H256) -> BtcBlockHeader {
    BtcBlockHeader {
        version: 1,
        prev_block_hash: H256::zero(),
        merkle_root,
        timestamp: 0,
        bits: 0x1d00ffff,
        nonce: 0,
        height: 1,
    }
}

#[test]
fn btc_merkle_proof_reconstructs_both_child_positions() {
    new_test_ext().execute_with(|| {
        let leaves: Vec<H256> = (0..4u8).map(|i| H256::repeat_byte(0x10 + i)).collect();
        let n01 = dsha(leaves[0].as_bytes(), leaves[1].as_bytes());
        let n23 = dsha(leaves[2].as_bytes(), leaves[3].as_bytes());
        let root = dsha(n01.as_bytes(), n23.as_bytes());
        let header = btc_header_with_root(root);

        // Index 2 is an even (left) child at level 0 and an odd (right) child at
        // level 1, so one proof exercises both concatenation orders.
        assert!(
            Pallet::<Test>::verify_btc_merkle_proof(&leaves[2], 2, &[leaves[3], n01], &header)
                .unwrap(),
            "a left-then-right path must reconstruct the root"
        );
        // Index 1 is odd at level 0 and even at level 1: the other two orders.
        assert!(
            Pallet::<Test>::verify_btc_merkle_proof(&leaves[1], 1, &[leaves[0], n23], &header)
                .unwrap(),
            "a right-then-left path must reconstruct the root"
        );
        // A single-leaf block: the txid is the root and an empty proof says so.
        let single = btc_header_with_root(leaves[0]);
        assert!(Pallet::<Test>::verify_btc_merkle_proof(&leaves[0], 0, &[], &single).unwrap());
        // A wrong sibling must reconstruct a different root, not any root.
        assert!(
            !Pallet::<Test>::verify_btc_merkle_proof(&leaves[2], 2, &[leaves[0], n01], &header)
                .unwrap(),
            "a wrong sibling must not verify"
        );
    });
}

#[test]
fn btc_target_decoding_matches_bitcoins_compact_format() {
    // `size` books and a 24-bit mantissa: size 3 shifts by zero.
    assert_eq!(
        Pallet::<Test>::btc_target_le(0x0312_3456).unwrap(),
        [
            0x56, 0x34, 0x12, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0
        ]
    );
    // Size 4 shifts the mantissa one byte up.
    assert_eq!(
        Pallet::<Test>::btc_target_le(0x0412_3456).unwrap(),
        [
            0x00, 0x56, 0x34, 0x12, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0
        ]
    );
    // Bitcoin mainnet's nBits: mantissa 0x00ffff at byte offset 26.
    assert_eq!(
        Pallet::<Test>::btc_target_le(0x1d00_ffff).unwrap(),
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff,
            0xff, 0, 0, 0, 0
        ]
    );
    // Size 34 with mantissa <= 0xff is the largest representable target...
    assert_eq!(
        Pallet::<Test>::btc_target_le(0x2200_00ff).unwrap()[31],
        0xff
    );
    // ...size 34 with a larger mantissa overflows 256 bits...
    assert_eq!(Pallet::<Test>::btc_target_le(0x2200_0100), None);
    // ...and size 35 always does.
    assert_eq!(Pallet::<Test>::btc_target_le(0x2300_ffff), None);
    // Size 33 with a big mantissa is still representable (the `== 34` check is
    // not `!= 34`).
    assert!(Pallet::<Test>::btc_target_le(0x210f_ffff).is_some());
    // A zero mantissa is not a target at all...
    assert_eq!(Pallet::<Test>::btc_target_le(0x1d00_0000), None);
    // ...and neither is a negative one (the 0x0080_0000 sign bit).
    assert_eq!(Pallet::<Test>::btc_target_le(0x1d80_0000), None);
    // The sign bit is refused *because it is set*, not because the mantissa
    // happened to be zero: with both conditions true only the `||` refuses.
    assert_eq!(Pallet::<Test>::btc_target_le(0x1d80_0001), None);
}

#[test]
fn should_wait_is_strict_at_every_urgency_boundary() {
    let eth = FinalityOracle::default_config(ExternalChainId::Ethereum);
    assert_eq!(eth.confirmations_required, 12);

    assert!(FinalityOracle::should_wait(&eth, 0, Urgency::Immediate));
    assert!(!FinalityOracle::should_wait(&eth, 1, Urgency::Immediate));

    assert!(FinalityOracle::should_wait(&eth, 11, Urgency::Normal));
    assert!(!FinalityOracle::should_wait(&eth, 12, Urgency::Normal));

    assert!(FinalityOracle::should_wait(&eth, 23, Urgency::Conservative));
    assert!(!FinalityOracle::should_wait(
        &eth,
        24,
        Urgency::Conservative
    ));
}

#[test]
fn reentrancy_detection_only_fires_for_nested_same_vm_entries() {
    let enforcer = InvariantEnforcer::new(InvariantConfig::default());

    // Two entries into the same VM while only one frame is live are NOT the
    // cross-VM pattern this check targets: the stack-length guard (`> 1`) is
    // load-bearing, so repetition alone must stay Pass.
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Evm),
        ]),
        InvariantCheckResult::Pass
    );
    // A single entry is not re-entrancy even though the VM is on the stack.
    assert_eq!(
        enforcer.check_reentrancy(&[VmExecutionEvent::Enter(VmType::Evm)]),
        InvariantCheckResult::Pass
    );
    // Entering the same VM while another VM's frame is live is re-entrancy.
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
            VmExecutionEvent::Enter(VmType::Evm),
        ]),
        InvariantCheckResult::Fail(InvariantViolationType::CrossVmReentrancy)
    );
    // A matched exit pops the frame, so the next entry is a fresh call.
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
            VmExecutionEvent::Exit(VmType::Svm),
            VmExecutionEvent::Enter(VmType::Svm),
        ]),
        InvariantCheckResult::Pass
    );
    // An exit naming a different VM must not pop the live frame: the later
    // re-entry still sees the original frame underneath.
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
            VmExecutionEvent::Exit(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
        ]),
        InvariantCheckResult::Fail(InvariantViolationType::CrossVmReentrancy)
    );
}

#[test]
fn check_all_returns_one_result_per_invariant() {
    let enforcer = InvariantEnforcer::new(InvariantConfig::default());

    let clean = enforcer.check_all(1, 1, IntentState::Finalized, false, false, 0, 0, 100, 50);
    assert_eq!(clean.len(), 3, "every invariant must contribute a result");
    assert!(clean
        .iter()
        .all(|result| *result == InvariantCheckResult::Pass));

    let broken = enforcer.check_all(2, 1, IntentState::Finalized, false, false, 0, 0, 100, 50);
    assert!(
        broken.contains(&InvariantCheckResult::Fail(
            InvariantViolationType::PartialExecution
        )),
        "a half-claimed finalization must be reported"
    );
}

#[test]
fn escrow_addresses_are_chain_specific_and_secret_bound() {
    new_test_ext().execute_with(|| {
        let secret = H256::repeat_byte(0x42);

        // Bitcoin: P2SH prefix plus the first 20 bytes of the secret hash.
        let btc =
            CrossVmEscrow::generate_escrow_address(&ExternalChainId::Bitcoin, &secret, &[], &[], 0);
        assert_eq!(btc.len(), 21);
        assert_eq!(btc[0], 0x05);
        assert_eq!(&btc[1..], &secret.as_bytes()[..20]);

        // Solana: sha256(b"escrow" ++ secret hash).
        let svm =
            CrossVmEscrow::generate_escrow_address(&ExternalChainId::Solana, &secret, &[], &[], 0);
        let mut seeds = Vec::new();
        seeds.extend_from_slice(b"escrow");
        seeds.extend_from_slice(secret.as_bytes());
        assert_eq!(svm, sp_io::hashing::sha2_256(&seeds).to_vec());

        // X3 native: blake2_256(b"x3escrow" ++ secret hash).
        let x3 = CrossVmEscrow::generate_escrow_address(
            &ExternalChainId::X3Native,
            &secret,
            &[],
            &[],
            0,
        );
        let mut preimage = Vec::new();
        preimage.extend_from_slice(b"x3escrow");
        preimage.extend_from_slice(secret.as_bytes());
        assert_eq!(x3, sp_io::hashing::blake2_256(&preimage).to_vec());

        // EVM: a 20-byte CREATE2-style address that binds the secret.
        let evm = CrossVmEscrow::generate_escrow_address(
            &ExternalChainId::Ethereum,
            &secret,
            &[],
            &[],
            0,
        );
        assert_eq!(evm.len(), 20);
        let other = CrossVmEscrow::generate_escrow_address(
            &ExternalChainId::Ethereum,
            &H256::repeat_byte(0x43),
            &[],
            &[],
            0,
        );
        assert_ne!(evm, other, "two secrets must not share an escrow address");
    });
}

#[test]
fn escrow_account_derivation_is_deterministic_in_its_inputs() {
    // The derivation is the first 32 bytes of `intent_id || b"escrow" || nonce`,
    // so for a 32-byte intent it is the intent itself — deterministic and never
    // the zero or one constant a body-erasure would return.
    let intent = H256::repeat_byte(0xab);
    let derived = EscrowAccount::derive_bytes(&intent.0, 0);
    assert_eq!(derived, intent.0);
    assert_eq!(EscrowAccount::derive_bytes(&intent.0, 7), intent.0);
    assert_ne!(derived, [0u8; 32]);
    assert_ne!(derived, [1u8; 32]);
}

#[test]
fn adaptor_recovery_id_mapping_is_exact_for_both_parities() {
    let mut msg = [0u8; 32];
    {
        use rand::RngCore;
        rand::rngs::OsRng.fill_bytes(&mut msg);
    }
    let (sig, _maker_pubkey, final_sig, _t) = crate::tests::real_adaptor_signature(msg);
    let parity = final_sig.0[64];
    assert!(parity < 2, "secp256k1 recovery ids are 0 or 1");

    // The compact form recovers the adapted key...
    assert!(sig.verify_with_recovery_id(&msg, &sig.adapted_pubkey, parity));
    // ...the +27-mapped form (2/3) is equivalent...
    assert!(sig.verify_with_recovery_id(&msg, &sig.adapted_pubkey, parity + 2));
    // ...the other parity must not recover the adapted key...
    assert!(!sig.verify_with_recovery_id(&msg, &sig.adapted_pubkey, parity ^ 1));
    // ...and mapping 3 through `* 27` instead of `+ 27` lands on the wrong parity,
    // which this case catches whichever parity the signature happens to have.
    assert!(!sig.verify_with_recovery_id(&msg, &sig.adapted_pubkey, 2 + (parity ^ 1)));
    // Only 0..=3 are valid recovery ids.
    assert!(!sig.verify_with_recovery_id(&msg, &sig.adapted_pubkey, 4));
    // The caller's key claim is honoured, not just its length: the same
    // pre-signature must not verify against a different 33-byte key. (Before
    // the campaign this argument was ignored and this returned true.)
    assert!(!sig.verify_with_recovery_id(&msg, &[0xffu8; 33], parity));
}

#[test]
fn btc_redeem_script_pushes_only_the_significant_timeout_bytes() {
    let params = BtcHtlcParams {
        secret_hash: H256::repeat_byte(0xab),
        recipient_pkh: [0x11; 20],
        refund_pkh: [0x22; 20],
        // 0x0100 little-endian is [00, 01, 00, 00, 00, 00, 00, 00]: two
        // significant bytes, not eight.
        timeout_height: 0x0100,
    };
    let script = params.to_redeem_script();
    assert!(
        script
            .windows(4)
            .any(|window| window == &[0x02u8, 0x00, 0x01, 0xb1][..]),
        "the timeout must be pushed as its 2 significant LE bytes then OP_CLTV: {script:02x?}"
    );
}

#[test]
fn cross_vm_partial_state_is_reported_in_both_shapes() {
    let enforcer = InvariantEnforcer::new(InvariantConfig::default());
    let partial = || InvariantCheckResult::Fail(InvariantViolationType::PartialExecution);

    // Any leg executed without all of them executed is partial unless the
    // executed part was reverted.
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, false, false, false, false),
        partial()
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(false, true, false, false, false),
        partial()
    );
    // One leg left to run while the others "completed" is the same partial
    // shape: any=true, all=false, nothing reverted.
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(false, false, true, true, false),
        partial()
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, false, false, false, true),
        InvariantCheckResult::Pass
    );
    // All three executed, one failed, nothing reverted: partial.
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, true, true, false, false),
        partial()
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, true, true, true, false),
        InvariantCheckResult::Pass
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(false, false, false, false, false),
        InvariantCheckResult::Pass
    );
    // Two of three executed with the executed part "successful" but nothing
    // reverted is still partial: the third leg never ran, so the only clean
    // shape is a revert. (`all_executed = e && s && x` mutated to
    // `e || (s && x)` answers Pass here.)
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(false, true, true, true, false),
        partial()
    );
}

#[test]
fn reorg_detector_supersedes_a_height_and_evicts_the_oldest() {
    let mut detector = ReorgDetector::new(ExternalChainId::Ethereum, 2);
    let (h5a, h5b, h6, h7) = (
        H256::repeat_byte(0x51),
        H256::repeat_byte(0x52),
        H256::repeat_byte(0x60),
        H256::repeat_byte(0x70),
    );

    detector.record_block(5, h5a);
    detector.record_block(6, h6);
    assert_eq!(detector.recent_hashes, vec![(5, h5a), (6, h6)]);

    detector.record_block(5, h5b);
    assert!(
        detector.recent_hashes.contains(&(5, h5b)),
        "a new hash at a known height supersedes the old one"
    );
    assert!(!detector.recent_hashes.contains(&(5, h5a)));

    detector.record_block(7, h7);
    assert_eq!(
        detector.recent_hashes.len(),
        2,
        "max_depth trims the buffer"
    );
    assert!(
        !detector
            .recent_hashes
            .iter()
            .any(|(height, _)| *height == 5),
        "the oldest height is the one evicted"
    );
}

#[test]
fn atomic_lock_commit_phase_computes_its_finalize_deadline() {
    let mut lock = AtomicLock::<u128, u64>::new_prepare([1u8; 32], 7, 100, 8, [9u8; 32], 10, 50);
    assert_eq!(
        lock.phase,
        LockPhase::LockedForCommit {
            locked_at_block: 10,
            commit_deadline: 60
        }
    );
    assert_ok!(lock.lock_for_commit(10, 50));
    assert_eq!(
        lock.phase,
        LockPhase::CommitInProgress {
            locked_at_block: 10,
            finalize_deadline: 60
        },
        "the finalize deadline is current_block + blocks, not a product"
    );
}

#[test]
fn btc_spv_proof_verifies_only_a_real_path() {
    let tx_bytes = vec![0x01u8, 0x02, 0x03];
    let txid = dsha(&tx_bytes, &[]);
    let sibling = H256::repeat_byte(0x11);
    let root = dsha(txid.as_bytes(), sibling.as_bytes());
    let proof = BtcSpvProof {
        tx_bytes: tx_bytes.clone(),
        block_header: btc_header_with_root(root),
        merkle_path: vec![sibling],
        tx_index: 0,
    };
    assert!(proof.verify(), "a correct path must reconstruct the root");

    let mut tampered = proof.clone();
    tampered.tx_bytes.push(0xff);
    assert!(!tampered.verify(), "changed bytes must change the txid");

    let mut wrong_side = proof.clone();
    wrong_side.tx_index = 1;
    assert!(
        !wrong_side.verify(),
        "the index decides concatenation order; the wrong parity must not verify"
    );
}

#[test]
fn btc_p2sh_address_is_hash160_of_the_redeem_script() {
    use ripemd::{Digest, Ripemd160};

    let params = BtcHtlcParams {
        secret_hash: H256::repeat_byte(0xab),
        recipient_pkh: [0x11; 20],
        refund_pkh: [0x22; 20],
        timeout_height: 0x0100,
    };
    let script = params.to_redeem_script();
    let sha = sp_io::hashing::sha2_256(&script);
    let mut hasher = Ripemd160::new();
    hasher.update(sha);
    let hash160: [u8; 20] = hasher.finalize().into();

    let address = params.to_p2sh_address(false);
    assert_eq!(address.len(), 25);
    assert_eq!(address[0], 0x05, "mainnet P2SH version byte");
    assert_eq!(
        &address[1..21],
        &hash160,
        "payload is RIPEMD160(SHA256(script))"
    );
    let checksum = dsha(&address[..21], &[]);
    assert_eq!(&address[21..25], &checksum.as_bytes()[..4]);
    assert_eq!(
        params.to_p2sh_address(true)[0],
        0xC4,
        "testnet version byte"
    );
}

#[test]
fn btc_median_time_past_stops_at_a_height_zero_anchor() {
    new_test_ext().execute_with(|| {
        // A parent chain above the anchor, long enough for a median if the walk
        // were allowed to leave the anchor's height-0 boundary behind.
        let mut prev = H256::zero();
        let mut top_of_chain = H256::zero();
        for height in 1..=10u64 {
            let hash = H256::repeat_byte(0x40 + height as u8);
            let header = BtcBlockHeader {
                version: 1,
                prev_block_hash: prev,
                merkle_root: H256::zero(),
                timestamp: 1_700_000_000 + height as u32,
                bits: 0x1d00ffff,
                nonce: 0,
                height,
            };
            BtcHeaders::<Test>::insert(hash, header);
            BtcHeaderMetaStore::<Test>::insert(
                hash,
                BtcHeaderMeta {
                    height,
                    anchored: true,
                },
            );
            prev = hash;
            top_of_chain = hash;
        }

        // The height-0 anchor links down into that chain.
        let anchor = H256::repeat_byte(0x30);
        BtcHeaders::<Test>::insert(
            anchor,
            BtcBlockHeader {
                version: 1,
                prev_block_hash: top_of_chain,
                merkle_root: H256::zero(),
                timestamp: 1_800_000_000,
                bits: 0x1d00ffff,
                nonce: 0,
                height: 0,
            },
        );
        BtcHeaderMetaStore::<Test>::insert(
            anchor,
            BtcHeaderMeta {
                height: 0,
                anchored: true,
            },
        );

        let target = H256::repeat_byte(0x20);
        BtcHeaders::<Test>::insert(
            target,
            BtcBlockHeader {
                version: 1,
                prev_block_hash: anchor,
                merkle_root: H256::zero(),
                timestamp: 1_900_000_000,
                bits: 0x1d00ffff,
                nonce: 0,
                height: 11,
            },
        );
        BtcHeaderMetaStore::<Test>::insert(
            target,
            BtcHeaderMeta {
                height: 11,
                anchored: false,
            },
        );

        // The walk stops at the anchor, so only two timestamps are visible —
        // fewer than Bitcoin's eleven, which is `None`, never an invented median.
        assert_eq!(Pallet::<Test>::btc_median_time_past(&target), None);
    });
}

#[test]
fn on_finalize_slashes_expired_nonzero_locks_only() {
    new_test_ext().execute_with(|| {
        let block = frame_system::Pallet::<Test>::block_number();
        let real_id = H256::repeat_byte(0xa1);
        let zero_id = H256::repeat_byte(0xa2);
        // Committed at block 0, so the deadline (block 0) is strictly before the
        // current block: `slash_on_timeout` only fires past the deadline.
        let real = AtomicLock::<u128, u64>::new_prepare([1u8; 32], 7, 100, 8, [9u8; 32], 0, 0);
        let zero = AtomicLock::<u128, u64>::new_prepare([2u8; 32], 7, 0, 8, [9u8; 32], 0, 0);
        AtomicLocks::<Test>::insert(real_id, real);
        AtomicLocks::<Test>::insert(zero_id, zero);
        AtomicLockExpiryIndex::<Test>::insert(
            block as u32,
            BoundedVec::try_from(vec![real_id, zero_id]).expect("two ids fit"),
        );

        Pallet::<Test>::on_finalize(block);

        let real_phase = AtomicLocks::<Test>::get(real_id).expect("real lock").phase;
        assert!(
            matches!(real_phase, LockPhase::Slashed { at_block, .. } if at_block == block as u32),
            "an expired funded lock must be slashed"
        );
        let zero_phase = AtomicLocks::<Test>::get(zero_id).expect("zero lock").phase;
        assert!(
            matches!(zero_phase, LockPhase::LockedForCommit { .. }),
            "zero-amount locks are skipped, never slashed"
        );
        let slashed_events = frame_system::Pallet::<Test>::events()
            .iter()
            .filter(|record| {
                matches!(
                    &record.event,
                    crate::mock::RuntimeEvent::X3SettlementEngine(
                        crate::Event::AtomicLockTimeoutSlashed { .. }
                    )
                )
            })
            .count();
        assert_eq!(slashed_events, 1, "exactly the funded lock is reported");
    });
}

#[test]
fn on_idle_refunds_only_once_a_settlement_is_past_the_block_timeout() {
    new_test_ext().execute_with(|| {
        let secret_hash = H256::repeat_byte(0x78);
        assert_ok!(Pallet::<Test>::create_intent(
            RuntimeOrigin::signed(ALICE),
            BOB,
            AssetSpec {
                chain: ExternalChainId::Ethereum,
                token: TokenId::Native,
                amount: 1_000,
            },
            AssetSpec {
                chain: ExternalChainId::Solana,
                token: TokenId::Native,
                amount: 500,
            },
            secret_hash,
            None,
        ));
        let intent_id = SettlementIntents::<Test>::iter()
            .find(|(_, intent)| intent.secret_hash == secret_hash)
            .map(|(id, _)| id)
            .expect("intent exists");

        let timeout_blocks = 28_800u64; // mock `SettlementTimeoutBlocks`
                                        // Age == the limit is not yet "past" it.
        frame_system::Pallet::<Test>::set_block_number(1 + timeout_blocks);
        let idle_weight = Pallet::<Test>::on_idle(1 + timeout_blocks, Weight::zero());
        assert_eq!(IntentStates::<Test>::get(intent_id), IntentState::Created);
        assert_eq!(
            idle_weight.ref_time(),
            0,
            "no timeout found, no weight charged"
        );

        // One block past it, the checker refunds and charges for the work.
        frame_system::Pallet::<Test>::set_block_number(2 + timeout_blocks);
        let idle_weight = Pallet::<Test>::on_idle(2 + timeout_blocks, Weight::zero());
        assert_eq!(IntentStates::<Test>::get(intent_id), IntentState::Refunded);
        assert!(idle_weight.ref_time() > 0, "refund work must be charged");
    });
}

#[test]
fn on_initialize_refunds_at_the_deadline_only_with_its_refund_proofs() {
    new_test_ext().execute_with(|| {
        let armed_secret = H256::repeat_byte(0x81);
        let unarmed_secret = H256::repeat_byte(0x82);
        for secret in [armed_secret, unarmed_secret] {
            assert_ok!(Pallet::<Test>::create_intent(
                RuntimeOrigin::signed(ALICE),
                BOB,
                AssetSpec {
                    chain: ExternalChainId::Ethereum,
                    token: TokenId::Native,
                    amount: 1_000,
                },
                AssetSpec {
                    chain: ExternalChainId::Solana,
                    token: TokenId::Native,
                    amount: 500,
                },
                H256::from(sp_io::hashing::sha2_256(secret.as_bytes())),
                Some(30),
            ));
            // Escrow both legs: the canonical proof check is keyed on the
            // recorded escrow legs, so an intent without them can never be
            // "fully proven" and would never auto-refund.
            let id = SettlementIntents::<Test>::iter()
                .find(|(_, intent)| {
                    intent.secret_hash == H256::from(sp_io::hashing::sha2_256(secret.as_bytes()))
                })
                .map(|(id, _)| id)
                .expect("intent exists");
            assert_ok!(Pallet::<Test>::lock_escrow(
                RuntimeOrigin::signed(BOB),
                id,
                0,
                ExternalChainId::Ethereum,
                1_000,
                vec![],
            ));
            assert_ok!(Pallet::<Test>::lock_escrow(
                RuntimeOrigin::signed(ALICE),
                id,
                1,
                ExternalChainId::Solana,
                500,
                vec![],
            ));
        }
        let find = |secret: H256| {
            let hash = H256::from(sp_io::hashing::sha2_256(secret.as_bytes()));
            SettlementIntents::<Test>::iter()
                .find(|(_, intent)| intent.secret_hash == hash)
                .map(|(id, _)| id)
                .expect("intent exists")
        };
        let armed = find(armed_secret);
        let unarmed = find(unarmed_secret);

        // 30 s / 6 s blocks + 1 padding block, one block after genesis.
        let due = IntentDeadlineIndex::<Test>::get(7u64);
        assert!(
            due.contains(&armed) && due.contains(&unarmed),
            "both intents must be due at block 7: {due:?}"
        );

        // Arm only `armed` with canonical Refund proofs for both legs. The mock
        // allows unattested sets, so the compact keys are recorded directly.
        let runtime_armed = armed.to_fixed_bytes();
        let set = CrossDomainProofSet {
            intent_id: 1,
            runtime_intent_id: runtime_armed,
            intent_hash: [0x11u8; 32],
            bundles: vec![
                crate::tests::fabricated_bundle(
                    runtime_armed,
                    "ethereum-mainnet",
                    ProofVmType::Evm,
                    CrossDomainOperation::Refund,
                    "0xrefund-evm".into(),
                ),
                crate::tests::fabricated_bundle(
                    runtime_armed,
                    "solana-mainnet",
                    ProofVmType::Svm,
                    CrossDomainOperation::Refund,
                    "0xrefund-svm".into(),
                ),
            ],
        };
        assert_ok!(Pallet::<Test>::submit_cross_domain_proof_set(
            RuntimeOrigin::signed(ALICE),
            armed,
            set
        ));

        // Exactly at the Unix timeout the automatic refund fires; a second
        // intent without its Refund proofs must be rescheduled instead.
        let timeout = SettlementIntents::<Test>::get(armed)
            .expect("intent")
            .timeout;
        pallet_timestamp::Pallet::<Test>::set_timestamp(timeout * 1_000);
        frame_system::Pallet::<Test>::set_block_number(7);
        let weight = Pallet::<Test>::on_initialize(7);

        assert_eq!(IntentStates::<Test>::get(armed), IntentState::Refunded);
        assert_eq!(
            IntentStates::<Test>::get(unarmed),
            IntentState::FullyFunded,
            "without Refund proofs the intent must be rescheduled, not refunded"
        );
        assert!(
            IntentDeadlineIndex::<Test>::get(8u64).contains(&unarmed),
            "the reschedule must land one block ahead"
        );
        assert!(weight.ref_time() > 0, "the hook must charge for its work");
    });
}

// ────────────────────────────────────────────────────────────────────────────
// Batch 3 (2026-10-03): the remaining survivor clusters — RLP/receipt and
// compact-u32 decoding, SVM and EVM proof boundaries, the BTC time/target
// arithmetic, the hook deadline indexes, the intent planner, the escrow
// encoders, and the invariant-resolution deadlines.
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn intent_resolution_fires_only_strictly_past_each_deadline() {
    let enforcer = InvariantEnforcer::new(InvariantConfig {
        max_settlement_time: 1_000,
        ..InvariantConfig::default()
    });
    let late = InvariantCheckResult::Fail(InvariantViolationType::TimeoutBypass);

    // Unix timeout: strictly greater fires, equal does not.
    assert_eq!(
        enforcer.check_intent_resolution(IntentState::Claiming, 0, 50, 51),
        late
    );
    assert_eq!(
        enforcer.check_intent_resolution(IntentState::Claiming, 0, 50, 50),
        InvariantCheckResult::Pass
    );
    // A resolved intent is never "late".
    assert_eq!(
        enforcer.check_intent_resolution(IntentState::Finalized, 0, 50, 99),
        InvariantCheckResult::Pass
    );
    assert_eq!(
        enforcer.check_intent_resolution(IntentState::Refunded, 0, 0, u64::MAX),
        InvariantCheckResult::Pass
    );
    // Max-settlement-time backstop: strictly greater fires, equal does not.
    assert_eq!(
        enforcer.check_intent_resolution(IntentState::Claiming, 100, u64::MAX, 1_100),
        InvariantCheckResult::Pass
    );
    assert_eq!(
        enforcer.check_intent_resolution(IntentState::Claiming, 100, u64::MAX, 1_101),
        late
    );
    // Before either deadline nothing fires.
    assert_eq!(
        enforcer.check_intent_resolution(IntentState::Claiming, 100, u64::MAX, 100),
        InvariantCheckResult::Pass
    );
}

#[test]
fn reorg_detector_classifies_conflicts_duplicates_and_trimming() {
    let mut detector = ReorgDetector::new(ExternalChainId::Ethereum, 2);

    assert!(!detector.record_block(100, H256::repeat_byte(0x01)));
    // A different height with a different hash is not a reorg.
    assert!(!detector.record_block(101, H256::repeat_byte(0x02)));
    assert_eq!(detector.tracked_len(), 2, "both heights stay tracked");
    // The same height with the same hash is a duplicate, not a reorg.
    assert!(!detector.record_block(101, H256::repeat_byte(0x02)));
    assert_eq!(detector.tracked_len(), 2);
    // The same height with a new hash is a reorg and replaces the entry.
    assert!(detector.record_block(101, H256::repeat_byte(0x03)));
    assert_eq!(detector.tracked_len(), 2, "the old hash is evicted");
    // A third height trims the oldest entry, keeping the tracker bounded.
    assert!(!detector.record_block(102, H256::repeat_byte(0x04)));
    assert_eq!(detector.tracked_len(), 2, "the tracker stays bounded");

    // Stability needs max_depth confirmations, not max_depth - 1.
    assert!(!detector.is_stable(102, 103));
    assert!(detector.is_stable(102, 104));
    assert!(detector.is_stable(100, 200));
    assert!(
        !detector.is_stable(200, 100),
        "a future height is not stable"
    );
}

#[test]
fn btc_reorg_risk_matches_its_documented_curve() {
    assert_eq!(BtcReorgRisk::estimate(0), 10_000);
    assert_eq!(BtcReorgRisk::estimate(1), 2_500);
    assert_eq!(BtcReorgRisk::estimate(2), 500);
    assert_eq!(BtcReorgRisk::estimate(3), 100);
    assert_eq!(BtcReorgRisk::estimate(4), 50);
    assert_eq!(BtcReorgRisk::estimate(5), 10);
    assert_eq!(BtcReorgRisk::estimate(6), 1);
    assert_eq!(BtcReorgRisk::estimate(7), 0, "beyond the curve is final");
}

#[test]
fn bitcoin_hashes_match_known_vectors() {
    // RIPEMD160 of the empty byte string (the raw hash, not hash160).
    assert_eq!(
        BtcHtlcParams::ripemd160(&[]),
        [
            0x9c, 0x11, 0x85, 0xa5, 0xc5, 0xe9, 0xfc, 0x54, 0x61, 0x28, 0x08, 0x97, 0x7e, 0xe8,
            0xf5, 0x48, 0xb2, 0x25, 0x8d, 0x31,
        ]
    );
    // HASH160 of the empty byte string: ripemd160(sha256([])).
    assert_eq!(
        BtcHtlcParams::ripemd160(&sp_io::hashing::sha2_256(&[])),
        [
            0xb4, 0x72, 0xa2, 0x66, 0xd0, 0xbd, 0x89, 0xc1, 0x37, 0x06, 0xa4, 0x13, 0x2c, 0xcf,
            0xb1, 0x6f, 0x7c, 0x3b, 0x9f, 0xcb,
        ]
    );
    // double-SHA256 of "abc".
    assert_eq!(
        BtcHtlcParams::double_sha256(b"abc"),
        [
            0x4f, 0x8b, 0x42, 0xc2, 0x2d, 0xd3, 0x72, 0x9b, 0x51, 0x9b, 0xa6, 0xf6, 0x8d, 0x2d,
            0xa7, 0xcc, 0x5b, 0x2d, 0x60, 0x6d, 0x05, 0xda, 0xed, 0x5a, 0xd5, 0x12, 0x8c, 0xc0,
            0x3e, 0x6c, 0x63, 0x58,
        ]
    );
}

#[test]
fn adaptor_secret_recovery_inverts_both_scalar_paths() {
    use sp_core::U256;

    fn signature_with_s(last_byte: u8) -> [u8; 64] {
        let mut sig = [0u8; 64];
        sig[63] = last_byte;
        sig
    }
    fn adaptor_with_s(last_byte: u8) -> BtcAdaptorSignature {
        BtcAdaptorSignature {
            pre_signature: signature_with_s(last_byte),
            adaptor_point: [0u8; 33],
            nonce: [0u8; 33],
            adapted_pubkey: [0u8; 33],
        }
    }

    // s_complete >= s_pre: secret = s_complete - s_pre.
    let recovered = adaptor_with_s(10)
        .extract_secret(&signature_with_s(25))
        .expect("the direct path recovers");
    let mut expected = [0u8; 32];
    expected[31] = 15;
    assert_eq!(recovered, expected);

    // Equal scalars recover zero, not n.
    let zero = adaptor_with_s(7)
        .extract_secret(&signature_with_s(7))
        .expect("equal scalars are well-formed");
    assert_eq!(zero, [0u8; 32]);

    // s_complete < s_pre: secret = n - (s_pre - s_complete).
    let n = U256::from_big_endian(&[
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFE, 0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B, 0xBF, 0xD2, 0x5E, 0x8C, 0xD0, 0x36,
        0x41, 0x41,
    ]);
    let wrapped = adaptor_with_s(25)
        .extract_secret(&signature_with_s(10))
        .expect("the wrapped path recovers");
    assert_eq!(wrapped, (n - U256::from(15u64)).to_big_endian());
}

#[test]
fn svm_escrow_instruction_encodings_are_exact() {
    let secret_hash = H256::repeat_byte(0x5a);
    let params = SvmEscrowParams {
        secret_hash,
        recipient: [0x11; 32],
        refund_authority: [0x22; 32],
        timeout_slot: 0x0102_0304_0506_0708,
        token_mint: [0x33; 32],
        amount: 0x0a0b_0c0d_0e0f_1011,
    };

    let mut initialize = vec![0u8];
    initialize.extend_from_slice(secret_hash.as_bytes());
    initialize.extend_from_slice(&params.timeout_slot.to_le_bytes());
    initialize.extend_from_slice(&params.amount.to_le_bytes());
    assert_eq!(params.encode_initialize_ix(), initialize);

    let mut claim = vec![1u8];
    claim.extend_from_slice(secret_hash.as_bytes());
    assert_eq!(SvmEscrowParams::encode_claim_ix(&secret_hash), claim);

    assert_eq!(SvmEscrowParams::encode_refund_ix(), vec![2u8]);

    let mut seeds = Vec::new();
    seeds.extend_from_slice(b"escrow");
    seeds.extend_from_slice(secret_hash.as_bytes());
    seeds.extend_from_slice(&params.recipient);
    assert_eq!(params.escrow_pda(), sp_io::hashing::sha2_256(&seeds));
}

#[test]
fn atomic_lock_deadlines_are_strict_and_phase_specific() {
    let mut lock =
        AtomicLock::new_prepare([0x11u8; 32], 1u64, 1_000u128, 2u64, [0x22u8; 32], 10, 20);
    assert_eq!(lock.deadline_block(), Some(30));
    assert!(
        !lock.is_expired(30),
        "the deadline block itself is not late"
    );
    assert!(lock.is_expired(31));
    assert_eq!(lock.slash_on_timeout(30), Err("Deadline not yet reached"));
    assert_eq!(lock.slash_on_timeout(31), Ok(()));
    assert!(!lock.is_expired(99), "a slashed lock has no deadline");

    // The commit phase uses its own finalize deadline.
    let mut lock =
        AtomicLock::new_prepare([0x11u8; 32], 1u64, 1_000u128, 2u64, [0x22u8; 32], 5, 20);
    lock.lock_for_commit(8, 600).expect("prepare -> commit");
    match &lock.phase {
        LockPhase::CommitInProgress {
            locked_at_block,
            finalize_deadline,
        } => {
            assert_eq!(*locked_at_block, 8);
            assert_eq!(*finalize_deadline, 608);
        }
        other => panic!("expected CommitInProgress, got {other:?}"),
    }
    assert_eq!(lock.deadline_block(), Some(608));
    assert!(!lock.is_expired(608));
    assert!(lock.is_expired(609));
    assert_eq!(lock.slash_on_timeout(608), Err("Deadline not yet reached"));
    assert_eq!(lock.slash_on_timeout(609), Ok(()));

    // Releasing on abort moves the lock out of both expiring phases.
    let mut lock =
        AtomicLock::new_prepare([0x11u8; 32], 1u64, 1_000u128, 2u64, [0x22u8; 32], 5, 20);
    lock.release_on_abort(6).expect("prepare -> released");
    assert!(!lock.is_expired(u32::MAX));
    assert_eq!(lock.deadline_block(), None);
}

#[test]
fn plan_settlement_orders_slow_first_and_halves_the_fast_timeout() {
    let asset = |chain| AssetSpec {
        chain,
        token: TokenId::Native,
        amount: 1,
    };

    let plan = IntentPlanner::plan_settlement(
        &asset(ExternalChainId::Bitcoin),
        &asset(ExternalChainId::X3Native),
        100,
    );
    assert_eq!(plan.legs.len(), 2);
    assert_eq!(plan.legs[0].chain, ExternalChainId::Bitcoin);
    assert_eq!(plan.legs[0].timeout, 100);
    assert_eq!(plan.legs[0].confirmations_required, 6);
    assert_eq!(plan.legs[1].chain, ExternalChainId::X3Native);
    assert_eq!(plan.legs[1].timeout, 50);

    // Equal chain speeds keep the sell side as the slow leg.
    let tie = IntentPlanner::plan_settlement(
        &asset(ExternalChainId::Bnb),
        &asset(ExternalChainId::Avalanche),
        100,
    );
    assert_eq!(tie.legs[0].chain, ExternalChainId::Bnb);
    assert_eq!(tie.legs[1].chain, ExternalChainId::Avalanche);
}

#[test]
fn the_runtime_bridge_placeholder_stays_fail_closed() {
    use crate::bridge_integration::{
        CrossChainValidatorBridge, CrossChainValidatorProvider, NoOpCrossChainValidator,
    };

    // The runtime binds its own provider; the placeholder in this crate must
    // refuse everything and claim no headers.
    assert!(!CrossChainValidatorBridge::verify_evm_proof(
        1,
        H256::zero(),
        H256::zero(),
        H256::zero()
    ));
    assert!(!CrossChainValidatorBridge::verify_svm_proof(
        1,
        H256::zero(),
        H256::zero(),
        H256::zero()
    ));
    assert_eq!(
        CrossChainValidatorBridge::get_latest_evm_header_hash(),
        None
    );
    assert_eq!(
        CrossChainValidatorBridge::get_latest_svm_header_hash(),
        None
    );

    // The no-op validator accepts proofs but still reports no canonical header.
    assert!(NoOpCrossChainValidator::verify_evm_proof(
        1,
        H256::zero(),
        H256::zero(),
        H256::zero()
    ));
    assert!(NoOpCrossChainValidator::verify_svm_proof(
        1,
        H256::zero(),
        H256::zero(),
        H256::zero()
    ));
    assert_eq!(NoOpCrossChainValidator::get_latest_evm_header_hash(), None);
    assert_eq!(NoOpCrossChainValidator::get_latest_svm_header_hash(), None);
}

#[test]
fn receipt_trie_fixture_matches_an_independent_encoder() {
    use crate::proof_fixtures::receipt_trie;

    // A literal re-encoding of RLP integer -> nibbles -> hex-prefix leaf,
    // written from the yellow paper rather than sharing the implementation.
    fn rlp_bytes(bytes: &[u8]) -> Vec<u8> {
        if bytes.len() == 1 && bytes[0] < 0x80 {
            return vec![bytes[0]];
        }
        let mut out = if bytes.len() <= 55 {
            vec![0x80 + bytes.len() as u8]
        } else {
            let mut header = vec![0xb8, bytes.len() as u8];
            header.extend_from_slice(bytes);
            return header;
        };
        out.extend_from_slice(bytes);
        out
    }
    fn rlp_two(a: &[u8], b: &[u8]) -> Vec<u8> {
        let (ea, eb) = (rlp_bytes(a), rlp_bytes(b));
        let payload = ea.len() + eb.len();
        let mut out = if payload <= 55 {
            vec![0xc0 + payload as u8]
        } else {
            vec![0xf8, payload as u8]
        };
        out.extend_from_slice(&ea);
        out.extend_from_slice(&eb);
        out
    }
    fn expected(rlp: &[u8], index: u32) -> (H256, Vec<u8>) {
        let be = index.to_be_bytes();
        let start = be.iter().position(|b| *b != 0);
        let key: Vec<u8> = match start {
            None => vec![0x80],
            Some(0) if be[0] < 0x80 && be[0] != 0 => vec![be[0]],
            Some(start) => {
                let significant = &be[start..];
                if significant.len() == 1 && significant[0] < 0x80 {
                    vec![significant[0]]
                } else {
                    let mut out = vec![0x80 + significant.len() as u8];
                    out.extend_from_slice(significant);
                    out
                }
            }
        };
        let mut nibbles = Vec::new();
        for byte in &key {
            nibbles.push(byte >> 4);
            nibbles.push(byte & 0x0f);
        }
        let mut path = vec![0x20 + (nibbles.len() / 2) as u8];
        let mut i = 0;
        while i < nibbles.len() {
            path.push(nibbles[i] * 16 + nibbles[i + 1]);
            i += 2;
        }
        let leaf = rlp_two(&path, rlp);
        let root = H256::from(sp_io::hashing::keccak_256(&leaf));
        (root, rlp_bytes(&leaf))
    }

    for rlp in [&[0xc3u8, 0x01, 0x00, 0xc0][..], &[0x7fu8; 60][..]] {
        for index in [0u32, 1, 0x7f, 0x80, 0xff, 0x100, 0x01_0000, u32::MAX] {
            let (root, proof) = receipt_trie(rlp, index);
            let (want_root, want_leaf) = expected(rlp, index);
            assert_eq!(root, want_root, "root for index {index}");
            // The proof is a one-item list holding the leaf's own RLP.
            let mut want_proof = if want_leaf.len() <= 55 {
                vec![0xc0 + want_leaf.len() as u8]
            } else {
                vec![0xf8, want_leaf.len() as u8]
            };
            want_proof.extend_from_slice(&want_leaf);
            assert_eq!(proof, want_proof, "proof for index {index}");
        }
    }
}

#[test]
fn btc_target_le_pins_the_sub_word_shifts_and_the_word_bytes() {
    // size 1: word 0x010000 shifts right 16 -> target[0] = 1.
    let size1 = Pallet::<Test>::btc_target_le(0x0101_0000).expect("size 1");
    assert_eq!(size1[0], 1);
    assert!(size1[1..].iter().all(|b| *b == 0));
    // word 0x000100 is below one shifted unit at exponent 1 and truncates to zero.
    let truncated = Pallet::<Test>::btc_target_le(0x0100_0100).expect("size 1");
    assert!(truncated.iter().all(|b| *b == 0));

    // size 2: word 0x000101 shifts right 8 -> target = 0x000001 LE.
    let size2 = Pallet::<Test>::btc_target_le(0x0200_0101).expect("size 2");
    assert_eq!(&size2[..4], &[0x01, 0x00, 0x00, 0x00]);
    // size 2 must shift by exactly one byte: the mantissa's second byte lands in target[1].
    let size2b = Pallet::<Test>::btc_target_le(0x0201_0000).expect("size 2");
    assert_eq!(&size2b[..4], &[0x00, 0x01, 0x00, 0x00]);
    assert!(size2[4..].iter().all(|b| *b == 0));

    // size 3: word 0x010203 is the target verbatim, little-endian.
    let size3 = Pallet::<Test>::btc_target_le(0x0301_0203).expect("size 3");
    assert_eq!(&size3[..4], &[0x03, 0x02, 0x01, 0x00]);
    assert!(size3[4..].iter().all(|b| *b == 0));

    // size 33: word 0x000101 lands at byte 30 (little-endian continuation).
    let size33 = Pallet::<Test>::btc_target_le(0x2100_0101).expect("size 33");
    assert!(size33[..30].iter().all(|b| *b == 0));
    assert_eq!(&size33[30..], &[0x01, 0x01]);

    // size 34 may carry only one word byte (its most significant).
    let size34 = Pallet::<Test>::btc_target_le(0x2200_00ff).expect("size 34");
    assert!(size34[..31].iter().all(|b| *b == 0));
    assert_eq!(size34[31], 0xff);

    // The size-34 word bound applies at 34 only: size 30 may carry two bytes.
    let size30 = Pallet::<Test>::btc_target_le(0x1e00_0100).expect("size 30");
    assert_eq!(&size30[27..31], &[0x00, 0x01, 0x00, 0x00]);

    // Out of range and degenerate words fail closed.
    assert_eq!(Pallet::<Test>::btc_target_le(0x2300_0001), None, "size 35");
    assert_eq!(
        Pallet::<Test>::btc_target_le(0x2200_0100),
        None,
        "size 34 with a word above 0xff"
    );
    assert_eq!(
        Pallet::<Test>::btc_target_le(0x1e00_0000),
        None,
        "zero word"
    );
    assert_eq!(
        Pallet::<Test>::btc_target_le(0x1e80_0001),
        None,
        "the sign bit must be clear"
    );
}

#[test]
fn btc_median_time_past_needs_eleven_samples_and_stops_at_an_anchor() {
    fn header_after(prev: H256, timestamp: u32) -> BtcBlockHeader {
        BtcBlockHeader {
            version: 1,
            prev_block_hash: prev,
            merkle_root: H256::zero(),
            timestamp,
            bits: 0x1d00ffff,
            nonce: 0,
            height: 1,
        }
    }

    new_test_ext().execute_with(|| {
        // An eleven-block window; the timestamps are 1..=11, so the median
        // (the sixth of eleven sorted values) is 6.
        let mut hashes = Vec::new();
        let mut prev = H256::zero();
        for i in 0..11u64 {
            let hash = H256::repeat_byte(0x40 + i as u8);
            BtcHeaders::<Test>::insert(hash, header_after(prev, 1 + i as u32));
            BtcHeaderMetaStore::<Test>::insert(
                hash,
                BtcHeaderMeta {
                    height: i,
                    anchored: true,
                },
            );
            hashes.push(hash);
            prev = hash;
        }
        assert_eq!(
            Pallet::<Test>::btc_median_time_past(&hashes[10]),
            Some(6),
            "eleven samples give Bitcoin's median"
        );
        // Ten samples are not enough: the answer stays None rather than
        // guessing from a truncated window.
        assert_eq!(Pallet::<Test>::btc_median_time_past(&hashes[9]), None);

        // A height-zero anchor ends the walk even when its parent is stored:
        // the missing predecessors cannot be recovered, so the answer is None.
        let mut prev = H256::repeat_byte(0x70);
        let mut stored = Vec::new();
        for i in 1..=11u64 {
            let hash = H256::repeat_byte(0x70 + i as u8);
            BtcHeaders::<Test>::insert(hash, header_after(prev, 100 + i as u32));
            BtcHeaderMetaStore::<Test>::insert(
                hash,
                BtcHeaderMeta {
                    height: i,
                    anchored: true,
                },
            );
            stored.push(hash);
            prev = hash;
        }
        let anchor = H256::repeat_byte(0x7f);
        BtcHeaders::<Test>::insert(anchor, header_after(stored[10], 200));
        BtcHeaderMetaStore::<Test>::insert(
            anchor,
            BtcHeaderMeta {
                height: 0,
                anchored: true,
            },
        );
        assert_eq!(
            Pallet::<Test>::btc_median_time_past(&anchor),
            None,
            "an anchor with fewer than eleven ancestors is not a window"
        );
    });
}

// ────────────────────────────────────────────────────────────────────────────
// Batch 3 (2026-10-03), part 2: finality scheduling, escrow op validation,
// the intent planner's time/risk math, adaptor recovery against real
// secp256k1 signatures, the SPV merkle walk, and the invariant checkers.
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn finality_urgency_boundaries_and_the_bitcoin_risk_curve() {
    use crate::types::{FinalityConfig, ProofType};

    let config = |chain, confirmations_required| FinalityConfig {
        chain,
        confirmations_required,
        block_time_ms: 600_000,
        proof_type: ProofType::BitcoinSpv,
        challenge_period_seconds: 0,
        max_reorg_depth: 6,
    };

    let btc = config(ExternalChainId::Bitcoin, 6);
    // Immediate: one confirmation is enough, zero is not.
    assert!(FinalityOracle::should_wait(&btc, 0, Urgency::Immediate));
    assert!(!FinalityOracle::should_wait(&btc, 1, Urgency::Immediate));
    // Normal: the configured count is the line.
    assert!(FinalityOracle::should_wait(&btc, 5, Urgency::Normal));
    assert!(!FinalityOracle::should_wait(&btc, 6, Urgency::Normal));
    // Conservative: twice the configured count.
    assert!(FinalityOracle::should_wait(&btc, 11, Urgency::Conservative));
    assert!(!FinalityOracle::should_wait(
        &btc,
        12,
        Urgency::Conservative
    ));

    // Bitcoin's decay curve, one assertion per arm.
    for (confirmations, expected) in [
        (0u32, 10_000u32),
        (1, 2_500),
        (2, 500),
        (3, 100),
        (4, 50),
        (5, 10),
    ] {
        assert_eq!(
            FinalityOracle::reorg_probability(&btc, confirmations),
            expected,
            "bitcoin at {confirmations} confirmations"
        );
    }
    // At the requirement, and past it, the risk is exactly zero.
    assert_eq!(FinalityOracle::reorg_probability(&btc, 6), 0);
    assert_eq!(FinalityOracle::reorg_probability(&btc, 7), 0);

    // Instant-finality chains never report a reorg risk.
    let avalanche = config(ExternalChainId::Avalanche, 1);
    assert_eq!(FinalityOracle::reorg_probability(&avalanche, 0), 0);
    let x3 = config(ExternalChainId::X3Native, 1);
    assert_eq!(FinalityOracle::reorg_probability(&x3, 0), 0);

    // Generic EVM: 50% at zero confirmations, halving each confirmation after.
    let evm = config(ExternalChainId::Polygon, 128);
    assert_eq!(FinalityOracle::reorg_probability(&evm, 0), 5_000);
    assert_eq!(FinalityOracle::reorg_probability(&evm, 1), 2_500);
    assert_eq!(FinalityOracle::reorg_probability(&evm, 2), 1_250);
    assert_eq!(FinalityOracle::reorg_probability(&evm, 3), 625);
}

#[test]
fn escrow_operation_validation_and_address_derivation() {
    use crate::escrow::EscrowOp;

    let ethereum = ExternalChainId::Ethereum;
    let lock_on_ethereum = EscrowOp::Lock {
        depositor: 1u64,
        amount: 5u128,
        chain: ethereum,
        escrow_data: vec![],
    };
    let lock_on_bitcoin = EscrowOp::Lock {
        depositor: 1u64,
        amount: 5u128,
        chain: ExternalChainId::Bitcoin,
        escrow_data: vec![],
    };
    assert!(CrossVmEscrow::validate_escrow_op(
        &lock_on_ethereum,
        &ethereum
    ));
    assert!(!CrossVmEscrow::validate_escrow_op(
        &lock_on_bitcoin,
        &ethereum
    ));
    // Release and refund carry no chain to validate; they are always well-formed.
    assert!(CrossVmEscrow::validate_escrow_op(
        &EscrowOp::Release {
            recipient: 2u64,
            amount: 5u128
        },
        &ethereum
    ));
    assert!(CrossVmEscrow::validate_escrow_op(
        &EscrowOp::Refund {
            depositor: 1u64,
            amount: 5u128
        },
        &ethereum
    ));

    // Address derivation is deterministic, non-empty, and input-sensitive.
    let one = H256::repeat_byte(0x01);
    let two = H256::repeat_byte(0x02);
    let btc_one =
        CrossVmEscrow::generate_escrow_address(&ExternalChainId::Bitcoin, &one, &[], &[], 1);
    let btc_one_again =
        CrossVmEscrow::generate_escrow_address(&ExternalChainId::Bitcoin, &one, &[], &[], 1);
    let btc_two =
        CrossVmEscrow::generate_escrow_address(&ExternalChainId::Bitcoin, &two, &[], &[], 1);
    assert!(!btc_one.is_empty());
    assert_eq!(btc_one, btc_one_again, "derivation is deterministic");
    assert_ne!(btc_one, btc_two, "the secret hash is part of the address");

    let x3_one =
        CrossVmEscrow::generate_escrow_address(&ExternalChainId::X3Native, &one, &[], &[], 1);
    let x3_two =
        CrossVmEscrow::generate_escrow_address(&ExternalChainId::X3Native, &two, &[], &[], 1);
    assert_eq!(x3_one.len(), 32);
    assert_ne!(x3_one, x3_two);
}

#[test]
fn intent_planner_risk_time_and_confirmations() {
    use crate::intent::{IntentStateMachine, RiskLevel};

    let asset = |chain| AssetSpec {
        chain,
        token: TokenId::Native,
        amount: 1,
    };

    let btc_to_x3 = IntentPlanner::plan_settlement(
        &asset(ExternalChainId::Bitcoin),
        &asset(ExternalChainId::X3Native),
        600,
    );
    assert_eq!(
        btc_to_x3.estimated_time, 3_606,
        "BTC 3600 s + X3 6 s, summed not multiplied"
    );
    assert_eq!(btc_to_x3.risk_level, RiskLevel::High);
    assert_eq!(btc_to_x3.legs[0].confirmations_required, 6);
    assert_eq!(btc_to_x3.legs[1].confirmations_required, 1);

    // Buying into BTC is just as risky as selling out of it.
    let eth_to_btc = IntentPlanner::plan_settlement(
        &asset(ExternalChainId::Ethereum),
        &asset(ExternalChainId::Bitcoin),
        600,
    );
    assert_eq!(eth_to_btc.risk_level, RiskLevel::High);

    // An X3-internal swap is low risk; one native leg among external ones is
    // medium, not low.
    let x3_to_x3 = IntentPlanner::plan_settlement(
        &asset(ExternalChainId::X3Native),
        &asset(ExternalChainId::X3Native),
        600,
    );
    assert_eq!(x3_to_x3.risk_level, RiskLevel::Low);
    let eth_to_x3 = IntentPlanner::plan_settlement(
        &asset(ExternalChainId::Ethereum),
        &asset(ExternalChainId::X3Native),
        600,
    );
    assert_eq!(eth_to_x3.risk_level, RiskLevel::Medium);
    let eth_to_polygon = IntentPlanner::plan_settlement(
        &asset(ExternalChainId::Ethereum),
        &asset(ExternalChainId::Polygon),
        600,
    );
    // Ethereum is slower, so it funds first; the confirmation counts are the
    // per-chain requirements, not a zero/one default.
    assert_eq!(eth_to_polygon.legs[0].chain, ExternalChainId::Ethereum);
    assert_eq!(eth_to_polygon.legs[0].confirmations_required, 12);
    assert_eq!(eth_to_polygon.legs[1].chain, ExternalChainId::Polygon);
    assert_eq!(eth_to_polygon.legs[1].confirmations_required, 128);

    // Terminal states are exactly the two resolved ones.
    assert!(IntentStateMachine::is_terminal(IntentState::Finalized));
    assert!(IntentStateMachine::is_terminal(IntentState::Refunded));
    assert!(!IntentStateMachine::is_terminal(IntentState::Created));
    assert!(!IntentStateMachine::is_terminal(IntentState::FullyFunded));
    assert!(!IntentStateMachine::is_terminal(IntentState::Claiming));
    assert!(!IntentStateMachine::is_terminal(IntentState::Halted));
}

#[test]
fn adaptor_signatures_recover_the_key_the_caller_names() {
    use secp256k1::{Message, Secp256k1, SecretKey};

    let secp = Secp256k1::new();
    let secret = SecretKey::from_slice(&[0x11u8; 32]).expect("valid scalar");
    let pubkey = secret.public_key(&secp).serialize();

    // Pick a message whose recoverable signature uses recovery id 1, so the
    // 27/28 mapping is exercised with the parity the signature actually has.
    let (message_bytes, recovery_id, compact) = (0..16u8)
        .find_map(|i| {
            let digest = [0x40 + i; 32];
            let message = Message::from_digest(digest);
            let signature = secp.sign_ecdsa_recoverable(&message, &secret);
            let (recovery_id, compact) = signature.serialize_compact();
            (recovery_id.to_i32() == 1).then_some((digest, 1u8, compact))
        })
        .expect("roughly half of the signatures recover with id 1");

    let adaptor = BtcAdaptorSignature {
        pre_signature: compact,
        adaptor_point: pubkey,
        nonce: pubkey,
        adapted_pubkey: pubkey,
    };

    // The two-attempt loop recovers the adapted key.
    assert!(adaptor.verify(&message_bytes, &pubkey));
    // A different message recovers a different point.
    let mut other_message = message_bytes;
    other_message[0] ^= 0x01;
    assert!(!adaptor.verify(&other_message, &pubkey));
    // The all-zero message is refused outright.
    assert!(!adaptor.verify(&[0u8; 32], &pubkey));

    // The explicit-recovery-id form checks against the pubkey the caller
    // names, not against the struct's own field.
    assert!(adaptor.verify_with_recovery_id(&message_bytes, &pubkey, recovery_id));
    let other_key = SecretKey::from_slice(&[0x33u8; 32])
        .expect("valid scalar")
        .public_key(&secp)
        .serialize();
    assert!(!adaptor.verify_with_recovery_id(&message_bytes, &other_key, recovery_id));
    // Recovery id 3 maps to v=28, which is this signature's parity.
    assert!(adaptor.verify_with_recovery_id(&message_bytes, &pubkey, 3));
    // Out-of-range recovery ids are refused rather than pushed to sp_io.
    assert!(!adaptor.verify_with_recovery_id(&message_bytes, &pubkey, 4));
}

#[test]
fn spv_proof_walks_the_merkle_index_it_states() {
    use crate::btc_gateway::BtcSpvProof;

    fn dsha(left: &[u8], right: &[u8]) -> H256 {
        let mut buf = Vec::with_capacity(left.len() + right.len());
        buf.extend_from_slice(left);
        buf.extend_from_slice(right);
        let first = sp_io::hashing::sha2_256(&buf);
        H256::from(sp_io::hashing::sha2_256(&first))
    }

    let tx_bytes = vec![0x01u8, 0x02, 0x03, 0x04];
    let txid = H256::from(sp_io::hashing::sha2_256(&sp_io::hashing::sha2_256(
        &tx_bytes,
    )));
    // Binary tree of four leaves; index 2 is the left child of the right pair.
    let sibling0 = H256::repeat_byte(0x51);
    let n1 = dsha(txid.as_bytes(), sibling0.as_bytes());
    let sibling1 = H256::repeat_byte(0x52);
    let root = dsha(sibling1.as_bytes(), n1.as_bytes());

    let mut proof = BtcSpvProof {
        tx_bytes: tx_bytes.clone(),
        block_header: btc_header_with_root(root),
        merkle_path: vec![sibling0, sibling1],
        tx_index: 2,
    };
    assert!(
        proof.verify(),
        "the stated index selects the left/right sides"
    );
    // Index 3 combines in the other order: a different root.
    proof.tx_index = 3;
    assert!(!proof.verify());
    proof.tx_index = 2;
    // A different header root does not match.
    proof.block_header = btc_header_with_root(H256::repeat_byte(0x60));
    assert!(!proof.verify());
    // A tampered transaction changes the leaf.
    proof.block_header = btc_header_with_root(root);
    proof.tx_bytes.push(0xff);
    assert!(!proof.verify());
}

#[test]
fn invariant_partial_execution_timeout_and_reentrancy_are_pinned() {
    use crate::invariants::VmType;

    let enforcer = InvariantEnforcer::new(InvariantConfig::default());
    let pass = InvariantCheckResult::Pass;
    let partial = InvariantCheckResult::Fail(InvariantViolationType::PartialExecution);
    let timeout = InvariantCheckResult::Fail(InvariantViolationType::TimeoutBypass);

    // Partial execution is a violation unless every executed leg reverted.
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, false, false, false, false),
        partial
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, false, false, false, true),
        pass
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(false, true, false, false, false),
        partial
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(false, false, false, false, false),
        pass
    );
    // Fully executed: success is fine, a mixed outcome is not.
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, true, true, true, false),
        pass
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, true, true, false, false),
        partial
    );
    assert_eq!(
        enforcer.check_no_cross_vm_partial_state(true, true, true, false, true),
        pass
    );

    // Timeout: both sides refundable is fine; either stuck side is a violation.
    assert_eq!(
        enforcer.check_timeout_favors_users(false, false, false),
        pass
    );
    assert_eq!(
        enforcer.check_timeout_favors_users(false, true, false),
        pass
    );
    assert_eq!(enforcer.check_timeout_favors_users(true, true, true), pass);
    assert_eq!(
        enforcer.check_timeout_favors_users(true, false, true),
        timeout
    );
    assert_eq!(
        enforcer.check_timeout_favors_users(true, true, false),
        timeout
    );
    assert_eq!(
        enforcer.check_timeout_favors_users(true, false, false),
        timeout
    );

    // Reentrancy: entering a VM already on the stack, through another VM.
    let reentrancy = InvariantCheckResult::Fail(InvariantViolationType::CrossVmReentrancy);
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
            VmExecutionEvent::Enter(VmType::Evm),
        ]),
        reentrancy
    );
    // Re-entering the same VM at depth one is not cross-VM reentrancy...
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Evm),
        ]),
        pass
    );
    // ...and an exit for a different VM does not pop the stack.
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Exit(VmType::Svm),
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Evm),
        ]),
        reentrancy
    );
    // A balanced enter/exit cycle is clean.
    assert_eq!(
        enforcer.check_reentrancy(&[
            VmExecutionEvent::Enter(VmType::Evm),
            VmExecutionEvent::Enter(VmType::Svm),
            VmExecutionEvent::Exit(VmType::Svm),
            VmExecutionEvent::Exit(VmType::Evm),
        ]),
        pass
    );

    // The facade returns one result per focused check, not an empty list.
    let results = enforcer.check_all(2, 0, IntentState::FullyFunded, false, false, 0, 0, 100, 50);
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|result| *result == pass));
}

#[test]
fn intent_error_display_names_each_failure() {
    use crate::intent::FromIntentError;

    // Every adapter refusal carries a message; a `fmt` body replaced by
    // `Ok(Default::default())` writes nothing and would render as "".
    assert_eq!(
        format!(
            "{}",
            FromIntentError::UnsupportedChain { chain: "x3".into() }
        ),
        "settlement: unsupported chain 'x3'"
    );
    assert_eq!(
        format!("{}", FromIntentError::EmptyIntent),
        "settlement: empty intent"
    );
    assert_eq!(
        format!("{}", FromIntentError::ZeroTimeout),
        "settlement: zero timeout"
    );
    let mismatch = format!(
        "{}",
        FromIntentError::HashMismatch {
            stored: [1u8; 32],
            recomputed: [2u8; 32]
        }
    );
    assert!(
        mismatch.starts_with("settlement: hash mismatch (stored=[1"),
        "{mismatch}"
    );
    assert!(mismatch.contains("recomputed=[2"), "{mismatch}");
}

#[test]
fn proof_domain_key_binds_chain_vm_and_operation() {
    // A body replaced by `Default::default()` (the zero hash) would make every
    // domain collide, so a proof stored under one would authorize the others.
    let key = Pallet::<Test>::proof_domain_key(
        "ethereum-mainnet",
        ProofVmType::Evm,
        CrossDomainOperation::Claim,
    );
    assert_ne!(
        key,
        H256::zero(),
        "the domain key must not be the zero hash"
    );
    assert_ne!(
        key,
        Pallet::<Test>::proof_domain_key(
            "solana-mainnet",
            ProofVmType::Evm,
            CrossDomainOperation::Claim,
        ),
        "the chain is part of the domain"
    );
    assert_ne!(
        key,
        Pallet::<Test>::proof_domain_key(
            "ethereum-mainnet",
            ProofVmType::Svm,
            CrossDomainOperation::Claim,
        ),
        "the VM is part of the domain"
    );
    assert_ne!(
        key,
        Pallet::<Test>::proof_domain_key(
            "ethereum-mainnet",
            ProofVmType::Evm,
            CrossDomainOperation::Refund,
        ),
        "the operation is part of the domain"
    );
}
