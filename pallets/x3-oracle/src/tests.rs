//! Unit tests for pallet-x3-oracle

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

#[test]
fn authorize_oracle_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));
        assert!(Oracle::is_authorized_oracle(1));
        System::assert_has_event(RuntimeEvent::Oracle(Event::OracleAuthorized { account: 1 }));
    });
}

#[test]
fn authorize_oracle_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Oracle::authorize_oracle(RuntimeOrigin::signed(1), 2),
            BadOrigin
        );
    });
}

#[test]
fn deauthorize_oracle_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));
        assert_ok!(Oracle::deauthorize_oracle(RuntimeOrigin::root(), 1));
        assert!(!Oracle::is_authorized_oracle(1));
        System::assert_has_event(RuntimeEvent::Oracle(Event::OracleDeauthorized {
            account: 1,
        }));
    });
}

#[test]
fn submit_price_requires_authorized_oracle() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Oracle::submit_price(RuntimeOrigin::signed(1), 1, 1000),
            Error::<Test>::NotAuthorizedOracle
        );
    });
}

#[test]
fn submit_price_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));

        assert_ok!(Oracle::submit_price(RuntimeOrigin::signed(1), 1, 1000));

        // Check that price was submitted
        assert!(PriceSubmissions::<Test>::contains_key(1, 1));

        System::assert_has_event(RuntimeEvent::Oracle(Event::PriceSubmitted {
            asset_id: 1,
            oracle: 1,
            price: 1000,
            block: 1,
        }));
    });
}

#[test]
fn submit_price_rejects_zero_price() {
    new_test_ext().execute_with(|| {
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));
        assert_noop!(
            Oracle::submit_price(RuntimeOrigin::signed(1), 1, 0),
            Error::<Test>::InvalidPrice
        );
    });
}

#[test]
fn submit_price_rate_limit_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));

        // Submit maximum allowed per block
        for i in 0..MaxSubmissionsPerBlock::get() {
            assert_ok!(Oracle::submit_price(
                RuntimeOrigin::signed(1),
                i as u32,
                1000 + i as u64
            ));
        }

        // Next submission should be rate limited
        assert_noop!(
            Oracle::submit_price(RuntimeOrigin::signed(1), 99, 2000),
            Error::<Test>::SubmissionRateLimitExceeded
        );
    });
}

#[test]
fn median_price_calculation_works() {
    new_test_ext().execute_with(|| {
        // Authorize multiple oracles
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 2));
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 3));

        // Submit prices: 100, 200, 300 (median should be 200)
        assert_ok!(Oracle::submit_price(RuntimeOrigin::signed(1), 1, 100));
        assert_ok!(Oracle::submit_price(RuntimeOrigin::signed(2), 1, 200));
        assert_ok!(Oracle::submit_price(RuntimeOrigin::signed(3), 1, 300));

        // Check median was calculated
        let price_data = Oracle::get_price(1).unwrap();
        assert_eq!(price_data.price, 200);
        assert_eq!(price_data.submission_count, 3);

        System::assert_has_event(RuntimeEvent::Oracle(Event::PriceUpdated {
            asset_id: 1,
            price: 200,
            submission_count: 3,
            median_block: 1,
        }));
    });
}

#[test]
fn median_price_requires_minimum_submissions() {
    new_test_ext().execute_with(|| {
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 2));

        // Only 2 submissions, minimum is 3
        assert_ok!(Oracle::submit_price(RuntimeOrigin::signed(1), 1, 100));
        assert_ok!(Oracle::submit_price(RuntimeOrigin::signed(2), 1, 200));

        // No median should be calculated
        assert!(Oracle::get_price(1).is_none());
    });
}

#[test]
fn cleanup_old_submissions_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Oracle::authorize_oracle(RuntimeOrigin::root(), 1));

        // Submit a price
        assert_ok!(Oracle::submit_price(RuntimeOrigin::signed(1), 1, 1000));

        // Manually set timestamp to be old (simulate time passing)
        PriceSubmissions::<Test>::mutate(1, 1, |submission| {
            if let Some(s) = submission {
                s.timestamp = 0; // Very old timestamp
            }
        });

        // Cleanup with max age of 100 seconds
        let removed = Oracle::cleanup_old_submissions(1, 100);
        assert_eq!(removed, 1);

        // Submission should be gone
        assert!(!PriceSubmissions::<Test>::contains_key(1, 1));
    });
}
