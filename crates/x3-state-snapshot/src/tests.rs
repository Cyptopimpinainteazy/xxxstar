//! The audit's snapshot "murder test" matrix: every one of these inputs must be
//! refused, and the honest one must be accepted.

use super::*;

const CHAIN_ID: &str = "x3_testnet_v1";
const BLOCK_HASH: &str = "0xaabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";
const STATE_ROOT: &str = "0x00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
const RUNTIME_VERSION: u32 = 23;

fn chunk_bytes(tag: u8, len: usize) -> Vec<u8> {
    (0..len).map(|i| tag.wrapping_add(i as u8)).collect()
}

/// A well-formed snapshot: three chunks of 8 bytes each.
fn honest_snapshot() -> (SnapshotManifest, Vec<Option<Vec<u8>>>) {
    let chunks: Vec<Vec<u8>> = (1..=3).map(|tag| chunk_bytes(tag, 8)).collect();
    let manifest = SnapshotManifest {
        format: SNAPSHOT_MAGIC.to_string(),
        format_version: SNAPSHOT_FORMAT_VERSION,
        chain_id: CHAIN_ID.to_string(),
        block_number: 4_242,
        block_hash: BLOCK_HASH.to_string(),
        state_root: STATE_ROOT.to_string(),
        runtime_spec_version: RUNTIME_VERSION,
        database_format: "rocksdb-substrate-trie-v1".to_string(),
        chunk_size: 8,
        chunk_count: chunks.len() as u64,
        chunk_hashes: chunks.iter().map(|c| chunk_hash(c)).collect(),
        finality_proof: "0xdeadbeef".to_string(),
    };
    (manifest, chunks.into_iter().map(Some).collect())
}

fn anchor<'a>(manifest_hash: Option<&'a str>) -> TrustedAnchor<'a> {
    TrustedAnchor {
        chain_id: CHAIN_ID,
        block_hash: BLOCK_HASH,
        state_root: STATE_ROOT,
        runtime_spec_version: RUNTIME_VERSION,
        minimum_block_number: 0,
        manifest_hash,
    }
}

#[test]
fn an_honest_snapshot_is_accepted() {
    let (manifest, chunks) = honest_snapshot();
    assert_eq!(verify_snapshot(&manifest, &anchor(None), &chunks), Ok(()));
}

#[test]
fn a_snapshot_from_another_format_version_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.format_version = SNAPSHOT_FORMAT_VERSION + 1;

    assert_eq!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::UnsupportedFormatVersion {
            got: SNAPSHOT_FORMAT_VERSION + 1,
            expected: SNAPSHOT_FORMAT_VERSION,
        })
    );
}

#[test]
fn a_file_that_is_not_our_format_at_all_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.format = "SomeOtherBackup".to_string();

    assert_eq!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::NotASnapshotManifest {
            got: "SomeOtherBackup".to_string(),
        })
    );
}

#[test]
fn a_snapshot_from_another_chain_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.chain_id = "x3_mainnet".to_string();

    assert_eq!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::WrongChain {
            expected: CHAIN_ID.to_string(),
            got: "x3_mainnet".to_string(),
        })
    );
}

#[test]
fn a_snapshot_anchored_to_a_different_block_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.block_hash =
        "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();

    assert!(matches!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::WrongBlockHash { .. })
    ));
}

#[test]
fn a_snapshot_carrying_a_different_state_root_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.state_root =
        "0x1111111111111111111111111111111111111111111111111111111111111111".to_string();

    assert!(matches!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::WrongStateRoot { .. })
    ));
}

#[test]
fn a_snapshot_from_a_different_runtime_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.runtime_spec_version = RUNTIME_VERSION + 1;

    assert_eq!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::WrongRuntimeVersion {
            expected: RUNTIME_VERSION,
            got: RUNTIME_VERSION + 1,
        })
    );
}

#[test]
fn a_stale_snapshot_is_refused_when_the_caller_sets_a_floor() {
    let (manifest, chunks) = honest_snapshot();
    let mut stale_anchor = anchor(None);
    stale_anchor.minimum_block_number = manifest.block_number + 1;

    assert_eq!(
        verify_snapshot(&manifest, &stale_anchor, &chunks),
        Err(SnapshotError::Stale {
            got: manifest.block_number,
            minimum: manifest.block_number + 1,
        })
    );

    // The same snapshot is fine once the floor is lowered to its own height.
    stale_anchor.minimum_block_number = manifest.block_number;
    assert_eq!(verify_snapshot(&manifest, &stale_anchor, &chunks), Ok(()));
}

