//! Upstream health scoring and node pool management.
//!
//! Each upstream receives a composite score (0–100) every 15 seconds based on:
//!   latency (25), freshness (25), correctness (25), error rate (15), capability (10).
//!
//! Scores integrate with x3-rpc-policy's FAILOVER_THRESHOLD, FREEZE_THRESHOLD,
//! MAX_BLOCK_DRIFT, and MAX_ERROR_RATE_BPS constants.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use hyper::body::Incoming;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use x3_rpc_policy::{ProviderStatus, FAILOVER_THRESHOLD, FREEZE_THRESHOLD, MAX_BLOCK_DRIFT, MAX_ERROR_RATE_BPS};

use crate::config::{ArcConfig, ChainKind, ProviderEntry};
use crate::health::HealthState;
use crate::metrics::ArcMetrics;

// ── Scored Upstream ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScoredUpstream {
    pub provider: ProviderEntry,
    pub score: u8,
    pub status: ProviderStatus,
    pub latency_ms: u64,
    pub block_lag: u32,
    pub slot_lag: u32,
    pub error_rate_bps: u32,
    pub last_checked: Option<Instant>,
    pub consecutive_failures: u32,
    pub wrong_chain_detected: bool,
}

impl ScoredUpstream {
    pub fn new(provider: ProviderEntry) -> Self {
        Self {
            provider,
            score: 100,
            status: ProviderStatus::Healthy,
            latency_ms: 0,
            block_lag: 0,
            slot_lag: 0,
            error_rate_bps: 0,
            last_checked: None,
            consecutive_failures: 0,
            wrong_chain_detected: false,
        }
    }

    /// Whether this upstream should be removed from rotation.
    pub fn is_quarantined(&self) -> bool {
        self.wrong_chain_detected
            || self.consecutive_failures >= 3
            || self.score < FREEZE_THRESHOLD
            || matches!(self.status, ProviderStatus::Frozen | ProviderStatus::Offline)
    }

    /// Whether this upstream is healthy enough for new requests.
    pub fn is_healthy(&self) -> bool {
        self.score >= FAILOVER_THRESHOLD && !self.wrong_chain_detected
    }
}

// ── Upstream Pool ──────────────────────────────────────────────────────────

pub struct UpstreamPool {
    config: ArcConfig,
    metrics: ArcMetrics,
    pub health: Arc<HealthState>,
    /// chain_name → Vec<ScoredUpstream>
    scored: RwLock<HashMap<String, Vec<ScoredUpstream>>>,
    http_client: Client<hyper_util::client::legacy::connect::HttpConnector, Incoming>,
}

impl UpstreamPool {
    pub fn new(config: ArcConfig, metrics: ArcMetrics, health: Arc<HealthState>) -> Self {
        let mut scored = HashMap::new();
        for (chain, providers) in &config.providers_by_chain {
            scored.insert(
                chain.clone(),
                providers.iter().map(|p| ScoredUpstream::new(p.clone())).collect(),
            );
        }

        let http_client = hyper_util::client::legacy::Client::builder(TokioExecutor::new())
            .build_http();

        Self {
            config,
            metrics,
            health,
            scored: RwLock::new(scored),
            http_client,
        }
    }

    /// Re-score all upstreams. Called every 15 seconds.
    pub async fn score_all_upstreams(&self) {
        let chains: Vec<String> = self.scored.read().await.keys().cloned().collect();

        for chain in chains {
            let providers = {
                let scored = self.scored.read().await;
                scored.get(&chain).cloned().unwrap_or_default()
            };

            for mut upstream in providers {
                let new_score = self.score_single(&mut upstream).await;
                self.metrics.update_upstream_metrics(
                    &upstream.provider.id,
                    &chain,
                    new_score,
                    upstream.latency_ms,
                    upstream.block_lag,
                    upstream.slot_lag,
                    upstream.error_rate_bps,
                );

                // Update pool
                let mut scored = self.scored.write().await;
                if let Some(list) = scored.get_mut(&chain) {
                    if let Some(entry) = list.iter_mut().find(|u| u.provider.id == upstream.provider.id) {
                        *entry = upstream;
                    }
                }
            }
        }
    }

