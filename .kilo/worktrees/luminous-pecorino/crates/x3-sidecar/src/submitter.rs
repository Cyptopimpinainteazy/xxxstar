//! Chain submitter for receipt submission

use crate::receipt::ExecutionReceipt;
use serde::{Deserialize, Serialize};

/// Chain submitter
pub struct ChainSubmitter {
    /// RPC URL
    rpc_url: String,
    /// Executor key (hex)
    _executor_key: String,
    /// HTTP client
    client: reqwest::Client,
}

/// RPC request
#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: u32,
    method: String,
    params: Vec<serde_json::Value>,
}

/// RPC response
#[derive(Deserialize)]
struct RpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u32,
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

/// RPC error
#[derive(Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl ChainSubmitter {
    /// Create a new chain submitter
    pub fn new(rpc_url: String, executor_key: String) -> Self {
        Self {
            rpc_url,
            _executor_key: executor_key,
            client: reqwest::Client::new(),
        }
    }

    /// Submit a receipt to the chain
    pub async fn submit_receipt(&self, receipt: &ExecutionReceipt) -> anyhow::Result<String> {
        tracing::info!("Submitting receipt for job {}", hex::encode(receipt.job_id));

        // Encode receipt
        let receipt_bytes = receipt.encode();
        let receipt_hex = format!("0x{}", hex::encode(&receipt_bytes));

        // Create RPC call
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "x3Verifier_submitReceipt".to_string(),
            params: vec![serde_json::json!(receipt_hex)],
        };

        // Send request
        let response: RpcResponse = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        // Check for errors
        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        // Extract transaction hash
        if let Some(result) = response.result {
            if let Some(tx_hash) = result.as_str() {
                return Ok(tx_hash.to_string());
            }
        }

        Err(anyhow::anyhow!("No transaction hash in response"))
    }

    /// Get job status from chain
    pub async fn get_job_status(&self, job_id: [u8; 32]) -> anyhow::Result<String> {
        let job_id_hex = format!("0x{}", hex::encode(job_id));

        match self
            .call_rpc(
                "x3Verifier_getJobStatus",
                vec![serde_json::json!(job_id_hex.clone())],
            )
            .await
        {
            Ok(result) => Ok(normalize_job_status(&result).unwrap_or_else(|| result.to_string())),
            Err(err) if is_method_not_found(&err) => {
                // Backward-compatible fallback to current node RPC surface.
                let result = self
                    .call_rpc(
                        "x3Verifier_getJob",
                        vec![serde_json::json!(job_id_hex), serde_json::Value::Null],
                    )
                    .await?;
                Ok(normalize_job_status(&result).unwrap_or_else(|| "unknown".to_string()))
            }
            Err(err) => Err(err),
        }
    }

    /// Check if executor is registered on chain
    pub async fn is_registered(&self, executor_pubkey: [u8; 32]) -> anyhow::Result<bool> {
        let executor_hex = format!("0x{}", hex::encode(executor_pubkey));

        match self
            .call_rpc(
                "x3Verifier_isExecutorRegistered",
                vec![serde_json::json!(executor_hex.clone())],
            )
            .await
        {
            Ok(result) => Ok(result.as_bool().unwrap_or(false)),
            Err(err) if is_method_not_found(&err) => {
                let result = self
                    .call_rpc(
                        "x3Verifier_isExecutor",
                        vec![serde_json::json!(executor_hex), serde_json::Value::Null],
                    )
                    .await?;
                Ok(result.as_bool().unwrap_or(false))
            }
            Err(err) => Err(err),
        }
    }

    /// Get pending jobs from chain
    pub async fn get_pending_jobs(&self, limit: u32) -> anyhow::Result<Vec<PendingJob>> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "x3Verifier_getPendingJobs".to_string(),
            params: vec![serde_json::json!(limit)],
        };

        let response: RpcResponse = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        if let Some(result) = response.result {
            let jobs: Vec<PendingJob> = serde_json::from_value(result)?;
            return Ok(jobs);
        }

        Ok(vec![])
    }

    async fn call_rpc(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: method.to_string(),
            params,
        };

        let response: RpcResponse = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result field returned for method {}", method))
    }
}

fn is_method_not_found(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("RPC error -32601") || msg.contains("Method not found")
}

fn normalize_job_status(value: &serde_json::Value) -> Option<String> {
    if value.is_null() {
        return Some("unknown".to_string());
    }

    if let Some(status) = value.as_str() {
        return Some(status.to_string());
    }

    if let Some(status) = value.get("status") {
        if let Some(raw) = status.as_u64() {
            return Some(status_code_to_name(raw).to_string());
        }
        if let Some(raw) = status.as_str() {
            if let Ok(code) = raw.parse::<u64>() {
                return Some(status_code_to_name(code).to_string());
            }
            return Some(raw.to_string());
        }
    }

    None
}

fn status_code_to_name(code: u64) -> &'static str {
    match code {
        0 => "pending",
        1 => "submitted",
        2 => "verified",
        3 => "applied",
        4 => "failed",
        5 => "disputed",
        _ => "unknown",
    }
}

/// Pending job from chain
#[derive(Debug, Clone, Deserialize)]
pub struct PendingJob {
    pub job_id: String,
    pub bytecode_hash: String,
    pub input_hash: String,
    pub gas_limit: String,
    pub reward: String,
    pub submitter: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, routing::post, Json, Router};
    use std::sync::Arc;

    #[derive(Clone)]
    struct MockRpcState {
        fallback_only: bool,
    }

    async fn rpc_handler(
        State(state): State<Arc<MockRpcState>>,
        Json(request): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let method = request
            .get("method")
            .and_then(|value| value.as_str())
            .unwrap_or_default();

        let response = match method {
            "x3Verifier_isExecutorRegistered" if state.fallback_only => serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": { "code": -32601, "message": "Method not found" }
            }),
            "x3Verifier_isExecutorRegistered" | "x3Verifier_isExecutor" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": true
            }),
            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": { "code": -32601, "message": "Method not found" }
            }),
        };

        Json(response)
    }

    async fn spawn_mock_rpc_server(fallback_only: bool) -> String {
        let app = Router::new()
            .route("/", post(rpc_handler))
            .with_state(Arc::new(MockRpcState { fallback_only }));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("listener address");
        let listener = listener.into_std().expect("convert listener");

        tokio::spawn(async move {
            axum::Server::from_tcp(listener)
                .expect("server from tcp")
                .serve(app.into_make_service())
                .await
                .expect("serve mock rpc");
        });

        format!("http://{}", address)
    }

    #[tokio::test]
    async fn is_registered_uses_primary_rpc_when_available() {
        let submitter = ChainSubmitter::new(spawn_mock_rpc_server(false).await, "01".repeat(32));

        let is_registered = submitter
            .is_registered([7u8; 32])
            .await
            .expect("primary registration check succeeds");
        assert!(is_registered);
    }

    #[tokio::test]
    async fn is_registered_falls_back_to_legacy_rpc_surface() {
        let submitter = ChainSubmitter::new(spawn_mock_rpc_server(true).await, "01".repeat(32));

        let is_registered = submitter
            .is_registered([9u8; 32])
            .await
            .expect("fallback registration check succeeds");
        assert!(is_registered);
    }
}
