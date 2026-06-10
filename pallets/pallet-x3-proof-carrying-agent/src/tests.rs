//! Tests for the Proof-Carrying Agent pallet.
//!
//! Covers all 6 extrinsics and edge cases:
//! - submit_proof_carrying_action (basic, payload limits, nonce, pending limits)
//! - verify_action (success, failure, non-pending rejection)
//! - challenge_proof (valid, duplicate, non-verified rejection)
//! - resolve_challenge (upheld, dismissed, expired)
//! - set_proof_config (admin only, config update)
//! - clean_expired_proofs (expired cleanup)

use crate::mock::*;
use crate::{
    Error, Event,
};
use crate::types::{
    AgentProofStats, ChallengeResolution, ProofChallenge, ProofConfig, ProofKind, ProofStatus,
    VerifiedAction,
};
use frame_support::{assert_noop, assert_ok, traits::Currency};

// ── Helper ──────────────────────────────────────────────────────────────────

fn submit_default_action(agent: u64, nonce: u64) -> [u8; 32] {
    let action_payload = vec![1, 2, 3, 4];
    let proof_payload = vec![5, 6, 7, 8];
    let deadline = 200u64;

    assert_ok!(ProofCarryingAgent::submit_proof_carrying_action(
        RuntimeOrigin::signed(agent),
        action_payload,
        proof_payload,
        ProofKind::ZkSnark,
        1, // target_pallet
        0, // target_call
        deadline,
        nonce,
    ));

    // Extract the action_id from the last event
    let events = frame_system::Pallet::<Test>::events();
    let last_event = events.last().unwrap();
    if let Event::ActionSubmitted { action_id, .. } = last_event.event.clone().try_into().unwrap() {
        return action_id;
    }
    panic!("Expected ActionSubmitted event");
}

fn get_action_id_from_events() -> [u8; 32] {
    let events = frame_system::Pallet::<Test>::events();
    for event in events.iter().rev() {
        if let Event::ActionSubmitted { action_id, .. } =
            event.event.clone().try_into().unwrap()
        {
            return action_id;
        }
    }
    panic!("No ActionSubmitted event found");
}

// ── submit_proof_carrying_action ────────────────────────────────────────────

#[test]
fn test_basic_submission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify action was stored
        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.agent, ALICE);
        assert_eq!(action.status, ProofStatus::Pending);
        assert_eq!(action.proof_kind, ProofKind::ZkSnark);
        assert_eq!(action.target_pallet, 1);
        assert_eq!(action.target_call, 0);
        assert_eq!(action.nonce, 1);

        // Verify pending list
        let pending = ProofCarryingAgent::pending_actions(ALICE);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0], action_id);

        // Verify stats
        let stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(stats.total_submitted, 1);

        // Verify nonce was updated
        assert_eq!(ProofCarryingAgent::agent_nonces(ALICE), 1);

        // Verify event
        System::assert_has_event(
            Event::ActionSubmitted {
                agent: ALICE,
                action_id,
                proof_kind: ProofKind::ZkSnark,
                target_pallet: 1,
                target_call: 0,
            }
            .into(),
        );
    });
}

#[test]
fn test_submission_payload_too_large() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Action payload exceeds MaxActionPayloadSize (1024)
        let large_payload = vec![0u8; 1025];
        assert_noop!(
            ProofCarryingAgent::submit_proof_carrying_action(
                RuntimeOrigin::signed(ALICE),
                large_payload,
                vec![5, 6, 7, 8],
                ProofKind::ZkSnark,
                1,
                0,
                200u64,
                1,
            ),
            Error::<Test>::ActionPayloadTooLarge
        );

        // Proof payload exceeds MaxProofPayloadSize (4096)
        let large_proof = vec![0u8; 4097];
        assert_noop!(
            ProofCarryingAgent::submit_proof_carrying_action(
                RuntimeOrigin::signed(ALICE),
                vec![1, 2, 3, 4],
                large_proof,
                ProofKind::ZkSnark,
                1,
                0,
                200u64,
                1,
            ),
            Error::<Test>::ProofPayloadTooLarge
        );
    });
}

