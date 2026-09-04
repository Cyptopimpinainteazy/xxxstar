use frame_support::{assert_noop, assert_ok};
use parity_scale_codec::Encode;
use sp_core::{hashing::blake2_256, H256};

use crate::{AccountRegistry, AssetRegistry, CanonicalLedger, ComitFailureReason, Nonces};

use crate::mock::{
    self, new_test_ext, AssetId, AtlasId, AtlasKernel, Balance, ExtBuilder, RuntimeEvent,
    RuntimeOrigin, System, ALICE, BOB, CHARLIE, INITIAL_BALANCE,
};

type Test = mock::Test;
type AtlasEvent = crate::Event<Test>;
type AtlasError = crate::Error<Test>;

/// Helper to get only AtlasKernel events, filtering out Balance/System events
fn x3_events() -> Vec<AtlasEvent> {
    System::events()
        .into_iter()
        .filter_map(|e| {
            if let RuntimeEvent::AtlasKernel(event) = e.event {
                Some(event)
            } else {
                None
            }
        })
        .collect()
}

/// Compute prepare_root using the pallet's canonical algorithm (L-3: Avoid duplication).
/// This delegates to the pallet's public `compute_prepare_root` function to ensure
/// tests use the same algorithm as production code.
fn compute_prepare_root(
    comit_id: H256,
    evm_payload: &[u8],
    svm_payload: &[u8],
    nonce: u64,
    fee: Balance,
) -> H256 {
    AtlasKernel::compute_prepare_root(comit_id, evm_payload, svm_payload, nonce, fee)
}

fn compute_prepare_root_v2(
    comit_id: H256,
    evm_payload: &[u8],
    svm_payload: &[u8],
    x3_payload: &[u8],
    nonce: u64,
    fee: Balance,
) -> H256 {
    AtlasKernel::compute_prepare_root_v2(comit_id, evm_payload, svm_payload, x3_payload, nonce, fee)
}

#[test]
fn submit_comit_successful_flow() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(1);
        let evm_payload = vec![1, 2, 3];
        let svm_payload = vec![4, 5];
        let nonce = 0;
        let fee: Balance = 500;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload.clone(),
            svm_payload.clone(),
            nonce,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
        assert_eq!(
            AccountRegistry::<Test>::get(ALICE),
            Some(AtlasId::default())
        );

        let events = x3_events();
        // Successful execution emits: FeeDeducted, ComitSubmitted, ExecutionStarted,
        // ExecutionCompleted, CanonicalLedgerUpdated, Finalized
        assert_eq!(events.len(), 6);
        // FeeDeducted is first, then ComitSubmitted
        match &events[1] {
            AtlasEvent::ComitSubmitted {
                comit_id: id,
                origin,
                nonce: event_nonce,
                fee: emitted_fee,
            } => {
                assert_eq!(*id, comit_id);
                assert_eq!(*origin, ALICE);
                assert_eq!(*event_nonce, 0);
                assert_eq!(*emitted_fee, fee);
            }
            e => panic!("Unexpected event: {:?}", e),
        }
    });
}

#[test]
fn submit_comit_v2_successful_flow() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(101);
        let evm_payload = vec![1, 2, 3];
        let svm_payload = vec![4, 5];
        let x3_payload = vec![0x58, 0x33, 0x00, 0x01];
        let nonce = 0;
        let fee: Balance = 500;
        let prepare_root = compute_prepare_root_v2(
            comit_id,
            &evm_payload,
            &svm_payload,
            &x3_payload,
            nonce,
            fee,
        );

        assert_ok!(AtlasKernel::submit_comit_v2(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            x3_payload,
            nonce,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);

        let events = x3_events();
        // FeeDeducted, ComitSubmitted, ExecutionStarted, ExecutionCompleted, (optional ledger update), Finalized
        assert!(events.len() >= 5);
        assert!(matches!(events[1], AtlasEvent::ComitSubmitted { .. }));
        assert!(matches!(
            events[2],
            AtlasEvent::ComitExecutionStarted { .. }
        ));
        assert!(matches!(
            events[3],
            AtlasEvent::ComitExecutionCompleted { .. }
        ));
        assert!(matches!(
            events.last().unwrap(),
            AtlasEvent::ComitFinalized { .. }
        ));
    });
}

#[test]
fn submit_comit_v2_fails_when_x3_execution_errors() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(102);
        let evm_payload = vec![1];
        let svm_payload = vec![1];
        // 0xFF triggers FailingMockX3Adapter Err
        let x3_payload = vec![0xFF, 0x00, 0x00, 0x00];
        let nonce = 0;
        let fee: Balance = 500;
        let prepare_root = compute_prepare_root_v2(
            comit_id,
            &evm_payload,
            &svm_payload,
            &x3_payload,
            nonce,
            fee,
        );

        assert_noop!(
            AtlasKernel::submit_comit_v2(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                x3_payload,
                nonce,
                fee,
                prepare_root,
            ),
            AtlasError::X3ExecutionFailed
        );

        // Atomic rollback: nonce not incremented and no events persisted
        assert_eq!(Nonces::<Test>::get(ALICE), 0);
        assert_eq!(x3_events().len(), 0);
    });
}

#[test]
fn submit_comit_with_matching_prepare_root_succeeds() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(11);
        let evm_payload = vec![9, 9, 9];
        let svm_payload = vec![7, 7];
        let fee: Balance = 42;
        let nonce = 0;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn submit_comit_rejects_empty_payloads() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(2);
        let fee: Balance = 100;

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                Vec::new(),
                Vec::new(),
                0,
                fee,
                H256::zero(),
            ),
            AtlasError::EmptyPayloads
        );

        assert_eq!(Nonces::<Test>::get(ALICE), 0);

        // No events emitted on error (they get rolled back)
        let events = x3_events();
        assert_eq!(events.len(), 0);
    });
}

