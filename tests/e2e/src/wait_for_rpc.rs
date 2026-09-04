use rand::{thread_rng, Rng};
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WaitError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub initial_interval: Duration,
    pub max_interval: Duration,
    pub max_elapsed: Duration,
    pub jitter_ratio: f32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_millis(500),
            max_interval: Duration::from_secs(8),
            max_elapsed: Duration::from_secs(300), // 5 minutes
            jitter_ratio: 0.2,
        }
    }
}

/// Waits for a JSON-RPC endpoint to satisfy the predicate.
///
/// endpoint: base URL (e.g., http://127.0.0.1:9933)
/// method: JSON-RPC method to call (e.g., "system_health" or "chain_getHeader")
/// expected_predicate: closure that returns true for acceptable response
/// client: reqwest client to use
/// retry: RetryPolicy
pub async fn wait_for_rpc_health<F>(
    endpoint: &str,
    method: &str,
    expected_predicate: F,
    client: &Client,
    retry: RetryPolicy,
) -> Result<(), WaitError>
where
    F: Fn(&Value) -> bool,
{
    let start = Instant::now();
    let mut interval = retry.initial_interval;

    let url = endpoint.trim_end_matches('/').to_string();

    let mut id = 1u64;

    loop {
        if start.elapsed() > retry.max_elapsed {
            return Err(WaitError::Timeout(retry.max_elapsed));
        }

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": []
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(json) => {
                    if expected_predicate(&json) {
                        return Ok(());
                    }
                }
                Err(e) => {
                    // continue and retry
                    tracing::debug!("rpc parse error: %s", %e);
                }
            },
            Err(e) => {
                tracing::debug!("rpc request error: %s", %e);
            }
        }

        // exponential backoff with jitter
        let jitter = thread_rng().gen_range(0.0..(interval.as_secs_f32() * retry.jitter_ratio));
        let sleep = interval + Duration::from_secs_f32(jitter);
        tokio::time::sleep(sleep).await;

        interval = std::cmp::min(interval * 2, retry.max_interval);
        id = id.wrapping_add(1);
    }
}
