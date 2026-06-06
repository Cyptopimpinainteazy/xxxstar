use crate::crm::CrmAdapter;
use crate::error::ControlPlaneError;
use crate::service::OrchestraControlPlane;
use crate::types::{
    ApprovalCase, EvidenceBundle, Intent, IntentDispatchRequest, NewApprovalCase, NewIntent,
    NewRewardAccrual, NewVoteReceipt, NewVoteWindow, RewardAccrual, VoteReceipt, VoteTally,
    VoteWindow,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

#[derive(Clone)]
struct HttpState {
    service: Arc<OrchestraControlPlane>,
    crm: Arc<dyn CrmAdapter>,
}

#[derive(serde::Serialize)]
struct VoteWindowClosureResponse {
    vote_window: VoteWindow,
    approval_case: ApprovalCase,
    evidence: EvidenceBundle,
}

#[derive(serde::Serialize)]
struct IntentDispatchResponse {
    intent: Intent,
    evidence: EvidenceBundle,
}

pub fn create_router(service: Arc<OrchestraControlPlane>, crm: Arc<dyn CrmAdapter>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/intents", post(create_intent))
        .route("/intents/:intent_id", get(get_intent))
        .route("/intents/:intent_id/dispatch", post(dispatch_intent))
        .route("/approval-cases", post(create_approval_case))
        .route("/approval-cases/:case_id", get(get_approval_case))
        .route("/vote-windows", post(open_vote_window))
        .route("/vote-windows/:window_id", get(get_vote_window))
        .route("/vote-windows/:window_id/votes", post(record_vote))
        .route("/vote-windows/:window_id/close", post(close_vote_window))
        .route(
            "/vote-windows/:window_id/imported-tally",
            post(import_vote_window_tally),
        )
        .route("/rewards", post(accrue_reward))
        .route("/evidence/:bundle_id", get(get_evidence_bundle))
        .with_state(HttpState { service, crm })
}

async fn health() -> &'static str {
    "OK"
}

async fn create_intent(
    State(state): State<HttpState>,
    Json(input): Json<NewIntent>,
) -> HttpResult<Intent> {
    Ok(Json(
        state
            .service
            .create_intent(input, unix_now())
            .await
            .map_err(map_error)?,
    ))
}

async fn get_intent(
    State(state): State<HttpState>,
    Path(intent_id): Path<String>,
) -> HttpResult<Intent> {
    Ok(Json(
        state
            .service
            .get_intent(&intent_id)
            .await
            .map_err(map_error)?,
    ))
}

async fn create_approval_case(
    State(state): State<HttpState>,
    Json(input): Json<NewApprovalCase>,
) -> HttpResult<ApprovalCase> {
    Ok(Json(
        state
            .service
            .create_approval_case(input, unix_now())
            .await
            .map_err(map_error)?,
    ))
}

async fn get_approval_case(
    State(state): State<HttpState>,
    Path(case_id): Path<String>,
) -> HttpResult<ApprovalCase> {
    Ok(Json(
        state
            .service
            .get_approval_case(&case_id)
            .await
            .map_err(map_error)?,
    ))
}

async fn open_vote_window(
    State(state): State<HttpState>,
    Json(input): Json<NewVoteWindow>,
) -> HttpResult<VoteWindow> {
    Ok(Json(
        state
            .service
            .open_vote_window(input, state.crm.as_ref(), unix_now())
            .await
            .map_err(map_error)?,
    ))
}

async fn get_vote_window(
    State(state): State<HttpState>,
    Path(window_id): Path<String>,
) -> HttpResult<VoteWindow> {
    Ok(Json(
        state
            .service
            .get_vote_window(&window_id)
            .await
            .map_err(map_error)?,
    ))
}

async fn record_vote(
    State(state): State<HttpState>,
    Path(window_id): Path<String>,
    Json(input): Json<NewVoteReceipt>,
) -> HttpResult<VoteReceipt> {
    Ok(Json(
        state
            .service
            .record_vote(&window_id, input)
            .await
            .map_err(map_error)?,
    ))
}

async fn close_vote_window(
    State(state): State<HttpState>,
    Path(window_id): Path<String>,
) -> HttpResult<VoteWindowClosureResponse> {
    let (vote_window, approval_case, evidence) = state
        .service
        .close_vote_window(&window_id, unix_now())
        .await
        .map_err(map_error)?;
    Ok(Json(VoteWindowClosureResponse {
        vote_window,
        approval_case,
        evidence,
    }))
}

async fn import_vote_window_tally(
    State(state): State<HttpState>,
    Path(window_id): Path<String>,
) -> HttpResult<VoteTally> {
    Ok(Json(
        state
            .service
            .import_vote_window_tally(&window_id, state.crm.as_ref(), unix_now())
            .await
            .map_err(map_error)?,
    ))
}

async fn dispatch_intent(
    State(state): State<HttpState>,
    Path(intent_id): Path<String>,
    Json(input): Json<IntentDispatchRequest>,
) -> HttpResult<IntentDispatchResponse> {
    let (intent, evidence) = state
        .service
        .dispatch_intent(&intent_id, input, unix_now())
        .await
        .map_err(map_error)?;
    Ok(Json(IntentDispatchResponse { intent, evidence }))
}

async fn accrue_reward(
    State(state): State<HttpState>,
    Json(input): Json<NewRewardAccrual>,
) -> HttpResult<RewardAccrual> {
    Ok(Json(
        state
            .service
            .accrue_reward(input, unix_now())
            .await
            .map_err(map_error)?,
    ))
}

async fn get_evidence_bundle(
    State(state): State<HttpState>,
    Path(bundle_id): Path<String>,
) -> HttpResult<EvidenceBundle> {
    Ok(Json(
        state
            .service
            .get_evidence_bundle(&bundle_id)
            .await
            .map_err(map_error)?,
    ))
}

type HttpResult<T> = std::result::Result<Json<T>, (StatusCode, String)>;

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn map_error(error: ControlPlaneError) -> (StatusCode, String) {
    let status = match error {
        ControlPlaneError::NotFound(_) => StatusCode::NOT_FOUND,
        ControlPlaneError::InvalidRequest(_)
        | ControlPlaneError::ApprovalRequired
        | ControlPlaneError::IntentNotDispatchable
        | ControlPlaneError::VoteWindowNotOpen
        | ControlPlaneError::VoteWindowStillOpen
        | ControlPlaneError::IneligibleVoter
        | ControlPlaneError::DuplicateVote => StatusCode::BAD_REQUEST,
        ControlPlaneError::Crm(_) => StatusCode::BAD_GATEWAY,
        ControlPlaneError::Persistence(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, error.to_string())
}
