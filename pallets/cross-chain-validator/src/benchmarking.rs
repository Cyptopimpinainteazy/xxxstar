use super::*;
use codec::Decode;
use frame_benchmarking::v2::*;
use frame_benchmarking::{account, whitelisted_caller, BenchmarkError};
use frame_support::traits::EnsureOrigin;
use frame_system::RawOrigin;
use sp_core::{ed25519, Pair, H256};
use sp_std::{vec, vec::Vec};

const SEED: u32 = 0;

fn admin_origin<T: Config>() -> Result<T::RuntimeOrigin, BenchmarkError> {
    T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)
}

fn decode_fixture<T: Decode>(bytes: &[u8]) -> Result<T, BenchmarkError> {
    T::decode(&mut &bytes[..]).map_err(|_| BenchmarkError::Weightless)
}

fn beacon_checkpoint(
) -> Result<x3_verification_router::ethereum_beacon::EthereumCheckpointUpdate, BenchmarkError> {
    decode_fixture(include_bytes!(
        "../../../crates/x3-verification-router/tests/fixtures/ethereum/bench/checkpoint.scale"
    ))
}

fn beacon_forks() -> Result<x3_verification_router::ethereum_beacon::ForkVersions, BenchmarkError> {
    decode_fixture(include_bytes!(
        "../../../crates/x3-verification-router/tests/fixtures/ethereum/bench/forks.scale"
    ))
}

fn beacon_update(
    fixture: &'static [u8],
) -> Result<x3_verification_router::ethereum_beacon::EthereumUpdate, BenchmarkError> {
    decode_fixture(fixture)
}

fn execution_proof(
) -> Result<x3_verification_router::ethereum_beacon::EthereumExecutionProof, BenchmarkError> {
    decode_fixture(include_bytes!(
        "../../../crates/x3-verification-router/tests/fixtures/ethereum/bench/execution-proof.scale"
    ))
}

fn attester(seed: u8) -> ed25519::Pair {
    ed25519::Pair::from_seed(&[seed; 32])
}

