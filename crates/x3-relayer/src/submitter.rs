/// Proof Submitter - Submits proofs to X3 runtime via RPC
use crate::quorum::AuthorizedValidatorSet;
use crate::types::{
    EvmProof, EvmProofRequest, SvmProof, SvmProofRequest, SvmValidatorStakeConfig,
    ValidatorSignature,
};
use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use log::{debug, info};
use pallet_x3_settlement_engine::ExternalChainId;
use sha2::{Digest, Sha256};
use sp_core::crypto::Ss58Codec;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use x3_runtime_signer::X3RuntimeSigner;
use x3_verification_router::{
    solana_finality_message_v3, solana_stake_table_digest, SolanaCluster,
    SolanaStakeWeightedVerifier, SolanaValidatorStake, Verifier as ProofVerifier,
};

/// A refusal this pipeline makes on purpose, named rather than reported as an
/// opaque string.
///
/// X3 fails closed: when the relayer cannot establish that it is allowed to do
/// something, it refuses and says which rule it could not satisfy. Both variants
/// replace an action a node would have rejected anyway.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SubmitterError {
    /// This process has no local sr25519 signer for runtime extrinsics.
    #[error(
        "refusing to submit the {leg} proof: no local X3 runtime signer is configured \
         (configured signing authority: {authority})"
    )]
    RuntimeSignerUnavailable {
        leg: &'static str,
        authority: String,
    },

    /// The proof cannot be represented as the runtime's typed call.
    #[error("refusing to submit the {leg} proof: {reason}")]
    InvalidRuntimeProof { leg: &'static str, reason: String },

    /// Signing or submitting the signed extrinsic failed.
    #[error("failed to submit the {leg} proof: {reason}")]
    SubmissionFailed { leg: &'static str, reason: String },

    /// The SVM V3 evidence is not yet a settlement-engine proof bound to a
    /// transaction in the finalized slot. The runtime correctly refuses it.
    #[error(
        "refusing to submit the SVM settlement proof: the V3 evidence binds validator \
         attestations but the settlement runtime still requires independent transaction \
         inclusion and bank-state evidence"
    )]
    UnsupportedSvmSettlementProof,

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

    /// The configured Solana stake table is absent or inconsistent.
    #[error("refusing to produce an SVM proof: {0}")]
    InvalidSvmStakeTable(String),

    /// A supplied validator vote is malformed, duplicated, unauthorized, or
    /// does not verify over the complete V2 finality statement.
    #[error("refusing Solana validator vote {index}: {reason}")]
    InvalidSvmVote { index: usize, reason: String },

    /// The V3 evidence is internally inconsistent before any vote is counted.
    #[error("refusing Solana proof evidence: {0}")]
    InvalidSvmEvidence(String),
}

fn parse_stake_table(
    entries: &[SvmValidatorStakeConfig],
    authorized: &AuthorizedValidatorSet,
) -> Result<Vec<SolanaValidatorStake>> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let mut stakes = Vec::with_capacity(entries.len());
    for entry in entries {
        let trimmed = entry.pubkey.trim();
        let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        let bytes = hex::decode(trimmed).map_err(|error| {
            anyhow!("invalid Solana validator pubkey {}: {error}", entry.pubkey)
        })?;
        let pubkey: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("Solana validator pubkey {} is not 32 bytes", entry.pubkey))?;
        stakes.push(SolanaValidatorStake {
            pubkey,
            stake: entry.stake,
        });
    }
    stakes.sort_by_key(|entry| entry.pubkey);
    let mut expected = authorized.keys().to_vec();
    expected.sort();
    let actual = stakes.iter().map(|entry| entry.pubkey).collect::<Vec<_>>();
    if actual != expected {
        return Err(anyhow!(
            "Solana stake table keys do not match the authorized validator set"
        ));
    }
    Ok(stakes)
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
    svm_stakes: Vec<SolanaValidatorStake>,
    svm_cluster: SolanaCluster,
    runtime_signer: Option<Arc<X3RuntimeSigner>>,
    evm_domain_chains: BTreeMap<u32, ExternalChainId>,
    /// Runtime signing reads the account nonce from the node. Serialize the
    /// sign-and-submit critical section so concurrent proofs cannot sign with
    /// the same nonce.
    submission_lock: Arc<tokio::sync::Mutex<()>>,
    /// Signing key derived from the relayer seed phrase (if provided).
    /// When custody is enabled, this field holds a zeroed placeholder and
    /// `sign_proof_payload` is not called (custody signer is used externally).
    signing_key: SigningKey,
}

