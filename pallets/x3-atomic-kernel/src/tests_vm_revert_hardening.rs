// SPDX-License-Identifier: Apache-2.0
//
// tests_vm_revert_hardening.rs — pins for the mutation gate on vm_revert.rs.
//
// The first mutation campaign on pallet-x3-atomic-kernel left 84 survivors in vm_revert.rs:
// every bound check in the diff decoders (`offset + N > len` and friends), every storage-key
// body, the `+=` log counters and the reverts' effect on storage. The decoders were rewritten
// to a checked `DiffReader` (which also removed two real panic paths: a truncated X3VM value
// and a truncated overlay key both indexed out of bounds), and these tests pin what remains:
// truncation at every prefix, exact key derivation, exact storage writes, and the counters the
// reverters log.

use super::*;

/// Reverter writes go through `sp_io::storage`, which panics without an Externalities
/// overlay; wrap storage-touching test bodies like vm_revert's own test module does.
fn run<F: FnOnce()>(f: F) {
    let mut ext = sp_io::TestExternalities::new_empty();
    ext.execute_with(f);
}

/// Reject every shortened prefix of `full` (except the empty diff, the canonical
/// no-op) and accept every prefix from `ok_from` upward, without panicking.
fn assert_prefix_boundary<F>(mut decode: F, full: &StateDiff, ok_from: usize)
where
    F: FnMut(&StateDiff) -> Result<usize, RevertError>,
{
    let bytes = full.as_bytes();
    for len in 0..bytes.len() {
        let prefix = StateDiff::from(bytes[..len].to_vec());
        if len == 0 {
            assert!(
                decode(&prefix).is_ok(),
                "the empty diff is the canonical no-op"
            );
        } else if len < ok_from {
            assert!(
                decode(&prefix).is_err(),
                "prefix of {len} bytes must be rejected"
            );
        } else {
            assert!(
                decode(&prefix).is_ok(),
                "prefix of {len} bytes must be accepted"
            );
        }
    }
    assert!(decode(full).is_ok(), "the full diff must decode");
}

fn make_evm_change(
    key: u8,
    old: &[u8],
    new: &[u8],
    contract: Option<[u8; 20]>,
) -> EvmStorageChange {
    EvmStorageChange {
        key: [key; 32],
        old_value: old.to_vec(),
        new_value: new.to_vec(),
        contract,
    }
}

// ── DiffReader ─────────────────────────────────────────────────────────────

#[test]
fn diff_reader_reads_exactly_and_fails_past_the_end() {
    let bytes = [0xAAu8, 0xBB, 0xCC, 0xDD];
    let mut r = DiffReader::new(&bytes);

    assert_eq!(r.take(0).unwrap(), b"");
    assert_eq!(r.take(2).unwrap(), &[0xAA, 0xBB]);
    assert_eq!(r.remaining(), 2);
    assert_eq!(r.u8().unwrap(), 0xCC);
    assert_eq!(r.remaining(), 1);
    assert_eq!(r.take(1).unwrap(), &[0xDD]);
    assert_eq!(r.remaining(), 0);
    assert!(r.take(1).is_err(), "reading past the end must fail");

    let word = [1u8, 2, 3, 4];
    let mut r = DiffReader::new(&word);
    assert_eq!(r.u32().unwrap(), 0x0403_0201);

    let mut buf = [0u8; 32];
    buf[31] = 0x5A;
    let mut r = DiffReader::new(&buf);
    let mut expected = [0u8; 32];
    expected[31] = 0x5A;
    assert_eq!(r.array32().unwrap(), expected);
    assert_eq!(r.remaining(), 0);
}

// ── StateDiff byte API ─────────────────────────────────────────────────────

#[test]
fn state_diff_byte_api_is_exact() {
    let empty = StateDiff::from(Vec::new());
    assert!(empty.is_empty());
    assert_eq!(empty.as_bytes(), b"");

    let bytes = vec![9u8, 8, 7, 6, 5];
    let diff = StateDiff::from(bytes.clone());
    assert!(!diff.is_empty());
    assert_eq!(diff.as_bytes(), bytes.as_slice());
    assert_eq!(
        StateDiff::from_vec_lossy(bytes.clone()).as_bytes(),
        bytes.as_slice()
    );
}

// ── EVM decode ─────────────────────────────────────────────────────────────