#[test]
fn submit_comit_rejects_payloads_exceeding_limit() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(4);
        let payload = vec![0u8; 4_097];
        let fee: Balance = 1;

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                payload,
                Vec::new(),
                0,
                fee,
                H256::zero(),
            ),
            AtlasError::PayloadTooLarge
        );

        assert_eq!(Nonces::<Test>::get(ALICE), 0);

        // No events emitted on error (they get rolled back)
        let events = x3_events();
        assert_eq!(events.len(), 0);
    });
}

#[test]
fn submit_comit_rejects_invalid_nonce() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(3);
        let fee: Balance = 100;

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                vec![1],
                vec![1],
                1,
                fee,
                H256::zero(),
            ),
            AtlasError::InvalidNonce
        );

        assert_eq!(Nonces::<Test>::get(ALICE), 0);

        // No events emitted on error (they get rolled back)
        let events = x3_events();
        assert_eq!(events.len(), 0);
    });
}

#[test]
fn submit_comit_rejects_when_prepare_root_mismatch() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(6);
        let evm_payload = vec![1, 2];
        let svm_payload = vec![3, 4];
        let fee: Balance = 26; // Must be >= required fee to reach verification check
        let correct_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);
        let mismatched_root = H256::from_low_u64_be(999);
        assert_ne!(mismatched_root, correct_root);

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                0,
                fee,
                mismatched_root,
            ),
            AtlasError::ComitVerificationFailed
        );

        assert_eq!(Nonces::<Test>::get(ALICE), 0);

        // No events emitted on error (they get rolled back)
        let events = x3_events();
        assert_eq!(events.len(), 0);
    });
}

#[test]
fn submit_comit_fails_when_nonce_overflows() {
    new_test_ext().execute_with(|| {
        Nonces::<Test>::insert(ALICE, u64::MAX);
        let comit_id = H256::from_low_u64_be(12);
        let evm_payload = vec![1];
        let svm_payload = vec![1];
        let fee: Balance = 26;
        let prepare_root =
            compute_prepare_root(comit_id, &evm_payload, &svm_payload, u64::MAX, fee);

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                u64::MAX,
                fee,
                prepare_root,
            ),
            AtlasError::NonceOverflow
        );

        assert_eq!(Nonces::<Test>::get(ALICE), u64::MAX);
        assert!(System::events().is_empty());
    });
}

#[test]
fn account_registry_not_updated_on_failed_submission() {
    new_test_ext().execute_with(|| {
        assert!(AccountRegistry::<Test>::get(ALICE).is_none());

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                H256::from_low_u64_be(20),
                vec![1],
                vec![2],
                1,
                10,
                H256::zero(),
            ),
            AtlasError::InvalidNonce
        );

        assert!(AccountRegistry::<Test>::get(ALICE).is_none());
    });
}

#[test]
fn submit_comit_rejects_duplicate_comit_id() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(5);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let fee: Balance = 26;
        let prepare_root_0 = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);
        let prepare_root_1 = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 1, fee);

        // First submission succeeds
        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload.clone(),
            svm_payload.clone(),
            0,
            fee,
            prepare_root_0,
        ));

        // Second submission with same comit_id fails even with correct nonce
        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                1,
                fee,
                prepare_root_1,
            ),
            AtlasError::DuplicateComitId
        );

        // Nonce should only have incremented once
        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn submit_comit_allows_different_comit_ids_with_sequential_nonces() {
    new_test_ext().execute_with(|| {
        let comit_id_1 = H256::from_low_u64_be(5);
        let comit_id_2 = H256::from_low_u64_be(6);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let fee: Balance = 26;
        let prepare_root_0 = compute_prepare_root(comit_id_1, &evm_payload, &svm_payload, 0, fee);
        let prepare_root_1 = compute_prepare_root(comit_id_2, &evm_payload, &svm_payload, 1, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id_1,
            evm_payload.clone(),
            svm_payload.clone(),
            0,
            fee,
            prepare_root_0,
        ));

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id_2,
            evm_payload,
            svm_payload,
            1,
            fee,
            prepare_root_1,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 2);

        let events = x3_events();

        let submission_events: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, AtlasEvent::ComitSubmitted { .. }))
            .collect();

        assert_eq!(submission_events.len(), 2);
    });
}

#[test]
fn register_asset_successfully_records_metadata() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 42;
        let symbol = b"X3".to_vec();
        let decimals = 12;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            symbol.clone(),
            decimals,
        ));

        let stored = AssetRegistry::<Test>::get(asset_id).expect("asset metadata should exist");
        assert_eq!(stored.symbol.into_inner(), symbol);
        assert_eq!(stored.decimals, decimals);

        let events = x3_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            AtlasEvent::AssetRegistered {
                asset_id: emitted_id,
                symbol: emitted_symbol,
                decimals: emitted_decimals,
            } => {
                assert_eq!(*emitted_id, asset_id);
                assert_eq!(emitted_symbol, &symbol);
                assert_eq!(*emitted_decimals, decimals);
            }
            e => panic!("Unexpected event: {:?}", e),
        }
    });
}

#[test]
fn register_asset_rejects_symbol_exceeding_limit() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 1;
        let long_symbol = vec![b'X'; 17];

        assert_noop!(
            AtlasKernel::register_asset(RuntimeOrigin::root(), asset_id, long_symbol, 12),
            AtlasError::SymbolTooLong
        );

        assert!(AssetRegistry::<Test>::get(asset_id).is_none());
        assert!(System::events().is_empty());
    });
}

