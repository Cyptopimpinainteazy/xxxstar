/// Relayer configuration structures and types
use serde::{Deserialize, Serialize};

/// Main relayer configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub x3: X3Config,
    pub evm_chains: Vec<EvmChainConfig>,
    pub svm_clusters: Vec<SvmClusterConfig>,
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
    pub timestamp: u64,
    pub chain_id: u32,
}

#[derive(Clone, Debug)]
pub struct EvmProof {
    pub source_domain: u32,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub finalized_block: u64,
    pub proof_nonce: u32,
}

#[derive(Clone, Debug)]
pub struct SvmProof {
    pub source_domain: u32,
    pub slot: u64,
    pub blockhash: [u8; 32],
    pub validator_signatures: Vec<[u8; 32]>,
    pub required_signatures: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RelayerStateEnum {
    Initializing,
    Active,
    Paused,
    Shutting,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct RelayerMetrics {
    pub blocks_polled: u64,
    pub blocks_finalized: u64,
    pub proofs_submitted: u64,
    pub proofs_failed: u64,
    pub poll_failures: u64,
    pub pause_events: u64,
    pub uptime_secs: u64,
}

impl Default for RelayerMetrics {
    fn default() -> Self {
        Self {
            blocks_polled: 0,
            blocks_finalized: 0,
            proofs_submitted: 0,
            proofs_failed: 0,
            poll_failures: 0,
            pause_events: 0,
            uptime_secs: 0,
        }
    }
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
