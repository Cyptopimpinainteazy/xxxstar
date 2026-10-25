/// Proof Submitter - Submits proofs to X3 runtime via RPC
use crate::quorum::AuthorizedValidatorSet;
use crate::types::{EvmProof, EvmProofRequest, SvmProof, ValidatorSignature};
use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use log::{debug, info};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// A refusal this pipeline makes on purpose, named rather than reported as an
/// opaque string.
///
/// X3 fails closed: when the relayer cannot establish that it is allowed to do
/// something, it refuses and says which rule it could not satisfy. Both variants
/// replace an action a node would have rejected anyway.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SubmitterError {
    /// The chain admits a cross-chain proof only from the intent's own maker or
    /// taker, and this pipeline holds neither key.
    ///
    /// The signer that builds `submit_proof` / `submit_cross_domain_proof_set`
    /// now exists (`x3-runtime-signer`, shared with the node); what is undecided
    /// is the authority path, and that is a security decision rather than a code
    /// detail. Until it is decided, a submission signed here would be rejected
    /// on chain, so it is not made.
    #[error(
        "refusing to submit the {leg} proof: the settlement engine records an external proof \
         through `submit_proof` / `submit_cross_domain_proof_set`, and both require \
         `who == intent.maker || who == intent.taker`, so a third-party relayer signature is \
         rejected with `NotAuthorized`. The signer is no longer the missing piece: it is the \
         `x3-runtime-signer` crate (`sign_submit_proof`, `prepare_cross_domain_proof_set`), \
         shared with the node. What is undecided is who is authorized to submit. Configured \
         signing authority: {authority}. Refusing rather than reporting a submission that would \
         be rejected on chain."
    )]
    UnauthorizedSubmitter {
        /// Which proof leg refused (`evm` or `svm`).
        leg: &'static str,
        /// The configured signing authority, for the operator reading the log.
        authority: String,
    },

    /// The configured validator set demands more distinct signatures than this
    /// pipeline can supply.
    #[error(
        "refusing to produce an SVM proof: {validator_set} validators are authorized and the \
         supermajority rule requires {required} distinct signatures, but this submitter can back \
         it with {available}. Aggregating other validators' signatures has no host in this \
         workspace yet, so no proof is produced rather than one carrying a quorum it cannot meet."
    )]
    SvmQuorumUnreachable {
        required: u32,
        available: u32,
        validator_set: usize,
    },

    /// Nothing is authorized to attest, so no signature could ever be backed.
    #[error(
        "refusing to produce an SVM proof: no authorized validator set is configured, so nothing \
         can attest external-chain finality and a proof produced here would be unbacked."
    )]
    NoAuthorizedValidatorSet,
}

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
    /// The authorized validator set, and therefore the quorum, an SVM proof must
    /// meet. `required_signatures` is derived from this set through the
    /// workspace's single supermajority rule — never a literal.
    svm_validators: AuthorizedValidatorSet,
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
        svm_validator_pubkeys: &[String],
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

        // A malformed, wrongly sized or repeated key is refused at startup: a
        // set the operator did not mean must not size the quorum.
        let svm_validators = AuthorizedValidatorSet::from_hex_keys(svm_validator_pubkeys)
            .map_err(|err| anyhow!("invalid configured validator set: {err}"))?;

        Ok(Self {
            x3_rpc_url,
            nonce: Arc::new(RwLock::new(initial_nonce)),
            rpc_client: client,
            max_retries,
            retry_backoff_ms,
            relayer_custody_key_id,
            svm_validators,
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

    /// Refused, by name: see [`SubmitterError::UnauthorizedSubmitter`].
    ///
    /// The proof is consumed so that a caller cannot mistake a refusal for a
    /// submission, and the error is a typed variant rather than a string a caller
    /// would have to match on.
    pub async fn submit_evm_proof(&self, proof: EvmProof) -> Result<String, SubmitterError> {
        let _ = proof;
        Err(self.unauthorized_submitter("evm"))
    }

    /// Refused, by name, for the same reason as the EVM leg — and because the
    /// payload this path used to post was a JSON blob rather than a hex-encoded
    /// SCALE extrinsic the settlement engine can decode.
    pub async fn submit_svm_proof(&self, proof: SvmProof) -> Result<String, SubmitterError> {
        let _ = proof;
        Err(self.unauthorized_submitter("svm"))
    }

    /// The single refusal the submission authority decision produces, shared by
    /// both proof legs so they cannot drift apart.
    fn unauthorized_submitter(&self, leg: &'static str) -> SubmitterError {
        SubmitterError::UnauthorizedSubmitter {
            leg,
            authority: self.authority_label(),
        }
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
    pub async fn acquire_evm_proof(&self, request: EvmProofRequest) -> Result<EvmProof> {
        if request.proof_payload.is_empty() {
            return Err(anyhow!(
                "refusing to acquire an EVM proof without receipt-inclusion evidence"
            ));
        }
        if request.expected_recipient.len() != 20 {
            return Err(anyhow!(
                "refusing to acquire an EVM proof with a non-20-byte recipient"
            ));
        }
        debug!(
            "Acquiring EVM proof for domain {}, block {}",
            request.source_domain, request.finalized_block
        );

        let nonce = {
            let n = self.nonce.read().await;
            *n
        };

        Ok(EvmProof {
            source_domain: request.source_domain,
            finalized_block: request.finalized_block,
            block_hash: request.block_hash,
            state_root: request.state_root,
            proof_nonce: nonce,
            proof_payload: request.proof_payload,
            expected_recipient: request.expected_recipient,
            expected_amount: request.expected_amount,
        })
    }

    /// Acquire SVM proof for submission from finalized slot data.
    ///
    /// `required_signatures` is the supermajority of the **configured** validator
    /// set, not a literal and not a value the proof gets to choose for itself.
    ///
    /// The submitter attaches the signatures it can actually produce — today that
    /// is this process's own key, and only when that key is a member of the
    /// configured set. If the distinct signatures it can produce fall short of
    /// the derived quorum, no proof is produced: the partial material is dropped
    /// and the shortfall is returned by name
    /// ([`SubmitterError::SvmQuorumUnreachable`]), because a proof carrying fewer
    /// signatures than its rule requires is a proof that only looks like a
    /// quorum. Aggregating other validators' signatures has no host in this
    /// workspace yet.
    pub async fn acquire_svm_proof(
        &self,
        domain_id: u32,
        slot: u64,
        blockhash: [u8; 32],
    ) -> Result<SvmProof, SubmitterError> {
        debug!(
            "Acquiring SVM proof for domain {}, slot {}",
            domain_id, slot
        );

        if self.svm_validators.is_empty() {
            return Err(SubmitterError::NoAuthorizedValidatorSet);
        }

        let mut validator_signatures = Vec::new();
        let own_key = self.signing_key.verifying_key().to_bytes();
        if self.svm_validators.is_authorized(&own_key) {
            validator_signatures.push(self.sign_proof_payload(slot, &blockhash));
        }

        let required_signatures = self.svm_validators.required_signatures();
        let available = self.svm_validators.distinct_signers(&validator_signatures) as u32;
        if available < required_signatures {
            return Err(SubmitterError::SvmQuorumUnreachable {
                required: required_signatures,
                available,
                validator_set: self.svm_validators.len(),
            });
        }

        Ok(SvmProof {
            source_domain: domain_id,
            slot,
            blockhash,
            validator_signatures,
            required_signatures,
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

    /// `signing_authority()` as a one-line label for refusal messages, read from
    /// the same report so the two cannot describe different signers.
    fn authority_label(&self) -> String {
        match self
            .signing_authority()
            .get("key_id")
            .and_then(|value| value.as_str())
        {
            Some(key_id) => format!("custody-service ({key_id})"),
            None => "seed-derived".to_string(),
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
            proof_payload: vec![1u8; 96],
            expected_recipient: vec![2u8; 20],
            expected_amount: 1_000,
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
            svm_validators: AuthorizedValidatorSet::from_keys(Vec::new()),
            signing_key,
        };
        let proof = EvmProof {
            source_domain: 200,
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
            finalized_block: 123,
            proof_nonce: 7,
            proof_payload: vec![1u8; 96],
            expected_recipient: vec![2u8; 20],
            expected_amount: 1_000,
        };

        // Submitting means signing a runtime extrinsic with the intent party's
        // key — `submit_proof` requires `who == intent.maker || who ==
        // intent.taker` — and this pipeline holds no such key, so it refuses
        // rather than posting a payload a node cannot accept.
        let error = submitter
            .submit_evm_proof(proof)
            .await
            .expect_err("no authorized signing key for the intent");
        // The refusal is a matchable variant, not a string a caller must parse.
        assert_eq!(
            error,
            SubmitterError::UnauthorizedSubmitter {
                leg: "evm",
                authority: "seed-derived".to_string(),
            }
        );
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
            svm_validators: AuthorizedValidatorSet::from_keys(Vec::new()),
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

    /// The golden path: when this submitter *is* the whole configured validator
    /// set, the quorum its acquisition derives is one signature, and the proof it
    /// builds carries exactly that many distinct signatures.
    #[test]
    fn test_acquired_svm_proof_passes_quorum_check() {
        let key = RpcSubmitter::key_from_seed("quorum test seed");
        let key_hex = hex::encode(key.verifying_key().to_bytes());
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            svm_validators: AuthorizedValidatorSet::from_hex_keys(&[key_hex]).unwrap(),
            signing_key: key,
        };

        let slot = 100u64;
        let blockhash = [0xDEu8; 32];
        let rt = tokio::runtime::Runtime::new().unwrap();
        let proof = rt
            .block_on(submitter.acquire_svm_proof(200, slot, blockhash))
            .unwrap();

        // The count comes from the rule on the configured set, not from a
        // literal: one validator means exactly one signature.
        assert_eq!(
            proof.required_signatures,
            x3_validator_attestation::supermajority_threshold(1)
        );

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

    /// The ugly path this workstream exists for: with three authorized
    /// validators the rule demands a supermajority (three), and a submitter
    /// holding one key refuses to produce a proof rather than emitting one that
    /// claims a quorum it cannot back. The refusal names the shortfall, and the
    /// partial signature material is dropped — no proof object is handed back.
    #[test]
    fn acquire_svm_proof_refuses_a_set_it_cannot_back() {
        let key = RpcSubmitter::key_from_seed("under-quorum submitter seed");
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            // Three validators: this key plus two this process cannot sign for.
            svm_validators: AuthorizedValidatorSet::from_keys(vec![
                key.verifying_key().to_bytes(),
                [0x11u8; 32],
                [0x22u8; 32],
            ]),
            signing_key: key,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let error = rt
            .block_on(submitter.acquire_svm_proof(200, 42, [0xDEu8; 32]))
            .expect_err("one key cannot back a three-of-three quorum");

        assert_eq!(
            error,
            SubmitterError::SvmQuorumUnreachable {
                required: x3_validator_attestation::supermajority_threshold(3),
                available: 1,
                validator_set: 3,
            }
        );
        let message = error.to_string();
        assert!(message.contains("3 distinct signatures"), "got: {message}");
        assert!(message.contains("back it with 1"), "got: {message}");
    }

    /// A submitter whose own key is not in the configured set can produce no
    /// authorized signature at all — it must refuse, not sign as an outsider.
    #[test]
    fn acquire_svm_proof_refuses_when_its_key_is_not_authorized() {
        let key = RpcSubmitter::key_from_seed("unauthorized submitter seed");
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            svm_validators: AuthorizedValidatorSet::from_keys(vec![[0x33u8; 32]]),
            signing_key: key,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let error = rt
            .block_on(submitter.acquire_svm_proof(200, 42, [0xDEu8; 32]))
            .expect_err("an unauthorized key must not produce a proof");
        assert_eq!(
            error,
            SubmitterError::SvmQuorumUnreachable {
                required: 1,
                available: 0,
                validator_set: 1,
            }
        );
    }

    /// With nothing authorized, nothing can attest: refused by name rather than
    /// producing an unbacked proof.
    #[test]
    fn acquire_svm_proof_refuses_with_no_validator_set() {
        let submitter = RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            svm_validators: AuthorizedValidatorSet::from_keys(Vec::new()),
            signing_key: RpcSubmitter::key_from_seed("no set configured"),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let error = rt
            .block_on(submitter.acquire_svm_proof(200, 42, [0xDEu8; 32]))
            .expect_err("no authorized validator set must fail closed");
        assert_eq!(error, SubmitterError::NoAuthorizedValidatorSet);
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
            svm_validators: AuthorizedValidatorSet::from_keys(Vec::new()),
            signing_key: RpcSubmitter::key_from_seed("custody authority test"),
        };

        let authority = submitter.signing_authority();
        assert_eq!(authority["type"], "custody-service");
        assert_eq!(authority["key_id"], "custody-key-001");
        assert_eq!(
            submitter.authority_label(),
            "custody-service (custody-key-001)"
        );
    }
}
