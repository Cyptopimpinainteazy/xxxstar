pub mod client;
pub mod crm;
pub mod error;
pub mod service;
pub mod storage;
pub mod transport;
pub mod types;

pub use client::{ControlPlaneClient, IntentDispatchResponse, VoteWindowClosureResponse};
pub use crm::{CrmAdapter, ElectorateSnapshot, ImportedVoteTally, MemoryCrmAdapter};
pub use error::{ControlPlaneError, Result};
pub use service::OrchestraControlPlane;
pub use storage::PersistentStateStore;
pub use transport::create_router;
pub use types::{
    ApprovalCase, ApprovalStatus, DispatchEvidenceRequest, EvidenceBundle, EvidenceSummary, Intent,
    IntentDispatchRequest, IntentKind, IntentStatus, NewApprovalCase, NewIntent, NewRewardAccrual,
    NewVoteReceipt, NewVoteWindow, RewardAccrual, RewardAccrualStatus, RiskClass, VoteChoice,
    VoteReceipt, VoteTally, VoteWindow, VoteWindowStatus,
};
