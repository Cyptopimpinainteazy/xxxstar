use jsonrpc_core::{Error as JsonRpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::Row;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BenchmarkChainType {
    Evm,
    OpStack,
    Substrate,
    Cosmos,
    Svm,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BenchmarkIntegrationTier {
    BenchmarkOnly,
    SidecarMode,
    TurboLaneMode,
    SharedSettlementMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BenchmarkProfile {
    #[default]
    Standard,
    ProviderOnboarding,
}

impl std::str::FromStr for BenchmarkJobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(BenchmarkJobStatus::Queued),
            "running" => Ok(BenchmarkJobStatus::Running),
            "completed" => Ok(BenchmarkJobStatus::Completed),
            "failed" => Ok(BenchmarkJobStatus::Failed),
            _ => Err(format!("unknown job status: {}", s)),
        }
    }
}

impl Display for BenchmarkJobStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchmarkJobStatus::Queued => write!(f, "queued"),
            BenchmarkJobStatus::Running => write!(f, "running"),
            BenchmarkJobStatus::Completed => write!(f, "completed"),
            BenchmarkJobStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub avg_tps: f64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub failure_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReportArtifact {
    pub artifact_type: String,
    pub uri: String,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOnboardingMetadata {
    pub provider_id: String,
    pub operator_id: String,
    pub region: String,
    pub hardware_id: String,
    pub hardware_kind: String,
    pub cpu_model: String,
    pub gpu_model: String,
    pub memory_gb: u64,
    pub provider_signature: String,
    pub hardware_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReportSummary {
    pub projected_soft_confirmation_improvement: String,
    pub projected_app_throughput_improvement: String,
    pub projected_route_latency_delta: String,
    pub projected_bridge_latency_delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkLogClassStat {
    pub class_name: String,
    pub count: u64,
    pub share_of_logs: f64,
    pub unique_contracts: u64,
    pub unique_transactions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkWorkloadProfile {
    pub total_transactions: u64,
    pub total_receipts: u64,
    pub total_logs: u64,
    pub active_lanes: u64,
    pub active_log_lanes: u64,
    pub low_conflict_ratio: f64,
    pub medium_conflict_ratio: f64,
    pub high_conflict_ratio: f64,
    pub estimated_serial_fraction: f64,
    pub log_classes: Vec<BenchmarkLogClassStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub report_id: String,
    pub generated_at_unix: u64,
    #[serde(default)]
    pub profile: BenchmarkProfile,
    pub chain_name: String,
    pub chain_type: BenchmarkChainType,
    pub baseline: BenchmarkMetrics,
    pub x3_replay: BenchmarkMetrics,
    pub recommendation: BenchmarkIntegrationTier,
    pub summary: BenchmarkReportSummary,
    pub workload_profile: BenchmarkWorkloadProfile,
    pub artifacts: Vec<BenchmarkReportArtifact>,
    pub signer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkJobRequest {
    pub tenant_id: String,
    #[serde(default)]
    pub profile: BenchmarkProfile,
    pub chain_name: String,
    pub chain_type: BenchmarkChainType,
    pub rpc_endpoints: Vec<String>,
    pub explorer_endpoint: Option<String>,
    pub workload_trace_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onboarding_metadata: Option<ProviderOnboardingMetadata>,
    pub date_range_start_unix: u64,
    pub date_range_end_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkJobResponse {
    pub job_id: String,
    pub status: BenchmarkJobStatus,
    pub report_id: Option<String>,
    pub submitted_at_unix: u64,
    pub updated_at_unix: u64,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum BenchmarkError {
    InvalidRequest(String),
    NotFound(String),
    Internal(String),
}

impl Display for BenchmarkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "invalid request: {msg}"),
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for BenchmarkError {}

impl From<BenchmarkError> for JsonRpcError {
    fn from(value: BenchmarkError) -> Self {
        match value {
            BenchmarkError::InvalidRequest(message) => JsonRpcError {
                code: ErrorCode::InvalidParams,
                message,
                data: None,
            },
            BenchmarkError::NotFound(message) => JsonRpcError {
                code: ErrorCode::ServerError(404),
                message,
                data: None,
            },
            BenchmarkError::Internal(message) => JsonRpcError {
                code: ErrorCode::InternalError,
                message,
                data: None,
            },
        }
    }
}

pub trait BenchmarkService: Send + Sync + 'static {
    fn submit_job(
        &self,
        request: BenchmarkJobRequest,
    ) -> Result<BenchmarkJobResponse, BenchmarkError>;
    fn get_job(&self, job_id: &str) -> Result<BenchmarkJobResponse, BenchmarkError>;
    fn get_report(&self, report_id: &str) -> Result<BenchmarkReport, BenchmarkError>;
    fn list_jobs(
        &self,
        tenant_id: Option<String>,
    ) -> Result<Vec<BenchmarkJobResponse>, BenchmarkError>;
}

/// Database-backed benchmark service implementation using PostgreSQL.
/// Provides persistent job storage, status tracking, and report retrieval.
///
/// Note: Due to JSON-RPC requiring synchronous trait methods, this implementation
/// uses blocking database calls. In a production environment, consider using
/// tokio::task::block_in_place within a tokio runtime context, or refactor the
/// RPC layer to support async methods.
#[derive(Clone)]
pub struct DatabaseBenchmarkService {
    pool: PgPool,
}

impl DatabaseBenchmarkService {
    /// Creates a new DatabaseBenchmarkService with the provided connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Validates a benchmark job request, returning an error if any required fields are missing.
    fn validate_request(request: &BenchmarkJobRequest) -> Result<(), BenchmarkError> {
        if request.tenant_id.trim().is_empty() {
            return Err(BenchmarkError::InvalidRequest(
                "tenant_id is required".to_string(),
            ));
        }
        if request.chain_name.trim().is_empty() {
            return Err(BenchmarkError::InvalidRequest(
                "chain_name is required".to_string(),
            ));
        }
        if request.rpc_endpoints.is_empty() {
            return Err(BenchmarkError::InvalidRequest(
                "at least one rpc_endpoints entry is required".to_string(),
            ));
        }
        if request.date_range_end_unix <= request.date_range_start_unix {
            return Err(BenchmarkError::InvalidRequest(
                "date range end must be greater than start".to_string(),
            ));
        }
        Ok(())
    }

    /// Serializes BenchmarkChainType to string for database storage.
    fn chain_type_to_string(ct: &BenchmarkChainType) -> String {
        match ct {
            BenchmarkChainType::Evm => "evm",
            BenchmarkChainType::OpStack => "opstack",
            BenchmarkChainType::Substrate => "substrate",
            BenchmarkChainType::Cosmos => "cosmos",
            BenchmarkChainType::Svm => "svm",
            BenchmarkChainType::Custom => "custom",
        }
        .to_string()
    }

    /// Deserializes string from database to BenchmarkChainType.
    fn string_to_chain_type(s: &str) -> BenchmarkChainType {
        match s {
            "evm" => BenchmarkChainType::Evm,
            "opstack" => BenchmarkChainType::OpStack,
            "substrate" => BenchmarkChainType::Substrate,
            "cosmos" => BenchmarkChainType::Cosmos,
            "svm" => BenchmarkChainType::Svm,
            _ => BenchmarkChainType::Custom,
        }
    }

    /// Serializes BenchmarkIntegrationTier to string for database storage.
    fn recommendation_to_string(tier: &BenchmarkIntegrationTier) -> String {
        match tier {
            BenchmarkIntegrationTier::BenchmarkOnly => "benchmark-only",
            BenchmarkIntegrationTier::SidecarMode => "sidecar-mode",
            BenchmarkIntegrationTier::TurboLaneMode => "turbo-lane-mode",
            BenchmarkIntegrationTier::SharedSettlementMode => "shared-settlement-mode",
        }
        .to_string()
    }

    /// Deserializes string from database to BenchmarkIntegrationTier.
    fn string_to_recommendation(s: &str) -> BenchmarkIntegrationTier {
        match s {
            "benchmark-only" => BenchmarkIntegrationTier::BenchmarkOnly,
            "sidecar-mode" => BenchmarkIntegrationTier::SidecarMode,
            "turbo-lane-mode" => BenchmarkIntegrationTier::TurboLaneMode,
            "shared-settlement-mode" => BenchmarkIntegrationTier::SharedSettlementMode,
            _ => BenchmarkIntegrationTier::BenchmarkOnly,
        }
    }

    /// Converts a database row to a BenchmarkJobResponse.
    fn row_to_response(
        &self,
        row: (String, String, Option<String>, i64, i64, Option<String>),
    ) -> BenchmarkJobResponse {
        let (job_id, status_str, report_id, submitted_at_unix, updated_at_unix, error_message) =
            row;
        let status = status_str
            .parse::<BenchmarkJobStatus>()
            .unwrap_or(BenchmarkJobStatus::Failed);

        BenchmarkJobResponse {
            job_id,
            status,
            report_id,
            submitted_at_unix: submitted_at_unix as u64,
            updated_at_unix: updated_at_unix as u64,
            error: error_message,
        }
    }
}

impl BenchmarkService for DatabaseBenchmarkService {
    fn submit_job(
        &self,
        request: BenchmarkJobRequest,
    ) -> Result<BenchmarkJobResponse, BenchmarkError> {
        // Validate all request inputs upfront
        Self::validate_request(&request)?;

        let job_id = Uuid::new_v4().to_string();
        let now = now_unix();
        let chain_type_str = Self::chain_type_to_string(&request.chain_type);

        // Execute blocking database operation using tokio::task::block_in_place
        // This requires being called from within a tokio runtime
        let job_id_result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(async {
                sqlx::query(
                    r#"
                    INSERT INTO benchmark_jobs 
                    (job_id, tenant_id, chain_name, chain_type, rpc_endpoints, 
                     explorer_endpoint, workload_trace_uri, date_range_start_unix, 
                     date_range_end_unix, status, submitted_at_unix, updated_at_unix)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    "#,
                )
                .bind(&job_id)
                .bind(&request.tenant_id)
                .bind(&request.chain_name)
                .bind(&chain_type_str)
                .bind(&request.rpc_endpoints)
                .bind(&request.explorer_endpoint)
                .bind(&request.workload_trace_uri)
                .bind(request.date_range_start_unix as i64)
                .bind(request.date_range_end_unix as i64)
                .bind("queued")
                .bind(now as i64)
                .bind(now as i64)
                .execute(&self.pool)
                .await
            })
        } else {
            Err(sqlx::Error::PoolClosed)
        };

        match job_id_result {
            Ok(_) => Ok(BenchmarkJobResponse {
                job_id,
                status: BenchmarkJobStatus::Queued,
                report_id: None,
                submitted_at_unix: now,
                updated_at_unix: now,
                error: None,
            }),
            Err(e) => Err(BenchmarkError::Internal(format!(
                "failed to insert benchmark job: {}",
                e
            ))),
        }
    }

    fn get_job(&self, job_id: &str) -> Result<BenchmarkJobResponse, BenchmarkError> {
        let job_id_str = job_id.to_string();

        let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(async {
                sqlx::query_as::<_, (String, String, Option<String>, i64, i64, Option<String>)>(
                    r#"
                    SELECT job_id, status, report_id, submitted_at_unix, updated_at_unix, error_message
                    FROM benchmark_jobs
                    WHERE job_id = $1
                    "#
                )
                .bind(&job_id_str)
                .fetch_optional(&self.pool)
                .await
            })
        } else {
            Err(sqlx::Error::PoolClosed)
        };

        match result {
            Ok(Some(row)) => Ok(self.row_to_response(row)),
            Ok(None) => Err(BenchmarkError::NotFound(format!(
                "benchmark job {job_id} not found"
            ))),
            Err(e) => Err(BenchmarkError::Internal(format!("database error: {}", e))),
        }
    }

    fn get_report(&self, report_id: &str) -> Result<BenchmarkReport, BenchmarkError> {
        let report_id_str = report_id.to_string();

        let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(async {
                // Query reports using a raw SQL query and manually convert results
                let row = sqlx::query(
                    r#"
                    SELECT 
                        report_id, tenant_id, chain_name, chain_type, recommendation,
                        baseline_avg_tps, baseline_p50_latency_ms, baseline_p95_latency_ms, 
                        baseline_p99_latency_ms, baseline_failure_rate,
                        x3_avg_tps, x3_p50_latency_ms, x3_p95_latency_ms, 
                        x3_p99_latency_ms, x3_failure_rate, signer, 
                        projected_soft_confirmation_improvement, projected_app_throughput_improvement,
                        projected_route_latency_delta, projected_bridge_latency_delta,
                        workload_profile, artifacts
                    FROM benchmark_reports
                    WHERE report_id = $1
                    "#
                )
                .bind(&report_id_str)
                .fetch_optional(&self.pool)
                .await?;

                Ok(row.map(|r| {
                    let report_id: String = r.get(0);
                    let _tenant_id: String = r.get(1);
                    let chain_name: String = r.get(2);
                    let chain_type_str: String = r.get(3);
                    let recommendation_str: String = r.get(4);
                    let baseline_tps: f64 = r.get(5);
                    let baseline_p50: i64 = r.get(6);
                    let baseline_p95: i64 = r.get(7);
                    let baseline_p99: i64 = r.get(8);
                    let baseline_failure: f64 = r.get(9);
                    let x3_tps: f64 = r.get(10);
                    let x3_p50: i64 = r.get(11);
                    let x3_p95: i64 = r.get(12);
                    let x3_p99: i64 = r.get(13);
                    let x3_failure: f64 = r.get(14);
                    let signer: String = r.get(15);
                    let summary_soft: String = r.get(16);
                    let summary_throughput: String = r.get(17);
                    let summary_latency_route: String = r.get(18);
                    let summary_latency_bridge: String = r.get(19);
                    let workload_profile_json: serde_json::Value = r.get(20);
                    let artifacts_json: serde_json::Value = r.get(21);

                    (report_id, chain_name, chain_type_str, recommendation_str,
                     baseline_tps, baseline_p50, baseline_p95, baseline_p99, baseline_failure,
                     x3_tps, x3_p50, x3_p95, x3_p99, x3_failure, signer,
                     summary_soft, summary_throughput, summary_latency_route, summary_latency_bridge,
                     workload_profile_json, artifacts_json)
                }))
            })
        } else {
            Err(sqlx::Error::PoolClosed)
        };

        match result {
            Ok(Some((
                report_id,
                chain_name,
                chain_type_str,
                recommendation_str,
                baseline_tps,
                baseline_p50,
                baseline_p95,
                baseline_p99,
                baseline_failure,
                x3_tps,
                x3_p50,
                x3_p95,
                x3_p99,
                x3_failure,
                signer,
                summary_soft,
                summary_throughput,
                summary_latency_route,
                summary_latency_bridge,
                workload_profile_json,
                artifacts_json,
            ))) => {
                let chain_type = Self::string_to_chain_type(&chain_type_str);
                let recommendation = Self::string_to_recommendation(&recommendation_str);

                let workload_profile =
                    serde_json::from_value::<BenchmarkWorkloadProfile>(workload_profile_json)
                        .map_err(|e| {
                            BenchmarkError::Internal(format!("invalid workload profile: {}", e))
                        })?;

                let artifacts =
                    serde_json::from_value::<Vec<BenchmarkReportArtifact>>(artifacts_json)
                        .map_err(|e| {
                            BenchmarkError::Internal(format!("invalid artifacts: {}", e))
                        })?;

                Ok(BenchmarkReport {
                    report_id,
                    generated_at_unix: 0, // Will be set from database timestamp if needed
                    profile: BenchmarkProfile::Standard,
                    chain_name,
                    chain_type,
                    baseline: BenchmarkMetrics {
                        avg_tps: baseline_tps,
                        p50_latency_ms: baseline_p50 as u64,
                        p95_latency_ms: baseline_p95 as u64,
                        p99_latency_ms: baseline_p99 as u64,
                        failure_rate: baseline_failure,
                    },
                    x3_replay: BenchmarkMetrics {
                        avg_tps: x3_tps,
                        p50_latency_ms: x3_p50 as u64,
                        p95_latency_ms: x3_p95 as u64,
                        p99_latency_ms: x3_p99 as u64,
                        failure_rate: x3_failure,
                    },
                    recommendation,
                    summary: BenchmarkReportSummary {
                        projected_soft_confirmation_improvement: summary_soft,
                        projected_app_throughput_improvement: summary_throughput,
                        projected_route_latency_delta: summary_latency_route,
                        projected_bridge_latency_delta: summary_latency_bridge,
                    },
                    workload_profile,
                    artifacts,
                    signer,
                })
            }
            Ok(None) => Err(BenchmarkError::NotFound(format!(
                "benchmark report {report_id} not found"
            ))),
            Err(e) => Err(BenchmarkError::Internal(format!("database error: {}", e))),
        }
    }

    fn list_jobs(
        &self,
        tenant_id: Option<String>,
    ) -> Result<Vec<BenchmarkJobResponse>, BenchmarkError> {
        let result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(async {
                if let Some(tid) = tenant_id {
                    sqlx::query_as::<_, (String, String, Option<String>, i64, i64, Option<String>)>(
                        r#"
                        SELECT job_id, status, report_id, submitted_at_unix, updated_at_unix, error_message
                        FROM benchmark_jobs
                        WHERE tenant_id = $1
                        ORDER BY updated_at_unix DESC
                        "#
                    )
                    .bind(&tid)
                    .fetch_all(&self.pool)
                    .await
                } else {
                    sqlx::query_as::<_, (String, String, Option<String>, i64, i64, Option<String>)>(
                        r#"
                        SELECT job_id, status, report_id, submitted_at_unix, updated_at_unix, error_message
                        FROM benchmark_jobs
                        ORDER BY updated_at_unix DESC
                        "#
                    )
                    .fetch_all(&self.pool)
                    .await
                }
            })
        } else {
            Err(sqlx::Error::PoolClosed)
        };

        match result {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|row| self.row_to_response(row))
                .collect()),
            Err(e) => Err(BenchmarkError::Internal(format!("database error: {}", e))),
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[rpc]
pub trait BenchmarkRpcApi {
    #[rpc(name = "x3_benchmarkSubmitJob")]
    fn benchmark_submit_job(
        &self,
        request: BenchmarkJobRequest,
    ) -> JsonRpcResult<BenchmarkJobResponse>;

    #[rpc(name = "x3_benchmarkGetJob")]
    fn benchmark_get_job(&self, job_id: String) -> JsonRpcResult<BenchmarkJobResponse>;

    #[rpc(name = "x3_benchmarkGetReport")]
    fn benchmark_get_report(&self, report_id: String) -> JsonRpcResult<BenchmarkReport>;

    #[rpc(name = "x3_benchmarkListJobs")]
    fn benchmark_list_jobs(
        &self,
        tenant_id: Option<String>,
    ) -> JsonRpcResult<Vec<BenchmarkJobResponse>>;
}

pub struct X3BenchmarkRpc<S> {
    service: Arc<S>,
}

impl<S> X3BenchmarkRpc<S> {
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }
}

impl<S> BenchmarkRpcApi for X3BenchmarkRpc<S>
where
    S: BenchmarkService,
{
    fn benchmark_submit_job(
        &self,
        request: BenchmarkJobRequest,
    ) -> JsonRpcResult<BenchmarkJobResponse> {
        self.service.submit_job(request).map_err(Into::into)
    }

    fn benchmark_get_job(&self, job_id: String) -> JsonRpcResult<BenchmarkJobResponse> {
        self.service.get_job(&job_id).map_err(Into::into)
    }

    fn benchmark_get_report(&self, report_id: String) -> JsonRpcResult<BenchmarkReport> {
        self.service.get_report(&report_id).map_err(Into::into)
    }

    fn benchmark_list_jobs(
        &self,
        tenant_id: Option<String>,
    ) -> JsonRpcResult<Vec<BenchmarkJobResponse>> {
        self.service.list_jobs(tenant_id).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> BenchmarkJobRequest {
        BenchmarkJobRequest {
            tenant_id: "tenant-1".to_string(),
            profile: BenchmarkProfile::Standard,
            chain_name: "PartnerChain".to_string(),
            chain_type: BenchmarkChainType::Evm,
            rpc_endpoints: vec!["https://rpc.example".to_string()],
            explorer_endpoint: Some("https://explorer.example".to_string()),
            workload_trace_uri: Some("s3://bucket/trace.json".to_string()),
            onboarding_metadata: None,
            date_range_start_unix: 1,
            date_range_end_unix: 2,
        }
    }

    #[test]
    fn benchmark_request_validation_rejects_empty_tenant() {
        let mut request = sample_request();
        request.tenant_id.clear();

        let result = DatabaseBenchmarkService::validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("tenant_id"));
    }

    #[test]
    fn benchmark_request_validation_rejects_empty_chain_name() {
        let mut request = sample_request();
        request.chain_name.clear();

        let result = DatabaseBenchmarkService::validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("chain_name"));
    }

    #[test]
    fn benchmark_request_validation_rejects_empty_rpc_endpoints() {
        let mut request = sample_request();
        request.rpc_endpoints.clear();

        let result = DatabaseBenchmarkService::validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rpc_endpoints"));
    }

    #[test]
    fn benchmark_request_validation_rejects_invalid_date_range() {
        let mut request = sample_request();
        request.date_range_start_unix = 100;
        request.date_range_end_unix = 50;

        let result = DatabaseBenchmarkService::validate_request(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("date range"));
    }

    #[test]
    fn benchmark_chain_type_serialization_roundtrip() {
        let types = vec![
            BenchmarkChainType::Evm,
            BenchmarkChainType::OpStack,
            BenchmarkChainType::Substrate,
            BenchmarkChainType::Cosmos,
            BenchmarkChainType::Svm,
            BenchmarkChainType::Custom,
        ];

        for chain_type in types {
            let serialized = DatabaseBenchmarkService::chain_type_to_string(&chain_type);
            let deserialized = DatabaseBenchmarkService::string_to_chain_type(&serialized);
            assert_eq!(chain_type, deserialized);
        }
    }

    #[test]
    fn benchmark_recommendation_serialization_roundtrip() {
        let tiers = vec![
            BenchmarkIntegrationTier::BenchmarkOnly,
            BenchmarkIntegrationTier::SidecarMode,
            BenchmarkIntegrationTier::TurboLaneMode,
            BenchmarkIntegrationTier::SharedSettlementMode,
        ];

        for tier in tiers {
            let serialized = DatabaseBenchmarkService::recommendation_to_string(&tier);
            let deserialized = DatabaseBenchmarkService::string_to_recommendation(&serialized);
            assert_eq!(tier, deserialized);
        }
    }

    #[test]
    fn benchmark_job_status_display() {
        assert_eq!(BenchmarkJobStatus::Queued.to_string(), "queued");
        assert_eq!(BenchmarkJobStatus::Running.to_string(), "running");
        assert_eq!(BenchmarkJobStatus::Completed.to_string(), "completed");
        assert_eq!(BenchmarkJobStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn benchmark_job_status_parse() {
        assert_eq!(
            "queued".parse::<BenchmarkJobStatus>().unwrap(),
            BenchmarkJobStatus::Queued
        );
        assert_eq!(
            "running".parse::<BenchmarkJobStatus>().unwrap(),
            BenchmarkJobStatus::Running
        );
        assert_eq!(
            "completed".parse::<BenchmarkJobStatus>().unwrap(),
            BenchmarkJobStatus::Completed
        );
        assert_eq!(
            "failed".parse::<BenchmarkJobStatus>().unwrap(),
            BenchmarkJobStatus::Failed
        );
    }
}
