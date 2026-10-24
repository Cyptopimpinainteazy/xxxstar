//! Bitcoin's own data, and forgeries Bitcoin would refuse.
//!
//! Every mainnet value here is checked by the code under test rather than trusted: a header whose
//! recalled fields were wrong would not hash under its target, a Merkle root that was misremembered
//! would not be reached by its transactions. The retarget vectors are Bitcoin Core's
//! `pow_tests.cpp` cases.

use x3_btc_spv::{
    bits_from_target, check_proof_of_work, display_order, median_time_past, merkle_root_from_path,
    next_work_required, sha256d, target_from_bits, verify_merkle_inclusion, verify_transaction,
    work_from_bits, BlockHeader, Checkpoint, Hash, LightClient, Params, SpvError, U256,
};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

/// A hash as an explorer displays it, converted to internal order.
fn h(display: &str) -> Hash {
    let bytes: Hash = hex(display).try_into().expect("32 bytes");
    display_order(&bytes)
}

fn genesis() -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_blockhash: [0; 32],
        merkle_root: h("4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"),
        time: 1231006505,
        bits: 0x1d00ffff,
        nonce: 2083236893,
    }
}

fn block1() -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_blockhash: genesis().block_hash(),
        merkle_root: h("0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098"),
        time: 1231469665,
        bits: 0x1d00ffff,
        nonce: 2573394689,
    }
}

fn block2() -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_blockhash: block1().block_hash(),
        merkle_root: h("9b0fc92260312ce44e74ef369f5c66bbb85848f2eddd5a7a1cde251e54ccfdd5"),
        time: 1231469744,
        bits: 0x1d00ffff,
        nonce: 1639830024,
    }
}

const GENESIS_COINBASE: &str = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000";

fn mainnet_client() -> LightClient {
    LightClient::new(
        Params::mainnet(),
        Checkpoint {
            height: 0,
            header: genesis(),
            prior_timestamps: vec![],
        },
    )
    .expect("genesis is a valid checkpoint")
}

#[test]
fn mainnet_headers_hash_to_their_published_ids_and_meet_their_targets() {
    let mainnet = Params::mainnet();
    for (header, id) in [
        (
            genesis(),
            "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
        ),
        (
            block1(),
            "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048",
        ),
        (
            block2(),
            "000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd",
        ),
    ] {
        assert_eq!(header.block_hash(), h(id));
        assert_eq!(check_proof_of_work(&header, &mainnet), Ok(()));
        assert_eq!(BlockHeader::decode(&header.encode()), Ok(header));
    }
}

#[test]
fn a_real_header_with_one_bit_changed_no_longer_meets_its_target() {
    let mut header = block1();
    header.nonce ^= 1;
    assert_eq!(
        check_proof_of_work(&header, &Params::mainnet()),
        Err(SpvError::InsufficientProofOfWork)
    );
}

#[test]
fn the_genesis_coinbase_is_its_blocks_only_transaction() {
    let raw = hex(GENESIS_COINBASE);
    let txid = verify_transaction(&raw, 0, &[], &genesis().merkle_root).expect("included");
    assert_eq!(txid, genesis().merkle_root);
    let outputs = x3_btc_spv::transaction_outputs(&raw).expect("reads");
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].value, 50 * 100_000_000);
    assert_eq!(
        outputs[0].script_pubkey.len(),
        67,
        "pay-to-pubkey: push 65, key, OP_CHECKSIG"
    );
    assert_eq!(outputs[0].script_pubkey[66], 0xac);
    for cut in [0, 4, 41, raw.len() - 1] {
        assert!(
            x3_btc_spv::transaction_outputs(&raw[..cut]).is_err(),
            "truncated at {cut}"
        );
    }
    let mut trailing = raw.clone();
    trailing.push(0);
    assert_eq!(
        x3_btc_spv::transaction_outputs(&trailing),
        Err(x3_btc_spv::TxError::TrailingBytes)
    );
    // Any other position in a one-transaction block is out of range.
    assert_eq!(
        verify_transaction(&raw, 1, &[], &genesis().merkle_root),
        Err(SpvError::MerkleIndexOutOfRange { index: 1, depth: 0 })
    );
}

