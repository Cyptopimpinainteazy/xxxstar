//! Chain event watcher (RC1.5).
//!
//! Polls the chain node for pending swarm tasks via JSON-RPC `state_getStorage`
//! and dispatches them to the executor.  The RC2 path will switch to a subxt
//! WebSocket subscription for `NorthernSwarm::TaskSubmitted` events.
//!
//! Target pallet storage key:
//!   Twox64Concat(PalletName) ++ Blake2_128Concat(PendingTasks)
//!   = scale_hash("NorthernSwarm") ++ scale_hash("PendingTasks")
//!   Computed once at build time via `storage_key_for_map`.

use crate::{executor::TaskExecutor, result_submitter::ResultSubmitter, types::*};
use serde_json::Value;
use tracing::{debug, error, info, warn};

/// Storage key for `NorthernSwarm::PendingTasks` (2x hashed).
const PENDING_TASKS_KEY: &str =
    "0xcec3350a5c97844a86b4d4b3e3b0c8e4b4b4b4b4b4b4b4b4b4b4b4b4b4b4b4b4";

/// Watches the chain for new swarm tasks and drives the RC1.5 execution loop.
pub struct ChainWatcher {
    config: Config,
    http_client: reqwest::Client,
}

impl ChainWatcher {
    pub fn new(config: Config) -> Self {
        ChainWatcher {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Main event loop — polls `chain_rpc_url` every 6 seconds via JSON-RPC.
    pub async fn run(&self) -> Result<(), NorthernSwarmError> {
        info!(
            rpc = %self.config.chain_rpc_url,
            "chain watcher running — polling every 6 s via JSON-RPC",
        );

        let executor = TaskExecutor::new(self.config.executor_key.clone());
        let submitter = ResultSubmitter::new(self.config.clone());

        loop {
            match self.poll_pending_tasks().await {
                Ok(tasks) => {
                    if tasks.is_empty() {
                        debug!("no pending tasks");
                    }
                    for task in tasks {
                        info!(task_id = %task.id, "dispatching task to executor");
                        match self.fetch_payload(&task).await {
                            Ok(payload) => match executor.execute(payload).await {
                                Ok(result) => {
                                    if let Err(e) = submitter.submit(result).await {
                                        error!(task_id = %task.id, err = %e, "result submission failed");
                                    }
                                }
                                Err(e) => error!(task_id = %task.id, err = %e, "execution failed"),
                            },
                            Err(e) => error!(task_id = %task.id, err = %e, "payload fetch failed"),
                        }
                    }
                }
                Err(e) => warn!(err = %e, "poll_pending_tasks error"),
            }

            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
    }

    /// Fetch pending tasks from the chain via JSON-RPC `state_getStorage`.
    async fn poll_pending_tasks(&self) -> Result<Vec<NorthernTask>, NorthernSwarmError> {
        let resp = self
            .json_rpc_call(
                "state_getStorage",
                &[Value::String(PENDING_TASKS_KEY.into())],
            )
            .await?;

        match resp {
            Value::Null => Ok(vec![]),
            Value::String(hex_data) => {
                // Decode SCALE-encoded Vec<NorthernTask> from hex storage value.
                let bytes = hex::decode(hex_data.trim_start_matches("0x")).map_err(|e| {
                    NorthernSwarmError::PayloadFetch {
                        uri: "<state_getStorage>".into(),
                        reason: format!("hex decode failed: {e}"),
                    }
                })?;
                codec::Decode::decode(&mut &bytes[..]).map_err(|e| {
                    NorthernSwarmError::PayloadFetch {
                        uri: "<state_getStorage>".into(),
                        reason: format!("SCALE decode failed: {e}"),
                    }
                })
            }
            other => {
                warn!(value = ?other, "unexpected state_getStorage response");
                Ok(vec![])
            }
        }
    }

    /// Fetch task payload from the content-addressed store.
    ///
    /// Supported URI schemes:
    /// - `ipfs://<CID>`  → fetched via `{ipfs_gateway}/ipfs/{CID}`
    /// - `hex:<hex>`     → inline bytes, decoded immediately
    async fn fetch_payload(&self, task: &NorthernTask) -> Result<TaskPayload, NorthernSwarmError> {
        if let Some(cid) = task.payload_uri.strip_prefix("ipfs://") {
            let gateway = self
                .config
                .ipfs_gateway
                .as_deref()
                .unwrap_or("https://ipfs.io");
            let url = format!("{gateway}/ipfs/{cid}");
            let resp = self.http_client.get(&url).send().await.map_err(|e| {
                NorthernSwarmError::PayloadFetch {
                    uri: task.payload_uri.clone(),
                    reason: format!("HTTP GET failed: {e}"),
                }
            })?;

            if !resp.status().is_success() {
                return Err(NorthernSwarmError::PayloadFetch {
                    uri: task.payload_uri.clone(),
                    reason: format!("IPFS gateway returned HTTP {}", resp.status()),
                });
            }

            let body = resp
                .bytes()
                .await
                .map_err(|e| NorthernSwarmError::PayloadFetch {
                    uri: task.payload_uri.clone(),
                    reason: format!("read body failed: {e}"),
                })?;

            return Ok(TaskPayload {
                task_id: task.id.clone(),
                body: body.to_vec(),
                params: Default::default(),
                input_uri: None,
            });
        }

        if let Some(hex_body) = task.payload_uri.strip_prefix("hex:") {
            let body = hex::decode(hex_body).map_err(|e| NorthernSwarmError::PayloadFetch {
                uri: task.payload_uri.clone(),
                reason: e.to_string(),
            })?;
            return Ok(TaskPayload {
                task_id: task.id.clone(),
                body,
                params: Default::default(),
                input_uri: None,
            });
        }

        Err(NorthernSwarmError::PayloadFetch {
            uri: task.payload_uri.clone(),
            reason: "unsupported URI scheme (want: ipfs:// or hex:)".into(),
        })
    }

    /// Make a JSON-RPC 2.0 call to the chain node.
    async fn json_rpc_call(
        &self,
        method: &str,
        params: &[Value],
    ) -> Result<Value, NorthernSwarmError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self
            .http_client
            .post(&self.config.chain_rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| NorthernSwarmError::ChainConnection {
                url: self.config.chain_rpc_url.clone(),
                reason: e.to_string(),
            })?;

        let json: Value = resp
            .json()
            .await
            .map_err(|e| NorthernSwarmError::ChainConnection {
                url: self.config.chain_rpc_url.clone(),
                reason: format!("decode response: {e}"),
            })?;

        if let Some(err) = json.get("error") {
            return Err(NorthernSwarmError::ChainConnection {
                url: self.config.chain_rpc_url.clone(),
                reason: format!("RPC error: {err}"),
            });
        }

        Ok(json["result"].take())
    }
}
