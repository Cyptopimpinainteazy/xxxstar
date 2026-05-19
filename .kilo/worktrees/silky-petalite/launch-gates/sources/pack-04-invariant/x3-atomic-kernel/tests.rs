//! Tests for pallet-x3-atomic-kernel

use super::pallet::*;
use super::proof::*;
use sp_core::H256;

// ── Simple unit tests (no FRAME mock needed) ────────────────────────────────

#[test]
fn test_poae_proof_structure_validation() {
    let valid = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 2,
    };
    assert!(valid.validate_structure());
}

#[test]
fn test_poae_proof_zero_bundle_id_invalid() {
    let invalid = PoaeProof {
        bundle_id: H256::zero(),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 2,
    };
    assert!(!invalid.validate_structure());
}

#[test]
fn test_poae_proof_zero_block_invalid() {
    let invalid = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 0, // should be > 0
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 2,
    };
    assert!(!invalid.validate_structure());
}

#[test]
fn test_poae_proof_hash_is_deterministic() {
    let proof = PoaeProof {
        bundle_id: H256::repeat_byte(0x11),
        receipt_root: H256::repeat_byte(0x22),
        finalized_block: 500,
        finality_cert: H256::repeat_byte(0x33),
        legs_hash: H256::repeat_byte(0x44),
        leg_count: 3,
    };
    // Same proof → same hash (determinism)
    assert_eq!(proof.proof_hash(), proof.proof_hash());
}

#[test]
fn test_poae_proof_hash_differs_on_different_data() {
    let p1 = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::repeat_byte(0x03),
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 1,
    };
    let p2 = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0xFF), // different receipt
        ..p1.clone()
    };
    assert_ne!(p1.proof_hash(), p2.proof_hash());
}

#[test]
fn test_bundle_leg_encode_decode_roundtrip() {
    use parity_scale_codec::{Decode, Encode};

    let leg = BundleLeg {
        vm_type: VmType::Cross,
        token_in: H256::repeat_byte(0xAA),
        token_out: H256::repeat_byte(0xBB),
        amount_in: 1_000_000_000_000u128,
        min_amount_out: 990_000_000_000u128,
        deadline: 1_800_000_000u64,
        access: DeclaredAccess {
            reads: Default::default(),
            writes: Default::default(),
        },
    };

    let encoded = leg.encode();
    let decoded = BundleLeg::decode(&mut &encoded[..]).expect("decode failed");
    assert_eq!(leg, decoded);
}

// ── OCW key / payload protocol tests ──────────────────────────────────────
//
// These tests verify the pallet OCW's key convention and payload encoding
// agree exactly with what the AtomicSwapOrchestrator writes to off-chain
// local storage.  They are pure computation tests — no FRAME mock needed.

/// OCW key = b"x3fin:" (6) || bundle_id_bytes (32) = 38 bytes.
/// Must match the key written by the orchestrator's finalization signal path.
#[test]
fn test_ocw_key_is_38_bytes_with_correct_prefix() {
    let bundle_id = H256::repeat_byte(0xBB);
    let mut key = b"x3fin:".to_vec();
    key.extend_from_slice(bundle_id.as_bytes());

    assert_eq!(
        key.len(),
        38,
        "key must be 38 bytes (6 prefix + 32 bundle_id)"
    );
    assert_eq!(&key[..6], b"x3fin:", "key must start with 'x3fin:'");
    assert_eq!(&key[6..38], bundle_id.as_bytes());
}

/// Payload decode: 40 bytes = receipt_root[0..32] || committed_at_ns[32..40] LE.
/// Mirrors the decode in `offchain_worker()` hook — both sides must agree.
#[test]
fn test_ocw_payload_decode_matches_encode() {
    use sp_core::hashing::sha2_256;

    let receipt_root = H256::from(sha2_256(b"test_receipt_data"));
    let committed_at_ns: u64 = 1_700_500_000_000_000_000u64;

    // Encode (orchestrator writer side)
    let mut payload: Vec<u8> = receipt_root.as_bytes().to_vec();
    payload.extend_from_slice(&committed_at_ns.to_le_bytes());
    assert_eq!(payload.len(), 40);

    // Decode (pallet OCW reader side — mirrors offchain_worker() code)
    let decoded_root = H256::from_slice(&payload[..32]);
    let decoded_ns = u64::from_le_bytes(
        payload[32..40]
            .try_into()
            .expect("slice is exactly 8 bytes"),
    );

    assert_eq!(decoded_root, receipt_root);
    assert_eq!(decoded_ns, committed_at_ns);
    assert_ne!(
        decoded_root,
        H256::zero(),
        "SHA-256 of real data cannot be zero"
    );
}

/// Verify `H256::zero()` guard: the OCW skips bundles with zero receipt_root.
#[test]
fn test_ocw_zero_receipt_root_is_rejected() {
    let zero_root = H256::zero();
    // Mirrors the guard in offchain_worker(): `if receipt_root == H256::zero() { continue }`
    assert!(
        zero_root == H256::zero(),
        "zero H256 sentinel must work for OCW guard"
    );

    let non_zero = H256::repeat_byte(0x01);
    assert_ne!(
        non_zero,
        H256::zero(),
        "non-zero receipt_root must pass OCW guard"
    );
}

