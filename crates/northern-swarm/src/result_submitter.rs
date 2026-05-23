//! Result hash submitter (RC1).
//!
//! Submits `ExecutionResult.result_hash` back to the chain via an extrinsic
//! call to `NorthernSwarm::submit_result`.
//!
//! **RC1 stub**: logs the hash and writes the proof bundle to `./proofs/`.
//! **RC2 target**: sign and submit `pallet_northern_swarm::Call::submit_result`
//! via `subxt` using the executor's `NS_EXECUTOR_KEY`.

use crate::types::*;
use tracing::{info, warn};

/// Submits result hashes and proof bundles to the chain.
pub struct ResultSubmitter {
    config: Config,
}

impl ResultSubmitter {
    pub fn new(config: Config) -> Self {
        ResultSubmitter { config }
    }

    /// Submit a task execution result.
    ///
    /// Skips submission for non-successful results and logs a warning instead.
    /// Stores the proof bundle locally for RC3 quorum comparison regardless.
    pub async fn submit(&self, result: ExecutionResult) -> Result<(), NorthernSwarmError> {
        if result.status != ExecutionStatus::Success {
            warn!(
                task_id = %result.task_id,
                status  = ?result.status,
                "skipping submission for non-success result",
            );
            return Ok(());
        }

        // RC1 stub: log what would be submitted.
        info!(
            task_id     = %result.task_id,
            result_hash = %result.result_hash,
            chain_rpc   = %self.config.chain_rpc_url,
            "[RC1 stub] would call NorthernSwarm::submit_result on-chain here",
        );

        // TODO(RC2): replace stub with subxt extrinsic:
        //
        //   let api = OnlineClient::<PolkadotConfig>::from_url(&self.config.chain_rpc_url)
        //       .await.map_err(|e| NorthernSwarmError::ChainRpc(e.to_string()))?;
        //   let signer = PairSigner::new(sr25519::Pair::from_string(&self.config.executor_key, None)?);
        //   let payload = result.result_hash.as_bytes().to_vec();
        //   let tx = northern_swarm::tx()
        //       .northern_swarm()
        //       .submit_result(result.task_id.as_bytes().to_vec(), payload);
        //   api.tx().sign_and_submit_then_watch_default(&tx, &signer).await?;

        self.store_proof_locally(&result.proof).await?;
        Ok(())
    }

    /// Persist a proof bundle to `./proofs/<task_id>.json`.
    ///
    /// Used by the RC3 quorum engine to compare bundles from multiple executors.
    async fn store_proof_locally(&self, proof: &ProofBundle) -> Result<(), NorthernSwarmError> {
        let dir = std::path::PathBuf::from("proofs");
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("{}.proof.json", proof.task_id));
        let json = serde_json::to_vec_pretty(proof)?;
        tokio::fs::write(&path, json).await?;
        info!(task_id = %proof.task_id, path = %path.display(), "proof bundle stored");
        Ok(())
    }
}
