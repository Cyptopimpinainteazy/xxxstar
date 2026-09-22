//! Benchmarks for x3-settlement-engine pallet
//!
//! These benchmarks measure the cost of critical settlement operations:
//! - Intent creation and lifecycle management
//! - Escrow locking and release
//! - BTC SPV verification
//! - Settlement finalization
//!
//! Weights generated from these benchmarks are used in extrinsic dispatch to ensure
//! blocks don't exceed weight limits and to calculate transaction fees accurately.

use super::*;
use frame_benchmarking::benchmarks;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_core::H256;
use sp_std::vec;
use sp_std::vec::Vec;

const SEED: u32 = 0;

fn fund_account<T: Config>(account: &T::AccountId) {
    let _ = <T as pallet::Config>::Currency::make_free_balance_be(account, 10_000_000u32.into());
}

fn setup_intent<T: Config>() -> (T::AccountId, T::AccountId, H256, AssetSpec, AssetSpec) {
    let maker: T::AccountId = frame_benchmarking::account("maker", 0, SEED);
    let taker: T::AccountId = frame_benchmarking::account("taker", 1, SEED);
    fund_account::<T>(&maker);
    fund_account::<T>(&taker);
    let secret_hash = H256::from(sp_io::hashing::sha2_256(
        H256::from_low_u64_be(1).as_bytes(),
    ));
    let asset_a = AssetSpec {
        chain: ExternalChainId::Ethereum,
        token: TokenId::Native,
        amount: 1_000_000u128,
    };
    let asset_b = AssetSpec {
        chain: ExternalChainId::Bitcoin,
        token: TokenId::Native,
        amount: 1_000_000u128,
    };
    (maker, taker, secret_hash, asset_a, asset_b)
}