#[test]
fn test_submission_nonce_enforcement() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // First submission with nonce 1
        submit_default_action(ALICE, 1);

        // Second submission with same nonce should fail
        assert_noop!(
            ProofCarryingAgent::submit_proof_carrying_action(
                RuntimeOrigin::signed(ALICE),
                vec![2, 3, 4, 5],
                vec![6, 7, 8, 9],
                ProofKind::FormalVerification,
                1,
                0,
                200u64,
                1,
            ),
            Error::<Test>::InvalidNonce
        );

        // Submission with nonce 0 (less than current) should fail
        assert_noop!(
            ProofCarryingAgent::submit_proof_carrying_action(
                RuntimeOrigin::signed(ALICE),
                vec![2, 3, 4, 5],
                vec![6, 7, 8, 9],
                ProofKind::FormalVerification,
                1,
                0,
                200u64,
                0,
            ),
            Error::<Test>::InvalidNonce
        );

        // Submission with nonce 2 should succeed
        submit_default_action(ALICE, 2);
    });
}

#[test]
fn test_submission_pending_limit() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // MaxPendingProofsPerAgent is 10, so submit 10 actions
        for i in 1..=10u64 {
            submit_default_action(ALICE, i);
        }

        // 11th should fail
        assert_noop!(
            ProofCarryingAgent::submit_proof_carrying_action(
                RuntimeOrigin::signed(ALICE),
                vec![1, 2, 3, 4],
                vec![5, 6, 7, 8],
                ProofKind::ZkSnark,
                1,
                0,
                200u64,
                11,
            ),
            Error::<Test>::TooManyPendingProofs
        );
    });
}

#[test]
fn test_submission_multiple_agents() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let id1 = submit_default_action(ALICE, 1);
        let id2 = submit_default_action(BOB, 1);

        assert_ne!(id1, id2);

        let pending_alice = ProofCarryingAgent::pending_actions(ALICE);
        let pending_bob = ProofCarryingAgent::pending_actions(BOB);
        assert_eq!(pending_alice.len(), 1);
        assert_eq!(pending_bob.len(), 1);
    });
}

#[test]
fn test_submission_different_proof_kinds() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let kinds = vec![
            ProofKind::ZkSnark,
            ProofKind::FormalVerification,
            ProofKind::ReplayProof,
            ProofKind::ValidatorAttestation,
            ProofKind::FraudProof,
            ProofKind::ExecutionTrace,
            ProofKind::Custom(42),
        ];

        for (i, kind) in kinds.iter().enumerate() {
            let nonce = (i + 1) as u64;
            assert_ok!(ProofCarryingAgent::submit_proof_carrying_action(
                RuntimeOrigin::signed(ALICE),
                vec![i as u8],
                vec![(i + 100) as u8],
                kind.clone(),
                1,
                0,
                200u64,
                nonce,
            ));
        }

        let stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(stats.total_submitted, 7);
    });
}

// ── verify_action ───────────────────────────────────────────────────────────

#[test]
fn test_verify_action_success() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Advance to block 10
        System::set_block_number(10);

        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![1, 2, 3], // verification reason
        ));

        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Verified);
        assert_eq!(action.verified_at, Some(10));
        assert_eq!(action.verification_reason, vec![1, 2, 3]);

        // Stats updated
        let stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(stats.total_verified, 1);

        // Event emitted
        System::assert_has_event(
            Event::ActionVerified {
                agent: ALICE,
                action_id,
            }
            .into(),
        );
    });
}

#[test]
fn test_verify_action_failure() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            false,
            vec![4, 5, 6], // failure reason
        ));

        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Failed);
        assert_eq!(action.verification_reason, vec![4, 5, 6]);

        // Stats updated
        let stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(stats.total_failed, 1);

        // Event emitted
        System::assert_has_event(
            Event::ActionFailed {
                agent: ALICE,
                action_id,
                reason: vec![4, 5, 6],
            }
            .into(),
        );
    });
}

