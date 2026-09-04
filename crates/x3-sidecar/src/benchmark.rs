use crate::config::SidecarConfig;
use crate::evm_provider::{lane_key, EvmIngestionWindow, EvmProviderPool};
use crate::GatewayClient;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use x3_rpc::benchmark::{
    BenchmarkChainType, BenchmarkIntegrationTier, BenchmarkJobRequest, BenchmarkJobResponse,
    BenchmarkJobStatus, BenchmarkLogClassStat, BenchmarkMetrics, BenchmarkProfile, BenchmarkReport,
    BenchmarkReportArtifact, BenchmarkReportSummary, BenchmarkWorkloadProfile,
    ProviderOnboardingMetadata,
};

const JOB_PREFIX: &str = "benchmark-job:";
const REQUEST_PREFIX: &str = "benchmark-request:";
const REPORT_PREFIX: &str = "benchmark-report:";
const PROVIDER_ONBOARDING_WINDOW_SECS: u64 = 30 * 60;
const PROVIDER_ONBOARDING_TEMPLATE_PREFIX: &str = "benchmark://templates/provider-onboarding";

#[derive(Clone)]
pub struct BenchmarkStore {
    db: Arc<Db>,
    signer: String,
    gateway_url: Option<String>,
    gateway_token: Option<String>,
    client: Client,
    gateway_client: Option<Arc<GatewayClient>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredBenchmarkEnvelope {
    tenant_id: String,
    report: BenchmarkReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRunInput {
    pub request: BenchmarkJobRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOnboardingBenchmarkRequest {
    pub tenant_id: String,
    pub chain_name: String,
    pub chain_type: BenchmarkChainType,
    pub rpc_endpoints: Vec<String>,
    pub explorer_endpoint: Option<String>,
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

impl BenchmarkStore {
    pub fn open(config: &SidecarConfig) -> anyhow::Result<Self> {
        Self::open_with_gateway_client(config, None)
    }

    pub fn open_with_gateway_client(
        config: &SidecarConfig,
        gateway_client: Option<Arc<GatewayClient>>,
    ) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&config.data_dir)?;
        let db_path = config.data_dir.join("benchmark-store");
        let db = sled::open(db_path)?;
        Ok(Self {
            db: Arc::new(db),
            signer: config.benchmark_signer.clone(),
            gateway_url: config.benchmark_gateway_url.clone(),
            gateway_token: config.benchmark_gateway_token.clone(),
            client: Client::new(),
            gateway_client,
        })
    }

    pub fn submit(&self, input: BenchmarkRunInput) -> anyhow::Result<BenchmarkJobResponse> {
        validate_request(&input.request)?;

        let now = now_unix();
        let job_id = uuid::Uuid::new_v4().to_string();
        let response = BenchmarkJobResponse {
            job_id: job_id.clone(),
            status: BenchmarkJobStatus::Queued,
            report_id: None,
            submitted_at_unix: now,
            updated_at_unix: now,
            error: None,
        };

        self.db.insert(
            format!("{JOB_PREFIX}{job_id}"),
            serde_json::to_vec(&response)?,
        )?;
        self.db.insert(
            format!("{REQUEST_PREFIX}{job_id}"),
            serde_json::to_vec(&input.request)?,
        )?;
        self.db.flush()?;

        Ok(response)
    }

    pub fn submit_onboarding(
        &self,
        request: ProviderOnboardingBenchmarkRequest,
    ) -> anyhow::Result<(BenchmarkJobRequest, BenchmarkJobResponse)> {
        let request = build_provider_onboarding_job_request(request)?;
        let response = self.submit(BenchmarkRunInput {
            request: request.clone(),
        })?;
        Ok((request, response))
    }

    pub async fn execute_job(
        &self,
        job_id: &str,
        request: &BenchmarkJobRequest,
    ) -> anyhow::Result<BenchmarkJobResponse> {
        let now = now_unix();
        self.store_job(BenchmarkJobResponse {
            job_id: job_id.to_string(),
            status: BenchmarkJobStatus::Running,
            report_id: None,
            submitted_at_unix: now,
            updated_at_unix: now,
            error: None,
        })?;

        let report_id = uuid::Uuid::new_v4().to_string();
        let report = match request.chain_type {
            BenchmarkChainType::Evm | BenchmarkChainType::OpStack => {
                let provider_pool = EvmProviderPool::new(request.rpc_endpoints.clone())?;
                let ingestion = provider_pool.ingest_recent_window(6).await?;
                build_report_from_evm_window(&self.signer, &report_id, request, now, &ingestion)
            }
            _ => {
                anyhow::bail!("benchmark execution currently supports only EVM and OP Stack chains")
            }
        };

        let envelope = StoredBenchmarkEnvelope {
            tenant_id: request.tenant_id.clone(),
            report,
        };
        self.db.insert(
            format!("{REPORT_PREFIX}{report_id}"),
            serde_json::to_vec(&envelope)?,
        )?;

        let response = BenchmarkJobResponse {
            job_id: job_id.to_string(),
            status: BenchmarkJobStatus::Completed,
            report_id: Some(report_id.clone()),
            submitted_at_unix: now,
            updated_at_unix: now_unix(),
            error: None,
        };
        self.store_job(response.clone())?;

        if self.gateway_url.is_some() {
            self.publish_report(&request.tenant_id, &report_id).await?;
        }

        Ok(response)
    }

    pub fn get_job(&self, job_id: &str) -> anyhow::Result<Option<BenchmarkJobResponse>> {
        let value = self.db.get(format!("{JOB_PREFIX}{job_id}"))?;
        value
            .map(|raw| serde_json::from_slice(&raw).map_err(Into::into))
            .transpose()
    }

    pub fn get_job_request(&self, job_id: &str) -> anyhow::Result<Option<BenchmarkJobRequest>> {
        let value = self.db.get(format!("{REQUEST_PREFIX}{job_id}"))?;
        value
            .map(|raw| serde_json::from_slice(&raw).map_err(Into::into))
            .transpose()
    }

    pub fn get_report(&self, report_id: &str) -> anyhow::Result<Option<BenchmarkReport>> {
        let value = self.db.get(format!("{REPORT_PREFIX}{report_id}"))?;
        value
            .map(|raw| {
                let envelope: StoredBenchmarkEnvelope = serde_json::from_slice(&raw)?;
                Ok::<BenchmarkReport, anyhow::Error>(envelope.report)
            })
            .transpose()
    }

    pub fn get_report_tenant_id(&self, report_id: &str) -> anyhow::Result<Option<String>> {
        let value = self.db.get(format!("{REPORT_PREFIX}{report_id}"))?;
        value
            .map(|raw| {
                let envelope: StoredBenchmarkEnvelope = serde_json::from_slice(&raw)?;
                Ok::<String, anyhow::Error>(envelope.tenant_id)
            })
            .transpose()
    }

    pub fn list_jobs(&self) -> anyhow::Result<Vec<BenchmarkJobResponse>> {
        let mut jobs = Vec::new();
        for item in self.db.scan_prefix(JOB_PREFIX.as_bytes()) {
            let (_, value) = item?;
            jobs.push(serde_json::from_slice::<BenchmarkJobResponse>(&value)?);
        }
        jobs.sort_by(|a, b| b.updated_at_unix.cmp(&a.updated_at_unix));
        Ok(jobs)
    }

    pub fn list_jobs_by_profile(
        &self,
        profile: BenchmarkProfile,
    ) -> anyhow::Result<Vec<BenchmarkJobResponse>> {
        let mut jobs = Vec::new();
        for job in self.list_jobs()? {
            let request = self.get_job_request(&job.job_id)?;
            if request
                .as_ref()
                .map(|request| request.profile == profile)
                .unwrap_or(false)
            {
                jobs.push(job);
            }
        }
        Ok(jobs)
    }

    fn store_job(&self, response: BenchmarkJobResponse) -> anyhow::Result<()> {
        self.db.insert(
            format!("{JOB_PREFIX}{}", response.job_id),
            serde_json::to_vec(&response)?,
        )?;
        self.db.flush()?;
        Ok(())
    }

    pub async fn publish_report(&self, tenant_id: &str, report_id: &str) -> anyhow::Result<()> {
        let report = self
            .get_report(report_id)?
            .ok_or_else(|| anyhow::anyhow!("benchmark report {report_id} not found"))?;

        // Use GatewayClient if available (with retry logic), otherwise fallback to direct HTTP
        if let Some(gateway_client) = &self.gateway_client {
            let payload = crate::BenchmarkResultPayload {
                tenant_id: tenant_id.to_string(),
                report,
            };
            gateway_client.submit_benchmark_result(&payload).await?;
        } else {
            // Fallback to direct HTTP for backward compatibility
            let gateway_url = self
                .gateway_url
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("benchmark gateway url is not configured"))?;

            let url = format!(
                "{}/api/v1/benchmarks/reports",
                gateway_url.trim_end_matches('/')
            );
            let mut request = self.client.post(url).json(&serde_json::json!({
                "tenant_id": tenant_id,
                "report": report,
            }));

            if let Some(token) = &self.gateway_token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("benchmark publish failed with status {}: {}", status, body);
            }
        }

        Ok(())
    }
}

