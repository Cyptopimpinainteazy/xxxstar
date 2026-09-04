use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    Validation,
    Benchmarking,
    Publication,
    Sanctions,
    TreasuryAction,
    StrategyActivation,
}

impl IntentKind {
    pub fn requires_approval(&self) -> bool {
        matches!(
            self,
            Self::Publication | Self::Sanctions | Self::TreasuryAction | Self::StrategyActivation
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    PendingApproval,
    Ready,
    Dispatched,
    Completed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Open,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VoteWindowStatus {
    Scheduled,
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VoteChoice {
    Approve,
    Reject,
    Abstain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RewardAccrualStatus {
    Pending,
    Accrued,
    Settled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Intent {
    pub intent_id: String,
    pub tenant_id: String,
    pub kind: IntentKind,
    pub status: IntentStatus,
    pub risk_class: RiskClass,
    pub submitter: String,
    pub requires_approval: bool,
    pub payload: serde_json::Value,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewIntent {
    pub tenant_id: String,
    pub kind: IntentKind,
    pub risk_class: RiskClass,
    pub submitter: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalCase {
    pub case_id: String,
    pub intent_id: String,
    pub status: ApprovalStatus,
    pub review_kind: String,
    pub requested_by: String,
    pub summary: String,
    pub metadata: serde_json::Value,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewApprovalCase {
    pub intent_id: String,
    pub review_kind: String,
    pub requested_by: String,
    pub summary: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VoteTally {
    pub approvals: u64,
    pub rejections: u64,
    pub abstentions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoteWindow {
    pub window_id: String,
    pub approval_case_id: String,
    pub title: String,
    pub status: VoteWindowStatus,
    pub opens_at_unix: u64,
    pub closes_at_unix: u64,
    pub electorate: Vec<String>,
    pub tally: VoteTally,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewVoteWindow {
    pub approval_case_id: String,
    pub title: String,
    pub opens_at_unix: u64,
    pub closes_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoteReceipt {
    pub receipt_id: String,
    pub window_id: String,
    pub voter_id: String,
    pub vote_choice: VoteChoice,
    pub rationale: Option<String>,
    pub cast_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewVoteReceipt {
    pub voter_id: String,
    pub vote_choice: VoteChoice,
    pub rationale: Option<String>,
    pub cast_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSummary {
    pub action: String,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceBundle {
    pub bundle_id: String,
    pub intent_id: Option<String>,
    pub approval_case_id: Option<String>,
    pub vote_window_id: Option<String>,
    pub artifact_uri: String,
    pub digest: String,
    pub summary: EvidenceSummary,
    pub created_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchEvidenceRequest {
    pub artifact_uri: String,
    pub digest: String,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentDispatchRequest {
    pub evidence: DispatchEvidenceRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewardAccrual {
    pub accrual_id: String,
    pub intent_id: String,
    pub beneficiary: String,
    pub amount_units: u64,
    pub status: RewardAccrualStatus,
    pub created_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewRewardAccrual {
    pub intent_id: String,
    pub beneficiary: String,
    pub amount_units: u64,
}