#[test]
fn block_170s_two_transactions_prove_against_its_root_only_in_their_own_positions() {
    // Block 170: the coinbase and the first person-to-person payment.
    let root = h("7dac2c5666815c17a3b36427de37bb9d2e2c5ccec3f8633eb91a4205cb4c10ff");
    let coinbase = h("b1fea52486ce0c62bb442b530a3f0132b826c74e473d1f2c220bfa78111c5082");
    let payment = h("f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16");

    assert_eq!(
        verify_merkle_inclusion(&coinbase, 0, &[payment], &root),
        Ok(())
    );
    assert_eq!(
        verify_merkle_inclusion(&payment, 1, &[coinbase], &root),
        Ok(())
    );
    // Position decides the side; the swapped claim fails.
    assert_eq!(
        verify_merkle_inclusion(&payment, 0, &[coinbase], &root),
        Err(SpvError::MerkleRootMismatch)
    );
}

#[test]
fn a_duplicated_subtree_is_refused_and_an_odd_levels_padding_is_not() {
    let a = sha256d(b"a");
    let b = sha256d(b"b");
    let c = sha256d(b"c");
    // Three transactions: level one is [ab, cc], so c's sibling is itself, as the left child.
    let ab = merkle_root_from_path(&a, 0, &[b]).unwrap();
    let cc = merkle_root_from_path(&c, 0, &[c]).unwrap();
    let root = merkle_root_from_path(&ab, 0, &[cc]).unwrap();
    assert_eq!(verify_merkle_inclusion(&c, 2, &[c, ab], &root), Ok(()));
    // CVE-2012-2459: claim a fourth transaction, a copy of c on the right.
    assert_eq!(
        verify_merkle_inclusion(&c, 3, &[c, ab], &root),
        Err(SpvError::MerkleDuplicateSubtree)
    );
}

#[test]
fn a_64_byte_transaction_is_refused_because_it_is_an_inner_node() {
    let a = sha256d(b"a");
    let b = sha256d(b"b");
    let mut node = [0u8; 64];
    node[..32].copy_from_slice(&a);
    node[32..].copy_from_slice(&b);
    let root = sha256d(&node);
    // Hashing the inner node as if it were a transaction reaches the root with an empty path.
    assert_eq!(
        verify_transaction(&node, 0, &[], &root),
        Err(SpvError::AmbiguousTransactionLength)
    );
}

#[test]
fn targets_decode_as_bitcoin_core_decodes_them() {
    assert_eq!(
        target_from_bits(0x1d00ffff),
        Ok(U256::from_u64(0xffff).shl(208))
    );
    assert_eq!(target_from_bits(0x03123456), Ok(U256::from_u64(0x123456)));
    assert_eq!(target_from_bits(0x02123456), Ok(U256::from_u64(0x1234)));
    // Negative, zero and overflowing targets are not targets.
    for bits in [
        0x04923456, 0x1d000000, 0x01003456, 0x23000001, 0x22000100, 0x21010000,
    ] {
        assert_eq!(
            target_from_bits(bits),
            Err(SpvError::InvalidTarget(bits)),
            "{bits:#x}"
        );
    }
    for bits in [0x1d00ffff, 0x1b0404cb, 0x1c05a3f4, 0x207fffff, 0x03123456] {
        assert_eq!(bits_from_target(&target_from_bits(bits).unwrap()), bits);
    }
}

#[test]
fn an_easy_target_is_refused_under_mainnets_limit() {
    let mut forged = BlockHeader {
        bits: 0x207fffff,
        ..block1()
    };
    while check_proof_of_work(&forged, &Params::regtest()).is_err() {
        forged.nonce += 1;
    }
    assert_eq!(
        check_proof_of_work(&forged, &Params::mainnet()),
        Err(SpvError::TargetAbovePowLimit(0x207fffff))
    );
}

#[test]
fn retargets_match_bitcoin_core() {
    let mainnet = Params::mainnet();
    // pow_tests.cpp get_next_work: block 32255 from window start 30240.
    assert_eq!(
        next_work_required(&mainnet, 0x1d00ffff, 1262152739, 1261130161),
        0x1d00d86a
    );
    // get_next_work_pow_limit: a slow first window cannot ease past the limit.
    assert_eq!(
        next_work_required(&mainnet, 0x1d00ffff, 1233061996, 1231006505),
        0x1d00ffff
    );
    // get_next_work_lower_limit_actual: a fast window is clamped to a 4x increase.
    assert_eq!(
        next_work_required(&mainnet, 0x1c05a3f4, 1279297671, 1279008237),
        0x1c0168fd
    );
    // get_next_work_upper_limit_actual: a slow window is clamped to a 4x decrease.
    assert_eq!(
        next_work_required(&mainnet, 0x1c387f6f, 1269211443, 1263163443),
        0x1d00e1fd
    );
}