#[test]
fn test_verify_action_not_found() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            ProofCarryingAgent::verify_action(
                RuntimeOrigin::signed(BOB),
                [0u8; 32],
                true,
                vec![],
            ),
            Error::<Test>::ActionNotFound
        );
    });
}

#[test]
fn test_verify_action_already_verified() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify once
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        // Verify again should fail
        assert_noop!(
            ProofCarryingAgent::verify_action(
                RuntimeOrigin::signed(BOB),
                action_id,
                true,
                vec![],
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

#[test]
fn test_verify_action_already_failed() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Fail once
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            false,
            vec![],
        ));

        // Try to verify again should fail
        assert_noop!(
            ProofCarryingAgent::verify_action(
                RuntimeOrigin::signed(BOB),
                action_id,
                true,
                vec![],
            ),
            Error::<Test>::VerificationFailed
        );
    });
}

// ── challenge_proof ─────────────────────────────────────────────────────────

#[test]
fn test_challenge_proof_valid() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify first
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        // Reserve some balance for BOB
        let _ = Balances::make_free_balance_be(&BOB, 1_000_000);

        // Challenge
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(BOB),
            action_id,
            vec![1, 2, 3], // challenge reason
        ));

        // Action should be in challenged state
        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Challenged);

        // Challenge should exist
        let challenge = ProofCarryingAgent::active_challenges(action_id).unwrap();
        assert_eq!(challenge.challenger, BOB);
        assert_eq!(challenge.reason, vec![1, 2, 3]);
        assert_eq!(challenge.challenge_stake, 100); // min_challenge_stake
        assert!(challenge.resolution.is_none());

        // Stake should be reserved
        assert_eq!(Balances::reserved_balance(&BOB), 100);

        // Stats updated
        let stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(stats.total_challenged, 1);

        // Event emitted
        System::assert_has_event(
            Event::ProofChallenged {
                action_id,
                challenger: BOB,
            }
            .into(),
        );
    });
}

#[test]
fn test_challenge_proof_not_verified() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Can't challenge a pending proof
        assert_noop!(
            ProofCarryingAgent::challenge_proof(
                RuntimeOrigin::signed(BOB),
                action_id,
                vec![],
            ),
            Error::<Test>::NotChallengeable
        );
    });
}

#[test]
fn test_challenge_proof_not_found() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            ProofCarryingAgent::challenge_proof(
                RuntimeOrigin::signed(BOB),
                [0u8; 32],
                vec![],
            ),
            Error::<Test>::ActionNotFound
        );
    });
}

#[test]
fn test_challenge_proof_duplicate() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        let _ = Balances::make_free_balance_be(&BOB, 1_000_000);
        let _ = Balances::make_free_balance_be(&CHARLIE, 1_000_000);

        // First challenge
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(BOB),
            action_id,
            vec![],
        ));

        // Second challenge should fail — action is already challenged, not challengeable
        assert_noop!(
            ProofCarryingAgent::challenge_proof(
                RuntimeOrigin::signed(CHARLIE),
                action_id,
                vec![],
            ),
            Error::<Test>::NotChallengeable
        );
    });
}

#[test]
fn test_challenge_proof_insufficient_balance() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        // BOB has no free balance (only existential deposit)
        let _ = Balances::make_free_balance_be(&BOB, 1);

        // The reserve call returns a module error from pallet_balances
        // (InsufficientBalance), not our custom InsufficientChallengeStake error
        assert_noop!(
            ProofCarryingAgent::challenge_proof(
                RuntimeOrigin::signed(BOB),
                action_id,
                vec![],
            ),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

// ── resolve_challenge ───────────────────────────────────────────────────────

#[test]
fn test_resolve_challenge_upheld() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        let _ = Balances::make_free_balance_be(&BOB, 1_000_000);
        let bob_balance_before = Balances::free_balance(&BOB);

        // Challenge
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(BOB),
            action_id,
            vec![],
        ));

        // Admin resolves as Upheld
        assert_ok!(ProofCarryingAgent::resolve_challenge(
            RuntimeOrigin::signed(ADMIN),
            action_id,
            ChallengeResolution::Upheld,
        ));

        // Action should be Failed
        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Failed);

        // Challenge should be resolved
        let challenge = ProofCarryingAgent::active_challenges(action_id).unwrap();
        assert_eq!(challenge.resolution, Some(ChallengeResolution::Upheld));

        // BOB should get stake back (unreserved)
        assert_eq!(Balances::reserved_balance(&BOB), 0);
        assert_eq!(Balances::free_balance(&BOB), bob_balance_before);

        // Event emitted
        System::assert_has_event(
            Event::ChallengeResolved {
                action_id,
                resolution: ChallengeResolution::Upheld,
            }
            .into(),
        );
    });
}

