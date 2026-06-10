use super::mock::*;
use crate::pallet::{Error, Event};
use crate::{
    ExternalChainId, GatewayMode, GatewayTransferStatus, Pallet, RouteConfig,
    RouteVerificationLevel, X3Domain,
};
use frame_support::{assert_noop, assert_ok, BoundedVec};

fn expected_withdrawal_id(amount: u128) -> [u8; 32] {
    // Must mirror the pallet's derive_withdrawal_id exactly: it
    // mixes x3_asset_id, recipient bytes, amount, and the current
    // block (which is 1 in new_test_ext).
    let mut out = [9u8; 32];
    for (idx, byte) in b"0xRECIPIENT".iter().enumerate() {
        out[idx % 32] ^= *byte;
    }
    for (idx, byte) in amount.to_be_bytes().iter().enumerate() {
        out[idx] ^= *byte;
    }
    for (idx, byte) in 1u64.to_be_bytes().iter().enumerate() {
        out[idx] ^= *byte;
    }
    out
}

fn register_and_enable() {
    assert_ok!(X3CrosschainGateway::register_asset(
        RuntimeOrigin::root(),
        ExternalChainId::BaseSepolia,
        bounded("0xTOKEN"),
        [9u8; 32],
    ));
    assert_ok!(X3CrosschainGateway::enable_route(
        RuntimeOrigin::root(),
        route(),
    ));
}

#[test]
fn register_asset_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3CrosschainGateway::register_asset(
            RuntimeOrigin::root(),
            ExternalChainId::BaseSepolia,
            bounded("0xTOKEN"),
            [9u8; 32],
        ));
    });
}

#[test]
fn enable_route_requires_registered_asset() {
    new_test_ext().execute_with(|| {
        // Asset not registered yet — should fail.
        assert_noop!(
            X3CrosschainGateway::enable_route(RuntimeOrigin::root(), route()),
            Error::<Test>::AssetNotRegistered
        );
    });
}

#[test]
fn submit_deposit_proof_happy_path() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        let proof_id = [1u8; 32];
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof(proof_id, 100),
        ));
        // ExternalLocked is updated; route's daily counter is updated.
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 100);
        // Transfer exists and is verified.
        let t = Pallet::<Test>::transfers(proof_id).expect("transfer should exist");
        assert_eq!(t.amount, 100);
        assert_eq!(t.status, GatewayTransferStatus::Verified);
    });
}

#[test]
fn submit_deposit_proof_replay_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        let proof_id = [1u8; 32];
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof(proof_id, 100),
        ));
        // Replay of the same proof_id fails.
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [1u8; 32],
                deposit_proof(proof_id, 100),
            ),
            Error::<Test>::ProofReplay
        );
    });
}

#[test]
fn submit_deposit_proof_external_nonce_replay_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        let p1 = deposit_proof([1u8; 32], 100);
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            p1,
        ));
        // Different proof_id but same nonce/token/chain -> nonce replay.
        let mut p2 = deposit_proof([2u8; 32], 100);
        p2.nonce = 1;
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [1u8; 32], p2,),
            Error::<Test>::ExternalNonceReplay
        );
    });
}

#[test]
fn submit_deposit_proof_rejects_disabled_route() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3CrosschainGateway::register_asset(
            RuntimeOrigin::root(),
            ExternalChainId::BaseSepolia,
            bounded("0xTOKEN"),
            [9u8; 32],
        ));
        // Don't enable; route lookup itself will fail first.
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [1u8; 32],
                deposit_proof([1u8; 32], 100),
            ),
            Error::<Test>::RouteNotFound
        );
    });
}

#[test]
fn submit_deposit_proof_rejects_dry_run_mode() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3CrosschainGateway::register_asset(
            RuntimeOrigin::root(),
            ExternalChainId::BaseSepolia,
            bounded("0xTOKEN"),
            [9u8; 32],
        ));
        let mut r = route();
        r.mode = GatewayMode::DryRun;
        assert_ok!(X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r));
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [1u8; 32],
                deposit_proof([1u8; 32], 100),
            ),
            Error::<Test>::ModeBlocksCredit
        );
    });
}

