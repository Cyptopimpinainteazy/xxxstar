use crate::types::{ApprovalCase, VoteTally, VoteWindow};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectorateSnapshot {
    pub snapshot_id: String,
    pub voters: Vec<String>,
    pub captured_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedVoteTally {
    pub tally: VoteTally,
    pub imported_at_unix: u64,
}

#[async_trait]
pub trait CrmAdapter: Send + Sync {
    async fn snapshot_eligible_voters(
        &self,
        approval_case: &ApprovalCase,
        captured_at_unix: u64,
    ) -> anyhow::Result<ElectorateSnapshot>;

    async fn publish_ballot(
        &self,
        vote_window: &VoteWindow,
        snapshot: &ElectorateSnapshot,
    ) -> anyhow::Result<()>;

    async fn import_closed_tally(
        &self,
        vote_window: &VoteWindow,
        imported_at_unix: u64,
    ) -> anyhow::Result<ImportedVoteTally>;
}

#[derive(Clone, Default)]
pub struct MemoryCrmAdapter {
    voters: Arc<RwLock<Vec<String>>>,
    imported_tallies: Arc<RwLock<HashMap<String, VoteTally>>>,
    published_windows: Arc<RwLock<Vec<String>>>,
}

impl MemoryCrmAdapter {
    pub fn new(voters: Vec<String>) -> Self {
        Self {
            voters: Arc::new(RwLock::new(voters)),
            imported_tallies: Arc::new(RwLock::new(HashMap::new())),
            published_windows: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn set_imported_tally(&self, window_id: impl Into<String>, tally: VoteTally) {
        self.imported_tallies
            .write()
            .await
            .insert(window_id.into(), tally);
    }

    pub async fn published_windows(&self) -> Vec<String> {
        self.published_windows.read().await.clone()
    }
}

#[async_trait]
impl CrmAdapter for MemoryCrmAdapter {
    async fn snapshot_eligible_voters(
        &self,
        approval_case: &ApprovalCase,
        captured_at_unix: u64,
    ) -> anyhow::Result<ElectorateSnapshot> {
        Ok(ElectorateSnapshot {
            snapshot_id: format!("snapshot-{}", approval_case.case_id),
            voters: self.voters.read().await.clone(),
            captured_at_unix,
        })
    }

    async fn publish_ballot(
        &self,
        vote_window: &VoteWindow,
        _snapshot: &ElectorateSnapshot,
    ) -> anyhow::Result<()> {
        self.published_windows
            .write()
            .await
            .push(vote_window.window_id.clone());
        Ok(())
    }

    async fn import_closed_tally(
        &self,
        vote_window: &VoteWindow,
        imported_at_unix: u64,
    ) -> anyhow::Result<ImportedVoteTally> {
        let tally = self
            .imported_tallies
            .read()
            .await
            .get(&vote_window.window_id)
            .cloned()
            .unwrap_or_default();
        Ok(ImportedVoteTally {
            tally,
            imported_at_unix,
        })
    }
}
