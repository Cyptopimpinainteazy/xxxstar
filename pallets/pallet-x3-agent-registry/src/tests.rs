//! Tests for the X3 Unified Agent Registry pallet.

use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok, BoundedVec};

// ============================================================================
// Registration Tests
// ============================================================================

#[test]
fn register_agent_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"TestAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name.clone(),
            metadata,
            AgentKind::AutonomousAgent,
        ));

        let agent = AgentRegistry::agents(0).unwrap();
        assert_eq!(agent.controller, ALICE);
        assert_eq!(agent.operator, OPERATOR1);
        assert_eq!(agent.name, name);
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.reputation, 100);
        assert_eq!(agent.kind, AgentKind::AutonomousAgent);

        assert_eq!(AgentRegistry::total_agents(), 1);
        assert_eq!(AgentRegistry::active_agents(), 1);

        System::assert_has_event(RuntimeEvent::AgentRegistry(Event::AgentRegistered {
            agent_id: 0,
            controller: ALICE,
            operator: OPERATOR1,
            kind: AgentKind::AutonomousAgent,
        }));
    });
}

#[test]
fn register_agent_reserves_deposit() {
    new_test_ext().execute_with(|| {
        let initial_balance = Balances::free_balance(ALICE);
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_eq!(Balances::reserved_balance(ALICE), 1000);
        assert_eq!(Balances::free_balance(ALICE), initial_balance - 1000);
    });
}

#[test]
fn cannot_register_with_same_operator() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name.clone(),
            metadata.clone(),
            AgentKind::AutonomousAgent,
        ));

        assert_noop!(
            AgentRegistry::register_agent(
                RuntimeOrigin::signed(BOB),
                OPERATOR1,
                name,
                metadata,
                AgentKind::AutonomousAgent,
            ),
            Error::<Test>::NotAuthorized
        );
    });
}

#[test]
fn cannot_exceed_max_agents_per_controller() {
    new_test_ext().execute_with(|| {
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        // Register 10 agents (max)
        for i in 0..10u64 {
            let name: BoundedVec<_, _> = format!("Agent{}", i).as_bytes().to_vec().try_into().unwrap();
            let operator = OPERATOR1 + i as u64;
            assert_ok!(AgentRegistry::register_agent(
                RuntimeOrigin::signed(ALICE),
                operator,
                name,
                metadata.clone(),
                AgentKind::AutonomousAgent,
            ));
        }

        // 11th should fail
        let name: BoundedVec<_, _> = b"AgentOverflow".to_vec().try_into().unwrap();
        assert_noop!(
            AgentRegistry::register_agent(
                RuntimeOrigin::signed(ALICE),
                999,
                name,
                metadata,
                AgentKind::AutonomousAgent,
            ),
            Error::<Test>::TooManyAgents
        );
    });
}

// ============================================================================
// Atlas ID Binding Tests
// ============================================================================

#[test]
fn bind_atlas_id_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::bind_atlas_id(
            RuntimeOrigin::signed(ALICE),
            0,
            42,
        ));

        let agent = AgentRegistry::agents(0).unwrap();
        assert_eq!(agent.atlas_id, Some(42));
        assert_eq!(AgentRegistry::atlas_to_agent(42), Some(0));
    });
}

#[test]
fn cannot_bind_atlas_id_twice() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::bind_atlas_id(
            RuntimeOrigin::signed(ALICE),
            0,
            42,
        ));

        assert_noop!(
            AgentRegistry::bind_atlas_id(RuntimeOrigin::signed(ALICE), 0, 43),
            Error::<Test>::AtlasIdAlreadyBound
        );
    });
}

// ============================================================================
// Operator Update Tests
// ============================================================================

#[test]
fn update_operator_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::update_operator(
            RuntimeOrigin::signed(ALICE),
            0,
            OPERATOR2,
        ));

        let agent = AgentRegistry::agents(0).unwrap();
        assert_eq!(agent.operator, OPERATOR2);
        assert_eq!(AgentRegistry::operator_agent(OPERATOR2), Some(0));
        assert_eq!(AgentRegistry::operator_agent(OPERATOR1), None);
    });
}

