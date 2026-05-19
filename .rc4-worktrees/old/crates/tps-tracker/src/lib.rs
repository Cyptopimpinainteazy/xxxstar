#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    non_snake_case,
    unexpected_cfgs,
    unused_parens,
    non_camel_case_types,
    clippy::all
)]

/// X3 Chain TPS Tracker
///
/// This module polls the X3 Chain node for transaction data,
/// calculates transactions per second (TPS) from the data received,
/// and inserts the calculated TPS along with other metrics into InfluxDB.
///
/// Adapted from: Solana Project by Amil Shrivastava
/// For: X3 Chain Blockchain Platform
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

/// Response from X3 Chain RPC API for block information
#[derive(Debug, Deserialize, Serialize)]
struct BlockInfoResponse {
    jsonrpc: String,
    result: Option<BlockInfo>,
    error: Option<RpcError>,
}

/// Block information structure
#[derive(Debug, Deserialize, Serialize, Clone)]
struct BlockInfo {
    #[serde(rename = "blockTime")]
    block_time: i64,

    #[serde(rename = "blockHeight")]
    block_height: u64,

    #[serde(rename = "transactionCount")]
    transaction_count: Option<u64>,

    #[serde(rename = "parentSlot")]
    parent_slot: Option<u64>,

    parent_hash: Option<String>,
    blockhash: Option<String>,
}

/// RPC error response
#[derive(Debug, Deserialize, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

/// Transaction statistics for time-series storage
#[derive(Debug, Clone)]
struct TransactionStats {
    time: DateTime<Utc>,
    block_height: u64,
    transaction_count: u64,
    tps: f64,
    block_time_seconds: i64,
}

impl TransactionStats {
    /// Convert to InfluxDB line protocol string
    fn to_line_protocol(&self) -> String {
        let timestamp_ns = self.time.timestamp_nanos_opt().unwrap_or(0);
        format!(
            "transaction_stats,block={} tps={},transaction_count={},block_time_seconds={} {}",
            self.block_height,
            self.tps,
            self.transaction_count,
            self.block_time_seconds,
            timestamp_ns
        )
    }
}

/// Network performance metrics
#[derive(Debug, Clone)]
struct NetworkMetrics {
    time: DateTime<Utc>,
    avg_tps: f64,
    peak_tps: f64,
    min_tps: f64,
    block_interval_ms: f64,
}

/// TPS Tracker configuration
#[derive(Debug, Clone)]
pub struct TpsTrackerConfig {
    pub rpc_url: String,
    pub influx_url: String,
    pub influx_db: String,
    pub influx_token: String,
    pub poll_interval_secs: u64,
    pub buffer_size: usize,
}

impl Default for TpsTrackerConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:9944".to_string(),
            influx_url: "http://influxdb:8086".to_string(),
            influx_db: "x3_chain_tps".to_string(),
            influx_token: "x3-chain-key".to_string(),
            poll_interval_secs: 1,
            buffer_size: 100,
        }
    }
}

/// Main TPS tracker
pub struct TpsTracker {
    config: TpsTrackerConfig,
    http_client: Client,
    metrics_buffer: Vec<TransactionStats>,
    last_block_height: u64,
}

impl TpsTracker {
    pub fn new(config: TpsTrackerConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            metrics_buffer: Vec::new(),
            last_block_height: 0,
        }
    }

    /// Fetch current block information from RPC
    async fn get_block_info(&self) -> Result<BlockInfo, Box<dyn std::error::Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "system_syncState",
            "params": []
        });

        let response = self
            .http_client
            .post(&self.config.rpc_url)
            .json(&request)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        let block_response: BlockInfoResponse = response.json().await?;

        if let Some(error) = block_response.error {
            return Err(format!("RPC Error: {}", error.message).into());
        }

        block_response.result.ok_or("No result in response".into())
    }

    /// Calculate TPS from block information
    fn calculate_tps(&self, tx_count: u64, block_time: i64) -> f64 {
        if block_time <= 0 {
            return 0.0;
        }
        tx_count as f64 / block_time as f64
    }

    /// Store metrics in InfluxDB
    async fn write_metrics(
        &self,
        metrics: &[TransactionStats],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if metrics.is_empty() {
            return Ok(());
        }

        // Build line protocol string
        let lines: Vec<String> = metrics.iter().map(|m| m.to_line_protocol()).collect();
        let data = lines.join("\n");

        // Write to InfluxDB using HTTP API
        self.http_client
            .post(&format!(
                "{}/write?db={}",
                self.config.influx_url, self.config.influx_db
            ))
            .header(
                "Authorization",
                format!("Bearer {}", self.config.influx_token),
            )
            .body(data)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        Ok(())
    }

    /// Main tracking loop
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("TPS Tracker starting...");
        info!("RPC URL: {}", self.config.rpc_url);
        info!("InfluxDB: {}", self.config.influx_url);

        // Wait for services to be ready
        sleep(Duration::from_secs(5)).await;

        loop {
            match self.get_block_info().await {
                Ok(block_info) => {
                    let current_height = block_info.block_height;
                    let tx_count = block_info.transaction_count.unwrap_or(0);
                    let block_time = block_info.block_time;

                    if current_height > self.last_block_height {
                        let tps = self.calculate_tps(tx_count, block_time);

                        let stat = TransactionStats {
                            time: Utc::now(),
                            block_height: current_height,
                            transaction_count: tx_count,
                            tps,
                            block_time_seconds: block_time,
                        };

                        info!(
                            "Block #{}: {} txs, TPS: {:.2}",
                            current_height, tx_count, tps
                        );

                        self.metrics_buffer.push(stat);
                        self.last_block_height = current_height;

                        // Flush buffer when it reaches capacity
                        if self.metrics_buffer.len() >= self.config.buffer_size {
                            if let Err(e) = self.write_metrics(&self.metrics_buffer).await {
                                error!("Failed to write metrics: {}", e);
                            } else {
                                info!("Flushed {} metrics to InfluxDB", self.metrics_buffer.len());
                                self.metrics_buffer.clear();
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch block info: {}", e);
                }
            }

            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_tps() {
        let config = TpsTrackerConfig::default();
        let tracker = TpsTracker::new(config);

        // Test normal case
        let tps = tracker.calculate_tps(100, 10);
        assert_eq!(tps, 10.0);

        // Test zero block time
        let tps = tracker.calculate_tps(100, 0);
        assert_eq!(tps, 0.0);
    }

    #[test]
    fn test_tracker_creation() {
        let config = TpsTrackerConfig {
            rpc_url: "http://localhost:9944".to_string(),
            ..Default::default()
        };

        let tracker = TpsTracker::new(config);
        assert_eq!(tracker.config.rpc_url, "http://localhost:9944");
    }
}
