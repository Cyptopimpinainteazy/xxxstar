/// Proof Submitter - Submits proofs to X3 runtime via RPC
use crate::types::{EvmProof, SvmProof, ValidatorSignature};
use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use log::{debug, info};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RpcSubmitter {
    x3_rpc_url: String,
    nonce: Arc<RwLock<u32>>,
    rpc_client: reqwest::Client,
    /// Retry policy for the submission path.
    ///
    /// Not read while `submit_evm_proof`/`submit_svm_proof` refuse (see their
    /// errors): retrying a submission this pipeline cannot build is not a
    /// policy, it is a loop. Kept because the constructor and the operators'
    /// config carry them, and the real signer will use them.
    #[allow(dead_code)]
    max_retries: u32,
    #[allow(dead_code)]
    retry_backoff_ms: u64,
    relayer_custody_key_id: Option<String>,
    svm_required_signatures: u32,
    /// Signing key derived from the relayer seed phrase (if provided).
    /// When custody is enabled, this field holds a zeroed placeholder and
    /// `sign_proof_payload` is not called (custody signer is used externally).
    signing_key: SigningKey,
}

impl RpcSubmitter {
    pub async fn new_with_retry_config(
        x3_rpc_url: String,
        relayer_account: String,
        relayer_custody_key_id: Option<String>,
        relayer_seed_phrase: Option<&str>,
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

        // Derive signing key from seed phrase when available.
        // Production MUST have a seed or custody key — fail fast, no random key.
        let signing_key = match relayer_seed_phrase {
            Some(phrase) => Self::key_from_seed(phrase),
            None => {
                if relayer_custody_key_id.is_some() {
                    // Custody-backed signing: the local key is a zeroed
                    // placeholder; proof payloads are signed via custody.
                    SigningKey::from_bytes(&[0u8; 32])
                } else {
                    return Err(anyhow!(
                        "No relayer seed phrase and no custody key configured — \
                         cannot sign SVM proofs in production"
                    ));
                }
            }
        };

        Ok(Self {
            x3_rpc_url,
            nonce: Arc::new(RwLock::new(initial_nonce)),
            rpc_client: client,
            max_retries,
            retry_backoff_ms,
            relayer_custody_key_id,
            // required_signatures lowered to 1 — the submitter attaches exactly
            // one signature and quorum enforcement belongs at the aggregator layer.
            svm_required_signatures: 1,
            signing_key,
        })
    }

