use crate::crm::CrmAdapter;
use crate::error::{ControlPlaneError, Result};
use crate::storage::PersistentStateStore;
use crate::types::{
    ApprovalCase, ApprovalStatus, DispatchEvidenceRequest, EvidenceBundle, EvidenceSummary, Intent,
    IntentDispatchRequest, IntentStatus, NewApprovalCase, NewIntent, NewRewardAccrual,
    NewVoteReceipt, NewVoteWindow, RewardAccrual, RewardAccrualStatus, VoteChoice, VoteReceipt,
    VoteTally, VoteWindow, VoteWindowStatus,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct OrchestraState {
    intents: HashMap<String, Intent>,
    approval_cases: HashMap<String, ApprovalCase>,
    vote_windows: HashMap<String, VoteWindow>,
    vote_receipts: HashMap<String, Vec<VoteReceipt>>,
    evidence_bundles: HashMap<String, EvidenceBundle>,
    reward_accruals: HashMap<String, RewardAccrual>,
}

pub struct OrchestraControlPlane {
    state: RwLock<OrchestraState>,
    store: Option<Arc<PersistentStateStore>>,
}

impl Default for OrchestraControlPlane {
    fn default() -> Self {
        Self {
            state: RwLock::new(OrchestraState::default()),
            store: None,
        }
    }
}

impl OrchestraControlPlane {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_store(store: Arc<PersistentStateStore>) -> Result<Self> {
        let restored = store.load_state()?.unwrap_or_default();
        Ok(Self {
            state: RwLock::new(restored),
            store: Some(store),
        })
    }

    pub fn open_persistent(path: impl AsRef<Path>) -> Result<Self> {
        let store = Arc::new(PersistentStateStore::open(path)?);
        Self::with_store(store)
    }

    fn persist_snapshot(&self, snapshot: &OrchestraState) -> Result<()> {
        if let Some(store) = &self.store {
            store.save_state(snapshot)?;
        }
        Ok(())
    }

    pub async fn create_intent(&self, input: NewIntent, now_unix: u64) -> Result<Intent> {
        if input.tenant_id.trim().is_empty() {
            return Err(ControlPlaneError::InvalidRequest(
                "tenant_id is required".to_string(),
            ));
        }
        if input.submitter.trim().is_empty() {
            return Err(ControlPlaneError::InvalidRequest(
                "submitter is required".to_string(),
            ));
        }
        if !input.payload.is_object() {
            return Err(ControlPlaneError::InvalidRequest(
                "payload must be a JSON object".to_string(),
            ));
        }

        let intent = Intent {
            intent_id: Uuid::new_v4().to_string(),
            tenant_id: input.tenant_id,
            kind: input.kind.clone(),
            status: if input.kind.requires_approval() {
                IntentStatus::PendingApproval
            } else {
                IntentStatus::Ready
            },
            risk_class: input.risk_class,
            submitter: input.submitter,
            requires_approval: input.kind.requires_approval(),
            payload: input.payload,
            created_at_unix: now_unix,
            updated_at_unix: now_unix,
        };

        let mut state = self.state.write().await;
        state
            .intents
            .insert(intent.intent_id.clone(), intent.clone());
        let snapshot = state.clone();
        drop(state);
        self.persist_snapshot(&snapshot)?;
        Ok(intent)
    }

    pub async fn create_approval_case(
        &self,
        input: NewApprovalCase,
        now_unix: u64,
    ) -> Result<ApprovalCase> {
        if input.summary.trim().is_empty() {
            return Err(ControlPlaneError::InvalidRequest(
                "summary is required".to_string(),
            ));
        }
        if !input.metadata.is_object() {
            return Err(ControlPlaneError::InvalidRequest(
                "metadata must be a JSON object".to_string(),
            ));
        }

        let mut state = self.state.write().await;
        let intent = state
            .intents
            .get_mut(&input.intent_id)
            .ok_or_else(|| ControlPlaneError::NotFound(format!("intent {}", input.intent_id)))?;

        let approval_case = ApprovalCase {
            case_id: Uuid::new_v4().to_string(),
            intent_id: intent.intent_id.clone(),
            status: ApprovalStatus::Open,
            review_kind: input.review_kind,
            requested_by: input.requested_by,
            summary: input.summary,
            metadata: input.metadata,
            created_at_unix: now_unix,
            updated_at_unix: now_unix,
        };

        intent.status = IntentStatus::PendingApproval;
        intent.updated_at_unix = now_unix;
        state
            .approval_cases
            .insert(approval_case.case_id.clone(), approval_case.clone());
        let snapshot = state.clone();
        drop(state);
        self.persist_snapshot(&snapshot)?;
        Ok(approval_case)
    }

    pub async fn open_vote_window(
        &self,
        input: NewVoteWindow,
        crm: &dyn CrmAdapter,
        now_unix: u64,
    ) -> Result<VoteWindow> {
        if input.opens_at_unix >= input.closes_at_unix {
            return Err(ControlPlaneError::InvalidRequest(
                "opens_at_unix must be earlier than closes_at_unix".to_string(),
            ));
        }

        let approval_case = {
            let state = self.state.read().await;
            state
                .approval_cases
                .get(&input.approval_case_id)
                .cloned()
                .ok_or_else(|| {
                    ControlPlaneError::NotFound(format!("approval case {}", input.approval_case_id))
                })?
        };

        let snapshot = crm
            .snapshot_eligible_voters(&approval_case, now_unix)
            .await
            .map_err(|err| ControlPlaneError::Crm(err.to_string()))?;

        let vote_window = VoteWindow {
            window_id: Uuid::new_v4().to_string(),
            approval_case_id: input.approval_case_id,
            title: input.title,
            status: if now_unix >= input.opens_at_unix {
                VoteWindowStatus::Open
            } else {
                VoteWindowStatus::Scheduled
            },
            opens_at_unix: input.opens_at_unix,
            closes_at_unix: input.closes_at_unix,
            electorate: snapshot.voters.clone(),
            tally: VoteTally::default(),
            created_at_unix: now_unix,
            updated_at_unix: now_unix,
        };

        crm.publish_ballot(&vote_window, &snapshot)
            .await
            .map_err(|err| ControlPlaneError::Crm(err.to_string()))?;

        let mut state = self.state.write().await;
        state
            .vote_windows
            .insert(vote_window.window_id.clone(), vote_window.clone());
        let snapshot = state.clone();
        drop(state);
        self.persist_snapshot(&snapshot)?;
        Ok(vote_window)
    }

    pub async fn record_vote(&self, window_id: &str, input: NewVoteReceipt) -> Result<VoteReceipt> {
        let mut state = self.state.write().await;
        {
            let window = state
                .vote_windows
                .get(window_id)
                .ok_or_else(|| ControlPlaneError::NotFound(format!("vote window {}", window_id)))?;

            if window.status != VoteWindowStatus::Open {
                return Err(ControlPlaneError::VoteWindowNotOpen);
            }
            if input.cast_at_unix > window.closes_at_unix {
                return Err(ControlPlaneError::VoteWindowStillOpen);
            }
            if !window
                .electorate
                .iter()
                .any(|voter| voter == &input.voter_id)
            {
                return Err(ControlPlaneError::IneligibleVoter);
            }
        }

        let receipts = state
            .vote_receipts
            .entry(window_id.to_string())
            .or_default();
        if receipts
            .iter()
            .any(|receipt| receipt.voter_id == input.voter_id)
        {
            return Err(ControlPlaneError::DuplicateVote);
        }

        let receipt = VoteReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            window_id: window_id.to_string(),
            voter_id: input.voter_id,
            vote_choice: input.vote_choice,
            rationale: input.rationale,
            cast_at_unix: input.cast_at_unix,
        };
        receipts.push(receipt.clone());
        let window = state
            .vote_windows
            .get_mut(window_id)
            .ok_or_else(|| ControlPlaneError::NotFound(format!("vote window {}", window_id)))?;
        window.updated_at_unix = input.cast_at_unix;
        let snapshot = state.clone();
        drop(state);
        self.persist_snapshot(&snapshot)?;
        Ok(receipt)
    }

    pub async fn close_vote_window(
        &self,
        window_id: &str,
        now_unix: u64,
    ) -> Result<(VoteWindow, ApprovalCase, EvidenceBundle)> {
        let mut state = self.state.write().await;
        let receipts = state
            .vote_receipts
            .get(window_id)
            .cloned()
            .unwrap_or_default();
        let tally = tally_votes(&receipts);
        let (updated_window, approval_case_id) = {
            let window = state
                .vote_windows
                .get_mut(window_id)
                .ok_or_else(|| ControlPlaneError::NotFound(format!("vote window {}", window_id)))?;
            if now_unix < window.closes_at_unix {
                return Err(ControlPlaneError::VoteWindowStillOpen);
            }

            window.status = VoteWindowStatus::Closed;
            window.tally = tally.clone();
            window.updated_at_unix = now_unix;
            (window.clone(), window.approval_case_id.clone())
        };

        let updated_approval_case = {
            let approval_case =
                state
                    .approval_cases
                    .get_mut(&approval_case_id)
                    .ok_or_else(|| {
                        ControlPlaneError::NotFound(format!("approval case {}", approval_case_id))
                    })?;
            approval_case.status = if tally.approvals > tally.rejections {
                ApprovalStatus::Approved
            } else {
                ApprovalStatus::Rejected
            };
            approval_case.updated_at_unix = now_unix;
            approval_case.clone()
        };

        let updated_intent = {
            let intent = state
                .intents
                .get_mut(&updated_approval_case.intent_id)
                .ok_or_else(|| {
                    ControlPlaneError::NotFound(format!(
                        "intent {}",
                        updated_approval_case.intent_id
                    ))
                })?;
            intent.status = match updated_approval_case.status {
                ApprovalStatus::Approved => IntentStatus::Ready,
                ApprovalStatus::Rejected => IntentStatus::Blocked,
                ApprovalStatus::Open => IntentStatus::PendingApproval,
            };
            intent.updated_at_unix = now_unix;
            intent.clone()
        };

        let evidence = create_evidence_bundle(
            Some(updated_intent.intent_id.clone()),
            Some(updated_approval_case.case_id.clone()),
            Some(updated_window.window_id.clone()),
            DispatchEvidenceRequest {
                artifact_uri: format!("orchestra://vote-windows/{window_id}/closure"),
                digest: format!(
                    "vote-window:{window_id}:{}:{}:{}",
                    tally.approvals, tally.rejections, tally.abstentions
                ),
                detail: serde_json::json!({
                    "action": "vote_window_closed",
                    "tally": tally,
                    "approval_status": updated_approval_case.status,
                }),
            },
            now_unix,
        );
        state
            .evidence_bundles
            .insert(evidence.bundle_id.clone(), evidence.clone());
        let snapshot = state.clone();
        drop(state);
        self.persist_snapshot(&snapshot)?;

        Ok((updated_window, updated_approval_case, evidence))
    }

    pub async fn import_vote_window_tally(
        &self,
        window_id: &str,
        crm: &dyn CrmAdapter,
        now_unix: u64,
    ) -> Result<VoteTally> {
        let window =
            {
                let state = self.state.read().await;
                state.vote_windows.get(window_id).cloned().ok_or_else(|| {
                    ControlPlaneError::NotFound(format!("vote window {}", window_id))
                })?
            };
        let imported = crm
            .import_closed_tally(&window, now_unix)
            .await
            .map_err(|err| ControlPlaneError::Crm(err.to_string()))?;
        Ok(imported.tally)
    }

    pub async fn dispatch_intent(
        &self,
        intent_id: &str,
        request: IntentDispatchRequest,
        now_unix: u64,
    ) -> Result<(Intent, EvidenceBundle)> {
        let mut state = self.state.write().await;
        let current_intent = state
            .intents
            .get(intent_id)
            .cloned()
            .ok_or_else(|| ControlPlaneError::NotFound(format!("intent {}", intent_id)))?;

        if current_intent.requires_approval {
            let approved = state.approval_cases.values().any(|case| {
                case.intent_id == current_intent.intent_id
                    && case.status == ApprovalStatus::Approved
            });
            if !approved {
                return Err(ControlPlaneError::ApprovalRequired);
            }
        }

        if current_intent.status != IntentStatus::Ready {
            return Err(ControlPlaneError::IntentNotDispatchable);
        }

        let updated_intent = {
            let intent = state
                .intents
                .get_mut(intent_id)
                .ok_or_else(|| ControlPlaneError::NotFound(format!("intent {}", intent_id)))?;
            intent.status = IntentStatus::Dispatched;
            intent.updated_at_unix = now_unix;
            intent.clone()
        };

        let evidence = create_evidence_bundle(
            Some(updated_intent.intent_id.clone()),
            None,
            None,
            request.evidence,
            now_unix,
        );
        state
            .evidence_bundles
            .insert(evidence.bundle_id.clone(), evidence.clone());
        let snapshot = state.clone();
        drop(state);
        self.persist_snapshot(&snapshot)?;
        Ok((updated_intent, evidence))
    }

    pub async fn accrue_reward(
        &self,
        input: NewRewardAccrual,
        now_unix: u64,
    ) -> Result<RewardAccrual> {
        let mut state = self.state.write().await;
        if !state.intents.contains_key(&input.intent_id) {
            return Err(ControlPlaneError::NotFound(format!(
                "intent {}",
                input.intent_id
            )));
        }
        let accrual = RewardAccrual {
            accrual_id: Uuid::new_v4().to_string(),
            intent_id: input.intent_id,
            beneficiary: input.beneficiary,
            amount_units: input.amount_units,
            status: RewardAccrualStatus::Accrued,
            created_at_unix: now_unix,
        };
        state
            .reward_accruals
            .insert(accrual.accrual_id.clone(), accrual.clone());
        let snapshot = state.clone();
        drop(state);
        self.persist_snapshot(&snapshot)?;
        Ok(accrual)
    }

    pub async fn get_intent(&self, intent_id: &str) -> Result<Intent> {
        self.state
            .read()
            .await
            .intents
            .get(intent_id)
            .cloned()
            .ok_or_else(|| ControlPlaneError::NotFound(format!("intent {}", intent_id)))
    }

    pub async fn get_approval_case(&self, case_id: &str) -> Result<ApprovalCase> {
        self.state
            .read()
            .await
            .approval_cases
            .get(case_id)
            .cloned()
            .ok_or_else(|| ControlPlaneError::NotFound(format!("approval case {}", case_id)))
    }

    pub async fn get_vote_window(&self, window_id: &str) -> Result<VoteWindow> {
        self.state
            .read()
            .await
            .vote_windows
            .get(window_id)
            .cloned()
            .ok_or_else(|| ControlPlaneError::NotFound(format!("vote window {}", window_id)))
    }

    pub async fn get_evidence_bundle(&self, bundle_id: &str) -> Result<EvidenceBundle> {
        self.state
            .read()
            .await
            .evidence_bundles
            .get(bundle_id)
            .cloned()
            .ok_or_else(|| ControlPlaneError::NotFound(format!("evidence bundle {}", bundle_id)))
    }
}