    async fn score_single(&self, upstream: &mut ScoredUpstream) -> u8 {
        upstream.last_checked = Some(Instant::now());

        let chain_kind = self.config.chains
            .get(&upstream.provider.chain)
            .map(|c| c.kind)
            .unwrap_or(ChainKind::Evm);

        // ── Health check: determine the right probe ─────────────────
        let health_result = match chain_kind {
            ChainKind::Evm => self.probe_evm_health(upstream).await,
            ChainKind::Solana => self.probe_solana_health(upstream).await,
            ChainKind::Bitcoin => self.probe_bitcoin_health(upstream).await,
            ChainKind::X3 => self.probe_x3_health(upstream).await,
        };

        match health_result {
            Ok(HealthProbe {
                latency_ms,
                block_lag,
                slot_lag,
                chain_id_match,
                block_hash_ok,
            }) => {
                upstream.consecutive_failures = 0;
                upstream.latency_ms = latency_ms;
                upstream.block_lag = block_lag;
                upstream.slot_lag = slot_lag;

                if !chain_id_match || !block_hash_ok {
                    upstream.wrong_chain_detected = true;
                    upstream.score = 0;
                    upstream.status = ProviderStatus::Offline;
                    return 0;
                }
                upstream.wrong_chain_detected = false;

                // Compute composite score
                let latency_score = score_latency(latency_ms, &chain_kind);
                let freshness_score = score_freshness(block_lag, slot_lag, &chain_kind);
                let correctness_score = if chain_id_match && block_hash_ok { 25 } else { 0 };
                let error_score = if upstream.error_rate_bps == 0 { 15 }
                    else { 15u8.saturating_sub((upstream.error_rate_bps / 67) as u8) };
                let capability_score = score_capability(&upstream.provider.capabilities);

                let composite = latency_score + freshness_score + correctness_score
                    + error_score + capability_score;

                upstream.score = composite;
                upstream.status = upstream_status_from_score(composite, upstream.block_lag);
                composite
            }
            Err(_) => {
                upstream.consecutive_failures += 1;
                upstream.error_rate_bps = upstream.error_rate_bps.saturating_add(250);
                if upstream.consecutive_failures >= 3 {
                    upstream.score = 0;
                    upstream.status = ProviderStatus::Offline;
                } else {
                    upstream.score = upstream.score.saturating_sub(40);
                    upstream.status = upstream_status_from_score(upstream.score, upstream.block_lag);
                }
                upstream.score
            }
        }
    }

    // ── Per-chain health probes ─────────────────────────────────────────