#[test]
fn a_manifest_without_a_finality_proof_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.finality_proof = "   ".to_string();

    assert_eq!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::MissingFinalityProof)
    );
}

#[test]
fn a_corrupt_chunk_is_refused_at_the_chunk_that_is_wrong() {
    let (manifest, mut chunks) = honest_snapshot();
    chunks[1].as_mut().expect("chunk 1 present")[3] ^= 0x01;

    match verify_chunk_set(&manifest, &chunks) {
        Err(SnapshotError::ChunkHashMismatch { index, .. }) => assert_eq!(index, 1),
        other => panic!("expected a hash mismatch on chunk 1, got {other:?}"),
    }
}

#[test]
fn chunks_presented_out_of_order_are_refused() {
    let (manifest, mut chunks) = honest_snapshot();
    chunks.swap(0, 1);

    match verify_chunk_set(&manifest, &chunks) {
        Err(SnapshotError::ChunkHashMismatch { index, .. }) => assert_eq!(index, 0),
        other => panic!("expected chunk 0 to fail once reordered, got {other:?}"),
    }
}

#[test]
fn an_incomplete_snapshot_is_refused() {
    let (manifest, mut chunks) = honest_snapshot();
    let missing = chunks.len() as u64 - 1;
    chunks.pop();

    assert_eq!(
        verify_chunk_set(&manifest, &chunks),
        Err(SnapshotError::UnexpectedChunkCount {
            declared: manifest.chunk_count,
            actual: missing as usize,
        })
    );

    // A dropped chunk in the middle must be caught even when the count matches.
    let (manifest, mut chunks) = honest_snapshot();
    chunks[2] = None;
    assert_eq!(
        verify_chunk_set(&manifest, &chunks),
        Err(SnapshotError::MissingChunk { index: 2 })
    );
}

#[test]
fn a_manifest_that_lies_about_its_chunk_count_is_refused() {
    let (mut manifest, chunks) = honest_snapshot();
    manifest.chunk_count += 1;

    assert!(matches!(
        verify_snapshot(&manifest, &anchor(None), &chunks),
        Err(SnapshotError::ChunkCountMismatch { .. })
    ));
}

#[test]
fn a_chunk_larger_than_the_declared_chunk_size_is_refused() {
    let (mut manifest, mut chunks) = honest_snapshot();
    let big = vec![9u8; manifest.chunk_size as usize + 1];
    manifest.chunk_hashes[0] = chunk_hash(&big);
    chunks[0] = Some(big);
    let declared = manifest.chunk_size as usize + 1;

    assert_eq!(
        verify_chunk_set(&manifest, &chunks),
        Err(SnapshotError::ChunkTooLarge {
            index: 0,
            declared: manifest.chunk_size,
            got: declared,
        })
    );
}

#[test]
fn a_manifest_hash_that_does_not_match_the_published_one_is_refused() {
    let (manifest, chunks) = honest_snapshot();
    let wrong = "0x0000000000000000000000000000000000000000000000000000000000000000";

    assert!(matches!(
        verify_snapshot(&manifest, &anchor(Some(wrong)), &chunks),
        Err(SnapshotError::ManifestHashMismatch { .. })
    ));

    // The published hash of the honest manifest is accepted, case-insensitively.
    let correct = manifest_hash(&manifest).expect("manifest hashes");
    assert_eq!(
        verify_snapshot(&manifest, &anchor(Some(&correct.to_uppercase())), &chunks),
        Ok(())
    );
}

#[test]
fn a_malformed_hash_field_is_refused_rather_than_silently_ignored() {
    for bad in ["not-hex", "0x00", ""] {
        let (mut manifest, chunks) = honest_snapshot();
        manifest.state_root = bad.to_string();
        assert!(
            matches!(
                verify_snapshot(&manifest, &anchor(None), &chunks),
                Err(SnapshotError::MalformedHash { .. })
            ),
            "{bad:?} must be refused"
        );
    }
}

