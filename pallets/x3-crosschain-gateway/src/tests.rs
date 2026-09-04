use super::mock::*;
use crate::pallet::Error;
use crate::{
    ExternalAssetRef, ExternalChainId, GatewayMode, GatewayTransferStatus, Pallet, RouteConfig,
};
use frame_support::{assert_noop, assert_ok};

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

fn register_asset(chain: ExternalChainId, token: &str, x3_asset_id: [u8; 32]) {
    assert_ok!(X3CrosschainGateway::register_asset(
        RuntimeOrigin::root(),
        chain,
        bounded(token),
        x3_asset_id,
    ));
}

fn enable_route_for(config: RouteConfig) {
    assert_ok!(X3CrosschainGateway::enable_route(
        RuntimeOrigin::root(),
        config,
    ));
}

fn deposit_and_credit(proof_id: [u8; 32], route_id: [u8; 32], amount: u128) {
    assert_ok!(X3CrosschainGateway::submit_deposit_proof(
        RuntimeOrigin::signed(1),
        route_id,
        deposit_proof(proof_id, amount),
    ));
    assert_ok!(X3CrosschainGateway::credit_x3_representation(
        RuntimeOrigin::signed(1),
        proof_id,
    ));
}

fn request_burn_withdrawal(amount: u128) -> [u8; 32] {
    let w_id = expected_withdrawal_id(amount);
    assert_ok!(X3CrosschainGateway::request_withdrawal(
        RuntimeOrigin::signed(2),
        [9u8; 32],
        ExternalChainId::BaseSepolia,
        bounded("0xRECIPIENT"),
        amount,
    ));
    assert_ok!(X3CrosschainGateway::burn_x3_representation(
        RuntimeOrigin::signed(2),
        w_id,
    ));
    w_id
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

// ═════════════════════════════════════════════════════════════════════════
// Full deposit flow — register → enable → submit_deposit_proof → verify
// events and storage
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn full_deposit_flow_emits_events_and_updates_storage() {
    new_test_ext().execute_with(|| {
        register_and_enable();

        let proof_id = [1u8; 32];
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof(proof_id, 100),
        ));

        // Storage checks
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 100);
        let t = Pallet::<Test>::transfers(proof_id).expect("transfer must exist");
        assert_eq!(t.amount, 100);
        assert_eq!(t.status, GatewayTransferStatus::Verified);
        assert_eq!(t.route_id, [1u8; 32]);

        // Event checks
        let events = System::events();
        assert!(events.iter().any(|r| matches!(
            r.event,
            RuntimeEvent::X3CrosschainGateway(crate::Event::DepositProofVerified { .. })
        )));
        assert!(events.iter().any(|r| matches!(
            r.event,
            RuntimeEvent::X3CrosschainGateway(crate::Event::AssetRegistered { .. })
        )));
        assert!(events.iter().any(|r| matches!(
            r.event,
            RuntimeEvent::X3CrosschainGateway(crate::Event::RouteEnabled { .. })
        )));
    });
}

// ═════════════════════════════════════════════════════════════════════════
// Full withdrawal flow with submit_release_proof
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn submit_release_proof_happy_path() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        deposit_and_credit([1u8; 32], [1u8; 32], 100);

        let w_id = request_burn_withdrawal(40);

        // Submit release proof
        assert_ok!(X3CrosschainGateway::submit_release_proof(
            RuntimeOrigin::signed(1),
            w_id,
            [1u8; 32],
            bounded_payload("release_proof_data"),
        ));

        // Storage: locked decremented, pending decremented
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 60);
        assert_eq!(Pallet::<Test>::pending_withdrawals([9u8; 32]), 0);

        // Event emitted
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::X3CrosschainGateway(crate::Event::WithdrawalReleased {
                withdrawal_id: id,
            }) if id == w_id
        )));
    });
}

