use crate::db::{
    ApprovalCase, Database, EvidenceBundle, NewApprovalCase, NewOrchestraIntent, NewVoteReceipt,
    NewVoteWindow, OrchestraIntent, VoteReceipt, VoteWindow,
};
use crate::error::{GatewayError, Result};
use std::sync::Arc;
use x3_orchestra_control_plane::{
    ControlPlaneClient, DispatchEvidenceRequest, IntentKind,
    NewApprovalCase as ControlPlaneNewApprovalCase, NewIntent as ControlPlaneNewIntent,
    NewVoteReceipt as ControlPlaneNewVoteReceipt, NewVoteWindow as ControlPlaneNewVoteWindow,
    RiskClass, VoteChoice, VoteTally,
};

pub async fn create_orchestra_intent(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    request: NewOrchestraIntent,
) -> Result<OrchestraIntent> {
    match orchestra_client {
        Some(client) => {
            let remote = client
                .create_intent(&to_control_plane_new_intent(&request)?)
                .await
                .map_err(map_upstream_error)?;
            db.upsert_orchestra_intent_from_control_plane(&remote).await
        }
        None => db.create_orchestra_intent(request).await,
    }
}

pub async fn create_approval_case(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    request: NewApprovalCase,
) -> Result<ApprovalCase> {
    match orchestra_client {
        Some(client) => {
            let remote = client
                .create_approval_case(&ControlPlaneNewApprovalCase {
                    intent_id: request.intent_id,
                    review_kind: request.review_kind,
                    requested_by: request.requested_by,
                    summary: request.summary,
                    metadata: request.metadata,
                })
                .await
                .map_err(map_upstream_error)?;
            db.upsert_approval_case_from_control_plane(&remote).await
        }
        None => db.create_approval_case(request).await,
    }
}

pub async fn create_vote_window(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    request: NewVoteWindow,
) -> Result<VoteWindow> {
    match orchestra_client {
        Some(client) => {
            let remote = client
                .open_vote_window(&ControlPlaneNewVoteWindow {
                    approval_case_id: request.approval_case_id,
                    title: request.title,
                    opens_at_unix: request.opens_at_unix as u64,
                    closes_at_unix: request.closes_at_unix as u64,
                })
                .await
                .map_err(map_upstream_error)?;
            db.upsert_vote_window_from_control_plane(&remote).await
        }
        None => db.create_vote_window(request).await,
    }
}

pub async fn create_vote_receipt(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    window_id: &str,
    request: NewVoteReceipt,
) -> Result<VoteReceipt> {
    match orchestra_client {
        Some(client) => {
            let remote = client
                .record_vote(
                    window_id,
                    &ControlPlaneNewVoteReceipt {
                        voter_id: request.voter_id,
                        vote_choice: parse_vote_choice(&request.vote_choice)?,
                        rationale: request.rationale,
                        cast_at_unix: request.cast_at_unix as u64,
                    },
                )
                .await
                .map_err(map_upstream_error)?;
            db.upsert_vote_receipt_from_control_plane(&remote).await
        }
        None => db.create_vote_receipt(window_id, request).await,
    }
}

pub async fn dispatch_orchestra_intent(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    intent_id: &str,
    evidence: DispatchEvidenceRequest,
) -> Result<(OrchestraIntent, EvidenceBundle)> {
    match orchestra_client {
        Some(client) => {
            let remote = client
                .dispatch_intent(intent_id, evidence)
                .await
                .map_err(map_upstream_error)?;
            let intent = db
                .upsert_orchestra_intent_from_control_plane(&remote.intent)
                .await?;
            let evidence = db
                .upsert_evidence_bundle_from_control_plane(&remote.evidence)
                .await?;
            Ok((intent, evidence))
        }
        None => Err(GatewayError::BadRequest(
            "orchestra control-plane relay is not configured".to_string(),
        )),
    }
}

