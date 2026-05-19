//! Database connection and queries.

use crate::config::DatabaseConfig;
use crate::error::{GatewayError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;
use x3_orchestra_control_plane::{
    ApprovalCase as ControlPlaneApprovalCase, ApprovalStatus as ControlPlaneApprovalStatus,
    Intent as ControlPlaneIntent, IntentKind as ControlPlaneIntentKind,
    IntentStatus as ControlPlaneIntentStatus, VoteChoice as ControlPlaneVoteChoice,
    VoteReceipt as ControlPlaneVoteReceipt, VoteWindow as ControlPlaneVoteWindow,
    VoteWindowStatus as ControlPlaneVoteWindowStatus,
};
use x3_rpc::benchmark::{
    BenchmarkChainType, BenchmarkIntegrationTier, BenchmarkMetrics, BenchmarkReport,
    BenchmarkReportArtifact, BenchmarkReportSummary, BenchmarkWorkloadProfile,
};

/// Database connection pool wrapper.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

// ============================================================================
// Models
// ============================================================================

/// Block data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Block {
    pub number: i64,
    pub hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub timestamp: DateTime<Utc>,
    pub author: Option<String>,
    pub extrinsic_count: i32,
    pub event_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Extrinsic data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Extrinsic {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub hash: String,
    pub pallet: String,
    pub call: String,
    pub signer: Option<String>,
    pub success: bool,
    pub fee: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Event data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: Option<i32>,
    pub event_index: i32,
    pub pallet: String,
    pub variant: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Comit transaction data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ComitTransaction {
    pub id: i64,
    pub block_number: i64,
    pub extrinsic_index: i32,
    pub comit_hash: String,
    pub origin: String,
    pub evm_payload_size: i32,
    pub svm_payload_size: i32,
    pub evm_gas_used: Option<i64>,
    pub svm_compute_used: Option<i64>,
    pub fee_paid: String,
    pub success: bool,
    pub evm_success: Option<bool>,
    pub svm_success: Option<bool>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Account data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub address: String,
    pub native_balance: String,
    pub nonce: i64,
    pub is_authorized: bool,
    pub first_seen_block: i64,
    pub last_seen_block: i64,
    pub total_transactions: i64,
    pub updated_at: DateTime<Utc>,
}

/// Chain statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStats {
    pub total_blocks: i64,
    pub latest_block: Option<i64>,
    pub total_extrinsics: i64,
    pub total_events: i64,
    pub total_comits: i64,
    pub successful_comits: i64,
    pub failed_comits: i64,
    pub total_accounts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BenchmarkReportRow {
    pub report_id: String,
    pub tenant_id: String,
    pub chain_name: String,
    pub chain_type: String,
    pub recommendation: String,
    pub signer: String,
    pub generated_at: DateTime<Utc>,
    pub baseline_avg_tps: f64,
    pub baseline_p50_latency_ms: i64,
    pub baseline_p95_latency_ms: i64,
    pub baseline_p99_latency_ms: i64,
    pub baseline_failure_rate: f64,
    pub x3_avg_tps: f64,
    pub x3_p50_latency_ms: i64,
    pub x3_p95_latency_ms: i64,
    pub x3_p99_latency_ms: i64,
    pub x3_failure_rate: f64,
    pub projected_soft_confirmation_improvement: String,
    pub projected_app_throughput_improvement: String,
    pub projected_route_latency_delta: String,
    pub projected_bridge_latency_delta: String,
    pub workload_profile: serde_json::Value,
    pub artifacts: serde_json::Value,
}

impl TryFrom<BenchmarkReportRow> for BenchmarkReport {
    type Error = crate::error::GatewayError;

    fn try_from(value: BenchmarkReportRow) -> std::result::Result<Self, Self::Error> {
        let artifacts = serde_json::from_value::<Vec<BenchmarkReportArtifact>>(value.artifacts)
            .map_err(|e| {
                crate::error::GatewayError::Internal(format!("invalid benchmark artifacts: {e}"))
            })?;
        let workload_profile = serde_json::from_value::<BenchmarkWorkloadProfile>(
            value.workload_profile,
        )
        .map_err(|e| {
            crate::error::GatewayError::Internal(format!("invalid benchmark workload profile: {e}"))
        })?;

        Ok(BenchmarkReport {
            report_id: value.report_id,
            generated_at_unix: value.generated_at.timestamp() as u64,
            profile: x3_rpc::benchmark::BenchmarkProfile::Standard,
            chain_name: value.chain_name,
            chain_type: parse_chain_type(&value.chain_type)?,
            baseline: BenchmarkMetrics {
                avg_tps: value.baseline_avg_tps,
                p50_latency_ms: value.baseline_p50_latency_ms as u64,
                p95_latency_ms: value.baseline_p95_latency_ms as u64,
                p99_latency_ms: value.baseline_p99_latency_ms as u64,
                failure_rate: value.baseline_failure_rate,
            },
            x3_replay: BenchmarkMetrics {
                avg_tps: value.x3_avg_tps,
                p50_latency_ms: value.x3_p50_latency_ms as u64,
                p95_latency_ms: value.x3_p95_latency_ms as u64,
                p99_latency_ms: value.x3_p99_latency_ms as u64,
                failure_rate: value.x3_failure_rate,
            },
            recommendation: parse_integration_tier(&value.recommendation)?,
            summary: BenchmarkReportSummary {
                projected_soft_confirmation_improvement: value
                    .projected_soft_confirmation_improvement,
                projected_app_throughput_improvement: value.projected_app_throughput_improvement,
                projected_route_latency_delta: value.projected_route_latency_delta,
                projected_bridge_latency_delta: value.projected_bridge_latency_delta,
            },
            workload_profile,
            artifacts,
            signer: value.signer,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredBenchmarkReport {
    pub tenant_id: String,
    pub report: BenchmarkReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrchestraIntent {
    pub intent_id: String,
    pub tenant_id: String,
    pub kind: String,
    pub status: String,
    pub risk_class: String,
    pub submitter: String,
    pub requires_approval: bool,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrchestraIntent {
    pub tenant_id: String,
    pub kind: String,
    pub status: String,
    pub risk_class: String,
    pub submitter: String,
    pub requires_approval: bool,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApprovalCase {
    pub case_id: String,
    pub intent_id: String,
    pub status: String,
    pub review_kind: String,
    pub requested_by: String,
    pub summary: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewApprovalCase {
    pub intent_id: String,
    pub status: String,
    pub review_kind: String,
    pub requested_by: String,
    pub summary: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VoteWindow {
    pub window_id: String,
    pub approval_case_id: String,
    pub title: String,
    pub status: String,
    pub opens_at_unix: i64,
    pub closes_at_unix: i64,
    pub electorate: serde_json::Value,
    pub tally: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewVoteWindow {
    pub approval_case_id: String,
    pub title: String,
    pub status: String,
    pub opens_at_unix: i64,
    pub closes_at_unix: i64,
    pub electorate: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VoteReceipt {
    pub receipt_id: String,
    pub window_id: String,
    pub voter_id: String,
    pub vote_choice: String,
    pub rationale: Option<String>,
    pub cast_at_unix: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewVoteReceipt {
    pub voter_id: String,
    pub vote_choice: String,
    pub rationale: Option<String>,
    pub cast_at_unix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EvidenceBundle {
    pub bundle_id: String,
    pub intent_id: Option<String>,
    pub approval_case_id: Option<String>,
    pub vote_window_id: Option<String>,
    pub artifact_uri: String,
    pub digest: String,
    pub summary: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEvidenceBundle {
    pub intent_id: Option<String>,
    pub approval_case_id: Option<String>,
    pub vote_window_id: Option<String>,
    pub artifact_uri: String,
    pub digest: String,
    pub summary: serde_json::Value,
}

fn require_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(GatewayError::BadRequest(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn validate_orchestra_intent(input: &NewOrchestraIntent) -> Result<()> {
    require_non_empty(&input.tenant_id, "tenant_id")?;
    require_non_empty(&input.kind, "kind")?;
    require_non_empty(&input.status, "status")?;
    require_non_empty(&input.risk_class, "risk_class")?;
    require_non_empty(&input.submitter, "submitter")?;
    if !input.payload.is_object() {
        return Err(GatewayError::BadRequest(
            "payload must be a JSON object".to_string(),
        ));
    }
    Ok(())
}

fn validate_approval_case(input: &NewApprovalCase) -> Result<()> {
    require_non_empty(&input.intent_id, "intent_id")?;
    require_non_empty(&input.status, "status")?;
    require_non_empty(&input.review_kind, "review_kind")?;
    require_non_empty(&input.requested_by, "requested_by")?;
    require_non_empty(&input.summary, "summary")?;
    if !input.metadata.is_object() {
        return Err(GatewayError::BadRequest(
            "metadata must be a JSON object".to_string(),
        ));
    }
    Ok(())
}

fn validate_vote_window(input: &NewVoteWindow) -> Result<()> {
    require_non_empty(&input.approval_case_id, "approval_case_id")?;
    require_non_empty(&input.title, "title")?;
    require_non_empty(&input.status, "status")?;
    if input.opens_at_unix >= input.closes_at_unix {
        return Err(GatewayError::BadRequest(
            "opens_at_unix must be earlier than closes_at_unix".to_string(),
        ));
    }
    if !input.electorate.is_object() && !input.electorate.is_array() {
        return Err(GatewayError::BadRequest(
            "electorate must be a JSON object or array".to_string(),
        ));
    }
    Ok(())
}

fn validate_vote_receipt(input: &NewVoteReceipt) -> Result<()> {
    require_non_empty(&input.voter_id, "voter_id")?;
    require_non_empty(&input.vote_choice, "vote_choice")?;
    Ok(())
}

fn validate_evidence_bundle(input: &NewEvidenceBundle) -> Result<()> {
    require_non_empty(&input.artifact_uri, "artifact_uri")?;
    require_non_empty(&input.digest, "digest")?;
    if !input.summary.is_object() {
        return Err(GatewayError::BadRequest(
            "summary must be a JSON object".to_string(),
        ));
    }
    Ok(())
}

fn parse_chain_type(
    value: &str,
) -> std::result::Result<BenchmarkChainType, crate::error::GatewayError> {
    match value {
        "evm" => Ok(BenchmarkChainType::Evm),
        "op-stack" => Ok(BenchmarkChainType::OpStack),
        "substrate" => Ok(BenchmarkChainType::Substrate),
        "cosmos" => Ok(BenchmarkChainType::Cosmos),
        "svm" => Ok(BenchmarkChainType::Svm),
        "custom" => Ok(BenchmarkChainType::Custom),
        other => Err(crate::error::GatewayError::Internal(format!(
            "unknown benchmark chain type {other}"
        ))),
    }
}

fn parse_integration_tier(
    value: &str,
) -> std::result::Result<BenchmarkIntegrationTier, crate::error::GatewayError> {
    match value {
        "benchmark-only" => Ok(BenchmarkIntegrationTier::BenchmarkOnly),
        "sidecar-mode" => Ok(BenchmarkIntegrationTier::SidecarMode),
        "turbo-lane-mode" => Ok(BenchmarkIntegrationTier::TurboLaneMode),
        "shared-settlement-mode" => Ok(BenchmarkIntegrationTier::SharedSettlementMode),
        other => Err(crate::error::GatewayError::Internal(format!(
            "unknown benchmark integration tier {other}"
        ))),
    }
}

// ============================================================================
// Database Implementation
// ============================================================================

impl Database {
    /// Connect to the database.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&config.url)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|err| GatewayError::Internal(format!("database migration failed: {err}")))?;

        info!("Database connected");

        Ok(Self { pool })
    }

    /// Get the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[cfg(test)]
    pub(crate) async fn connect_isolated_for_test(database_url: &str) -> Result<(Self, String)> {
        use std::sync::Arc;

        let schema = format!("x3_gateway_test_{}", Uuid::new_v4().simple());
        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;

        sqlx::query(&format!("CREATE SCHEMA {schema}"))
            .execute(&admin_pool)
            .await?;

        let search_path_schema = Arc::new(schema.clone());
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |conn, _meta| {
                let search_path_schema = Arc::clone(&search_path_schema);
                Box::pin(async move {
                    let query = format!("SET search_path TO {}", search_path_schema.as_str());
                    sqlx::query(&query).execute(conn).await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|err| GatewayError::Internal(format!("database migration failed: {err}")))?;

        Ok((Self { pool }, schema))
    }

    #[cfg(test)]
    pub(crate) async fn drop_test_schema(database_url: &str, schema: &str) -> Result<()> {
        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;

        sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
            .execute(&admin_pool)
            .await?;

        Ok(())
    }

    pub async fn create_orchestra_intent(
        &self,
        input: NewOrchestraIntent,
    ) -> Result<OrchestraIntent> {
        validate_orchestra_intent(&input)?;

        let intent_id = Uuid::new_v4().to_string();
        let row = sqlx::query_as(
            "INSERT INTO orchestra_intents (intent_id, tenant_id, kind, status, risk_class, submitter, requires_approval, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING *",
        )
        .bind(&intent_id)
        .bind(&input.tenant_id)
        .bind(&input.kind)
        .bind(&input.status)
        .bind(&input.risk_class)
        .bind(&input.submitter)
        .bind(input.requires_approval)
        .bind(input.payload)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert_orchestra_intent_from_control_plane(
        &self,
        intent: &ControlPlaneIntent,
    ) -> Result<OrchestraIntent> {
        let row = sqlx::query_as(
            "INSERT INTO orchestra_intents (intent_id, tenant_id, kind, status, risk_class, submitter, requires_approval, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9), to_timestamp($10))
             ON CONFLICT (intent_id) DO UPDATE SET
                 tenant_id = EXCLUDED.tenant_id,
                 kind = EXCLUDED.kind,
                 status = EXCLUDED.status,
                 risk_class = EXCLUDED.risk_class,
                 submitter = EXCLUDED.submitter,
                 requires_approval = EXCLUDED.requires_approval,
                 payload = EXCLUDED.payload,
                 created_at = EXCLUDED.created_at,
                 updated_at = EXCLUDED.updated_at
             RETURNING *",
        )
        .bind(&intent.intent_id)
        .bind(&intent.tenant_id)
        .bind(control_plane_intent_kind_as_str(&intent.kind))
        .bind(control_plane_intent_status_as_str(&intent.status))
        .bind(control_plane_risk_class_as_str(&intent.risk_class))
        .bind(&intent.submitter)
        .bind(intent.requires_approval)
        .bind(intent.payload.clone())
        .bind(intent.created_at_unix as i64)
        .bind(intent.updated_at_unix as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_orchestra_intent(&self, intent_id: &str) -> Result<Option<OrchestraIntent>> {
        sqlx::query_as("SELECT * FROM orchestra_intents WHERE intent_id = $1")
            .bind(intent_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_orchestra_intents(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrchestraIntent>> {
        sqlx::query_as(
            "SELECT * FROM orchestra_intents ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create_approval_case(&self, input: NewApprovalCase) -> Result<ApprovalCase> {
        validate_approval_case(&input)?;

        if self.get_orchestra_intent(&input.intent_id).await?.is_none() {
            return Err(GatewayError::NotFound(format!(
                "intent {} not found",
                input.intent_id
            )));
        }

        let case_id = Uuid::new_v4().to_string();
        let row = sqlx::query_as(
            "INSERT INTO approval_cases (case_id, intent_id, status, review_kind, requested_by, summary, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(&case_id)
        .bind(&input.intent_id)
        .bind(&input.status)
        .bind(&input.review_kind)
        .bind(&input.requested_by)
        .bind(&input.summary)
        .bind(input.metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert_approval_case_from_control_plane(
        &self,
        approval_case: &ControlPlaneApprovalCase,
    ) -> Result<ApprovalCase> {
        let row = sqlx::query_as(
            "INSERT INTO approval_cases (case_id, intent_id, status, review_kind, requested_by, summary, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8), to_timestamp($9))
             ON CONFLICT (case_id) DO UPDATE SET
                 intent_id = EXCLUDED.intent_id,
                 status = EXCLUDED.status,
                 review_kind = EXCLUDED.review_kind,
                 requested_by = EXCLUDED.requested_by,
                 summary = EXCLUDED.summary,
                 metadata = EXCLUDED.metadata,
                 created_at = EXCLUDED.created_at,
                 updated_at = EXCLUDED.updated_at
             RETURNING *",
        )
        .bind(&approval_case.case_id)
        .bind(&approval_case.intent_id)
        .bind(control_plane_approval_status_as_str(&approval_case.status))
        .bind(&approval_case.review_kind)
        .bind(&approval_case.requested_by)
        .bind(&approval_case.summary)
        .bind(approval_case.metadata.clone())
        .bind(approval_case.created_at_unix as i64)
        .bind(approval_case.updated_at_unix as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_approval_case(&self, case_id: &str) -> Result<Option<ApprovalCase>> {
        sqlx::query_as("SELECT * FROM approval_cases WHERE case_id = $1")
            .bind(case_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_approval_cases(&self, limit: i64, offset: i64) -> Result<Vec<ApprovalCase>> {
        sqlx::query_as("SELECT * FROM approval_cases ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn create_vote_window(&self, input: NewVoteWindow) -> Result<VoteWindow> {
        validate_vote_window(&input)?;

        if self
            .get_approval_case(&input.approval_case_id)
            .await?
            .is_none()
        {
            return Err(GatewayError::NotFound(format!(
                "approval case {} not found",
                input.approval_case_id
            )));
        }

        let window_id = Uuid::new_v4().to_string();
        let row = sqlx::query_as(
            "INSERT INTO vote_windows (window_id, approval_case_id, title, status, opens_at_unix, closes_at_unix, electorate, tally)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING *",
        )
        .bind(&window_id)
        .bind(&input.approval_case_id)
        .bind(&input.title)
        .bind(&input.status)
        .bind(input.opens_at_unix)
        .bind(input.closes_at_unix)
        .bind(input.electorate)
        .bind(serde_json::json!({
            "approvals": 0,
            "rejections": 0,
            "abstentions": 0,
        }))
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert_vote_window_from_control_plane(
        &self,
        vote_window: &ControlPlaneVoteWindow,
    ) -> Result<VoteWindow> {
        let row = sqlx::query_as(
            "INSERT INTO vote_windows (window_id, approval_case_id, title, status, opens_at_unix, closes_at_unix, electorate, tally, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9), to_timestamp($10))
             ON CONFLICT (window_id) DO UPDATE SET
                 approval_case_id = EXCLUDED.approval_case_id,
                 title = EXCLUDED.title,
                 status = EXCLUDED.status,
                 opens_at_unix = EXCLUDED.opens_at_unix,
                 closes_at_unix = EXCLUDED.closes_at_unix,
                 electorate = EXCLUDED.electorate,
                 tally = EXCLUDED.tally,
                 created_at = EXCLUDED.created_at,
                 updated_at = EXCLUDED.updated_at
             RETURNING *",
        )
        .bind(&vote_window.window_id)
        .bind(&vote_window.approval_case_id)
        .bind(&vote_window.title)
        .bind(control_plane_vote_window_status_as_str(&vote_window.status))
        .bind(vote_window.opens_at_unix as i64)
        .bind(vote_window.closes_at_unix as i64)
        .bind(serde_json::to_value(vote_window.electorate.clone())?)
        .bind(serde_json::json!({
            "approvals": vote_window.tally.approvals,
            "rejections": vote_window.tally.rejections,
            "abstentions": vote_window.tally.abstentions,
        }))
        .bind(vote_window.created_at_unix as i64)
        .bind(vote_window.updated_at_unix as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn update_vote_window_tally(
        &self,
        window_id: &str,
        tally: &x3_orchestra_control_plane::VoteTally,
    ) -> Result<Option<VoteWindow>> {
        sqlx::query_as(
            "UPDATE vote_windows
             SET tally = $2,
                 updated_at = NOW()
             WHERE window_id = $1
             RETURNING *",
        )
        .bind(window_id)
        .bind(serde_json::json!({
            "approvals": tally.approvals,
            "rejections": tally.rejections,
            "abstentions": tally.abstentions,
        }))
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_vote_window(&self, window_id: &str) -> Result<Option<VoteWindow>> {
        sqlx::query_as("SELECT * FROM vote_windows WHERE window_id = $1")
            .bind(window_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_vote_windows(&self, limit: i64, offset: i64) -> Result<Vec<VoteWindow>> {
        sqlx::query_as("SELECT * FROM vote_windows ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn create_vote_receipt(
        &self,
        window_id: &str,
        input: NewVoteReceipt,
    ) -> Result<VoteReceipt> {
        validate_vote_receipt(&input)?;

        if self.get_vote_window(window_id).await?.is_none() {
            return Err(GatewayError::NotFound(format!(
                "vote window {} not found",
                window_id
            )));
        }

        let receipt_id = Uuid::new_v4().to_string();
        let result = sqlx::query_as(
            "INSERT INTO vote_receipts (receipt_id, window_id, voter_id, vote_choice, rationale, cast_at_unix)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(&receipt_id)
        .bind(window_id)
        .bind(&input.voter_id)
        .bind(&input.vote_choice)
        .bind(&input.rationale)
        .bind(input.cast_at_unix)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(row) => Ok(row),
            Err(sqlx::Error::Database(db_err))
                if db_err.constraint() == Some("vote_receipts_window_id_voter_id_key") =>
            {
                Err(GatewayError::BadRequest(
                    "vote already recorded for this voter in the target window".to_string(),
                ))
            }
            Err(err) => Err(err.into()),
        }
    }

    pub async fn upsert_vote_receipt_from_control_plane(
        &self,
        receipt: &ControlPlaneVoteReceipt,
    ) -> Result<VoteReceipt> {
        let row = sqlx::query_as(
            "INSERT INTO vote_receipts (receipt_id, window_id, voter_id, vote_choice, rationale, cast_at_unix, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7))
             ON CONFLICT (receipt_id) DO UPDATE SET
                 window_id = EXCLUDED.window_id,
                 voter_id = EXCLUDED.voter_id,
                 vote_choice = EXCLUDED.vote_choice,
                 rationale = EXCLUDED.rationale,
                 cast_at_unix = EXCLUDED.cast_at_unix,
                 created_at = EXCLUDED.created_at
             RETURNING *",
        )
        .bind(&receipt.receipt_id)
        .bind(&receipt.window_id)
        .bind(&receipt.voter_id)
        .bind(control_plane_vote_choice_as_str(&receipt.vote_choice))
        .bind(&receipt.rationale)
        .bind(receipt.cast_at_unix as i64)
        .bind(receipt.cast_at_unix as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn create_evidence_bundle(&self, input: NewEvidenceBundle) -> Result<EvidenceBundle> {
        validate_evidence_bundle(&input)?;

        let bundle_id = Uuid::new_v4().to_string();
        let row = sqlx::query_as(
            "INSERT INTO evidence_bundles (bundle_id, intent_id, approval_case_id, vote_window_id, artifact_uri, digest, summary)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(&bundle_id)
        .bind(&input.intent_id)
        .bind(&input.approval_case_id)
        .bind(&input.vote_window_id)
        .bind(&input.artifact_uri)
        .bind(&input.digest)
        .bind(input.summary)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert_evidence_bundle_from_control_plane(
        &self,
        evidence_bundle: &x3_orchestra_control_plane::EvidenceBundle,
    ) -> Result<EvidenceBundle> {
        let row = sqlx::query_as(
            "INSERT INTO evidence_bundles (bundle_id, intent_id, approval_case_id, vote_window_id, artifact_uri, digest, summary, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8), to_timestamp($9))
             ON CONFLICT (bundle_id) DO UPDATE SET
                 intent_id = EXCLUDED.intent_id,
                 approval_case_id = EXCLUDED.approval_case_id,
                 vote_window_id = EXCLUDED.vote_window_id,
                 artifact_uri = EXCLUDED.artifact_uri,
                 digest = EXCLUDED.digest,
                 summary = EXCLUDED.summary,
                 created_at = EXCLUDED.created_at,
                 updated_at = EXCLUDED.updated_at
             RETURNING *",
        )
        .bind(&evidence_bundle.bundle_id)
        .bind(&evidence_bundle.intent_id)
        .bind(&evidence_bundle.approval_case_id)
        .bind(&evidence_bundle.vote_window_id)
        .bind(&evidence_bundle.artifact_uri)
        .bind(&evidence_bundle.digest)
        .bind(serde_json::json!({
            "action": evidence_bundle.summary.action,
            "detail": evidence_bundle.summary.detail,
        }))
        .bind(evidence_bundle.created_at_unix as i64)
        .bind(evidence_bundle.created_at_unix as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_evidence_bundle(&self, bundle_id: &str) -> Result<Option<EvidenceBundle>> {
        sqlx::query_as("SELECT * FROM evidence_bundles WHERE bundle_id = $1")
            .bind(bundle_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_evidence_bundles(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EvidenceBundle>> {
        sqlx::query_as("SELECT * FROM evidence_bundles ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_benchmark_report(&self, report_id: &str) -> Result<Option<BenchmarkReport>> {
        let report: Option<BenchmarkReportRow> =
            sqlx::query_as("SELECT * FROM benchmark_reports WHERE report_id = $1")
                .bind(report_id)
                .fetch_optional(&self.pool)
                .await?;

        report.map(TryInto::try_into).transpose()
    }

    pub async fn insert_benchmark_report(&self, stored: &StoredBenchmarkReport) -> Result<()> {
        let artifacts = serde_json::to_value(&stored.report.artifacts)?;
        let workload_profile = serde_json::to_value(&stored.report.workload_profile)?;
        sqlx::query(
            "INSERT INTO benchmark_reports (
                report_id,
                tenant_id,
                chain_name,
                chain_type,
                recommendation,
                signer,
                generated_at,
                baseline_avg_tps,
                baseline_p50_latency_ms,
                baseline_p95_latency_ms,
                baseline_p99_latency_ms,
                baseline_failure_rate,
                x3_avg_tps,
                x3_p50_latency_ms,
                x3_p95_latency_ms,
                x3_p99_latency_ms,
                x3_failure_rate,
                projected_soft_confirmation_improvement,
                projected_app_throughput_improvement,
                projected_route_latency_delta,
                projected_bridge_latency_delta,
                workload_profile,
                artifacts
            ) VALUES (
                $1,$2,$3,$4,$5,$6,to_timestamp($7),$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23
            )
            ON CONFLICT (report_id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                chain_name = EXCLUDED.chain_name,
                chain_type = EXCLUDED.chain_type,
                recommendation = EXCLUDED.recommendation,
                signer = EXCLUDED.signer,
                generated_at = EXCLUDED.generated_at,
                baseline_avg_tps = EXCLUDED.baseline_avg_tps,
                baseline_p50_latency_ms = EXCLUDED.baseline_p50_latency_ms,
                baseline_p95_latency_ms = EXCLUDED.baseline_p95_latency_ms,
                baseline_p99_latency_ms = EXCLUDED.baseline_p99_latency_ms,
                baseline_failure_rate = EXCLUDED.baseline_failure_rate,
                x3_avg_tps = EXCLUDED.x3_avg_tps,
                x3_p50_latency_ms = EXCLUDED.x3_p50_latency_ms,
                x3_p95_latency_ms = EXCLUDED.x3_p95_latency_ms,
                x3_p99_latency_ms = EXCLUDED.x3_p99_latency_ms,
                x3_failure_rate = EXCLUDED.x3_failure_rate,
                projected_soft_confirmation_improvement = EXCLUDED.projected_soft_confirmation_improvement,
                projected_app_throughput_improvement = EXCLUDED.projected_app_throughput_improvement,
                projected_route_latency_delta = EXCLUDED.projected_route_latency_delta,
                projected_bridge_latency_delta = EXCLUDED.projected_bridge_latency_delta,
                workload_profile = EXCLUDED.workload_profile,
                artifacts = EXCLUDED.artifacts",
        )
        .bind(&stored.report.report_id)
        .bind(&stored.tenant_id)
        .bind(&stored.report.chain_name)
        .bind(chain_type_as_str(&stored.report.chain_type))
        .bind(integration_tier_as_str(&stored.report.recommendation))
        .bind(&stored.report.signer)
        .bind(stored.report.generated_at_unix as i64)
        .bind(stored.report.baseline.avg_tps)
        .bind(stored.report.baseline.p50_latency_ms as i64)
        .bind(stored.report.baseline.p95_latency_ms as i64)
        .bind(stored.report.baseline.p99_latency_ms as i64)
        .bind(stored.report.baseline.failure_rate)
        .bind(stored.report.x3_replay.avg_tps)
        .bind(stored.report.x3_replay.p50_latency_ms as i64)
        .bind(stored.report.x3_replay.p95_latency_ms as i64)
        .bind(stored.report.x3_replay.p99_latency_ms as i64)
        .bind(stored.report.x3_replay.failure_rate)
        .bind(&stored.report.summary.projected_soft_confirmation_improvement)
        .bind(&stored.report.summary.projected_app_throughput_improvement)
        .bind(&stored.report.summary.projected_route_latency_delta)
        .bind(&stored.report.summary.projected_bridge_latency_delta)
        .bind(workload_profile)
        .bind(artifacts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_benchmark_reports(
        &self,
        tenant_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BenchmarkReport>> {
        let rows: Vec<BenchmarkReportRow> = if let Some(tenant_id) = tenant_id {
            sqlx::query_as(
                "SELECT * FROM benchmark_reports WHERE tenant_id = $1 ORDER BY generated_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(tenant_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT * FROM benchmark_reports ORDER BY generated_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(TryInto::try_into).collect()
    }

    // ========================================================================
    // Block queries
    // ========================================================================

    /// Get block by number.
    pub async fn get_block(&self, number: i64) -> Result<Option<Block>> {
        let block: Option<Block> = sqlx::query_as("SELECT * FROM blocks WHERE number = $1")
            .bind(number)
            .fetch_optional(&self.pool)
            .await?;

        Ok(block)
    }

    /// Get block by hash.
    pub async fn get_block_by_hash(&self, hash: &str) -> Result<Option<Block>> {
        let block: Option<Block> = sqlx::query_as("SELECT * FROM blocks WHERE hash = $1")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(block)
    }

    /// Get latest block.
    pub async fn get_latest_block(&self) -> Result<Option<Block>> {
        let block: Option<Block> =
            sqlx::query_as("SELECT * FROM blocks ORDER BY number DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        Ok(block)
    }

    /// Get recent blocks.
    pub async fn get_recent_blocks(&self, limit: i64, offset: i64) -> Result<Vec<Block>> {
        let blocks: Vec<Block> =
            sqlx::query_as("SELECT * FROM blocks ORDER BY number DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        Ok(blocks)
    }

    /// Get blocks in range.
    pub async fn get_blocks_range(&self, from: i64, to: i64) -> Result<Vec<Block>> {
        let blocks: Vec<Block> = sqlx::query_as(
            "SELECT * FROM blocks WHERE number >= $1 AND number <= $2 ORDER BY number",
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(blocks)
    }

    // ========================================================================
    // Extrinsic queries
    // ========================================================================

    /// Get extrinsic by hash.
    pub async fn get_extrinsic(&self, hash: &str) -> Result<Option<Extrinsic>> {
        let ext: Option<Extrinsic> = sqlx::query_as("SELECT * FROM extrinsics WHERE hash = $1")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(ext)
    }

    /// Get extrinsics for a block.
    pub async fn get_block_extrinsics(&self, block_number: i64) -> Result<Vec<Extrinsic>> {
        let exts: Vec<Extrinsic> = sqlx::query_as(
            "SELECT * FROM extrinsics WHERE block_number = $1 ORDER BY extrinsic_index",
        )
        .bind(block_number)
        .fetch_all(&self.pool)
        .await?;

        Ok(exts)
    }

    /// Get extrinsics by account.
    pub async fn get_account_extrinsics(
        &self,
        address: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Extrinsic>> {
        let exts: Vec<Extrinsic> = sqlx::query_as(
            "SELECT * FROM extrinsics WHERE signer = $1 ORDER BY id DESC LIMIT $2 OFFSET $3",
        )
        .bind(address)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(exts)
    }

    /// Get recent extrinsics.
    pub async fn get_recent_extrinsics(&self, limit: i64, offset: i64) -> Result<Vec<Extrinsic>> {
        let exts: Vec<Extrinsic> =
            sqlx::query_as("SELECT * FROM extrinsics ORDER BY id DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        Ok(exts)
    }

    // ========================================================================
    // Event queries
    // ========================================================================

    /// Get events for a block.
    pub async fn get_block_events(&self, block_number: i64) -> Result<Vec<Event>> {
        let events: Vec<Event> =
            sqlx::query_as("SELECT * FROM events WHERE block_number = $1 ORDER BY event_index")
                .bind(block_number)
                .fetch_all(&self.pool)
                .await?;

        Ok(events)
    }

    /// Get events by pallet.
    pub async fn get_events_by_pallet(
        &self,
        pallet: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Event>> {
        let events: Vec<Event> = sqlx::query_as(
            "SELECT * FROM events WHERE pallet = $1 ORDER BY id DESC LIMIT $2 OFFSET $3",
        )
        .bind(pallet)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get events by pallet and variant.
    pub async fn get_events_by_type(
        &self,
        pallet: &str,
        variant: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Event>> {
        let events: Vec<Event> = sqlx::query_as(
            "SELECT * FROM events WHERE pallet = $1 AND variant = $2 ORDER BY id DESC LIMIT $3 OFFSET $4"
        )
        .bind(pallet)
        .bind(variant)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    // ========================================================================
    // Comit queries
    // ========================================================================

    /// Get Comit by hash.
    pub async fn get_comit(&self, hash: &str) -> Result<Option<ComitTransaction>> {
        let comit: Option<ComitTransaction> =
            sqlx::query_as("SELECT * FROM comit_transactions WHERE comit_hash = $1")
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(comit)
    }

    /// Get recent Comits.
    pub async fn get_recent_comits(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ComitTransaction>> {
        let comits: Vec<ComitTransaction> =
            sqlx::query_as("SELECT * FROM comit_transactions ORDER BY id DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        Ok(comits)
    }

    /// Get Comits by origin account.
    pub async fn get_account_comits(
        &self,
        origin: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ComitTransaction>> {
        let comits: Vec<ComitTransaction> = sqlx::query_as(
            "SELECT * FROM comit_transactions WHERE origin = $1 ORDER BY id DESC LIMIT $2 OFFSET $3"
        )
        .bind(origin)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(comits)
    }

    // ========================================================================
    // Account queries
    // ========================================================================

    /// Get account by address.
    pub async fn get_account(&self, address: &str) -> Result<Option<Account>> {
        let account: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE address = $1")
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;

        Ok(account)
    }

    /// Get top accounts by balance.
    pub async fn get_top_accounts(&self, limit: i64) -> Result<Vec<Account>> {
        let accounts: Vec<Account> = sqlx::query_as(
            "SELECT * FROM accounts ORDER BY CAST(native_balance AS NUMERIC) DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(accounts)
    }

    /// Search accounts.
    pub async fn search_accounts(&self, query: &str, limit: i64) -> Result<Vec<Account>> {
        let pattern = format!("{}%", query);
        let accounts: Vec<Account> =
            sqlx::query_as("SELECT * FROM accounts WHERE address LIKE $1 LIMIT $2")
                .bind(&pattern)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?;

        Ok(accounts)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get chain statistics.
    pub async fn get_stats(&self) -> Result<ChainStats> {
        let total_blocks: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM blocks")
            .fetch_one(&self.pool)
            .await?;

        let latest: Option<(i64,)> = sqlx::query_as("SELECT MAX(number) FROM blocks")
            .fetch_optional(&self.pool)
            .await?;

        let total_extrinsics: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM extrinsics")
            .fetch_one(&self.pool)
            .await?;

        let total_events: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM events")
            .fetch_one(&self.pool)
            .await?;

        let total_comits: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM comit_transactions")
                .fetch_one(&self.pool)
                .await?;

        let successful_comits: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM comit_transactions WHERE success = true")
                .fetch_one(&self.pool)
                .await?;

        let total_accounts: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM accounts")
            .fetch_one(&self.pool)
            .await?;

        Ok(ChainStats {
            total_blocks: total_blocks.0,
            latest_block: latest.and_then(|l| Some(l.0)),
            total_extrinsics: total_extrinsics.0,
            total_events: total_events.0,
            total_comits: total_comits.0,
            successful_comits: successful_comits.0,
            failed_comits: total_comits.0 - successful_comits.0,
            total_accounts: total_accounts.0,
        })
    }
}

fn control_plane_intent_kind_as_str(kind: &ControlPlaneIntentKind) -> &'static str {
    match kind {
        ControlPlaneIntentKind::Validation => "validation",
        ControlPlaneIntentKind::Benchmarking => "benchmarking",
        ControlPlaneIntentKind::Publication => "publication",
        ControlPlaneIntentKind::Sanctions => "sanctions",
        ControlPlaneIntentKind::TreasuryAction => "treasury_action",
        ControlPlaneIntentKind::StrategyActivation => "strategy_activation",
    }
}

fn control_plane_intent_status_as_str(status: &ControlPlaneIntentStatus) -> &'static str {
    match status {
        ControlPlaneIntentStatus::PendingApproval => "pending_approval",
        ControlPlaneIntentStatus::Ready => "ready",
        ControlPlaneIntentStatus::Dispatched => "dispatched",
        ControlPlaneIntentStatus::Completed => "completed",
        ControlPlaneIntentStatus::Blocked => "blocked",
    }
}

fn control_plane_risk_class_as_str(
    risk_class: &x3_orchestra_control_plane::RiskClass,
) -> &'static str {
    match risk_class {
        x3_orchestra_control_plane::RiskClass::Low => "low",
        x3_orchestra_control_plane::RiskClass::Medium => "medium",
        x3_orchestra_control_plane::RiskClass::High => "high",
        x3_orchestra_control_plane::RiskClass::Critical => "critical",
    }
}

fn control_plane_approval_status_as_str(status: &ControlPlaneApprovalStatus) -> &'static str {
    match status {
        ControlPlaneApprovalStatus::Open => "open",
        ControlPlaneApprovalStatus::Approved => "approved",
        ControlPlaneApprovalStatus::Rejected => "rejected",
    }
}

fn control_plane_vote_window_status_as_str(status: &ControlPlaneVoteWindowStatus) -> &'static str {
    match status {
        ControlPlaneVoteWindowStatus::Scheduled => "scheduled",
        ControlPlaneVoteWindowStatus::Open => "open",
        ControlPlaneVoteWindowStatus::Closed => "closed",
    }
}

fn control_plane_vote_choice_as_str(choice: &ControlPlaneVoteChoice) -> &'static str {
    match choice {
        ControlPlaneVoteChoice::Approve => "approve",
        ControlPlaneVoteChoice::Reject => "reject",
        ControlPlaneVoteChoice::Abstain => "abstain",
    }
}

fn chain_type_as_str(value: &BenchmarkChainType) -> &'static str {
    match value {
        BenchmarkChainType::Evm => "evm",
        BenchmarkChainType::OpStack => "op-stack",
        BenchmarkChainType::Substrate => "substrate",
        BenchmarkChainType::Cosmos => "cosmos",
        BenchmarkChainType::Svm => "svm",
        BenchmarkChainType::Custom => "custom",
    }
}

fn integration_tier_as_str(value: &BenchmarkIntegrationTier) -> &'static str {
    match value {
        BenchmarkIntegrationTier::BenchmarkOnly => "benchmark-only",
        BenchmarkIntegrationTier::SidecarMode => "sidecar-mode",
        BenchmarkIntegrationTier::TurboLaneMode => "turbo-lane-mode",
        BenchmarkIntegrationTier::SharedSettlementMode => "shared-settlement-mode",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchestra_intent_validation_rejects_non_object_payload() {
        let result = validate_orchestra_intent(&NewOrchestraIntent {
            tenant_id: "tenant-1".to_string(),
            kind: "benchmark".to_string(),
            status: "pending".to_string(),
            risk_class: "high".to_string(),
            submitter: "operator".to_string(),
            requires_approval: true,
            payload: serde_json::json!("invalid"),
        });

        assert!(matches!(result, Err(GatewayError::BadRequest(_))));
    }

    #[test]
    fn vote_window_validation_rejects_invalid_time_range() {
        let result = validate_vote_window(&NewVoteWindow {
            approval_case_id: "case-1".to_string(),
            title: "launch vote".to_string(),
            status: "scheduled".to_string(),
            opens_at_unix: 10,
            closes_at_unix: 10,
            electorate: serde_json::json!(["voter-1"]),
        });

        assert!(matches!(result, Err(GatewayError::BadRequest(_))));
    }

    #[test]
    fn evidence_bundle_validation_requires_summary_object() {
        let result = validate_evidence_bundle(&NewEvidenceBundle {
            intent_id: None,
            approval_case_id: None,
            vote_window_id: None,
            artifact_uri: "ipfs://bundle".to_string(),
            digest: "sha256:abc".to_string(),
            summary: serde_json::json!("invalid"),
        });

        assert!(matches!(result, Err(GatewayError::BadRequest(_))));
    }
}