#[test]
fn submit_release_proof_without_burn_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        deposit_and_credit([1u8; 32], [1u8; 32], 100);

        // Request withdrawal but don't burn
        let w_id = expected_withdrawal_id(40);
        assert_ok!(X3CrosschainGateway::request_withdrawal(
            RuntimeOrigin::signed(2),
            [9u8; 32],
            ExternalChainId::BaseSepolia,
            bounded("0xRECIPIENT"),
            40,
        ));

        assert_noop!(
            X3CrosschainGateway::submit_release_proof(
                RuntimeOrigin::signed(1),
                w_id,
                [1u8; 32],
                bounded_payload("x"),
            ),
            Error::<Test>::WithdrawalNotBurned
        );
    });
}

#[test]
fn submit_release_proof_double_release_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        deposit_and_credit([1u8; 32], [1u8; 32], 100);
        let w_id = request_burn_withdrawal(40);

        // First release works
        assert_ok!(X3CrosschainGateway::submit_release_proof(
            RuntimeOrigin::signed(1),
            w_id,
            [1u8; 32],
            bounded_payload("x"),
        ));
        // Second release fails
        assert_noop!(
            X3CrosschainGateway::submit_release_proof(
                RuntimeOrigin::signed(1),
                w_id,
                [1u8; 32],
                bounded_payload("x"),
            ),
            Error::<Test>::WithdrawalAlreadyReleased
        );
    });
}

#[test]
fn submit_release_proof_wrong_chain_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        deposit_and_credit([1u8; 32], [1u8; 32], 100);

        // Request withdrawal with a different destination chain (EthereumSepolia
        // instead of BaseSepolia)
        let w_id = expected_withdrawal_id(40);
        assert_ok!(X3CrosschainGateway::request_withdrawal(
            RuntimeOrigin::signed(2),
            [9u8; 32],
            ExternalChainId::EthereumSepolia,
            bounded("0xRECIPIENT"),
            40,
        ));
        assert_ok!(X3CrosschainGateway::burn_x3_representation(
            RuntimeOrigin::signed(2),
            w_id,
        ));

        // Route [1u8;32] has external_chain_id = BaseSepolia — mismatches
        // withdrawal's EthereumSepolia
        assert_noop!(
            X3CrosschainGateway::submit_release_proof(
                RuntimeOrigin::signed(1),
                w_id,
                [1u8; 32],
                bounded_payload("x"),
            ),
            Error::<Test>::WrongChain
        );
    });
}

#[test]
fn submit_release_proof_disabled_route_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        deposit_and_credit([1u8; 32], [1u8; 32], 100);
        let w_id = request_burn_withdrawal(40);

        // Disable the route
        assert_ok!(X3CrosschainGateway::disable_route(
            RuntimeOrigin::root(),
            [1u8; 32],
        ));

        assert_noop!(
            X3CrosschainGateway::submit_release_proof(
                RuntimeOrigin::signed(1),
                w_id,
                [1u8; 32],
                bounded_payload("x"),
            ),
            Error::<Test>::RouteDisabled
        );
    });
}

// ═════════════════════════════════════════════════════════════════════════
// Verifier integration — test each supported verifier strategy
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn deposit_proof_validator_quorum_works() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        enable_route_for(validator_route());

        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [2u8; 32],
            deposit_proof_with(
                [1u8; 32],
                100,
                ExternalChainId::BaseSepolia,
                asset(),
                1,
                valid_proof_payload(),
            ),
        ));

        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 100);
    });
}

#[test]
fn deposit_proof_solana_verifier_works() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::SolanaDevnet, "0xSOLTOKEN", [10u8; 32]);
        enable_route_for(solana_route());

        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [3u8; 32],
            deposit_proof_with(
                [1u8; 32],
                100,
                ExternalChainId::SolanaDevnet,
                solana_asset(),
                1,
                valid_proof_payload(),
            ),
        ));

        assert_eq!(Pallet::<Test>::external_locked([10u8; 32]), 100);
    });
}

