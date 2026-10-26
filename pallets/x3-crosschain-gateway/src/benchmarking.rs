//! Benchmarking setup for `pallet-x3-crosschain-gateway`.
//!
//! The original dispatchables charged `Weight::from_parts(20_000..60_000, 0)` with
//! no read or write count, behind a `runtime-benchmarks` feature that pulled the
//! two frame passthroughs and nothing else.
//!
//! Every benchmark establishes its state through the pallet's own extrinsics —
//! `register_asset` then `enable_route` for the asset/route pair,
//! `submit_deposit_proof` for a verified transfer, `request_withdrawal` then
//! `burn_x3_representation` for a burned withdrawal — so the proof path being
//! measured is the real one. The route is `X3Internal` for exactly that reason:
//! it is the level whose verifier this repository can actually satisfy, and the
//! mock's suite proves the same route accepts the same proof shape.
//!
//! Governance, relayer and operational origins all come from
//! `try_successful_origin()`, which is the only form correct for both runtimes:
//! this one gates governance on a council majority and the other two on root or
//! half the council, while the mock uses root and any signed account.

use super::*;
use codec::Encode;
use frame_benchmarking::v2::*;
use frame_support::traits::{ConstU32, EnsureOrigin};
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use sp_core::{ed25519, Pair};
use sp_runtime::traits::SaturatedConversion;
// A benchmark module also compiles into the runtime's `wasm32v1-none` build,
// where `vec!` is not in the prelude.
use sp_std::vec;
use x3_verification_router::{ChainKind, ProofEnvelope, VerificationStrategy};

#[benchmarks]
mod benchmarks {
    use super::*;

    const SOLANA_CHAIN: ExternalChainId = ExternalChainId::SolanaDevnet;
    const CHAIN: ExternalChainId = SOLANA_CHAIN;
    const ROUTE_ID: RouteId = [1u8; 32];
    const X3_ASSET: X3AssetId = [9u8; 32];
    const PROOF_ID: ProofId = [1u8; 32];
    const DEPOSIT_AMOUNT: Balance = 100;
    const WITHDRAW_AMOUNT: Balance = 100;