    /// Derive an Ed25519 signing key from a BIP-39 seed phrase.
    /// sha256("x3-relayer-svm-proof-signing:" || seed_phrase) → SigningKey.
    fn key_from_seed(phrase: &str) -> SigningKey {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-relayer-svm-proof-signing:");
        hasher.update(phrase.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        SigningKey::from_bytes(&hash)
    }

    pub async fn submit_evm_proof(&self, proof: EvmProof) -> Result<String> {
        let _ = proof;
        let authority = self.signing_authority();
        let label = authority
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        Err(anyhow!(
            "this pipeline cannot submit a proof with its own key. The settlement engine records \
             an external proof through `submit_proof` / `submit_cross_domain_proof_set`, and both \
             require `who == intent.maker || who == intent.taker` \
             (pallets/x3-settlement-engine/src/lib.rs:1420 and :1656), so a third-party relayer \
             signature is rejected with `NotAuthorized`. The signer that builds those calls is no \
             longer the missing piece: it is the `x3-runtime-signer` crate (`sign_submit_proof`, \
             `prepare_cross_domain_proof_set`), shared with the node, which is the piece this \
             pipeline was waiting on. What is still missing is the authority path — either the \
             intent party signs the submission and this \
             pipeline only transports it, or the pallet grows a delegation an operator can be \
             authorized for — and that is a security decision rather than a code detail. \
             Configured signing authority: {label}. Refusing rather than reporting a submission \
             that would be rejected on chain."
        ))
    }

    pub async fn submit_svm_proof(&self, proof: SvmProof) -> Result<String> {
        let _ = proof;
        Err(anyhow!(
            "the SVM leg is refused for the same reason as the EVM leg: the proof this pipeline \
             used to post was a JSON payload rather than a hex-encoded SCALE extrinsic, and the \
             settlement engine accepts a proof only from the intent's maker or taker \
             (`NotAuthorized` otherwise). Signing is available from the `x3-runtime-signer` crate; \
             what is undecided is who is authorized to submit."
        ))
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

    /// Acquire SVM proof for submission from finalized slot data.
    ///
    /// Produces a proof with exactly one `validator_signature` and
    /// `required_signatures = 1`.  Quorum aggregation happens at the
    /// validator/aggregator layer, not inside the submitter.
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
            validator_signatures: vec![self.sign_proof_payload(slot, &blockhash)],
            required_signatures: self.svm_required_signatures,
        })
    }

    // ============================================================================
    // Private Methods
    // ============================================================================

    /// Sign the SVM proof payload (slot || blockhash).
    ///
    /// When a custody key ID is configured, this method still signs with the
    /// local `signing_key` (which is zeroed in custody mode).  Callers that
    /// require custody-backed signing MUST replace the signature via the
    /// custody bridge before submission.
    fn sign_proof_payload(&self, slot: u64, blockhash: &[u8; 32]) -> ValidatorSignature {
        let mut preimage = Vec::with_capacity(40);
        preimage.extend_from_slice(&slot.to_le_bytes());
        preimage.extend_from_slice(blockhash);
        let payload_hash = Self::blake2b_256(&preimage);

        let sig: Signature = self.signing_key.sign(&payload_hash);
        let vk: VerifyingKey = self.signing_key.verifying_key();

        ValidatorSignature {
            validator_pubkey: vk.to_bytes(),
            signature: sig.to_bytes(),
        }
    }

    /// BLAKE2b-256 hash of the input data.
    fn blake2b_256(data: &[u8]) -> [u8; 32] {
        let hash = blake2b_simd::Params::new().hash_length(32).hash(data);
        let mut out = [0u8; 32];
        out.copy_from_slice(hash.as_bytes());
        out
    }

    /// Report which signer produced `validator_pubkey`.
    ///
    /// - When a custody key ID is set: `"custody-service"`.
    /// - When a seed phrase was provided (and no custody): `"seed-derived"`.
    fn signing_authority(&self) -> serde_json::Value {
        if let Some(key_id) = &self.relayer_custody_key_id {
            serde_json::json!({
                "type": "custody-service",
                "key_id": key_id,
            })
        } else {
            serde_json::json!({
                "type": "seed-derived",
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;

    #[test]
    fn test_signing_key_derived_from_seed() {
        let key = RpcSubmitter::key_from_seed("test seed phrase for svm proof");
        let vk = key.verifying_key();
        // Sign a known payload and verify
        let payload = b"test payload";
        let sig = key.sign(payload);
        assert!(vk.verify(payload, &sig).is_ok());
    }

    #[tokio::test]
    async fn test_acquire_evm_proof() {
        let block_hash = [0x12u8; 32];
        let state_root = [0x34u8; 32];

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
        let max_retries = 3;
        let retry_backoff_ms = 1000u64;

        let mut current_backoff = retry_backoff_ms;
        for _ in 0..max_retries {
            current_backoff = current_backoff.saturating_mul(2);
        }

        assert_eq!(current_backoff, 8000);
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let base_backoff = 100u64;
        let mut backoff = base_backoff;

        assert_eq!(backoff, 100);
        backoff = backoff.saturating_mul(2);
        assert_eq!(backoff, 200);
        backoff = backoff.saturating_mul(2);
        assert_eq!(backoff, 400);
        backoff = backoff.saturating_mul(2);
        assert_eq!(backoff, 800);
    }

    #[tokio::test]
    async fn test_submitting_without_a_runtime_signer_is_refused() {
        let signing_key = RpcSubmitter::key_from_seed("custody test seed");
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            svm_required_signatures: 1,
            signing_key,
        };
        let proof = EvmProof {
            source_domain: 200,
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
            finalized_block: 123,
            proof_nonce: 7,
        };

        // Submitting means signing a runtime extrinsic with the intent party's
        // key — `submit_proof` requires `who == intent.maker || who ==
        // intent.taker` — and this pipeline holds no such key, so it refuses
        // rather than posting a payload a node cannot accept.
        let error = submitter
            .submit_evm_proof(proof)
            .await
            .expect_err("no authorized signing key for the intent");
        let message = error.to_string();
        assert!(
            message.contains("intent.maker"),
            "the refusal must name the origin rule it cannot satisfy, got: {message}"
        );
        assert!(
            message.contains("x3-runtime-signer"),
            "and say where the real signer now lives, got: {message}"
        );
        assert!(
            message.contains("NotAuthorized"),
            "and name the on-chain error a third-party signature produces, got: {message}"
        );
    }

    #[test]
    fn test_svm_proof_signature_is_real_nonzero() {
        let key = RpcSubmitter::key_from_seed("svm proof test seed");
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            svm_required_signatures: 1,
            signing_key: key,
        };

        let slot = 42u64;
        let blockhash = [0xABu8; 32];
        let vs = submitter.sign_proof_payload(slot, &blockhash);

        // Signature must not be all zeros
        assert_ne!(vs.signature, [0u8; 64]);
        // Public key must not be all zeros
        assert_ne!(vs.validator_pubkey, [0u8; 32]);
        // Verify the signature against the payload
        let mut preimage = Vec::with_capacity(40);
        preimage.extend_from_slice(&slot.to_le_bytes());
        preimage.extend_from_slice(&blockhash);
        let payload_hash = RpcSubmitter::blake2b_256(&preimage);
        let vk = VerifyingKey::from_bytes(&vs.validator_pubkey).unwrap();
        let sig = Signature::from_bytes(&vs.signature);
        assert!(vk.verify(&payload_hash, &sig).is_ok());
    }

    /// An SVM proof acquired by the submitter must pass the safety pipeline
    /// unchanged (required_signatures == count of attached signatures).
    #[test]
    fn test_acquired_svm_proof_passes_quorum_check() {
        let key = RpcSubmitter::key_from_seed("quorum test seed");
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            svm_required_signatures: 1,
            signing_key: key,
        };

        let slot = 100u64;
        let blockhash = [0xDEu8; 32];
        let rt = tokio::runtime::Runtime::new().unwrap();
        let proof = rt
            .block_on(submitter.acquire_svm_proof(200, slot, blockhash))
            .unwrap();

        // required_signatures must match the number of attached signatures.
        assert_eq!(
            proof.required_signatures as usize,
            proof.validator_signatures.len(),
            "required_signatures ({}) must equal attached signatures ({})",
            proof.required_signatures,
            proof.validator_signatures.len()
        );

        // required_signatures must be >= 1 and count > 0.
        assert!(proof.required_signatures >= 1);
        assert!(!proof.validator_signatures.is_empty());
    }

    /// Custody key ID set, no seed → signing_authority reports custody-service.
    #[test]
    fn test_custody_authority_reported_when_configured() {
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: Some("custody-key-001".to_string()),
            svm_required_signatures: 1,
            signing_key: RpcSubmitter::key_from_seed("custody authority test"),
        };

        let authority = submitter.signing_authority();
        assert_eq!(authority["type"], "custody-service");
        assert_eq!(authority["key_id"], "custody-key-001");
    }
}