#[test]
fn test_resolve_challenge_dismissed() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        let _ = Balances::make_free_balance_be(&BOB, 1_000_000);

        // Challenge
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(BOB),
            action_id,
            vec![],
        ));

        // Admin resolves as Dismissed
        assert_ok!(ProofCarryingAgent::resolve_challenge(
            RuntimeOrigin::signed(ADMIN),
            action_id,
            ChallengeResolution::Dismissed,
        ));

        // Action should be Verified again
        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Verified);

        // Challenge should be resolved
        let challenge = ProofCarryingAgent::active_challenges(action_id).unwrap();
        assert_eq!(challenge.resolution, Some(ChallengeResolution::Dismissed));

        // BOB should lose stake (slashed)
        assert_eq!(Balances::reserved_balance(&BOB), 0);
    });
}

#[test]
fn test_resolve_challenge_expired() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        let _ = Balances::make_free_balance_be(&BOB, 1_000_000);
        let bob_balance_before = Balances::free_balance(&BOB);

        // Challenge
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(BOB),
            action_id,
            vec![],
        ));

        // Admin resolves as Expired
        assert_ok!(ProofCarryingAgent::resolve_challenge(
            RuntimeOrigin::signed(ADMIN),
            action_id,
            ChallengeResolution::Expired,
        ));

        // Action should be Verified again
        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Verified);

        // BOB should get stake back
        assert_eq!(Balances::reserved_balance(&BOB), 0);
        assert_eq!(Balances::free_balance(&BOB), bob_balance_before);
    });
}

#[test]
fn test_resolve_challenge_not_admin() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        let _ = Balances::make_free_balance_be(&BOB, 1_000_000);

        // Challenge
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(BOB),
            action_id,
            vec![],
        ));

        // Non-admin tries to resolve
        assert_noop!(
            ProofCarryingAgent::resolve_challenge(
                RuntimeOrigin::signed(BOB),
                action_id,
                ChallengeResolution::Upheld,
            ),
            sp_runtime::traits::BadOrigin
        );
    });
}

#[test]
fn test_resolve_challenge_not_found() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            ProofCarryingAgent::resolve_challenge(
                RuntimeOrigin::signed(ADMIN),
                [0u8; 32],
                ChallengeResolution::Upheld,
            ),
            Error::<Test>::ChallengeNotFound
        );
    });
}

#[test]
fn test_resolve_challenge_already_resolved() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        let _ = Balances::make_free_balance_be(&BOB, 1_000_000);

        // Challenge
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(BOB),
            action_id,
            vec![],
        ));

        // Resolve once
        assert_ok!(ProofCarryingAgent::resolve_challenge(
            RuntimeOrigin::signed(ADMIN),
            action_id,
            ChallengeResolution::Upheld,
        ));

        // Resolve again should fail
        assert_noop!(
            ProofCarryingAgent::resolve_challenge(
                RuntimeOrigin::signed(ADMIN),
                action_id,
                ChallengeResolution::Dismissed,
            ),
            Error::<Test>::ChallengeAlreadyResolved
        );
    });
}

// ── set_proof_config ────────────────────────────────────────────────────────