#[test]
fn the_manifest_hash_is_stable_and_covers_every_field() {
    let (manifest, _) = honest_snapshot();
    let baseline = manifest_hash(&manifest).expect("manifest hashes");
    assert_eq!(
        baseline,
        manifest_hash(&manifest).expect("manifest hashes again")
    );

    // Casing of hex fields is presentation, not meaning.
    let mut upper = manifest.clone();
    upper.block_hash = manifest.block_hash.to_uppercase().replacen("0X", "0x", 1);
    assert_eq!(
        baseline,
        manifest_hash(&upper).expect("uppercase manifest hashes"),
        "hex casing must not change the manifest hash"
    );

    // Every semantic field must move the hash.
    let mut mutations: Vec<(&str, SnapshotManifest)> = Vec::new();
    let mut m = manifest.clone();
    m.chain_id = "x3_other".to_string();
    mutations.push(("chain_id", m));
    let mut m = manifest.clone();
    m.block_number += 1;
    mutations.push(("block_number", m));
    let mut m = manifest.clone();
    m.runtime_spec_version += 1;
    mutations.push(("runtime_spec_version", m));
    let mut m = manifest.clone();
    m.database_format = "another-format".to_string();
    mutations.push(("database_format", m));
    let mut m = manifest.clone();
    m.finality_proof = "0xcafebabe".to_string();
    mutations.push(("finality_proof", m));

    for (field, mutated) in mutations {
        assert_ne!(
            baseline,
            manifest_hash(&mutated).expect("mutated manifest hashes"),
            "changing {field} must change the manifest hash"
        );
    }
}

#[test]
fn a_manifest_serialises_and_reloads_to_the_same_hash() {
    // The manifest travels as JSON, so the round trip has to be lossless.
    let (manifest, _) = honest_snapshot();
    let json = serde_json::to_string_pretty(&manifest).expect("manifest serialises");
    let reloaded: SnapshotManifest = serde_json::from_str(&json).expect("manifest parses");

    assert_eq!(reloaded, manifest);
    assert_eq!(
        manifest_hash(&manifest).expect("hash"),
        manifest_hash(&reloaded).expect("hash")
    );
}

// ── Export ──────────────────────────────────────────────────────────────────

fn entries() -> Vec<(Vec<u8>, Vec<u8>)> {
    vec![
        (vec![0x26, 0xaa], vec![0x01]),
        (vec![0x1d, 0xa5], vec![0x00, 0x00]),
        (vec![0x26, 0xaa, 0x39, 0x4e], vec![0xde, 0xad, 0xbe, 0xef]),
    ]
}

fn builder() -> SnapshotBuilder {
    SnapshotBuilder {
        chain_id: CHAIN_ID.to_string(),
        block_number: 4_242,
        block_hash: BLOCK_HASH.to_string(),
        state_root: STATE_ROOT.to_string(),
        runtime_spec_version: RUNTIME_VERSION,
        database_format: "rocksdb-substrate-trie-v1".to_string(),
        chunk_size: 16,
        finality_proof: "0xdeadbeef".to_string(),
    }
}

#[test]
fn the_state_stream_does_not_depend_on_the_order_entries_arrive_in() {
    let in_order = entries();
    let mut shuffled = entries();
    shuffled.reverse();

    assert_eq!(
        encode_state_entries(&in_order).expect("encode"),
        encode_state_entries(&shuffled).expect("encode"),
        "two nodes visiting the same state in different orders must agree byte for byte"
    );
}

#[test]
fn a_repeated_storage_key_is_refused_rather_than_collapsed() {
    let mut duplicated = entries();
    duplicated.push((vec![0x26, 0xaa], vec![0xff]));

    assert_eq!(
        encode_state_entries(&duplicated),
        Err(SnapshotError::DuplicateKey {
            key: "0x26aa".to_string(),
        })
    );
}

#[test]
fn an_empty_state_is_refused() {
    assert_eq!(encode_state_entries(&[]), Err(SnapshotError::EmptyState));
}

#[test]
fn the_state_stream_round_trips() {
    let stream = encode_state_entries(&entries()).expect("encode");
    let mut decoded = decode_state_entries(&stream).expect("decode");
    let mut expected = entries();
    // The stream is canonically ordered, so compare in that order.
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    decoded.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(decoded, expected);
}

#[test]
fn a_truncated_state_stream_is_refused() {
    let stream = encode_state_entries(&entries()).expect("encode");

    for cut in [1usize, 4, stream.len() - 1] {
        let truncated = &stream[..stream.len() - cut];
        assert!(
            matches!(
                decode_state_entries(truncated),
                Err(SnapshotError::MalformedStateStream { .. })
            ),
            "truncating {cut} byte(s) must be refused"
        );
    }

    assert_eq!(decode_state_entries(&[]), Err(SnapshotError::EmptyState));
}