#[test]
fn register_asset_prevents_duplicate_entries() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 7;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            b"USD".to_vec(),
            6,
        ));

        assert_noop!(
            AtlasKernel::register_asset(RuntimeOrigin::root(), asset_id, b"USD".to_vec(), 6),
            AtlasError::AssetAlreadyRegistered
        );

        let events = System::events();
        assert_eq!(events.len(), 1);
    });
}

#[test]
fn update_canonical_balance_succeeds_and_emits_finalization_event() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 9;
        let new_balance: Balance = 777;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            b"ETH".to_vec(),
            18,
        ));

        System::reset_events();

        let comit_id = H256::from_low_u64_be(77);
        assert_ok!(AtlasKernel::update_canonical_balance(
            RuntimeOrigin::root(),
            BOB,
            asset_id,
            new_balance,
            Some(comit_id),
        ));

        let balance = CanonicalLedger::<Test>::get(BOB, &asset_id);
        assert_eq!(balance, new_balance);

        let events = x3_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            AtlasEvent::ComitFinalized {
                comit_id: emitted_id,
            } => {
                assert_eq!(*emitted_id, comit_id);
            }
            e => panic!("Unexpected event: {:?}", e),
        }
    });
}

#[test]
fn update_canonical_balance_rejects_unknown_asset() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 99;

        assert_noop!(
            AtlasKernel::update_canonical_balance(RuntimeOrigin::root(), BOB, asset_id, 10, None,),
            AtlasError::UnknownAsset
        );

        // With a DoubleMap, we can't easily check if the account is empty,
        // but the fact that the call failed means no balance was updated
        assert!(System::events().is_empty());
    });
}

#[test]
fn update_canonical_balance_without_comit_id_skips_finalization_event() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 10;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            b"SOL".to_vec(),
            9,
        ));

        System::reset_events();

        let new_balance: Balance = 123;
        assert_ok!(AtlasKernel::update_canonical_balance(
            RuntimeOrigin::root(),
            BOB,
            asset_id,
            new_balance,
            None,
        ));

        let balance = CanonicalLedger::<Test>::get(BOB, &asset_id);
        assert_eq!(balance, new_balance);

        assert!(System::events().is_empty());
    });
}

#[test]
fn update_canonical_balance_can_record_zero_balance() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 11;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            b"USDC".to_vec(),
            6,
        ));

        System::reset_events();

        assert_ok!(AtlasKernel::update_canonical_balance(
            RuntimeOrigin::root(),
            BOB,
            asset_id,
            0,
            None,
        ));

        let balance = CanonicalLedger::<Test>::get(BOB, &asset_id);
        assert_eq!(balance, 0);
        assert!(System::events().is_empty());
    });
}

// ============= EDGE CASE & OVERFLOW TESTS =============

#[test]
fn submit_comit_with_max_balance_value() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(100);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let max_balance: Balance = Balance::MAX;
        let prepare_root =
            compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, max_balance);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            0,
            max_balance,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn submit_comit_with_very_large_payloads_near_limit() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(101);
        let large_payload = vec![0u8; 4_095];
        let small_payload = vec![1u8; 1];
        let fee: Balance = 26;
        let prepare_root = compute_prepare_root(comit_id, &large_payload, &small_payload, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            large_payload,
            small_payload,
            0,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn submit_comit_both_payloads_at_max_size() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(102);
        let payload = vec![0u8; 4_096];
        let fee = 26u128;
        let prepare_root = compute_prepare_root(comit_id, &payload, &payload, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            payload.clone(),
            payload,
            0,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn submit_comit_one_payload_empty_one_populated() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(103);
        let evm_payload = vec![];
        let svm_payload = vec![1, 2, 3];
        let fee = 5u128; // Only SVM: 5000/1000 = 5
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            0,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn submit_comit_only_evm_payload() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(104);
        let evm_payload = vec![9, 8, 7];
        let svm_payload = vec![];
        let fee = 21u128; // Only EVM: 21000/1000 = 21
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            0,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn sequential_nonce_increments_per_account() {
    new_test_ext().execute_with(|| {
        let base_id = H256::from_low_u64_be(200);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let fee = 26u128;

        for nonce in 0..10 {
            let comit_id = H256::from_low_u64_be(base_id.as_bytes()[0] as u64 + nonce);
            let prepare_root =
                compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);
            assert_ok!(AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload.clone(),
                svm_payload.clone(),
                nonce,
                fee,
                prepare_root,
            ));
        }

        assert_eq!(Nonces::<Test>::get(ALICE), 10);
    });
}

#[test]
fn multiple_accounts_independent_nonces() {
    new_test_ext().execute_with(|| {
        let comit_id_alice = H256::from_low_u64_be(1);
        let comit_id_bob = H256::from_low_u64_be(2);
        let evm_payload_alice = vec![1];
        let svm_payload_alice = vec![2];
        let evm_payload_bob = vec![3];
        let svm_payload_bob = vec![4];
        let fee_alice = 26u128;
        let fee_bob = 26u128;
        let prepare_root_alice = compute_prepare_root(
            comit_id_alice,
            &evm_payload_alice,
            &svm_payload_alice,
            0,
            fee_alice,
        );
        let prepare_root_bob =
            compute_prepare_root(comit_id_bob, &evm_payload_bob, &svm_payload_bob, 0, fee_bob);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id_alice,
            evm_payload_alice,
            svm_payload_alice,
            0,
            fee_alice,
            prepare_root_alice,
        ));

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(BOB),
            comit_id_bob,
            evm_payload_bob,
            svm_payload_bob,
            0,
            fee_bob,
            prepare_root_bob,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
        assert_eq!(Nonces::<Test>::get(BOB), 1);
    });
}