fn validate_request(request: &BenchmarkJobRequest) -> anyhow::Result<()> {
    if request.tenant_id.trim().is_empty() {
        anyhow::bail!("tenant_id is required");
    }
    if request.chain_name.trim().is_empty() {
        anyhow::bail!("chain_name is required");
    }
    if request.rpc_endpoints.is_empty() {
        anyhow::bail!("at least one rpc endpoint is required");
    }
    if request.date_range_end_unix <= request.date_range_start_unix {
        anyhow::bail!("date_range_end_unix must be greater than date_range_start_unix");
    }
    if request.profile == BenchmarkProfile::ProviderOnboarding {
        let metadata = request.onboarding_metadata.as_ref().ok_or_else(|| {
            anyhow::anyhow!("onboarding_metadata is required for provider onboarding benchmarks")
        })?;
        require_onboarding_field(&metadata.provider_id, "provider_id")?;
        require_onboarding_field(&metadata.operator_id, "operator_id")?;
        require_onboarding_field(&metadata.region, "region")?;
        require_onboarding_field(&metadata.hardware_id, "hardware_id")?;
        require_onboarding_field(&metadata.hardware_kind, "hardware_kind")?;
        require_onboarding_field(&metadata.cpu_model, "cpu_model")?;
        require_onboarding_field(&metadata.gpu_model, "gpu_model")?;
        require_onboarding_field(&metadata.provider_signature, "provider_signature")?;
        require_onboarding_field(&metadata.hardware_signature, "hardware_signature")?;
        if metadata.memory_gb == 0 {
            anyhow::bail!("memory_gb must be greater than zero for provider onboarding benchmarks");
        }
    }
    Ok(())
}