#[test]
fn chunking_is_deterministic_and_only_the_last_chunk_may_be_short() {
    let stream: Vec<u8> = (0..35u8).collect();
    let chunks = split_into_chunks(&stream, 16).expect("split");

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].len(), 16);
    assert_eq!(chunks[1].len(), 16);
    assert_eq!(chunks[2].len(), 3);
    assert_eq!(chunks.concat(), stream);
    assert_eq!(chunks, split_into_chunks(&stream, 16).expect("split again"));
}

#[test]
fn a_zero_chunk_size_or_an_empty_stream_is_refused() {
    assert_eq!(
        split_into_chunks(&[1, 2, 3], 0),
        Err(SnapshotError::ZeroChunkSize)
    );
    assert_eq!(split_into_chunks(&[], 16), Err(SnapshotError::EmptyState));
}

#[test]
fn the_builder_commits_to_the_chunks_it_returns() {
    let (manifest, chunks) = builder().build(&entries()).expect("build");

    assert_eq!(manifest.chunk_count, chunks.len() as u64);
    assert_eq!(
        manifest.chunk_hashes,
        chunks
            .iter()
            .map(|chunk| chunk_hash(chunk))
            .collect::<Vec<_>>()
    );
    // A chunk size of 16 over this state gives more than one chunk, so the
    // split is actually exercised rather than everything landing in chunk 0.
    assert!(
        chunks.len() > 1,
        "expected the fixture to span several chunks"
    );
}

#[test]
fn exporting_the_same_state_twice_produces_the_same_manifest_hash() {
    let (first, _) = builder().build(&entries()).expect("build");
    let mut shuffled = entries();
    shuffled.reverse();
    let (second, _) = builder().build(&shuffled).expect("build");

    assert_eq!(
        manifest_hash(&first).expect("hash"),
        manifest_hash(&second).expect("hash")
    );
}

#[test]
fn a_written_snapshot_round_trips_through_disk_and_verifies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (manifest, chunks) = builder().build(&entries()).expect("build");
    write_snapshot(dir.path(), &manifest, &chunks).expect("write");

    // The manifest is written last; if it is there, every chunk is there.
    assert!(dir.path().join("manifest.json").exists());
    for index in 0..manifest.chunk_count {
        assert!(chunk_path(dir.path(), index).exists(), "chunk {index}");
    }

    let (reloaded, read_chunks) = read_snapshot(dir.path()).expect("read");
    assert_eq!(reloaded, manifest);
    assert_eq!(
        verify_snapshot(&reloaded, &anchor(None), &read_chunks),
        Ok(())
    );

    // And the entries come back out of the restored stream.
    let stream: Vec<u8> = read_chunks
        .iter()
        .flat_map(|chunk| chunk.clone().expect("chunk present"))
        .collect();
    assert_eq!(
        decode_state_entries(&stream).expect("decode").len(),
        entries().len()
    );
}

#[test]
fn a_snapshot_directory_missing_a_chunk_is_refused_even_with_a_valid_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut anchor_builder = builder();
    anchor_builder.chunk_size = 4; // force several chunks
    let (manifest, chunks) = anchor_builder.build(&entries()).expect("build");
    assert!(chunks.len() > 1, "the fixture must span several chunks");
    write_snapshot(dir.path(), &manifest, &chunks).expect("write");

    std::fs::remove_file(chunk_path(dir.path(), 1)).expect("remove chunk 1");

    let (reloaded, read_chunks) = read_snapshot(dir.path()).expect("read");
    assert_eq!(
        verify_snapshot(&reloaded, &anchor(None), &read_chunks),
        Err(SnapshotError::MissingChunk { index: 1 })
    );
}

fn raw_spec(top: serde_json::Value, children: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "name": "fixture",
        "id": CHAIN_ID,
        "genesis": { "raw": { "top": top, "childrenDefault": children } }
    })
}