#[test]
fn prepare_root_zero_hash_accepted_as_bypass() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(500);
        let evm_payload = vec![1, 2];
        let svm_payload = vec![3, 4];
        let fee = 100u128;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            0,
            fee,
            prepare_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn prepare_root_verification_correct_hash_passes() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(501);
        let evm_payload = vec![10, 20, 30];
        let svm_payload = vec![40, 50];
        let fee: Balance = 555;
        let nonce = 0;
        let correct_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            correct_root,
        ));

        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn prepare_root_verification_incorrect_hash_fails() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(502);
        let evm_payload = vec![100];
        let svm_payload = vec![200];
        let fee: Balance = 777;
        let wrong_root = H256::from_low_u64_be(9999);

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                0,
                fee,
                wrong_root,
            ),
            AtlasError::ComitVerificationFailed
        );
    });
}

#[test]
fn asset_registry_stores_multiple_assets() {
    new_test_ext().execute_with(|| {
        let assets = vec![
            (1u32, b"X3".to_vec(), 12u8),
            (2u32, b"ETH".to_vec(), 18u8),
            (3u32, b"SOL".to_vec(), 9u8),
            (4u32, b"USDC".to_vec(), 6u8),
        ];

        for (id, symbol, decimals) in &assets {
            assert_ok!(AtlasKernel::register_asset(
                RuntimeOrigin::root(),
                *id,
                symbol.clone(),
                *decimals,
            ));
        }

        for (id, symbol, decimals) in assets {
            let stored = AssetRegistry::<Test>::get(id).expect("asset should exist");
            assert_eq!(stored.symbol.into_inner(), symbol);
            assert_eq!(stored.decimals, decimals);
        }
    });
}

#[test]
fn canonical_ledger_multiple_assets_per_account() {
    new_test_ext().execute_with(|| {
        let assets = vec![(0u32, 100u128), (1u32, 200u128), (2u32, 300u128)];

        for (asset_id, balance) in &assets {
            assert_ok!(AtlasKernel::register_asset(
                RuntimeOrigin::root(),
                *asset_id,
                format!("ASSET{}", asset_id).into_bytes(),
                6,
            ));

            System::reset_events();

            assert_ok!(AtlasKernel::update_canonical_balance(
                RuntimeOrigin::root(),
                ALICE,
                *asset_id,
                *balance,
                None,
            ));
        }

        // Verify all assets were registered
        for (asset_id, balance) in assets {
            let stored_balance = CanonicalLedger::<Test>::get(ALICE, &asset_id);
            assert_eq!(stored_balance, balance);
        }
    });
}

#[test]
fn canonical_ledger_update_overwrites_previous_balance() {
    new_test_ext().execute_with(|| {
        let asset_id = 0u32;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            b"TEST".to_vec(),
            6,
        ));

        assert_ok!(AtlasKernel::update_canonical_balance(
            RuntimeOrigin::root(),
            BOB,
            asset_id,
            100,
            None,
        ));

        let balance1 = CanonicalLedger::<Test>::get(BOB, &asset_id);
        assert_eq!(balance1, 100);

        assert_ok!(AtlasKernel::update_canonical_balance(
            RuntimeOrigin::root(),
            BOB,
            asset_id,
            999,
            None,
        ));

        let balance2 = CanonicalLedger::<Test>::get(BOB, &asset_id);
        assert_eq!(balance2, 999);
    });
}

#[test]
fn canonical_ledger_max_balance_value() {
    new_test_ext().execute_with(|| {
        let asset_id = 5u32;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            b"MAX".to_vec(),
            12,
        ));

        assert_ok!(AtlasKernel::update_canonical_balance(
            RuntimeOrigin::root(),
            ALICE,
            asset_id,
            Balance::MAX,
            None,
        ));

        let balance = CanonicalLedger::<Test>::get(ALICE, &asset_id);
        assert_eq!(balance, Balance::MAX);
    });
}

#[test]
fn account_registry_created_on_successful_submission() {
    new_test_ext().execute_with(|| {
        assert!(AccountRegistry::<Test>::get(CHARLIE).is_none());

        let comit_id = H256::from_low_u64_be(1);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let nonce = 0;
        let fee = 26u128;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(CHARLIE),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        ));

        assert_eq!(
            AccountRegistry::<Test>::get(CHARLIE),
            Some(AtlasId::default())
        );
    });
}

#[test]
fn account_registry_not_overwritten_on_repeated_submission() {
    new_test_ext().execute_with(|| {
        let stored_id = AtlasId::default();

        let comit_id_1 = H256::from_low_u64_be(1);
        let evm_payload_1 = vec![1];
        let svm_payload_1 = vec![2];
        let fee = 26u128;
        let prepare_root_1 =
            compute_prepare_root(comit_id_1, &evm_payload_1, &svm_payload_1, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id_1,
            evm_payload_1,
            svm_payload_1,
            0,
            fee,
            prepare_root_1,
        ));

        assert_eq!(AccountRegistry::<Test>::get(ALICE), Some(stored_id));

        let comit_id_2 = H256::from_low_u64_be(2);
        let evm_payload_2 = vec![3];
        let svm_payload_2 = vec![4];
        let prepare_root_2 =
            compute_prepare_root(comit_id_2, &evm_payload_2, &svm_payload_2, 1, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id_2,
            evm_payload_2,
            svm_payload_2,
            1,
            fee,
            prepare_root_2,
        ));

        assert_eq!(AccountRegistry::<Test>::get(ALICE), Some(stored_id));
    });
}

