//! Chain event watcher (RC1).
//!
//! Polls the chain node for pending swarm tasks and dispatches them to the
//! executor.  Currently uses a 6-second block-time polling loop; RC1.5 will
//! switch to a `subxt` WebSocket subscription for `NorthernSwarm::TaskSubmitted`
//! events from the RC2 pallet.

use crate::{executor::TaskExecutor, result_submitter::ResultSubmitter, types::*};
use tracing::{debug, error, info, warn};

/// Watches the chain for new swarm tasks and drives the RC1 execution loop.
pub struct ChainWatcher {
    config: Config,
}

impl ChainWatcher {
    pub fn new(config: Config) -> Self {
        ChainWatcher { config }
    }

    /// Main event loop — runs indefinitely until an unrecoverable error occurs.
    ///
    /// **RC1**: polls `chain_rpc_url` every 6 seconds (one block target).
    /// **RC1.5**: replace `poll_pending_tasks` with a `subxt` WS subscription:
    /// ```text
    /// let api = OnlineClient::<PolkadotConfig>::from_url(&url).await?;
    /// let mut sub = api.blocks().subscribe_finalized().await?;
    /// while let Some(block) = sub.next().await { ... }
    /// ```
    pub async fn run(&self) -> Result<(), NorthernSwarmError> {
        info!(
            rpc = %self.config.chain_rpc_url,
            "chain watcher running — polling every 6 s",
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

    /// Fetch pending tasks from the chain.
    ///
    /// **RC1 stub**: returns empty vec.
    ///
    /// **RC1.5 implementation target**:
    /// ```text
    /// let resp = json_rpc_call(
    ///     &self.config.chain_rpc_url,
    ///     "state_getStorage",
    ///     &[NORTHERN_SWARM_PENDING_TASKS_KEY],
    /// ).await?;
    /// SCALE::decode::<Vec<NorthernTask>>(resp)
    /// ```
    async fn poll_pending_tasks(&self) -> Result<Vec<NorthernTask>, NorthernSwarmError> {
        // TODO(RC1.5): implement JSON-RPC `state_getStorage` call for
        // `NorthernSwarm::PendingTasks` storage map.
        Ok(vec![])
    }

    /// Fetch task payload from the content-addressed store.
    ///
    /// Supported URI schemes:
    /// - `ipfs://<CID>`  → fetched via `{ipfs_gateway}/ipfs/{CID}` (RC1.5)
    /// - `hex:<hex>`     → inline bytes, decoded immediately
    async fn fetch_payload(&self, task: &NorthernTask) -> Result<TaskPayload, NorthernSwarmError> {
        if task.payload_uri.starts_with("ipfs://") {
            // TODO(RC1.5): GET {ipfs_gateway}/ipfs/{cid}
            return Err(NorthernSwarmError::PayloadFetch {
                uri: task.payload_uri.clone(),
                source: "IPFS fetch not yet implemented — target RC1.5".into(),
            });
        }

        if let Some(hex_body) = task.payload_uri.strip_prefix("hex:") {
            let body = hex::decode(hex_body).map_err(|e| NorthernSwarmError::PayloadFetch {
                uri: task.payload_uri.clone(),
                source: e.to_string(),
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
            source: "unsupported URI scheme (want: ipfs:// or hex:)".into(),
        })
    }
}