#[test]
fn evm_decode_rejects_truncated_prefixes_and_pins_the_contract_boundary() {
    let changes = vec![make_evm_change(
        0x11,
        &[0xAA; 5],
        &[0xBB; 3],
        Some([0xCC; 20]),
    )];
    let full = encode_evm_state_diff(&changes, Some([0xCC; 20]));
    // header + key + old_len + old + new_len + new — the entry without its contract.
    let without_contract = 4 + 32 + 4 + 5 + 4 + 3;
    assert_prefix_boundary(
        |d| decode_evm_state_diff(d).map(|v| v.len()),
        &full,
        without_contract,
    );

    // Prefixes in the contract window decode with `contract: None` and ignore the
    // trailing bytes; only a full 20 bytes is a contract address.
    let bytes = full.as_bytes();
    for len in without_contract..bytes.len() - 20 {
        let prefix = StateDiff::from(bytes[..len].to_vec());
        let decoded = decode_evm_state_diff(&prefix).unwrap();
        assert_eq!(
            decoded[0].contract, None,
            "len {len}: no full 20 bytes remain"
        );
    }
}

#[test]
fn evm_decode_contract_presence_boundary_is_twenty_bytes() {
    let mut bytes = 1u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&[0x11u8; 32]);
    bytes.extend_from_slice(&0u32.to_le_bytes()); // old_value_len
    bytes.extend_from_slice(&0u32.to_le_bytes()); // new_value_len

    let mut nineteen = bytes.clone();
    nineteen.extend_from_slice(&[0xCC; 19]);
    assert_eq!(
        decode_evm_state_diff(&StateDiff::from(nineteen)).unwrap()[0].contract,
        None
    );

    let mut twenty = bytes.clone();
    twenty.extend_from_slice(&[0xCC; 20]);
    assert_eq!(
        decode_evm_state_diff(&StateDiff::from(twenty)).unwrap()[0].contract,
        Some([0xCC; 20])
    );

    let mut twenty_one = bytes.clone();
    twenty_one.extend_from_slice(&[0xCC; 21]);
    let decoded = decode_evm_state_diff(&StateDiff::from(twenty_one)).unwrap();
    assert_eq!(decoded[0].contract, Some([0xCC; 20]));
}

#[test]
fn evm_decode_accepts_a_zero_entry_header_and_rejects_hostile_counts() {
    let only_header = StateDiff::from(0u32.to_le_bytes().to_vec());
    assert_eq!(decode_evm_state_diff(&only_header).unwrap(), Vec::new());

    let mut hostile = u32::MAX.to_le_bytes().to_vec();
    hostile.extend_from_slice(&[0u8; 8]);
    assert!(decode_evm_state_diff(&StateDiff::from(hostile)).is_err());
}

#[test]
fn evm_decode_rejects_a_truncated_value() {
    let mut bytes = 1u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&[0x11u8; 32]);
    bytes.extend_from_slice(&5u32.to_le_bytes());
    bytes.extend_from_slice(&[0x01, 0x02]); // claims 5 bytes, provides 2
    assert!(decode_evm_state_diff(&StateDiff::from(bytes)).is_err());
}

// ── SVM decode ─────────────────────────────────────────────────────────────

#[test]
fn svm_decode_rejects_truncated_prefixes() {
    let changes = vec![SvmStorageChange {
        account: [0x21; 32],
        key: b"balance".to_vec(),
        old_value: vec![0xDE, 0xAD, 0xBE],
    }];
    let full = encode_svm_state_diff(&changes);
    assert_prefix_boundary(
        |d| decode_svm_state_diff(d).map(|v| v.len()),
        &full,
        full.as_bytes().len(),
    );
}

// ── X3VM decode ────────────────────────────────────────────────────────────

#[test]
fn x3vm_decode_rejects_truncated_prefixes_without_panicking() {
    let changes = vec![X3VmStorageChange {
        key: [0x31; 32],
        old_value: Some([0xAA; 32]),
    }];
    let full = encode_x3vm_state_diff(&changes);
    assert_prefix_boundary(
        |d| decode_x3vm_state_diff(d).map(|v| v.len()),
        &full,
        full.as_bytes().len(),
    );
}