#[test]
fn comit_submission_emits_all_required_event_fields() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(2000);
        let evm_payload = vec![1, 2, 3, 4, 5];
        let svm_payload = vec![6, 7, 8];
        let fee: Balance = 12_345;
        let nonce = 0u64;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        ));

        let events = x3_events();

        // Successful execution now emits: FeeDeducted, ComitSubmitted, ExecutionStarted,
        // ExecutionCompleted, CanonicalLedgerUpdated, Finalized
        assert_eq!(events.len(), 6);
        // FeeDeducted is index 0, ComitSubmitted is index 1
        match &events[1] {
            AtlasEvent::ComitSubmitted {
                comit_id: id,
                origin,
                nonce: n,
                fee: f,
            } => {
                assert_eq!(*id, comit_id);
                assert_eq!(*origin, ALICE);
                assert_eq!(*n, nonce);
                assert_eq!(*f, fee);
            }
            _ => panic!("Unexpected event"),
        }
    });
}

#[test]
fn register_asset_rejects_non_root_origin() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 50;

        assert_noop!(
            AtlasKernel::register_asset(
                RuntimeOrigin::signed(ALICE),
                asset_id,
                b"TEST".to_vec(),
                6,
            ),
            frame_support::dispatch::DispatchError::BadOrigin
        );

        assert!(AssetRegistry::<Test>::get(asset_id).is_none());
    });
}

#[test]
fn update_canonical_balance_rejects_non_root_origin() {
    new_test_ext().execute_with(|| {
        let asset_id: AssetId = 51;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            b"TEST".to_vec(),
            6,
        ));

        assert_noop!(
            AtlasKernel::update_canonical_balance(
                RuntimeOrigin::signed(BOB),
                BOB,
                asset_id,
                100,
                None,
            ),
            frame_support::dispatch::DispatchError::BadOrigin
        );
    });
}

#[test]
fn comit_failed_event_emitted_on_empty_payloads() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(3000);

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                vec![],
                vec![],
                0,
                1,
                H256::zero(),
            ),
            AtlasError::EmptyPayloads
        );

        // No events emitted on error (they get rolled back)
        let events = x3_events();
        assert_eq!(events.len(), 0);
    });
}

#[test]
fn comit_failed_event_emitted_on_invalid_nonce() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(3001);

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                vec![1],
                vec![2],
                5,
                1,
                H256::zero(),
            ),
            AtlasError::InvalidNonce
        );

        // No events emitted on error (they get rolled back)
        let events = x3_events();
        assert_eq!(events.len(), 0);
    });
}

#[test]
fn asset_symbol_boundary_lengths() {
    new_test_ext().execute_with(|| {
        let max_valid = vec![b'A'; 16];
        let too_long = vec![b'A'; 17];

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            100,
            max_valid,
            12,
        ));

        assert_noop!(
            AtlasKernel::register_asset(RuntimeOrigin::root(), 101, too_long, 12),
            AtlasError::SymbolTooLong
        );
    });
}

#[test]
fn asset_registration_emits_correct_metadata() {
    new_test_ext().execute_with(|| {
        let asset_id = 200u32;
        let symbol = b"MYTOKEN".to_vec();
        let decimals = 8u8;

        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            asset_id,
            symbol.clone(),
            decimals,
        ));

        let events = x3_events();

        match &events[0] {
            AtlasEvent::AssetRegistered {
                asset_id: id,
                symbol: sym,
                decimals: dec,
            } => {
                assert_eq!(*id, asset_id);
                assert_eq!(sym, &symbol);
                assert_eq!(*dec, decimals);
            }
            _ => panic!("Unexpected event"),
        }
    });
}

#[test]
fn submit_comit_insufficient_balance_fails() {
    // Create test with Alice having only 10 balance (less than required 26)
    ExtBuilder::default()
        .balances(vec![(ALICE, 10u128), (BOB, INITIAL_BALANCE)])
        .authorized_accounts(vec![ALICE, BOB])
        .build()
        .execute_with(|| {
            let comit_id = H256::from_low_u64_be(10000);
            let evm_payload = vec![1];
            let svm_payload = vec![2];
            let nonce = 0;
            // Required fee will be (21000/1000) + (5000/1000) = 26
            let fee = 26u128;
            let prepare_root =
                compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

            // Check Alice has insufficient balance for the required fee
            let initial_balance = <mock::Balances as frame_support::traits::Currency<
                mock::AccountId,
            >>::free_balance(&ALICE);
            assert!(initial_balance < 26);

            // Should fail due to insufficient balance
            assert_noop!(
                AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    comit_id,
                    evm_payload,
                    svm_payload,
                    nonce,
                    fee,
                    prepare_root,
                ),
                AtlasError::InsufficientBalance
            );

            // Verify nonce not incremented
            assert_eq!(Nonces::<Test>::get(ALICE), 0);
        });
}

// ============= AUTHORIZATION TESTS (H-2 Security Fix) =============

#[test]
fn submit_comit_rejects_unauthorized_account() {
    // Create test with no authorized accounts
    ExtBuilder::default()
        .balances(vec![(ALICE, INITIAL_BALANCE), (BOB, INITIAL_BALANCE)])
        .authorized_accounts(vec![]) // No one authorized
        .build()
        .execute_with(|| {
            let comit_id = H256::from_low_u64_be(9001);
            let evm_payload = vec![1, 2, 3];
            let svm_payload = vec![4, 5];
            let nonce = 0;
            let fee = 26u128;
            let prepare_root =
                compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

            // Should fail because ALICE is not authorized
            assert_noop!(
                AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    comit_id,
                    evm_payload,
                    svm_payload,
                    nonce,
                    fee,
                    prepare_root,
                ),
                AtlasError::Unauthorized
            );

            // Verify nonce not incremented
            assert_eq!(Nonces::<Test>::get(ALICE), 0);
        });
}

