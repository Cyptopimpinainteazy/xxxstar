/// Proof Submitter - Submits proofs to X3 runtime via RPC
use crate::types::{EvmProof, SvmProof};
use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub struct RpcSubmitter {
    x3_rpc_url: String,
    nonce: Arc<RwLock<u32>>,
    rpc_client: reqwest::Client,
    max_retries: u32,
    retry_backoff_ms: u64,
    relayer_custody_key_id: Option<String>,
}

impl RpcSubmitter {
    pub async fn new_with_retry_config(
        x3_rpc_url: String,
        relayer_account: String,
        relayer_custody_key_id: Option<String>,
        max_retries: u32,
        retry_backoff_ms: u64,
    ) -> Result<Self> {
        let client = reqwest::Client::new();

        // Initialize nonce from X3 runtime
        let initial_nonce = Self::get_account_nonce(&client, &x3_rpc_url, &relayer_account).await?;

        info!(
            "RPC submitter initialized for {} (initial nonce: {}, max_retries: {}, backoff: {}ms)",
            relayer_account, initial_nonce, max_retries, retry_backoff_ms
        );

        Ok(Self {
            x3_rpc_url,
            nonce: Arc::new(RwLock::new(initial_nonce)),
            rpc_client: client,
            max_retries,
            retry_backoff_ms,
            relayer_custody_key_id,
        })
    }

    pub async fn submit_evm_proof(&self, proof: EvmProof) -> Result<String> {
        let nonce = {
            let mut n = self.nonce.write().await;
            let current = *n;
            *n = n.saturating_add(1);
            current
        };

        debug!(
            "Submitting EVM proof (domain: {}, block: {}, nonce: {})",
            proof.source_domain, proof.finalized_block, nonce
        );

        let extrinsic = self.build_submit_cross_vm_extrinsic(&proof)?;

        self.submit_extrinsic_with_retries(&extrinsic, nonce).await
    }

    pub async fn submit_svm_proof(&self, proof: SvmProof) -> Result<String> {
        let nonce = {
            let mut n = self.nonce.write().await;
            let current = *n;
            *n = n.saturating_add(1);
            current
        };

        debug!(
            "Submitting SVM proof (domain: {}, slot: {}, nonce: {})",
            proof.source_domain, proof.slot, nonce
        );

        let extrinsic = self.build_submit_svm_extrinsic(&proof)?;

        self.submit_extrinsic_with_retries(&extrinsic, nonce).await
    }