// ============================================================================
// Permission Tests
// ============================================================================

#[test]
fn update_permissions_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        let new_perms = AgentPermissions {
            can_deploy: true,
            can_stake: true,
            can_vote: true,
            can_trade: true,
            can_transfer: true,
            can_call_contracts: true,
            can_submit_proofs: true,
            can_validate: true,
        };

        assert_ok!(AgentRegistry::update_permissions(
            RuntimeOrigin::signed(ALICE),
            0,
            new_perms.clone(),
        ));

        let stored_perms = AgentRegistry::permissions(0);
        assert_eq!(stored_perms.can_deploy, true);
        assert_eq!(stored_perms.can_validate, true);
    });
}

// ============================================================================
// Quota Tests
// ============================================================================

#[test]
fn update_quota_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::update_quota(
            RuntimeOrigin::root(),
            0,
            2_000_000,
            1_000_000,
            200_000_000,
            100_000_000,
        ));

        let quota = AgentRegistry::quotas(0).unwrap();
        assert_eq!(quota.gas_per_block, 2_000_000);
        assert_eq!(quota.compute_per_block, 1_000_000);
    });
}

// ============================================================================
// Lifecycle Tests
// ============================================================================

#[test]
fn suspend_and_reactivate_agent_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<_, _> = b"misbehavior".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_eq!(AgentRegistry::active_agents(), 1);

        assert_ok!(AgentRegistry::suspend_agent(
            RuntimeOrigin::root(),
            0,
            reason,
        ));

        let agent = AgentRegistry::agents(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Suspended);
        assert_eq!(AgentRegistry::active_agents(), 0);

        assert_ok!(AgentRegistry::reactivate_agent(
            RuntimeOrigin::root(),
            0,
        ));

        let agent = AgentRegistry::agents(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(AgentRegistry::active_agents(), 1);
    });
}

#[test]
fn terminate_agent_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::terminate_agent(
            RuntimeOrigin::signed(ALICE),
            0,
        ));

        let agent = AgentRegistry::agents(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Terminated);
        assert_eq!(AgentRegistry::active_agents(), 0);
        assert_eq!(AgentRegistry::total_agents(), 1);

        // Deposit should be unreserved
        assert_eq!(Balances::reserved_balance(ALICE), 0);
    });
}

// ============================================================================
// Policy Tests
// ============================================================================

#[test]
fn register_policy_works() {
    new_test_ext().execute_with(|| {
        let policies = vec![
            PolicyRule::ReputationMinimum(50u64),
            PolicyRule::MaxTasksPerBlock(5u32),
        ];

        assert_ok!(AgentRegistry::register_policy(
            RuntimeOrigin::root(),
            ALICE,
            policies,
        ));

        let stored = AgentRegistry::agent_policies(ALICE);
        assert_eq!(stored.len(), 2);
    });
}

#[test]
fn cannot_register_too_many_policies() {
    new_test_ext().execute_with(|| {
        let policies = vec![PolicyRule::ReputationMinimum(1u64); 17];

        assert_noop!(
            AgentRegistry::register_policy(RuntimeOrigin::root(), ALICE, policies),
            Error::<Test>::TooManyPolicies
        );
    });
}

// ============================================================================
// Staking Tests
// ============================================================================

#[test]
fn post_bond_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::post_bond(
            RuntimeOrigin::signed(ALICE),
            2_000_000,
            None,
        ));

        assert_eq!(Balances::reserved_balance(ALICE), 1000 + 2_000_000);
    });
}

#[test]
fn cannot_post_bond_below_minimum() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentRegistry::post_bond(RuntimeOrigin::signed(ALICE), 500, None),
            Error::<Test>::BondTooSmall
        );
    });
}

#[test]
fn release_bond_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::post_bond(
            RuntimeOrigin::signed(ALICE),
            2_000_000,
            None,
        ));

        // Find the bond ID
        let bonds = AgentRegistry::bonds_by_agent(ALICE);
        assert_eq!(bonds.len(), 1);

        assert_ok!(AgentRegistry::release_bond(
            RuntimeOrigin::root(),
            bonds[0],
        ));

        let bond = AgentRegistry::bonds(bonds[0]).unwrap();
        assert_eq!(bond.status, BondStatus::Released);
    });
}