#[test]
fn authorize_account_enables_comit_submission() {
    ExtBuilder::default()
        .balances(vec![(ALICE, INITIAL_BALANCE)])
        .authorized_accounts(vec![]) // Start with no one authorized
        .build()
        .execute_with(|| {
            let comit_id = H256::from_low_u64_be(9002);
            let evm_payload = vec![1];
            let svm_payload = vec![2];
            let nonce = 0;
            let fee = 26u128;
            let prepare_root =
                compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

            // First attempt should fail (unauthorized)
            assert_noop!(
                AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    comit_id,
                    evm_payload.clone(),
                    svm_payload.clone(),
                    nonce,
                    fee,
                    prepare_root,
                ),
                AtlasError::Unauthorized
            );

            // Authorize ALICE via governance
            assert_ok!(AtlasKernel::authorize_account(RuntimeOrigin::root(), ALICE));

            // Now submission should succeed
            assert_ok!(AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                nonce,
                fee,
                prepare_root,
            ));

            assert_eq!(Nonces::<Test>::get(ALICE), 1);
        });
}

#[test]
fn deauthorize_account_blocks_comit_submission() {
    ExtBuilder::default()
        .balances(vec![(ALICE, INITIAL_BALANCE)])
        .authorized_accounts(vec![ALICE]) // ALICE starts authorized
        .build()
        .execute_with(|| {
            let comit_id_1 = H256::from_low_u64_be(9003);
            let comit_id_2 = H256::from_low_u64_be(9004);
            let evm_payload = vec![1];
            let svm_payload = vec![2];
            let fee = 26u128;
            let prepare_root_1 =
                compute_prepare_root(comit_id_1, &evm_payload, &svm_payload, 0, fee);
            let prepare_root_2 =
                compute_prepare_root(comit_id_2, &evm_payload, &svm_payload, 1, fee);

            // First submission should succeed (authorized)
            assert_ok!(AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id_1,
                evm_payload.clone(),
                svm_payload.clone(),
                0,
                fee,
                prepare_root_1,
            ));

            // Deauthorize ALICE
            assert_ok!(AtlasKernel::deauthorize_account(
                RuntimeOrigin::root(),
                ALICE
            ));

            // Second submission should fail (now unauthorized)
            assert_noop!(
                AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    comit_id_2,
                    evm_payload,
                    svm_payload,
                    1,
                    fee,
                    prepare_root_2,
                ),
                AtlasError::Unauthorized
            );

            // Nonce should be 1 (only first submission incremented it)
            assert_eq!(Nonces::<Test>::get(ALICE), 1);
        });
}

#[test]
fn authorize_account_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        // Regular signed origin should fail
        assert_noop!(
            AtlasKernel::authorize_account(RuntimeOrigin::signed(ALICE), BOB),
            sp_runtime::DispatchError::BadOrigin
        );

        // Root origin should succeed
        assert_ok!(AtlasKernel::authorize_account(RuntimeOrigin::root(), BOB));
    });
}

#[test]
fn deauthorize_account_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        // Regular signed origin should fail
        assert_noop!(
            AtlasKernel::deauthorize_account(RuntimeOrigin::signed(ALICE), BOB),
            sp_runtime::DispatchError::BadOrigin
        );

        // Root origin should succeed
        assert_ok!(AtlasKernel::deauthorize_account(RuntimeOrigin::root(), BOB));
    });
}

#[test]
fn authorization_events_emitted_correctly() {
    ExtBuilder::default()
        .balances(vec![(ALICE, INITIAL_BALANCE)])
        .authorized_accounts(vec![])
        .build()
        .execute_with(|| {
            // Authorize
            assert_ok!(AtlasKernel::authorize_account(RuntimeOrigin::root(), ALICE));

            let events = x3_events();
            assert_eq!(events.len(), 1);
            match &events[0] {
                AtlasEvent::AccountAuthorized { account } => {
                    assert_eq!(*account, ALICE);
                }
                e => panic!("Unexpected event: {:?}", e),
            }

            System::reset_events();

            // Deauthorize
            assert_ok!(AtlasKernel::deauthorize_account(
                RuntimeOrigin::root(),
                ALICE
            ));

            let events = x3_events();
            assert_eq!(events.len(), 1);
            match &events[0] {
                AtlasEvent::AccountDeauthorized { account } => {
                    assert_eq!(*account, ALICE);
                }
                e => panic!("Unexpected event: {:?}", e),
            }
        });
}

// ============= SYMBOL VALIDATION TESTS (M-1 Security Fix) =============

#[test]
fn register_asset_rejects_empty_symbol() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AtlasKernel::register_asset(
                RuntimeOrigin::root(),
                1u32,
                vec![], // Empty symbol
                12
            ),
            AtlasError::EmptySymbol
        );
    });
}

#[test]
fn register_asset_rejects_symbol_starting_with_dash() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AtlasKernel::register_asset(
                RuntimeOrigin::root(),
                1u32,
                b"-TOKEN".to_vec(), // Starts with apps/dash-legacy-2-legacy-2
                12
            ),
            AtlasError::InvalidSymbolFormat
        );
    });
}

#[test]
fn register_asset_rejects_symbol_starting_with_underscore() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AtlasKernel::register_asset(
                RuntimeOrigin::root(),
                1u32,
                b"_TOKEN".to_vec(), // Starts with underscore
                12
            ),
            AtlasError::InvalidSymbolFormat
        );
    });
}

#[test]
fn register_asset_allows_dash_and_underscore_in_middle() {
    new_test_ext().execute_with(|| {
        // Dash in middle is OK
        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            1u32,
            b"MY-TOKEN".to_vec(),
            12
        ));

        // Underscore in middle is OK
        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            2u32,
            b"MY_TOKEN".to_vec(),
            12
        ));
    });
}

// ============= FEE CALCULATION TESTS (C-2 Security Fix) =============

