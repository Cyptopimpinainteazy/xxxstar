use crate::mock::*;
use crate::LastEvmHeader;
use frame_support::assert_ok;
use sp_core::H256;
use sp_runtime::BuildStorage;

#[test]
fn test_evm_header_validation() {
    new_test_ext().execute_with(|| {
        let block_number = 100u64;
        let block_hash = H256::from([1u8; 32]);
        let state_root = H256::from([2u8; 32]);
        let merkle_root = H256::from([3u8; 32]);
        let proof = vec![4u8; 32];

        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            block_number,
            block_hash,
            state_root,
            merkle_root,
            proof,
        ));

        let stored = LastEvmHeader::<MockRuntime>::get();
        assert!(stored.is_some());
        let header = stored.unwrap();
        assert_eq!(header.block_number, block_number);
        assert_eq!(header.block_hash, block_hash);
    });
}

#[test]
fn test_invalid_evm_header_zero_block() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            0,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([3u8; 32]),
            vec![4u8; 32],
        )
        .is_err());
    });
}

#[test]
fn test_svm_header_validation() {
    new_test_ext().execute_with(|| {
        let slot = 200u64;
        let block_hash = H256::from([10u8; 32]);
        let state_root = H256::from([11u8; 32]);
        let validator_set = vec![12u8; 64];
        let parent_slot_hashes = vec![H256::from([13u8; 32]); 3];

        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            slot,
            block_hash,
            state_root,
            validator_set,
            parent_slot_hashes.clone(),
        ));

        let stored = crate::LastSvmHeader::<MockRuntime>::get();
        assert!(stored.is_some());
        let header = stored.unwrap();
        assert_eq!(header.slot, slot);
        assert_eq!(header.block_hash, block_hash);
        assert_eq!(header.parent_slot_hashes.len(), 3);
    });
}

#[test]
fn test_invalid_svm_header_zero_slot() {
    new_test_ext().execute_with(|| {
        assert!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            0,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            vec![3u8; 32],
            vec![],
        )
        .is_err());
    });
}

#[test]
fn test_merkle_root_caching() {
    new_test_ext().execute_with(|| {
        let block_number = 150u64;
        let merkle_root = H256::from([50u8; 32]);

        // Store EVM header with merkle root
        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            block_number,
            H256::from([51u8; 32]),
            H256::from([52u8; 32]),
            merkle_root,
            vec![53u8; 32],
        ));

        // Verify merkle root is cached
        let is_verified = crate::Pallet::<MockRuntime>::is_evm_merkle_root_verified(block_number, merkle_root);
        assert!(is_verified);

        // Verify wrong merkle root is not verified
        let wrong_root = H256::from([99u8; 32]);
        let is_wrong_verified = crate::Pallet::<MockRuntime>::is_evm_merkle_root_verified(block_number, wrong_root);
        assert!(!is_wrong_verified);
    });
}

#[test]
fn test_validator_set_caching() {
    new_test_ext().execute_with(|| {
        let slot = 300u64;
        let validator_set = vec![60u8; 64];
        let validator_set_hash = H256::from(sp_io::hashing::blake2_256(&validator_set));
        let parent_slot_hashes = vec![H256::from([63u8; 32]); 3];

        // Store SVM header with validator set
        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            slot,
            H256::from([61u8; 32]),
            H256::from([62u8; 32]),
            validator_set,
            parent_slot_hashes,
        ));

        // Verify validator set is cached
        let is_verified = crate::Pallet::<MockRuntime>::is_svm_validator_set_verified(slot, validator_set_hash);
        assert!(is_verified);

        // Verify wrong validator set is not verified
        let wrong_set_hash = H256::from([99u8; 32]);
        let is_wrong_verified = crate::Pallet::<MockRuntime>::is_svm_validator_set_verified(slot, wrong_set_hash);
        assert!(!is_wrong_verified);
    });
}

#[test]
fn test_validation_statistics_update() {
    new_test_ext().execute_with(|| {
        // Initial stats should be zero
        let initial_stats = crate::ValidationStats::<MockRuntime>::get();
        assert_eq!(initial_stats.evm_headers_validated, 0);
        assert_eq!(initial_stats.svm_headers_validated, 0);

        // Add EVM header
        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            100,
            H256::from([1u8; 32]),
            H256::from([2u8; 32]),
            H256::from([3u8; 32]),
            vec![4u8; 32],
        ));

        // Add SVM header
        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            200,
            H256::from([10u8; 32]),
            H256::from([11u8; 32]),
            vec![12u8; 64],  // 64 bytes = 2 validators
            vec![H256::from([13u8; 32])],  // At least 1 parent_slot_hash
        ));

        // Stats should be updated
        let updated_stats = crate::ValidationStats::<MockRuntime>::get();
        assert_eq!(updated_stats.evm_headers_validated, 1);
        assert_eq!(updated_stats.svm_headers_validated, 1);
    });
}

#[test]
fn test_cross_chain_settlement_scenario() {
    new_test_ext().execute_with(|| {
        // Simulate EVM → Solana cross-chain settlement
        
        // Step 1: Validate EVM header (source chain)
        let evm_block = 1000u64;
        let evm_merkle_root = H256::from([70u8; 32]);
        assert_ok!(crate::Pallet::<MockRuntime>::validate_evm_header(
            RuntimeOrigin::signed(1),
            evm_block,
            H256::from([71u8; 32]),
            H256::from([72u8; 32]),
            evm_merkle_root,
            vec![73u8; 32],
        ));

        // Step 2: Validate SVM header (destination chain)
        let svm_slot = 2000u64;
        let svm_validator_set = vec![80u8; 64];
        let svm_validator_set_hash = H256::from(sp_io::hashing::blake2_256(&svm_validator_set));
        assert_ok!(crate::Pallet::<MockRuntime>::validate_svm_header(
            RuntimeOrigin::signed(1),
            svm_slot,
            H256::from([81u8; 32]),
            H256::from([82u8; 32]),
            svm_validator_set,
            vec![H256::from([83u8; 32])],
        ));

        // Step 3: Verify both chain states are available
        let evm_verified = crate::Pallet::<MockRuntime>::is_evm_merkle_root_verified(evm_block, evm_merkle_root);
        let svm_verified = crate::Pallet::<MockRuntime>::is_svm_validator_set_verified(svm_slot, svm_validator_set_hash);
        
        assert!(evm_verified);
        assert!(svm_verified);

        // Step 4: Check cross-chain validation statistics
        let stats = crate::ValidationStats::<MockRuntime>::get();
        assert_eq!(stats.evm_headers_validated, 1);
        assert_eq!(stats.svm_headers_validated, 1);
    });
}

fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<MockRuntime>::default()
        .build_storage()
        .unwrap()
        .into()
}