#[test]
fn slash_bond_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::post_bond(
            RuntimeOrigin::signed(ALICE),
            2_000_000,
            None,
        ));

        let bonds = AgentRegistry::bonds_by_agent(ALICE);
        let reason = b"invalid_proof".to_vec();

        assert_ok!(AgentRegistry::slash_bond(
            RuntimeOrigin::root(),
            bonds[0],
            2, // Major severity (25%)
            reason,
        ));

        let bond = AgentRegistry::bonds(bonds[0]).unwrap();
        assert_eq!(bond.status, BondStatus::FullySlashed);

        // 25% of 2,000,000 = 500,000 should be slashed
        let treasury_balance = Balances::free_balance(TREASURY);
        assert!(treasury_balance > 1_000_000_000); // Treasury got slashed funds
    });
}

// ============================================================================
// Resource Consumption Tests
// ============================================================================

#[test]
fn record_consumption_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::record_consumption(
            RuntimeOrigin::signed(ALICE),
            0,
            5000,
            3000,
        ));

        let activity = AgentRegistry::activity(0);
        assert_eq!(activity.gas_used_block, 5000);
        assert_eq!(activity.compute_used_block, 3000);
        assert_eq!(activity.total_actions, 1);
    });
}

#[test]
fn record_consumption_rejects_quota_exceeded() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        // Exceed default gas_per_block (1,000,000)
        assert_noop!(
            AgentRegistry::record_consumption(
                RuntimeOrigin::signed(ALICE),
                0,
                2_000_000,
                0,
            ),
            Error::<Test>::QuotaExceeded
        );
    });
}

// ============================================================================
// Reputation Tests
// ============================================================================

#[test]
fn update_reputation_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::update_reputation(
            RuntimeOrigin::root(),
            0,
            150,
        ));

        let agent = AgentRegistry::agents(0).unwrap();
        assert_eq!(agent.reputation, 150);
    });
}

#[test]
fn cannot_set_reputation_out_of_bounds() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_noop!(
            AgentRegistry::update_reputation(RuntimeOrigin::root(), 0, 300),
            Error::<Test>::ReputationOutOfBounds
        );
    });
}

// ============================================================================
// Rewards Distribution Tests
// ============================================================================

#[test]
fn distribute_rewards_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        let alice_balance_before = Balances::free_balance(ALICE);

        assert_ok!(AgentRegistry::distribute_rewards(
            RuntimeOrigin::root(),
            0,
            5000,
        ));

        let alice_balance_after = Balances::free_balance(ALICE);
        assert_eq!(alice_balance_after, alice_balance_before + 5000);
    });
}

// ============================================================================
// Action Emission Tests
// ============================================================================

#[test]
fn emit_action_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let data: BoundedVec<_, _> = b"trade_executed".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::emit_action(
            RuntimeOrigin::signed(ALICE),
            0,
            ActionType::ExecuteTrade,
            data,
        ));

        System::assert_has_event(RuntimeEvent::AgentRegistry(Event::AgentAction {
            agent_id: 0,
            action_type: ActionType::ExecuteTrade,
            data: b"trade_executed".to_vec().try_into().unwrap(),
        }));
    });
}

// ============================================================================
// Epoch Tests
// ============================================================================

#[test]
fn epoch_advances_correctly() {
    new_test_ext().execute_with(|| {
        assert_eq!(AgentRegistry::current_epoch(), 0);

        // Advance past epoch boundary (100 blocks)
        // Epoch fires at block 100 (when on_initialize sees n >= last_epoch + blocks_per_epoch)
        run_to_block(101);

        assert_eq!(AgentRegistry::current_epoch(), 1);
        // last_epoch_block is set to 100 because on_initialize at block 100 triggers the epoch
        assert_eq!(AgentRegistry::last_epoch_block(), 100);

        System::assert_has_event(RuntimeEvent::AgentRegistry(Event::EpochStarted {
            epoch: 1,
            block: 100,
        }));
    });
}

// ============================================================================
// Blacklist Tests
// ============================================================================

