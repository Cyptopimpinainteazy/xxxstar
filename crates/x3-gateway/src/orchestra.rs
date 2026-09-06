//! Orchestra workflow coordination for the gateway.
//!
//! The gateway exposes the REST endpoints under `/api/v1/orchestra/*`. In
//! **standalone** mode (no `ControlPlaneClient` configured) these operations
//! persist directly into the gateway's own Postgres data source. When a
//! remote orchestra **control-plane** is configured, the gateway acts as a
//! thin relay: it forwards authoritative write/transition operations to the
//! control-plane and mirrors the control-plane's response back into the
//! gateway DB (so the gateway's indexed view stays consistent with the
//! source of truth).
//!
//! Operations that represent a state *transition of record* (intent dispatch,
//! vote-window close, imported tally) have no meaning without an upstream
//! authority — the control-plane owns those transitions. In standalone mode
//! those calls fail fast with a clear error instead of pretending to perform
//! an irreversible cross-service action locally.

use crate::db::{self, ApprovalCase, Database, EvidenceBundle, OrchestraIntent, VoteWindow};
use crate::error::{GatewayError, Result};
use std::sync::Arc;
use x3_orchestra_control_plane::{ControlPlaneClient, VoteTally};
use x3_orchestra_control_plane::types::{IntentKind, NewIntent, NewVoteWindow, RiskClass, VoteChoice};

/// A possibly-present handle to the remote control-plane as stored on state
/// (`Option<Arc<...>>`). We deref on use.
type ClientRef<'a> = Option<&'a Arc<ControlPlaneClient>>;

fn deref(client: ClientRef<'_>) -> Option<&ControlPlaneClient> {
    client.map(|c| c.as_ref())
}

// ────────────────────────────────────────────────────────────────────────
// String → control-plane enum mapping
// ────────────────────────────────────────────────────────────────────────

pub(crate) fn parse_intent_kind(s: &str) -> Result<IntentKind> {
    match s {
        "validation" => Ok(IntentKind::Validation),
        "benchmarking" => Ok(IntentKind::Benchmarking),
        "publication" => Ok(IntentKind::Publication),
        "sanctions" => Ok(IntentKind::Sanctions),
        "treasury_action" => Ok(IntentKind::TreasuryAction),
        "strategy_activation" => Ok(IntentKind::StrategyActivation),
        other => Err(GatewayError::BadRequest(format!(
            "intent kind `{other}` is not a control-plane vocabulary kind"
        ))),
    }
}

fn parse_risk_class(s: &str) -> Result<RiskClass> {
    match s {
        "low" => Ok(RiskClass::Low),
        "medium" => Ok(RiskClass::Medium),
        "high" => Ok(RiskClass::High),
        "critical" => Ok(RiskClass::Critical),
        other => Err(GatewayError::BadRequest(format!(
            "risk class `{other}` is not a control-plane vocabulary class"
        ))),
    }
}

