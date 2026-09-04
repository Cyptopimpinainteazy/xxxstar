use crate::policy::ApprovalRequirement;
use tracing::info;

/// Approval gate for high-risk operations.
#[derive(Debug, Clone)]
pub struct ApprovalGate {
    requirement: ApprovalRequirement,
}

impl ApprovalGate {
    pub fn new(req: ApprovalRequirement) -> Self {
        Self { requirement: req }
    }

    pub fn request_approval(&self, context: &str) -> bool {
        match self.requirement {
            ApprovalRequirement::None => true,
            ApprovalRequirement::HumanReview => {
                info!("Human review required for: {}", context);
                false // Stub: wait for input
            }
            ApprovalRequirement::SecurityReview => false,
            ApprovalRequirement::GovernanceReview => false,
            ApprovalRequirement::Blocked => false,
        }
    }

    pub fn grant(&mut self) {
        self.requirement = ApprovalRequirement::None;
    }
}
