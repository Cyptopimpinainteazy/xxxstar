//! REST API routes and handlers.

use crate::cache::RedisCache;
use crate::db::{
    ChainStats, Database, NewApprovalCase, NewEvidenceBundle, NewOrchestraIntent, NewVoteReceipt,
    NewVoteWindow, StoredBenchmarkReport,
};
use crate::error::GatewayError;
use crate::graphql::AppSchema;
use crate::orchestra;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{Path, Query, State},
    http::{header, Method, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use x3_orchestra_control_plane::{ControlPlaneClient, DispatchEvidenceRequest, VoteTally};
use x3_rpc::benchmark::{BenchmarkProfile, BenchmarkReport};

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub schema: AppSchema,
    pub benchmark_publish_token: Option<String>,
    pub orchestra_client: Option<Arc<ControlPlaneClient>>,
    pub redis_cache: Option<RedisCache>,
    pub cache_metrics: Arc<CacheMetrics>,
}

#[derive(Debug, Default)]
pub struct CacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    fallbacks: AtomicU64,
}

impl CacheMetrics {
    fn snapshot(&self) -> CacheMetricsSnapshot {
        CacheMetricsSnapshot {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            fallbacks: self.fallbacks.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Serialize)]
struct CacheMetricsSnapshot {
    hits: u64,
    misses: u64,
    fallbacks: u64,
}

/// Pagination parameters.
#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

fn bounded_limit(limit: i64) -> i64 {
    limit.clamp(1, 100)
}

/// Create the API router.
pub fn create_router(
    db: Database,
    schema: AppSchema,
    orchestra_client: Option<Arc<ControlPlaneClient>>,
    redis_cache: Option<RedisCache>,
) -> Router {
    let state = AppState {
        db,
        schema,
        benchmark_publish_token: std::env::var("X3_GATEWAY_BENCHMARK_TOKEN").ok(),
        orchestra_client,
        redis_cache,
        cache_metrics: Arc::new(CacheMetrics::default()),
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);

    Router::new()
        // Health and status
        .route("/health", get(health))
        .route("/status", get(status))
        // GraphQL
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        // REST API v1
        .nest("/api/v1", api_routes())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// API routes.
fn api_routes() -> Router<AppState> {
    Router::new()
        // Stats
        .route("/stats", get(get_stats))
        .route(
            "/benchmarks/reports",
            post(publish_benchmark_report).get(get_benchmark_reports),
        )
        .route("/benchmarks/reports/:report_id", get(get_benchmark_report))
        .route("/benchmarks/results", post(submit_benchmark_result))
        .route(
            "/orchestra/intents",
            post(create_orchestra_intent).get(list_orchestra_intents),
        )
        .route("/orchestra/intents/:intent_id", get(get_orchestra_intent))
        .route(
            "/orchestra/intents/:intent_id/dispatch",
            post(dispatch_orchestra_intent),
        )
        .route(
            "/orchestra/approval-cases",
            post(create_approval_case).get(list_approval_cases),
        )
        .route("/orchestra/approval-cases/:case_id", get(get_approval_case))
        .route(
            "/orchestra/vote-windows",
            post(create_vote_window).get(list_vote_windows),
        )
        .route("/orchestra/vote-windows/:window_id", get(get_vote_window))
        .route(
            "/orchestra/vote-windows/:window_id/receipts",
            post(create_vote_receipt),
        )
        .route(
            "/orchestra/vote-windows/:window_id/close",
            post(close_vote_window),
        )
        .route(
            "/orchestra/vote-windows/:window_id/imported-tally",
            post(import_vote_window_tally),
        )
        .route(
            "/orchestra/evidence-bundles",
            post(create_evidence_bundle).get(list_evidence_bundles),
        )
        .route(
            "/orchestra/evidence-bundles/:bundle_id",
            get(get_evidence_bundle),
        )
        // Blocks
        .route("/blocks", get(get_blocks))
        .route("/blocks/latest", get(get_latest_block))
        .route("/blocks/:number", get(get_block))
        .route("/blocks/:number/extrinsics", get(get_block_extrinsics))
        .route("/blocks/:number/events", get(get_block_events))
        // Extrinsics
        .route("/extrinsics", get(get_extrinsics))
        .route("/extrinsics/:hash", get(get_extrinsic))
        // Events
        .route("/events", get(get_events))
        // Comits
        .route("/comits", get(get_comits))
        .route("/comits/:hash", get(get_comit))
        // Accounts
        .route("/accounts/:address", get(get_account))
        .route("/accounts/:address/extrinsics", get(get_account_extrinsics))
        .route("/accounts/:address/comits", get(get_account_comits))
}

// ============================================================================
// Health endpoints
// ============================================================================

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    latest_block: Option<i64>,
    total_blocks: i64,
    total_comits: i64,
    cache: Option<CacheMetricsSnapshot>,
}

#[derive(Debug, Deserialize)]
struct DispatchIntentRequest {
    artifact_uri: String,
    digest: String,
    detail: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct IntentDispatchResponse {
    intent: crate::db::OrchestraIntent,
    evidence: crate::db::EvidenceBundle,
}

#[derive(Debug, Serialize)]
struct VoteWindowClosureResponse {
    vote_window: crate::db::VoteWindow,
    approval_case: crate::db::ApprovalCase,
    evidence: crate::db::EvidenceBundle,
}

async fn status(State(state): State<AppState>) -> Result<Json<StatusResponse>, GatewayError> {
    let stats = load_chain_stats(&state).await?;

    Ok(Json(StatusResponse {
        status: "ok".to_string(),
        latest_block: stats.latest_block,
        total_blocks: stats.total_blocks,
        total_comits: stats.total_comits,
        cache: state
            .redis_cache
            .as_ref()
            .map(|_| state.cache_metrics.snapshot()),
    }))
}

// ============================================================================
// GraphQL endpoints
// ============================================================================

async fn graphql_handler(State(state): State<AppState>, req: GraphQLRequest) -> GraphQLResponse {
    state.schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

// ============================================================================
// Block endpoints
// ============================================================================

async fn get_blocks(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let blocks = state
        .db
        .get_recent_blocks(pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(blocks))
}

async fn get_latest_block(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, GatewayError> {
    let block = load_latest_block(&state).await?;
    match block {
        Some(b) => Ok(Json(b)),
        None => Err(GatewayError::NotFound("No blocks indexed yet".to_string())),
    }
}

async fn get_block(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<impl IntoResponse, GatewayError> {
    let block = state.db.get_block(number).await?;
    match block {
        Some(b) => Ok(Json(b)),
        None => Err(GatewayError::NotFound(format!(
            "Block {} not found",
            number
        ))),
    }
}

async fn get_block_extrinsics(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsics = state.db.get_block_extrinsics(number).await?;
    Ok(Json(extrinsics))
}

async fn get_block_events(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<impl IntoResponse, GatewayError> {
    let events = state.db.get_block_events(number).await?;
    Ok(Json(events))
}

// ============================================================================
// Extrinsic endpoints
// ============================================================================

async fn get_extrinsics(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsics = state
        .db
        .get_recent_extrinsics(pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(extrinsics))
}

async fn get_extrinsic(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsic = state.db.get_extrinsic(&hash).await?;
    match extrinsic {
        Some(e) => Ok(Json(e)),
        None => Err(GatewayError::NotFound(format!(
            "Extrinsic {} not found",
            hash
        ))),
    }
}

// ============================================================================
// Event endpoints
// ============================================================================

#[derive(Deserialize)]
struct EventQuery {
    pallet: Option<String>,
    variant: Option<String>,
    #[serde(flatten)]
    pagination: Pagination,
}

#[derive(Debug, Deserialize)]
struct BenchmarkReportQuery {
    tenant_id: Option<String>,
    min_high_conflict_ratio: Option<f64>,
    min_serial_fraction: Option<f64>,
    log_class: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    #[serde(flatten)]
    pagination: Pagination,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BenchmarkReportSortField {
    GeneratedAt,
    HighConflictRatio,
    SerialFraction,
    TotalTransactions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortOrder {
    Asc,
    Desc,
}

impl BenchmarkReportSortField {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "generated_at" => Some(BenchmarkReportSortField::GeneratedAt),
            "high_conflict_ratio" => Some(BenchmarkReportSortField::HighConflictRatio),
            "serial_fraction" => Some(BenchmarkReportSortField::SerialFraction),
            "total_transactions" => Some(BenchmarkReportSortField::TotalTransactions),
            _ => None,
        }
    }
}

impl SortOrder {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "desc" => SortOrder::Desc,
            _ => SortOrder::Asc,
        }
    }
}

#[derive(Debug, Deserialize)]
struct PublishBenchmarkReportRequest {
    tenant_id: String,
    report: BenchmarkReport,
}

#[derive(Debug, Deserialize)]
struct SubmitBenchmarkResultRequest {
    tenant_id: String,
    report: BenchmarkReport,
}

#[derive(Debug, Serialize)]
struct BenchmarkResultResponse {
    report_id: String,
    status: String,
}

async fn get_events(
    State(state): State<AppState>,
    Query(query): Query<EventQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = query.pagination.limit.min(100);
    let offset = query.pagination.offset;

    let events = match (query.pallet, query.variant) {
        (Some(pallet), Some(variant)) => {
            state
                .db
                .get_events_by_type(&pallet, &variant, limit, offset)
                .await?
        }
        (Some(pallet), None) => {
            state
                .db
                .get_events_by_pallet(&pallet, limit, offset)
                .await?
        }
        _ => {
            // No filter - return error, need at least pallet filter
            return Err(GatewayError::BadRequest(
                "pallet parameter required".to_string(),
            ));
        }
    };

    Ok(Json(events))
}

// ============================================================================
// Comit endpoints
// ============================================================================

async fn get_comits(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let comits = state
        .db
        .get_recent_comits(pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(comits))
}

async fn get_comit(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let comit = state.db.get_comit(&hash).await?;
    match comit {
        Some(c) => Ok(Json(c)),
        None => Err(GatewayError::NotFound(format!("Comit {} not found", hash))),
    }
}

// ============================================================================
// Account endpoints
// ============================================================================

async fn get_account(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let account = state.db.get_account(&address).await?;
    match account {
        Some(a) => Ok(Json(a)),
        None => Err(GatewayError::NotFound(format!(
            "Account {} not found",
            address
        ))),
    }
}

async fn get_account_extrinsics(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsics = state
        .db
        .get_account_extrinsics(&address, pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(extrinsics))
}

async fn get_account_comits(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let comits = state
        .db
        .get_account_comits(&address, pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(comits))
}

// ============================================================================
// Stats endpoint
// ============================================================================

async fn get_stats(State(state): State<AppState>) -> Result<impl IntoResponse, GatewayError> {
    let stats = load_chain_stats(&state).await?;
    Ok(Json(stats))
}

async fn load_chain_stats(state: &AppState) -> Result<ChainStats, GatewayError> {
    if let Some(cache) = &state.redis_cache {
        match cache.get_chain_stats().await {
            Ok(Some(stats)) => {
                state.cache_metrics.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(stats);
            }
            Ok(None) => {
                state.cache_metrics.misses.fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                state
                    .cache_metrics
                    .fallbacks
                    .fetch_add(1, Ordering::Relaxed);
                tracing::warn!(error = %err, "redis cache read failed, falling back to database");
            }
        }
    }

    let stats = state.db.get_stats().await?;

    if let Some(cache) = &state.redis_cache {
        if let Err(err) = cache.set_chain_stats(&stats).await {
            tracing::warn!(error = %err, "redis cache write failed");
        }
    }

    Ok(stats)
}

async fn load_latest_block(state: &AppState) -> Result<Option<crate::db::Block>, GatewayError> {
    const KEY: &str = "x3-gateway:latest-block";
    const TTL_SECS: u64 = 3;

    if let Some(cache) = &state.redis_cache {
        match cache.get_json::<crate::db::Block>(KEY).await {
            Ok(Some(block)) => {
                state.cache_metrics.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(Some(block));
            }
            Ok(None) => {
                state.cache_metrics.misses.fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                state
                    .cache_metrics
                    .fallbacks
                    .fetch_add(1, Ordering::Relaxed);
                tracing::warn!(error = %err, "redis latest-block cache read failed");
            }
        }
    }

    let block = state.db.get_latest_block().await?;

    if let (Some(cache), Some(block_ref)) = (&state.redis_cache, block.as_ref()) {
        if let Err(err) = cache.set_json(KEY, block_ref, TTL_SECS).await {
            tracing::warn!(error = %err, "redis latest-block cache write failed");
        }
    }

    Ok(block)
}

async fn create_orchestra_intent(
    State(state): State<AppState>,
    Json(request): Json<NewOrchestraIntent>,
) -> Result<impl IntoResponse, GatewayError> {
    let intent =
        orchestra::create_orchestra_intent(&state.db, state.orchestra_client.as_ref(), request)
            .await?;
    Ok((StatusCode::CREATED, Json(intent)))
}

async fn list_orchestra_intents(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let intents = state
        .db
        .list_orchestra_intents(bounded_limit(pagination.limit), pagination.offset.max(0))
        .await?;
    Ok(Json(intents))
}

async fn get_orchestra_intent(
    State(state): State<AppState>,
    Path(intent_id): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    match state.db.get_orchestra_intent(&intent_id).await? {
        Some(intent) => Ok(Json(intent)),
        None => Err(GatewayError::NotFound(format!(
            "Intent {} not found",
            intent_id
        ))),
    }
}

async fn dispatch_orchestra_intent(
    State(state): State<AppState>,
    Path(intent_id): Path<String>,
    Json(request): Json<DispatchIntentRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let (intent, evidence) = orchestra::dispatch_orchestra_intent(
        &state.db,
        state.orchestra_client.as_ref(),
        &intent_id,
        DispatchEvidenceRequest {
            artifact_uri: request.artifact_uri,
            digest: request.digest,
            detail: request.detail,
        },
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(IntentDispatchResponse { intent, evidence }),
    ))
}

async fn create_approval_case(
    State(state): State<AppState>,
    Json(request): Json<NewApprovalCase>,
) -> Result<impl IntoResponse, GatewayError> {
    let approval_case =
        orchestra::create_approval_case(&state.db, state.orchestra_client.as_ref(), request)
            .await?;
    Ok((StatusCode::CREATED, Json(approval_case)))
}

async fn list_approval_cases(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let approval_cases = state
        .db
        .list_approval_cases(bounded_limit(pagination.limit), pagination.offset.max(0))
        .await?;
    Ok(Json(approval_cases))
}

async fn get_approval_case(
    State(state): State<AppState>,
    Path(case_id): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    match state.db.get_approval_case(&case_id).await? {
        Some(approval_case) => Ok(Json(approval_case)),
        None => Err(GatewayError::NotFound(format!(
            "Approval case {} not found",
            case_id
        ))),
    }
}

async fn create_vote_window(
    State(state): State<AppState>,
    Json(request): Json<NewVoteWindow>,
) -> Result<impl IntoResponse, GatewayError> {
    let vote_window =
        orchestra::create_vote_window(&state.db, state.orchestra_client.as_ref(), request).await?;
    Ok((StatusCode::CREATED, Json(vote_window)))
}

async fn list_vote_windows(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let vote_windows = state
        .db
        .list_vote_windows(bounded_limit(pagination.limit), pagination.offset.max(0))
        .await?;
    Ok(Json(vote_windows))
}

async fn get_vote_window(
    State(state): State<AppState>,
    Path(window_id): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    match state.db.get_vote_window(&window_id).await? {
        Some(vote_window) => Ok(Json(vote_window)),
        None => Err(GatewayError::NotFound(format!(
            "Vote window {} not found",
            window_id
        ))),
    }
}

async fn close_vote_window(
    State(state): State<AppState>,
    Path(window_id): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let (vote_window, approval_case, evidence) =
        orchestra::close_vote_window(&state.db, state.orchestra_client.as_ref(), &window_id)
            .await?;
    Ok((
        StatusCode::OK,
        Json(VoteWindowClosureResponse {
            vote_window,
            approval_case,
            evidence,
        }),
    ))
}

async fn import_vote_window_tally(
    State(state): State<AppState>,
    Path(window_id): Path<String>,
) -> Result<Json<VoteTally>, GatewayError> {
    Ok(Json(
        orchestra::import_vote_window_tally(&state.db, state.orchestra_client.as_ref(), &window_id)
            .await?,
    ))
}

async fn create_vote_receipt(
    State(state): State<AppState>,
    Path(window_id): Path<String>,
    Json(request): Json<NewVoteReceipt>,
) -> Result<impl IntoResponse, GatewayError> {
    let vote_receipt = orchestra::create_vote_receipt(
        &state.db,
        state.orchestra_client.as_ref(),
        &window_id,
        request,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(vote_receipt)))
}

async fn create_evidence_bundle(
    State(state): State<AppState>,
    Json(request): Json<NewEvidenceBundle>,
) -> Result<impl IntoResponse, GatewayError> {
    let evidence_bundle = state.db.create_evidence_bundle(request).await?;
    Ok((StatusCode::CREATED, Json(evidence_bundle)))
}

async fn list_evidence_bundles(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let evidence_bundles = state
        .db
        .list_evidence_bundles(bounded_limit(pagination.limit), pagination.offset.max(0))
        .await?;
    Ok(Json(evidence_bundles))
}

async fn get_evidence_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    match orchestra::get_evidence_bundle(&state.db, state.orchestra_client.as_ref(), &bundle_id)
        .await?
    {
        Some(evidence_bundle) => Ok(Json(evidence_bundle)),
        None => Err(GatewayError::NotFound(format!(
            "Evidence bundle {} not found",
            bundle_id
        ))),
    }
}

async fn get_benchmark_reports(
    State(state): State<AppState>,
    Query(query): Query<BenchmarkReportQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    if let Some(cached) = load_benchmark_reports_from_cache(&state, &query).await? {
        return Ok(Json::<Vec<BenchmarkReport>>(cached));
    }

    let fetch_limit = if query.min_high_conflict_ratio.is_some()
        || query.min_serial_fraction.is_some()
        || query.log_class.is_some()
    {
        (query.pagination.limit * 5).min(100)
    } else {
        query.pagination.limit.min(100)
    };

    let reports = state
        .db
        .get_benchmark_reports(
            query.tenant_id.as_deref(),
            fetch_limit,
            query.pagination.offset,
        )
        .await?;
    let mut filtered = reports
        .into_iter()
        .filter(|report| {
            benchmark_report_matches(
                report,
                query.min_high_conflict_ratio,
                query.min_serial_fraction,
                query.log_class.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    sort_benchmark_reports(
        &mut filtered,
        query.sort_by.as_deref(),
        query.sort_order.as_deref(),
    );

    filtered.truncate(query.pagination.limit.min(100) as usize);
    write_benchmark_reports_cache(&state, &query, &filtered).await;
    Ok(Json::<Vec<BenchmarkReport>>(filtered))
}

fn benchmark_reports_cache_key(query: &BenchmarkReportQuery) -> String {
    format!(
        "x3-gateway:benchmark-reports:tenant={}:min_hcr={:?}:min_sf={:?}:log={}:sort_by={}:sort_order={}:limit={}:offset={}",
        query.tenant_id.as_deref().unwrap_or(""),
        query.min_high_conflict_ratio,
        query.min_serial_fraction,
        query.log_class.as_deref().unwrap_or(""),
        query.sort_by.as_deref().unwrap_or(""),
        query.sort_order.as_deref().unwrap_or(""),
        query.pagination.limit,
        query.pagination.offset
    )
}

async fn load_benchmark_reports_from_cache(
    state: &AppState,
    query: &BenchmarkReportQuery,
) -> Result<Option<Vec<BenchmarkReport>>, GatewayError> {
    let Some(cache) = &state.redis_cache else {
        return Ok(None);
    };

    let key = benchmark_reports_cache_key(query);
    match cache.get_json::<Vec<BenchmarkReport>>(&key).await {
        Ok(Some(reports)) => {
            state.cache_metrics.hits.fetch_add(1, Ordering::Relaxed);
            Ok(Some(reports))
        }
        Ok(None) => {
            state.cache_metrics.misses.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        }
        Err(err) => {
            state
                .cache_metrics
                .fallbacks
                .fetch_add(1, Ordering::Relaxed);
            tracing::warn!(error = %err, "redis benchmark report cache read failed");
            Ok(None)
        }
    }
}

async fn write_benchmark_reports_cache(
    state: &AppState,
    query: &BenchmarkReportQuery,
    reports: &[BenchmarkReport],
) {
    const TTL_SECS: u64 = 5;
    if let Some(cache) = &state.redis_cache {
        let key = benchmark_reports_cache_key(query);
        if let Err(err) = cache.set_json(&key, &reports, TTL_SECS).await {
            tracing::warn!(error = %err, "redis benchmark report cache write failed");
        }
    }
}

async fn get_benchmark_report(
    State(state): State<AppState>,
    Path(report_id): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let report = state.db.get_benchmark_report(&report_id).await?;
    match report {
        Some(report) => Ok(Json::<BenchmarkReport>(report)),
        None => Err(GatewayError::NotFound(format!(
            "Benchmark report {} not found",
            report_id
        ))),
    }
}

async fn publish_benchmark_report(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<PublishBenchmarkReportRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    authorize_benchmark_publish(&state, &headers)?;
    if request.tenant_id.trim().is_empty() {
        return Err(GatewayError::BadRequest(
            "tenant_id is required".to_string(),
        ));
    }
    validate_benchmark_report_for_publish(&request.report)?;

    state
        .db
        .insert_benchmark_report(&StoredBenchmarkReport {
            tenant_id: request.tenant_id,
            report: request.report,
        })
        .await?;

    Ok(StatusCode::CREATED)
}

async fn submit_benchmark_result(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<SubmitBenchmarkResultRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    // Authorize the submission if a token is configured
    authorize_benchmark_publish(&state, &headers)?;

    if request.tenant_id.trim().is_empty() {
        return Err(GatewayError::BadRequest(
            "tenant_id is required".to_string(),
        ));
    }

    // Validate the report has required fields
    if request.report.report_id.trim().is_empty() {
        return Err(GatewayError::BadRequest(
            "report_id is required".to_string(),
        ));
    }
    validate_benchmark_report_for_publish(&request.report)?;

    // Store the benchmark report
    state
        .db
        .insert_benchmark_report(&StoredBenchmarkReport {
            tenant_id: request.tenant_id,
            report: request.report.clone(),
        })
        .await?;

    Ok(Json(BenchmarkResultResponse {
        report_id: request.report.report_id,
        status: "stored".to_string(),
    }))
}

fn authorize_benchmark_publish(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<(), GatewayError> {
    let Some(expected) = &state.benchmark_publish_token else {
        return Ok(());
    };

    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match provided {
        Some(token) if token == expected => Ok(()),
        _ => Err(GatewayError::BadRequest(
            "missing or invalid benchmark publish token".to_string(),
        )),
    }
}

fn benchmark_report_matches(
    report: &BenchmarkReport,
    min_high_conflict_ratio: Option<f64>,
    min_serial_fraction: Option<f64>,
    log_class: Option<&str>,
) -> bool {
    if let Some(min_high_conflict_ratio) = min_high_conflict_ratio {
        if report.workload_profile.high_conflict_ratio < min_high_conflict_ratio {
            return false;
        }
    }

    if let Some(min_serial_fraction) = min_serial_fraction {
        if report.workload_profile.estimated_serial_fraction < min_serial_fraction {
            return false;
        }
    }

    if let Some(log_class) = log_class {
        let wanted = log_class.to_lowercase();
        if !report
            .workload_profile
            .log_classes
            .iter()
            .any(|entry| entry.class_name.eq_ignore_ascii_case(&wanted))
        {
            return false;
        }
    }

    true
}

fn validate_benchmark_report_for_publish(report: &BenchmarkReport) -> Result<(), GatewayError> {
    if report.profile != BenchmarkProfile::ProviderOnboarding {
        return Ok(());
    }

    let provider_manifest = report
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_type == "provider-manifest")
        .ok_or_else(|| {
            GatewayError::BadRequest(
                "provider onboarding reports require a provider-manifest artifact".to_string(),
            )
        })?;
    let hardware_attestation = report
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_type == "hardware-attestation")
        .ok_or_else(|| {
            GatewayError::BadRequest(
                "provider onboarding reports require a hardware-attestation artifact".to_string(),
            )
        })?;

    validate_signed_onboarding_artifact(
        provider_manifest,
        &["provider_id", "operator_id", "region", "hardware_id"],
    )?;
    validate_signed_onboarding_artifact(
        hardware_attestation,
        &[
            "hardware_id",
            "hardware_kind",
            "cpu_model",
            "gpu_model",
            "memory_gb",
        ],
    )?;

    Ok(())
}

fn validate_signed_onboarding_artifact(
    artifact: &x3_rpc::benchmark::BenchmarkReportArtifact,
    required_fields: &[&str],
) -> Result<(), GatewayError> {
    let metadata = artifact.metadata.as_ref().ok_or_else(|| {
        GatewayError::BadRequest(format!(
            "{} artifacts must include signed metadata",
            artifact.artifact_type
        ))
    })?;
    let metadata = metadata.as_object().ok_or_else(|| {
        GatewayError::BadRequest(format!(
            "{} artifact metadata must be a JSON object",
            artifact.artifact_type
        ))
    })?;

    for field in required_fields {
        let value = metadata.get(*field).ok_or_else(|| {
            GatewayError::BadRequest(format!(
                "{} artifact metadata is missing {}",
                artifact.artifact_type, field
            ))
        })?;
        let is_empty_string = value
            .as_str()
            .map(|value| value.trim().is_empty())
            .unwrap_or(false);
        if value.is_null() || is_empty_string {
            return Err(GatewayError::BadRequest(format!(
                "{} artifact metadata has an empty {}",
                artifact.artifact_type, field
            )));
        }
    }

    let signature = artifact.signature.as_deref().unwrap_or("").trim();
    if signature.is_empty() {
        return Err(GatewayError::BadRequest(format!(
            "{} artifacts must include a signature",
            artifact.artifact_type
        )));
    }

    Ok(())
}

fn sort_benchmark_reports(
    reports: &mut Vec<BenchmarkReport>,
    sort_by: Option<&str>,
    sort_order: Option<&str>,
) {
    let field = sort_by
        .and_then(BenchmarkReportSortField::from_str)
        .unwrap_or(BenchmarkReportSortField::GeneratedAt);
    let order = sort_order
        .map(SortOrder::from_str)
        .unwrap_or(SortOrder::Desc);

    match (field, order) {
        (BenchmarkReportSortField::GeneratedAt, SortOrder::Asc) => {
            reports.sort_by_key(|r| r.generated_at_unix);
        }
        (BenchmarkReportSortField::GeneratedAt, SortOrder::Desc) => {
            reports.sort_by_key(|r| std::cmp::Reverse(r.generated_at_unix));
        }
        (BenchmarkReportSortField::HighConflictRatio, SortOrder::Asc) => {
            reports.sort_by(|a, b| {
                a.workload_profile
                    .high_conflict_ratio
                    .partial_cmp(&b.workload_profile.high_conflict_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::HighConflictRatio, SortOrder::Desc) => {
            reports.sort_by(|a, b| {
                b.workload_profile
                    .high_conflict_ratio
                    .partial_cmp(&a.workload_profile.high_conflict_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::SerialFraction, SortOrder::Asc) => {
            reports.sort_by(|a, b| {
                a.workload_profile
                    .estimated_serial_fraction
                    .partial_cmp(&b.workload_profile.estimated_serial_fraction)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::SerialFraction, SortOrder::Desc) => {
            reports.sort_by(|a, b| {
                b.workload_profile
                    .estimated_serial_fraction
                    .partial_cmp(&a.workload_profile.estimated_serial_fraction)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        (BenchmarkReportSortField::TotalTransactions, SortOrder::Asc) => {
            reports.sort_by_key(|r| r.workload_profile.total_transactions);
        }
        (BenchmarkReportSortField::TotalTransactions, SortOrder::Desc) => {
            reports.sort_by_key(|r| std::cmp::Reverse(r.workload_profile.total_transactions));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewEvidenceBundle, NewOrchestraIntent, NewVoteWindow};
    use crate::graphql::create_schema;
    use axum::{body::Body, http::Request, Router};
    use hyper::body::to_bytes;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tower::ServiceExt;
    use x3_orchestra_control_plane::ControlPlaneClient;
    use x3_rpc::benchmark::{
        BenchmarkChainType, BenchmarkIntegrationTier, BenchmarkLogClassStat, BenchmarkMetrics,
        BenchmarkProfile, BenchmarkReportArtifact, BenchmarkReportSummary,
        BenchmarkWorkloadProfile,
    };

    fn sample_report() -> BenchmarkReport {
        BenchmarkReport {
            report_id: "report-1".to_string(),
            generated_at_unix: 1,
            profile: BenchmarkProfile::Standard,
            chain_name: "PartnerChain".to_string(),
            chain_type: BenchmarkChainType::Evm,
            baseline: BenchmarkMetrics {
                avg_tps: 100.0,
                p50_latency_ms: 1000,
                p95_latency_ms: 2000,
                p99_latency_ms: 3000,
                failure_rate: 0.02,
            },
            x3_replay: BenchmarkMetrics {
                avg_tps: 240.0,
                p50_latency_ms: 300,
                p95_latency_ms: 700,
                p99_latency_ms: 1200,
                failure_rate: 0.005,
            },
            recommendation: BenchmarkIntegrationTier::TurboLaneMode,
            summary: BenchmarkReportSummary {
                projected_soft_confirmation_improvement: "3.3x faster".to_string(),
                projected_app_throughput_improvement: "2.4x higher".to_string(),
                projected_route_latency_delta: "65% lower".to_string(),
                projected_bridge_latency_delta: "60% lower".to_string(),
            },
            workload_profile: BenchmarkWorkloadProfile {
                total_transactions: 10,
                total_receipts: 10,
                total_logs: 6,
                active_lanes: 3,
                active_log_lanes: 2,
                low_conflict_ratio: 0.3,
                medium_conflict_ratio: 0.4,
                high_conflict_ratio: 0.3,
                estimated_serial_fraction: 0.36,
                log_classes: vec![
                    BenchmarkLogClassStat {
                        class_name: "erc20-transfer".to_string(),
                        count: 4,
                        share_of_logs: 0.66,
                        unique_contracts: 2,
                        unique_transactions: 4,
                    },
                    BenchmarkLogClassStat {
                        class_name: "bridge-event".to_string(),
                        count: 2,
                        share_of_logs: 0.33,
                        unique_contracts: 1,
                        unique_transactions: 2,
                    },
                ],
            },
            artifacts: vec![BenchmarkReportArtifact {
                artifact_type: "report-json".to_string(),
                uri: "benchmark://reports/report-1".to_string(),
                digest: "report-1".to_string(),
                metadata: None,
                signature: None,
            }],
            signer: "x3-sidecar".to_string(),
        }
    }

    #[test]
    fn benchmark_report_matches_conflict_filters() {
        let report = sample_report();
        assert!(benchmark_report_matches(
            &report,
            Some(0.25),
            Some(0.30),
            None
        ));
        assert!(!benchmark_report_matches(&report, Some(0.35), None, None));
        assert!(!benchmark_report_matches(&report, None, Some(0.40), None));
    }

    #[test]
    fn benchmark_report_matches_log_class_case_insensitive() {
        let report = sample_report();
        assert!(benchmark_report_matches(
            &report,
            None,
            None,
            Some("ERC20-TRANSFER")
        ));
        assert!(!benchmark_report_matches(
            &report,
            None,
            None,
            Some("amm-sync")
        ));
    }

    #[test]
    fn sort_benchmark_reports_by_generated_at_desc() {
        let mut reports = vec![
            BenchmarkReport {
                report_id: "report-1".to_string(),
                generated_at_unix: 100,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-2".to_string(),
                generated_at_unix: 300,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-3".to_string(),
                generated_at_unix: 200,
                ..sample_report()
            },
        ];
        sort_benchmark_reports(&mut reports, Some("generated_at"), Some("desc"));
        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-1");
    }

    #[test]
    fn sort_benchmark_reports_by_generated_at_asc() {
        let mut reports = vec![
            BenchmarkReport {
                report_id: "report-1".to_string(),
                generated_at_unix: 100,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-2".to_string(),
                generated_at_unix: 300,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-3".to_string(),
                generated_at_unix: 200,
                ..sample_report()
            },
        ];
        sort_benchmark_reports(&mut reports, Some("generated_at"), Some("asc"));
        assert_eq!(reports[0].report_id, "report-1");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-2");
    }

    #[test]
    fn sort_benchmark_reports_by_high_conflict_ratio_desc() {
        let mut report1 = sample_report();
        report1.report_id = "report-1".to_string();
        report1.workload_profile.high_conflict_ratio = 0.2;

        let mut report2 = sample_report();
        report2.report_id = "report-2".to_string();
        report2.workload_profile.high_conflict_ratio = 0.5;

        let mut report3 = sample_report();
        report3.report_id = "report-3".to_string();
        report3.workload_profile.high_conflict_ratio = 0.3;

        let mut reports = vec![report1, report2, report3];
        sort_benchmark_reports(&mut reports, Some("high_conflict_ratio"), Some("desc"));

        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-1");
    }

    #[test]
    fn sort_benchmark_reports_by_serial_fraction_asc() {
        let mut report1 = sample_report();
        report1.report_id = "report-1".to_string();
        report1.workload_profile.estimated_serial_fraction = 0.5;

        let mut report2 = sample_report();
        report2.report_id = "report-2".to_string();
        report2.workload_profile.estimated_serial_fraction = 0.2;

        let mut report3 = sample_report();
        report3.report_id = "report-3".to_string();
        report3.workload_profile.estimated_serial_fraction = 0.4;

        let mut reports = vec![report1, report2, report3];
        sort_benchmark_reports(&mut reports, Some("serial_fraction"), Some("asc"));

        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-3");
        assert_eq!(reports[2].report_id, "report-1");
    }

    #[test]
    fn sort_benchmark_reports_default_is_generated_at_desc() {
        let mut reports = vec![
            BenchmarkReport {
                report_id: "report-1".to_string(),
                generated_at_unix: 100,
                ..sample_report()
            },
            BenchmarkReport {
                report_id: "report-2".to_string(),
                generated_at_unix: 300,
                ..sample_report()
            },
        ];
        sort_benchmark_reports(&mut reports, None, None);
        assert_eq!(reports[0].report_id, "report-2");
        assert_eq!(reports[1].report_id, "report-1");
    }

    #[test]
    fn bounded_limit_clamps_values() {
        assert_eq!(bounded_limit(-50), 1);
        assert_eq!(bounded_limit(20), 20);
        assert_eq!(bounded_limit(500), 100);
    }

    #[test]
    fn orchestra_intent_request_shape_uses_object_payload() {
        let request = NewOrchestraIntent {
            tenant_id: "tenant-1".to_string(),
            kind: "research".to_string(),
            status: "pending".to_string(),
            risk_class: "high".to_string(),
            submitter: "operator".to_string(),
            requires_approval: true,
            payload: serde_json::json!({"campaign": "alpha"}),
        };

        assert!(request.payload.is_object());
    }

    #[test]
    fn vote_window_request_shape_uses_ordered_times() {
        let request = NewVoteWindow {
            approval_case_id: "case-1".to_string(),
            title: "risk review".to_string(),
            status: "scheduled".to_string(),
            opens_at_unix: 100,
            closes_at_unix: 200,
            electorate: serde_json::json!(["member-1"]),
        };

        assert!(request.opens_at_unix < request.closes_at_unix);
    }

    #[test]
    fn evidence_bundle_request_shape_uses_object_summary() {
        let request = NewEvidenceBundle {
            intent_id: Some("intent-1".to_string()),
            approval_case_id: None,
            vote_window_id: None,
            artifact_uri: "ipfs://bundle-1".to_string(),
            digest: "sha256:abc".to_string(),
            summary: serde_json::json!({"status": "approved"}),
        };

        assert!(request.summary.is_object());
    }

    #[test]
    fn provider_onboarding_reports_require_signed_metadata() {
        let mut report = sample_report();
        report.profile = BenchmarkProfile::ProviderOnboarding;

        report.artifacts.push(BenchmarkReportArtifact {
            artifact_type: "provider-manifest".to_string(),
            uri: "benchmark://providers/provider-1".to_string(),
            digest: "provider-1:operator-1".to_string(),
            metadata: Some(json!({
                "provider_id": "provider-1",
                "operator_id": "operator-1",
                "region": "us-east",
                "hardware_id": "gpu-rig-01"
            })),
            signature: None,
        });
        report.artifacts.push(BenchmarkReportArtifact {
            artifact_type: "hardware-attestation".to_string(),
            uri: "benchmark://hardware/gpu-rig-01".to_string(),
            digest: "gpu-rig-01:gpu-rig".to_string(),
            metadata: Some(json!({
                "hardware_id": "gpu-rig-01",
                "hardware_kind": "gpu-rig",
                "cpu_model": "EPYC 7B13",
                "gpu_model": "RTX 4090",
                "memory_gb": 256
            })),
            signature: Some("hardware-sig".to_string()),
        });

        let error = validate_benchmark_report_for_publish(&report)
            .expect_err("provider artifact without signature should fail");
        assert!(error
            .to_string()
            .contains("provider-manifest artifacts must include a signature"));

        report
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.artifact_type == "provider-manifest")
            .expect("provider manifest")
            .signature = Some("provider-sig".to_string());
        assert!(validate_benchmark_report_for_publish(&report).is_ok());
    }

    fn integration_database_url() -> Option<String> {
        std::env::var("X3_GATEWAY_TEST_DATABASE_URL")
            .ok()
            .or_else(|| std::env::var("DATABASE_URL").ok())
    }

    async fn read_json(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body())
            .await
            .expect("read response body");
        serde_json::from_slice(&body).expect("deserialize response body")
    }

    fn integration_app(db: Database) -> Router {
        let state = AppState {
            schema: create_schema(db.clone(), None),
            db,
            benchmark_publish_token: None,
            orchestra_client: None,
            redis_cache: None,
            cache_metrics: Arc::new(CacheMetrics::default()),
        };

        Router::new()
            .nest("/api/v1", api_routes())
            .with_state(state)
    }

    fn integration_app_with_orchestra_client(
        db: Database,
        orchestra_client: Arc<ControlPlaneClient>,
    ) -> Router {
        let state = AppState {
            schema: create_schema(db.clone(), Some(orchestra_client.clone())),
            db,
            benchmark_publish_token: None,
            orchestra_client: Some(orchestra_client),
            redis_cache: None,
            cache_metrics: Arc::new(CacheMetrics::default()),
        };

        Router::new()
            .nest("/api/v1", api_routes())
            .with_state(state)
    }

    async fn spawn_mock_control_plane() -> (String, tokio::task::JoinHandle<()>) {
        let app = axum::Router::new()
            .route(
                "/intents",
                post(|| async move {
                    Json(json!({
                        "intent_id": "remote-intent-1",
                        "tenant_id": "tenant-orchestra",
                        "kind": "publication",
                        "status": "pending_approval",
                        "risk_class": "high",
                        "submitter": "operator-1",
                        "requires_approval": true,
                        "payload": {"provider_id": "gpu-1"},
                        "created_at_unix": 100,
                        "updated_at_unix": 100
                    }))
                }),
            )
            .route(
                "/approval-cases",
                post(|| async move {
                    Json(json!({
                        "case_id": "remote-case-1",
                        "intent_id": "remote-intent-1",
                        "status": "open",
                        "review_kind": "operator-approval",
                        "requested_by": "operator-1",
                        "summary": "review provider onboarding",
                        "metadata": {"queue": "risk"},
                        "created_at_unix": 110,
                        "updated_at_unix": 110
                    }))
                }),
            )
            .route(
                "/vote-windows",
                post(|| async move {
                    Json(json!({
                        "window_id": "remote-window-1",
                        "approval_case_id": "remote-case-1",
                        "title": "risk review",
                        "status": "open",
                        "opens_at_unix": 100,
                        "closes_at_unix": 200,
                        "electorate": ["member-1", "member-2"],
                        "tally": {"approvals": 0, "rejections": 0, "abstentions": 0},
                        "created_at_unix": 120,
                        "updated_at_unix": 120
                    }))
                }),
            )
            .route(
                "/vote-windows/:window_id/votes",
                post(|| async move {
                    Json(json!({
                        "receipt_id": "remote-receipt-1",
                        "window_id": "remote-window-1",
                        "voter_id": "member-1",
                        "vote_choice": "approve",
                        "rationale": "looks good",
                        "cast_at_unix": 130
                    }))
                }),
            )
            .route(
                "/intents/:intent_id/dispatch",
                post(|| async move {
                    Json(json!({
                        "intent": {
                            "intent_id": "remote-intent-1",
                            "tenant_id": "tenant-orchestra",
                            "kind": "publication",
                            "status": "dispatched",
                            "risk_class": "high",
                            "submitter": "operator-1",
                            "requires_approval": true,
                            "payload": {"provider_id": "gpu-1"},
                            "created_at_unix": 100,
                            "updated_at_unix": 140
                        },
                        "evidence": {
                            "bundle_id": "remote-dispatch-evidence-1",
                            "intent_id": "remote-intent-1",
                            "approval_case_id": null,
                            "vote_window_id": null,
                            "artifact_uri": "ipfs://dispatch-evidence-1",
                            "digest": "sha256:dispatch-evidence-1",
                            "summary": {
                                "action": "dispatch_intent",
                                "detail": {"status": "dispatched"}
                            },
                            "created_at_unix": 140
                        }
                    }))
                }),
            )
            .route(
                "/vote-windows/:window_id/close",
                post(|| async move {
                    Json(json!({
                        "vote_window": {
                            "window_id": "remote-window-1",
                            "approval_case_id": "remote-case-1",
                            "title": "risk review",
                            "status": "closed",
                            "opens_at_unix": 100,
                            "closes_at_unix": 200,
                            "electorate": ["member-1", "member-2"],
                            "tally": {"approvals": 1, "rejections": 0, "abstentions": 0},
                            "created_at_unix": 120,
                            "updated_at_unix": 210
                        },
                        "approval_case": {
                            "case_id": "remote-case-1",
                            "intent_id": "remote-intent-1",
                            "status": "approved",
                            "review_kind": "operator-approval",
                            "requested_by": "operator-1",
                            "summary": "review provider onboarding",
                            "metadata": {"queue": "risk"},
                            "created_at_unix": 110,
                            "updated_at_unix": 210
                        },
                        "evidence": {
                            "bundle_id": "remote-close-evidence-1",
                            "intent_id": null,
                            "approval_case_id": "remote-case-1",
                            "vote_window_id": "remote-window-1",
                            "artifact_uri": "ipfs://close-evidence-1",
                            "digest": "sha256:close-evidence-1",
                            "summary": {
                                "action": "close_vote_window",
                                "detail": {"status": "approved"}
                            },
                            "created_at_unix": 210
                        }
                    }))
                }),
            )
            .route(
                "/vote-windows/:window_id/imported-tally",
                post(|| async move {
                    Json(json!({
                        "approvals": 1,
                        "rejections": 0,
                        "abstentions": 0
                    }))
                }),
            )
            .route(
                "/evidence/:bundle_id",
                get(|| async move {
                    Json(json!({
                        "bundle_id": "remote-fetched-evidence-1",
                        "intent_id": "remote-intent-1",
                        "approval_case_id": null,
                        "vote_window_id": null,
                        "artifact_uri": "ipfs://fetched-evidence-1",
                        "digest": "sha256:fetched-evidence-1",
                        "summary": {
                            "action": "dispatch_intent",
                            "detail": {"source": "control-plane"}
                        },
                        "created_at_unix": 220
                    }))
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock control-plane listener");
        let addr = listener
            .local_addr()
            .expect("mock control-plane local addr");
        let std_listener = listener
            .into_std()
            .expect("convert mock control-plane listener");
        let handle = tokio::spawn(async move {
            axum::Server::from_tcp(std_listener)
                .expect("serve mock control-plane from tcp")
                .serve(app.into_make_service())
                .await
                .expect("run mock control-plane server");
        });

        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn rest_reads_stay_segregated_by_workflow_type() {
        let Some(database_url) = integration_database_url() else {
            eprintln!("skipping gateway integration test: set X3_GATEWAY_TEST_DATABASE_URL or DATABASE_URL");
            return;
        };

        let (db, schema) = Database::connect_isolated_for_test(&database_url)
            .await
            .expect("create isolated test database");
        let app = integration_app(db.clone());

        let benchmark_request = json!({
            "tenant_id": "tenant-benchmark",
            "report": sample_report(),
        });
        let benchmark_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/benchmarks/reports")
                    .header("content-type", "application/json")
                    .body(Body::from(benchmark_request.to_string()))
                    .expect("build benchmark request"),
            )
            .await
            .expect("publish benchmark report");
        assert_eq!(benchmark_response.status(), StatusCode::CREATED);

        let intent_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/orchestra/intents")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "tenant_id": "tenant-orchestra",
                            "kind": "provider-onboarding",
                            "status": "pending",
                            "risk_class": "high",
                            "submitter": "operator-1",
                            "requires_approval": true,
                            "payload": {"provider_id": "gpu-1"}
                        })
                        .to_string(),
                    ))
                    .expect("build intent request"),
            )
            .await
            .expect("create orchestra intent");
        assert_eq!(intent_response.status(), StatusCode::CREATED);
        let intent_json = read_json(intent_response).await;
        let intent_id = intent_json["intent_id"]
            .as_str()
            .expect("intent id")
            .to_string();

        let approval_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/orchestra/approval-cases")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "intent_id": intent_id,
                            "status": "open",
                            "review_kind": "operator-approval",
                            "requested_by": "operator-1",
                            "summary": "review provider onboarding",
                            "metadata": {"queue": "risk"}
                        })
                        .to_string(),
                    ))
                    .expect("build approval request"),
            )
            .await
            .expect("create approval case");
        assert_eq!(approval_response.status(), StatusCode::CREATED);
        let approval_json = read_json(approval_response).await;
        let approval_case_id = approval_json["case_id"]
            .as_str()
            .expect("approval case id")
            .to_string();

        let evidence_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/orchestra/evidence-bundles")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "intent_id": intent_json["intent_id"],
                            "approval_case_id": approval_case_id,
                            "vote_window_id": null,
                            "artifact_uri": "ipfs://bundle-1",
                            "digest": "sha256:evidence-1",
                            "summary": {"status": "queued"}
                        })
                        .to_string(),
                    ))
                    .expect("build evidence request"),
            )
            .await
            .expect("create evidence bundle");
        assert_eq!(evidence_response.status(), StatusCode::CREATED);
        let evidence_json = read_json(evidence_response).await;
        let evidence_id = evidence_json["bundle_id"]
            .as_str()
            .expect("evidence bundle id")
            .to_string();

        let benchmark_list = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/benchmarks/reports")
                        .body(Body::empty())
                        .expect("build benchmark list request"),
                )
                .await
                .expect("list benchmark reports"),
        )
        .await;
        let intent_list = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/orchestra/intents")
                        .body(Body::empty())
                        .expect("build intent list request"),
                )
                .await
                .expect("list orchestra intents"),
        )
        .await;
        let approval_list = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/orchestra/approval-cases")
                        .body(Body::empty())
                        .expect("build approval list request"),
                )
                .await
                .expect("list approval cases"),
        )
        .await;
        let evidence_list = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/orchestra/evidence-bundles")
                        .body(Body::empty())
                        .expect("build evidence list request"),
                )
                .await
                .expect("list evidence bundles"),
        )
        .await;
        let benchmark_single = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/benchmarks/reports/report-1")
                        .body(Body::empty())
                        .expect("build benchmark get request"),
                )
                .await
                .expect("get benchmark report"),
        )
        .await;
        let intent_single = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!(
                            "/api/v1/orchestra/intents/{}",
                            intent_json["intent_id"].as_str().expect("intent id")
                        ))
                        .body(Body::empty())
                        .expect("build intent get request"),
                )
                .await
                .expect("get orchestra intent"),
        )
        .await;
        let approval_single = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!(
                            "/api/v1/orchestra/approval-cases/{approval_case_id}"
                        ))
                        .body(Body::empty())
                        .expect("build approval get request"),
                )
                .await
                .expect("get approval case"),
        )
        .await;
        let evidence_single = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/api/v1/orchestra/evidence-bundles/{evidence_id}"))
                        .body(Body::empty())
                        .expect("build evidence get request"),
                )
                .await
                .expect("get evidence bundle"),
        )
        .await;

        assert_eq!(benchmark_list.as_array().expect("benchmark list").len(), 1);
        assert_eq!(intent_list.as_array().expect("intent list").len(), 1);
        assert_eq!(approval_list.as_array().expect("approval list").len(), 1);
        assert_eq!(evidence_list.as_array().expect("evidence list").len(), 1);

        assert_eq!(benchmark_single["report_id"], json!("report-1"));
        assert!(benchmark_single.get("intent_id").is_none());

        assert_eq!(intent_single["intent_id"], intent_json["intent_id"]);
        assert!(intent_single.get("report_id").is_none());

        assert_eq!(approval_single["case_id"], approval_json["case_id"]);
        assert_eq!(approval_single["intent_id"], intent_json["intent_id"]);
        assert!(approval_single.get("report_id").is_none());

        assert_eq!(evidence_single["bundle_id"], evidence_json["bundle_id"]);
        assert_eq!(evidence_single["intent_id"], intent_json["intent_id"]);
        assert_eq!(
            evidence_single["approval_case_id"],
            approval_json["case_id"]
        );
        assert!(evidence_single.get("report_id").is_none());

        Database::drop_test_schema(&database_url, &schema)
            .await
            .expect("drop isolated test schema");
    }

    #[tokio::test]
    async fn orchestra_write_endpoints_relay_to_control_plane_when_configured() {
        let Some(database_url) = integration_database_url() else {
            eprintln!("skipping gateway relay integration test: set X3_GATEWAY_TEST_DATABASE_URL or DATABASE_URL");
            return;
        };

        let (db, schema) = Database::connect_isolated_for_test(&database_url)
            .await
            .expect("create isolated test database");
        let (control_plane_url, control_plane_handle) = spawn_mock_control_plane().await;
        let app = integration_app_with_orchestra_client(
            db.clone(),
            Arc::new(ControlPlaneClient::new(control_plane_url, None)),
        );

        let intent_json = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/orchestra/intents")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "tenant_id": "tenant-orchestra",
                                "kind": "publication",
                                "status": "pending",
                                "risk_class": "high",
                                "submitter": "operator-1",
                                "requires_approval": false,
                                "payload": {"provider_id": "gpu-1"}
                            })
                            .to_string(),
                        ))
                        .expect("build relayed intent request"),
                )
                .await
                .expect("create relayed orchestra intent"),
        )
        .await;

        assert_eq!(intent_json["intent_id"], json!("remote-intent-1"));
        assert_eq!(intent_json["status"], json!("pending_approval"));
        assert_eq!(intent_json["requires_approval"], json!(true));

        let approval_json = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/orchestra/approval-cases")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "intent_id": "remote-intent-1",
                                "status": "closed",
                                "review_kind": "operator-approval",
                                "requested_by": "operator-1",
                                "summary": "review provider onboarding",
                                "metadata": {"queue": "risk"}
                            })
                            .to_string(),
                        ))
                        .expect("build relayed approval request"),
                )
                .await
                .expect("create relayed approval case"),
        )
        .await;

        assert_eq!(approval_json["case_id"], json!("remote-case-1"));
        assert_eq!(approval_json["status"], json!("open"));

        let vote_window_json = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/orchestra/vote-windows")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "approval_case_id": "remote-case-1",
                                "title": "risk review",
                                "status": "scheduled",
                                "opens_at_unix": 100,
                                "closes_at_unix": 200,
                                "electorate": ["bogus-member"]
                            })
                            .to_string(),
                        ))
                        .expect("build relayed vote window request"),
                )
                .await
                .expect("create relayed vote window"),
        )
        .await;

        assert_eq!(vote_window_json["window_id"], json!("remote-window-1"));
        assert_eq!(vote_window_json["status"], json!("open"));
        assert_eq!(
            vote_window_json["electorate"],
            json!(["member-1", "member-2"])
        );

        let vote_receipt_json = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/orchestra/vote-windows/remote-window-1/receipts")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "voter_id": "member-1",
                                "vote_choice": "approve",
                                "rationale": "looks good",
                                "cast_at_unix": 130
                            })
                            .to_string(),
                        ))
                        .expect("build relayed vote receipt request"),
                )
                .await
                .expect("create relayed vote receipt"),
        )
        .await;

        assert_eq!(vote_receipt_json["receipt_id"], json!("remote-receipt-1"));
        assert_eq!(vote_receipt_json["window_id"], json!("remote-window-1"));
        assert_eq!(vote_receipt_json["vote_choice"], json!("approve"));

        let dispatch_json = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/orchestra/intents/remote-intent-1/dispatch")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            json!({
                                "artifact_uri": "ipfs://dispatch-proof",
                                "digest": "sha256:dispatch-proof",
                                "detail": {"executor": "gateway"}
                            })
                            .to_string(),
                        ))
                        .expect("build relayed dispatch request"),
                )
                .await
                .expect("dispatch relayed orchestra intent"),
        )
        .await;
        assert_eq!(dispatch_json["intent"]["status"], json!("dispatched"));
        assert_eq!(
            dispatch_json["evidence"]["bundle_id"],
            json!("remote-dispatch-evidence-1")
        );

        let imported_tally_json = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/orchestra/vote-windows/remote-window-1/imported-tally")
                        .body(Body::empty())
                        .expect("build relayed imported tally request"),
                )
                .await
                .expect("import relayed vote tally"),
        )
        .await;
        assert_eq!(imported_tally_json["approvals"], json!(1));

        let closure_json = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/orchestra/vote-windows/remote-window-1/close")
                        .body(Body::empty())
                        .expect("build relayed vote window close request"),
                )
                .await
                .expect("close relayed vote window"),
        )
        .await;
        assert_eq!(closure_json["vote_window"]["status"], json!("closed"));
        assert_eq!(closure_json["approval_case"]["status"], json!("approved"));
        assert_eq!(
            closure_json["evidence"]["bundle_id"],
            json!("remote-close-evidence-1")
        );

        let fetched_remote_evidence = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/orchestra/evidence-bundles/remote-fetched-evidence-1")
                        .body(Body::empty())
                        .expect("build remote evidence fetch request"),
                )
                .await
                .expect("fetch remote evidence through gateway"),
        )
        .await;
        assert_eq!(
            fetched_remote_evidence["bundle_id"],
            json!("remote-fetched-evidence-1")
        );

        let persisted_intent = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/orchestra/intents/remote-intent-1")
                        .body(Body::empty())
                        .expect("build persisted intent request"),
                )
                .await
                .expect("get persisted intent"),
        )
        .await;
        let persisted_vote_window = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/orchestra/vote-windows/remote-window-1")
                        .body(Body::empty())
                        .expect("build persisted vote window request"),
                )
                .await
                .expect("get persisted vote window"),
        )
        .await;

        let persisted_dispatch_evidence = read_json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/v1/orchestra/evidence-bundles/remote-dispatch-evidence-1")
                        .body(Body::empty())
                        .expect("build persisted dispatch evidence request"),
                )
                .await
                .expect("get persisted dispatch evidence"),
        )
        .await;

        assert_eq!(persisted_intent["status"], json!("dispatched"));
        assert_eq!(
            persisted_vote_window["electorate"],
            json!(["member-1", "member-2"])
        );
        assert_eq!(persisted_vote_window["status"], json!("closed"));
        assert_eq!(
            persisted_vote_window["tally"],
            json!({"approvals": 1, "rejections": 0, "abstentions": 0})
        );
        assert_eq!(
            persisted_dispatch_evidence["bundle_id"],
            json!("remote-dispatch-evidence-1")
        );

        control_plane_handle.abort();
        Database::drop_test_schema(&database_url, &schema)
            .await
            .expect("drop isolated test schema");
    }
}