#[test]
fn x3vm_decode_keeps_short_values_zero_padded_and_rejects_oversized_ones() {
    let mut bytes = 1u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&[0x77; 32]);
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE]);
    let decoded = decode_x3vm_state_diff(&StateDiff::from(bytes)).unwrap();
    let mut expected = [0u8; 32];
    expected[..3].copy_from_slice(&[0xDE, 0xAD, 0xBE]);
    assert_eq!(
        decoded,
        vec![X3VmStorageChange {
            key: [0x77; 32],
            old_value: Some(expected),
        }]
    );

    let mut oversized = 1u32.to_le_bytes().to_vec();
    oversized.extend_from_slice(&[0x77; 32]);
    oversized.extend_from_slice(&33u32.to_le_bytes());
    oversized.extend_from_slice(&[0xAB; 33]);
    assert!(
        decode_x3vm_state_diff(&StateDiff::from(oversized)).is_err(),
        "a 33-byte X3VM value is malformed"
    );
}

// ── Overlay decode ─────────────────────────────────────────────────────────

#[test]
fn overlay_decode_rejects_truncated_prefixes() {
    let changes = vec![OverlayLegChange {
        domain: OverlayDomain::Evm,
        address: vec![0x42; 20],
        key: H256([0x51; 32]),
        old_value: Some(H256([0x52; 32])),
        new_value: None,
    }];
    let full = encode_overlay_state_diff(&changes);
    assert_prefix_boundary(
        |d| decode_overlay_state_diff(d).map(|v| v.len()),
        &full,
        full.as_bytes().len(),
    );
}

#[test]
fn overlay_decode_rejects_bad_magic_even_when_the_rest_parses() {
    let mut bytes = b"BADMAGIC".to_vec();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    assert!(decode_overlay_state_diff(&StateDiff::from(bytes)).is_err());
}

#[test]
fn overlay_encoding_pins_domain_bytes_and_round_trips_all_three() {
    for (domain, byte) in [
        (OverlayDomain::Evm, 0u8),
        (OverlayDomain::Svm, 1u8),
        (OverlayDomain::X3, 2u8),
    ] {
        let change = OverlayLegChange {
            domain,
            address: vec![0x07; 32],
            key: H256([0x08; 32]),
            old_value: Some(H256([0x09; 32])),
            new_value: Some(H256([0x0A; 32])),
        };
        let diff = encode_overlay_state_diff(core::slice::from_ref(&change));
        assert_eq!(diff.as_bytes()[..8], OVERLAY_DIFF_MAGIC);
        assert_eq!(diff.as_bytes()[12], byte, "domain byte for {domain:?}");
        assert_eq!(
            decode_overlay_state_diff(&diff).unwrap(),
            vec![change],
            "round trip for {domain:?}"
        );
    }
}

#[test]
fn overlay_decode_rejects_bad_domain_and_option_flags() {
    let mut bad_domain = OVERLAY_DIFF_MAGIC.to_vec();
    bad_domain.extend_from_slice(&1u32.to_le_bytes());
    bad_domain.push(3); // only 0/1/2 are domains
    assert!(decode_overlay_state_diff(&StateDiff::from(bad_domain)).is_err());

    let mut bad_flag = OVERLAY_DIFF_MAGIC.to_vec();
    bad_flag.extend_from_slice(&1u32.to_le_bytes());
    bad_flag.push(0); // domain Evm
    bad_flag.extend_from_slice(&0u32.to_le_bytes()); // empty address
    bad_flag.extend_from_slice(&[0x01; 32]); // key
    bad_flag.push(2); // invalid option flag
    assert!(decode_overlay_state_diff(&StateDiff::from(bad_flag)).is_err());

    let mut short_some = OVERLAY_DIFF_MAGIC.to_vec();
    short_some.extend_from_slice(&1u32.to_le_bytes());
    short_some.push(0);
    short_some.extend_from_slice(&0u32.to_le_bytes());
    short_some.extend_from_slice(&[0x01; 32]);
    short_some.push(1); // Some flag
    short_some.extend_from_slice(&[0xEE; 31]);
    assert!(decode_overlay_state_diff(&StateDiff::from(short_some)).is_err());
}