#[test]
fn daily_limit_enforced() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        // Daily limit is 10_000; first deposit of 9_500 should be ok.
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 9_500),
        ));
        // Second deposit of 600 would exceed the limit.
        let mut p2 = deposit_proof([2u8; 32], 600);
        p2.nonce = 2;
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [1u8; 32], p2,),
            Error::<Test>::DailyLimitExceeded
        );
    });
}

#[test]
fn request_and_burn_withdrawal_preserves_invariant() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        // 1. Deposit some X3 representation.
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 100),
        ));
        assert_ok!(X3CrosschainGateway::credit_x3_representation(
            RuntimeOrigin::signed(1),
            [1u8; 32],
        ));
        // 2. Request a withdrawal.
        assert_ok!(X3CrosschainGateway::request_withdrawal(
            RuntimeOrigin::signed(2),
            [9u8; 32],
            ExternalChainId::BaseSepolia,
            bounded("0xRECIPIENT"),
            40,
        ));
        let w_id = expected_withdrawal_id(40);
        // 3. Burn the X3 representation.
        assert_ok!(X3CrosschainGateway::burn_x3_representation(
            RuntimeOrigin::signed(2),
            w_id,
        ));
        // 4. External locked is unchanged (still 100), pending is 40.
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 100);
        // 5. Finalize the release — external locked decrements.
        assert_ok!(X3CrosschainGateway::finalize_external_release(
            RuntimeOrigin::signed(2),
            w_id,
        ));
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 60);
    });
}

#[test]
fn collateral_invariant_holds_after_burn_and_release() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        // Deposit, credit, request, burn, release.
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 200),
        ));
        assert_ok!(X3CrosschainGateway::credit_x3_representation(
            RuntimeOrigin::signed(1),
            [1u8; 32],
        ));
        assert_ok!(X3CrosschainGateway::request_withdrawal(
            RuntimeOrigin::signed(2),
            [9u8; 32],
            ExternalChainId::BaseSepolia,
            bounded("0xRECIPIENT"),
            200,
        ));
        let w_id = expected_withdrawal_id(200);
        assert_ok!(X3CrosschainGateway::burn_x3_representation(
            RuntimeOrigin::signed(2),
            w_id,
        ));
        // invariant: locked(200) >= pending(200)
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 200);
        assert_eq!(Pallet::<Test>::pending_withdrawals([9u8; 32]), 200);
        assert_ok!(X3CrosschainGateway::finalize_external_release(
            RuntimeOrigin::signed(2),
            w_id,
        ));
        // After release: locked = 0, pending = 0.
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 0);
        assert_eq!(Pallet::<Test>::pending_withdrawals([9u8; 32]), 0);
    });
}

#[test]
fn double_release_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 100),
        ));
        assert_ok!(X3CrosschainGateway::credit_x3_representation(
            RuntimeOrigin::signed(1),
            [1u8; 32],
        ));
        assert_ok!(X3CrosschainGateway::request_withdrawal(
            RuntimeOrigin::signed(2),
            [9u8; 32],
            ExternalChainId::BaseSepolia,
            bounded("0xRECIPIENT"),
            100,
        ));
        let w_id = expected_withdrawal_id(100);
        assert_ok!(X3CrosschainGateway::burn_x3_representation(
            RuntimeOrigin::signed(2),
            w_id,
        ));
        assert_ok!(X3CrosschainGateway::finalize_external_release(
            RuntimeOrigin::signed(2),
            w_id,
        ));
        // Second release should fail because the withdrawal is already released.
        assert_noop!(
            X3CrosschainGateway::finalize_external_release(RuntimeOrigin::signed(2), w_id),
            Error::<Test>::WithdrawalAlreadyReleased
        );
    });
}