#[test]
fn remove_blacklist_works() {
    new_test_ext().execute_with(|| {
        // First blacklist ALICE via internal function
        assert_ok!(AgentRegistry::blacklist_agent(&ALICE, 100));

        assert!(AgentRegistry::blacklist_expiry(ALICE).is_some());

        assert_ok!(AgentRegistry::remove_blacklist(
            RuntimeOrigin::root(),
            ALICE,
        ));

        assert!(AgentRegistry::blacklist_expiry(ALICE).is_none());
    });
}

// ============================================================================
// Helper Function Tests
// ============================================================================

#[test]
fn has_permission_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        // Default permissions: can_trade=true, can_deploy=false
        assert!(AgentRegistry::has_permission(0, PermissionType::Trade));
        assert!(!AgentRegistry::has_permission(0, PermissionType::Deploy));
    });
}

#[test]
fn agent_id_for_account_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_eq!(AgentRegistry::agent_id_for_account(&OPERATOR1), Some(0));
        assert_eq!(AgentRegistry::agent_id_for_account(&BOB), None);
    });
}

#[test]
fn calculate_penalty_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(AgentRegistry::calculate_penalty(&SlashingReason::InvalidProof), 500);
        assert_eq!(AgentRegistry::calculate_penalty(&SlashingReason::TaskGriefing), 200);
        assert_eq!(AgentRegistry::calculate_penalty(&SlashingReason::CollusionDetected), 800);
        assert_eq!(AgentRegistry::calculate_penalty(&SlashingReason::PolicyViolation), 350);
        assert_eq!(AgentRegistry::calculate_penalty(&SlashingReason::RepeatOffender), 1200);
        assert_eq!(AgentRegistry::calculate_penalty(&SlashingReason::BondExpired), 100);
    });
}

// ============================================================================
// Error Path Tests
// ============================================================================

#[test]
fn cannot_act_on_nonexistent_agent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentRegistry::update_operator(RuntimeOrigin::signed(ALICE), 999, OPERATOR2),
            Error::<Test>::AgentNotFound
        );
    });
}

#[test]
fn non_controller_cannot_update_operator() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_noop!(
            AgentRegistry::update_operator(RuntimeOrigin::signed(BOB), 0, OPERATOR2),
            Error::<Test>::NotController
        );
    });
}

#[test]
fn cannot_suspend_already_suspended_agent() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<_, _> = b"test".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::suspend_agent(
            RuntimeOrigin::root(),
            0,
            reason.clone(),
        ));

        assert_noop!(
            AgentRegistry::suspend_agent(RuntimeOrigin::root(), 0, reason),
            Error::<Test>::InvalidStatusTransition
        );
    });
}

#[test]
fn cannot_terminate_already_terminated_agent() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::terminate_agent(
            RuntimeOrigin::signed(ALICE),
            0,
        ));

        assert_noop!(
            AgentRegistry::terminate_agent(RuntimeOrigin::signed(ALICE), 0),
            Error::<Test>::AgentTerminated
        );
    });
}

#[test]
fn cannot_slash_nonexistent_bond() {
    new_test_ext().execute_with(|| {
        let reason = b"test".to_vec();
        assert_noop!(
            AgentRegistry::slash_bond(
                RuntimeOrigin::root(),
                H256::default(),
                1,
                reason,
            ),
            Error::<Test>::BondNotFound
        );
    });
}

#[test]
fn cannot_record_consumption_for_suspended_agent() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<_, _> = b"suspend".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::suspend_agent(
            RuntimeOrigin::root(),
            0,
            reason,
        ));

        assert_noop!(
            AgentRegistry::record_consumption(RuntimeOrigin::signed(ALICE), 0, 100, 100),
            Error::<Test>::AgentNotActive
        );
    });
}

// ============================================================================
// Bond Expiry Tests (on_finalize)
// ============================================================================

#[test]
fn bond_expires_in_on_finalize() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_ok!(AgentRegistry::post_bond(
            RuntimeOrigin::signed(ALICE),
            2_000_000,
            None,
        ));

        // FinalityWindow is 100 blocks, so bond expires at block 101
        // Advance past expiry
        run_to_block(150);

        // Bond should be expired
        let bonds = AgentRegistry::bonds_by_agent(ALICE);
        if let Some(bond) = AgentRegistry::bonds(bonds[0]) {
            assert_eq!(bond.status, BondStatus::Expired);
        }
    });
}