#[test]
fn deposit_proof_different_chain_asset_works() {
    new_test_ext().execute_with(|| {
        // Test a complete deposit flow for a second asset/chain pair
        // using ValidatorQuorum verification (which accepts any non-empty payload).
        register_asset(ExternalChainId::BitcoinTestnet, "0xBTCTOKEN", [11u8; 32]);
        let mut btc_route = validator_route();
        btc_route.route_id = [4u8; 32];
        btc_route.external_chain_id = ExternalChainId::BitcoinTestnet;
        btc_route.external_asset = bitcoin_asset();
        btc_route.x3_asset_id = [11u8; 32];
        enable_route_for(btc_route);

        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [4u8; 32],
            deposit_proof_with(
                [1u8; 32],
                100,
                ExternalChainId::BitcoinTestnet,
                bitcoin_asset(),
                1,
                valid_proof_payload(),
            ),
        ));

        assert_eq!(Pallet::<Test>::external_locked([11u8; 32]), 100);
    });
}

#[test]
fn deposit_proof_evm_verifier_fails_on_malformed_proof() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        enable_route_for(evm_route());

        // ProductionEvmReceiptVerifier will fail on a short random payload
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [5u8; 32],
                deposit_proof_with(
                    [1u8; 32],
                    100,
                    ExternalChainId::BaseSepolia,
                    asset(),
                    1,
                    bounded_payload("short"),
                ),
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

#[test]
fn release_proof_validator_quorum_works() {
    new_test_ext().execute_with(|| {
        // Two routes: one for deposit (X3Internal), one for release
        // (ValidatorQuorum) with matching chain + asset ids.
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let deposit_route = route();
        enable_route_for(deposit_route.clone());
        let mut release_route = validator_route();
        release_route.external_chain_id = ExternalChainId::BaseSepolia;
        release_route.external_asset = asset();
        enable_route_for(release_route);

        deposit_and_credit([1u8; 32], [1u8; 32], 100);
        let w_id = request_burn_withdrawal(40);

        assert_ok!(X3CrosschainGateway::submit_release_proof(
            RuntimeOrigin::signed(1),
            w_id,
            [2u8; 32], // validator route
            valid_proof_payload(),
        ));

        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 60);
    });
}

// ═════════════════════════════════════════════════════════════════════════
// Route management tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn enable_then_disable_then_re_enable_route() {
    new_test_ext().execute_with(|| {
        register_and_enable();

        // Disable
        assert_ok!(X3CrosschainGateway::disable_route(
            RuntimeOrigin::root(),
            [1u8; 32],
        ));
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::X3CrosschainGateway(crate::Event::RouteDisabled { .. })
        )));
        // Verify route is actually disabled
        let r = crate::Routes::<Test>::get([1u8; 32]).unwrap();
        assert!(!r.enabled);

        // Re-enable requires RouteConfig
        let mut r = route();
        r.enabled = false;
        assert_ok!(X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r));
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::X3CrosschainGateway(crate::Event::RouteEnabled { .. })
        )));
        // Verify route is re-enabled
        let r = crate::Routes::<Test>::get([1u8; 32]).unwrap();
        assert!(r.enabled);
    });
}

#[test]
fn enable_route_min_gt_max_fails() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut r = route();
        r.min_amount = 200;
        r.max_amount = 100;
        assert_noop!(
            X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r),
            Error::<Test>::AmountAboveMaximum
        );
    });
}

#[test]
fn enable_route_zero_daily_limit_for_live_fails() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut r = route();
        r.daily_limit = 0;
        // TestnetLive mode requires daily_limit > 0
        assert_noop!(
            X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r),
            Error::<Test>::AmountBelowMinimum
        );
    });
}

// ═════════════════════════════════════════════════════════════════════════
// Daily limit tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn daily_limit_rolling_reset() {
    new_test_ext().execute_with(|| {
        register_and_enable();

        // Deposit 9_500 (under 10_000 daily limit)
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 9_500),
        ));

        // Exceed limit
        let mut p2 = deposit_proof([2u8; 32], 600);
        p2.nonce = 2;
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [1u8; 32], p2),
            Error::<Test>::DailyLimitExceeded
        );

        // Advance past the daily window (DailyWindow = 100 blocks)
        System::set_block_number(101);

        // Deposit 500 should now succeed (limit reset)
        let mut p3 = deposit_proof([3u8; 32], 500);
        p3.nonce = 3;
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            p3,
        ));
    });
}