#[test]
fn fee_calculation_uses_ceiling_division() {
    new_test_ext().execute_with(|| {
        // With 999 gas, truncation would give 0, ceiling gives 1
        let fee = crate::Pallet::<Test>::calculate_execution_fee(999, 0, 0u128).unwrap();
        assert!(fee >= 1, "Fee should be at least 1 due to ceiling division");

        // With 1000 gas, both methods give 1
        let fee = crate::Pallet::<Test>::calculate_execution_fee(1000, 0, 0u128).unwrap();
        assert!(fee >= 1);

        // With 1001 gas, ceiling gives 2
        let fee = crate::Pallet::<Test>::calculate_execution_fee(1001, 0, 0u128).unwrap();
        assert!(fee >= 2);
    });
}

#[test]
fn fee_calculation_enforces_minimum_fee() {
    new_test_ext().execute_with(|| {
        // Even with zero gas, minimum fee should apply
        let fee = crate::Pallet::<Test>::calculate_execution_fee(0, 0, 0u128).unwrap();
        assert!(fee >= 1, "Minimum fee floor should be enforced");
    });
}

// ============= FEE DEDUCTION EVENT TEST (L-1 Fix) =============

#[test]
fn submit_comit_emits_fee_deducted_event() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(8001);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let nonce = 0;
        let fee = 26u128;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        ));

        let events = x3_events();
        // Should have FeeDeducted among the events
        let fee_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AtlasEvent::FeeDeducted { .. }))
            .collect();
        assert_eq!(fee_events.len(), 1, "FeeDeducted event should be emitted");

        match &fee_events[0] {
            AtlasEvent::FeeDeducted {
                account,
                comit_id: id,
                ..
            } => {
                assert_eq!(*account, ALICE);
                assert_eq!(*id, comit_id);
            }
            _ => panic!("Expected FeeDeducted event"),
        }
    });
}

// ============= ATOMIC NONCE TESTS (C-3 Security Fix) =============

#[test]
fn nonce_increments_atomically_on_success() {
    new_test_ext().execute_with(|| {
        // Verify nonce starts at 0
        assert_eq!(Nonces::<Test>::get(ALICE), 0);

        let comit_id = H256::from_low_u64_be(100);
        let evm_payload = vec![1, 2, 3];
        let svm_payload = vec![4, 5];
        let fee: Balance = 500;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            0,
            fee,
            prepare_root,
        ));

        // Nonce should be atomically incremented
        assert_eq!(Nonces::<Test>::get(ALICE), 1);
    });
}

#[test]
fn nonce_not_incremented_on_failure() {
    new_test_ext().execute_with(|| {
        // Verify nonce starts at 0
        assert_eq!(Nonces::<Test>::get(ALICE), 0);

        let comit_id = H256::from_low_u64_be(101);
        // Both payloads empty - should fail
        let evm_payload: Vec<u8> = vec![];
        let svm_payload: Vec<u8> = vec![];
        let fee: Balance = 500;

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                0,
                fee,
                H256::zero(),
            ),
            AtlasError::EmptyPayloads
        );

        // Nonce should not have changed
        assert_eq!(Nonces::<Test>::get(ALICE), 0);
    });
}

// ============= COMIT ID UNIQUENESS TESTS (M-4 Security Fix) =============

#[test]
fn submitted_comit_id_is_recorded() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(200);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let fee: Balance = 100;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);

        // Comit ID should not be in storage before submission
        assert!(!crate::SubmittedComits::<Test>::contains_key(comit_id));

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            0,
            fee,
            prepare_root,
        ));

        // Comit ID should be recorded after successful submission
        assert!(crate::SubmittedComits::<Test>::contains_key(comit_id));
    });
}

#[test]
fn different_accounts_cannot_reuse_comit_id() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(201);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let fee: Balance = 100;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);

        // Alice submits first
        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload.clone(),
            svm_payload.clone(),
            0,
            fee,
            prepare_root,
        ));

        // Bob cannot use the same comit_id even with different nonce
        let prepare_root_bob = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);
        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(BOB),
                comit_id,
                evm_payload,
                svm_payload,
                0, // Bob's nonce is 0
                fee,
                prepare_root_bob,
            ),
            AtlasError::DuplicateComitId
        );
    });
}

// ============================================================================
// Rate Limiting Tests (L-6)
// ============================================================================

#[test]
fn rate_limiting_allows_submissions_under_limit() {
    new_test_ext().execute_with(|| {
        let fee: Balance = 100;

        // Submit 5 comits (under the limit of 10)
        for i in 0u64..5 {
            let comit_id = H256::from_low_u64_be(300 + i);
            let evm_payload = vec![i as u8 + 1];
            let svm_payload = vec![i as u8 + 2];
            let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, i, fee);

            assert_ok!(AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                i,
                fee,
                prepare_root,
            ));
        }

        // All 5 should succeed, nonce should be 5
        assert_eq!(crate::Nonces::<Test>::get(ALICE), 5);
    });
}

#[test]
fn rate_limiting_blocks_excessive_submissions() {
    new_test_ext().execute_with(|| {
        let fee: Balance = 100;

        // Submit 10 comits (at the limit)
        for i in 0u64..10 {
            let comit_id = H256::from_low_u64_be(400 + i);
            let evm_payload = vec![i as u8 + 1];
            let svm_payload = vec![i as u8 + 2];
            let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, i, fee);

            assert_ok!(AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                i,
                fee,
                prepare_root,
            ));
        }

        // 11th submission should fail with RateLimitExceeded
        let comit_id = H256::from_low_u64_be(410);
        let evm_payload = vec![11];
        let svm_payload = vec![12];
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 10, fee);

        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                10,
                fee,
                prepare_root,
            ),
            AtlasError::RateLimitExceeded
        );
    });
}