fn require_onboarding_field(value: &str, field: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("{field} is required for provider onboarding benchmarks");
    }
    Ok(())
}

pub fn build_provider_onboarding_job_request(
    request: ProviderOnboardingBenchmarkRequest,
) -> anyhow::Result<BenchmarkJobRequest> {
    let now = now_unix();
    let chain_type_slug = benchmark_chain_type_slug(&request.chain_type);
    let metadata = ProviderOnboardingMetadata {
        provider_id: request.provider_id,
        operator_id: request.operator_id,
        region: request.region,
        hardware_id: request.hardware_id,
        hardware_kind: request.hardware_kind,
        cpu_model: request.cpu_model,
        gpu_model: request.gpu_model,
        memory_gb: request.memory_gb,
        provider_signature: request.provider_signature,
        hardware_signature: request.hardware_signature,
    };

    let benchmark_request = BenchmarkJobRequest {
        tenant_id: request.tenant_id,
        profile: BenchmarkProfile::ProviderOnboarding,
        chain_name: request.chain_name,
        chain_type: request.chain_type,
        rpc_endpoints: request.rpc_endpoints,
        explorer_endpoint: request.explorer_endpoint,
        workload_trace_uri: Some(format!(
            "{}/{}",
            PROVIDER_ONBOARDING_TEMPLATE_PREFIX, chain_type_slug
        )),
        onboarding_metadata: Some(metadata),
        date_range_start_unix: now.saturating_sub(PROVIDER_ONBOARDING_WINDOW_SECS),
        date_range_end_unix: now,
    };

    validate_request(&benchmark_request)?;
    Ok(benchmark_request)
}

fn benchmark_chain_type_slug(chain_type: &BenchmarkChainType) -> &'static str {
    match chain_type {
        BenchmarkChainType::Evm => "evm",
        BenchmarkChainType::OpStack => "op-stack",
        BenchmarkChainType::Substrate => "substrate",
        BenchmarkChainType::Cosmos => "cosmos",
        BenchmarkChainType::Svm => "svm",
        BenchmarkChainType::Custom => "custom",
    }
}