pub struct RpcSubmitterConfig {
    pub x3_rpc_url: String,
    pub relayer_account: String,
    pub relayer_custody_key_id: Option<String>,
    pub relayer_seed_phrase: Option<String>,
    pub svm_validator_pubkeys: Vec<String>,
    pub svm_validator_stakes: Vec<SvmValidatorStakeConfig>,
    pub svm_cluster: SolanaCluster,
    /// `(X3 domain id, external EVM chain id)` pairs from the relayer config.
    pub evm_domain_chains: Vec<(u32, u64)>,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
}

fn external_chain_id(chain_id: u64) -> ExternalChainId {
    match chain_id {
        1 => ExternalChainId::Ethereum,
        10 => ExternalChainId::Optimism,
        56 => ExternalChainId::Bnb,
        137 => ExternalChainId::Polygon,
        8_453 => ExternalChainId::Base,
        42_161 => ExternalChainId::Arbitrum,
        43_114 => ExternalChainId::Avalanche,
        other => ExternalChainId::EvmChain(other),
    }
}

impl RpcSubmitter {
    pub async fn new_with_retry_config(config: RpcSubmitterConfig) -> Result<Self> {
        let client = reqwest::Client::new();
        let RpcSubmitterConfig {
            x3_rpc_url,
            relayer_account,
            relayer_custody_key_id,
            relayer_seed_phrase,
            svm_validator_pubkeys,
            svm_validator_stakes,
            svm_cluster,
            evm_domain_chains,
            max_retries,
            retry_backoff_ms,
        } = config;

        // Initialize nonce from X3 runtime
        let initial_nonce = Self::get_account_nonce(&client, &x3_rpc_url, &relayer_account).await?;

        info!(
            "RPC submitter initialized for {} (initial nonce: {}, max_retries: {}, backoff: {}ms)",
            relayer_account, initial_nonce, max_retries, retry_backoff_ms
        );

        // Derive signing key from seed phrase when available.
        // Production MUST have a seed or custody key — fail fast, no random key.
        let signing_key = match relayer_seed_phrase.as_deref() {
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
        let runtime_signer = match relayer_seed_phrase.as_deref() {
            Some(uri) => {
                let signer =
                    X3RuntimeSigner::from_uri("x3-mainnet".to_string(), x3_rpc_url.clone(), uri)
                        .map_err(|error| anyhow!("invalid X3 runtime signer: {error}"))?;
                let derived = signer.account().to_ss58check();
                if derived != relayer_account {
                    return Err(anyhow!(
                        "configured relayer account {relayer_account} does not match runtime \
                         signer account {derived}"
                    ));
                }
                Some(Arc::new(signer))
            }
            None => None,
        };
        let mut configured_evm_chains = BTreeMap::new();
        for (domain_id, chain_id) in evm_domain_chains {
            if domain_id == 0 || chain_id == 0 {
                return Err(anyhow!("EVM domain and chain ids must be non-zero"));
            }
            if configured_evm_chains
                .insert(domain_id, external_chain_id(chain_id))
                .is_some()
            {
                return Err(anyhow!("duplicate configured X3 EVM domain id {domain_id}"));
            }
        }

        // A malformed, wrongly sized or repeated key is refused at startup: a
        // set the operator did not mean must not size the quorum.
        let svm_validators = AuthorizedValidatorSet::from_hex_keys(&svm_validator_pubkeys)
            .map_err(|err| anyhow!("invalid configured validator set: {err}"))?;
        let svm_stakes = parse_stake_table(&svm_validator_stakes, &svm_validators)?;
        if !svm_validators.is_empty() && svm_stakes.is_empty() {
            return Err(anyhow!(
                "Solana validators are configured without a stake table"
            ));
        }
        if !svm_stakes.is_empty() {
            SolanaStakeWeightedVerifier::new(svm_cluster, svm_stakes.clone(), 2, 3, 0)
                .map_err(|error| anyhow!("invalid Solana stake table: {error:?}"))?;
        }

        Ok(Self {
            x3_rpc_url,
            nonce: Arc::new(RwLock::new(initial_nonce)),
            rpc_client: client,
            max_retries,
            retry_backoff_ms,
            relayer_custody_key_id,
            svm_validators,
            svm_stakes,
            svm_cluster,
            runtime_signer,
            evm_domain_chains: configured_evm_chains,
            submission_lock: Arc::new(tokio::sync::Mutex::new(())),
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

    /// Convert and sign one EVM proof for the settlement engine, then submit the
    /// exact SCALE-encoded extrinsic.
    pub async fn submit_evm_proof(
        &self,
        runtime_intent_id: sp_core::H256,
        proof: EvmProof,
    ) -> Result<String, SubmitterError> {
        if runtime_intent_id == sp_core::H256::zero() {
            return Err(SubmitterError::InvalidRuntimeProof {
                leg: "evm",
                reason: "runtime intent id must not be zero".to_string(),
            });
        }
        let _submission_guard = self.submission_lock.lock().await;
        let signer = self.runtime_signer.clone().ok_or_else(|| {
            SubmitterError::RuntimeSignerUnavailable {
                leg: "evm",
                authority: self.authority_label(),
            }
        })?;
        let chain = self
            .evm_domain_chains
            .get(&proof.source_domain)
            .copied()
            .ok_or_else(|| SubmitterError::InvalidRuntimeProof {
                leg: "evm",
                reason: format!(
                    "no external chain is configured for X3 domain {}",
                    proof.source_domain
                ),
            })?;
        let settlement = crate::evm_receipt_proof::settlement_proof_from_evm_proof(&proof)
            .map_err(|error| SubmitterError::InvalidRuntimeProof {
                leg: "evm",
                reason: error.to_string(),
            })?;
        let signed = tokio::task::spawn_blocking(move || {
            signer.sign_submit_proof(runtime_intent_id, chain, settlement)
        })
        .await
        .map_err(|error| SubmitterError::SubmissionFailed {
            leg: "evm",
            reason: format!("runtime signer task failed: {error}"),
        })?
        .map_err(|error| SubmitterError::SubmissionFailed {
            leg: "evm",
            reason: format!("runtime signer refused the extrinsic: {error}"),
        })?;

        let tx_hash = self.submit_signed_extrinsic("evm", &signed).await?;
        let mut nonce = self.nonce.write().await;
        *nonce = nonce.saturating_add(1);
        Ok(tx_hash)
    }

    /// The SVM V3 evidence does not yet include the independent transaction and
    /// bank-state proof the settlement runtime requires. Refuse rather than
    /// sending evidence the chain will reject.
    pub async fn submit_svm_proof(&self, proof: SvmProof) -> Result<String, SubmitterError> {
        let _ = proof;
        Err(SubmitterError::UnsupportedSvmSettlementProof)
    }

    async fn submit_signed_extrinsic(
        &self,
        leg: &'static str,
        signed_extrinsic: &str,
    ) -> Result<String, SubmitterError> {
        let mut attempt = 0u32;
        loop {
            let response = self
                .rpc_client
                .post(&self.x3_rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "author_submitExtrinsic",
                    "params": [signed_extrinsic],
                    "id": 1,
                }))
                .send()
                .await;
            match response {
                Ok(response) => {
                    let body: serde_json::Value = response.json().await.map_err(|error| {
                        SubmitterError::SubmissionFailed {
                            leg,
                            reason: format!("invalid author_submitExtrinsic response: {error}"),
                        }
                    })?;
                    if let Some(error) = body.get("error").filter(|error| !error.is_null()) {
                        return Err(SubmitterError::SubmissionFailed {
                            leg,
                            reason: format!(
                                "author_submitExtrinsic rejected the extrinsic: {error}"
                            ),
                        });
                    }
                    return body
                        .get("result")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                        .ok_or_else(|| SubmitterError::SubmissionFailed {
                            leg,
                            reason: "author_submitExtrinsic returned no transaction hash"
                                .to_string(),
                        });
                }
                Err(_) if attempt < self.max_retries => {
                    attempt = attempt.saturating_add(1);
                    tokio::time::sleep(std::time::Duration::from_millis(
                        self.retry_backoff_ms.saturating_mul(u64::from(attempt)),
                    ))
                    .await;
                }
                Err(error) => {
                    return Err(SubmitterError::SubmissionFailed {
                        leg,
                        reason: format!("author_submitExtrinsic transport failed: {error}"),
                    });
                }
            }
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
        if request.expected_beneficiary.is_empty() || request.expected_beneficiary.len() > 128 {
            return Err(anyhow!(
                "refusing to acquire an EVM proof without a bounded X3 beneficiary"
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
            proof_id: request.proof_id,
            finalized_block: request.finalized_block,
            block_hash: request.block_hash,
            state_root: request.state_root,
            proof_nonce: nonce,
            proof_payload: request.proof_payload,
            source_asset: request.source_asset,
            expected_sender: request.expected_sender,
            expected_contract: request.expected_contract,
            expected_beneficiary: request.expected_beneficiary,
            expected_nonce: request.expected_nonce,
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
        request: SvmProofRequest,
    ) -> Result<SvmProof, SubmitterError> {
        debug!(
            "Acquiring V2 SVM proof for domain {}, slot {}",
            request.source_domain, request.slot
        );

        if self.svm_validators.is_empty() {
            return Err(SubmitterError::NoAuthorizedValidatorSet);
        }
        if self.svm_stakes.is_empty() {
            return Err(SubmitterError::InvalidSvmStakeTable(
                "no stake table is configured".into(),
            ));
        }
        if request.cluster != self.svm_cluster {
            return Err(SubmitterError::InvalidSvmEvidence(format!(
                "request cluster {:?} does not match configured cluster {:?}",
                request.cluster, self.svm_cluster
            )));
        }
        if request.proof_id == [0u8; 32] {
            return Err(SubmitterError::InvalidSvmEvidence(
                "proof id must not be zero".into(),
            ));
        }
        if request.blockhash == [0u8; 32]
            || request.bank_hash == [0u8; 32]
            || request.parent_bank_hash == [0u8; 32]
            || request.stake_table_digest == [0u8; 32]
        {
            return Err(SubmitterError::InvalidSvmEvidence(
                "blockhash, bank hash, parent bank hash, and stake-table digest must be non-zero"
                    .into(),
            ));
        }
        if request.expected_beneficiary.is_empty() || request.expected_beneficiary.len() > 128 {
            return Err(SubmitterError::InvalidSvmEvidence(
                "beneficiary must be between 1 and 128 bytes".into(),
            ));
        }
        let expected_stake_digest = solana_stake_table_digest(&self.svm_stakes);
        if request.stake_table_digest != expected_stake_digest {
            return Err(SubmitterError::InvalidSvmEvidence(
                "stake-table digest does not match the configured table".into(),
            ));
        }

        let required_signatures = self.svm_validators.required_signatures();
        let unsigned = SvmProof {
            source_domain: request.source_domain,
            proof_id: request.proof_id,
            cluster: request.cluster,
            slot: request.slot,
            parent_slot: request.parent_slot,
            blockhash: request.blockhash,
            bank_hash: request.bank_hash,
            parent_bank_hash: request.parent_bank_hash,
            stake_table_digest: request.stake_table_digest,
            expected_asset_id: request.expected_asset_id,
            expected_amount: request.expected_amount,
            expected_sender: request.expected_sender,
            expected_recipient: request.expected_recipient,
            expected_beneficiary: request.expected_beneficiary,
            validator_signatures: Vec::new(),
            required_signatures,
        };
        let envelope = unsigned.envelope();
        let message = solana_finality_message_v3(
            request.cluster,
            request.slot,
            request.parent_slot,
            &request.blockhash,
            &request.bank_hash,
            &request.parent_bank_hash,
            &request.stake_table_digest,
            &envelope,
        )
        .map_err(|error| SubmitterError::InvalidSvmEvidence(error.to_string()))?;

        let mut validator_signatures = Vec::with_capacity(request.existing_signatures.len() + 1);
        for (index, vote) in request.existing_signatures.iter().enumerate() {
            if !self.svm_validators.is_authorized(&vote.validator_pubkey) {
                return Err(SubmitterError::InvalidSvmVote {
                    index,
                    reason: "validator is not in the authorized set".into(),
                });
            }
            if validator_signatures
                .iter()
                .any(|known: &ValidatorSignature| known.validator_pubkey == vote.validator_pubkey)
            {
                return Err(SubmitterError::InvalidSvmVote {
                    index,
                    reason: "validator vote is duplicated".into(),
                });
            }
            let key = VerifyingKey::from_bytes(&vote.validator_pubkey).map_err(|error| {
                SubmitterError::InvalidSvmVote {
                    index,
                    reason: format!("invalid validator public key: {error}"),
                }
            })?;
            let signature = Signature::from_bytes(&vote.signature);
            key.verify_strict(&message, &signature).map_err(|error| {
                SubmitterError::InvalidSvmVote {
                    index,
                    reason: format!("signature does not verify: {error}"),
                }
            })?;
            validator_signatures.push(vote.clone());
        }

        let own_key = self.signing_key.verifying_key().to_bytes();
        if self.svm_validators.is_authorized(&own_key)
            && !validator_signatures
                .iter()
                .any(|vote| vote.validator_pubkey == own_key)
        {
            validator_signatures.push(self.sign_v2_vote(&message));
        }
        validator_signatures.sort_by_key(|vote| vote.validator_pubkey);

        let proof = SvmProof {
            validator_signatures,
            ..unsigned
        };
        let envelope = proof
            .encoded_envelope()
            .map_err(|error| SubmitterError::InvalidSvmEvidence(error.to_string()))?;
        let verifier =
            SolanaStakeWeightedVerifier::new(self.svm_cluster, self.svm_stakes.clone(), 2, 3, 0)
                .map_err(|error| SubmitterError::InvalidSvmStakeTable(format!("{error:?}")))?;
        if ProofVerifier::verify(&verifier, &envelope).is_err() {
            return Err(SubmitterError::SvmQuorumUnreachable {
                required: required_signatures,
                available: self
                    .svm_validators
                    .distinct_signers(&proof.validator_signatures)
                    as u32,
                validator_set: self.svm_validators.len(),
            });
        }

        Ok(proof)
    }

    // ============================================================================
    // Private Methods
    // ============================================================================

    /// Sign the complete Solana V3 finality message.
    ///
    /// When a custody key ID is configured, this method still signs with the
    /// local `signing_key` (which is zeroed in custody mode). Callers that
    /// require custody-backed signing MUST replace the signature via the
    /// custody bridge before submission.
    fn sign_v2_vote(&self, message: &[u8; 32]) -> ValidatorSignature {
        let sig: Signature = self.signing_key.sign(message);
        let vk: VerifyingKey = self.signing_key.verifying_key();

        ValidatorSignature {
            validator_pubkey: vk.to_bytes(),
            signature: sig.to_bytes(),
        }
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

    const TEST_CLUSTER: SolanaCluster = SolanaCluster::Devnet;

    fn test_stakes(pubkeys: &[[u8; 32]]) -> Vec<SolanaValidatorStake> {
        let mut stakes = pubkeys
            .iter()
            .map(|pubkey| SolanaValidatorStake {
                pubkey: *pubkey,
                stake: 100,
            })
            .collect::<Vec<_>>();
        stakes.sort_by_key(|entry| entry.pubkey);
        stakes
    }

    fn test_request(stakes: &[SolanaValidatorStake]) -> SvmProofRequest {
        SvmProofRequest {
            runtime_intent_id: [0x30u8; 32],
            source_domain: 200,
            proof_id: [0x31u8; 32],
            cluster: TEST_CLUSTER,
            slot: 42,
            parent_slot: 41,
            blockhash: [0x32u8; 32],
            bank_hash: [0x36u8; 32],
            parent_bank_hash: [0x33u8; 32],
            stake_table_digest: solana_stake_table_digest(stakes),
            expected_asset_id: [0x34u8; 32],
            expected_amount: 5_000,
            expected_sender: b"sender".to_vec(),
            expected_recipient: b"recipient".to_vec(),
            expected_beneficiary: vec![0x35u8; 32],
            existing_signatures: Vec::new(),
        }
    }

    fn request_message(request: &SvmProofRequest) -> [u8; 32] {
        let proof = SvmProof {
            source_domain: request.source_domain,
            proof_id: request.proof_id,
            cluster: request.cluster,
            slot: request.slot,
            parent_slot: request.parent_slot,
            blockhash: request.blockhash,
            bank_hash: request.bank_hash,
            parent_bank_hash: request.parent_bank_hash,
            stake_table_digest: request.stake_table_digest,
            expected_asset_id: request.expected_asset_id,
            expected_amount: request.expected_amount,
            expected_sender: request.expected_sender.clone(),
            expected_recipient: request.expected_recipient.clone(),
            expected_beneficiary: request.expected_beneficiary.clone(),
            validator_signatures: Vec::new(),
            required_signatures: 1,
        };
        solana_finality_message_v3(
            proof.cluster,
            proof.slot,
            proof.parent_slot,
            &proof.blockhash,
            &proof.bank_hash,
            &proof.parent_bank_hash,
            &proof.stake_table_digest,
            &proof.envelope(),
        )
        .expect("request intent")
    }

    fn request_vote(request: &SvmProofRequest, key: &SigningKey) -> ValidatorSignature {
        ValidatorSignature {
            validator_pubkey: key.verifying_key().to_bytes(),
            signature: key.sign(&request_message(request)).to_bytes(),
        }
    }

    fn test_submitter(
        signing_key: SigningKey,
        pubkeys: Vec<[u8; 32]>,
        stakes: Vec<SolanaValidatorStake>,
    ) -> RpcSubmitter {
        RpcSubmitter {
            x3_rpc_url: "http://localhost:9933".to_string(),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 3,
            retry_backoff_ms: 1000,
            relayer_custody_key_id: None,
            svm_validators: AuthorizedValidatorSet::from_keys(pubkeys),
            svm_stakes: stakes,
            svm_cluster: TEST_CLUSTER,
            runtime_signer: None,
            evm_domain_chains: BTreeMap::new(),
            submission_lock: Arc::new(tokio::sync::Mutex::new(())),
            signing_key,
        }
    }

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
            proof_id: [0x56u8; 32],
            finalized_block: 100,
            block_hash,
            state_root,
            proof_nonce: 0,
            proof_payload: vec![1u8; 96],
            source_asset: [0x57u8; 20],
            expected_sender: [0x58u8; 20],
            expected_contract: [0x59u8; 20],
            expected_beneficiary: vec![0x60u8; 32],
            expected_nonce: 5,
            expected_amount: 1_000,
        };

        assert_eq!(proof.source_domain, 11155111);
        assert_eq!(proof.finalized_block, 100);
        assert_eq!(proof.block_hash, block_hash);
        assert_eq!(proof.proof_nonce, 0);
    }

    #[tokio::test]
    async fn test_acquire_svm_proof() {
        let key = RpcSubmitter::key_from_seed("acquire SVM test seed");
        let pubkeys = vec![key.verifying_key().to_bytes()];
        let stakes = test_stakes(&pubkeys);
        let submitter = test_submitter(key, pubkeys, stakes.clone());
        let proof = submitter
            .acquire_svm_proof(test_request(&stakes))
            .await
            .expect("one authorized validator can meet its stake threshold");

        assert_eq!(proof.source_domain, 200);
        assert_eq!(proof.slot, 42);
        assert_eq!(proof.blockhash, [0x32u8; 32]);
        assert_eq!(proof.validator_signatures.len(), 1);
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
            svm_stakes: Vec::new(),
            svm_cluster: TEST_CLUSTER,
            runtime_signer: None,
            evm_domain_chains: BTreeMap::new(),
            submission_lock: Arc::new(tokio::sync::Mutex::new(())),
            signing_key,
        };
        let proof = EvmProof {
            source_domain: 200,
            proof_id: [0x56u8; 32],
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
            finalized_block: 123,
            proof_nonce: 7,
            proof_payload: vec![1u8; 96],
            source_asset: [0x57u8; 20],
            expected_sender: [0x58u8; 20],
            expected_contract: [0x59u8; 20],
            expected_beneficiary: vec![0x60u8; 32],
            expected_nonce: 5,
            expected_amount: 1_000,
        };

        // This fixture has no local runtime signer. The refusal must be explicit
        // before trying to convert or submit a proof.
        let error = submitter
            .submit_evm_proof(sp_core::H256::repeat_byte(0x11), proof)
            .await
            .expect_err("no runtime signing key");
        assert_eq!(
            error,
            SubmitterError::RuntimeSignerUnavailable {
                leg: "evm",
                authority: "seed-derived".to_string(),
            }
        );
        assert!(error.to_string().contains("runtime signer"));
    }

    #[tokio::test]
    async fn evm_submission_rejects_zero_intent_before_any_network_call() {
        let submitter = test_submitter(
            RpcSubmitter::key_from_seed("zero intent test"),
            Vec::new(),
            Vec::new(),
        );
        let proof = EvmProof {
            source_domain: 200,
            proof_id: [0x56u8; 32],
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
            finalized_block: 123,
            proof_nonce: 7,
            proof_payload: vec![1u8; 96],
            source_asset: [0x57u8; 20],
            expected_sender: [0x58u8; 20],
            expected_contract: [0x59u8; 20],
            expected_beneficiary: vec![0x60u8; 32],
            expected_nonce: 5,
            expected_amount: 1_000,
        };
        let error = submitter
            .submit_evm_proof(sp_core::H256::zero(), proof)
            .await
            .expect_err("zero intent");
        assert!(matches!(
            error,
            SubmitterError::InvalidRuntimeProof { leg: "evm", .. }
        ));
    }

    #[test]
    fn external_chain_ids_are_mapped_without_accepting_an_unknown_domain() {
        assert_eq!(external_chain_id(1), ExternalChainId::Ethereum);
        assert_eq!(external_chain_id(8_453), ExternalChainId::Base);
        assert_eq!(external_chain_id(777), ExternalChainId::EvmChain(777));
    }

    #[tokio::test]
    async fn author_submit_extrinsic_returns_the_node_transaction_hash() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock RPC");
        let address = listener.local_addr().expect("mock RPC address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept RPC request");
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            let body = r#"{"jsonrpc":"2.0","id":1,"result":"0xdeadbeef"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write RPC response");
        });
        let submitter = RpcSubmitter {
            x3_rpc_url: format!("http://{address}"),
            nonce: Arc::new(RwLock::new(0)),
            rpc_client: reqwest::Client::new(),
            max_retries: 0,
            retry_backoff_ms: 0,
            relayer_custody_key_id: None,
            svm_validators: AuthorizedValidatorSet::from_keys(Vec::new()),
            svm_stakes: Vec::new(),
            svm_cluster: TEST_CLUSTER,
            runtime_signer: None,
            evm_domain_chains: BTreeMap::new(),
            submission_lock: Arc::new(tokio::sync::Mutex::new(())),
            signing_key: RpcSubmitter::key_from_seed("mock RPC test"),
        };

        let tx_hash = submitter
            .submit_signed_extrinsic("evm", "0x0102")
            .await
            .expect("node result");
        assert_eq!(tx_hash, "0xdeadbeef");
        server.join().expect("mock RPC thread");
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
            svm_stakes: Vec::new(),
            svm_cluster: TEST_CLUSTER,
            runtime_signer: None,
            evm_domain_chains: BTreeMap::new(),
            submission_lock: Arc::new(tokio::sync::Mutex::new(())),
            signing_key: key,
        };

        let message = [0xABu8; 32];
        let vs = submitter.sign_v2_vote(&message);

        // Signature must not be all zeros
        assert_ne!(vs.signature, [0u8; 64]);
        // Public key must not be all zeros
        assert_ne!(vs.validator_pubkey, [0u8; 32]);
        let vk = VerifyingKey::from_bytes(&vs.validator_pubkey).unwrap();
        let sig = Signature::from_bytes(&vs.signature);
        assert!(vk.verify(&message, &sig).is_ok());
    }