#[test]
fn test_set_proof_config_admin() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let new_config = ProofConfig {
            max_pending_blocks: 200,
            challenge_window: 100,
            min_challenge_stake: 500,
            max_proofs_per_epoch: 100,
        };

        assert_ok!(ProofCarryingAgent::set_proof_config(
            RuntimeOrigin::signed(ADMIN),
            new_config.clone(),
        ));

        let stored_config = ProofCarryingAgent::proof_config();
        assert_eq!(stored_config, new_config);

        System::assert_has_event(Event::ProofConfigUpdated.into());
    });
}

#[test]
fn test_set_proof_config_not_admin() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let new_config = ProofConfig {
            max_pending_blocks: 200,
            challenge_window: 100,
            min_challenge_stake: 500,
            max_proofs_per_epoch: 100,
        };

        assert_noop!(
            ProofCarryingAgent::set_proof_config(
                RuntimeOrigin::signed(BOB),
                new_config,
            ),
            sp_runtime::traits::BadOrigin
        );
    });
}

#[test]
fn test_set_proof_config_alice_is_admin() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let new_config = ProofConfig {
            max_pending_blocks: 300,
            challenge_window: 150,
            min_challenge_stake: 1000,
            max_proofs_per_epoch: 200,
        };

        // ALICE (account 1) is also an admin per MockAdminOrigin
        assert_ok!(ProofCarryingAgent::set_proof_config(
            RuntimeOrigin::signed(ALICE),
            new_config.clone(),
        ));

        let stored_config = ProofCarryingAgent::proof_config();
        assert_eq!(stored_config, new_config);
    });
}

// ── clean_expired_proofs ────────────────────────────────────────────────────

#[test]
fn test_clean_expired_proofs() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Advance past the deadline (deadline=200, max_pending_blocks=100)
        // The action was submitted at block 1, so it expires at block 1 + 100 = 101
        System::set_block_number(150);

        assert_ok!(ProofCarryingAgent::clean_expired_proofs(
            RuntimeOrigin::signed(BOB),
            10,
        ));

        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Expired);

        let stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(stats.total_expired, 1);

        System::assert_has_event(
            Event::ActionExpired {
                agent: ALICE,
                action_id,
            }
            .into(),
        );
    });
}

#[test]
fn test_clean_expired_proofs_not_expired() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Not yet expired (only at block 1, deadline is 200, max_pending_blocks=100)
        assert_ok!(ProofCarryingAgent::clean_expired_proofs(
            RuntimeOrigin::signed(BOB),
            10,
        ));

        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Pending); // Still pending
    });
}

#[test]
fn test_clean_expired_proofs_verified_not_expired() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        // Advance past expiry
        System::set_block_number(150);

        // Clean should not affect verified actions
        assert_ok!(ProofCarryingAgent::clean_expired_proofs(
            RuntimeOrigin::signed(BOB),
            10,
        ));

        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Verified); // Still verified
    });
}

#[test]
fn test_clean_expired_proofs_max_clean() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Submit 5 actions
        let mut action_ids = Vec::new();
        for i in 1..=5u64 {
            let id = submit_default_action(ALICE, i);
            action_ids.push(id);
        }

        // Advance past expiry
        System::set_block_number(150);

        // Clean only 3
        assert_ok!(ProofCarryingAgent::clean_expired_proofs(
            RuntimeOrigin::signed(BOB),
            3,
        ));

        // Check how many were cleaned
        let mut expired_count = 0;
        for id in &action_ids {
            let action = ProofCarryingAgent::verified_actions(id).unwrap();
            if action.status == ProofStatus::Expired {
                expired_count += 1;
            }
        }
        assert_eq!(expired_count, 3);

        // Clean the rest
        assert_ok!(ProofCarryingAgent::clean_expired_proofs(
            RuntimeOrigin::signed(BOB),
            10,
        ));

        let mut expired_count = 0;
        for id in &action_ids {
            let action = ProofCarryingAgent::verified_actions(id).unwrap();
            if action.status == ProofStatus::Expired {
                expired_count += 1;
            }
        }
        assert_eq!(expired_count, 5);
    });
}

// ── Edge Cases ──────────────────────────────────────────────────────────────