// ═════════════════════════════════════════════════════════════════════════
// Asset registration tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn register_multiple_assets_on_different_chains() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN_A", [1u8; 32]);
        register_asset(ExternalChainId::EthereumSepolia, "0xTOKEN_B", [2u8; 32]);
        register_asset(ExternalChainId::SolanaDevnet, "0xSOLTOKEN", [3u8; 32]);

        // Verify storage
        assert_eq!(
            crate::Assets::<Test>::get(ExternalChainId::BaseSepolia, bounded("0xTOKEN_A")),
            Some([1u8; 32])
        );
        assert_eq!(
            crate::Assets::<Test>::get(ExternalChainId::EthereumSepolia, bounded("0xTOKEN_B")),
            Some([2u8; 32])
        );
        assert_eq!(
            crate::Assets::<Test>::get(ExternalChainId::SolanaDevnet, bounded("0xSOLTOKEN")),
            Some([3u8; 32])
        );
    });
}

// ═════════════════════════════════════════════════════════════════════════
// Error case tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn submit_deposit_proof_wrong_chain_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        // Register a second asset on EthereumSepolia so AssetNotRegistered doesn't
        // fire before WrongChain.
        register_asset(ExternalChainId::EthereumSepolia, "0xTOKEN", [9u8; 32]);
        // Proof has EthereumSepolia, route is BaseSepolia
        let mut p = deposit_proof([1u8; 32], 100);
        p.source_chain = ExternalChainId::EthereumSepolia;
        p.external_asset = ExternalAssetRef {
            chain_id: ExternalChainId::EthereumSepolia,
            token_address_or_mint: bounded("0xTOKEN"),
        };
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [1u8; 32], p),
            Error::<Test>::WrongChain
        );
    });
}

#[test]
fn submit_deposit_proof_wrong_token_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        // Register a second token on the same chain so AssetNotRegistered
        // doesn't fire before WrongToken.
        register_asset(ExternalChainId::BaseSepolia, "0xOTHERTOKEN", [9u8; 32]);
        let mut p = deposit_proof([1u8; 32], 100);
        p.external_asset = ExternalAssetRef {
            chain_id: ExternalChainId::BaseSepolia,
            token_address_or_mint: bounded("0xOTHERTOKEN"),
        };
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [1u8; 32], p),
            Error::<Test>::WrongToken
        );
    });
}

#[test]
fn submit_deposit_proof_amount_below_minimum_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [1u8; 32],
                deposit_proof([1u8; 32], 0),
            ),
            Error::<Test>::AmountBelowMinimum
        );
    });
}

#[test]
fn submit_deposit_proof_amount_above_maximum_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [1u8; 32],
                deposit_proof([1u8; 32], 100_001),
            ),
            Error::<Test>::AmountAboveMaximum
        );
    });
}

#[test]
fn submit_deposit_proof_unfinalized_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        let mut p = deposit_proof([1u8; 32], 100);
        p.finalized_at_block = 0;
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [1u8; 32], p),
            Error::<Test>::UnfinalizedProof
        );
    });
}

#[test]
fn submit_deposit_proof_empty_recipient_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        let mut p = deposit_proof([1u8; 32], 100);
        p.recipient = bounded("");
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [1u8; 32], p),
            Error::<Test>::EmptyRecipient
        );
    });
}

#[test]
fn request_withdrawal_empty_recipient_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        assert_noop!(
            X3CrosschainGateway::request_withdrawal(
                RuntimeOrigin::signed(2),
                [9u8; 32],
                ExternalChainId::BaseSepolia,
                bounded(""),
                100,
            ),
            Error::<Test>::EmptyRecipient
        );
    });
}

#[test]
fn request_withdrawal_zero_amount_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        assert_noop!(
            X3CrosschainGateway::request_withdrawal(
                RuntimeOrigin::signed(2),
                [9u8; 32],
                ExternalChainId::BaseSepolia,
                bounded("0xRECIPIENT"),
                0,
            ),
            Error::<Test>::AmountBelowMinimum
        );
    });
}