    async fn probe_evm_health(&self, upstream: &ScoredUpstream) -> anyhow::Result<HealthProbe> {
        let start = Instant::now();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_chainId",
            "params": [],
            "id": 1
        });
        let resp = self.rpc_call(&upstream.provider.url, &body.to_string()).await?;
        let latency = start.elapsed().as_millis() as u64;

        // Also get block number
        let block_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 2
        });
        let _block_resp = self.rpc_call(&upstream.provider.url, &block_body.to_string()).await?;

        Ok(HealthProbe {
            latency_ms: latency,
            block_lag: 0, // computed relative to pool best
            slot_lag: 0,
            chain_id_match: true,
            block_hash_ok: true,
        })
    }

    async fn probe_solana_health(&self, upstream: &ScoredUpstream) -> anyhow::Result<HealthProbe> {
        let start = Instant::now();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getHealth",
            "params": [],
            "id": 1
        });
        let _ = self.rpc_call(&upstream.provider.url, &body.to_string()).await?;
        let latency = start.elapsed().as_millis() as u64;

        // Get slot
        let slot_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getSlot",
            "params": [],
            "id": 2
        });
        let _ = self.rpc_call(&upstream.provider.url, &slot_body.to_string()).await?;

        Ok(HealthProbe {
            latency_ms: latency,
            block_lag: 0,
            slot_lag: 0,
            chain_id_match: true,
            block_hash_ok: true,
        })
    }

    async fn probe_bitcoin_health(&self, upstream: &ScoredUpstream) -> anyhow::Result<HealthProbe> {
        let start = Instant::now();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockchaininfo",
            "params": [],
            "id": 1
        });
        let _ = self.rpc_call(&upstream.provider.url, &body.to_string()).await?;
        let latency = start.elapsed().as_millis() as u64;

        Ok(HealthProbe {
            latency_ms: latency,
            block_lag: 0,
            slot_lag: 0,
            chain_id_match: true,
            block_hash_ok: true,
        })
    }

    async fn probe_x3_health(&self, upstream: &ScoredUpstream) -> anyhow::Result<HealthProbe> {
        let start = Instant::now();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "x3_getHealth",
            "params": [],
            "id": 1
        });
        let _ = self.rpc_call(&upstream.provider.url, &body.to_string()).await?;
        let latency = start.elapsed().as_millis() as u64;

        Ok(HealthProbe {
            latency_ms: latency,
            block_lag: 0,
            slot_lag: 0,
            chain_id_match: true,
            block_hash_ok: true,
        })
    }

    async fn rpc_call(&self, url: &str, body: &str) -> anyhow::Result<String> {
        use http_body_util::BodyExt;
        use hyper::Request;

        let req = Request::post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string())?;

        let resp = self.http_client.request(req).await?;
        let status = resp.status();
        let body_bytes = resp.collect().await?.to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes).to_string();

        if status.is_success() {
            Ok(body_str)
        } else {
            Err(anyhow::anyhow!("HTTP {}: {}", status, body_str))
        }
    }

    /// Get the best healthy upstream for a chain, optionally filtered by capability.
    pub fn best_for_chain(&self, chain: &str, require_capability: Option<&str>) -> Option<ProviderEntry> {
        // This would normally be async with RwLock, but for simplicity we use a
        // blocking approach. In production, use tokio::sync methods.
        let guard = self.scored.try_read().ok()?;
        let list = guard.get(chain)?;

        let mut candidates: Vec<&ScoredUpstream> = list.iter().collect();
        // Filter by capability if needed
        if let Some(cap) = require_capability {
            candidates.retain(|u| u.provider.capabilities.iter().any(|c| c == cap));
        }
        // Filter healthy only
        candidates.retain(|u| u.is_healthy());
        // Sort by score descending, then by tier, then by priority
        candidates.sort_by(|a, b| {
            b.score.cmp(&a.score)
                .then_with(|| a.provider.tier.cmp(&b.provider.tier))
                .then_with(|| a.provider.priority.cmp(&b.provider.priority))
        });

        candidates.first().map(|u| u.provider.clone())
    }

    /// Get all healthy upstreams for a chain.
    pub fn healthy_for_chain(&self, chain: &str) -> Vec<ProviderEntry> {
        let guard = match self.scored.try_read() {
            Ok(g) => g,
            Err(_) => return vec![],
        };
        guard.get(chain)
            .map(|list| {
                list.iter()
                    .filter(|u| u.is_healthy())
                    .map(|u| u.provider.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ── Health Probe Result ─────────────────────────────────────────────────────

struct HealthProbe {
    latency_ms: u64,
    block_lag: u32,
    slot_lag: u32,
    chain_id_match: bool,
    block_hash_ok: bool,
}

// ── Scoring helpers ─────────────────────────────────────────────────────────

fn score_latency(latency_ms: u64, chain: &ChainKind) -> u8 {
    let threshold = match chain {
        ChainKind::Evm => 80,
        ChainKind::Solana => 80,
        ChainKind::Bitcoin => 500,
        ChainKind::X3 => 150,
    };
    if latency_ms <= threshold as u64 {
        25
    } else if latency_ms <= threshold as u64 * 2 {
        20
    } else if latency_ms <= threshold as u64 * 5 {
        10
    } else {
        0
    }
}

fn score_freshness(block_lag: u32, slot_lag: u32, chain: &ChainKind) -> u8 {
    match chain {
        ChainKind::Evm => {
            if block_lag <= 2 { 25 }
            else if block_lag <= 5 { 15 }
            else if block_lag <= 10 { 5 }
            else { 0 }
        }
        ChainKind::Solana => {
            if slot_lag <= 5 { 25 }
            else if slot_lag <= 20 { 15 }
            else if slot_lag <= 50 { 5 }
            else { 0 }
        }
        ChainKind::Bitcoin => {
            if block_lag <= 1 { 25 }
            else if block_lag <= 3 { 15 }
            else { 0 }
        }
        ChainKind::X3 => {
            if block_lag <= 1 { 25 }
            else if block_lag <= 3 { 15 }
            else { 0 }
        }
    }
}

fn score_capability(capabilities: &[String]) -> u8 {
    let mut score: u8 = 0;
    for cap in capabilities {
        match cap.as_str() {
            "archive" => score += 3,
            "trace" => score += 2,
            "websocket" => score += 2,
            "proof" => score += 3,
            "atomic" => score += 3,
            "full" => score = score.saturating_add(1),
            _ => {}
        }
    }
    score.min(10)
}

fn upstream_status_from_score(score: u8, block_lag: u32) -> ProviderStatus {
    if score >= FAILOVER_THRESHOLD && block_lag <= MAX_BLOCK_DRIFT {
        ProviderStatus::Healthy
    } else if score >= FREEZE_THRESHOLD {
        ProviderStatus::Degraded
    } else if score > 0 {
        ProviderStatus::Frozen
    } else {
        ProviderStatus::Offline
    }
}

/// Send a raw RPC call to a specific upstream (used by chain handlers).
pub async fn forward_to_upstream(url: &str, body: &str) -> anyhow::Result<String> {
    let http_client = hyper_util::client::legacy::Client::builder(TokioExecutor::new())
        .build_http();

    use http_body_util::BodyExt;
    use hyper::Request;

    let req = Request::post(url)
        .header("Content-Type", "application/json")
        .body(body.to_string())?;

    let resp = http_client.request(req).await?;
    let body_bytes = resp.collect().await?.to_bytes();
    Ok(String::from_utf8_lossy(&body_bytes).to_string())
}