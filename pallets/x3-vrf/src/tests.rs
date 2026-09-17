//! Unit tests for pallet-x3-vrf

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::traits::BadOrigin;

#[test]
fn request_randomness_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let seed = vec![1, 2, 3, 4];
        let max_fee = 1000;

        assert_ok!(Vrf::request_randomness(
            RuntimeOrigin::signed(1),
            seed,
            max_fee
        ));

        let requests = Vrf::account_requests(1);
        assert_eq!(requests.len(), 1);

        let request_id = requests[0];
        let request = Vrf::pending_requests(request_id).unwrap();
        assert_eq!(request.block_number, 1);

        System::assert_has_event(RuntimeEvent::Vrf(Event::RandomnessRequested {
            request_id,
            requester: 1,
            fee: 104,
        }));
    });
}

#[test]
fn request_randomness_requires_signed_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Vrf::request_randomness(RuntimeOrigin::none(), vec![], 100),
            BadOrigin
        );
    });
}

#[test]
fn request_randomness_insufficient_balance() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Vrf::request_randomness(RuntimeOrigin::signed(999), vec![], 1000),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn request_randomness_too_many_pending() {
    new_test_ext().execute_with(|| {
        for i in 0..MaxPendingRequests::get() {
            assert_ok!(Vrf::request_randomness(
                RuntimeOrigin::signed(1),
                vec![i as u8],
                1000
            ));
        }
        assert_noop!(
            Vrf::request_randomness(RuntimeOrigin::signed(1), vec![255], 1000),
            Error::<Test>::TooManyPendingRequests
        );
    });
}

#[test]
fn request_randomness_seed_too_long() {
    new_test_ext().execute_with(|| {
        let long_seed = vec![0u8; (MaxSeedLength::get() + 1) as usize];
        assert_noop!(
            Vrf::request_randomness(RuntimeOrigin::signed(1), long_seed, 10000),
            Error::<Test>::SeedTooLong
        );
    });
}

#[test]
fn fulfill_randomness_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Vrf::request_randomness(
            RuntimeOrigin::signed(1), vec![42], 1000
        ));
        let request_id = Vrf::account_requests(1)[0];

        assert_ok!(Vrf::fulfill_randomness(
            RuntimeOrigin::signed(2), request_id
        ));

        let result = Vrf::fulfilled_requests(request_id).unwrap();
        assert_eq!(result.request_id, request_id);
        assert!(Vrf::pending_requests(request_id).is_none());

        System::assert_has_event(RuntimeEvent::Vrf(Event::RandomnessFulfilled {
            request_id,
            randomness: result.randomness,
        }));
    });
}

#[test]
fn fulfill_randomness_not_found() {
    new_test_ext().execute_with(|| {
        let fake_request_id = H256::from([1; 32]);
        assert_noop!(
            Vrf::fulfill_randomness(RuntimeOrigin::signed(2), fake_request_id),
            Error::<Test>::RequestNotFound
        );
    });
}

#[test]
fn fulfill_randomness_already_fulfilled() {
    new_test_ext().execute_with(|| {
        assert_ok!(Vrf::request_randomness(
            RuntimeOrigin::signed(1), vec![42], 1000
        ));
        let request_id = Vrf::account_requests(1)[0];
        assert_ok!(Vrf::fulfill_randomness(
            RuntimeOrigin::signed(2), request_id
        ));

        assert_noop!(
            Vrf::fulfill_randomness(RuntimeOrigin::signed(2), request_id),
            Error::<Test>::RequestAlreadyFulfilled
        );
    });
}

#[test]
fn cancel_randomness_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Vrf::request_randomness(
            RuntimeOrigin::signed(1), vec![42], 1000
        ));
        let request_id = Vrf::account_requests(1)[0];

        assert_ok!(Vrf::cancel_randomness(RuntimeOrigin::signed(1), request_id));

        assert!(Vrf::pending_requests(request_id).is_none());
        assert_eq!(Vrf::account_requests(1).len(), 0);

        System::assert_has_event(RuntimeEvent::Vrf(Event::RandomnessCancelled {
            request_id,
            requester: 1,
        }));
    });
}

#[test]
fn cancel_randomness_not_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(Vrf::request_randomness(
            RuntimeOrigin::signed(1), vec![42], 1000
        ));
        let request_id = Vrf::account_requests(1)[0];

        assert_noop!(
            Vrf::cancel_randomness(RuntimeOrigin::signed(2), request_id),
            Error::<Test>::NotRequestOwner
        );
    });
}

#[test]
fn cancel_randomness_already_fulfilled() {
    new_test_ext().execute_with(|| {
        assert_ok!(Vrf::request_randomness(
            RuntimeOrigin::signed(1), vec![42], 1000
        ));
        let request_id = Vrf::account_requests(1)[0];
        assert_ok!(Vrf::fulfill_randomness(
            RuntimeOrigin::signed(2), request_id
        ));

        assert_noop!(
            Vrf::cancel_randomness(RuntimeOrigin::signed(1), request_id),
            Error::<Test>::RequestAlreadyFulfilled
        );
    });
}

#[test]
fn get_randomness_public_function() {
    new_test_ext().execute_with(|| {
        let fake_id = H256::from([1; 32]);
        assert!(Vrf::get_randomness(fake_id).is_none());

        assert_ok!(Vrf::request_randomness(
            RuntimeOrigin::signed(1), vec![42], 1000
        ));
        let request_id = Vrf::account_requests(1)[0];
        assert_ok!(Vrf::fulfill_randomness(
            RuntimeOrigin::signed(2), request_id
        ));

        assert!(Vrf::get_randomness(request_id).is_some());
    });
}