//! Benchmarks for the X3 Identity Verifier pallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as X3IdentityVerifier;
use frame_benchmarking::{account, benchmarks, whitelist_account, BenchmarkError, Vec};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;

benchmarks! {
    /// Benchmark the registration of a new verifier.
    register_verifier {
        let v in 0 .. T::MaxIdentitiesPerVerifier::get();
    }: _(RawOrigin::Signed(account("verifier", v, 0)),)
    verify {
        assert!(Verifiers::<T>::contains_key(&account("verifier", v, 0)));
    }

    /// Benchmark submitting an identity proof.
    submit_identity_proof {
        let v = 0u32;
        let caller = account("verifier", 0, 0);
        whitelist_account!(caller);

        // Register verifier first
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller.clone())).unwrap();

        let identity_id = IdentityId::from_slice(&[v as u8; 32]);
        let proof_data: BoundedVec<u8, T::MaxProofSize> =
            vec![0u8; T::MaxProofSize::get() as usize / 2]
                .try_into()
                .unwrap();
        let v_hash = H256::from([v as u8; 32]);
    }: _(RawOrigin::Signed(caller), identity_id, ProofType::Signature, proof_data, v_hash)
    verify {
        assert!(IdentityProofs::<T>::iter().count() > 0);
    }

    /// Benchmark confirming an identity proof.
    confirm_identity_proof {
        let v = 0u32;
        let caller1 = account("verifier", 0, 0);
        let caller2 = account("verifier", 1, 0);
        whitelist_account!(caller1);
        whitelist_account!(caller2);

        // Setup: fund, register two verifiers
        let _ = T::Currency::make_free_balance_be(&caller1, BalanceOf::<T>::max_value());
        let _ = T::Currency::make_free_balance_be(&caller2, BalanceOf::<T>::max_value());
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller1.clone())).unwrap();
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller2.clone())).unwrap();

        let identity_id = IdentityId::from_slice(&[v as u8; 32]);
        let proof_data: BoundedVec<u8, T::MaxProofSize> =
            vec![0u8; 32usize]
                .try_into()
                .unwrap();
        let v_hash = H256::from([v as u8; 32]);

        // Caller1 submits proof
        let proof_id = {
            let block = frame_system::Pallet::<T>::block_number();
            let mut data = caller1.encode();
            data.extend(identity_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };
        <X3IdentityVerifier<T>>::submit_identity_proof(
            RawOrigin::Signed(caller1.clone()),
            identity_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ).unwrap();

        // Ensure caller2 hasn't confirmed yet
        ensure!(!ProofConfirmations::<T>::contains_key(&proof_id, &caller2), "caller2 already confirmed");
    }: _(RawOrigin::Signed(caller2), proof_id)
    verify {
        let proof = IdentityProofs::<T>::get(&proof_id).unwrap();
        // With MinConfirmations = 2 (default test), this triggers verification
        assert_eq!(proof.confirmations, 2);
    }

    /// Benchmark rejecting an identity proof.
    reject_identity_proof {
        let v = 1u32;
        let caller1 = account("verifier", 2, 0);
        let caller2 = account("verifier", 3, 0);
        whitelist_account!(caller1);
        whitelist_account!(caller2);

        let _ = T::Currency::make_free_balance_be(&caller1, BalanceOf::<T>::max_value());
        let _ = T::Currency::make_free_balance_be(&caller2, BalanceOf::<T>::max_value());
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller1.clone())).unwrap();
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller2.clone())).unwrap();

        let identity_id = IdentityId::from_slice(&[v as u8; 32]);
        let proof_data: BoundedVec<u8, T::MaxProofSize> =
            vec![0u8; 32usize]
                .try_into()
                .unwrap();
        let v_hash = H256::from([v as u8; 32]);

        let proof_id = {
            let block = frame_system::Pallet::<T>::block_number();
            let mut data = caller1.encode();
            data.extend(identity_id.as_bytes());
            data.extend(v_hash.as_bytes());
            data.extend(block.encode());
            H256::from(blake2_256(&data))
        };
        <X3IdentityVerifier<T>>::submit_identity_proof(
            RawOrigin::Signed(caller1.clone()),
            identity_id,
            ProofType::Signature,
            proof_data,
            v_hash,
        ).unwrap();

        let reason: BoundedVec<u8, ConstU32<128>> =
            b"Invalid proof format".to_vec().try_into().unwrap();
    }: _(RawOrigin::Signed(caller2), proof_id, reason)
    verify {
        let proof = IdentityProofs::<T>::get(&proof_id).unwrap();
        assert_eq!(proof.status, VerificationStatus::Rejected);
    }

    /// Benchmark deactivating a verifier.
    deactivate_verifier {
        let caller = account("verifier", 4, 0);
        whitelist_account!(caller);

        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller.clone())).unwrap();
        assert!(Verifiers::<T>::get(&caller).unwrap().active);
    }: _(RawOrigin::Signed(caller))
    verify {
        assert!(!Verifiers::<T>::get(&account("verifier", 4, 0)).unwrap().active);
    }

    /// Benchmark reactivating a verifier.
    reactivate_verifier {
        let caller = account("verifier", 5, 0);
        whitelist_account!(caller);

        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller.clone())).unwrap();
        <X3IdentityVerifier<T>>::deactivate_verifier(RawOrigin::Signed(caller.clone())).unwrap();
    }: _(RawOrigin::Root, caller)
    verify {
        assert!(Verifiers::<T>::get(&account("verifier", 5, 0)).unwrap().active);
    }

    /// Benchmark renewing a verified identity.
    renew_identity {
        let v = 2u32;
        let caller = account("verifier", 6, 0);
        whitelist_account!(caller);

        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        <X3IdentityVerifier<T>>::register_verifier(RawOrigin::Signed(caller.clone())).unwrap();

        let identity_id = IdentityId::from_slice(&[v as u8; 32]);
        let verification_result = VerificationResult {
            verified: true,
            identity_id,
            confirmation_count: 2,
            timestamp: sp_io::offchain::timestamp().unix_millis() as u64,
            proof_type: ProofType::Signature,
        };
        VerifiedIdentities::<T>::insert(&identity_id, &verification_result);
    }: _(RawOrigin::Signed(caller), identity_id)
    verify {
        assert!(VerifiedIdentities::<T>::contains_key(&identity_id));
    }

    // Verify worst case analysis
    // register_verifier: 1 DB write (verifier) + 1 DB write (total count) + 1 reserve
    // submit_identity_proof: 1 DB write (proof) + 1 DB write (total count)
    // confirm_identity_proof: 2 DB writes (confirmation + proof update) + possibly 1 more for verified identity
    // reject_identity_proof: 3 DB writes (proof update + slash update + stats update)
    // deactivate_verifier: 1 DB write + 1 unreserve
    // reactivate_verifier: 1 DB write + 1 reserve
    // renew_identity: 1 DB read + 1 DB write
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_register_verifier_benchmark() {
        new_test_ext().execute_with(|| {
            assert_ok!(X3IdentityVerifier::<Test>::register_verifier(
                <Test as frame_system::Config>::RuntimeOrigin::signed(1)
            ));
            assert!(Verifiers::<Test>::contains_key(&1u64));
        });
    }

    #[test]
    fn test_submit_identity_proof_benchmark() {
        new_test_ext().execute_with(|| {
            use sp_core::H256;

            assert_ok!(X3IdentityVerifier::<Test>::register_verifier(
                <Test as frame_system::Config>::RuntimeOrigin::signed(1)
            ));

            let id_id = IdentityId::from_slice(&[1u8; 32]);
            let proof_data: BoundedVec<u8, crate::ConstU32<512>> =
                b"bench_proof".to_vec().try_into().unwrap();
            let v_hash = H256::from([1u8; 32]);

            assert_ok!(X3IdentityVerifier::<Test>::submit_identity_proof(
                <Test as frame_system::Config>::RuntimeOrigin::signed(1),
                id_id,
                ProofType::Signature,
                proof_data,
                v_hash,
            ));

            assert!(IdentityProofs::<Test>::iter().count() > 0);
        });
    }
}