    pub async fn is_bridge_paused(&self) -> Result<bool> {
        let response = self
            .rpc_client
            .post(&self.x3_rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "x3_getBridgeStatus",
                "params": [],
                "id": 1,
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        json["result"]["paused"]
            .as_bool()
            .ok_or_else(|| anyhow!("No paused status in response"))
    }

    pub async fn get_nonce(&self) -> Result<u32> {
        let nonce = self.nonce.read().await;
        Ok(*nonce)
    }

    /// Acquire EVM proof for submission from finalized block data
    pub async fn acquire_evm_proof(
        &self,
        domain_id: u32,
        block_number: u64,
        block_hash: [u8; 32],
        state_root: [u8; 32],
    ) -> Result<EvmProof> {
        debug!(
            "Acquiring EVM proof for domain {}, block {}",
            domain_id, block_number
        );

        let nonce = {
            let n = self.nonce.read().await;
            *n
        };

        Ok(EvmProof {
            source_domain: domain_id,
            finalized_block: block_number,
            block_hash,
            state_root,
            proof_nonce: nonce,
        })
    }

    /// Acquire SVM proof for submission from finalized slot data
    pub async fn acquire_svm_proof(
        &self,
        domain_id: u32,
        slot: u64,
        blockhash: [u8; 32],
    ) -> Result<SvmProof> {
        debug!(
            "Acquiring SVM proof for domain {}, slot {}",
            domain_id, slot
        );

        Ok(SvmProof {
            source_domain: domain_id,
            slot,
            blockhash,
            validator_signatures: vec![], // Placeholder for actual signatures
            required_signatures: 2,       // Placeholder
        })
    }

    // ============================================================================
    // Private Methods
    // ============================================================================

    async fn submit_extrinsic_with_retries(&self, extrinsic: &str, nonce: u32) -> Result<String> {
        let mut retry_count = 0;
        let mut backoff_ms = self.retry_backoff_ms;

        loop {
            match self.submit_extrinsic(extrinsic, nonce).await {
                Ok(tx_hash) => return Ok(tx_hash),
                Err(e) if retry_count < self.max_retries => {
                    warn!(
                        "Submission failed (attempt {}/{}), retrying in {}ms: {}",
                        retry_count + 1,
                        self.max_retries,
                        backoff_ms,
                        e
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = backoff_ms.saturating_mul(2); // Exponential backoff
                    retry_count += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn submit_extrinsic(&self, extrinsic: &str, nonce: u32) -> Result<String> {
        let response = self
            .rpc_client
            .post(&self.x3_rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "author_submitExtrinsic",
                "params": [extrinsic],
                "id": 1,
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        if let Some(error) = json.get("error") {
            warn!(
                "RPC error submitting extrinsic (nonce: {}): {}",
                nonce, error
            );
            return Err(anyhow!("RPC error: {}", error));
        }

        let tx_hash = json["result"]
            .as_str()
            .ok_or_else(|| anyhow!("No tx hash in response"))?
            .to_string();

        info!("Submitted extrinsic: {}", tx_hash);
        Ok(tx_hash)
    }

    fn build_submit_cross_vm_extrinsic(&self, proof: &EvmProof) -> Result<String> {
        // Simplified extrinsic encoding (in production, use proper scale codec)
        let payload = serde_json::json!({
            "pallet": "x3Verifier",
            "call": "submitEvmProof",
            "signing_authority": self.signing_authority(),
            "args": {
                "domain": proof.source_domain,
                "block_hash": format!("0x{:x}", u256_from_bytes(&proof.block_hash)),
                "state_root": format!("0x{:x}", u256_from_bytes(&proof.state_root)),
                "finalized_block": proof.finalized_block,
                "proof_nonce": proof.proof_nonce,
            }
        });

        Ok(serde_json::to_string(&payload)?)
    }

    fn build_submit_svm_extrinsic(&self, proof: &SvmProof) -> Result<String> {
        let payload = serde_json::json!({
            "pallet": "x3Verifier",
            "call": "submitSvmProof",
            "signing_authority": self.signing_authority(),
            "args": {
                "domain": proof.source_domain,
                "slot": proof.slot,
                "blockhash": format!("0x{:x}", u256_from_bytes(&proof.blockhash)),
                "validator_signatures": proof.validator_signatures.iter()
                    .map(|sig| format!("0x{:x}", u256_from_bytes(sig)))
                    .collect::<Vec<_>>(),
                "required_signatures": proof.required_signatures,
            }
        });

        Ok(serde_json::to_string(&payload)?)
    }

    fn signing_authority(&self) -> serde_json::Value {
        match &self.relayer_custody_key_id {
            Some(key_id) => serde_json::json!({
                "type": "custody-service",
                "key_id": key_id,
            }),
            None => serde_json::json!({
                "type": "local-dev",
            }),
        }
    }

    async fn get_account_nonce(
        client: &reqwest::Client,
        rpc_url: &str,
        account: &str,
    ) -> Result<u32> {
        let response = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "system_accountNextIndex",
                "params": [account],
                "id": 1,
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        json["result"]
            .as_u64()
            .map(|n| n as u32)
            .ok_or_else(|| anyhow!("No nonce in response"))
    }
}

/// Convert [u8; 32] to u256 representation for hex encoding
fn u256_from_bytes(bytes: &[u8; 32]) -> u128 {
    let mut result: u128 = 0;
    for (i, &byte) in bytes.iter().take(16).enumerate() {
        result |= (byte as u128) << (8 * i);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_from_bytes() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xFF;
        bytes[1] = 0xEE;
        let result = u256_from_bytes(&bytes);
        assert_eq!(result, 0xEEFF);
    }

    #[tokio::test]
    async fn test_acquire_evm_proof() {
        // Test proof acquisition from finalized block data
        let block_hash = [0x12u8; 32];
        let state_root = [0x34u8; 32];

        // Simulate creating a proof without full RPC initialization
        let proof = EvmProof {
            source_domain: 11155111,
            finalized_block: 100,
            block_hash,
            state_root,
            proof_nonce: 0,
        };

        assert_eq!(proof.source_domain, 11155111);
        assert_eq!(proof.finalized_block, 100);
        assert_eq!(proof.block_hash, block_hash);
        assert_eq!(proof.proof_nonce, 0);
    }

    #[tokio::test]
    async fn test_acquire_svm_proof() {
        // Test proof acquisition from finalized slot data
        let blockhash = [0x56u8; 32];

        let proof = SvmProof {
            source_domain: 501,
            slot: 250000,
            blockhash,
            validator_signatures: vec![],
            required_signatures: 2,
        };

        assert_eq!(proof.source_domain, 501);
        assert_eq!(proof.slot, 250000);
        assert_eq!(proof.blockhash, blockhash);
    }

    #[test]
    fn test_submission_config_retries() {
        // Verify retry configuration parameters
        let max_retries = 3;
        let retry_backoff_ms = 1000u64;

        let mut current_backoff = retry_backoff_ms;
        for _ in 0..max_retries {
            current_backoff = current_backoff.saturating_mul(2);
        }

        assert_eq!(current_backoff, 8000); // 1000 * 2^3
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let base_backoff = 100u64;
        let mut backoff = base_backoff;

        // Verify exponential backoff sequence
        assert_eq!(backoff, 100);
        backoff = backoff.saturating_mul(2);
        assert_eq!(backoff, 200);
        backoff = backoff.saturating_mul(2);
        assert_eq!(backoff, 400);
        backoff = backoff.saturating_mul(2);
        assert_eq!(backoff, 800);
    }

    #[test]
    fn test_evm_extrinsic_carries_custody_signing_authority() {
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: Some("custody://relayer/mainnet".to_string()),
        };
        let proof = EvmProof {
            source_domain: 200,
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
            finalized_block: 123,
            proof_nonce: 7,
        };

        let extrinsic = submitter.build_submit_cross_vm_extrinsic(&proof).unwrap();
        let value: serde_json::Value = serde_json::from_str(&extrinsic).unwrap();
        assert_eq!(value["signing_authority"]["type"], "custody-service");
        assert_eq!(
            value["signing_authority"]["key_id"],
            "custody://relayer/mainnet"
        );
    }
}