#[test]
fn storage_entries_come_out_of_a_raw_spec_in_canonical_order() {
    let spec = raw_spec(
        serde_json::json!({
            "0xbb": "0x02",
            "0xaa": "0x01",
            "0x01": "0xdeadbeef"
        }),
        serde_json::json!({}),
    );

    let parsed = state_entries_from_raw_spec(&spec).expect("parse");
    // The parse order is whatever the JSON object gives us (serde_json sorts
    // map keys); it is deliberately NOT load-bearing, because the stream
    // encoder sorts by raw key bytes regardless of how entries arrive.
    let mut as_pairs = parsed.clone();
    as_pairs.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        as_pairs,
        vec![
            (vec![0x01], vec![0xde, 0xad, 0xbe, 0xef]),
            (vec![0xaa], vec![0x01]),
            (vec![0xbb], vec![0x02]),
        ]
    );

    // And the stream built from them is canonical regardless of that order.
    let stream = encode_state_entries(&parsed).expect("encode");
    let decoded = decode_state_entries(&stream).expect("decode");
    assert_eq!(
        decoded.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
        vec![vec![0x01], vec![0xaa], vec![0xbb]]
    );
}

#[test]
fn raw_spec_child_tries_are_refused_rather_than_dropped() {
    let spec = raw_spec(
        serde_json::json!({ "0xaa": "0x01" }),
        serde_json::json!({ "0xchild": { "0xbb": "0x02" } }),
    );

    assert!(matches!(
        state_entries_from_raw_spec(&spec),
        Err(SnapshotError::UnsupportedChildTries { .. })
    ));
}

#[test]
fn a_raw_spec_without_genesis_raw_is_refused() {
    let spec = serde_json::json!({ "name": "fixture" });
    assert!(matches!(
        state_entries_from_raw_spec(&spec),
        Err(SnapshotError::MalformedRawSpec { .. })
    ));

    let spec = serde_json::json!({ "genesis": { "raw": { "childrenDefault": {} } } });
    assert!(matches!(
        state_entries_from_raw_spec(&spec),
        Err(SnapshotError::MalformedRawSpec { .. })
    ));
}

#[test]
fn malformed_hex_in_a_raw_spec_is_refused() {
    for (key, value) in [("0xzz", "0x01"), ("0xaa", "0xzz"), ("aa", "0x01")] {
        let spec = raw_spec(serde_json::json!({ key: value }), serde_json::json!({}));
        assert!(
            matches!(
                state_entries_from_raw_spec(&spec),
                Err(SnapshotError::MalformedRawSpec { .. })
            ),
            "{key}={value} must be refused"
        );
    }
}

// ── State-root recomputation ────────────────────────────────────────────────

#[test]
fn the_recomputed_root_matches_the_state_machines_own_reference_implementation() {
    // `sp_state_machine::basic` is the SDK's own "root of a set of pairs"
    // implementation, and it maps StateVersion to exactly the two layout calls
    // `compute_state_root` uses. If these two disagree, the recompute is wrong
    // and every snapshot check built on it is worthless — so this is the test
    // that has to hold before the rest means anything.
    use sp_core::storage::StateVersion;
    use sp_core::traits::Externalities;
    use sp_state_machine::BasicExternalities;

    let entries = entries();

    for (version, sdk_version) in [
        (TrieVersion::V0, StateVersion::V0),
        (TrieVersion::V1, StateVersion::V1),
    ] {
        let mut basic = BasicExternalities::new_empty();
        for (key, value) in &entries {
            basic.insert(key.clone(), value.clone());
        }
        let reference = format!("0x{}", hex::encode(basic.storage_root(sdk_version)));

        assert_eq!(
            compute_state_root(&entries, version).expect("compute"),
            reference,
            "{version:?} must agree with the state machine"
        );
    }
}

#[test]
fn the_root_does_not_depend_on_the_order_entries_arrive_in() {
    let mut shuffled = entries();
    shuffled.reverse();

    assert_eq!(
        compute_state_root(&entries(), TrieVersion::V1).expect("compute"),
        compute_state_root(&shuffled, TrieVersion::V1).expect("compute")
    );
}

#[test]
fn changing_one_value_changes_the_root() {
    let baseline = compute_state_root(&entries(), TrieVersion::V1).expect("compute");

    let mut mutated = entries();
    mutated[0].1[0] ^= 0x01;

    assert_ne!(
        baseline,
        compute_state_root(&mutated, TrieVersion::V1).expect("compute")
    );
}

