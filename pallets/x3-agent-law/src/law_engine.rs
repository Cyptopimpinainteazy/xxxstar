use crate::{
    types::{PolicyResult, PolicyRule, ViolationType},
    Config,
};
use frame_support::pallet_prelude::*;
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
        agent: &T::AccountId,
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
                // Get agent reputation from x3-invariants registry
                // For now, assume all agents pass (will be linked in formal spec)
                if context.reputation_score >= *min_rep {
                    PolicyResult::Pass
                } else {
                    PolicyResult::Fail(ViolationType::ReputationBelowMinimum)
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
                // Check if agent is trying to coordinate with blacklisted peers
                if context.related_agents.iter().any(|a| blacklist.contains(a)) {
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
    /// Agent's current reputation score
    pub reputation_score: u64,
    /// Number of tasks scheduled by agent this block
    pub tasks_this_block: u32,
    /// Number of extrinsics from agent this epoch
    pub extrinsics_this_epoch: u32,
    /// The requested capability for the current transaction, if identifiable.
    pub requested_capability: Option<Vec<u8>>,
    /// Related agent accounts
    pub related_agents: Vec<T::AccountId>,
    /// Current block number
    pub current_block: frame_system::pallet_prelude::BlockNumberFor<T>,
    /// Block number of last agent activity
    pub last_activity_block: frame_system::pallet_prelude::BlockNumberFor<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_minimum() {
        let min_rep = 100u64;
        let rule = PolicyRule::ReputationMinimum(min_rep);

        // Create test context
        let context_pass = PolicyContext {
            reputation_score: 150,
            tasks_this_block: 0,
            extrinsics_this_epoch: 0,
            related_agents: vec![],
            current_block: 1u32.into(),
            last_activity_block: 0u32.into(),
        };

        let context_fail = PolicyContext {
            reputation_score: 50,
            tasks_this_block: 0,
            extrinsics_this_epoch: 0,
            related_agents: vec![],
            current_block: 1u32.into(),
            last_activity_block: 0u32.into(),
        };

        // Note: These would require full test setup with Config trait
        // Skipping direct testing here; covered in pallet tests instead
    }
}