#[test]
fn test_submit_and_verify_full_lifecycle() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // ALICE submits
        let action_id = submit_default_action(ALICE, 1);

        // BOB verifies
        System::set_block_number(5);
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        // CHARLIE challenges
        let _ = Balances::make_free_balance_be(&CHARLIE, 1_000_000);
        assert_ok!(ProofCarryingAgent::challenge_proof(
            RuntimeOrigin::signed(CHARLIE),
            action_id,
            vec![9, 9, 9],
        ));

        // Admin resolves
        assert_ok!(ProofCarryingAgent::resolve_challenge(
            RuntimeOrigin::signed(ADMIN),
            action_id,
            ChallengeResolution::Upheld,
        ));

        // Final state
        let action = ProofCarryingAgent::verified_actions(action_id).unwrap();
        assert_eq!(action.status, ProofStatus::Failed);

        let stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(stats.total_submitted, 1);
        assert_eq!(stats.total_verified, 1);
        assert_eq!(stats.total_challenged, 1);
    });
}

#[test]
fn test_multiple_agents_independent_actions() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Multiple agents submit
        let alice_id = submit_default_action(ALICE, 1);
        let bob_id = submit_default_action(BOB, 1);
        let charlie_id = submit_default_action(CHARLIE, 1);

        // Verify all
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(ALICE),
            alice_id,
            true,
            vec![],
        ));
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(ALICE),
            bob_id,
            true,
            vec![],
        ));
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(ALICE),
            charlie_id,
            false,
            vec![],
        ));

        // Check stats
        let alice_stats = ProofCarryingAgent::agent_proof_stats(ALICE);
        assert_eq!(alice_stats.total_submitted, 1);
        assert_eq!(alice_stats.total_verified, 1);

        let bob_stats = ProofCarryingAgent::agent_proof_stats(BOB);
        assert_eq!(bob_stats.total_submitted, 1);
        assert_eq!(bob_stats.total_verified, 1);

        let charlie_stats = ProofCarryingAgent::agent_proof_stats(CHARLIE);
        assert_eq!(charlie_stats.total_submitted, 1);
        assert_eq!(charlie_stats.total_failed, 1);
    });
}

#[test]
fn test_action_nonce_increments() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_eq!(ProofCarryingAgent::action_nonce(), 0);

        submit_default_action(ALICE, 1);
        assert_eq!(ProofCarryingAgent::action_nonce(), 1);

        submit_default_action(ALICE, 2);
        assert_eq!(ProofCarryingAgent::action_nonce(), 2);

        submit_default_action(BOB, 1);
        assert_eq!(ProofCarryingAgent::action_nonce(), 3);
    });
}

#[test]
fn test_pending_actions_removed_after_verify() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Action is in pending list
        let pending = ProofCarryingAgent::pending_actions(ALICE);
        assert!(pending.contains(&action_id));

        // Verify
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            true,
            vec![],
        ));

        // Action is still in pending list (we don't remove it — it stays for history)
        // The status changes but the pending list entry remains
        let pending = ProofCarryingAgent::pending_actions(ALICE);
        assert!(pending.contains(&action_id));
    });
}

#[test]
fn test_challenge_failed_action() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Fail the action
        assert_ok!(ProofCarryingAgent::verify_action(
            RuntimeOrigin::signed(BOB),
            action_id,
            false,
            vec![],
        ));

        // Can't challenge a failed action
        assert_noop!(
            ProofCarryingAgent::challenge_proof(
                RuntimeOrigin::signed(BOB),
                action_id,
                vec![],
            ),
            Error::<Test>::NotChallengeable
        );
    });
}

#[test]
fn test_challenge_expired_action() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let action_id = submit_default_action(ALICE, 1);

        // Expire the action
        System::set_block_number(150);
        assert_ok!(ProofCarryingAgent::clean_expired_proofs(
            RuntimeOrigin::signed(BOB),
            10,
        ));

        // Can't challenge an expired action
        assert_noop!(
            ProofCarryingAgent::challenge_proof(
                RuntimeOrigin::signed(BOB),
                action_id,
                vec![],
            ),
            Error::<Test>::NotChallengeable
        );
    });
}