#[test]
fn the_two_trie_layouts_produce_different_roots() {
    // The layouts only diverge once a value is large enough to be externalised
    // into a value node, so both halves are worth pinning: identical roots for
    // small values, different roots once a large value is present.
    let small = entries();
    assert_eq!(
        compute_state_root(&small, TrieVersion::V0).expect("compute"),
        compute_state_root(&small, TrieVersion::V1).expect("compute"),
        "with nothing above the inline threshold the two layouts must agree"
    );

    let mut large = entries();
    large[1].1 = vec![0xab; 200];
    assert_ne!(
        compute_state_root(&large, TrieVersion::V0).expect("compute"),
        compute_state_root(&large, TrieVersion::V1).expect("compute"),
        "a value big enough to become a value node must move the V1 root"
    );
}

#[test]
fn a_build_then_verify_round_trip_recomputes_the_declared_root() {
    let entries = entries();
    let derived = compute_state_root(&entries, TrieVersion::V1).expect("compute");

    let mut builder = builder();
    builder.state_root = derived.clone();
    let (manifest, chunks) = builder.build(&entries).expect("build");
    let chunk_slots: Vec<ChunkSlot> = chunks.into_iter().map(Some).collect();
    let anchor = TrustedAnchor {
        state_root: &derived,
        ..anchor(None)
    };

    assert_eq!(
        verify_state_root(&manifest, &chunk_slots, TrieVersion::V1),
        Ok(())
    );
    assert_eq!(
        verify_snapshot_with_state_root(&manifest, &anchor, &chunk_slots, TrieVersion::V1),
        Ok(())
    );
}

#[test]
fn a_snapshot_whose_bytes_do_not_produce_the_declared_root_is_refused() {
    let entries = entries();
    let derived = compute_state_root(&entries, TrieVersion::V1).expect("compute");

    // Same chunk bytes, a manifest that points at some other block's root.
    let mut builder = builder();
    builder.state_root = derived;
    let (mut manifest, chunks) = builder.build(&entries).expect("build");
    let chunk_slots: Vec<ChunkSlot> = chunks.into_iter().map(Some).collect();

    manifest.state_root =
        "0x1111111111111111111111111111111111111111111111111111111111111111".to_string();

    match verify_state_root(&manifest, &chunk_slots, TrieVersion::V1) {
        Err(SnapshotError::RecomputedStateRootMismatch {
            declared,
            recomputed,
        }) => {
            assert_eq!(declared, manifest.state_root);
            assert_ne!(declared, recomputed);
        }
        other => panic!("expected a recomputed-root mismatch, got {other:?}"),
    }
}

#[test]
fn verification_detects_state_swapped_in_under_a_copied_manifest() {
    // The attack the recompute exists for: a mirror keeps the manifest (so the
    // hash, block hash and state root all still line up with what the operator
    // was told) and serves different state in the chunks.
    let honest = entries();
    let derived = compute_state_root(&honest, TrieVersion::V1).expect("compute");

    let mut builder = builder();
    builder.state_root = derived;
    let (manifest, _) = builder.build(&honest).expect("build");

    let mut swapped = honest.clone();
    swapped[0].1 = vec![0xff, 0xff, 0xff, 0xff];
    let (_, substituted_chunks) = builder.build(&swapped).expect("build");
    let channel: Vec<ChunkSlot> = substituted_chunks.into_iter().map(Some).collect();

    // Different bytes can change the chunk count, the per-chunk hashes, or both.
    // Whichever it is, the snapshot has to be refused, and it has to be refused
    // here rather than after a database has been written.
    assert!(
        verify_chunk_set(&manifest, &channel).is_err(),
        "substituted state must not pass the chunk layer"
    );
    assert!(
        verify_state_root(&manifest, &channel, TrieVersion::V1).is_err(),
        "substituted state must not survive the full check either"
    );
}

// ── Restore ─────────────────────────────────────────────────────────────────

/// A snapshot of three real entries, anchored to the root its own bytes produce.
fn restorable_snapshot() -> (SnapshotManifest, Vec<ChunkSlot>, String, Vec<StateEntry>) {
    let entries = entries();
    let root = compute_state_root(&entries, TrieVersion::V1).expect("compute");

    let mut builder = builder();
    builder.state_root = root.clone();
    let (manifest, chunks) = builder.build(&entries).expect("build");

    (
        manifest,
        chunks.into_iter().map(Some).collect(),
        root,
        entries,
    )
}

