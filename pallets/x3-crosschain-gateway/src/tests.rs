use super::mock::*;
use crate::pallet::Error;
use crate::{
    ExternalAssetRef, ExternalChainId, GatewayMode, GatewayTransferStatus, Pallet, RouteConfig,
};
use frame_support::{assert_noop, assert_ok};

// ── SVM validator-set helpers ──────────────────────────────────────────────

/// Authorized-validator list for `set_svm_validators` (32-byte Ed25519 keys).
fn validator_keys(
    keys: &[&ed25519_dalek::SigningKey],
) -> frame_support::BoundedVec<[u8; 32], crate::MaxSvmValidators> {
    frame_support::BoundedVec::try_from(
        keys.iter()
            .map(|key| key.verifying_key().to_bytes())
            .collect::<Vec<_>>(),
    )
    .expect("test validator set fits the bound")
}

/// Builds a `SOLANA_FINALIZED_FORMAT_V1` payload signed by `keys` over
/// `BLAKE2b-256(slot_le || blockhash)` — the message the relayer signs.
fn solana_payload_v1(
    slot: u64,
    blockhash: [u8; 32],
    keys: &[&ed25519_dalek::SigningKey],
) -> frame_support::BoundedVec<u8, frame_support::traits::ConstU32<4096>> {
    use ed25519_dalek::Signer;

    let message = x3_verification_router::solana_attestation_message(slot, &blockhash);
    let mut payload = vec![x3_verification_router::SOLANA_FINALIZED_FORMAT_V1];
    payload.extend_from_slice(&slot.to_le_bytes());
    payload.extend_from_slice(&blockhash);
    payload.extend_from_slice(&(keys.len() as u32).to_le_bytes());
    for key in keys {
        payload.extend_from_slice(&key.verifying_key().to_bytes());
        payload.extend_from_slice(&key.sign(&message).to_bytes());
    }
    frame_support::BoundedVec::try_from(payload).expect("payload fits the proof bound")
}

fn solana_stake_committee(
    keys: &[&ed25519_dalek::SigningKey],
    stakes: &[u64],
) -> frame_support::BoundedVec<crate::SolanaStakeEntry, crate::MaxSvmValidators> {
    let mut entries = keys
        .iter()
        .zip(stakes)
        .map(|(key, stake)| crate::SolanaStakeEntry {
            pubkey: key.verifying_key().to_bytes(),
            stake: *stake,
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.pubkey);
    frame_support::BoundedVec::try_from(entries).expect("test stake committee fits the bound")
}

fn solana_deposit_proof_v2(
    proof_id: [u8; 32],
    amount: u128,
    nonce: u64,
    finality: (u64, u64, [u8; 32], [u8; 32], [u8; 32]),
    keys: &[(&ed25519_dalek::SigningKey, u64)],
) -> crate::DepositProof {
    use ed25519_dalek::Signer;

    let (finalized_slot, parent_slot, finalized_blockhash, finalized_bank_hash, parent_bank_hash) =
        finality;
    let mut proof = deposit_proof_with(
        proof_id,
        amount,
        ExternalChainId::SolanaDevnet,
        solana_asset(),
        nonce,
        bounded_payload(""),
    );
    let route = solana_route();
    let envelope = Pallet::<Test>::envelope_from_deposit(&proof, &route);
    let mut stakes = keys
        .iter()
        .map(
            |(key, stake)| x3_verification_router::SolanaValidatorStake {
                pubkey: key.verifying_key().to_bytes(),
                stake: *stake,
            },
        )
        .collect::<Vec<_>>();
    stakes.sort_by_key(|entry| entry.pubkey);
    let stake_digest = x3_verification_router::solana_stake_table_digest(&stakes);
    let message = x3_verification_router::solana_finality_message_v3(
        x3_verification_router::SolanaCluster::Devnet,
        finalized_slot,
        parent_slot,
        &finalized_blockhash,
        &finalized_bank_hash,
        &parent_bank_hash,
        &stake_digest,
        &envelope,
    )
    .expect("test proof intent fits");
    let votes = keys
        .iter()
        .map(|(key, _)| {
            (
                key.verifying_key().to_bytes(),
                key.sign(&message).to_bytes(),
            )
        })
        .collect::<Vec<_>>();
    let payload = x3_verification_router::encode_solana_finality_v3(
        x3_verification_router::SolanaCluster::Devnet,
        finalized_slot,
        parent_slot,
        finalized_blockhash,
        finalized_bank_hash,
        parent_bank_hash,
        stake_digest,
        &votes,
    )
    .expect("test proof fits");
    proof.proof_payload =
        frame_support::BoundedVec::try_from(payload).expect("payload fits the proof bound");
    proof
}

fn signing_key(seed: u8) -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
}