fn certificate(
    chain: u8,
    height: u64,
    block_hash: H256,
    state_root: H256,
    auxiliary_root: H256,
    pair: &ed25519::Pair,
) -> Vec<u8> {
    use x3_verification_router::header_finality::{
        encode_header_certificate, header_finality_message, FinalizedHeader,
    };

    let header = FinalizedHeader {
        chain,
        height,
        block_hash: block_hash.0,
        state_root: state_root.0,
        auxiliary_root: auxiliary_root.0,
    };
    let message = header_finality_message(&header);
    let signature = pair.sign(&message);
    encode_header_certificate(header, &[(pair.public().0, signature.0)])
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn validate_evm_header() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = account("caller", 0, SEED);
        Pallet::<T>::set_authorized_submitters(admin_origin::<T>()?, vec![caller.clone()])
            .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        validate_evm_header(
            RawOrigin::Signed(caller),
            100u64,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([4u8; 32]),
            vec![4u8; 32],
        );
        Ok(())
    }

    #[benchmark]
    fn validate_svm_header() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = account("caller", 0, SEED);
        Pallet::<T>::set_authorized_submitters(admin_origin::<T>()?, vec![caller.clone()])
            .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        validate_svm_header(
            RawOrigin::Signed(caller),
            100u64,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            vec![3u8; 32],
            vec![H256::from([5u8; 32])],
        );
        Ok(())
    }

    #[benchmark]
    fn set_authorized_submitters() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = account("caller", 0, SEED);

        #[extrinsic_call]
        set_authorized_submitters(admin_origin::<T>()?, vec![caller]);
        Ok(())
    }

    #[benchmark]
    fn set_header_attesters() -> Result<(), BenchmarkError> {
        let evm_key = attester(1).public().0;
        let svm_key = attester(2).public().0;

        #[extrinsic_call]
        set_header_attesters(admin_origin::<T>()?, vec![evm_key], 1, vec![svm_key], 1);
        Ok(())
    }

    #[benchmark]
    fn validate_finalized_evm_header() -> Result<(), BenchmarkError> {
        use x3_verification_router::header_finality::HEADER_FINALITY_ETHEREUM;

        let caller: T::AccountId = whitelisted_caller();
        let pair = attester(3);
        Pallet::<T>::set_header_attesters(
            admin_origin::<T>()?,
            vec![pair.public().0],
            1,
            vec![],
            0,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        let block_hash = H256::repeat_byte(0x11);
        let state_root = H256::repeat_byte(0x22);
        let receipts_root = H256::repeat_byte(0x33);
        let cert = certificate(
            HEADER_FINALITY_ETHEREUM,
            100,
            block_hash,
            state_root,
            receipts_root,
            &pair,
        );

        #[extrinsic_call]
        validate_finalized_evm_header(
            RawOrigin::Signed(caller),
            100,
            block_hash,
            state_root,
            receipts_root,
            cert,
        );
        Ok(())
    }

    #[benchmark]
    fn validate_finalized_svm_header() -> Result<(), BenchmarkError> {
        use x3_verification_router::header_finality::HEADER_FINALITY_SOLANA;

        let caller: T::AccountId = whitelisted_caller();
        let pair = attester(4);
        Pallet::<T>::set_header_attesters(
            admin_origin::<T>()?,
            vec![],
            0,
            vec![pair.public().0],
            1,
        )
        .map_err(|_| BenchmarkError::Weightless)?;
        let block_hash = H256::repeat_byte(0x44);
        let state_root = H256::repeat_byte(0x55);
        let validator_set_hash = H256::repeat_byte(0x66);
        let cert = certificate(
            HEADER_FINALITY_SOLANA,
            100,
            block_hash,
            state_root,
            validator_set_hash,
            &pair,
        );

        #[extrinsic_call]
        validate_finalized_svm_header(
            RawOrigin::Signed(caller),
            100,
            block_hash,
            state_root,
            validator_set_hash,
            cert,
        );
        Ok(())
    }

    #[benchmark]
    fn set_ethereum_beacon_checkpoint() -> Result<(), BenchmarkError> {
        let checkpoint = beacon_checkpoint()?;
        let forks = beacon_forks()?;

        #[extrinsic_call]
        set_ethereum_beacon_checkpoint(admin_origin::<T>()?, checkpoint, forks);
        Ok(())
    }

    #[benchmark]
    fn submit_ethereum_beacon_update() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let checkpoint = beacon_checkpoint()?;
        let forks = beacon_forks()?;
        let update = beacon_update(include_bytes!(
        "../../../crates/x3-verification-router/tests/fixtures/ethereum/bench/sync-committee-update.scale"
    ))?;
        Pallet::<T>::set_ethereum_beacon_checkpoint(admin_origin::<T>()?, checkpoint, forks)
            .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        submit_ethereum_beacon_update(RawOrigin::Signed(caller), update);
        Ok(())
    }

    #[benchmark]
    fn submit_ethereum_execution_proof() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let checkpoint = beacon_checkpoint()?;
        let forks = beacon_forks()?;
        let first = beacon_update(include_bytes!(
        "../../../crates/x3-verification-router/tests/fixtures/ethereum/bench/sync-committee-update.scale"
    ))?;
        let second = beacon_update(include_bytes!(
        "../../../crates/x3-verification-router/tests/fixtures/ethereum/bench/finalized-header-update.scale"
    ))?;
        let proof = execution_proof()?;
        Pallet::<T>::set_ethereum_beacon_checkpoint(admin_origin::<T>()?, checkpoint, forks)
            .map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::submit_ethereum_beacon_update(RawOrigin::Signed(caller.clone()).into(), first)
            .map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::submit_ethereum_beacon_update(
            RawOrigin::Signed(caller.clone()).into(),
            second,
        )
        .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        submit_ethereum_execution_proof(RawOrigin::Signed(caller), proof);
        Ok(())
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::mock::new_test_ext(),
        crate::mock::MockRuntime
    );
}