fn restore_request<'a>(
    manifest: &'a SnapshotManifest,
    anchor: &'a TrustedAnchor<'a>,
    chunks: &'a [ChunkSlot],
) -> RestoreRequest<'a> {
    RestoreRequest {
        manifest,
        anchor,
        chunks,
        version: TrieVersion::V1,
        chain_name: "x3 restored".to_string(),
        spec_id: "x3_restored".to_string(),
        template: None,
    }
}

/// The hex map a raw spec carries, for comparing state without caring about order.
fn top_of(spec: &serde_json::Value) -> serde_json::Value {
    spec["genesis"]["raw"]["top"].clone()
}

#[test]
fn a_restored_spec_reproduces_exactly_the_state_that_was_verified() {
    let (manifest, chunks, root, entries) = restorable_snapshot();
    let anchor = TrustedAnchor {
        state_root: &root,
        ..anchor(None)
    };

    let (spec, provenance) =
        restore_snapshot(&restore_request(&manifest, &anchor, &chunks)).expect("restore");

    assert_eq!(provenance.state_root, root);
    assert_eq!(provenance.state_entries, entries.len() as u64);
    assert_eq!(provenance.chunk_count, manifest.chunk_count);
    assert_eq!(provenance.block_hash, BLOCK_HASH);
    assert_eq!(
        provenance.manifest_hash,
        manifest_hash(&manifest).expect("hash")
    );

    // Read it back through the reader the exporter uses, and hash it again: the
    // spec handed to the operator is the state the snapshot verified.
    let reread = state_entries_from_raw_spec(&spec).expect("re-read restored spec");
    // The spec carries the same pairs, in canonical key order — a raw spec is a
    // map, so entry order is not part of either side of this comparison.
    let mut sorted_reread = reread.clone();
    sorted_reread.sort();
    let mut sorted_entries = entries.clone();
    sorted_entries.sort();
    assert_eq!(
        sorted_reread, sorted_entries,
        "the restored spec must carry the same pairs"
    );
    assert_eq!(
        compute_state_root(&reread, TrieVersion::V1).expect("root"),
        root
    );

    // `RawGenesis` denies unknown fields and requires both halves, so both must
    // be there or the node cannot read the spec at all.
    assert_eq!(top_of(&spec).as_object().expect("top").len(), entries.len());
    assert_eq!(
        spec["genesis"]["raw"]["childrenDefault"],
        serde_json::json!({})
    );
}

#[test]
fn a_restored_spec_records_where_its_state_came_from() {
    let (manifest, chunks, root, _) = restorable_snapshot();
    let anchor = TrustedAnchor {
        state_root: &root,
        ..anchor(None)
    };

    let (spec, _) =
        restore_snapshot(&restore_request(&manifest, &anchor, &chunks)).expect("restore");

    // The provenance travels inside the spec, so it cannot be separated from the
    // state it describes by copying one file.
    let properties = spec["properties"].as_object().expect("properties");
    assert_eq!(properties["x3SnapshotChainId"], CHAIN_ID);
    assert_eq!(properties["x3SnapshotBlockNumber"], "4242");
    assert_eq!(properties["x3SnapshotBlockHash"], BLOCK_HASH);
    assert_eq!(properties["x3SnapshotStateRoot"], root);
    assert_eq!(properties["x3SnapshotRuntimeSpecVersion"], "23");
    assert_eq!(properties["x3SnapshotTrieLayout"], "V1");

    // And it does not claim to be a live chain.
    assert_eq!(spec["chainType"], "Local");
    assert_eq!(spec["name"], "x3 restored");
    assert_eq!(spec["id"], "x3_restored");
}

#[test]
fn restoring_a_tampered_chunk_is_refused_before_any_spec_exists() {
    let (manifest, mut chunks, root, _) = restorable_snapshot();
    let anchor = TrustedAnchor {
        state_root: &root,
        ..anchor(None)
    };

    chunks[1] = Some(vec![0xff; 16]);

    match restore_snapshot(&restore_request(&manifest, &anchor, &chunks)) {
        Err(SnapshotError::ChunkHashMismatch { index, .. }) => assert_eq!(index, 1),
        other => panic!("expected the tampered chunk to be refused, got {other:?}"),
    }
}

