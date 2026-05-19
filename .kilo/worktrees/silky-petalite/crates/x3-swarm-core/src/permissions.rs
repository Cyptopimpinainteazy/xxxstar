use crate::{AgentKind, AgentPermissionTier};

/// Permissions management for swarm agents.
pub struct Permissions {
    /// Reserved for operation-specific permission rules and audit attribution.
    _agent: AgentKind,
    tier: AgentPermissionTier,
}

impl Permissions {
    pub fn new(agent: AgentKind, tier: AgentPermissionTier) -> Self {
        Self {
            _agent: agent,
            tier,
        }
    }

    /// Check if agent can edit a path.
    pub fn can_edit_path(&self, path: &str) -> bool {
        self.tier.allows_path(path)
    }

    /// Get required approval for operation. The operation parameter is reserved for future tier-specific policies.
    pub fn required_approval(&self, _operation: &str) -> crate::policy::ApprovalRequirement {
        match self.tier {
            AgentPermissionTier::ReadOnly => crate::policy::ApprovalRequirement::HumanReview,
            AgentPermissionTier::DocsTestsReports => {
                crate::policy::ApprovalRequirement::HumanReview
            }
            AgentPermissionTier::TauriServiceWiring => {
                crate::policy::ApprovalRequirement::HumanReview
            }
            AgentPermissionTier::RuntimeProposalOnly => {
                crate::policy::ApprovalRequirement::SecurityReview
            }
            AgentPermissionTier::BridgeEconomicsProposalOnly => {
                crate::policy::ApprovalRequirement::SecurityReview
            }
            AgentPermissionTier::MainnetBlocked => crate::policy::ApprovalRequirement::Blocked,
        }
    }
}