pub async fn close_vote_window(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    window_id: &str,
) -> Result<(VoteWindow, ApprovalCase, EvidenceBundle)> {
    match orchestra_client {
        Some(client) => {
            let remote = client
                .close_vote_window(window_id)
                .await
                .map_err(map_upstream_error)?;
            let vote_window = db
                .upsert_vote_window_from_control_plane(&remote.vote_window)
                .await?;
            let approval_case = db
                .upsert_approval_case_from_control_plane(&remote.approval_case)
                .await?;
            let evidence = db
                .upsert_evidence_bundle_from_control_plane(&remote.evidence)
                .await?;
            Ok((vote_window, approval_case, evidence))
        }
        None => Err(GatewayError::BadRequest(
            "orchestra control-plane relay is not configured".to_string(),
        )),
    }
}

pub async fn import_vote_window_tally(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    window_id: &str,
) -> Result<VoteTally> {
    match orchestra_client {
        Some(client) => {
            let tally = client
                .import_vote_tally(window_id)
                .await
                .map_err(map_upstream_error)?;
            let _ = db.update_vote_window_tally(window_id, &tally).await?;
            Ok(tally)
        }
        None => Err(GatewayError::BadRequest(
            "orchestra control-plane relay is not configured".to_string(),
        )),
    }
}

pub async fn get_evidence_bundle(
    db: &Database,
    orchestra_client: Option<&Arc<ControlPlaneClient>>,
    bundle_id: &str,
) -> Result<Option<EvidenceBundle>> {
    if let Some(evidence_bundle) = db.get_evidence_bundle(bundle_id).await? {
        return Ok(Some(evidence_bundle));
    }

    match orchestra_client {
        Some(client) => match client.get_evidence_bundle(bundle_id).await {
            Ok(remote) => Ok(Some(
                db.upsert_evidence_bundle_from_control_plane(&remote)
                    .await?,
            )),
            Err(error) => {
                let message = error.to_string();
                if message.contains("status 404") {
                    Ok(None)
                } else {
                    Err(map_upstream_error(error))
                }
            }
        },
        None => Ok(None),
    }
}

fn to_control_plane_new_intent(request: &NewOrchestraIntent) -> Result<ControlPlaneNewIntent> {
    Ok(ControlPlaneNewIntent {
        tenant_id: request.tenant_id.clone(),
        kind: parse_intent_kind(&request.kind)?,
        risk_class: parse_risk_class(&request.risk_class)?,
        submitter: request.submitter.clone(),
        payload: request.payload.clone(),
    })
}

fn parse_intent_kind(kind: &str) -> Result<IntentKind> {
    let normalized = kind.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "validation" | "replay" | "defensive_analysis" => Ok(IntentKind::Validation),
        "benchmarking" | "benchmark" | "provider_onboarding" | "onboarding_benchmark" => {
            Ok(IntentKind::Benchmarking)
        }
        "publication" | "content_publication" | "media_publication" => Ok(IntentKind::Publication),
        "sanctions" | "sanction" => Ok(IntentKind::Sanctions),
        "treasury_action" | "treasury" => Ok(IntentKind::TreasuryAction),
        "strategy_activation" | "strategy" => Ok(IntentKind::StrategyActivation),
        _ => Err(GatewayError::BadRequest(format!(
            "unsupported orchestra intent kind for control-plane relay: {kind}"
        ))),
    }
}

fn parse_risk_class(risk_class: &str) -> Result<RiskClass> {
    match risk_class.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(RiskClass::Low),
        "medium" => Ok(RiskClass::Medium),
        "high" => Ok(RiskClass::High),
        "critical" => Ok(RiskClass::Critical),
        _ => Err(GatewayError::BadRequest(format!(
            "unsupported orchestra risk_class for control-plane relay: {risk_class}"
        ))),
    }
}

fn parse_vote_choice(vote_choice: &str) -> Result<VoteChoice> {
    match vote_choice.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" | "yes" => Ok(VoteChoice::Approve),
        "reject" | "rejected" | "no" => Ok(VoteChoice::Reject),
        "abstain" => Ok(VoteChoice::Abstain),
        _ => Err(GatewayError::BadRequest(format!(
            "unsupported vote_choice for control-plane relay: {vote_choice}"
        ))),
    }
}

fn map_upstream_error(error: anyhow::Error) -> GatewayError {
    GatewayError::Upstream(error.to_string())
}