#[test]
fn restoring_an_incomplete_snapshot_is_refused() {
    let (manifest, mut chunks, root, _) = restorable_snapshot();
    let anchor = TrustedAnchor {
        state_root: &root,
        ..anchor(None)
    };

    chunks[2] = None;

    assert!(matches!(
        restore_snapshot(&restore_request(&manifest, &anchor, &chunks)),
        Err(SnapshotError::MissingChunk { index: 2 })
    ));
}

#[test]
fn restoring_a_snapshot_anchored_to_another_chain_is_refused() {
    let (manifest, chunks, root, _) = restorable_snapshot();
    let anchor = TrustedAnchor {
        chain_id: "x3_mainnet",
        state_root: &root,
        ..anchor(None)
    };

    assert!(matches!(
        restore_snapshot(&restore_request(&manifest, &anchor, &chunks)),
        Err(SnapshotError::WrongChain { .. })
    ));
}

#[test]
fn restoring_a_snapshot_that_is_older_than_the_callers_floor_is_refused() {
    let (manifest, chunks, root, _) = restorable_snapshot();
    let anchor = TrustedAnchor {
        state_root: &root,
        minimum_block_number: manifest.block_number + 1,
        ..anchor(None)
    };

    assert!(matches!(
        restore_snapshot(&restore_request(&manifest, &anchor, &chunks)),
        Err(SnapshotError::Stale { .. })
    ));
}

#[test]
fn restoring_state_swapped_in_under_a_copied_manifest_is_refused() {
    // The mirror keeps every piece of metadata and serves different state: the
    // manifest is self-consistent, the chunks hash to what it commits to, and it
    // is only the root recomputed from the entries that gives it away.
    let (mut manifest, chunks, _, _) = restorable_snapshot();
    manifest.state_root =
        "0x2222222222222222222222222222222222222222222222222222222222222222".to_string();

    let anchor = TrustedAnchor {
        state_root: &manifest.state_root,
        ..anchor(None)
    };

    assert!(matches!(
        restore_snapshot(&restore_request(&manifest, &anchor, &chunks)),
        Err(SnapshotError::RecomputedStateRootMismatch { .. })
    ));
}

#[test]
fn a_template_contributes_metadata_but_never_state() {
    let (manifest, chunks, root, entries) = restorable_snapshot();
    let anchor = TrustedAnchor {
        state_root: &root,
        ..anchor(None)
    };

    // A template that carries its own, different state: a restore must replace it
    // rather than merge it, or the result would be a third state that neither the
    // snapshot nor the chain can name.
    let template = serde_json::json!({
        "name": "X3 Testnet",
        "id": "x3_testnet_alpha",
        "chainType": "Live",
        "bootNodes": ["/ip4/203.0.113.7/tcp/30333/p2p/12D3KooWabcdef"],
        "telemetryEndpoints": null,
        "protocolId": "x3",
        "properties": { "tokenSymbol": "X3", "tokenDecimals": 12 },
        "genesis": {
            "raw": {
                "top": { "0xdead": "0xbeef" },
                "childrenDefault": {}
            }
        }
    });

    let mut request = restore_request(&manifest, &anchor, &chunks);
    request.template = Some(&template);
    request.chain_name = "X3 Testnet snapshot at 4242".to_string();
    request.spec_id = "x3_testnet_alpha_snapshot".to_string();

    let (spec, _) = restore_snapshot(&request).expect("restore");

    // Metadata carried over ...
    assert_eq!(spec["chainType"], "Live");
    assert_eq!(spec["protocolId"], "x3");
    assert_eq!(
        spec["bootNodes"][0],
        "/ip4/203.0.113.7/tcp/30333/p2p/12D3KooWabcdef"
    );
    assert_eq!(spec["properties"]["tokenSymbol"], "X3");
    // ... and merged with provenance, not overwritten by it.
    assert_eq!(spec["properties"]["x3SnapshotStateRoot"], root);
    // ... but the template's name/id are replaced, so a restored spec cannot be
    // mistaken for the chain it was templated from.
    assert_eq!(spec["name"], "X3 Testnet snapshot at 4242");
    assert_eq!(spec["id"], "x3_testnet_alpha_snapshot");
    assert_ne!(spec["id"], template["id"]);

    // The template's state is gone; only the snapshot's entries remain.
    let top = top_of(&spec);
    assert_eq!(top.as_object().expect("top").len(), entries.len());
    assert!(
        top.get("0xdead").is_none(),
        "template state must be replaced"
    );
}