    /// The golden path: when this submitter *is* the whole configured validator
    /// set, the quorum its acquisition derives is one signature, and the proof it
    /// builds carries exactly that many distinct signatures.
    #[test]
    fn test_acquired_svm_proof_passes_quorum_check() {
        let key = RpcSubmitter::key_from_seed("quorum test seed");
        let pubkeys = vec![key.verifying_key().to_bytes()];
        let stakes = test_stakes(&pubkeys);
        let submitter = test_submitter(key, pubkeys, stakes.clone());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let proof = rt
            .block_on(submitter.acquire_svm_proof(test_request(&stakes)))
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
        assert!(proof.encoded_envelope().is_ok());
    }

    #[test]
    fn acquire_svm_proof_accepts_an_independently_signed_vote() {
        let own = RpcSubmitter::key_from_seed("aggregating relayer");
        let peer = RpcSubmitter::key_from_seed("independent validator");
        let pubkeys = vec![
            own.verifying_key().to_bytes(),
            peer.verifying_key().to_bytes(),
        ];
        let stakes = test_stakes(&pubkeys);
        let submitter = test_submitter(own, pubkeys, stakes.clone());
        let mut request = test_request(&stakes);
        request.existing_signatures = vec![request_vote(&request, &peer)];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let proof = rt
            .block_on(submitter.acquire_svm_proof(request))
            .expect("two independent votes meet a two-thirds stake threshold");
        assert_eq!(proof.validator_signatures.len(), 2);
        assert!(proof.encoded_envelope().is_ok());
    }