#[test]
fn decode_optional_h256_reads_none_some_and_rejects_bad_flags() {
    let mut r = DiffReader::new(&[0u8]);
    assert_eq!(decode_optional_h256(&mut r).unwrap(), None);

    let mut some_bytes = vec![1u8];
    some_bytes.extend_from_slice(&[0xCD; 32]);
    let mut r = DiffReader::new(&some_bytes);
    assert_eq!(
        decode_optional_h256(&mut r).unwrap(),
        Some(H256([0xCD; 32]))
    );

    let mut r = DiffReader::new(&[2u8]);
    assert!(decode_optional_h256(&mut r).is_err());

    let mut r = DiffReader::new(&[]);
    assert!(decode_optional_h256(&mut r).is_err());
}

#[test]
fn encode_optional_h256_writes_flag_and_value() {
    let mut out = Vec::new();
    encode_optional_h256(&mut out, None);
    assert_eq!(out, vec![0]);

    let mut out = Vec::new();
    encode_optional_h256(&mut out, Some(H256([0xCD; 32])));
    assert_eq!(out.len(), 33);
    assert_eq!(out[0], 1);
    assert_eq!(&out[1..], &[0xCD; 32]);
}

// ── Storage key derivation ─────────────────────────────────────────────────

#[test]
fn storage_key_matches_the_frame_hash_concat_layout() {
    use sp_io::hashing::{blake2_128, twox_128};

    let mut expected = Vec::new();
    expected.extend_from_slice(&twox_128(b"pallet_x"));
    expected.extend_from_slice(&twox_128(b"Item"));
    assert_eq!(storage_key(b"pallet_x", b"Item", None), expected);

    let k = [0x33u8; 7];
    let mut expected_map = expected.clone();
    expected_map.extend_from_slice(&blake2_128(&k));
    expected_map.extend_from_slice(&k);
    assert_eq!(storage_key(b"pallet_x", b"Item", Some(&k)), expected_map);
}

#[test]
fn evm_storage_keys_are_pinned() {
    use sp_io::hashing::{blake2_128, twox_128, twox_64};

    let address = [0xAAu8; 20];
    let slot = [0xBBu8; 32];

    let mut expected = Vec::new();
    expected.extend_from_slice(&twox_128(b"pallet_evm"));
    expected.extend_from_slice(&twox_128(b"AccountStorages"));
    expected.extend_from_slice(&twox_64(&address));
    expected.extend_from_slice(&address);
    expected.extend_from_slice(&blake2_128(&slot));
    expected.extend_from_slice(&slot);
    assert_eq!(evm_storage_slot_key(&address, &slot), expected);
    assert_ne!(
        evm_storage_slot_key(&address, &slot),
        evm_storage_slot_key(&address, &[0xCC; 32])
    );

    let mut expected_code = Vec::new();
    expected_code.extend_from_slice(&twox_128(b"pallet_evm"));
    expected_code.extend_from_slice(&twox_128(b"AccountCodes"));
    expected_code.extend_from_slice(&blake2_128(&address));
    expected_code.extend_from_slice(&address);
    assert_eq!(evm_account_code_key(&address), expected_code);
    assert_ne!(
        evm_account_code_key(&address),
        evm_account_code_key(&[0xBB; 20])
    );
}

#[test]
fn svm_and_x3vm_storage_keys_are_pinned() {
    use sp_io::hashing::{blake2_128, twox_128};

    let pubkey = [0x21u8; 32];
    let mut expected = Vec::new();
    expected.extend_from_slice(&twox_128(b"pallet_svm_runtime"));
    expected.extend_from_slice(&twox_128(b"AccountData"));
    expected.extend_from_slice(&blake2_128(&pubkey));
    expected.extend_from_slice(&pubkey);
    assert_eq!(svm_account_data_key(&pubkey), expected);

    let key = [0x31u8; 32];
    let mut expected = Vec::new();
    expected.extend_from_slice(&twox_128(b"x3_vm"));
    expected.extend_from_slice(&twox_128(b"VmStorage"));
    expected.extend_from_slice(&blake2_128(&key));
    expected.extend_from_slice(&key);
    assert_eq!(x3vm_storage_slot_key(&key), expected);
    assert_ne!(
        x3vm_storage_slot_key(&key),
        x3vm_storage_slot_key(&[0x32; 32])
    );
}