benchmarks! {
    create_intent {
        let (maker, taker, secret_hash, asset_a, asset_b) = setup_intent::<T>();
        let origin = RawOrigin::Signed(maker.clone());
    }: _(origin, taker, asset_a, asset_b, secret_hash, Some(86400u64))
    verify {
        let nonce = TotalIntents::<T>::get();
        assert!(nonce > 0);
    }

    lock_escrow {
        let (maker, taker, secret_hash, asset_a, asset_b) = setup_intent::<T>();

        // Create intent first
        let create_origin = RawOrigin::Signed(maker.clone()).into();
        Pallet::<T>::create_intent(
            create_origin,
            taker.clone(),
            asset_a.clone(),
            asset_b.clone(),
            secret_hash,
            Some(86400u64),
        ).ok();

        let intent_id = Pallet::<T>::generate_intent_id(&maker, &taker, 0);
        let escrow_data = vec![1u8; 64];

        let origin = RawOrigin::Signed(maker.clone());
    }: _(
        origin,
        intent_id,
        0u32,
        ExternalChainId::Ethereum,
        1_000_000u128,
        escrow_data
    )
    verify {
        assert!(EscrowStates::<T>::contains_key(intent_id, 0u32));
    }

    claim_settlement {
        let (maker, taker, secret_hash, asset_a, asset_b) = setup_intent::<T>();

        // Create intent
        let create_origin = RawOrigin::Signed(maker.clone()).into();
        Pallet::<T>::create_intent(
            create_origin,
            taker.clone(),
            asset_a.clone(),
            asset_b.clone(),
            secret_hash,
            Some(86400u64),
        ).ok();

        let intent_id = Pallet::<T>::generate_intent_id(&maker, &taker, 0);

        // Lock escrow
        let escrow_origin = RawOrigin::Signed(maker.clone()).into();
        Pallet::<T>::lock_escrow(
            escrow_origin,
            intent_id,
            0u32,
            ExternalChainId::Ethereum,
            1_000_000u128,
            vec![1u8; 64],
        ).ok();
        Pallet::<T>::lock_escrow(
            RawOrigin::Signed(taker.clone()).into(),
            intent_id,
            1u32,
            ExternalChainId::Bitcoin,
            1_000_000u128,
            vec![2u8; 64],
        ).ok();

        let secret = H256::from_low_u64_be(1);
        let origin = RawOrigin::Signed(maker.clone());
    }: _(origin, intent_id, secret)
    verify {
        assert!(ClaimedLegs::<T>::get(intent_id, 0u32));
    }

    refund_settlement {
        let (maker, taker, secret_hash, asset_a, asset_b) = setup_intent::<T>();

        // Create intent with very short timeout
        let create_origin = RawOrigin::Signed(maker.clone()).into();
        Pallet::<T>::create_intent(
            create_origin,
            taker.clone(),
            asset_a.clone(),
            asset_b.clone(),
            secret_hash,
            Some(0u64),
        ).ok();

        let intent_id = Pallet::<T>::generate_intent_id(&maker, &taker, 0);

        // Lock escrow
        let escrow_origin = RawOrigin::Signed(maker.clone()).into();
        Pallet::<T>::lock_escrow(
            escrow_origin,
            intent_id,
            0u32,
            ExternalChainId::Ethereum,
            1_000_000u128,
            vec![1u8; 64],
        ).ok();

        let origin = RawOrigin::Signed(maker.clone());
    }: _(origin, intent_id)
    verify {
        let state = IntentStates::<T>::get(intent_id);
        assert!(matches!(state, IntentState::Refunded));
    }

    submit_btc_proof {
        let (maker, taker, secret_hash, asset_a, asset_b) = setup_intent::<T>();

        // Create intent
        let create_origin = RawOrigin::Signed(maker.clone()).into();
        Pallet::<T>::create_intent(
            create_origin,
            taker.clone(),
            asset_a.clone(),
            asset_b.clone(),
            secret_hash,
            Some(86400u64),
        ).ok();

        let intent_id = Pallet::<T>::generate_intent_id(&maker, &taker, 0);
        let btc_txid = H256::from_low_u64_be(2);
        let merkle_proof: Vec<H256> = vec![];

        // Setup writes the state the pallet's own admission path would have written.
        //
        // It has to: admitting a header requires proof of work under the network's
        // `powLimit`, and a benchmark run cannot pay mainnet's ~2^32 double-SHA256
        // per header. Measuring `submit_btc_proof` is still meaningful — what is
        // being weighed is the merkle walk and the UTXO write, and the header has to
        // be admitted by `submit_btc_header` *before* a proof naming it can be
        // submitted at all. What is not being measured is header admission.
        let block_header = BtcBlockHeader {
            version: 1,
            prev_block_hash: H256::from_low_u64_be(0),
            merkle_root: btc_txid,
            timestamp: 1234567890u32,
            bits: 0x207fffff,
            nonce: 0,
            height: 0u64,
        };
        let block_hash = Pallet::<T>::compute_btc_block_hash(&block_header);
        BtcCheckpoints::<T>::insert(0u64, block_hash);
        BtcHeaders::<T>::insert(block_hash, block_header.clone());
        BtcHeaderMetaStore::<T>::insert(
            block_hash,
            crate::types::BtcHeaderMeta {
                height: 0,
                anchored: true,
            },
        );
        BtcBestHeight::<T>::put(0u64);

        let origin = RawOrigin::Signed(maker.clone());
    }: _(origin, intent_id, btc_txid, 0u32, 0u32, 0u64, merkle_proof, block_header)
    verify {
        // Just verify the extrinsic succeeds; actual BTC proof validation
        // depends on external chain state
    }

    // `submit_btc_header` and `anchor_btc_checkpoint` are deliberately not
    // benchmarked. Both verify proof of work under the network's `powLimit`, so a
    // benchmark body cannot produce an accepted input without either paying
    // mainnet's work per header or being a build where the check does not run. The
    // previous `submit_btc_header` bench only "passed" because the pallet then
    // accepted a header whose `nBits` the caller had chosen — that is the defect
    // this change closes, and a bench that needs it back is not worth keeping.
    // Their weights are the fixed constants in `weights.rs`, whose hashing term is
    // the same 3-processor term as `submit_external_proof`.

    submit_proof {
        let (maker, taker, secret_hash, asset_a, asset_b) = setup_intent::<T>();

        // Create intent
        let create_origin = RawOrigin::Signed(maker.clone()).into();
        Pallet::<T>::create_intent(
            create_origin,
            taker.clone(),
            asset_a.clone(),
            asset_b.clone(),
            secret_hash,
            Some(86400u64),
        ).ok();

        let intent_id = Pallet::<T>::generate_intent_id(&maker, &taker, 0);
        Pallet::<T>::lock_escrow(
            RawOrigin::Signed(maker.clone()).into(),
            intent_id,
            0u32,
            ExternalChainId::Ethereum,
            1_000_000u128,
            vec![1u8; 64],
        ).ok();
        Pallet::<T>::lock_escrow(
            RawOrigin::Signed(taker.clone()).into(),
            intent_id,
            1u32,
            ExternalChainId::Bitcoin,
            1_000_000u128,
            vec![2u8; 64],
        ).ok();
        let receipt_data = vec![0xc3, 0x80, 0x80, 0x80];
        let proof = SettlementProof {
            proof_type: ProofType::MerkleTrie,
            tx_hash: H256::from(sp_io::hashing::keccak_256(&receipt_data)),
            block_hash: H256::from_low_u64_be(3),
            // The height the proof is about. Stated rather than derived from
            // `tx_hash`, which is what the benchmark's proof used to imply
            // (TICKET-061).
            chain_height: Some(18_000_000),
            confirmations: 12u32,
            // Two entries: the state root and the receipt root, which is what the
            // EVM path verifies against (see `proof_roots`).
            merkle_proof: vec![H256::zero(), H256::zero()]
                .try_into()
                .expect("two-item proof is within the configured maximum"),
            receipt_data: receipt_data
                .try_into()
                .expect("four-byte receipt is within the configured maximum"),
            // The EVM path reads neither: `receipt_index` is the BTC/SPV position
            // and `trie_proof` is the Merkle-Patricia path, which this benchmark's
            // proof deliberately does not carry (it exercises the shape check, and
            // the proof is refused for exactly that reason).
            receipt_index: None,
            trie_proof: None,
        };

        let origin = RawOrigin::Signed(maker.clone());
    }: _(origin, intent_id, ExternalChainId::Ethereum, proof)
    verify {
        // Verify submission succeeds
    }

    deposit_bond {
        let depositor: T::AccountId = frame_benchmarking::account("depositor", 0, SEED);
        let amount = <<T as pallet::Config>::Currency as Currency<T::AccountId>>::minimum_balance() * 100u32.into();
        fund_account::<T>(&depositor);

        let origin = RawOrigin::Signed(depositor.clone());
    }: _(origin, vec![1u8; 32], amount, 0u8)
    verify {
        let bond_count = BondCounter::<T>::get();
        assert!(bond_count > 0);
    }

    finalize_bond_withdraw {
        let depositor: T::AccountId = frame_benchmarking::account("depositor", 0, SEED);
        let amount = <<T as pallet::Config>::Currency as Currency<T::AccountId>>::minimum_balance() * 100u32.into();
        fund_account::<T>(&depositor);

        // Create bond first
        let create_origin = RawOrigin::Signed(depositor.clone()).into();
        Pallet::<T>::deposit_bond(
            create_origin,
            vec![1u8; 32],
            amount,
            0u8,
        ).ok();

        let bond_id = {
            let mut bytes = [0u8; 32];
            bytes[..8].copy_from_slice(&BondCounter::<T>::get().to_le_bytes());
            H256::from(bytes)
        };
        Pallet::<T>::request_bond_withdraw(
            RawOrigin::Signed(depositor.clone()).into(),
            bond_id,
        ).ok();

        let origin = RawOrigin::Signed(depositor.clone());
    }: _(origin, bond_id)
    verify {
        // Verify claim succeeds
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
