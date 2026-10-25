//! Durable exchange for independently signed Solana finality votes.
//!
//! The exchange is deliberately an evidence exchange, not a trust anchor:
//! every submission carries a complete V4 evidence package and an Ed25519
//! signature over that package. The exchange checks the configured validator
//! set, rejects malformed or unauthorized signatures, records equivocation, and
//! persists accepted votes with atomic file replacement.

use crate::{AuthorizedValidatorSet, SvmProofRequest, ValidatorSignature};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use codec::{Decode, Encode};
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use x3_verification_router::{
    solana_finality_message_v4, ChainKind, ProofEnvelope, SolanaCluster, SolanaTransactionEvidence,
    VerificationStrategy, MAX_SOLANA_SETTLEMENT_FIELD_BYTES,
};

const MAX_VOTE_BODY_BYTES: usize = 64 * 1024;

/// Complete evidence a validator signs before it is eligible for aggregation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub struct SolanaVoteEvidence {
    pub runtime_intent_id: [u8; 32],
    pub source_domain: u32,
    pub proof_id: [u8; 32],
    pub cluster: u8,
    pub slot: u64,
    pub parent_slot: u64,
    pub blockhash: [u8; 32],
    pub bank_hash: [u8; 32],
    pub parent_bank_hash: [u8; 32],
    pub stake_table_digest: [u8; 32],
    pub expected_asset_id: [u8; 32],
    pub expected_amount: u128,
    pub expected_sender: Vec<u8>,
    pub expected_recipient: Vec<u8>,
    pub expected_beneficiary: Vec<u8>,
    pub transaction_slot: u64,
    pub transaction_index: u32,
    pub transaction_succeeded: bool,
    pub raw_transaction: Vec<u8>,
}

impl SolanaVoteEvidence {
    /// Build exchange evidence from a relayer proof request.
    pub fn from_request(request: &SvmProofRequest) -> Result<Self, VoteExchangeError> {
        if request.runtime_intent_id == [0u8; 32] {
            return Err(VoteExchangeError::InvalidEvidence(
                "runtime intent id must not be zero".into(),
            ));
        }
        if request.proof_id != request.runtime_intent_id {
            return Err(VoteExchangeError::InvalidEvidence(
                "signed proof id must equal the runtime intent id".into(),
            ));
        }
        if request.transaction.slot != request.slot {
            return Err(VoteExchangeError::InvalidEvidence(
                "transaction slot does not match the finalized slot".into(),
            ));
        }
        let cluster = match request.cluster {
            SolanaCluster::MainnetBeta => 1,
            SolanaCluster::Testnet => 2,
            SolanaCluster::Devnet => 3,
        };
        let evidence = Self {
            runtime_intent_id: request.runtime_intent_id,
            source_domain: request.source_domain,
            proof_id: request.proof_id,
            cluster,
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
            transaction_slot: request.transaction.slot,
            transaction_index: request.transaction.index,
            transaction_succeeded: request.transaction.succeeded,
            raw_transaction: request.transaction.raw_transaction.clone(),
        };
        evidence.validate()?;
        Ok(evidence)
    }

    /// Reconstruct the proof request after aggregating signatures.
    pub fn into_request(
        &self,
        existing_signatures: Vec<ValidatorSignature>,
    ) -> Result<SvmProofRequest, VoteExchangeError> {
        self.validate()?;
        let cluster = self.cluster()?;
        let transaction = SolanaTransactionEvidence::from_raw(
            self.transaction_slot,
            self.transaction_index,
            self.transaction_succeeded,
            self.raw_transaction.clone(),
        )
        .map_err(|error| VoteExchangeError::InvalidEvidence(error.to_string()))?;
        Ok(SvmProofRequest {
            runtime_intent_id: self.runtime_intent_id,
            source_domain: self.source_domain,
            proof_id: self.proof_id,
            cluster,
            slot: self.slot,
            parent_slot: self.parent_slot,
            blockhash: self.blockhash,
            bank_hash: self.bank_hash,
            parent_bank_hash: self.parent_bank_hash,
            stake_table_digest: self.stake_table_digest,
            expected_asset_id: self.expected_asset_id,
            expected_amount: self.expected_amount,
            expected_sender: self.expected_sender.clone(),
            expected_recipient: self.expected_recipient.clone(),
            expected_beneficiary: self.expected_beneficiary.clone(),
            transaction,
            existing_signatures,
        })
    }