#[test]
fn overlay_ledger_storage_key_is_pinned_per_domain() {
    use sp_io::hashing::{blake2_128, twox_128};

    let mut keys = Vec::new();
    for (domain, byte) in [
        (OverlayDomain::Evm, 0u8),
        (OverlayDomain::Svm, 1u8),
        (OverlayDomain::X3, 2u8),
    ] {
        let change = OverlayLegChange {
            domain,
            address: vec![0x42; 32],
            key: H256([0x51; 32]),
            old_value: None,
            new_value: None,
        };
        let mut map_key = vec![byte];
        map_key.extend_from_slice(&change.address);
        map_key.extend_from_slice(change.key.as_bytes());

        let mut expected = Vec::new();
        expected.extend_from_slice(&twox_128(b"x3-atomic-kernel"));
        expected.extend_from_slice(&twox_128(b"OverlayLedger"));
        expected.extend_from_slice(&blake2_128(&map_key));
        expected.extend_from_slice(&map_key);
        assert_eq!(overlay_ledger_storage_key(&change), expected, "{domain:?}");
        keys.push(overlay_ledger_storage_key(&change));
    }
    assert_ne!(keys[0], keys[1]);
    assert_ne!(keys[1], keys[2]);
    assert_ne!(keys[0], keys[2]);
}

// ── Tallies ────────────────────────────────────────────────────────────────

#[test]
fn evm_slot_tally_counts_restored_and_deleted() {
    let changes = vec![
        make_evm_change(0x01, &[0xAA; 32], &[], None),
        make_evm_change(0x02, &[0xBB; 32], &[], None),
        make_evm_change(0x03, &[], &[0xCC; 32], None),
    ];
    assert_eq!(evm_slot_tally(&changes), (2, 1));
    assert_eq!(
        evm_slot_tally(&[make_evm_change(0x04, &[], &[0xDD; 32], None)]),
        (0, 1)
    );
    assert_eq!(
        evm_slot_tally(&[make_evm_change(0x05, &[0xEE; 32], &[], None)]),
        (1, 0)
    );
    assert_eq!(evm_slot_tally(&[]), (0, 0));
}

#[test]
fn x3vm_and_overlay_restored_counts_only_count_restores() {
    let x3vm = vec![
        X3VmStorageChange {
            key: [0x01; 32],
            old_value: Some([0xAA; 32]),
        },
        X3VmStorageChange {
            key: [0x02; 32],
            old_value: None,
        },
    ];
    assert_eq!(x3vm_restored_count(&x3vm), 1);
    assert_eq!(x3vm_restored_count(&[]), 0);

    let overlay = vec![
        OverlayLegChange {
            domain: OverlayDomain::Evm,
            address: vec![0x01],
            key: H256([0x01; 32]),
            old_value: None,
            new_value: Some(H256([0xAA; 32])),
        },
        OverlayLegChange {
            domain: OverlayDomain::Svm,
            address: vec![0x02],
            key: H256([0x02; 32]),
            old_value: Some(H256([0xBB; 32])),
            new_value: None,
        },
    ];
    assert_eq!(overlay_restored_count(&overlay), 1);
    assert_eq!(overlay_restored_count(&[]), 0);
}

// ── Reverter storage effects ───────────────────────────────────────────────

#[test]
fn evm_reverter_writes_old_values_and_clears_new_slots() {
    run(|| {
        let contract = [0xCAu8; 20];
        let restored = make_evm_change(0x01, &[0xAA; 32], &[0xBB; 32], Some(contract));
        let deleted = make_evm_change(0x02, &[], &[0xCC; 32], Some(contract));
        let changes = vec![restored.clone(), deleted.clone()];
        let diff = encode_evm_state_diff(&changes, Some(contract));

        // The deleted slot must exist beforehand to prove the revert removes it.
        let deleted_key = evm_storage_slot_key(&contract, &deleted.key);
        sp_io::storage::set(&deleted_key, &[0xCC; 32]);

        let outcome = EvmReverter::revert(&diff).unwrap();
        assert_eq!(outcome, RevertOutcome::Reverted);

        let restored_key = evm_storage_slot_key(&contract, &restored.key);
        assert_eq!(
            sp_io::storage::get(&restored_key).unwrap().to_vec(),
            restored.old_value
        );
        assert_eq!(sp_io::storage::get(&deleted_key), None);
    });
}

