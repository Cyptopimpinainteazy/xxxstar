//! Benchmarking setup for pallet-x3-proof-carrying-agent

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as ProofCarryingAgent;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_runtime::traits::Hash;

const SEED: u32 = 0;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
    let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn submit_proof_carrying_action() {
        let caller = whitelisted_caller();
        let action_payload = vec![1u8; 128];
        let proof_payload = vec![2u8; 256];
        let deadline = 1000u64;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            action_payload.clone(),
            proof_payload.clone(),
            ProofKind::ZkSnark,
            1u8,
            0u8,
            deadline,
            1u64,
        );

        assert_last_event::<T>(
            Event::ActionSubmitted {
                agent: caller,
                action_id: ProofCarryingAgent::<T>::action_nonce(),
                proof_kind: ProofKind::ZkSnark,
                target_pallet: 1,
                target_call: 0,
            }
            .into(),
        );
    }

    #[benchmark]
    fn verify_action() {
        let caller = whitelisted_caller();
        let action_payload = vec![1u8; 128];
        let proof_payload = vec![2u8; 256];
        let deadline = 1000u64;

        ProofCarryingAgent::<T>::submit_proof_carrying_action(
            RawOrigin::Signed(caller.clone()).into(),
            action_payload,
            proof_payload,
            ProofKind::ZkSnark,
            1u8,
            0u8,
            deadline,
            1u64,
        )
        .unwrap();

        let action_id = ProofCarryingAgent::<T>::action_nonce();

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            action_id,
            true,
            vec![3u8; 32],
        );

        assert_last_event::<T>(
            Event::ActionVerified {
                agent: caller,
                action_id,
            }
            .into(),
        );
    }

    #[benchmark]
    fn challenge_proof() {
        let caller = whitelisted_caller();
        let challenger = account("challenger", 0, SEED);

        let action_payload = vec![1u8; 128];
        let proof_payload = vec![2u8; 256];
        let deadline = 1000u64;

        ProofCarryingAgent::<T>::submit_proof_carrying_action(
            RawOrigin::Signed(caller.clone()).into(),
            action_payload,
            proof_payload,
            ProofKind::ZkSnark,
            1u8,
            0u8,
            deadline,
            1u64,
        )
        .unwrap();

        let action_id = ProofCarryingAgent::<T>::action_nonce();

        ProofCarryingAgent::<T>::verify_action(
            RawOrigin::Signed(caller.clone()).into(),
            action_id,
            true,
            vec![],
        )
        .unwrap();

        // Fund challenger
        T::Currency::make_free_balance_be(&challenger, 1_000_000u32.into());

        #[extrinsic_call]
        _(
            RawOrigin::Signed(challenger.clone()),
            action_id,
            vec![4u8; 16],
        );

        assert_last_event::<T>(
            Event::ProofChallenged {
                action_id,
                challenger,
            }
            .into(),
        );
    }

    #[benchmark]
    fn resolve_challenge() {
        let caller = whitelisted_caller();
        let challenger = account("challenger", 0, SEED);

        let action_payload = vec![1u8; 128];
        let proof_payload = vec![2u8; 256];
        let deadline = 1000u64;

        ProofCarryingAgent::<T>::submit_proof_carrying_action(
            RawOrigin::Signed(caller.clone()).into(),
            action_payload,
            proof_payload,
            ProofKind::ZkSnark,
            1u8,
            0u8,
            deadline,
            1u64,
        )
        .unwrap();

        let action_id = ProofCarryingAgent::<T>::action_nonce();

        ProofCarryingAgent::<T>::verify_action(
            RawOrigin::Signed(caller.clone()).into(),
            action_id,
            true,
            vec![],
        )
        .unwrap();

        T::Currency::make_free_balance_be(&challenger, 1_000_000u32.into());

        ProofCarryingAgent::<T>::challenge_proof(
            RawOrigin::Signed(challenger).into(),
            action_id,
            vec![],
        )
        .unwrap();

        #[extrinsic_call]
        _(RawOrigin::Root, action_id, ChallengeResolution::Upheld);

        assert_last_event::<T>(
            Event::ChallengeResolved {
                action_id,
                resolution: ChallengeResolution::Upheld,
            }
            .into(),
        );
    }

    #[benchmark]
    fn set_proof_config() {
        let new_config = ProofConfig {
            max_pending_blocks: 200,
            challenge_window: 100,
            min_challenge_stake: 500,
            max_proofs_per_epoch: 100,
        };

        #[extrinsic_call]
        _(RawOrigin::Root, new_config.clone());

        assert_eq!(ProofCarryingAgent::<T>::proof_config(), new_config);
    }

    #[benchmark]
    fn clean_expired_proofs() {
        let caller = whitelisted_caller();
        let action_payload = vec![1u8; 128];
        let proof_payload = vec![2u8; 256];
        let deadline = 10u64;

        ProofCarryingAgent::<T>::submit_proof_carrying_action(
            RawOrigin::Signed(caller.clone()).into(),
            action_payload,
            proof_payload,
            ProofKind::ZkSnark,
            1u8,
            0u8,
            deadline,
            1u64,
        )
        .unwrap();

        // Advance past expiry
        frame_system::Pallet::<T>::set_block_number(200u32.into());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), 10u32);

        assert_last_event::<T>(
            Event::ActionExpired {
                agent: caller,
                action_id: 1u64.using_encoded(|b| {
                    let mut arr = [0u8; 32];
                    let len = b.len().min(32);
                    arr[..len].copy_from_slice(&b[..len]);
                    arr
                }),
            }
            .into(),
        );
    }

    impl_benchmark_test_suite!(
        ProofCarryingAgent,
        crate::mock::new_test_ext(),
        crate::mock::Test,
    );
}
