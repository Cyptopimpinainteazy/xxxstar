/// Relayer configuration structures and types
#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::vec::Vec;

use serde::{Deserialize, Serialize};
pub use x3_verification_router::SolanaCluster;
use x3_verification_router::{
    encode_solana_finality_v3, ChainKind, ProofEnvelope, VerificationError, VerificationStrategy,
};

/// Main relayer configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub x3: X3Config,
    pub evm_chains: Vec<EvmChainConfig>,
    pub svm_clusters: Vec<SvmClusterConfig>,
    /// The authorized validator set a proof's attestation quorum is derived
    /// from. `#[serde(default)]` keeps configurations written before this field
    /// existed loadable, and the default is the fail-closed empty set.
    #[serde(default)]
    pub validator_set: ValidatorSetConfig,
    pub submission: SubmissionConfig,
    pub governance: GovernanceConfig,
    pub logging: LoggingConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct X3Config {
    pub rpc_url: String,
    pub relayer_account: String,
    #[serde(default)]
    pub relayer_seed_phrase: Option<String>,
    #[serde(default)]
    pub relayer_custody_key_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvmChainConfig {
    pub name: String,
    pub chain_id: u32,
    pub x3_domain_id: u32,
    pub rpc_endpoint: String,
    pub state_root_contract: String,
    pub finality_threshold: u32,
    pub block_poll_interval_ms: u64,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SvmClusterConfig {
    pub name: String,
    pub cluster_name: String,
    pub x3_domain_id: u32,
    pub rpc_endpoint: String,
    pub finality_threshold: u32,
    pub slot_poll_interval_ms: u64,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: u32,
}

/// The validator set authorized to attest external-chain finality.
///
/// The number of signatures an SVM finalized-slot proof must carry is a
/// supermajority of this set — `floor(2n/3) + 1` distinct signers, the single
/// definition in `x3_validator_attestation::supermajority_threshold`. It is
/// derived from the set, never configured separately, so a set cannot be paired
/// with a threshold that does not belong to it.
///
/// An empty list is the fail-closed default: with no authorized validators,
/// nothing can attest, so every SVM proof is refused and the relayer refuses to
/// produce one.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorSetConfig {
    /// Hex-encoded Ed25519 public keys of the authorized validators, with or
    /// without a `0x` prefix. Malformed, wrongly sized or repeated entries are
    /// refused at startup rather than silently dropped.
    #[serde(default)]
    pub svm_validator_pubkeys: Vec<String>,
    /// Optional stake weights for the same validator set. Mainnet Solana proofs
    /// require these; the V1 signer-count path is never registered by production.
    #[serde(default)]
    pub svm_validator_stakes: Vec<SvmValidatorStakeConfig>,
}

/// One governance-authorized Solana validator stake entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SvmValidatorStakeConfig {
    /// Hex-encoded Ed25519 public key, with or without `0x`.
    pub pubkey: String,
    /// Effective stake weight.
    pub stake: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmissionConfig {
    pub batch_size: u32,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub poll_interval_secs: u64,
    pub enable_graceful_shutdown: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LoggingConfig {
    pub level: String,
    #[serde(default)]
    pub format: String,
}

fn default_max_concurrent() -> u32 {
    5
}

// ============================================================================
// Type Definitions
// ============================================================================

#[derive(Clone, Debug)]
pub struct HeaderInfo {
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub receipts_root: [u8; 32],
    pub timestamp: u64,
    pub chain_id: u32,
}

#[derive(Clone, Debug)]
pub struct EvmProof {
    pub source_domain: u32,
    /// The gateway's `messageId`; this is the proof's replay identity, not the
    /// containing block hash.
    pub proof_id: [u8; 32],
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub finalized_block: u64,
    pub proof_nonce: u32,
    /// Canonical payload for `ProductionEvmReceiptVerifier`.
    pub proof_payload: Vec<u8>,
    /// ERC-20 contract named by the gateway event.
    pub source_asset: [u8; 20],
    /// Depositor named by the gateway event.
    pub expected_sender: [u8; 20],
    /// Exact gateway contract that emitted the event.
    pub expected_contract: [u8; 20],
    /// Exact X3 recipient encoded in the deposit event.
    pub expected_beneficiary: Vec<u8>,
    /// Gateway event nonce.
    pub expected_nonce: u64,
    /// Exact deposit amount encoded in the deposit log.
    pub expected_amount: u128,
}

/// Inputs required to bind a finalized EVM header to an event-specific receipt proof.
#[derive(Clone, Debug)]
pub struct EvmProofRequest {
    pub source_domain: u32,
    pub proof_id: [u8; 32],
    pub finalized_block: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub proof_payload: Vec<u8>,
    pub source_asset: [u8; 20],
    pub expected_sender: [u8; 20],
    pub expected_contract: [u8; 20],
    pub expected_beneficiary: Vec<u8>,
    pub expected_nonce: u64,
    pub expected_amount: u128,
}

/// A single validator attestation over the complete Solana V3 finality and
/// transfer intent.
#[derive(Clone, Debug)]
pub struct ValidatorSignature {
    /// Compressed Ed25519 public key of the signing validator (32 bytes).
    pub validator_pubkey: [u8; 32],
    /// Ed25519 signature over the proof payload (64 bytes).
    pub signature: [u8; 64],
}

#[derive(Clone, Debug)]
pub struct SvmProof {
    pub source_domain: u32,
    /// Gateway transfer identity bound into the validator-signed message.
    pub proof_id: [u8; 32],
    pub cluster: SolanaCluster,
    pub slot: u64,
    pub parent_slot: u64,
    /// Finalized Solana blockhash.
    pub blockhash: [u8; 32],
    /// Finalized validator-attested bank hash for the same slot.
    pub bank_hash: [u8; 32],
    pub parent_bank_hash: [u8; 32],
    pub stake_table_digest: [u8; 32],
    pub expected_asset_id: [u8; 32],
    pub expected_amount: u128,
    pub expected_sender: Vec<u8>,
    pub expected_recipient: Vec<u8>,
    pub expected_beneficiary: Vec<u8>,
    /// Individual validator attestations (each carries a real 64-byte Ed25519 sig).
    pub validator_signatures: Vec<ValidatorSignature>,
    /// Number of distinct validators required for quorum (derived from runtime config).
    pub required_signatures: u32,
}

/// V3 evidence a relayer must obtain before it can produce a Solana proof.
///
/// The bank hashes, slots, stake-table digest, and transfer intent are signed
/// by the validators. `existing_signatures` lets independent operators
/// aggregate before the configured threshold is reachable.
#[derive(Clone, Debug)]
pub struct SvmProofRequest {
    pub source_domain: u32,
    pub proof_id: [u8; 32],
    pub cluster: SolanaCluster,
    pub slot: u64,
    pub parent_slot: u64,
    /// Finalized Solana blockhash observed from the block.
    pub blockhash: [u8; 32],
    /// Finalized bank hash supplied by validator evidence.
    pub bank_hash: [u8; 32],
    pub parent_bank_hash: [u8; 32],
    pub stake_table_digest: [u8; 32],
    pub expected_asset_id: [u8; 32],
    pub expected_amount: u128,
    pub expected_sender: Vec<u8>,
    pub expected_recipient: Vec<u8>,
    pub expected_beneficiary: Vec<u8>,
    pub existing_signatures: Vec<ValidatorSignature>,
}

impl SvmProof {
    /// Build the complete V3 envelope the production verifier receives.
    pub fn envelope(&self) -> ProofEnvelope {
        ProofEnvelope {
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
        }
    }

    /// Encode the stake-weighted V3 Solana finality payload.
    pub fn encode_payload(&self) -> Result<Vec<u8>, VerificationError> {
        let votes = self
            .validator_signatures
            .iter()
            .map(|vote| (vote.validator_pubkey, vote.signature))
            .collect::<Vec<_>>();
        encode_solana_finality_v3(
            self.cluster,
            self.slot,
            self.parent_slot,
            self.blockhash,
            self.bank_hash,
            self.parent_bank_hash,
            self.stake_table_digest,
            &votes,
        )
    }

    /// Envelope carrying the encoded V3 payload.
    pub fn encoded_envelope(&self) -> Result<ProofEnvelope, VerificationError> {
        let mut envelope = self.envelope();
        envelope.payload = self.encode_payload()?;
        Ok(envelope)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RelayerStateEnum {
    Initializing,
    Active,
    Paused,
    Shutting,
    Stopped,
}

#[derive(Clone, Debug, Default)]
pub struct RelayerMetrics {
    pub blocks_polled: u64,
    pub blocks_finalized: u64,
    pub proofs_submitted: u64,
    pub proofs_failed: u64,
    pub poll_failures: u64,
    pub pause_events: u64,
    pub uptime_secs: u64,
    /// Watchdog: number of stale-cycle warnings emitted (no new blocks/slots)
    pub stale_warnings: u64,
    /// Watchdog: number of times RPC retry was triggered
    pub rpc_retries: u64,
    /// Watchdog: number of reconnection attempts
    pub reconnections: u64,
}

impl Default for SubmissionConfig {
    fn default() -> Self {
        Self {
            batch_size: 1,
            timeout_secs: 60,
            max_retries: 3,
            retry_backoff_ms: 1000,
        }
    }
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 5,
            enable_graceful_shutdown: true,
        }
    }
}
