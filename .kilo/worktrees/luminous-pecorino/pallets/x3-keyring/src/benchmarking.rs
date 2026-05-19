//! Benchmarking for the X3 Keyring pallet.

use super::*;
use crate::pallet as pallet_x3_keyring;
use frame_benchmarking::v2::*;
use frame_support::pallet_prelude::*;
use frame_system::RawOrigin;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_attestor() {
        let caller: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        register_attestor(RawOrigin::Signed(caller));
    }

    #[benchmark]
    fn submit_keyring_proof() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = Pallet::<T>::register_attestor(RawOrigin::Signed(caller.clone()).into());

        let keyring_id: KeyringId = H256::from([1u8; 32]);
        let proof_data: BoundedVec<u8, T::MaxProofSize> =
            b"test_proof_data".to_vec().try_into().unwrap();
        let attestation_hash: H256 = H256::from([2u8; 32]);

        #[extrinsic_call]
        submit_keyring_proof(
            RawOrigin::Signed(caller),
            keyring_id,
            proof_data,
            attestation_hash,
        );
    }

    #[benchmark]
    fn confirm_keyring_proof() {
        let caller1: T::AccountId = whitelisted_caller();
        let caller2: T::AccountId = account("caller2", 0, 0);
        let _ = Pallet::<T>::register_attestor(RawOrigin::Signed(caller1.clone()).into());
        let _ = Pallet::<T>::register_attestor(RawOrigin::Signed(caller2.clone()).into());

        let keyring_id: KeyringId = H256::from([1u8; 32]);
        let proof_data: BoundedVec<u8, T::MaxProofSize> =
            b"test_proof_data".to_vec().try_into().unwrap();
        let attestation_hash: H256 = H256::from([2u8; 32]);

        let _ = Pallet::<T>::submit_keyring_proof(
            RawOrigin::Signed(caller1.clone()).into(),
            keyring_id,
            proof_data,
            attestation_hash,
        );

        // Compute proof_id the same way the pallet does
        let proof_id = {
            let block = T::BlockNumber::from(1u32);
            let mut data = caller1.encode();
            data.extend(keyring_id.as_bytes());
            data.extend(attestation_hash.as_bytes());
            data.extend(block.encode());
            H256::from(sp_io::hashing::blake2_256(&data))
        };

        #[extrinsic_call]
        confirm_keyring_proof(RawOrigin::Signed(caller2), proof_id);
    }

    #[benchmark]
    fn reject_keyring_proof() {
        let caller1: T::AccountId = whitelisted_caller();
        let caller2: T::AccountId = account("caller2", 0, 0);
        let _ = Pallet::<T>::register_attestor(RawOrigin::Signed(caller1.clone()).into());
        let _ = Pallet::<T>::register_attestor(RawOrigin::Signed(caller2.clone()).into());

        let keyring_id: KeyringId = H256::from([1u8; 32]);
        let proof_data: BoundedVec<u8, T::MaxProofSize> =
            b"test_proof_data".to_vec().try_into().unwrap();
        let attestation_hash: H256 = H256::from([2u8; 32]);

        let _ = Pallet::<T>::submit_keyring_proof(
            RawOrigin::Signed(caller1.clone()).into(),
            keyring_id,
            proof_data,
            attestation_hash,
        );

        let proof_id = {
            let block = T::BlockNumber::from(1u32);
            let mut data = caller1.encode();
            data.extend(keyring_id.as_bytes());
            data.extend(attestation_hash.as_bytes());
            data.extend(block.encode());
            H256::from(sp_io::hashing::blake2_256(&data))
        };

        let reason: BoundedVec<u8, ConstU32<128>> = b"Invalid proof".to_vec().try_into().unwrap();

        #[extrinsic_call]
        reject_keyring_proof(RawOrigin::Signed(caller2), proof_id, reason);
    }

    #[benchmark]
    fn deactivate_attestor() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = Pallet::<T>::register_attestor(RawOrigin::Signed(caller.clone()).into());

        #[extrinsic_call]
        deactivate_attestor(RawOrigin::Signed(caller));
    }

    #[benchmark]
    fn reactivate_attestor() {
        let caller: T::AccountId = whitelisted_caller();
        let _ = Pallet::<T>::register_attestor(RawOrigin::Signed(caller.clone()).into());
        let _ = Pallet::<T>::deactivate_attestor(RawOrigin::Signed(caller.clone()).into());

        #[extrinsic_call]
        reactivate_attestor(RawOrigin::Root, caller);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}