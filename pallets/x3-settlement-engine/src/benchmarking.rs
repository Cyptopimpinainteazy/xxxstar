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
use frame_benchmarking::{benchmarks, BenchmarkError};
use frame_support::traits::{Currency, ReservableCurrency};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::SaturatedConversion;
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

/// A canonical Refund proof set covering every escrowed leg of `intent_id`, one bundle per domain.
///
/// Returns a `Result` rather than panicking: the production panic/unwrap ratchet counts a
/// `#[cfg(feature = "runtime-benchmarks")]` module, and two `expect` calls here grew it 440 → 442 and
/// reddened `make mainnet-check` on the merge that brought them in.
fn refund_proof_set<T: Config>(
    intent_id: H256,
) -> Result<x3_atomic_swap::CrossDomainProofSet, BenchmarkError> {
    use x3_atomic_swap::{
        CrossDomainOperation, CrossDomainProofBundle, CrossDomainProofSet, FinalityProof, VmType,
    };
    let runtime_intent_id = intent_id.to_fixed_bytes();
    let intent = SettlementIntents::<T>::get(intent_id).ok_or(BenchmarkError::Weightless)?;
    let mut bundles: Vec<CrossDomainProofBundle> = Vec::new();
    for leg_idx in 0..intent.legs_total {
        let Some(escrow) = EscrowStates::<T>::get(intent_id, leg_idx) else {
            continue;
        };
        let (chain_id, vm_type, block_number): (&str, VmType, u64) = match escrow.chain {
            ExternalChainId::X3Native => ("x3-native", VmType::X3Vm, 1),
            ExternalChainId::Solana => ("solana-mainnet", VmType::Svm, 2),
            ExternalChainId::Bitcoin => ("bitcoin-mainnet", VmType::BitcoinScript, 3),
            _ => ("ethereum-mainnet", VmType::Evm, 4),
        };
        if bundles
            .iter()
            .any(|b| b.chain_id == chain_id && b.vm_type == vm_type)
        {
            continue;
        }
        let mut bundle = CrossDomainProofBundle {
            version: CrossDomainProofBundle::VERSION,
            intent_id: 1,
            runtime_intent_id,
            intent_hash: [0x11u8; 32],
            chain_id: chain_id.into(),
            vm_type,
            operation: CrossDomainOperation::Refund,
            tx_id: "0xbenchrefund".into(),
            block_number,
            block_hash: "0xbenchrefundblock".into(),
            execution_evidence: vec![1, 2, 3],
            finality: FinalityProof {
                chain_id: chain_id.into(),
                vm_type,
                tx_id: "0xbenchrefund".into(),
                block_number,
                block_hash: "0xbenchrefundblock".into(),
                confirmations: 12,
                finalized: true,
                finality_source: "benchmark".into(),
                safe_to_reveal_secret: true,
            },
            proof_hash: [0u8; 32],
        };
        bundle.proof_hash = bundle
            .compute_hash()
            .map_err(|_| BenchmarkError::Weightless)?;
        bundles.push(bundle);
    }
    Ok(CrossDomainProofSet {
        intent_id: 1,
        runtime_intent_id,
        intent_hash: [0x11u8; 32],
        bundles,
    })
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

    // The worst case: a native X3 leg, which reserves the depositor's funds as well as recording
    // the escrow (an external leg only records it).
    lock_escrow {
        let (maker, taker, secret_hash, _, asset_b) = setup_intent::<T>();
        let native_amount = 1_000_000u128;
        let asset_a = AssetSpec {
            chain: ExternalChainId::X3Native,
            token: TokenId::Native,
            amount: native_amount,
        };
        Pallet::<T>::create_intent(
            RawOrigin::Signed(maker.clone()).into(),
            taker.clone(),
            asset_a,
            asset_b,
            secret_hash,
            Some(86400u64),
        )?;
        let intent_id = Pallet::<T>::generate_intent_id(&maker, &taker, 0);
        let escrow_data = vec![1u8; 64];
        let held = <T as pallet::Config>::Currency::reserved_balance(&maker);

        let origin = RawOrigin::Signed(maker.clone());
    }: _(
        origin,
        intent_id,
        0u32,
        ExternalChainId::X3Native,
        native_amount,
        escrow_data
    )
    verify {
        assert!(EscrowStates::<T>::contains_key(intent_id, 0u32));
        let native: <<T as pallet::Config>::Currency as Currency<T::AccountId>>::Balance =
            native_amount.saturated_into();
        assert_eq!(
            <T as pallet::Config>::Currency::reserved_balance(&maker),
            held + native,
            "the native leg is held"
        );
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

    // The worst case: a native X3 leg this pallet holds (returned to its depositor) beside an
    // external leg, with the canonical Refund proof set a terminal refund requires. This bench
    // used to lock one external leg and present no proof set, so the call it measured was the
    // `CrossDomainProofSetIncomplete` refusal and the benchmark could not run.
    refund_settlement {
        let (maker, taker, secret_hash, _, asset_b) = setup_intent::<T>();
        let native_amount = 1_000_000u128;
        let asset_a = AssetSpec {
            chain: ExternalChainId::X3Native,
            token: TokenId::Native,
            amount: native_amount,
        };

        // Timeout 0: the intent is refundable as soon as it exists.
        Pallet::<T>::create_intent(
            RawOrigin::Signed(maker.clone()).into(),
            taker.clone(),
            asset_a,
            asset_b.clone(),
            secret_hash,
            Some(0u64),
        )?;
        let intent_id = Pallet::<T>::generate_intent_id(&maker, &taker, 0);
        Pallet::<T>::lock_escrow(
            RawOrigin::Signed(maker.clone()).into(),
            intent_id,
            0u32,
            ExternalChainId::X3Native,
            native_amount,
            vec![1u8; 32],
        )?;
        Pallet::<T>::lock_escrow(
            RawOrigin::Signed(taker.clone()).into(),
            intent_id,
            1u32,
            asset_b.chain,
            asset_b.amount,
            vec![2u8; 32],
        )?;
        let held = <T as pallet::Config>::Currency::reserved_balance(&maker);
        Pallet::<T>::submit_cross_domain_proof_set(
            RawOrigin::Signed(maker.clone()).into(),
            intent_id,
            refund_proof_set::<T>(intent_id)?,
        )?;

        let origin = RawOrigin::Signed(maker.clone());
    }: _(origin, intent_id)
    verify {
        let state = IntentStates::<T>::get(intent_id);
        assert!(matches!(state, IntentState::Refunded));
        let native: <<T as pallet::Config>::Currency as Currency<T::AccountId>>::Balance =
            native_amount.saturated_into();
        assert_eq!(
            <T as pallet::Config>::Currency::reserved_balance(&maker),
            held - native,
            "the native leg is returned to its depositor"
        );
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
        // The proof the EVM path accepts: a receipt that walks to its own receipts root. The
        // benchmark used to hand the pallet a two-zero-root proof with no trie path and no receipt
        // index, which the verification added in PR #520 refuses (`InvalidProof`) — and the
        // `benchmarks!` macro asserts the extrinsic succeeds, so that benchmark could never pass.
        let proof = crate::proof_fixtures::create_evm_receipt_proof()
            .map_err(|_| BenchmarkError::Weightless)?;

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

#[cfg(test)]
mod gate_pins {
    use super::*;
    use crate::mock::{new_test_ext, RuntimeOrigin, Test, ALICE, BOB};
    use crate::types::{AssetSpec, TokenId};
    use frame_support::assert_ok;

    #[test]
    fn refund_proof_set_deduplicates_only_identical_domains() {
        new_test_ext().execute_with(|| {
            // Two legs on the same chain and VM: one bundle for the domain.
            assert_ok!(Pallet::<Test>::create_intent(
                RuntimeOrigin::signed(ALICE),
                BOB,
                AssetSpec {
                    chain: ExternalChainId::Ethereum,
                    token: TokenId::Native,
                    amount: 10,
                },
                AssetSpec {
                    chain: ExternalChainId::Ethereum,
                    token: TokenId::Native,
                    amount: 20,
                },
                H256::repeat_byte(0x91),
                Some(3_600),
            ));
            let same = SettlementIntents::<Test>::iter()
                .find(|(_, intent)| intent.secret_hash == H256::repeat_byte(0x91))
                .map(|(id, _)| id)
                .expect("intent exists");
            assert_ok!(Pallet::<Test>::lock_escrow(
                RuntimeOrigin::signed(ALICE),
                same,
                0,
                ExternalChainId::Ethereum,
                10,
                vec![],
            ));
            assert_ok!(Pallet::<Test>::lock_escrow(
                RuntimeOrigin::signed(BOB),
                same,
                1,
                ExternalChainId::Ethereum,
                20,
                vec![],
            ));
            assert_eq!(
                refund_proof_set::<Test>(same)
                    .expect("set builds")
                    .bundles
                    .len(),
                1,
                "the second leg is the same domain and must be skipped"
            );

            // Two distinct domains: one bundle each.
            assert_ok!(Pallet::<Test>::create_intent(
                RuntimeOrigin::signed(ALICE),
                BOB,
                AssetSpec {
                    chain: ExternalChainId::Ethereum,
                    token: TokenId::Native,
                    amount: 30,
                },
                AssetSpec {
                    chain: ExternalChainId::Solana,
                    token: TokenId::Native,
                    amount: 40,
                },
                H256::repeat_byte(0x92),
                Some(3_600),
            ));
            let distinct = SettlementIntents::<Test>::iter()
                .find(|(_, intent)| intent.secret_hash == H256::repeat_byte(0x92))
                .map(|(id, _)| id)
                .expect("intent exists");
            assert_ok!(Pallet::<Test>::lock_escrow(
                RuntimeOrigin::signed(ALICE),
                distinct,
                0,
                ExternalChainId::Ethereum,
                30,
                vec![],
            ));
            assert_ok!(Pallet::<Test>::lock_escrow(
                RuntimeOrigin::signed(BOB),
                distinct,
                1,
                ExternalChainId::Solana,
                40,
                vec![],
            ));
            assert_eq!(
                refund_proof_set::<Test>(distinct)
                    .expect("set builds")
                    .bundles
                    .len(),
                2,
                "distinct domains each get a bundle"
            );
        });
    }
}