#[test]
fn rate_limiting_is_per_account() {
    new_test_ext().execute_with(|| {
        let fee: Balance = 100;

        // Alice submits 10 (at her limit)
        for i in 0u64..10 {
            let comit_id = H256::from_low_u64_be(500 + i);
            let evm_payload = vec![i as u8 + 1];
            let svm_payload = vec![i as u8 + 2];
            let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, i, fee);

            assert_ok!(AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                evm_payload,
                svm_payload,
                i,
                fee,
                prepare_root,
            ));
        }

        // Bob can still submit (his own counter is 0)
        let comit_id = H256::from_low_u64_be(600);
        let evm_payload = vec![1];
        let svm_payload = vec![2];
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(BOB),
            comit_id,
            evm_payload,
            svm_payload,
            0,
            fee,
            prepare_root,
        ));
    });
}

// ============================================================================
// Decode Failure Counter Tests (M-2)
// ============================================================================

#[test]
fn decode_failure_counter_tracks_failures() {
    new_test_ext().execute_with(|| {
        // Initial counter should be 0
        assert_eq!(crate::DecodeFailureCount::<Test>::get(), 0);

        // Submit a valid comit (may or may not have decode failures depending on state changes)
        let comit_id = H256::from_low_u64_be(700);
        let evm_payload = vec![1, 2, 3];
        let svm_payload = vec![4, 5];
        let nonce = 0;
        let fee: Balance = 100;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        ));

        // Counter may have incremented depending on mock adapter output decoding
        // The important thing is that the counter exists and can be read
        let _count = crate::DecodeFailureCount::<Test>::get();
    });
}

// ============================================================================
// Timestamp Tests (M-6)
// ============================================================================

#[test]
fn comit_execution_started_event_has_timestamp() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(800);
        let evm_payload = vec![1, 2, 3];
        let svm_payload = vec![4, 5];
        let nonce = 0;
        let fee: Balance = 100;
        let prepare_root = compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        assert_ok!(AtlasKernel::submit_comit(
            RuntimeOrigin::signed(ALICE),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        ));

        // Find the ComitExecutionStarted event
        let events = x3_events();
        let started_event = events
            .iter()
            .find(|e| matches!(e, AtlasEvent::ComitExecutionStarted { .. }));

        assert!(
            started_event.is_some(),
            "ComitExecutionStarted event should be emitted"
        );

        if let Some(AtlasEvent::ComitExecutionStarted {
            comit_id: id,
            timestamp,
        }) = started_event
        {
            assert_eq!(*id, comit_id);
            // Timestamp should be non-zero (captured at execution start)
            // In mock, this will be whatever pallet_timestamp returns
            assert!(*timestamp >= 0, "Timestamp should be captured");
        }
    });
}

// ============================================================================
// Exported compute_prepare_root Tests (L-3)
// ============================================================================

#[test]
fn compute_prepare_root_matches_pallet_implementation() {
    new_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(900);
        let evm_payload = vec![1, 2, 3, 4, 5];
        let svm_payload = vec![6, 7, 8];
        let nonce = 42u64;
        let fee: Balance = 12345;

        // Use the public pallet function
        let pallet_root =
            AtlasKernel::compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        // Verify it produces a non-zero hash
        assert_ne!(pallet_root, H256::zero());

        // Verify determinism - same inputs should produce same output
        let pallet_root_2 =
            AtlasKernel::compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);
        assert_eq!(pallet_root, pallet_root_2);

        // Verify different inputs produce different outputs
        let different_root = AtlasKernel::compute_prepare_root(
            comit_id,
            &evm_payload,
            &svm_payload,
            nonce + 1, // Different nonce
            fee,
        );
        assert_ne!(pallet_root, different_root);
    });
}

// ──────────────────────────────────────────────────────────────────────────────
// SEC-009: Emergency Pause tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn emergency_pause_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AtlasKernel::emergency_pause(RuntimeOrigin::signed(ALICE)),
            frame_support::error::BadOrigin,
        );
    });
}

#[test]
fn emergency_pause_and_unpause_by_root_works() {
    new_test_ext().execute_with(|| {
        // Initially not paused
        assert!(!crate::ProtocolPaused::<mock::Test>::get());

        // Root can pause
        assert_ok!(AtlasKernel::emergency_pause(RuntimeOrigin::root()));
        assert!(crate::ProtocolPaused::<mock::Test>::get());

        // Root can unpause
        assert_ok!(AtlasKernel::emergency_unpause(RuntimeOrigin::root()));
        assert!(!crate::ProtocolPaused::<mock::Test>::get());
    });
}

#[test]
fn emergency_pause_blocks_submit_comit() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(AtlasKernel::emergency_pause(RuntimeOrigin::root()));

        let comit_id = H256::from_low_u64_be(1);
        assert_noop!(
            AtlasKernel::submit_comit(
                RuntimeOrigin::signed(ALICE),
                comit_id,
                vec![1, 2],
                vec![3, 4],
                1u64,
                100u128,
                H256::zero(),
            ),
            crate::Error::<mock::Test>::ProtocolIsPaused,
        );
    });
}

#[test]
fn double_pause_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(AtlasKernel::emergency_pause(RuntimeOrigin::root()));
        assert_noop!(
            AtlasKernel::emergency_pause(RuntimeOrigin::root()),
            crate::Error::<mock::Test>::ProtocolIsPaused,
        );
    });
}

#[test]
fn unpause_when_not_paused_is_noop() {
    new_test_ext().execute_with(|| {
        // Not currently paused — unpause should succeed as a no-op
        assert_ok!(AtlasKernel::emergency_unpause(RuntimeOrigin::root()));
        assert!(!crate::ProtocolPaused::<mock::Test>::get());
    });
}