    /// The exact V4 message validators sign.
    pub fn message(&self) -> Result<[u8; 32], VoteExchangeError> {
        self.validate()?;
        let transaction = SolanaTransactionEvidence::from_raw(
            self.transaction_slot,
            self.transaction_index,
            self.transaction_succeeded,
            self.raw_transaction.clone(),
        )
        .map_err(|error| VoteExchangeError::InvalidEvidence(error.to_string()))?;
        let envelope = ProofEnvelope {
            proof_id: self.proof_id,
            strategy: VerificationStrategy::SolanaFinalizedProof,
            source_chain: ChainKind::Solana,
            destination_chain: ChainKind::X3,
            payload: Vec::new(),
            expected_asset_id: self.expected_asset_id,
            expected_amount: self.expected_amount,
            expected_sender: self.expected_sender.clone(),
            expected_recipient: self.expected_recipient.clone(),
            expected_beneficiary: self.expected_beneficiary.clone(),
        };
        solana_finality_message_v4(
            self.cluster()?,
            self.slot,
            self.parent_slot,
            &self.blockhash,
            &self.bank_hash,
            &self.parent_bank_hash,
            &self.stake_table_digest,
            &transaction,
            &envelope,
        )
        .map_err(|error| VoteExchangeError::InvalidEvidence(error.to_string()))
    }

    /// Stable digest used to identify one exact evidence package.
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"X3/SOLANA_VOTE_EVIDENCE/V1");
        hasher.update(self.encode());
        hasher.finalize().into()
    }

    fn cluster(&self) -> Result<SolanaCluster, VoteExchangeError> {
        match self.cluster {
            1 => Ok(SolanaCluster::MainnetBeta),
            2 => Ok(SolanaCluster::Testnet),
            3 => Ok(SolanaCluster::Devnet),
            _ => Err(VoteExchangeError::InvalidEvidence(
                "unknown Solana cluster".into(),
            )),
        }
    }

    fn validate(&self) -> Result<(), VoteExchangeError> {
        if self.runtime_intent_id == [0u8; 32]
            || self.runtime_intent_id != self.proof_id
            || self.blockhash == [0u8; 32]
            || self.bank_hash == [0u8; 32]
            || self.parent_bank_hash == [0u8; 32]
            || self.stake_table_digest == [0u8; 32]
        {
            return Err(VoteExchangeError::InvalidEvidence(
                "zero or mismatched evidence identity".into(),
            ));
        }
        if self.parent_slot >= self.slot || self.transaction_slot != self.slot {
            return Err(VoteExchangeError::InvalidEvidence(
                "invalid finalized or transaction slot binding".into(),
            ));
        }
        if self.raw_transaction.is_empty()
            || self.expected_sender.len() > MAX_SOLANA_SETTLEMENT_FIELD_BYTES
            || self.expected_recipient.len() > MAX_SOLANA_SETTLEMENT_FIELD_BYTES
            || self.expected_beneficiary.is_empty()
            || self.expected_beneficiary.len() > MAX_SOLANA_SETTLEMENT_FIELD_BYTES
        {
            return Err(VoteExchangeError::InvalidEvidence(
                "transaction or intent field is outside its bound".into(),
            ));
        }
        self.cluster()?;
        self.message_without_validate()?;
        Ok(())
    }

    fn message_without_validate(&self) -> Result<[u8; 32], VoteExchangeError> {
        let transaction = SolanaTransactionEvidence::from_raw(
            self.transaction_slot,
            self.transaction_index,
            self.transaction_succeeded,
            self.raw_transaction.clone(),
        )
        .map_err(|error| VoteExchangeError::InvalidEvidence(error.to_string()))?;
        let envelope = ProofEnvelope {
            proof_id: self.proof_id,
            strategy: VerificationStrategy::SolanaFinalizedProof,
            source_chain: ChainKind::Solana,
            destination_chain: ChainKind::X3,
            payload: Vec::new(),
            expected_asset_id: self.expected_asset_id,
            expected_amount: self.expected_amount,
            expected_sender: self.expected_sender.clone(),
            expected_recipient: self.expected_recipient.clone(),
            expected_beneficiary: self.expected_beneficiary.clone(),
        };
        solana_finality_message_v4(
            self.cluster()?,
            self.slot,
            self.parent_slot,
            &self.blockhash,
            &self.bank_hash,
            &self.parent_bank_hash,
            &self.stake_table_digest,
            &transaction,
            &envelope,
        )
        .map_err(|error| VoteExchangeError::InvalidEvidence(error.to_string()))
    }
}