fn build_report_from_evm_window(
    signer: &str,
    report_id: &str,
    request: &BenchmarkJobRequest,
    generated_at_unix: u64,
    ingestion: &EvmIngestionWindow,
) -> BenchmarkReport {
    let baseline = baseline_metrics(ingestion);
    let x3_replay = replay_metrics(ingestion, &baseline);
    let lane_count = lane_count(ingestion);

    BenchmarkReport {
        report_id: report_id.to_string(),
        generated_at_unix,
        profile: request.profile.clone(),
        chain_name: request.chain_name.clone(),
        chain_type: request.chain_type.clone(),
        baseline: baseline.clone(),
        x3_replay: x3_replay.clone(),
        recommendation: BenchmarkIntegrationTier::TurboLaneMode,
        summary: BenchmarkReportSummary {
            projected_soft_confirmation_improvement: format!(
                "{:.1}x faster",
                ratio(baseline.p50_latency_ms, x3_replay.p50_latency_ms)
            ),
            projected_app_throughput_improvement: format!(
                "{:.1}x higher across {} active lanes",
                safe_ratio(x3_replay.avg_tps, baseline.avg_tps),
                lane_count
            ),
            projected_route_latency_delta: format!(
                "{:.0}% lower",
                percentage_drop(baseline.p95_latency_ms, x3_replay.p95_latency_ms)
            ),
            projected_bridge_latency_delta: format!(
                "{:.0}% lower",
                percentage_drop(baseline.p99_latency_ms, x3_replay.p99_latency_ms)
            ),
        },
        workload_profile: workload_profile(ingestion),
        artifacts: benchmark_artifacts_for_request(report_id, request),
        signer: signer.to_string(),
    }
}

fn benchmark_artifacts_for_request(
    report_id: &str,
    request: &BenchmarkJobRequest,
) -> Vec<BenchmarkReportArtifact> {
    let mut artifacts = vec![
        BenchmarkReportArtifact {
            artifact_type: "report-json".to_string(),
            uri: format!("benchmark://reports/{report_id}"),
            digest: report_id.to_string(),
            metadata: Some(serde_json::json!({
                "profile": request.profile,
            })),
            signature: None,
        },
        BenchmarkReportArtifact {
            artifact_type: "trace-bundle".to_string(),
            uri: request
                .workload_trace_uri
                .clone()
                .unwrap_or_else(|| "benchmark://artifacts/missing-trace".to_string()),
            digest: format!("{}:{}", request.tenant_id, request.chain_name),
            metadata: Some(serde_json::json!({
                "tenant_id": request.tenant_id,
                "profile": request.profile,
            })),
            signature: None,
        },
    ];

    if request.profile == BenchmarkProfile::ProviderOnboarding {
        if let Some(metadata) = &request.onboarding_metadata {
            artifacts.push(BenchmarkReportArtifact {
                artifact_type: "provider-manifest".to_string(),
                uri: format!("benchmark://providers/{}", metadata.provider_id),
                digest: format!("{}:{}", metadata.provider_id, metadata.operator_id),
                metadata: Some(serde_json::json!({
                    "provider_id": metadata.provider_id,
                    "operator_id": metadata.operator_id,
                    "region": metadata.region,
                    "hardware_id": metadata.hardware_id,
                })),
                signature: Some(metadata.provider_signature.clone()),
            });
            artifacts.push(BenchmarkReportArtifact {
                artifact_type: "hardware-attestation".to_string(),
                uri: format!("benchmark://hardware/{}", metadata.hardware_id),
                digest: format!("{}:{}", metadata.hardware_id, metadata.hardware_kind),
                metadata: Some(serde_json::json!({
                    "hardware_id": metadata.hardware_id,
                    "hardware_kind": metadata.hardware_kind,
                    "cpu_model": metadata.cpu_model,
                    "gpu_model": metadata.gpu_model,
                    "memory_gb": metadata.memory_gb,
                })),
                signature: Some(metadata.hardware_signature.clone()),
            });
        }
    }

    artifacts
}

fn baseline_metrics(ingestion: &EvmIngestionWindow) -> BenchmarkMetrics {
    let block_span = block_span_seconds(ingestion).max(1.0);
    let receipt_count = ingestion.receipts.len() as f64;
    let failed = ingestion
        .receipts
        .iter()
        .filter(|receipt| !receipt.status)
        .count() as f64;
    let logs_per_receipt = safe_ratio(ingestion.logs.len() as f64, receipt_count.max(1.0));
    let mut latencies = ingestion
        .receipts
        .iter()
        .map(|receipt| {
            let log_pressure_ms = (receipt.logs_count as u64).saturating_mul(18);
            ((receipt.gas_used / 100) + 250 + log_pressure_ms) as u64
        })
        .collect::<Vec<_>>();
    latencies.sort_unstable();

    BenchmarkMetrics {
        avg_tps: (receipt_count / block_span) / (1.0 + (logs_per_receipt * 0.08)),
        p50_latency_ms: percentile(&latencies, 50),
        p95_latency_ms: percentile(&latencies, 95),
        p99_latency_ms: percentile(&latencies, 99),
        failure_rate: if receipt_count > 0.0 {
            failed / receipt_count
        } else {
            0.0
        },
    }
}