fn parse_vote_choice(s: &str) -> Result<VoteChoice> {
    match s {
        "approve" => Ok(VoteChoice::Approve),
        "reject" => Ok(VoteChoice::Reject),
        "abstain" => Ok(VoteChoice::Abstain),
        other => Err(GatewayError::BadRequest(format!(
            "vote choice `{other}` is not a control-plane vocabulary choice"
        ))),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Public orchestration API (called from crate::rest)
// ────────────────────────────────────────────────────────────────────────

/// Create an intent, either locally (standalone) or relayed + mirrored to a
/// configured control-plane.
pub async fn create_orchestra_intent(
    db: &Database,
    client: ClientRef<'_>,
    request: db::NewOrchestraIntent,
) -> Result<OrchestraIntent> {
    match deref(client) {
        Some(cp) => {
            let new_intent = NewIntent {
                tenant_id: request.tenant_id,
                kind: parse_intent_kind(&request.kind)?,
                risk_class: parse_risk_class(&request.risk_class)?,
                submitter: request.submitter,
                payload: request.payload,
            };
            let remote = cp.create_intent(&new_intent).await.map_err(cp_err)?;
            Ok(db
                .upsert_orchestra_intent_from_control_plane(&remote)
                .await?)
        }
        None => Ok(db.create_orchestra_intent(request).await?),
    }
}

/// Open an approval case against an intent.
pub async fn create_approval_case(
    db: &Database,
    client: ClientRef<'_>,
    request: db::NewApprovalCase,
) -> Result<ApprovalCase> {
    match deref(client) {
        Some(cp) => {
            let remote = cp
                .create_approval_case(
                    &x3_orchestra_control_plane::NewApprovalCase {
                        intent_id: request.intent_id,
                        review_kind: request.review_kind,
                        requested_by: request.requested_by,
                        summary: request.summary,
                        metadata: request.metadata,
                    },
                )
                .await
                .map_err(cp_err)?;
            Ok(db
                .upsert_approval_case_from_control_plane(&remote)
                .await?)
        }
        None => Ok(db.create_approval_case(request).await?),
    }
}

/// Open a vote window.
pub async fn create_vote_window(
    db: &Database,
    client: ClientRef<'_>,
    request: db::NewVoteWindow,
) -> Result<VoteWindow> {
    match deref(client) {
        Some(cp) => {
            let remote = cp
                .open_vote_window(&NewVoteWindow {
                    approval_case_id: request.approval_case_id,
                    title: request.title,
                    opens_at_unix: request.opens_at_unix as u64,
                    closes_at_unix: request.closes_at_unix as u64,
                })
                .await
                .map_err(cp_err)?;
            Ok(db
                .upsert_vote_window_from_control_plane(&remote)
                .await?)
        }
        None => Ok(db.create_vote_window(request).await?),
    }
}

/// Record a single vote into an open window.
pub async fn create_vote_receipt(
    db: &Database,
    client: ClientRef<'_>,
    window_id: &str,
    request: db::NewVoteReceipt,
) -> Result<db::VoteReceipt> {
    match deref(client) {
        Some(cp) => {
            let remote = cp
                .record_vote(
                    window_id,
                    &x3_orchestra_control_plane::NewVoteReceipt {
                        voter_id: request.voter_id,
                        vote_choice: parse_vote_choice(&request.vote_choice)?,
                        rationale: request.rationale,
                        cast_at_unix: request.cast_at_unix as u64,
                    },
                )
                .await
                .map_err(cp_err)?;
            Ok(db
                .upsert_vote_receipt_from_control_plane(&remote)
                .await?)
        }
        None => Ok(db.create_vote_receipt(window_id, request).await?),
    }
}

fn require_control_plane(client: ClientRef<'_>) -> Result<&ControlPlaneClient> {
    deref(client).ok_or_else(|| {
        GatewayError::BadRequest(
            "this operation requires a configured orchestra control-plane (set \
             X3_GATEWAY_CONTROL_PLANE_URL); the gateway does not perform these \
             irreversible transitions in standalone mode"
                .to_string(),
        )
    })
}

/// Dispatch an intent via the control-plane. Returns the post-dispatch intent
/// and the dispatch evidence bundle.
pub async fn dispatch_orchestra_intent(
    db: &Database,
    client: ClientRef<'_>,
    intent_id: &str,
    evidence: x3_orchestra_control_plane::DispatchEvidenceRequest,
) -> Result<(OrchestraIntent, EvidenceBundle)> {
    let cp = require_control_plane(client)?;
    let dispatch = cp
        .dispatch_intent(intent_id, evidence)
        .await
        .map_err(cp_err)?;
    let intent = db
        .upsert_orchestra_intent_from_control_plane(&dispatch.intent)
        .await?;
    let evidence = db
        .upsert_evidence_bundle_from_control_plane(&dispatch.evidence)
        .await?;
    Ok((intent, evidence))
}

/// Close a vote window via the control-plane and mirror the authoritative
/// outcome (window + resulting approval decision + evidence) locally.
pub async fn close_vote_window(
    db: &Database,
    client: ClientRef<'_>,
    window_id: &str,
) -> Result<(VoteWindow, ApprovalCase, EvidenceBundle)> {
    let cp = require_control_plane(client)?;
    let closure = cp.close_vote_window(window_id).await.map_err(cp_err)?;
    Ok((
        db.upsert_vote_window_from_control_plane(&closure.vote_window)
            .await?,
        db.upsert_approval_case_from_control_plane(&closure.approval_case)
            .await?,
        db.upsert_evidence_bundle_from_control_plane(&closure.evidence)
            .await?,
    ))
}

/// Import the authoritative vote tally from the control-plane and reflect it
/// into the gateway's indexed copy.
pub async fn import_vote_window_tally(
    db: &Database,
    client: ClientRef<'_>,
    window_id: &str,
) -> Result<VoteTally> {
    let cp = require_control_plane(client)?;
    let tally = cp.import_vote_tally(window_id).await.map_err(cp_err)?;
    db.update_vote_window_tally(window_id, &tally).await?;
    Ok(tally)
}

/// Fetch an evidence bundle. When a control-plane is configured the gateway
/// first attempts to source the authoritative bundle remotely (mirroring the
/// result); otherwise it returns the locally indexed bundle.
pub async fn get_evidence_bundle(
    db: &Database,
    client: ClientRef<'_>,
    bundle_id: &str,
) -> Result<Option<EvidenceBundle>> {
    if let Some(cp) = deref(client) {
        match cp.get_evidence_bundle(bundle_id).await {
            Ok(remote) => {
                let mirrored = db
                    .upsert_evidence_bundle_from_control_plane(&remote)
                    .await?;
                return Ok(Some(mirrored));
            }
            Err(err) => {
                tracing::info!(
                    bundle_id,
                    error = %err,
                    "control-plane evidence fetch failed; falling back to local index"
                );
            }
        }
    }
    db.get_evidence_bundle(bundle_id).await
}

fn cp_err(err: anyhow::Error) -> GatewayError {
    GatewayError::Internal(format!("control-plane error: {err:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_governance_vocabulary() {
        assert_eq!(
            parse_intent_kind("publication").unwrap(),
            IntentKind::Publication
        );
        assert!(parse_intent_kind("sanctions").is_ok());
    }

    #[test]
    fn rejects_out_of_vocabulary_kinds() {
        assert!(parse_intent_kind("freeform").is_err());
        assert!(parse_risk_class("nope").is_err());
        assert!(parse_vote_choice("maybe").is_err());
    }

    #[test]
    fn standalone_transitions_require_upstream_authority() {
        assert!(require_control_plane(None).is_err());
    }
}