// ============================================================================
// Reward Pool Tests
// ============================================================================

#[test]
fn set_proof_reward_works() {
    new_test_ext().execute_with(|| {
        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: true,
        };

        assert_ok!(AgentRegistry::set_proof_reward(
            RuntimeOrigin::root(),
            config.clone(),
        ));

        let stored = AgentRegistry::proof_reward_config();
        assert_eq!(stored.base_reward, 1000);
        assert_eq!(stored.verification_bonus, 500);
        assert_eq!(stored.challenge_resolution_bonus, 250);
        assert!(stored.enabled);

        System::assert_has_event(RuntimeEvent::AgentRegistry(Event::ProofRewardConfigUpdated {
            config,
        }));
    });
}

#[test]
fn set_proof_reward_requires_admin() {
    new_test_ext().execute_with(|| {
        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: true,
        };

        assert_noop!(
            AgentRegistry::set_proof_reward(
                RuntimeOrigin::signed(ALICE),
                config,
            ),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn fund_reward_pool_works() {
    new_test_ext().execute_with(|| {
        let alice_balance_before = Balances::free_balance(ALICE);
        let treasury_before = Balances::free_balance(TREASURY);

        assert_ok!(AgentRegistry::fund_reward_pool(
            RuntimeOrigin::signed(ALICE),
            50_000,
        ));

        // Alice's balance decreased
        assert_eq!(Balances::free_balance(ALICE), alice_balance_before - 50_000);
        // Treasury received the funds
        assert_eq!(Balances::free_balance(TREASURY), treasury_before + 50_000);
        // Total reward pool tracked
        assert_eq!(AgentRegistry::total_reward_pool(), 50_000);

        System::assert_has_event(RuntimeEvent::AgentRegistry(Event::RewardPoolFunded {
            from: ALICE,
            amount: 50_000,
            new_total: 50_000,
        }));
    });
}

#[test]
fn fund_reward_pool_multiple_times() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistry::fund_reward_pool(
            RuntimeOrigin::signed(ALICE),
            30_000,
        ));
        assert_eq!(AgentRegistry::total_reward_pool(), 30_000);

        assert_ok!(AgentRegistry::fund_reward_pool(
            RuntimeOrigin::signed(BOB),
            20_000,
        ));
        assert_eq!(AgentRegistry::total_reward_pool(), 50_000);
    });
}

#[test]
fn claim_rewards_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        // Fund the reward pool
        assert_ok!(AgentRegistry::fund_reward_pool(
            RuntimeOrigin::signed(BOB),
            100_000,
        ));

        // Set up reward config
        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: true,
        };
        assert_ok!(AgentRegistry::set_proof_reward(
            RuntimeOrigin::root(),
            config,
        ));

        // Reward the agent via internal helper
        let reason: BoundedVec<_, _> = b"proof_verified".to_vec().try_into().unwrap();
        assert_ok!(AgentRegistry::reward_agent_for_proof(0, reason));

        // Agent should have accumulated rewards
        assert_eq!(AgentRegistry::agent_reward_pool(0), 1500); // base + verification

        // Claim rewards
        let alice_balance_before = Balances::free_balance(ALICE);
        assert_ok!(AgentRegistry::claim_rewards(
            RuntimeOrigin::signed(ALICE),
            0,
        ));

        // Alice received the reward from treasury
        assert_eq!(Balances::free_balance(ALICE), alice_balance_before + 1500);
        // Agent reward pool should be cleared
        assert_eq!(AgentRegistry::agent_reward_pool(0), 0);
        // Total reward pool decreased
        assert_eq!(AgentRegistry::total_reward_pool(), 100_000 - 1500);

        System::assert_has_event(RuntimeEvent::AgentRegistry(Event::RewardsClaimed {
            agent_id: 0,
            recipient: ALICE,
            amount: 1500,
        }));
    });
}

#[test]
fn claim_rewards_fails_with_no_rewards() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        assert_noop!(
            AgentRegistry::claim_rewards(RuntimeOrigin::signed(ALICE), 0),
            Error::<Test>::NoRewardsToClaim
        );
    });
}