#[test]
fn work_is_two_to_the_256_over_target_plus_one() {
    // Difficulty 1 is 2^32 + 2^16 + 1 … expected hashes: 0x100010001.
    assert_eq!(
        work_from_bits(0x1d00ffff),
        Ok(U256::from_u64(0x1_0001_0001))
    );
    assert_eq!(work_from_bits(0x207fffff), Ok(U256::from_u64(2)));
}

#[test]
fn median_time_past_is_the_middle_of_the_last_eleven() {
    assert_eq!(median_time_past(&[]), None);
    assert_eq!(median_time_past(&[5, 1, 3]), Some(3));
    let twelve: Vec<u32> = (1..=12).rev().collect();
    // Only the last eleven count: 11..=1 → median 6.
    assert_eq!(median_time_past(&twelve), Some(6));
}

#[test]
fn the_client_follows_mainnet_from_genesis_and_counts_confirmations() {
    let mut client = mainnet_client();
    let now = 1231469744;
    let one = client.submit(block1(), now).expect("block 1");
    assert_eq!((one.height, one.new_tip), (1, true));
    let two = client.submit(block2(), now).expect("block 2");
    assert_eq!(client.tip(), (two.hash, 2));
    assert_eq!(client.tip_work(), U256::from_u64(3 * 0x1_0001_0001));

    let coinbase1 = block1().merkle_root;
    let proof = client
        .verify_inclusion(&coinbase1, 0, &[], &one.hash, 2)
        .expect("buried under one block");
    assert_eq!((proof.height, proof.confirmations), (1, 2));
    assert_eq!(
        client.verify_inclusion(&coinbase1, 0, &[], &one.hash, 3),
        Err(SpvError::InsufficientConfirmations {
            required: 3,
            actual: 2
        })
    );
    assert_eq!(
        client.verify_inclusion(&coinbase1, 0, &[], &two.hash, 1),
        Err(SpvError::MerkleRootMismatch),
        "block 1's coinbase is not in block 2"
    );
    assert_eq!(
        client.submit(block2(), now),
        Err(SpvError::DuplicateHeader(two.hash))
    );
}

#[test]
fn the_client_refuses_forged_mainnet_headers() {
    let mut client = mainnet_client();
    let now = 1231469744;

    // An orphan.
    assert_eq!(
        client.submit(block2(), now),
        Err(SpvError::UnknownParent(block1().block_hash()))
    );
    // An easy difficulty mined on top of genesis.
    let mut easy = BlockHeader {
        bits: 0x207fffff,
        ..block1()
    };
    while check_proof_of_work(&easy, &Params::regtest()).is_err() {
        easy.nonce += 1;
    }
    assert_eq!(
        client.submit(easy, now),
        Err(SpvError::UnexpectedDifficulty {
            height: 1,
            expected: 0x1d00ffff,
            actual: 0x207fffff
        })
    );
    // A real header is refused by a verifier whose clock is more than two hours behind it.
    assert_eq!(
        client.submit(block1(), 1231006505),
        Err(SpvError::TimestampTooFarInFuture {
            time: 1231469665,
            now: 1231006505
        })
    );
    // A checkpoint off a retarget boundary.
    assert_eq!(
        LightClient::new(
            Params::mainnet(),
            Checkpoint {
                height: 1,
                header: block1(),
                prior_timestamps: vec![],
            },
        )
        .err(),
        Some(SpvError::CheckpointNotAtRetargetBoundary(1))
    );
}

// ---- Synthetic chains: forks, median time and retargets, mined at regtest difficulty ----

fn mine(mut header: BlockHeader, params: &Params) -> BlockHeader {
    while check_proof_of_work(&header, params).is_err() {
        header.nonce = header.nonce.wrapping_add(1);
    }
    header
}

fn child(parent: &BlockHeader, time: u32, bits: u32, tag: u8, params: &Params) -> BlockHeader {
    mine(
        BlockHeader {
            version: 4,
            prev_blockhash: parent.block_hash(),
            merkle_root: sha256d(&[tag]),
            time,
            bits,
            nonce: 0,
        },
        params,
    )
}

fn regtest_root(params: &Params) -> BlockHeader {
    mine(
        BlockHeader {
            version: 4,
            prev_blockhash: [7; 32],
            merkle_root: sha256d(b"root"),
            time: 1_700_000_000,
            bits: 0x207fffff,
            nonce: 0,
        },
        params,
    )
}