#[test]
fn credit_x3_representation_twice_fails() {
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
        assert_noop!(
            X3CrosschainGateway::credit_x3_representation(RuntimeOrigin::signed(1), [1u8; 32]),
            Error::<Test>::InvalidTransferStatus
        );
    });
}

#[test]
fn credit_x3_representation_not_found_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        assert_noop!(
            X3CrosschainGateway::credit_x3_representation(RuntimeOrigin::signed(1), [99u8; 32],),
            Error::<Test>::TransferNotFound
        );
    });
}

#[test]
fn burn_x3_representation_not_found_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3CrosschainGateway::burn_x3_representation(RuntimeOrigin::signed(2), [99u8; 32],),
            Error::<Test>::WithdrawalNotFound
        );
    });
}

#[test]
fn finalize_external_release_not_found_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3CrosschainGateway::finalize_external_release(RuntimeOrigin::signed(2), [99u8; 32],),
            Error::<Test>::WithdrawalNotFound
        );
    });
}

// ═════════════════════════════════════════════════════════════════════════
// Gate mode tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn route_mode_guarded_live_works() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut r = route();
        r.mode = GatewayMode::GuardedLive;
        assert_ok!(X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r));
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 100),
        ));
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 100);
    });
}

#[test]
fn route_mode_full_live_works() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut r = route();
        r.mode = GatewayMode::FullLive;
        assert_ok!(X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r));
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 100),
        ));
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 100);
    });
}

#[test]
fn route_mode_disabled_blocks_deposit() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut r = route();
        r.mode = GatewayMode::Disabled;
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

// ═════════════════════════════════════════════════════════════════════════
// Dispute window tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn credit_with_dispute_window_after_window() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut r = route();
        r.require_dispute_window = true;
        r.dispute_window_blocks = 10;
        assert_ok!(X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r));
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 100),
        ));

        // Advance past dispute window (10 blocks, starting from block 1)
        System::set_block_number(12);

        assert_ok!(X3CrosschainGateway::credit_x3_representation(
            RuntimeOrigin::signed(1),
            [1u8; 32],
        ));
        let t = Pallet::<Test>::transfers([1u8; 32]).unwrap();
        assert_eq!(t.status, GatewayTransferStatus::X3Credited);
    });
}

#[test]
fn credit_with_dispute_window_before_close_fails() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut r = route();
        r.require_dispute_window = true;
        r.dispute_window_blocks = 10;
        assert_ok!(X3CrosschainGateway::enable_route(RuntimeOrigin::root(), r));
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [1u8; 32],
            deposit_proof([1u8; 32], 100),
        ));

        // Only advanced 5 blocks, not enough
        System::set_block_number(6);

        assert_noop!(
            X3CrosschainGateway::credit_x3_representation(RuntimeOrigin::signed(1), [1u8; 32]),
            Error::<Test>::DisputeWindowOpen
        );
    });
}

// ═════════════════════════════════════════════════════════════════════════
// submit_release_proof route-not-found error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn submit_release_proof_route_not_found_fails() {
    new_test_ext().execute_with(|| {
        register_and_enable();
        deposit_and_credit([1u8; 32], [1u8; 32], 100);
        let w_id = request_burn_withdrawal(40);

        assert_noop!(
            X3CrosschainGateway::submit_release_proof(
                RuntimeOrigin::signed(1),
                w_id,
                [99u8; 32], // non-existent route
                bounded_payload("x"),
            ),
            Error::<Test>::RouteNotFound
        );
    });
}

// ═════════════════════════════════════════════════════════════════════════
// submit_deposit_proof governance-only origin checks
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn register_asset_non_governance_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3CrosschainGateway::register_asset(
                RuntimeOrigin::signed(1),
                ExternalChainId::BaseSepolia,
                bounded("0xTOKEN"),
                [9u8; 32],
            ),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn enable_route_non_governance_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3CrosschainGateway::enable_route(RuntimeOrigin::signed(1), route()),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}