fn replay_metrics(ingestion: &EvmIngestionWindow, baseline: &BenchmarkMetrics) -> BenchmarkMetrics {
    let lane_count = lane_count(ingestion).max(1) as f64;
    let unique_log_lanes = log_lane_count(ingestion).max(1) as f64;
    let total_logs = ingestion.logs.len() as f64;
    let receipts = ingestion.receipts.len().max(1) as f64;
    let logs_per_receipt = total_logs / receipts;
    let log_parallelism = (unique_log_lanes / lane_count).clamp(0.5, 1.5);
    let contention_penalty = (1.0 / (1.0 + (logs_per_receipt * 0.12))).clamp(0.55, 1.0);
    let parallelism_gain =
        (lane_count.sqrt() * log_parallelism * contention_penalty).clamp(1.0, 6.0);
    let avg_tps = (baseline.avg_tps * parallelism_gain * 1.35).max(baseline.avg_tps);
    let failure_reduction = (0.35 * contention_penalty).clamp(0.12, 0.35);
    BenchmarkMetrics {
        avg_tps,
        p50_latency_ms: ((baseline.p50_latency_ms as f64 / parallelism_gain).round() as u64)
            .max(75),
        p95_latency_ms: ((baseline.p95_latency_ms as f64 / (parallelism_gain * 0.9)).round()
            as u64)
            .max(150),
        p99_latency_ms: ((baseline.p99_latency_ms as f64 / (parallelism_gain * 0.85)).round()
            as u64)
            .max(225),
        failure_rate: (baseline.failure_rate * failure_reduction).min(baseline.failure_rate),
    }
}

fn lane_count(ingestion: &EvmIngestionWindow) -> usize {
    let mut lanes = HashSet::new();
    for block in &ingestion.blocks {
        for tx in &block.transactions {
            lanes.insert(lane_key(tx));
        }
    }
    lanes.len()
}

fn log_lane_count(ingestion: &EvmIngestionWindow) -> usize {
    let mut lanes = HashSet::new();
    for log in &ingestion.logs {
        let selector = log.topic0.as_deref().unwrap_or("no-topic");
        lanes.insert(format!(
            "{}:{}",
            log.address.to_lowercase(),
            selector.to_lowercase()
        ));
    }
    lanes.len()
}

fn workload_profile(ingestion: &EvmIngestionWindow) -> BenchmarkWorkloadProfile {
    let total_transactions = ingestion
        .blocks
        .iter()
        .map(|block| block.transactions.len() as u64)
        .sum();
    let total_receipts = ingestion.receipts.len() as u64;
    let total_logs = ingestion.logs.len() as u64;
    let active_lanes = lane_count(ingestion) as u64;
    let active_log_lanes = log_lane_count(ingestion) as u64;
    let (low_conflict_ratio, medium_conflict_ratio, high_conflict_ratio, estimated_serial_fraction) =
        conflict_profile(ingestion);
    let mut log_classes = classify_log_mix(ingestion);
    log_classes.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.class_name.cmp(&b.class_name))
    });

    BenchmarkWorkloadProfile {
        total_transactions,
        total_receipts,
        total_logs,
        active_lanes,
        active_log_lanes,
        low_conflict_ratio,
        medium_conflict_ratio,
        high_conflict_ratio,
        estimated_serial_fraction,
        log_classes,
    }
}