#[test]
fn claim_rewards_fails_with_not_controller() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        // Fund the pool and set config
        assert_ok!(AgentRegistry::fund_reward_pool(
            RuntimeOrigin::signed(BOB),
            100_000,
        ));
        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: true,
        };
        assert_ok!(AgentRegistry::set_proof_reward(
            RuntimeOrigin::root(),
            config,
        ));

        // Reward the agent
        let reason: BoundedVec<_, _> = b"proof_verified".to_vec().try_into().unwrap();
        assert_ok!(AgentRegistry::reward_agent_for_proof(0, reason));

        // BOB is not the controller (ALICE is)
        assert_noop!(
            AgentRegistry::claim_rewards(RuntimeOrigin::signed(BOB), 0),
            Error::<Test>::NotController
        );
    });
}

#[test]
fn reward_agent_for_proof_works() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        // Fund the pool
        assert_ok!(AgentRegistry::fund_reward_pool(
            RuntimeOrigin::signed(BOB),
            100_000,
        ));

        // Set up reward config
        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: true,
        };
        assert_ok!(AgentRegistry::set_proof_reward(
            RuntimeOrigin::root(),
            config,
        ));

        // Reward the agent
        let reason: BoundedVec<_, _> = b"proof_verified".to_vec().try_into().unwrap();
        assert_ok!(AgentRegistry::reward_agent_for_proof(0, reason.clone()));

        // Agent reward pool should have base + verification bonus
        assert_eq!(AgentRegistry::agent_reward_pool(0), 1500);

        // Distribution history should be recorded
        let history = AgentRegistry::reward_distribution_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].agent_id, 0);
        assert_eq!(history[0].recipient, ALICE);
        assert_eq!(history[0].amount, 1500);
        assert_eq!(history[0].reason, reason);

        System::assert_has_event(RuntimeEvent::AgentRegistry(Event::ProofRewardDistributed {
            agent_id: 0,
            amount: 1500,
            reason,
        }));
    });
}

#[test]
fn reward_agent_for_proof_fails_when_disabled() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        // Set config with enabled = false
        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: false,
        };
        assert_ok!(AgentRegistry::set_proof_reward(
            RuntimeOrigin::root(),
            config,
        ));

        let reason: BoundedVec<_, _> = b"proof_verified".to_vec().try_into().unwrap();
        assert_noop!(
            AgentRegistry::reward_agent_for_proof(0, reason),
            Error::<Test>::PermissionDenied
        );
    });
}

#[test]
fn reward_agent_for_proof_fails_when_pool_insufficient() {
    new_test_ext().execute_with(|| {
        let name: BoundedVec<_, _> = b"Agent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<_, _> = b"{}".to_vec().try_into().unwrap();

        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(ALICE),
            OPERATOR1,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        // Set config with enabled = true but don't fund the pool
        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: true,
        };
        assert_ok!(AgentRegistry::set_proof_reward(
            RuntimeOrigin::root(),
            config,
        ));

        let reason: BoundedVec<_, _> = b"proof_verified".to_vec().try_into().unwrap();
        assert_noop!(
            AgentRegistry::reward_agent_for_proof(0, reason),
            Error::<Test>::RewardPoolInsufficient
        );
    });
}

#[test]
fn reward_agent_for_proof_fails_for_nonexistent_agent() {
    new_test_ext().execute_with(|| {
        // Fund the pool
        assert_ok!(AgentRegistry::fund_reward_pool(
            RuntimeOrigin::signed(ALICE),
            100_000,
        ));

        let config = types::ProofRewardConfig {
            base_reward: 1000,
            verification_bonus: 500,
            challenge_resolution_bonus: 250,
            enabled: true,
        };
        assert_ok!(AgentRegistry::set_proof_reward(
            RuntimeOrigin::root(),
            config,
        ));

        let reason: BoundedVec<_, _> = b"proof_verified".to_vec().try_into().unwrap();
        assert_noop!(
            AgentRegistry::reward_agent_for_proof(999, reason),
            Error::<Test>::AgentNotFound
        );
    });
}