fn tally_votes(receipts: &[VoteReceipt]) -> VoteTally {
    let mut tally = VoteTally::default();
    for receipt in receipts {
        match receipt.vote_choice {
            VoteChoice::Approve => tally.approvals += 1,
            VoteChoice::Reject => tally.rejections += 1,
            VoteChoice::Abstain => tally.abstentions += 1,
        }
    }
    tally
}

fn create_evidence_bundle(
    intent_id: Option<String>,
    approval_case_id: Option<String>,
    vote_window_id: Option<String>,
    request: DispatchEvidenceRequest,
    now_unix: u64,
) -> EvidenceBundle {
    EvidenceBundle {
        bundle_id: Uuid::new_v4().to_string(),
        intent_id,
        approval_case_id,
        vote_window_id,
        artifact_uri: request.artifact_uri,
        digest: request.digest,
        summary: EvidenceSummary {
            action: "externally_visible_action".to_string(),
            detail: request.detail,
        },
        created_at_unix: now_unix,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crm::MemoryCrmAdapter;
    use crate::types::{
        IntentKind, NewIntent, NewVoteReceipt, NewVoteWindow, RiskClass, VoteChoice,
    };
    use tempfile::tempdir;

    #[tokio::test]
    async fn publication_intent_cannot_bypass_approval() {
        let service = OrchestraControlPlane::new();
        let intent = service
            .create_intent(
                NewIntent {
                    tenant_id: "tenant-1".to_string(),
                    kind: IntentKind::Publication,
                    risk_class: RiskClass::High,
                    submitter: "operator-1".to_string(),
                    payload: serde_json::json!({"campaign": "alpha"}),
                },
                100,
            )
            .await
            .expect("create intent");

        let error = service
            .dispatch_intent(
                &intent.intent_id,
                IntentDispatchRequest {
                    evidence: DispatchEvidenceRequest {
                        artifact_uri: "orchestra://dispatch/publication".to_string(),
                        digest: "dispatch-publication".to_string(),
                        detail: serde_json::json!({"attempt": 1}),
                    },
                },
                101,
            )
            .await
            .expect_err("publication dispatch without approval should fail");

        assert!(matches!(error, ControlPlaneError::ApprovalRequired));
    }

    #[tokio::test]
    async fn benchmarking_intent_dispatches_and_emits_evidence() {
        let service = OrchestraControlPlane::new();
        let intent = service
            .create_intent(
                NewIntent {
                    tenant_id: "tenant-1".to_string(),
                    kind: IntentKind::Benchmarking,
                    risk_class: RiskClass::Low,
                    submitter: "sidecar-1".to_string(),
                    payload: serde_json::json!({"benchmark": "provider-onboarding"}),
                },
                100,
            )
            .await
            .expect("create intent");

        let (dispatched, evidence) = service
            .dispatch_intent(
                &intent.intent_id,
                IntentDispatchRequest {
                    evidence: DispatchEvidenceRequest {
                        artifact_uri: "orchestra://dispatch/benchmark".to_string(),
                        digest: "dispatch-benchmark".to_string(),
                        detail: serde_json::json!({"job": "bench-1"}),
                    },
                },
                101,
            )
            .await
            .expect("dispatch benchmarking intent");

        assert_eq!(dispatched.status, IntentStatus::Dispatched);
        assert_eq!(
            evidence.intent_id.as_deref(),
            Some(intent.intent_id.as_str())
        );
    }

    #[tokio::test]
    async fn vote_windows_close_deterministically_and_unlock_dispatch() {
        let service = OrchestraControlPlane::new();
        let crm = MemoryCrmAdapter::new(vec!["alice".to_string(), "bob".to_string()]);

        let intent = service
            .create_intent(
                NewIntent {
                    tenant_id: "tenant-1".to_string(),
                    kind: IntentKind::StrategyActivation,
                    risk_class: RiskClass::Critical,
                    submitter: "operator-1".to_string(),
                    payload: serde_json::json!({"strategy": "mean-reversion"}),
                },
                100,
            )
            .await
            .expect("create intent");
        let approval_case = service
            .create_approval_case(
                NewApprovalCase {
                    intent_id: intent.intent_id.clone(),
                    review_kind: "crm_vote".to_string(),
                    requested_by: "operator-1".to_string(),
                    summary: "Approve strategy activation".to_string(),
                    metadata: serde_json::json!({"program": "alpha"}),
                },
                101,
            )
            .await
            .expect("create approval case");

        let window = service
            .open_vote_window(
                NewVoteWindow {
                    approval_case_id: approval_case.case_id.clone(),
                    title: "Strategy vote".to_string(),
                    opens_at_unix: 102,
                    closes_at_unix: 110,
                },
                &crm,
                102,
            )
            .await
            .expect("open vote window");

        service
            .record_vote(
                &window.window_id,
                NewVoteReceipt {
                    voter_id: "alice".to_string(),
                    vote_choice: VoteChoice::Approve,
                    rationale: Some("ship it".to_string()),
                    cast_at_unix: 105,
                },
            )
            .await
            .expect("record alice vote");
        service
            .record_vote(
                &window.window_id,
                NewVoteReceipt {
                    voter_id: "bob".to_string(),
                    vote_choice: VoteChoice::Reject,
                    rationale: Some("needs more review".to_string()),
                    cast_at_unix: 106,
                },
            )
            .await
            .expect("record bob vote");

        let premature = service.close_vote_window(&window.window_id, 109).await;
        assert!(matches!(
            premature,
            Err(ControlPlaneError::VoteWindowStillOpen)
        ));

        service
            .record_vote(
                &window.window_id,
                NewVoteReceipt {
                    voter_id: "alice".to_string(),
                    vote_choice: VoteChoice::Approve,
                    rationale: None,
                    cast_at_unix: 107,
                },
            )
            .await
            .expect_err("duplicate vote should fail");

        let (_window, approval_case, evidence) = service
            .close_vote_window(&window.window_id, 110)
            .await
            .expect("close vote window");
        assert_eq!(approval_case.status, ApprovalStatus::Rejected);
        assert_eq!(
            evidence.vote_window_id.as_deref(),
            Some(window.window_id.as_str())
        );
    }

    #[tokio::test]
    async fn imported_crm_tally_round_trips() {
        let service = OrchestraControlPlane::new();
        let crm = MemoryCrmAdapter::new(vec!["alice".to_string()]);
        let intent = service
            .create_intent(
                NewIntent {
                    tenant_id: "tenant-1".to_string(),
                    kind: IntentKind::TreasuryAction,
                    risk_class: RiskClass::Critical,
                    submitter: "operator-1".to_string(),
                    payload: serde_json::json!({"amount": 42}),
                },
                1,
            )
            .await
            .expect("create intent");
        let approval_case = service
            .create_approval_case(
                NewApprovalCase {
                    intent_id: intent.intent_id,
                    review_kind: "treasury-board".to_string(),
                    requested_by: "operator-1".to_string(),
                    summary: "Treasury disbursement".to_string(),
                    metadata: serde_json::json!({"wallet": "hot"}),
                },
                2,
            )
            .await
            .expect("create approval case");
        let window = service
            .open_vote_window(
                NewVoteWindow {
                    approval_case_id: approval_case.case_id,
                    title: "Treasury board vote".to_string(),
                    opens_at_unix: 3,
                    closes_at_unix: 4,
                },
                &crm,
                3,
            )
            .await
            .expect("open vote window");

        crm.set_imported_tally(
            window.window_id.clone(),
            VoteTally {
                approvals: 3,
                rejections: 1,
                abstentions: 0,
            },
        )
        .await;

        let tally = service
            .import_vote_window_tally(&window.window_id, &crm, 5)
            .await
            .expect("import tally");
        assert_eq!(tally.approvals, 3);
        assert_eq!(crm.published_windows().await, vec![window.window_id]);
    }

    #[tokio::test]
    async fn persistent_store_restores_intents_approvals_vote_windows_and_evidence() {
        let dir = tempdir().expect("tempdir");
        let crm = MemoryCrmAdapter::new(vec!["crm-voter-1".to_string()]);

        let service = OrchestraControlPlane::open_persistent(dir.path()).expect("open store");
        let intent = service
            .create_intent(
                NewIntent {
                    tenant_id: "tenant-persist".to_string(),
                    kind: IntentKind::Publication,
                    risk_class: RiskClass::High,
                    submitter: "operator-persist".to_string(),
                    payload: serde_json::json!({"campaign": "persisted"}),
                },
                500,
            )
            .await
            .expect("create intent");
        let approval_case = service
            .create_approval_case(
                NewApprovalCase {
                    intent_id: intent.intent_id.clone(),
                    review_kind: "crm_vote".to_string(),
                    requested_by: "operator-persist".to_string(),
                    summary: "persist approval".to_string(),
                    metadata: serde_json::json!({"source": "test"}),
                },
                501,
            )
            .await
            .expect("approval case");
        let vote_window = service
            .open_vote_window(
                NewVoteWindow {
                    approval_case_id: approval_case.case_id.clone(),
                    title: "persist vote".to_string(),
                    opens_at_unix: 502,
                    closes_at_unix: 503,
                },
                &crm,
                502,
            )
            .await
            .expect("vote window");
        service
            .record_vote(
                &vote_window.window_id,
                NewVoteReceipt {
                    voter_id: "crm-voter-1".to_string(),
                    vote_choice: VoteChoice::Approve,
                    rationale: Some("persist okay".to_string()),
                    cast_at_unix: 503,
                },
            )
            .await
            .expect("record vote");
        let (_, closed_approval_case, evidence) = service
            .close_vote_window(&vote_window.window_id, 504)
            .await
            .expect("close window");

        drop(service);
        let restored = OrchestraControlPlane::open_persistent(dir.path()).expect("restore store");
        let restored_intent = restored
            .get_intent(&intent.intent_id)
            .await
            .expect("restored intent");
        let restored_case = restored
            .get_approval_case(&approval_case.case_id)
            .await
            .expect("restored case");
        let restored_window = restored
            .get_vote_window(&vote_window.window_id)
            .await
            .expect("restored window");
        let restored_evidence = restored
            .get_evidence_bundle(&evidence.bundle_id)
            .await
            .expect("restored evidence");

        assert_eq!(restored_intent.status, IntentStatus::Ready);
        assert_eq!(restored_case.status, ApprovalStatus::Approved);
        assert_eq!(restored_case.status, closed_approval_case.status);
        assert_eq!(restored_window.status, VoteWindowStatus::Closed);
        assert_eq!(restored_window.tally.approvals, 1);
        assert_eq!(restored_evidence.digest, evidence.digest);
    }
}