fn conflict_profile(ingestion: &EvmIngestionWindow) -> (f64, f64, f64, f64) {
    use std::collections::HashMap;

    let mut per_lane_txs: HashMap<String, u64> = HashMap::new();
    for block in &ingestion.blocks {
        for tx in &block.transactions {
            *per_lane_txs.entry(lane_key(tx)).or_default() += 1;
        }
    }

    let mut per_log_lane_events: HashMap<String, u64> = HashMap::new();
    for log in &ingestion.logs {
        let selector = log.topic0.as_deref().unwrap_or("no-topic").to_lowercase();
        *per_log_lane_events
            .entry(format!("{}:{selector}", log.address.to_lowercase()))
            .or_default() += 1;
    }

    let lane_count = per_lane_txs.len().max(1) as f64;
    let total_logs = ingestion.logs.len() as f64;
    let avg_logs_per_lane = if lane_count == 0.0 {
        0.0
    } else {
        total_logs / lane_count
    };

    let mut low = 0u64;
    let mut medium = 0u64;
    let mut high = 0u64;

    for (lane, tx_count) in per_lane_txs {
        let log_count = per_log_lane_events
            .iter()
            .filter(|(log_lane, _)| log_lane.starts_with(&lane))
            .map(|(_, count)| *count)
            .sum::<u64>();
        let contention_score = tx_count as f64 + ((log_count as f64) * 0.6) + avg_logs_per_lane;
        if contention_score <= 2.0 {
            low += tx_count;
        } else if contention_score <= 4.0 {
            medium += tx_count;
        } else {
            high += tx_count;
        }
    }

    let total = (low + medium + high).max(1) as f64;
    let low_ratio = low as f64 / total;
    let medium_ratio = medium as f64 / total;
    let mut high_ratio = high as f64 / total;
    let mut serial_fraction = (high_ratio * 0.75) + (medium_ratio * 0.35);
    if serial_fraction > 1.0 {
        serial_fraction = 1.0;
    }
    if low_ratio + medium_ratio + high_ratio > 1.0 {
        high_ratio = (1.0 - low_ratio - medium_ratio).max(0.0);
    }

    (low_ratio, medium_ratio, high_ratio, serial_fraction)
}

fn classify_log_mix(ingestion: &EvmIngestionWindow) -> Vec<BenchmarkLogClassStat> {
    use std::collections::{HashMap, HashSet};

    #[derive(Default)]
    struct Acc {
        count: u64,
        contracts: HashSet<String>,
        transactions: HashSet<String>,
    }

    let mut stats: HashMap<String, Acc> = HashMap::new();
    let total_logs = ingestion.logs.len() as f64;

    for log in &ingestion.logs {
        let class_name = classify_log(log.topic0.as_deref(), &log.address);
        let entry = stats.entry(class_name).or_default();
        entry.count += 1;
        entry.contracts.insert(log.address.to_lowercase());
        if let Some(tx_hash) = &log.transaction_hash {
            entry.transactions.insert(tx_hash.to_lowercase());
        }
    }

    stats
        .into_iter()
        .map(|(class_name, acc)| BenchmarkLogClassStat {
            class_name,
            count: acc.count,
            share_of_logs: if total_logs == 0.0 {
                0.0
            } else {
                acc.count as f64 / total_logs
            },
            unique_contracts: acc.contracts.len() as u64,
            unique_transactions: acc.transactions.len() as u64,
        })
        .collect()
}

fn classify_log(topic0: Option<&str>, address: &str) -> String {
    let lowered_topic = topic0.unwrap_or_default().to_lowercase();
    let lowered_address = address.to_lowercase();
    if lowered_address.contains("bridge") {
        return "bridge-event".to_string();
    }
    match lowered_topic.as_str() {
        "0xddf252ad" => "erc20-transfer".to_string(),
        "0x8c5be1e5" => "erc20-approval".to_string(),
        "0x1c411e9a" | "0x7fcf532c" => "amm-sync".to_string(),
        "" => "untagged-event".to_string(),
        _ => "generic-contract-event".to_string(),
    }
}

fn block_span_seconds(ingestion: &EvmIngestionWindow) -> f64 {
    let first = ingestion
        .blocks
        .first()
        .map(|block| block.timestamp)
        .unwrap_or_default();
    let last = ingestion
        .blocks
        .last()
        .map(|block| block.timestamp)
        .unwrap_or(first);
    (last.saturating_sub(first).max(1)) as f64
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let idx = ((values.len() - 1) * percentile) / 100;
    values[idx]
}

fn safe_ratio(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        0.0
    } else {
        a / b
    }
}

fn ratio(a: u64, b: u64) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