fn expected_withdrawal_id(amount: u128) -> [u8; 32] {
    // The off-chain gateway derives the same id for the same withdrawal, and the
    // pallet emits its own in `WithdrawalRequested` and keys `Withdrawals` by it.
    // Calling the crate's function here is what keeps the two from drifting: this
    // helper used to repeat the derivation (once as XOR, matching the pallet's XOR
    // bug) so both sides were wrong in the same way and every assertion passed.
    //
    // `new_test_ext` starts at block 1 and the test asset/recipient below are the
    // ones the callers use.
    x3_crosschain_gateway::gateway_withdrawal_id([9u8; 32], "0xRECIPIENT", amount, 1)
}

fn register_and_enable() {
    assert_ok!(X3CrosschainGateway::register_asset(
        RuntimeOrigin::root(),
        ExternalChainId::BaseSepolia,
        bounded(TOKEN_ADDRESS),
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
            bounded(TOKEN_ADDRESS),
            [9u8; 32],
        ));
    });
}

#[test]
fn external_chain_kinds_keep_their_eip155_ids() {
    use x3_verification_router::ChainKind;

    assert_eq!(
        ExternalChainId::EthereumMainnet.to_chain_kind(),
        ChainKind::Evm { chain_id: 1 }
    );
    assert_eq!(
        ExternalChainId::EthereumSepolia.to_chain_kind(),
        ChainKind::Evm {
            chain_id: 11_155_111
        }
    );
    assert_eq!(
        ExternalChainId::BaseMainnet.to_chain_kind(),
        ChainKind::Evm { chain_id: 8_453 }
    );
    assert_eq!(
        ExternalChainId::BaseSepolia.to_chain_kind(),
        ChainKind::Evm { chain_id: 84_532 }
    );
}