    fn gov<T: Config>() -> Result<T::RuntimeOrigin, BenchmarkError> {
        T::GovernanceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)
    }

    fn relayer<T: Config>() -> Result<T::RuntimeOrigin, BenchmarkError> {
        T::RelayerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)
    }

    fn operational<T: Config>() -> Result<T::RuntimeOrigin, BenchmarkError> {
        T::OperationalOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)
    }

    fn token() -> Result<BoundedVec<u8, ConstU32<128>>, BenchmarkError> {
        BoundedVec::try_from(b"0xTOKEN".to_vec()).map_err(|_| BenchmarkError::Weightless)
    }

    fn solana_token() -> Result<BoundedVec<u8, ConstU32<128>>, BenchmarkError> {
        BoundedVec::try_from(b"0xSOLTOKEN".to_vec()).map_err(|_| BenchmarkError::Weightless)
    }

    fn recipient() -> Result<BoundedVec<u8, ConstU32<128>>, BenchmarkError> {
        BoundedVec::try_from([0xCC; 32].to_vec()).map_err(|_| BenchmarkError::Weightless)
    }

    fn solana_stakes() -> Result<BoundedVec<SolanaStakeEntry, MaxSvmValidators>, BenchmarkError> {
        let mut stakes = vec![
            SolanaStakeEntry {
                pubkey: ed25519::Pair::from_seed(&[1u8; 32]).public().0,
                stake: 100,
            },
            SolanaStakeEntry {
                pubkey: ed25519::Pair::from_seed(&[2u8; 32]).public().0,
                stake: 100,
            },
        ];
        stakes.sort_by_key(|entry| entry.pubkey);
        BoundedVec::try_from(stakes).map_err(|_| BenchmarkError::Weightless)
    }

    fn external_asset() -> Result<ExternalAssetRef, BenchmarkError> {
        Ok(ExternalAssetRef {
            chain_id: SOLANA_CHAIN,
            token_address_or_mint: solana_token()?,
        })
    }

    fn route() -> Result<RouteConfig, BenchmarkError> {
        Ok(RouteConfig {
            route_id: ROUTE_ID,
            external_chain_id: SOLANA_CHAIN,
            external_asset: external_asset()?,
            x3_asset_id: X3_ASSET,
            destination_domain: X3Domain::Native,
            enabled: false,
            min_amount: 1,
            max_amount: 100_000,
            daily_limit: 10_000,
            daily_deposited: 0,
            daily_reset_at_block: 0,
            pending_limit: 10,
            finality_requirement: 12,
            // The one level whose verifier accepts a proof this repository can build.
            verification_level: RouteVerificationLevel::SolanaFinalizedProof,
            fee_bps: 10,
            mode: GatewayMode::TestnetLive,
            require_dispute_window: false,
            dispute_window_blocks: 0,
            contract_address: solana_token()?,
        })
    }

    fn deposit_proof(nonce: u64) -> Result<DepositProof, BenchmarkError> {
        Ok(DepositProof {
            version: 1,
            proof_id: PROOF_ID,
            source_chain: SOLANA_CHAIN,
            source_block: 100,
            source_tx_hash: [7u8; 32],
            event_index: 0,
            external_asset: external_asset()?,
            sender: token()?,
            recipient: recipient()?,
            amount: DEPOSIT_AMOUNT,
            nonce,
            observed_at_block: 110,
            finalized_at_block: 120,
            proof_payload: BoundedVec::default(),
        })
    }

    fn solana_finality_payload(
        envelope: &ProofEnvelope,
    ) -> Result<BoundedVec<u8, ConstU32<4096>>, BenchmarkError> {
        use x3_verification_router::{
            encode_solana_finality_v4, solana_finality_message_v4, solana_stake_table_digest,
            SolanaCluster, SolanaTransactionEvidence, SolanaValidatorStake,
        };

        let mut pairs = vec![
            ed25519::Pair::from_seed(&[1u8; 32]),
            ed25519::Pair::from_seed(&[2u8; 32]),
        ];
        pairs.sort_by_key(|pair| pair.public().0);
        let stakes = pairs
            .iter()
            .map(|pair| SolanaValidatorStake {
                pubkey: pair.public().0,
                stake: 100,
            })
            .collect::<Vec<_>>();
        let digest = solana_stake_table_digest(&stakes);
        let transaction_key = &pairs[0];
        let recent_blockhash = [5u8; 32];
        let mut transaction_message = vec![1, 0, 0, 1];
        transaction_message.extend_from_slice(&transaction_key.public().0);
        transaction_message.extend_from_slice(&recent_blockhash);
        transaction_message.push(0);
        let mut raw_transaction = vec![1];
        raw_transaction.extend_from_slice(&transaction_key.sign(&transaction_message).0);
        raw_transaction.extend_from_slice(&transaction_message);
        let transaction = SolanaTransactionEvidence::from_raw(42, 1, true, raw_transaction)
            .map_err(|_| BenchmarkError::Weightless)?;
        let message = solana_finality_message_v4(
            SolanaCluster::Devnet,
            42,
            41,
            &[6u8; 32],
            &[8u8; 32],
            &[7u8; 32],
            &digest,
            &transaction,
            envelope,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        let votes = pairs
            .iter()
            .map(|pair| (pair.public().0, pair.sign(&message).0))
            .collect::<Vec<_>>();
        let payload = encode_solana_finality_v4(
            SolanaCluster::Devnet,
            42,
            41,
            [6u8; 32],
            [8u8; 32],
            [7u8; 32],
            digest,
            transaction,
            &votes,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        BoundedVec::try_from(payload).map_err(|_| BenchmarkError::Weightless)
    }

    fn solana_deposit_proof<T: Config>(nonce: u64) -> Result<DepositProof, BenchmarkError> {
        let route = route()?;
        let mut proof = deposit_proof(nonce)?;
        let envelope = Pallet::<T>::envelope_from_deposit(&proof, &route);
        proof.proof_payload = solana_finality_payload(&envelope)?;
        Ok(proof)
    }

    fn solana_release_payload(
        withdrawal_id: WithdrawalId,
        recipient: &[u8],
    ) -> Result<BoundedVec<u8, ConstU32<4096>>, BenchmarkError> {
        let route = route()?;
        let envelope = ProofEnvelope {
            proof_id: withdrawal_id,
            strategy: VerificationStrategy::SolanaFinalizedProof,
            source_chain: ChainKind::Solana,
            destination_chain: ChainKind::X3,
            payload: Vec::new(),
            expected_asset_id: X3_ASSET,
            expected_amount: WITHDRAW_AMOUNT,
            expected_sender: recipient.to_vec(),
            expected_recipient: route.contract_address.to_vec(),
            expected_beneficiary: Vec::new(),
        };
        solana_finality_payload(&envelope)
    }

    /// Register the asset and open the route, through the pallet's own extrinsics.
    fn setup_route<T: Config>() -> Result<(), BenchmarkError> {
        Pallet::<T>::register_asset(gov::<T>()?, SOLANA_CHAIN, solana_token()?, X3_ASSET)
            .map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::enable_route(gov::<T>()?, route()?).map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::set_solana_stake_committee(gov::<T>()?, SOLANA_CHAIN, solana_stakes()?, 2, 3)
            .map_err(|_| BenchmarkError::Weightless)
    }

    /// A verified transfer, which `credit_x3_representation` requires.
    fn setup_verified_transfer<T: Config>() -> Result<(), BenchmarkError> {
        setup_route::<T>()?;
        T::DepositMinter::prepare_benchmark(
            SOLANA_CHAIN
                .wrapped_chain_id()
                .ok_or(BenchmarkError::Weightless)?,
            X3_ASSET,
            &recipient()?,
            0,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::submit_deposit_proof(relayer::<T>()?, ROUTE_ID, solana_deposit_proof::<T>(1)?)
            .map_err(|_| BenchmarkError::Weightless)
    }

    /// A withdrawal record whose burn has already happened, which both the
    /// release path and `finalize_external_release` require.
    ///
    /// The deposit comes first on purpose: `burn_x3_representation` re-checks
    /// `ExternalLocked >= PendingWithdrawals`, so a withdrawal burned with no
    /// deposit behind it is a collateral-invariant violation, not a benchmark.
    fn setup_burned_withdrawal<T: Config>() -> Result<WithdrawalId, BenchmarkError> {
        setup_verified_transfer::<T>()?;
        let who: T::AccountId = whitelisted_caller();
        let recipient = recipient()?;
        Pallet::<T>::request_withdrawal(
            RawOrigin::Signed(who.clone()).into(),
            X3_ASSET,
            CHAIN,
            recipient.clone(),
            WITHDRAW_AMOUNT,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        let now: u64 = frame_system::Pallet::<T>::block_number().saturated_into();
        let id = Pallet::<T>::derive_withdrawal_id(X3_ASSET, &recipient[..], WITHDRAW_AMOUNT, now);
        T::DepositMinter::prepare_benchmark(
            CHAIN.wrapped_chain_id().ok_or(BenchmarkError::Weightless)?,
            X3_ASSET,
            &who.encode(),
            WITHDRAW_AMOUNT,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::burn_x3_representation(RawOrigin::Signed(who).into(), id)
            .map_err(|_| BenchmarkError::Weightless)?;
        Ok(id)
    }

    #[benchmark]
    fn register_asset() -> Result<(), BenchmarkError> {
        #[extrinsic_call]
        register_asset(gov::<T>()?, CHAIN, token()?, X3_ASSET);

        assert!(Assets::<T>::contains_key(CHAIN, &token()?));
        Ok(())
    }

    #[benchmark]
    fn enable_route() -> Result<(), BenchmarkError> {
        Pallet::<T>::register_asset(gov::<T>()?, SOLANA_CHAIN, solana_token()?, X3_ASSET)
            .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        enable_route(gov::<T>()?, route()?);

        assert!(Routes::<T>::contains_key(ROUTE_ID));
        Ok(())
    }

    #[benchmark]
    fn disable_route() -> Result<(), BenchmarkError> {
        setup_route::<T>()?;

        #[extrinsic_call]
        disable_route(gov::<T>()?, ROUTE_ID);

        assert!(!Routes::<T>::get(ROUTE_ID)
            .map(|r| r.enabled)
            .unwrap_or(true));
        Ok(())
    }

    #[benchmark]
    fn submit_deposit_proof() -> Result<(), BenchmarkError> {
        setup_route::<T>()?;

        #[extrinsic_call]
        submit_deposit_proof(relayer::<T>()?, ROUTE_ID, solana_deposit_proof::<T>(1)?);

        assert_eq!(
            Transfers::<T>::get(PROOF_ID).map(|t| t.status),
            Some(GatewayTransferStatus::Verified)
        );
        Ok(())
    }

    #[benchmark]
    fn credit_x3_representation() -> Result<(), BenchmarkError> {
        setup_verified_transfer::<T>()?;

        #[extrinsic_call]
        credit_x3_representation(operational::<T>()?, PROOF_ID);

        assert_eq!(
            Transfers::<T>::get(PROOF_ID).map(|t| t.status),
            Some(GatewayTransferStatus::X3Credited)
        );
        Ok(())
    }

    #[benchmark]
    fn request_withdrawal() -> Result<(), BenchmarkError> {
        let who: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        request_withdrawal(
            RawOrigin::Signed(who),
            X3_ASSET,
            CHAIN,
            recipient()?,
            WITHDRAW_AMOUNT,
        );

        assert_eq!(Withdrawals::<T>::iter().count(), 1);
        Ok(())
    }

    #[benchmark]
    fn burn_x3_representation() -> Result<(), BenchmarkError> {
        setup_verified_transfer::<T>()?;
        let who: T::AccountId = whitelisted_caller();
        let recipient = recipient()?;
        Pallet::<T>::request_withdrawal(
            RawOrigin::Signed(who.clone()).into(),
            X3_ASSET,
            CHAIN,
            recipient.clone(),
            WITHDRAW_AMOUNT,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        let now: u64 = frame_system::Pallet::<T>::block_number().saturated_into();
        let id = Pallet::<T>::derive_withdrawal_id(X3_ASSET, &recipient[..], WITHDRAW_AMOUNT, now);
        T::DepositMinter::prepare_benchmark(
            CHAIN.wrapped_chain_id().ok_or(BenchmarkError::Weightless)?,
            X3_ASSET,
            &who.encode(),
            WITHDRAW_AMOUNT,
        )
        .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        burn_x3_representation(RawOrigin::Signed(who), id);

        assert!(Withdrawals::<T>::get(id).map(|w| w.burned).unwrap_or(false));
        Ok(())
    }

    #[benchmark]
    fn finalize_external_release() -> Result<(), BenchmarkError> {
        let id = setup_burned_withdrawal::<T>()?;

        #[extrinsic_call]
        finalize_external_release(operational::<T>()?, id);

        assert!(Withdrawals::<T>::get(id)
            .map(|w| w.released)
            .unwrap_or(false));
        Ok(())
    }

    #[benchmark]
    fn submit_release_proof() -> Result<(), BenchmarkError> {
        let id = setup_burned_withdrawal::<T>()?;
        let recipient = recipient()?;
        let payload = solana_release_payload(id, &recipient)?;

        #[extrinsic_call]
        submit_release_proof(relayer::<T>()?, id, ROUTE_ID, payload);

        assert!(Withdrawals::<T>::get(id)
            .map(|w| w.released)
            .unwrap_or(false));
        Ok(())
    }

    #[benchmark]
    fn set_svm_validators() -> Result<(), BenchmarkError> {
        let validators: BoundedVec<[u8; 32], MaxSvmValidators> =
            BoundedVec::try_from(vec![[3u8; 32]]).map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        set_svm_validators(gov::<T>()?, CHAIN, validators, 1u32);

        assert!(SvmValidatorSets::<T>::contains_key(CHAIN));
        Ok(())
    }

    #[benchmark]
    fn clear_svm_validators() -> Result<(), BenchmarkError> {
        let validators: BoundedVec<[u8; 32], MaxSvmValidators> =
            BoundedVec::try_from(vec![[3u8; 32]]).map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::set_svm_validators(gov::<T>()?, CHAIN, validators, 1u32)
            .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        clear_svm_validators(gov::<T>()?, CHAIN);

        assert!(!SvmValidatorSets::<T>::contains_key(CHAIN));
        Ok(())
    }

    #[benchmark]
    fn set_solana_stake_committee() -> Result<(), BenchmarkError> {
        #[extrinsic_call]
        set_solana_stake_committee(gov::<T>()?, SOLANA_CHAIN, solana_stakes()?, 2u32, 3u32);

        assert!(SolanaStakeCommittees::<T>::contains_key(SOLANA_CHAIN));
        Ok(())
    }

    #[benchmark]
    fn clear_solana_stake_committee() -> Result<(), BenchmarkError> {
        Pallet::<T>::set_solana_stake_committee(
            gov::<T>()?,
            SOLANA_CHAIN,
            solana_stakes()?,
            2u32,
            3u32,
        )
        .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        clear_solana_stake_committee(gov::<T>()?, SOLANA_CHAIN);

        assert!(!SolanaStakeCommittees::<T>::contains_key(SOLANA_CHAIN));
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