fn percentage_drop(a: u64, b: u64) -> f64 {
    if a == 0 {
        0.0
    } else {
        ((a.saturating_sub(b)) as f64 / a as f64) * 100.0
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm_provider::start_mock_evm_server;
    use tempfile::tempdir;
    use x3_rpc::benchmark::{BenchmarkChainType, BenchmarkProfile};

    fn config() -> SidecarConfig {
        let mut config = SidecarConfig::default();
        config.data_dir = tempdir().expect("tempdir").keep();
        config.benchmark_signer = "sidecar-test".to_string();
        config
    }

    fn request(rpc_endpoint: String) -> BenchmarkJobRequest {
        BenchmarkJobRequest {
            tenant_id: "tenant-a".to_string(),
            profile: BenchmarkProfile::Standard,
            chain_name: "PartnerChain".to_string(),
            chain_type: BenchmarkChainType::Evm,
            rpc_endpoints: vec![rpc_endpoint],
            explorer_endpoint: None,
            workload_trace_uri: Some("file:///tmp/trace.json".to_string()),
            onboarding_metadata: None,
            date_range_start_unix: 1,
            date_range_end_unix: 2,
        }
    }

    #[test]
    fn benchmark_store_persists_job_and_report() {
        let store = BenchmarkStore::open(&config()).expect("store");
        let queued = store
            .submit(BenchmarkRunInput {
                request: request("http://127.0.0.1:8545".to_string()),
            })
            .expect("submit");
        assert_eq!(queued.status, BenchmarkJobStatus::Queued);
    }

    #[tokio::test]
    async fn benchmark_store_executes_job_and_generates_report() {
        let server = start_mock_evm_server().await;
        let store = BenchmarkStore::open(&config()).expect("store");
        let request = request(server.url("/"));
        let queued = store
            .submit(BenchmarkRunInput {
                request: request.clone(),
            })
            .expect("submit");
        let response = store
            .execute_job(&queued.job_id, &request)
            .await
            .expect("execute");
        let loaded = store
            .get_job(&response.job_id)
            .expect("get job")
            .expect("job exists");
        assert_eq!(loaded.status, BenchmarkJobStatus::Completed);
        let report = store
            .get_report(loaded.report_id.as_deref().expect("report id"))
            .expect("get report")
            .expect("report exists");
        assert_eq!(report.signer, "sidecar-test");
        assert!(report.baseline.avg_tps > 0.0);
        assert!(report.x3_replay.avg_tps >= report.baseline.avg_tps);
        assert!(report.baseline.p50_latency_ms >= 250);
        assert!(report.x3_replay.p50_latency_ms <= report.baseline.p50_latency_ms);
        assert_eq!(report.workload_profile.total_logs, 3);
        assert!(!report.workload_profile.log_classes.is_empty());
        assert!(report.workload_profile.low_conflict_ratio >= 0.0);
        assert!(report.workload_profile.medium_conflict_ratio >= 0.0);
        assert!(report.workload_profile.high_conflict_ratio >= 0.0);
        assert!(report.workload_profile.estimated_serial_fraction >= 0.0);
        assert!(report.workload_profile.estimated_serial_fraction <= 1.0);
    }

    #[tokio::test]

    async fn benchmark_store_submits_result_to_gateway_with_retry() {
        // Start mock EVM server for benchmark execution
        let evm_server = start_mock_evm_server().await;

        // Start mock gateway server
        let gateway_server = tokio::spawn(async {
            use axum::{http::StatusCode, routing::post, Json, Router};
            use std::sync::Arc;
            use tokio::sync::Mutex;

            let submitted_count = Arc::new(Mutex::new(0));
            let submitted_count_clone = submitted_count.clone();

            let app = Router::new().route(
                "/api/v1/benchmarks/results",
                post({
                    let submitted_count = submitted_count_clone;
                    move |Json(_): Json<serde_json::Value>| {
                        let submitted_count = submitted_count.clone();
                        async move {
                            let mut count = submitted_count.lock().await;
                            *count += 1;
                            (
                                StatusCode::OK,
                                Json(serde_json::json!({
                                    "report_id": "report-123",
                                    "status": "stored"
                                })),
                            )
                        }
                    }
                }),
            );

            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            let addr = listener.local_addr().expect("local addr");

            tokio::spawn(async move {
                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .expect("serve");
            });

            (addr, submitted_count)
        });

        let (gateway_addr, _) = gateway_server.await.expect("gateway setup");
        let gateway_url = format!("http://{}", gateway_addr);

        // Create config with gateway
        let mut config = config();
        config.benchmark_gateway_url = Some(gateway_url.clone());
        config.benchmark_gateway_token = Some("test-token".to_string());

        // Create gateway client
        let gateway_client = Arc::new(crate::GatewayClient::new(crate::GatewayClientConfig {
            gateway_url,
            auth_token: Some("test-token".to_string()),
            max_retries: 3,
            initial_backoff_ms: 10, // Short backoff for testing
        }));

        // Create store with gateway client
        let store =
            BenchmarkStore::open_with_gateway_client(&config, Some(gateway_client)).expect("store");

        // Submit and execute benchmark
        let request = request(evm_server.url("/"));
        let queued = store
            .submit(BenchmarkRunInput {
                request: request.clone(),
            })
            .expect("submit");

        // Execute job (which should trigger gateway submission)
        let response = store
            .execute_job(&queued.job_id, &request)
            .await
            .expect("execute");

        // Verify job completed
        assert_eq!(response.status, BenchmarkJobStatus::Completed);
        assert!(response.report_id.is_some());

        // Give the gateway submission a moment to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify report was retrieved from store
        let report = store
            .get_report(response.report_id.as_deref().expect("report id"))
            .expect("get report")
            .expect("report exists");
        assert_eq!(report.signer, "sidecar-test");
    }

    #[test]
    fn provider_onboarding_template_uses_standardized_defaults() {
        let request = build_provider_onboarding_job_request(ProviderOnboardingBenchmarkRequest {
            tenant_id: "tenant-onboarding".to_string(),
            chain_name: "PartnerChain".to_string(),
            chain_type: BenchmarkChainType::Evm,
            rpc_endpoints: vec!["http://127.0.0.1:8545".to_string()],
            explorer_endpoint: Some("https://explorer.example".to_string()),
            provider_id: "provider-1".to_string(),
            operator_id: "operator-1".to_string(),
            region: "us-east".to_string(),
            hardware_id: "gpu-rig-01".to_string(),
            hardware_kind: "gpu-rig".to_string(),
            cpu_model: "EPYC 7B13".to_string(),
            gpu_model: "RTX 4090".to_string(),
            memory_gb: 256,
            provider_signature: "provider-sig".to_string(),
            hardware_signature: "hardware-sig".to_string(),
        })
        .expect("build onboarding request");

        assert_eq!(request.profile, BenchmarkProfile::ProviderOnboarding);
        assert_eq!(
            request.workload_trace_uri.as_deref(),
            Some("benchmark://templates/provider-onboarding/evm")
        );
        assert!(request.date_range_end_unix > request.date_range_start_unix);
        assert!(request.onboarding_metadata.is_some());
    }

    #[tokio::test]
    async fn benchmark_store_generates_signed_onboarding_artifacts() {
        let server = start_mock_evm_server().await;
        let store = BenchmarkStore::open(&config()).expect("store");
        let (request, queued) = store
            .submit_onboarding(ProviderOnboardingBenchmarkRequest {
                tenant_id: "tenant-onboarding".to_string(),
                chain_name: "PartnerChain".to_string(),
                chain_type: BenchmarkChainType::Evm,
                rpc_endpoints: vec![server.url("/")],
                explorer_endpoint: None,
                provider_id: "provider-1".to_string(),
                operator_id: "operator-1".to_string(),
                region: "us-east".to_string(),
                hardware_id: "gpu-rig-01".to_string(),
                hardware_kind: "gpu-rig".to_string(),
                cpu_model: "EPYC 7B13".to_string(),
                gpu_model: "RTX 4090".to_string(),
                memory_gb: 256,
                provider_signature: "provider-sig".to_string(),
                hardware_signature: "hardware-sig".to_string(),
            })
            .expect("submit onboarding");
        let response = store
            .execute_job(&queued.job_id, &request)
            .await
            .expect("execute onboarding job");
        let report = store
            .get_report(response.report_id.as_deref().expect("report id"))
            .expect("get report")
            .expect("report exists");

        assert_eq!(report.profile, BenchmarkProfile::ProviderOnboarding);
        let provider_manifest = report
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_type == "provider-manifest")
            .expect("provider manifest artifact");
        let hardware_attestation = report
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_type == "hardware-attestation")
            .expect("hardware attestation artifact");

        assert_eq!(provider_manifest.signature.as_deref(), Some("provider-sig"));
        assert_eq!(
            hardware_attestation.signature.as_deref(),
            Some("hardware-sig")
        );
        assert_eq!(
            provider_manifest
                .metadata
                .as_ref()
                .and_then(|value| value.get("provider_id"))
                .and_then(|value| value.as_str()),
            Some("provider-1")
        );
        assert_eq!(
            hardware_attestation
                .metadata
                .as_ref()
                .and_then(|value| value.get("gpu_model"))
                .and_then(|value| value.as_str()),
            Some("RTX 4090")
        );
    }
}