/// HTTP submission accepted by the exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaVoteSubmission {
    pub evidence: SolanaVoteEvidence,
    pub validator_pubkey: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
}

/// A persisted vote returned to an aggregator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteRecordWire {
    pub validator_pubkey: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
}

/// Result of publishing a vote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotePublishResponse {
    pub accepted: bool,
    pub validator_count: usize,
    pub required_signatures: u32,
    pub evidence_digest: String,
}

/// Votes for one exact evidence package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotesResponse {
    pub proof_id: String,
    pub evidence_digest: String,
    pub required_signatures: u32,
    pub votes: Vec<VoteRecordWire>,
}

/// Durable vote exchange errors.
#[derive(Debug, Error)]
pub enum VoteExchangeError {
    #[error("invalid vote evidence: {0}")]
    InvalidEvidence(String),
    #[error("validator {0} is not authorized")]
    UnauthorizedValidator(String),
    #[error("validator signature is malformed or does not verify")]
    InvalidSignature,
    #[error("validator {validator} has equivocated on proof {proof_id}")]
    Equivocation { validator: String, proof_id: String },
    #[error("proof {proof_id} already has different evidence")]
    EvidenceConflict { proof_id: String },
    #[error("vote store error: {0}")]
    Store(String),
}

fn hex_id(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct PersistedState {
    proofs: Vec<PersistedProof>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PersistedProof {
    evidence: SolanaVoteEvidence,
    evidence_digest: [u8; 32],
    votes: Vec<PersistedVote>,
    equivocated: Vec<[u8; 32]>,
    #[serde(default)]
    observations: Vec<PersistedObservation>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PersistedVote {
    validator_pubkey: [u8; 32],
    #[serde(with = "serde_bytes")]
    signature: [u8; 64],
}

#[derive(Clone, Serialize, Deserialize)]
struct PersistedObservation {
    validator_pubkey: [u8; 32],
    evidence_digests: Vec<[u8; 32]>,
}

#[derive(Clone)]
struct ProofRecord {
    evidence: SolanaVoteEvidence,
    evidence_digest: [u8; 32],
    votes: BTreeMap<[u8; 32], [u8; 64]>,
    equivocated: BTreeSet<[u8; 32]>,
    observations: BTreeMap<[u8; 32], BTreeSet<[u8; 32]>>,
}

#[derive(Clone, Default)]
struct ExchangeState {
    proofs: BTreeMap<[u8; 32], ProofRecord>,
}

/// A durable, validator-authenticated exchange for Solana V4 votes.
#[derive(Clone)]
pub struct VoteExchange {
    path: PathBuf,
    validators: AuthorizedValidatorSet,
    state: Arc<RwLock<ExchangeState>>,
}

impl VoteExchange {
    /// Load accepted votes from `path` and refuse any record that does not verify.
    pub async fn open(
        path: impl Into<PathBuf>,
        validators: AuthorizedValidatorSet,
    ) -> Result<Self, VoteExchangeError> {
        let path = path.into();
        let persisted = match tokio::fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice::<PersistedState>(&bytes)
                .map_err(|error| VoteExchangeError::Store(error.to_string()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => PersistedState::default(),
            Err(error) => return Err(VoteExchangeError::Store(error.to_string())),
        };
        let mut state = ExchangeState::default();
        for proof in persisted.proofs {
            proof.evidence.validate()?;
            let digest = proof.evidence.digest();
            if digest != proof.evidence_digest {
                return Err(VoteExchangeError::Store(
                    "persisted evidence digest does not match its evidence".into(),
                ));
            }
            let message = proof.evidence.message()?;
            let mut votes = BTreeMap::new();
            for vote in proof.votes {
                if !validators.is_authorized(&vote.validator_pubkey) {
                    return Err(VoteExchangeError::UnauthorizedValidator(hex_id(
                        &vote.validator_pubkey,
                    )));
                }
                if !verify_vote(&vote.validator_pubkey, &vote.signature, &message) {
                    return Err(VoteExchangeError::InvalidSignature);
                }
                votes.insert(vote.validator_pubkey, vote.signature);
            }
            let equivocated = proof.equivocated.into_iter().collect::<BTreeSet<_>>();
            if equivocated.iter().any(|key| !validators.is_authorized(key)) {
                return Err(VoteExchangeError::Store(
                    "persisted equivocation names an unauthorized validator".into(),
                ));
            }
            let mut observations: BTreeMap<[u8; 32], BTreeSet<[u8; 32]>> = BTreeMap::new();
            for observation in proof.observations {
                if !validators.is_authorized(&observation.validator_pubkey) {
                    return Err(VoteExchangeError::Store(
                        "persisted observation names an unauthorized validator".into(),
                    ));
                }
                observations.insert(
                    observation.validator_pubkey,
                    observation.evidence_digests.into_iter().collect(),
                );
            }
            for validator in votes.keys() {
                observations.entry(*validator).or_default().insert(digest);
            }
            for (validator, digests) in &observations {
                if digests.len() > 1 && !equivocated.contains(validator) {
                    return Err(VoteExchangeError::Store(format!(
                        "validator {} observed more than one digest without an equivocation record",
                        hex_id(validator)
                    )));
                }
            }
            if state
                .proofs
                .insert(
                    proof.evidence.proof_id,
                    ProofRecord {
                        evidence: proof.evidence,
                        evidence_digest: digest,
                        votes,
                        equivocated,
                        observations,
                    },
                )
                .is_some()
            {
                return Err(VoteExchangeError::Store(
                    "persisted store contains a duplicate proof id".into(),
                ));
            }
        }
        Ok(Self {
            path,
            validators,
            state: Arc::new(RwLock::new(state)),
        })
    }

    pub fn required_signatures(&self) -> u32 {
        self.validators.required_signatures()
    }

    /// Validate and durably record one validator vote.
    pub async fn publish(
        &self,
        submission: SolanaVoteSubmission,
    ) -> Result<VotePublishResponse, VoteExchangeError> {
        if !self.validators.is_authorized(&submission.validator_pubkey) {
            return Err(VoteExchangeError::UnauthorizedValidator(hex_id(
                &submission.validator_pubkey,
            )));
        }
        let message = submission.evidence.message()?;
        if !verify_vote(
            &submission.validator_pubkey,
            &submission.signature,
            &message,
        ) {
            return Err(VoteExchangeError::InvalidSignature);
        }
        let digest = submission.evidence.digest();
        let proof_id = submission.evidence.proof_id;
        let validator = submission.validator_pubkey;

        let mut current = self.state.write().await;
        let mut next = current.clone();
        let conflict = if let Some(record) = next.proofs.get_mut(&proof_id) {
            if record.evidence_digest != digest {
                record
                    .observations
                    .entry(validator)
                    .or_default()
                    .insert(digest);
                let equivocated = record.observations[&validator].len() > 1;
                if equivocated {
                    record.equivocated.insert(validator);
                    record.votes.remove(&validator);
                }
                Some(equivocated)
            } else if record.equivocated.contains(&validator) {
                Some(true)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(equivocated) = conflict {
            self.persist(&next).await?;
            *current = next;
            return Err(if equivocated {
                VoteExchangeError::Equivocation {
                    validator: hex_id(&validator),
                    proof_id: hex_id(&proof_id),
                }
            } else {
                VoteExchangeError::EvidenceConflict {
                    proof_id: hex_id(&proof_id),
                }
            });
        }
        let observed_multiple = if let Some(record) = next.proofs.get_mut(&proof_id) {
            let multiple = {
                let observations = record.observations.entry(validator).or_default();
                observations.insert(digest);
                observations.len() > 1
            };
            if multiple {
                record.equivocated.insert(validator);
                record.votes.remove(&validator);
            }
            multiple
        } else {
            false
        };
        if observed_multiple {
            self.persist(&next).await?;
            *current = next;
            return Err(VoteExchangeError::Equivocation {
                validator: hex_id(&validator),
                proof_id: hex_id(&proof_id),
            });
        }
        let validator_count = if let Some(record) = next.proofs.get_mut(&proof_id) {
            record.votes.insert(validator, submission.signature);
            record.votes.len()
        } else {
            let mut votes = BTreeMap::new();
            votes.insert(validator, submission.signature);
            let validator_count = votes.len();
            let mut observations = BTreeMap::new();
            observations.insert(validator, BTreeSet::from([digest]));
            next.proofs.insert(
                proof_id,
                ProofRecord {
                    evidence: submission.evidence,
                    evidence_digest: digest,
                    votes,
                    equivocated: BTreeSet::new(),
                    observations,
                },
            );
            validator_count
        };
        self.persist(&next).await?;
        *current = next;
        Ok(VotePublishResponse {
            accepted: true,
            validator_count,
            required_signatures: self.required_signatures(),
            evidence_digest: hex_id(&digest),
        })
    }

    /// Return every accepted vote for exactly this evidence package.
    pub async fn votes_for(
        &self,
        evidence: &SolanaVoteEvidence,
    ) -> Result<VotesResponse, VoteExchangeError> {
        evidence.validate()?;
        let digest = evidence.digest();
        let state = self.state.read().await;
        let mut votes = Vec::new();
        if let Some(record) = state.proofs.get(&evidence.proof_id) {
            if record.evidence_digest != digest {
                return Err(VoteExchangeError::EvidenceConflict {
                    proof_id: hex_id(&evidence.proof_id),
                });
            }
            votes = record
                .votes
                .iter()
                .filter(|(key, _)| !record.equivocated.contains(*key))
                .map(|(validator_pubkey, signature)| VoteRecordWire {
                    validator_pubkey: *validator_pubkey,
                    signature: *signature,
                })
                .collect();
        }
        Ok(VotesResponse {
            proof_id: hex_id(&evidence.proof_id),
            evidence_digest: hex_id(&digest),
            required_signatures: self.required_signatures(),
            votes,
        })
    }

    async fn persist(&self, state: &ExchangeState) -> Result<(), VoteExchangeError> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|error| VoteExchangeError::Store(error.to_string()))?;
            }
        }
        let persisted = PersistedState {
            proofs: state
                .proofs
                .values()
                .map(|record| PersistedProof {
                    evidence: record.evidence.clone(),
                    evidence_digest: record.evidence_digest,
                    votes: record
                        .votes
                        .iter()
                        .map(|(validator_pubkey, signature)| PersistedVote {
                            validator_pubkey: *validator_pubkey,
                            signature: *signature,
                        })
                        .collect(),
                    equivocated: record.equivocated.iter().copied().collect(),
                    observations: record
                        .observations
                        .iter()
                        .map(
                            |(validator_pubkey, evidence_digests)| PersistedObservation {
                                validator_pubkey: *validator_pubkey,
                                evidence_digests: evidence_digests.iter().copied().collect(),
                            },
                        )
                        .collect(),
                })
                .collect(),
        };
        let encoded = serde_json::to_vec_pretty(&persisted)
            .map_err(|error| VoteExchangeError::Store(error.to_string()))?;
        let temporary = self.path.with_extension("tmp");
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)
            .await
            .map_err(|error| VoteExchangeError::Store(error.to_string()))?;
        file.write_all(&encoded)
            .await
            .map_err(|error| VoteExchangeError::Store(error.to_string()))?;
        file.sync_all()
            .await
            .map_err(|error| VoteExchangeError::Store(error.to_string()))?;
        drop(file);
        tokio::fs::rename(&temporary, &self.path)
            .await
            .map_err(|error| VoteExchangeError::Store(error.to_string()))?;
        #[cfg(unix)]
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                let parent = parent.to_path_buf();
                tokio::task::spawn_blocking(move || std::fs::File::open(parent)?.sync_all())
                    .await
                    .map_err(|error| VoteExchangeError::Store(error.to_string()))?
                    .map_err(|error| VoteExchangeError::Store(error.to_string()))?;
            }
        }
        Ok(())
    }
}