#[test]
fn an_evm_route_with_an_invalid_asset_address_fails_closed() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, "0xTOKEN", [9u8; 32]);
        let mut malformed_route = evm_route();
        malformed_route.external_asset.token_address_or_mint = bounded("0xTOKEN");
        enable_route_for(malformed_route);
        let (payload, receipts_root, number, head) = evm_deposit_payload([1u8; 32], 100);
        attest_header(receipts_root, number, head);
        let mut proof = evm_deposit_proof([1u8; 32], 100, payload);
        proof.external_asset.token_address_or_mint = bounded("0xTOKEN");

        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(RuntimeOrigin::signed(1), [5u8; 32], proof,),
            Error::<Test>::VerificationFailed
        );
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
            bounded(TOKEN_ADDRESS),
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
            bounded(TOKEN_ADDRESS),
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        // The Solana verifier performs real Ed25519 verification in every
        // feature configuration (only the EVM/quorum stubs have a test hook), so
        // this plumbing test supplies an authorized key and a real attestation.
        let a = signing_key(1);
        register_asset(ExternalChainId::SolanaDevnet, "0xSOLTOKEN", [10u8; 32]);
        enable_route_for(solana_route());
        assert_ok!(X3CrosschainGateway::set_svm_validators(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
            validator_keys(&[&a]),
            1,
        ));

        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [3u8; 32],
            deposit_proof_with(
                [1u8; 32],
                100,
                ExternalChainId::SolanaDevnet,
                solana_asset(),
                1,
                solana_payload_v1(100, [7u8; 32], &[&a]),
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::EthereumSepolia, TOKEN_ADDRESS, [9u8; 32]);
        // Proof has EthereumSepolia, route is BaseSepolia
        let mut p = deposit_proof([1u8; 32], 100);
        p.source_chain = ExternalChainId::EthereumSepolia;
        p.external_asset = ExternalAssetRef {
            chain_id: ExternalChainId::EthereumSepolia,
            token_address_or_mint: bounded(TOKEN_ADDRESS),
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
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
                bounded(TOKEN_ADDRESS),
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

// ═════════════════════════════════════════════════════════════════════════
// SVM validator set (governance) and real Solana attestation verification
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn set_svm_validators_requires_governance() {
    new_test_ext().execute_with(|| {
        let a = signing_key(1);

        assert_noop!(
            X3CrosschainGateway::set_svm_validators(
                RuntimeOrigin::signed(1),
                ExternalChainId::SolanaDevnet,
                validator_keys(&[&a]),
                1,
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        assert_ok!(X3CrosschainGateway::set_svm_validators(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
            validator_keys(&[&a]),
            1,
        ));
        assert_eq!(
            crate::SvmValidatorSets::<Test>::get(ExternalChainId::SolanaDevnet)
                .expect("set stored")
                .threshold,
            1
        );
    });
}

#[test]
fn set_svm_validators_rejects_degenerate_sets() {
    new_test_ext().execute_with(|| {
        let (a, b) = (signing_key(1), signing_key(2));

        // Empty set.
        assert_noop!(
            X3CrosschainGateway::set_svm_validators(
                RuntimeOrigin::root(),
                ExternalChainId::SolanaDevnet,
                validator_keys(&[]),
                1,
            ),
            Error::<Test>::EmptySvmValidatorSet
        );

        // Zero threshold, and a threshold larger than the set.
        for threshold in [0u32, 3] {
            assert_noop!(
                X3CrosschainGateway::set_svm_validators(
                    RuntimeOrigin::root(),
                    ExternalChainId::SolanaDevnet,
                    validator_keys(&[&a, &b]),
                    threshold,
                ),
                Error::<Test>::InvalidSvmThreshold
            );
        }

        // A repeated key could occupy two threshold slots.
        assert_noop!(
            X3CrosschainGateway::set_svm_validators(
                RuntimeOrigin::root(),
                ExternalChainId::SolanaDevnet,
                validator_keys(&[&a, &a]),
                2,
            ),
            Error::<Test>::DuplicateSvmValidator
        );
    });
}

#[test]
fn set_solana_stake_committee_validates_the_canonical_table() {
    new_test_ext().execute_with(|| {
        let (a, b) = (signing_key(1), signing_key(2));
        let committee = solana_stake_committee(&[&a, &b], &[60, 40]);

        assert_noop!(
            X3CrosschainGateway::set_solana_stake_committee(
                RuntimeOrigin::signed(1),
                ExternalChainId::SolanaDevnet,
                committee.clone(),
                2,
                3,
            ),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            X3CrosschainGateway::set_solana_stake_committee(
                RuntimeOrigin::root(),
                ExternalChainId::SolanaDevnet,
                committee.clone(),
                0,
                3,
            ),
            Error::<Test>::InvalidSolanaStakeThreshold
        );
        assert_noop!(
            X3CrosschainGateway::set_solana_stake_committee(
                RuntimeOrigin::root(),
                ExternalChainId::SolanaDevnet,
                committee.clone(),
                4,
                3,
            ),
            Error::<Test>::InvalidSolanaStakeThreshold
        );

        let zero_stake = solana_stake_committee(&[&a, &b], &[60, 0]);
        assert_noop!(
            X3CrosschainGateway::set_solana_stake_committee(
                RuntimeOrigin::root(),
                ExternalChainId::SolanaDevnet,
                zero_stake,
                2,
                3,
            ),
            Error::<Test>::ZeroSolanaStake
        );

        let unsorted = frame_support::BoundedVec::try_from(
            committee.iter().copied().rev().collect::<Vec<_>>(),
        )
        .expect("same bounded size");
        assert_noop!(
            X3CrosschainGateway::set_solana_stake_committee(
                RuntimeOrigin::root(),
                ExternalChainId::SolanaDevnet,
                unsorted,
                2,
                3,
            ),
            Error::<Test>::InvalidSolanaStakeOrder
        );

        assert_ok!(X3CrosschainGateway::set_solana_stake_committee(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
            committee,
            2,
            3,
        ));
        assert!(crate::SolanaStakeCommittees::<Test>::get(ExternalChainId::SolanaDevnet).is_some());
    });
}

#[test]
fn solana_deposit_requires_authorized_attestations() {
    new_test_ext().execute_with(|| {
        let (a, b, rogue) = (signing_key(1), signing_key(2), signing_key(9));
        register_asset(ExternalChainId::SolanaDevnet, "0xSOLTOKEN", [10u8; 32]);
        enable_route_for(solana_route());
        assert_ok!(X3CrosschainGateway::set_svm_validators(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
            validator_keys(&[&a, &b]),
            2,
        ));

        // One authorized signature is short of the 2-validator threshold.
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [3u8; 32],
                deposit_proof_with(
                    [1u8; 32],
                    100,
                    ExternalChainId::SolanaDevnet,
                    solana_asset(),
                    1,
                    solana_payload_v1(100, [7u8; 32], &[&a]),
                ),
            ),
            Error::<Test>::VerificationFailed
        );

        // An unauthorized key cannot contribute, even beside an authorized one.
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [3u8; 32],
                deposit_proof_with(
                    [2u8; 32],
                    100,
                    ExternalChainId::SolanaDevnet,
                    solana_asset(),
                    2,
                    solana_payload_v1(100, [7u8; 32], &[&a, &rogue]),
                ),
            ),
            Error::<Test>::VerificationFailed
        );

        // Two authorized attestations satisfy the threshold: the deposit is
        // verified and credited.
        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [3u8; 32],
            deposit_proof_with(
                [4u8; 32],
                100,
                ExternalChainId::SolanaDevnet,
                solana_asset(),
                3,
                solana_payload_v1(100, [7u8; 32], &[&a, &b]),
            ),
        ));
        assert_eq!(Pallet::<Test>::external_locked([10u8; 32]), 100);
    });
}