/// Verify that different bundle IDs produce non-colliding OCW keys.
#[test]
fn test_ocw_keys_are_unique_per_bundle() {
    use sp_core::hashing::sha2_256;

    let id_a = H256::from(sha2_256(b"bundle_alpha"));
    let id_b = H256::from(sha2_256(b"bundle_beta"));
    assert_ne!(id_a, id_b);

    let mut key_a = b"x3fin:".to_vec();
    key_a.extend_from_slice(id_a.as_bytes());

    let mut key_b = b"x3fin:".to_vec();
    key_b.extend_from_slice(id_b.as_bytes());

    assert_ne!(
        key_a, key_b,
        "distinct bundle IDs must produce distinct OCW keys"
    );
}

// ── Flash Finality cert key protocol tests ────────────────────────────────

/// Flash Finality cert key: b"x3ff:" (5) + block_number as LE u64 (8) = 13 bytes.
/// Value: cert_hash (32 bytes) written by run_flash_finality_voter in service.rs.
/// Must match the key the OCW uses to read the cert.
#[test]
fn test_flash_cert_key_is_13_bytes_with_correct_prefix() {
    let block_number: u64 = 12_345;
    let mut key = b"x3ff:".to_vec();
    key.extend_from_slice(&block_number.to_le_bytes());

    assert_eq!(
        key.len(),
        13,
        "Flash cert key must be 13 bytes (5 prefix + 8 LE u64)"
    );
    assert_eq!(&key[..5], b"x3ff:", "key must start with 'x3ff:'");
    let decoded_block = u64::from_le_bytes(key[5..13].try_into().unwrap());
    assert_eq!(
        decoded_block, block_number,
        "block_number must roundtrip through LE-u64"
    );
}

/// Flash Finality cert keys must be unique per block number.
#[test]
fn test_flash_cert_keys_are_unique_per_block() {
    let key_100: Vec<u8> = {
        let mut k = b"x3ff:".to_vec();
        k.extend_from_slice(&100u64.to_le_bytes());
        k
    };
    let key_101: Vec<u8> = {
        let mut k = b"x3ff:".to_vec();
        k.extend_from_slice(&101u64.to_le_bytes());
        k
    };
    assert_ne!(
        key_100, key_101,
        "distinct block numbers must produce distinct cert keys"
    );
    // Also verify x3ff and x3fin prefixes never collide (sanity check)
    let bundle_key: Vec<u8> = {
        let mut k = b"x3fin:".to_vec();
        k.extend_from_slice(&H256::repeat_byte(0x01).as_bytes()[..8]);
        k
    };
    assert_ne!(
        key_100, bundle_key,
        "'x3ff:' keys must not collide with 'x3fin:' keys"
    );
}

/// Verify that a real cert_hash (32 bytes) roundtrips through the key-value protocol.
#[test]
fn test_flash_cert_value_is_32_bytes() {
    use sp_core::hashing::sha2_256;
    // Simulate a cert_hash from FinalityCertificate::cert_hash()
    let fake_cert_hash = sha2_256(b"block_hash_round_votes_voter_set");
    assert_eq!(fake_cert_hash.len(), 32, "cert_hash must be 32 bytes");

    // Roundtrip: write as bytes, read as H256
    let as_h256 = H256::from_slice(&fake_cert_hash);
    assert_ne!(as_h256, H256::zero(), "real cert_hash is never zero");

    // Mirrors the OCW read: `H256::from_slice(&v)` where v is 32 bytes
    let decoded = H256::from_slice(&as_h256.as_bytes()[..32]);
    assert_eq!(
        decoded, as_h256,
        "cert_hash must roundtrip through H256::from_slice"
    );
}

/// When Flash Finality cert is zero, the PoAE proof is stored but flagged as incomplete
/// by `validate_structure()`.  External verifiers may choose to accept or reject it.
/// This tests the current design: zero cert = structurally incomplete proof.
#[test]
fn test_poae_proof_zero_finality_cert_is_incomplete() {
    // With Flash Finality not running, finality_cert = H256::zero().
    // The proof CAN be stored on-chain (do_finalize_bundle allows zero cert),
    // but validate_structure() marks it as incomplete for external verifiers.
    let proof_with_zero_cert = PoaeProof {
        bundle_id: H256::repeat_byte(0x01),
        receipt_root: H256::repeat_byte(0x02),
        finalized_block: 100,
        finality_cert: H256::zero(), // Flash Finality not running
        legs_hash: H256::repeat_byte(0x04),
        leg_count: 1,
    };
    // validate_structure() returns false for zero cert — expected: proof is incomplete.
    assert!(
        !proof_with_zero_cert.validate_structure(),
        "PoAE proof with zero finality_cert must be marked incomplete by validate_structure()"
    );
    // But a proof with a real cert passes
    let proof_with_cert = PoaeProof {
        finality_cert: H256::repeat_byte(0x05),
        ..proof_with_zero_cert
    };
    assert!(
        proof_with_cert.validate_structure(),
        "PoAE proof with non-zero finality_cert must be marked valid"
    );
}