fn verify_vote(pubkey: &[u8; 32], signature: &[u8; 64], message: &[u8; 32]) -> bool {
    VerifyingKey::from_bytes(pubkey)
        .ok()
        .and_then(|key| {
            Ed25519Signature::from_slice(signature)
                .ok()
                .map(|signature| key.verify_strict(message, &signature).is_ok())
        })
        .unwrap_or(false)
}

/// Axum router for the vote exchange.
pub fn vote_exchange_router(exchange: Arc<VoteExchange>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/solana-votes", post(publish_vote))
        .route("/v1/solana-votes/query", post(query_votes))
        .layer(DefaultBodyLimit::max(MAX_VOTE_BODY_BYTES))
        .with_state(exchange)
}

/// Serve the exchange until the listener fails or the process stops.
pub async fn serve_vote_exchange(
    listener: tokio::net::TcpListener,
    exchange: VoteExchange,
) -> std::io::Result<()> {
    axum::serve(listener, vote_exchange_router(Arc::new(exchange)))
        .await
        .map_err(std::io::Error::other)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn publish_vote(
    State(exchange): State<Arc<VoteExchange>>,
    Json(submission): Json<SolanaVoteSubmission>,
) -> Result<Json<VotePublishResponse>, ApiError> {
    exchange
        .publish(submission)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn query_votes(
    State(exchange): State<Arc<VoteExchange>>,
    Json(evidence): Json<SolanaVoteEvidence>,
) -> Result<Json<VotesResponse>, ApiError> {
    exchange
        .votes_for(&evidence)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl From<VoteExchangeError> for ApiError {
    fn from(error: VoteExchangeError) -> Self {
        let status = match error {
            VoteExchangeError::UnauthorizedValidator(_) => StatusCode::FORBIDDEN,
            VoteExchangeError::InvalidEvidence(_) | VoteExchangeError::InvalidSignature => {
                StatusCode::BAD_REQUEST
            }
            VoteExchangeError::Equivocation { .. } | VoteExchangeError::EvidenceConflict { .. } => {
                StatusCode::CONFLICT
            }
            VoteExchangeError::Store(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

/// HTTP client used by validators and aggregators.
#[derive(Clone)]
pub struct VoteExchangeClient {
    base_url: String,
    client: reqwest::Client,
}

impl VoteExchangeClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').into(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn publish(
        &self,
        submission: &SolanaVoteSubmission,
    ) -> anyhow::Result<VotePublishResponse> {
        let response = self
            .client
            .post(format!("{}/v1/solana-votes", self.base_url))
            .json(submission)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body: Bytes = response.bytes().await?;
            anyhow::bail!(
                "vote exchange rejected submission with {status}: {}",
                String::from_utf8_lossy(&body)
            );
        }
        Ok(response.json().await?)
    }

    pub async fn votes_for(
        &self,
        evidence: &SolanaVoteEvidence,
    ) -> anyhow::Result<Vec<ValidatorSignature>> {
        let response = self
            .client
            .post(format!("{}/v1/solana-votes/query", self.base_url))
            .json(evidence)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body: Bytes = response.bytes().await?;
            anyhow::bail!(
                "vote exchange query failed with {status}: {}",
                String::from_utf8_lossy(&body)
            );
        }
        let votes: VotesResponse = response.json().await?;
        Ok(votes
            .votes
            .into_iter()
            .map(|vote| ValidatorSignature {
                validator_pubkey: vote.validator_pubkey,
                signature: vote.signature,
            })
            .collect())
    }
}

/// Build one signed submission.
pub fn sign_vote(
    evidence: &SolanaVoteEvidence,
    signing_key: &ed25519_dalek::SigningKey,
) -> Result<SolanaVoteSubmission, VoteExchangeError> {
    use ed25519_dalek::Signer;

    let message = evidence.message()?;
    Ok(SolanaVoteSubmission {
        evidence: evidence.clone(),
        validator_pubkey: signing_key.verifying_key().to_bytes(),
        signature: signing_key.sign(&message).to_bytes(),
    })
}

/// Convenience constructor for the exchange file path.
pub fn vote_store_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FILE: AtomicU64 = AtomicU64::new(1);

    fn store_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "x3-vote-exchange-{}-{}.json",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn keys() -> Vec<ed25519_dalek::SigningKey> {
        (1u8..=3)
            .map(|seed| ed25519_dalek::SigningKey::from_bytes(&[seed; 32]))
            .collect()
    }

    fn authorized(keys: &[ed25519_dalek::SigningKey]) -> AuthorizedValidatorSet {
        AuthorizedValidatorSet::from_keys(
            keys.iter()
                .map(|key| key.verifying_key().to_bytes())
                .collect(),
        )
    }

    fn evidence(amount: u128) -> SolanaVoteEvidence {
        let request = SvmProofRequest {
            runtime_intent_id: [0x11u8; 32],
            source_domain: 200,
            proof_id: [0x11u8; 32],
            cluster: SolanaCluster::Devnet,
            slot: 42,
            parent_slot: 41,
            blockhash: [7u8; 32],
            bank_hash: [6u8; 32],
            parent_bank_hash: [5u8; 32],
            stake_table_digest: [4u8; 32],
            expected_asset_id: [3u8; 32],
            expected_amount: amount,
            expected_sender: vec![2u8; 32],
            expected_recipient: vec![1u8; 32],
            expected_beneficiary: vec![9u8; 32],
            transaction: SolanaTransactionEvidence::from_raw(
                42,
                0,
                true,
                hex::decode(concat!(
                    "010ca9bbc1fd52fb8a2d08c512ccdb720fc3100804cb2fb8cba4bbc9ce4df7a7d0",
                    "567d96e7a21e7356e6f2b2b6d9efd725c6dd40bf4ad25dac00a27cb2dde3f509",
                    "01000001d04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c9778737",
                    "0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c00"
                ))
                .expect("signed transaction fixture"),
            )
            .expect("transaction"),
            existing_signatures: Vec::new(),
        };
        SolanaVoteEvidence::from_request(&request).expect("evidence")
    }

    #[tokio::test]
    async fn durable_quorum_rejects_equivocation_and_reloads() {
        let path = store_path();
        let keys = keys();
        let exchange = VoteExchange::open(&path, authorized(&keys))
            .await
            .expect("exchange");
        let original = evidence(5_000);
        let first = sign_vote(&original, &keys[0]).expect("vote");
        let second = sign_vote(&original, &keys[1]).expect("vote");
        assert_eq!(
            exchange
                .publish(first.clone())
                .await
                .expect("first vote")
                .validator_count,
            1
        );
        assert_eq!(
            exchange
                .publish(first.clone())
                .await
                .expect("idempotent vote")
                .validator_count,
            1
        );
        assert_eq!(
            exchange
                .publish(second)
                .await
                .expect("second vote")
                .validator_count,
            2
        );
        assert_eq!(
            exchange
                .votes_for(&original)
                .await
                .expect("votes")
                .votes
                .len(),
            2
        );

        let alternate = evidence(4_999);
        let uncommitted_conflict =
            sign_vote(&alternate, &keys[2]).expect("uncommitted conflicting vote");
        assert!(matches!(
            exchange.publish(uncommitted_conflict).await,
            Err(VoteExchangeError::EvidenceConflict { .. })
        ));
        assert!(
            matches!(
                exchange
                    .publish(sign_vote(&alternate, &keys[2]).expect("repeated conflicting vote"))
                    .await,
                Err(VoteExchangeError::EvidenceConflict { .. })
            ),
            "repeating one conflicting package is not equivocation over multiple packages"
        );
        assert_eq!(
            exchange
                .votes_for(&original)
                .await
                .expect("votes after refused conflict")
                .votes
                .len(),
            2
        );

        let conflicting = sign_vote(&alternate, &keys[0]).expect("conflicting vote");
        assert!(matches!(
            exchange.publish(conflicting).await,
            Err(VoteExchangeError::Equivocation { .. })
        ));
        assert_eq!(
            exchange
                .votes_for(&original)
                .await
                .expect("votes after conflict")
                .votes
                .len(),
            1,
            "an equivocating validator's prior vote is removed"
        );

        drop(exchange);
        let reloaded = VoteExchange::open(&path, authorized(&keys))
            .await
            .expect("reload");
        assert_eq!(
            reloaded
                .votes_for(&original)
                .await
                .expect("reloaded votes")
                .votes
                .len(),
            1
        );
        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn unauthorized_and_bad_signatures_are_refused() {
        let path = store_path();
        let keys = keys();
        let exchange = VoteExchange::open(&path, authorized(&keys[..2]))
            .await
            .expect("exchange");
        let evidence = evidence(5_000);

        let outsider = sign_vote(&evidence, &keys[2]).expect("vote");
        assert!(matches!(
            exchange.publish(outsider).await,
            Err(VoteExchangeError::UnauthorizedValidator(_))
        ));

        let mut forged = sign_vote(&evidence, &keys[0]).expect("vote");
        forged.signature[0] ^= 1;
        assert!(matches!(
            exchange.publish(forged).await,
            Err(VoteExchangeError::InvalidSignature)
        ));
        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn store_refuses_duplicate_proofs_and_unrecorded_equivocation() {
        let path = store_path();
        let keys = keys();
        let exchange = VoteExchange::open(&path, authorized(&keys))
            .await
            .expect("exchange");
        let evidence = evidence(5_000);
        exchange
            .publish(sign_vote(&evidence, &keys[0]).expect("vote"))
            .await
            .expect("published vote");
        drop(exchange);

        let original = tokio::fs::read(&path).await.expect("persisted store");
        let mut duplicate: serde_json::Value =
            serde_json::from_slice(&original).expect("persisted json");
        let proof = duplicate["proofs"][0].clone();
        duplicate["proofs"]
            .as_array_mut()
            .expect("proofs array")
            .push(proof);
        tokio::fs::write(&path, serde_json::to_vec(&duplicate).expect("json"))
            .await
            .expect("write duplicate");
        let error = VoteExchange::open(&path, authorized(&keys))
            .await
            .err()
            .expect("duplicate proof ids must fail closed");
        assert!(matches!(
            error,
            VoteExchangeError::Store(message) if message.contains("duplicate proof id")
        ));

        let mut inconsistent: serde_json::Value =
            serde_json::from_slice(&original).expect("persisted json");
        inconsistent["proofs"][0]["observations"][0]["evidence_digests"]
            .as_array_mut()
            .expect("evidence digests")
            .push(serde_json::to_value([0xAAu8; 32]).expect("digest json"));
        tokio::fs::write(&path, serde_json::to_vec(&inconsistent).expect("json"))
            .await
            .expect("write inconsistent");
        let error = VoteExchange::open(&path, authorized(&keys))
            .await
            .err()
            .expect("unrecorded equivocation must fail closed");
        assert!(matches!(
            error,
            VoteExchangeError::Store(message)
                if message.contains("without an equivocation record")
        ));

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn real_http_client_publishes_and_queries_votes() {
        let path = store_path();
        let keys = keys();
        let exchange = VoteExchange::open(&path, authorized(&keys))
            .await
            .expect("exchange");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(serve_vote_exchange(listener, exchange));
        let client = VoteExchangeClient::new(format!("http://{address}"));
        let evidence = evidence(5_000);
        client
            .publish(&sign_vote(&evidence, &keys[0]).expect("vote"))
            .await
            .expect("publish");
        let votes = client.votes_for(&evidence).await.expect("query");
        assert_eq!(votes.len(), 1);
        server.abort();
        tokio::fs::remove_file(path).await.expect("cleanup");
    }
}