#[test]
fn clearing_svm_validators_fails_closed() {
    new_test_ext().execute_with(|| {
        let (a, b) = (signing_key(1), signing_key(2));
        register_asset(ExternalChainId::SolanaDevnet, "0xSOLTOKEN", [10u8; 32]);
        enable_route_for(solana_route());
        assert_ok!(X3CrosschainGateway::set_svm_validators(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
            validator_keys(&[&a, &b]),
            2,
        ));
        assert_ok!(X3CrosschainGateway::clear_svm_validators(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
        ));
        assert!(crate::SvmValidatorSets::<Test>::get(ExternalChainId::SolanaDevnet).is_none());

        // Same valid proof as the happy path, but there is no authorized set.
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [3u8; 32],
                deposit_proof_with(
                    [5u8; 32],
                    100,
                    ExternalChainId::SolanaDevnet,
                    solana_asset(),
                    4,
                    solana_payload_v1(100, [7u8; 32], &[&a, &b]),
                ),
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

#[test]
fn solana_mainnet_never_falls_back_to_the_legacy_key_count_verifier() {
    new_test_ext().execute_with(|| {
        let a = signing_key(1);
        let asset = ExternalAssetRef {
            chain_id: ExternalChainId::SolanaMainnet,
            token_address_or_mint: bounded("0xSOLTOKEN"),
        };
        register_asset(ExternalChainId::SolanaMainnet, "0xSOLTOKEN", [12u8; 32]);
        let mut route = solana_route();
        route.route_id = [12u8; 32];
        route.external_chain_id = ExternalChainId::SolanaMainnet;
        route.external_asset = asset.clone();
        route.x3_asset_id = [12u8; 32];
        enable_route_for(route);
        assert_ok!(X3CrosschainGateway::set_svm_validators(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaMainnet,
            validator_keys(&[&a]),
            1,
        ));

        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [12u8; 32],
                deposit_proof_with(
                    [31u8; 32],
                    100,
                    ExternalChainId::SolanaMainnet,
                    asset,
                    31,
                    solana_payload_v1(400, [7u8; 32], &[&a]),
                ),
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

#[test]
fn stake_weighted_solana_finality_advances_and_enforces_the_slot_floor() {
    new_test_ext().execute_with(|| {
        let (a, b) = (signing_key(1), signing_key(2));
        register_asset(ExternalChainId::SolanaDevnet, "0xSOLTOKEN", [10u8; 32]);
        enable_route_for(solana_route());
        assert_ok!(X3CrosschainGateway::set_solana_stake_committee(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
            solana_stake_committee(&[&a, &b], &[60, 40]),
            2,
            3,
        ));

        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [3u8; 32],
            solana_deposit_proof_v2(
                [21u8; 32],
                100,
                21,
                (400, 399, [6u8; 32], [8u8; 32], [7u8; 32]),
                &[(&a, 60), (&b, 40)],
            ),
        ));
        assert_eq!(
            crate::SolanaFinalizedSlots::<Test>::get(ExternalChainId::SolanaDevnet),
            400
        );
        assert_eq!(Pallet::<Test>::external_locked([10u8; 32]), 100);

        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [3u8; 32],
                solana_deposit_proof_v2(
                    [22u8; 32],
                    100,
                    22,
                    (399, 398, [4u8; 32], [6u8; 32], [5u8; 32]),
                    &[(&a, 60), (&b, 40)],
                ),
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

#[test]
fn stake_weighted_solana_finality_rejects_less_than_two_thirds_stake() {
    new_test_ext().execute_with(|| {
        let (a, b) = (signing_key(1), signing_key(2));
        register_asset(ExternalChainId::SolanaDevnet, "0xSOLTOKEN", [10u8; 32]);
        enable_route_for(solana_route());
        assert_ok!(X3CrosschainGateway::set_solana_stake_committee(
            RuntimeOrigin::root(),
            ExternalChainId::SolanaDevnet,
            solana_stake_committee(&[&a, &b], &[60, 40]),
            2,
            3,
        ));

        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [3u8; 32],
                solana_deposit_proof_v2(
                    [23u8; 32],
                    100,
                    23,
                    (400, 399, [6u8; 32], [8u8; 32], [7u8; 32]),
                    &[(&a, 60)],
                ),
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

// ── The EVM-receipt route and its header anchor ────────────────────────────

#[test]
fn an_evm_route_refuses_a_proof_when_no_header_is_attested() {
    new_test_ext().execute_with(|| {
        forget_attested_header();
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
        enable_route_for(evm_route());
        let (payload, _, _, _) = evm_deposit_payload([1u8; 32], 100);

        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [5u8; 32],
                evm_deposit_proof([1u8; 32], 100, payload),
            ),
            Error::<Test>::VerificationFailed
        );
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 0);
    });
}

#[test]
fn an_evm_route_refuses_a_proof_for_a_header_the_chain_did_not_attest() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
        enable_route_for(evm_route());
        let (payload, receipts_root, number, head) = evm_deposit_payload([1u8; 32], 100);

        // A different root attested at the same height: the receipt is real, the
        // trie is real, the walk is real — and it is not the chain's block.
        let mut other_root = receipts_root;
        other_root[0] ^= 0xFF;
        attest_header(other_root, number, head);

        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [5u8; 32],
                evm_deposit_proof([1u8; 32], 100, payload),
            ),
            Error::<Test>::VerificationFailed
        );
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 0);
    });
}

#[test]
fn an_evm_route_verifies_a_proof_anchored_to_the_attested_header() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
        enable_route_for(evm_route());
        let (payload, receipts_root, number, head) = evm_deposit_payload([1u8; 32], 100);
        attest_header(receipts_root, number, head);

        assert_ok!(X3CrosschainGateway::submit_deposit_proof(
            RuntimeOrigin::signed(1),
            [5u8; 32],
            evm_deposit_proof([1u8; 32], 100, payload),
        ));
        assert_eq!(Pallet::<Test>::external_locked([9u8; 32]), 100);
    });
}