#[test]
fn svm_reverter_writes_and_clears_account_data() {
    run(|| {
        let restored = SvmStorageChange {
            account: [0x21; 32],
            key: b"balance".to_vec(),
            old_value: vec![0xDE, 0xAD],
        };
        let deleted = SvmStorageChange {
            account: [0x22; 32],
            key: b"data".to_vec(),
            old_value: Vec::new(),
        };
        let diff = encode_svm_state_diff(&[restored.clone(), deleted.clone()]);
        let deleted_key = svm_account_data_key(&deleted.account);
        sp_io::storage::set(&deleted_key, &[0xEF; 2]);

        assert_eq!(SvmReverter::revert(&diff).unwrap(), RevertOutcome::Reverted);
        assert_eq!(
            sp_io::storage::get(&svm_account_data_key(&restored.account))
                .unwrap()
                .to_vec(),
            restored.old_value
        );
        assert_eq!(sp_io::storage::get(&deleted_key), None);
    });
}

#[test]
fn x3vm_reverter_writes_and_clears_storage_slots() {
    run(|| {
        let restored = X3VmStorageChange {
            key: [0x31; 32],
            old_value: Some([0xAA; 32]),
        };
        let deleted = X3VmStorageChange {
            key: [0x32; 32],
            old_value: None,
        };
        let diff = encode_x3vm_state_diff(&[restored.clone(), deleted.clone()]);
        let deleted_key = x3vm_storage_slot_key(&deleted.key);
        sp_io::storage::set(&deleted_key, &[0xEF; 32]);

        assert_eq!(
            X3VmReverter::revert(&diff).unwrap(),
            RevertOutcome::Reverted
        );
        assert_eq!(
            sp_io::storage::get(&x3vm_storage_slot_key(&restored.key))
                .unwrap()
                .to_vec(),
            restored.old_value.unwrap().to_vec()
        );
        assert_eq!(sp_io::storage::get(&deleted_key), None);
    });
}

#[test]
fn overlay_reverter_writes_and_clears_exact_entries() {
    run(|| {
        let restored = OverlayLegChange {
            domain: OverlayDomain::X3,
            address: vec![0x71; 32],
            key: H256([0x77; 32]),
            old_value: Some(H256([0x99; 32])),
            new_value: Some(H256([0xAA; 32])),
        };
        let deleted = OverlayLegChange {
            domain: OverlayDomain::Svm,
            address: vec![0x72; 32],
            key: H256([0x78; 32]),
            old_value: None,
            new_value: Some(H256([0xBB; 32])),
        };
        let diff = encode_overlay_state_diff(&[restored.clone(), deleted.clone()]);
        let deleted_key = overlay_ledger_storage_key(&deleted);
        sp_io::storage::set(&deleted_key, &[0xBB; 32]);

        assert_eq!(
            OverlayReverter::revert(&diff).unwrap(),
            RevertOutcome::Reverted
        );
        assert_eq!(
            sp_io::storage::get(&overlay_ledger_storage_key(&restored))
                .unwrap()
                .to_vec(),
            restored.old_value.unwrap().as_bytes().to_vec()
        );
        assert_eq!(sp_io::storage::get(&deleted_key), None);
    });
}

#[test]
fn composite_reverter_routes_non_overlay_diffs_to_their_vm_decoder() {
    run(|| {
        let svm = SvmStorageChange {
            account: [0x21; 32],
            key: b"data".to_vec(),
            old_value: vec![0x01],
        };
        let svm_diff = encode_svm_state_diff(&[svm]);
        assert_eq!(
            CompositeReverter::revert_leg(VmType::Svm, &svm_diff).unwrap(),
            RevertOutcome::Reverted
        );

        let x3vm = X3VmStorageChange {
            key: [0x31; 32],
            old_value: Some([0xAA; 32]),
        };
        let x3vm_diff = encode_x3vm_state_diff(&[x3vm]);
        assert_eq!(
            CompositeReverter::revert_leg(VmType::X3, &x3vm_diff).unwrap(),
            RevertOutcome::Reverted
        );

        let cross = CompositeReverter::revert_leg(VmType::Cross, &x3vm_diff).unwrap();
        assert_eq!(cross, RevertOutcome::Reverted);
    });
}

#[test]
fn leg_receipt_mark_executed_records_the_diff() {
    let mut receipt = LegReceipt::new(3, VmType::X3);
    assert!(!receipt.executed);
    assert!(receipt.state_diff.is_empty());

    let diff = StateDiff::from(vec![1, 2, 3, 4]);
    receipt.mark_executed(diff.clone());
    assert!(receipt.executed);
    assert_eq!(receipt.state_diff, diff);
}
