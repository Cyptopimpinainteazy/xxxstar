#[cfg(test)]
mod tests {
    use crate::{
        pallet::Event, types::{PolicyRule, SlashingReason}, ActivePolicies, 
        Blacklist, Error, ExtrinsicCountThisEpoch, Pallet, ViolationCount,
    };
    use frame_support::{assert_noop, assert_ok};
    use sp_runtime::BoundedVec;

    #[path = "mock.rs"]
    mod mock;

    use mock::*;

    // ========================================================================
    // Policy Registration Tests
    // ========================================================================

    #[test]
    fn test_register_single_policy() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;
            let policy = PolicyRule::ReputationMinimum(100);

            assert_ok!(AgentLaw::register_policy(
                RuntimeOrigin::signed(0),
                agent,
                vec![policy.clone()]
            ));

            let stored_policies = ActivePolicies::<Test>::get(agent);
            assert_eq!(stored_policies.len(), 1);
        });
    }

    #[test]
    fn test_register_multiple_policies() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;
            let policies = vec![
                PolicyRule::ReputationMinimum(100),
                PolicyRule::MaxTasksPerBlock(50),
                PolicyRule::RateLimit(1000),
            ];

            assert_ok!(AgentLaw::register_policy(
                RuntimeOrigin::signed(0),
                agent,
                policies.clone()
            ));

            let stored_policies = ActivePolicies::<Test>::get(agent);
            assert_eq!(stored_policies.len(), 3);
        });
    }

    #[test]
    fn test_policy_count_limit() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;
            // Try to register 17 policies (max is 16)
            let policies: Vec<_> = (0..17)
                .map(|_| PolicyRule::ReputationMinimum(100))
                .collect();

            assert_noop!(
                AgentLaw::register_policy(RuntimeOrigin::signed(0), agent, policies),
                Error::<Test>::TooManyPolicies
            );
        });
    }

    // ========================================================================
    // Slashing Tests
    // ========================================================================

    #[test]
    fn test_slash_agent_invalid_proof() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;
            let reason = SlashingReason::InvalidProof;

            assert_ok!(AgentLaw::slash_agent(
                RuntimeOrigin::signed(0),
                agent,
                reason.clone()
            ));

            let violation_count = ViolationCount::<Test>::get(&agent);
            assert_eq!(violation_count, 1);
        });
    }

    #[test]
    fn test_slash_penalties_scale_correctly() {
        ExtBuilder::build().execute_with(|| {
            // InvalidProof → 500
            assert_eq!(Pallet::<Test>::calculate_penalty(&SlashingReason::InvalidProof), 500);
            // TaskGriefing → 200
            assert_eq!(Pallet::<Test>::calculate_penalty(&SlashingReason::TaskGriefing), 200);
            // CollusionDetected → 800
            assert_eq!(Pallet::<Test>::calculate_penalty(&SlashingReason::CollusionDetected), 800);
            // PolicyViolation → 350
            assert_eq!(Pallet::<Test>::calculate_penalty(&SlashingReason::PolicyViolation), 350);
            // RepeatOffender → 1200
            assert_eq!(Pallet::<Test>::calculate_penalty(&SlashingReason::RepeatOffender), 1200);
        });
    }

    #[test]
    fn test_auto_blacklist_on_third_violation() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;

            // First violation
            assert_ok!(AgentLaw::slash_agent(
                RuntimeOrigin::signed(0),
                agent,
                SlashingReason::InvalidProof
            ));
            assert_eq!(ViolationCount::<Test>::get(&agent), 1);
            assert!(Blacklist::<Test>::get(&agent).is_none());

            // Second violation
            assert_ok!(AgentLaw::slash_agent(
                RuntimeOrigin::signed(0),
                agent,
                SlashingReason::TaskGriefing
            ));
            assert_eq!(ViolationCount::<Test>::get(&agent), 2);
            assert!(Blacklist::<Test>::get(&agent).is_none());

            // Third violation → auto-blacklist
            assert_ok!(AgentLaw::slash_agent(
                RuntimeOrigin::signed(0),
                agent,
                SlashingReason::CollusionDetected
            ));
            assert_eq!(ViolationCount::<Test>::get(&agent), 3);
            assert!(Blacklist::<Test>::get(&agent).is_some());
        });
    }

    // ========================================================================
    // Blacklist Tests
    // ========================================================================

    #[test]
    fn test_blacklist_agent() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;
            let current_block = System::block_number();

            assert_ok!(AgentLaw::blacklist_agent(&agent, 100u32));

            let blacklist_expiry = Blacklist::<Test>::get(&agent);
            assert!(blacklist_expiry.is_some());
            assert_eq!(blacklist_expiry.unwrap(), current_block + 100);
        });
    }

    #[test]
    fn test_remove_blacklist() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;

            // Blacklist agent
            assert_ok!(AgentLaw::blacklist_agent(&agent, 100u32));
            assert!(Blacklist::<Test>::get(&agent).is_some());

            // Remove blacklist
            assert_ok!(AgentLaw::remove_blacklist(RuntimeOrigin::signed(0), agent));
            assert!(Blacklist::<Test>::get(&agent).is_none());
        });
    }

    // ========================================================================
    // Rate Limit Tests
    // ========================================================================

    #[test]
    fn test_rate_limit_tracking() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;

            // Initialize extrinsic count
            ExtrinsicCountThisEpoch::<Test>::insert(&agent, 0);

            // Increment counter
            for _ in 0..10 {
                ExtrinsicCountThisEpoch::<Test>::mutate(&agent, |count| {
                    *count = count.saturating_add(1);
                });
            }

            assert_eq!(ExtrinsicCountThisEpoch::<Test>::get(&agent), 10);
        });
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_policy_registration_emits_event() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;
            let policy = PolicyRule::ReputationMinimum(100);

            System::set_block_number(1);

            assert_ok!(AgentLaw::register_policy(
                RuntimeOrigin::signed(0),
                agent,
                vec![policy]
            ));

            // Check that PolicyRegistered event was emitted
            let events = System::events();
            assert!(events.iter().any(|e| matches!(
                e.event,
                RuntimeEvent::AgentLaw(Event::PolicyRegistered { policy_count: 1, .. })
            )));
        });
    }

    #[test]
    fn test_slash_emits_event() {
        ExtBuilder::build().execute_with(|| {
            let agent = 1u64;
            let reason = SlashingReason::InvalidProof;

            System::set_block_number(1);

            assert_ok!(AgentLaw::slash_agent(
                RuntimeOrigin::signed(0),
                agent,
                reason.clone()
            ));

            // Check that AgentSlashed event was emitted
            let events = System::events();
            assert!(events.iter().any(|e| matches!(
                e.event,
                RuntimeEvent::AgentLaw(Event::AgentSlashed { penalty: 500, .. })
            )));
        });
    }
}