#[test]
fn an_evm_route_measures_depth_from_the_attested_head_not_the_proof() {
    new_test_ext().execute_with(|| {
        register_asset(ExternalChainId::BaseSepolia, TOKEN_ADDRESS, [9u8; 32]);
        enable_route_for(evm_route()); // finality_requirement = 12
        let (payload, receipts_root, number, _head) = evm_deposit_payload([1u8; 32], 100);

        // The payload claims a head 12 blocks past the header. The chain has
        // attested a head only 4 blocks past it, so the deposit is too shallow
        // whatever the payload says.
        attest_header(receipts_root, number, number + 4);

        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [5u8; 32],
                evm_deposit_proof([1u8; 32], 100, payload),
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

#[test]
fn the_two_derivations_agree() {
    let pallet_id =
        crate::pallet::Pallet::<Test>::derive_withdrawal_id([9u8; 32], b"0xRECIPIENT", 1_000, 1);
    let crate_id = x3_crosschain_gateway::gateway_withdrawal_id([9u8; 32], "0xRECIPIENT", 1_000, 1);
    assert_eq!(
        pallet_id, crate_id,
        "pallet {:?} vs crate {:?}",
        pallet_id, crate_id
    );
}

// ── Bitcoin SPV route ─────────────────────────────────────────────────────

/// Bitcoin's genesis block, and its only transaction.
fn bitcoin_genesis() -> (x3_btc_spv::BlockHeader, Vec<u8>) {
    let hex = |s: &str| -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    };
    let coinbase = hex("01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000");
    let header = x3_btc_spv::BlockHeader {
        version: 1,
        prev_blockhash: [0; 32],
        merkle_root: x3_btc_spv::sha256d(&coinbase),
        time: 1231006505,
        bits: 0x1d00ffff,
        nonce: 2083236893,
    };
    (header, coinbase)
}

fn enable_bitcoin_route(vault_script: &[u8]) {
    register_asset(ExternalChainId::BitcoinTestnet, "0xBTCTOKEN", [11u8; 32]);
    let mut route = _bitcoin_route();
    route.max_amount = u128::MAX;
    route.daily_limit = u128::MAX;
    route.contract_address = frame_support::BoundedVec::try_from(vault_script.to_vec()).unwrap();
    enable_route_for(route);
}

fn bitcoin_deposit(payload: Vec<u8>, proof_id: [u8; 32], amount: u128) -> crate::DepositProof {
    deposit_proof_with(
        proof_id,
        amount,
        ExternalChainId::BitcoinTestnet,
        bitcoin_asset(),
        1,
        frame_support::BoundedVec::try_from(payload).unwrap(),
    )
}

#[test]
fn a_bitcoin_deposit_in_a_block_this_chain_does_not_hold_is_refused() {
    new_test_ext().execute_with(|| {
        let (header, coinbase) = bitcoin_genesis();
        let vault = x3_btc_spv::transaction_outputs(&coinbase).unwrap()[0]
            .script_pubkey
            .clone();
        enable_bitcoin_route(&vault);
        let txid = x3_btc_spv::sha256d(&coinbase);
        let payload = x3_verification_router::encode_btc_proof_v1(&header, 0, &[], 0, &coinbase);
        let id = x3_verification_router::bitcoin_deposit_id(&txid, 0);

        // A real block with real proof of work, paying the route's script — but not on this
        // chain's Bitcoin view, so it proves nothing here.
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [4u8; 32],
                bitcoin_deposit(payload.clone(), id, 5_000_000_000),
            ),
            Error::<Test>::VerificationFailed
        );

        // Held, deep enough, paying the vault — and still refused: the transaction does not name
        // the beneficiary, so whoever submits it would be the one credited.
        anchor_btc_block(header.block_hash(), 0, 100);
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [4u8; 32],
                bitcoin_deposit(payload, id, 5_000_000_000),
            ),
            Error::<Test>::VerificationFailed
        );
        assert_eq!(Pallet::<Test>::external_locked([11u8; 32]), 0);
    });
}

#[test]
fn the_payload_the_old_bitcoin_verifier_accepted_is_refused() {
    new_test_ext().execute_with(|| {
        enable_bitcoin_route(b"0xGATEWAY");
        // Tip 200, an all-zero header with an easy target, "index" 100, an all-zero txid.
        let mut payload = Vec::new();
        payload.extend_from_slice(&200u64.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes());
        let mut header = [0u8; 80];
        header[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
        header[76..80].copy_from_slice(&2561u32.to_le_bytes());
        payload.extend_from_slice(&header);
        payload.extend_from_slice(&100u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&[0u8; 32]);
        assert_noop!(
            X3CrosschainGateway::submit_deposit_proof(
                RuntimeOrigin::signed(1),
                [4u8; 32],
                bitcoin_deposit(payload, [1u8; 32], 100),
            ),
            Error::<Test>::VerificationFailed
        );
    });
}
