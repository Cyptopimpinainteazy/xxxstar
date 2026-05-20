//! Gateway client for submitting benchmark results from sidecar to gateway.
//!
//! This client handles HTTP communication between the sidecar and gateway,
//! including result submission, error handling, and retries with exponential backoff.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};
use x3_rpc::benchmark::BenchmarkReport;

/// Configuration for gateway client.
#[derive(Debug, Clone)]
pub struct GatewayClientConfig {
    /// Gateway base URL (e.g., http://localhost:3001)
    pub gateway_url: String,
    /// Optional bearer token for authentication
    pub auth_token: Option<String>,
    /// Max retries for failed submissions
    pub max_retries: u32,
    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,
}

/// Benchmark result payload to submit to gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResultPayload {
    /// Tenant ID
    pub tenant_id: String,
    /// Benchmark report
    pub report: BenchmarkReport,
}

/// Response from gateway on successful result submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResultResponse {
    /// Report ID that was stored
    pub report_id: String,
    /// Status message
    pub status: String,
}

/// Gateway client for benchmark result submission.
#[derive(Clone)]
pub struct GatewayClient {
    config: GatewayClientConfig,
    client: Client,
}

impl GatewayClient {
    /// Create a new gateway client.
    pub fn new(config: GatewayClientConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Submit benchmark results to gateway with retry logic.
    ///
    /// This method attempts to submit benchmark results to the gateway with
    /// exponential backoff retry strategy. If all retries are exhausted, it
    /// returns an error with context.
    pub async fn submit_benchmark_result(
        &self,
        payload: &BenchmarkResultPayload,
    ) -> Result<BenchmarkResultResponse> {
        let url = format!(
            "{}/api/v1/benchmarks/results",
            self.config.gateway_url.trim_end_matches('/')
        );

        let mut retry_count = 0;
        let max_retries = self.config.max_retries;

        loop {
            debug!(
                "Attempting to submit benchmark result (attempt {}/{})",
                retry_count + 1,
                max_retries + 1
            );

            match self.attempt_submission(&url, payload).await {
                Ok(response) => {
                    info!(
                        "Successfully submitted benchmark result: {}",
                        response.report_id
                    );
                    return Ok(response);
                }
                Err(e) => {
                    if retry_count < max_retries {
                        // Calculate exponential backoff: 100ms * 2^retry_count
                        let backoff_ms = self.config.initial_backoff_ms * (1 << retry_count);

                        warn!(
                            "Benchmark result submission failed (attempt {}/{}), retrying in {}ms: {}",
                            retry_count + 1,
                            max_retries + 1,
                            backoff_ms,
                            e
                        );

                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        retry_count += 1;
                    } else {
                        error!(
                            "Benchmark result submission failed after {} retries: {}",
                            max_retries + 1,
                            e
                        );
                        return Err(e).context(format!(
                            "Failed to submit benchmark result after {} retries",
                            max_retries + 1
                        ));
                    }
                }
            }
        }
    }

    /// Attempt a single submission.
    async fn attempt_submission(
        &self,
        url: &str,
        payload: &BenchmarkResultPayload,
    ) -> Result<BenchmarkResultResponse> {
        let mut request = self.client.post(url).json(&payload);

        // Add authorization header if token is configured
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .context("Failed to send benchmark result submission request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "(unable to read response body)".to_string());
            return Err(anyhow::anyhow!(
                "Gateway returned error status {}: {}",
                status,
                body
            ));
        }

        let result = response
            .json::<BenchmarkResultResponse>()
            .await
            .context("Failed to parse gateway response as JSON")?;

        Ok(result)
    }

    /// Check gateway health/connectivity.
    pub async fn check_health(&self) -> Result<bool> {
        let url = format!("{}/health", self.config.gateway_url.trim_end_matches('/'));

        match self.client.get(&url).send().await {
            Ok(response) => {
                debug!("Gateway health check: {}", response.status());
                Ok(response.status().is_success())
            }
            Err(e) => {
                warn!("Failed to check gateway health: {}", e);
                Err(e).context("Failed to connect to gateway")
            }
        }
    }
}

impl Default for GatewayClientConfig {
    fn default() -> Self {
        Self {
            gateway_url: "http://localhost:3001".to_string(),
            auth_token: None,
            max_retries: 3,
            initial_backoff_ms: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_client_config_default() {
        let config = GatewayClientConfig::default();
        assert_eq!(config.gateway_url, "http://localhost:3001");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 100);
        assert!(config.auth_token.is_none());
    }

    #[test]
    fn test_gateway_client_url_formatting() {
        let config = GatewayClientConfig {
            gateway_url: "http://localhost:3001/".to_string(),
            auth_token: None,
            max_retries: 3,
            initial_backoff_ms: 100,
        };
        let client = GatewayClient::new(config);
        // Verify client is created successfully
        assert!(!client.config.gateway_url.is_empty());
    }
}
