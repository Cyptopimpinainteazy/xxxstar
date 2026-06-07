use serde::{Deserialize, Serialize};

/// Approval levels required for task execution or file changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalRequirement {
    None,
    HumanReview,
    SecurityReview,
    GovernanceReview,
    Blocked,
}

impl ApprovalRequirement {
    /// Check if approval is satisfied (stub for now).
    pub fn is_satisfied(&self) -> bool {
        // TODO(X3-SWARM-APPROVAL): implement full approval verification before production use.
        // Current behavior only treats ApprovalRequirement::None as satisfied.
        matches!(self, ApprovalRequirement::None)
    }
}

/// Agent policy structure for swarm control.
///
/// `forbidden_paths` take precedence over `auto_edit_allowed`; policies with overlapping entries should be rejected by `validate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub kind: crate::agent::AgentKind,
    pub permission_tier: crate::agent::AgentPermissionTier,
    pub auto_edit_allowed: Vec<String>,
    pub approval_required: Vec<String>,
    pub forbidden_paths: Vec<String>,
}

impl AgentPolicy {
    pub fn allows_path(&self, path: &str) -> bool {
        if self
            .forbidden_paths
            .iter()
            .any(|prefix| path == prefix || path.starts_with(prefix))
        {
            return false;
        }
        self.auto_edit_allowed
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }

    pub fn validate(&self) -> Result<(), String> {
        for allowed in &self.auto_edit_allowed {
            if self.forbidden_paths.iter().any(|forbidden| {
                allowed == forbidden
                    || allowed.starts_with(forbidden)
                    || forbidden.starts_with(allowed)
            }) {
                return Err(format!(
                    "policy for {:?} has overlapping allowed and forbidden path: {}",
                    self.kind, allowed
                ));
            }
        }
        Ok(())
    }
}

const COMMON_FORBIDDEN_PATHS: &[&str] = &[".env", "private_keys", "validator_keys"];

pub fn default_agent_policies() -> Vec<AgentPolicy> {
    let common_forbidden: Vec<String> = COMMON_FORBIDDEN_PATHS
        .iter()
        .map(|path| (*path).into())
        .collect();
    vec![
        AgentPolicy {
            kind: crate::agent::AgentKind::RepoScanner,
            permission_tier: crate::agent::AgentPermissionTier::ReadOnly,
            auto_edit_allowed: vec![],
            approval_required: vec![],
            forbidden_paths: common_forbidden.clone(),
        },
        AgentPolicy {
            kind: crate::agent::AgentKind::TestBuilder,
            permission_tier: crate::agent::AgentPermissionTier::DocsTestsReports,
            auto_edit_allowed: vec!["tests/".into(), "docs/".into(), "reports/".into()],
            approval_required: vec!["runtime/".into(), "pallets/".into()],
            forbidden_paths: common_forbidden,
        },
    ]
}
