use crate::{
    types::{PolicyResult, PolicyRule, ViolationType},
    Config,
};
use sp_std::prelude::*;

/// Core policy evaluation engine
/// Implements the logic for checking PolicyRule compliance
pub struct PolicyEngine;

impl PolicyEngine {
    /// Evaluate all policies for an agent
    /// Returns Pass if all policies satisfied, Fail(ViolationType) if any violated
    pub fn evaluate_policies<T: Config>(
        agent: &T::AccountId,
        policies: &[PolicyRule<T::AccountId>],
        context: &PolicyContext<T>,
    ) -> PolicyResult {
        for policy in policies {
            let result = Self::evaluate_rule::<T>(policy, agent, context);
            if result.is_fail() {
                return result;
            }
        }
        PolicyResult::Pass
    }

    /// Evaluate a single policy rule
    pub fn evaluate_rule<T: Config>(
        rule: &PolicyRule<T::AccountId>,
        _agent: &T::AccountId,
        context: &PolicyContext<T>,
    ) -> PolicyResult {
        match rule {
            PolicyRule::CapabilityAllowed(capabilities) => {
                if let Some(requested) = &context.requested_capability {
                    if capabilities.iter().any(|c| c == requested) {
                        PolicyResult::Pass
                    } else {
                        PolicyResult::Fail(ViolationType::CapabilityNotPermitted)
                    }
                } else {
                    // No capability context available for this call.
                    // Treat as unconstrained for non-capability actions.
                    PolicyResult::Pass
                }
            }

            PolicyRule::ReputationMinimum(min_rep) => {
                // A minimum that cannot be evaluated must not pass. The context
                // used to carry a hardcoded `100` ("assume all agents pass"), so
                // this rule was decorative for every agent.
                match context.reputation_score {
                    Some(score) if score >= *min_rep => PolicyResult::Pass,
                    Some(_) => PolicyResult::Fail(ViolationType::ReputationBelowMinimum),
                    None => PolicyResult::Fail(ViolationType::ReputationUnknown),
                }
            }

            PolicyRule::MaxTasksPerBlock(max_tasks) => {
                if context.tasks_this_block <= *max_tasks {
                    PolicyResult::Pass
                } else {
                    PolicyResult::Fail(ViolationType::MaxTasksPerBlockExceeded)
                }
            }

            PolicyRule::NoCollusionWith(blacklist) => {
                // Same shape: the caller passed an always-empty relation list, so
                // "not blacklisted" was indistinguishable from "we have no idea".
                if !context.relations_known {
                    PolicyResult::Fail(ViolationType::CollusionCheckUnavailable)
                } else if context.related_agents.iter().any(|a| blacklist.contains(a)) {
                    PolicyResult::Fail(ViolationType::CollusionAttempted)
                } else {
                    PolicyResult::Pass
                }
            }

            PolicyRule::RateLimit(max_per_epoch) => {
                if context.extrinsics_this_epoch < *max_per_epoch {
                    PolicyResult::Pass
                } else {
                    PolicyResult::Fail(ViolationType::RateLimitExceeded)
                }
            }
        }
    }

    /// Check if agent is blacklisted
    pub fn is_blacklisted<T: Config>(
        _blacklist_expiry: Option<frame_system::pallet_prelude::BlockNumberFor<T>>,
        current_block: frame_system::pallet_prelude::BlockNumberFor<T>,
    ) -> bool {
        if let Some(expiry) = _blacklist_expiry {
            current_block < expiry
        } else {
            false
        }
    }

    /// Calculate accumulated violations
    pub fn should_auto_enforce(violation_count: u32) -> bool {
        violation_count >= 3 // Auto-blacklist on 3rd violation
    }
}

/// Policy evaluation context
/// Passed to `evaluate_policies` to provide all relevant state
pub struct PolicyContext<T: Config> {
    /// Agent's current reputation score, or `None` when no registry is wired.
    pub reputation_score: Option<u64>,
    /// Number of tasks scheduled by agent this block
    pub tasks_this_block: u32,
    /// Number of extrinsics from agent this epoch
    pub extrinsics_this_epoch: u32,
    /// The requested capability for the current transaction, if identifiable.
    pub requested_capability: Option<Vec<u8>>,
    /// Related agent accounts
    pub related_agents: Vec<T::AccountId>,
    /// Whether `related_agents` reflects a real relation set.
    ///
    /// `false` means "unknown", which a collusion policy must treat as a
    /// failure rather than as "no relations".
    pub relations_known: bool,
    /// Current block number
    pub current_block: frame_system::pallet_prelude::BlockNumberFor<T>,
    /// Block number of last agent activity
    pub last_activity_block: frame_system::pallet_prelude::BlockNumberFor<T>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::Test;

    #[test]
    fn test_reputation_minimum() {
        let min_rep = 100u64;
        let rule: PolicyRule<u64> = PolicyRule::ReputationMinimum(min_rep);

        assert!(matches!(rule, PolicyRule::ReputationMinimum(100)));
    }

    fn context(
        reputation_score: Option<u64>,
        related_agents: Vec<u64>,
        relations_known: bool,
    ) -> PolicyContext<Test> {
        PolicyContext {
            reputation_score,
            tasks_this_block: 0,
            extrinsics_this_epoch: 0,
            requested_capability: None,
            related_agents,
            relations_known,
            current_block: 1,
            last_activity_block: 0,
        }
    }

    /// A minimum that cannot be evaluated must not pass: the context used to
    /// carry a hardcoded `100`, so this rule never fired for any agent.
    #[test]
    fn unknown_reputation_does_not_satisfy_a_minimum() {
        let rule: PolicyRule<u64> = PolicyRule::ReputationMinimum(100);
        let unknown = context(None, vec![], false);
        let result = PolicyEngine::evaluate_rule::<Test>(&rule, &1u64, &unknown);
        assert!(
            matches!(result, PolicyResult::Fail(ViolationType::ReputationUnknown)),
            "unknown reputation must fail closed, got {result:?}"
        );

        // A measured score still evaluates normally.
        let measured = context(Some(99), vec![], false);
        assert!(matches!(
            PolicyEngine::evaluate_rule::<Test>(&rule, &1u64, &measured),
            PolicyResult::Fail(ViolationType::ReputationBelowMinimum)
        ));
        let good = context(Some(100), vec![], false);
        assert!(PolicyEngine::evaluate_rule::<Test>(&rule, &1u64, &good).is_pass());
    }

    /// `NoCollusionWith` was evaluated against an always-empty relation list, so
    /// "no known relations" was indistinguishable from "clean".
    #[test]
    fn unknown_relations_do_not_satisfy_a_collusion_policy() {
        let rule: PolicyRule<u64> = PolicyRule::NoCollusionWith(vec![2, 3]);

        let unknown = context(Some(100), vec![], false);
        assert!(matches!(
            PolicyEngine::evaluate_rule::<Test>(&rule, &1u64, &unknown),
            PolicyResult::Fail(ViolationType::CollusionCheckUnavailable)
        ));

        let clean = context(Some(100), vec![9], true);
        assert!(PolicyEngine::evaluate_rule::<Test>(&rule, &1u64, &clean).is_pass());

        let colluding = context(Some(100), vec![2], true);
        assert!(matches!(
            PolicyEngine::evaluate_rule::<Test>(&rule, &1u64, &colluding),
            PolicyResult::Fail(ViolationType::CollusionAttempted)
        ));
    }
}