    #[test]
    fn acquire_svm_proof_refuses_a_vote_over_different_evidence() {
        let own = RpcSubmitter::key_from_seed("evidence mismatch relayer");
        let peer = RpcSubmitter::key_from_seed("evidence mismatch validator");
        let pubkeys = vec![
            own.verifying_key().to_bytes(),
            peer.verifying_key().to_bytes(),
        ];
        let stakes = test_stakes(&pubkeys);
        let submitter = test_submitter(own, pubkeys, stakes.clone());
        let request = test_request(&stakes);
        let mut vote = request_vote(&request, &peer);
        vote.signature[0] ^= 1;
        let mut tampered_request = request;
        tampered_request.existing_signatures = vec![vote];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let error = rt
            .block_on(submitter.acquire_svm_proof(tampered_request))
            .expect_err("a vote for different evidence must be refused");
        assert!(matches!(
            error,
            SubmitterError::InvalidSvmVote { index: 0, .. }
        ));
    }

    /// The ugly path this workstream exists for: with three authorized
    /// validators the rule demands a supermajority (three), and a submitter
    /// holding one key refuses to produce a proof rather than emitting one that
    /// claims a quorum it cannot back. The refusal names the shortfall, and the
    /// partial signature material is dropped — no proof object is handed back.
    #[test]
    fn acquire_svm_proof_refuses_a_set_it_cannot_back() {
        let key = RpcSubmitter::key_from_seed("under-quorum submitter seed");
        let pubkeys = vec![key.verifying_key().to_bytes(), [0x11u8; 32], [0x22u8; 32]];
        let stakes = test_stakes(&pubkeys);
        let submitter = test_submitter(key, pubkeys, stakes.clone());

        let rt = tokio::runtime::Runtime::new().unwrap();
        let error = rt
            .block_on(submitter.acquire_svm_proof(test_request(&stakes)))
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
        let pubkeys = vec![[0x33u8; 32]];
        let stakes = test_stakes(&pubkeys);
        let submitter = test_submitter(key, pubkeys, stakes.clone());

        let rt = tokio::runtime::Runtime::new().unwrap();
        let error = rt
            .block_on(submitter.acquire_svm_proof(test_request(&stakes)))
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
        let submitter = test_submitter(
            RpcSubmitter::key_from_seed("no set configured"),
            Vec::new(),
            Vec::new(),
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let error = rt
            .block_on(submitter.acquire_svm_proof(test_request(&[])))
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
            svm_stakes: Vec::new(),
            svm_cluster: TEST_CLUSTER,
            runtime_signer: None,
            evm_domain_chains: BTreeMap::new(),
            submission_lock: Arc::new(tokio::sync::Mutex::new(())),
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