#[test]
fn a_heavier_branch_takes_the_tip_and_the_lighter_ones_blocks_stop_counting() {
    let params = Params::regtest();
    let root = regtest_root(&params);
    let mut client = LightClient::new(
        params,
        Checkpoint {
            height: 0,
            header: root,
            prior_timestamps: vec![],
        },
    )
    .unwrap();
    let now = u32::MAX - 7200;
    let a1 = child(&root, 1_700_000_600, 0x207fffff, 1, &params);
    let b1 = child(&root, 1_700_000_600, 0x207fffff, 2, &params);
    let b2 = child(&b1, 1_700_001_200, 0x207fffff, 3, &params);
    assert!(client.submit(a1, now).unwrap().new_tip);
    assert!(
        !client.submit(b1, now).unwrap().new_tip,
        "equal work keeps the first-seen tip"
    );
    assert_eq!(client.confirmations(&a1.block_hash()), Ok(1));
    assert!(client.submit(b2, now).unwrap().new_tip);
    assert_eq!(
        client.confirmations(&a1.block_hash()),
        Err(SpvError::NotInBestChain(a1.block_hash()))
    );
    assert_eq!(client.confirmations(&b1.block_hash()), Ok(2));
    assert_eq!(client.confirmations(&root.block_hash()), Ok(3));
}

#[test]
fn a_timestamp_at_or_before_the_median_of_eleven_is_refused() {
    let params = Params::regtest();
    let root = regtest_root(&params);
    // Ten prior blocks a minute apart before the checkpoint.
    let prior: Vec<u32> = (0..10).map(|i| root.time - 600 * (10 - i)).collect();
    let mut client = LightClient::new(
        params,
        Checkpoint {
            height: 0,
            header: root,
            prior_timestamps: prior.clone(),
        },
    )
    .unwrap();
    let mut eleven = prior;
    eleven.push(root.time);
    let median = median_time_past(&eleven).unwrap();
    let now = u32::MAX - 7200;
    let stale = child(&root, median, 0x207fffff, 1, &params);
    assert_eq!(
        client.submit(stale, now),
        Err(SpvError::TimestampNotAfterMedian {
            time: median,
            median
        })
    );
    // One second later is Bitcoin-valid even though it predates its parent.
    let ok = child(&root, median + 1, 0x207fffff, 1, &params);
    assert!(ok.time < root.time);
    assert!(client.submit(ok, now).is_ok());
}

#[test]
fn the_client_demands_the_exact_retarget_at_each_boundary() {
    // A network with four-block windows meant to take 40 seconds, so a retarget is cheap to reach.
    let params = Params {
        pow_limit: U256::MAX.shr(1),
        retarget_interval: 4,
        target_timespan: 40,
        no_retargeting: false,
    };
    let root = regtest_root(&params);
    let mut client = LightClient::new(
        params,
        Checkpoint {
            height: 0,
            header: root,
            prior_timestamps: vec![],
        },
    )
    .unwrap();
    let now = u32::MAX - 7200;
    // Three blocks one second apart: the window (root..=h3) took 3 seconds, far faster than 40.
    let mut tip = root;
    for i in 1..=3u8 {
        tip = child(&tip, root.time + u32::from(i), 0x207fffff, i, &params);
        client.submit(tip, now).unwrap();
    }
    let expected = next_work_required(&params, 0x207fffff, tip.time, root.time);
    assert_eq!(
        expected,
        bits_from_target(&target_from_bits(0x207fffff).unwrap().div_u64(4)),
        "a 3-second window is clamped to a 4x harder target"
    );
    // Keeping the old, easier difficulty at the boundary is refused …
    let unchanged = child(&tip, root.time + 4, 0x207fffff, 4, &params);
    assert_eq!(
        client.submit(unchanged, now),
        Err(SpvError::UnexpectedDifficulty {
            height: 4,
            expected,
            actual: 0x207fffff
        })
    );
    // … and the exact retarget is admitted, then carried unchanged inside the window.
    let retargeted = child(&tip, root.time + 4, expected, 4, &params);
    assert_eq!(client.submit(retargeted, now).unwrap().height, 4);
    let next = child(&retargeted, root.time + 5, 0x207fffff, 5, &params);
    assert!(matches!(
        client.submit(next, now),
        Err(SpvError::UnexpectedDifficulty { height: 5, .. })
    ));
    let next = child(&retargeted, root.time + 5, expected, 5, &params);
    assert!(client.submit(next, now).is_ok());
}
