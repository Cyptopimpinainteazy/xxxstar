//! Result hash submitter (RC1.5).
//!
//! Submits `ExecutionResult.result_hash` back to the chain via JSON-RPC
//! `author_submitExtrinsic` using a SCALE-encoded unsigned extrinsic that
//! calls `NorthernSwarm::submit_result`. This transforms the RC1 stub into
//! a real on-chain interaction without adding a `subxt` dependency.
//!
//! Proof bundles are still persisted to `./proofs/` for RC3 quorum comparison.

use crate::types::*;
use codec::Encode;
use serde_json::Value;
use tracing::{info, warn};

/// Submits result hashes and proof bundles to the chain.
pub struct ResultSubmitter {
    config: Config,
    http_client: reqwest::Client,
}

impl ResultSubmitter {
    pub fn new(config: Config) -> Self {
        ResultSubmitter {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Submit a task execution result via `author_submitExtrinsic`.
    pub async fn submit(&self, result: ExecutionResult) -> Result<(), NorthernSwarmError> {
        if result.status != ExecutionStatus::Success {
            warn!(
                task_id = %result.task_id,
                status  = ?result.status,
                "skipping submission for non-success result",
            );
            return Ok(());
        }

        let task_id_bytes = result.task_id.as_bytes().to_vec();
        let result_hash_bytes =
            hex::decode(result.result_hash.trim_start_matches("0x")).map_err(|e| {
                NorthernSwarmError::SubmitFailed {
                    task_id: result.task_id.clone(),
                    reason: format!("result_hash hex decode: {e}"),
                }
            })?;

        let call = SubmitResultCall {
            pallet_index: 82u8,
            call_index: 6u8,
            task_id: task_id_bytes,
            result_hash: result_hash_bytes,
        };

        let encoded_call = call.encode();
        let extrinsic_hex = format!("0x{}", hex::encode(&encoded_call));

        let resp = self
            .json_rpc_call("author_submitExtrinsic", &[Value::String(extrinsic_hex)])
            .await?;

        info!(
            task_id     = %result.task_id,
            result_hash = %result.result_hash,
            response    = %resp,
            "result hash submitted to chain",
        );

        self.store_proof_locally(&result.proof).await?;
        Ok(())
    }

    /// Persist a proof bundle to `./proofs/<task_id>.json`.
    async fn store_proof_locally(&self, proof: &ProofBundle) -> Result<(), NorthernSwarmError> {
        let dir = std::path::PathBuf::from("proofs");
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("{}.proof.json", proof.task_id));
        let json = serde_json::to_vec_pretty(proof)?;
        tokio::fs::write(&path, json).await?;
        info!(task_id = %proof.task_id, path = %path.display(), "proof bundle stored");
        Ok(())
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

/// SCALE-encoding structure for `pallet_northern_swarm::Call::submit_result`.
struct SubmitResultCall {
    pallet_index: u8,
    call_index: u8,
    task_id: Vec<u8>,
    result_hash: Vec<u8>,
}

impl Encode for SubmitResultCall {
    fn encode_to<W: codec::Output + ?Sized>(&self, dest: &mut W) {
        self.pallet_index.encode_to(dest);
        self.call_index.encode_to(dest);
        self.task_id.encode_to(dest);
        let mut h256 = [0u8; 32];
        let len = self.result_hash.len().min(32);
        h256[..len].copy_from_slice(&self.result_hash[..len]);
        h256.encode_to(dest);
    }
}
