use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Unique identifier for a swarm task — 32-byte hex string matching on-chain H256.
pub type TaskId = String;

/// Executor node identifier — typically the SS58 account address.
pub type ExecutorId = String;

/// Runtime configuration loaded from environment variables at startup.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// WebSocket (or HTTP) URL for the chain RPC endpoint.
    /// Env: `NS_CHAIN_RPC` · Default: `ws://127.0.0.1:9944`
    pub chain_rpc_url: String,

    /// Base URL for the IPFS gateway used to fetch task payloads.
    /// Env: `NS_IPFS_GATEWAY` · Default: `http://127.0.0.1:8080`
    pub ipfs_gateway: String,

    /// Executor signing key — SS58 seed phrase or `//DevKey` shorthand.
    /// Env: `NS_EXECUTOR_KEY` · **Never** hardcode in production.
    pub executor_key: String,

    /// Maximum number of concurrent task executions.
    /// Env: `NS_PARALLELISM` · Default: `4`
    pub parallelism: usize,
}

impl Config {
    /// Build config from environment variables; fall back to safe defaults.
    pub fn from_env() -> Self {
        Config {
            chain_rpc_url: std::env::var("NS_CHAIN_RPC")
                .unwrap_or_else(|_| "ws://127.0.0.1:9944".into()),
            ipfs_gateway: std::env::var("NS_IPFS_GATEWAY")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            executor_key: std::env::var("NS_EXECUTOR_KEY")
                .unwrap_or_else(|_| "//Alice".into()),
            parallelism: std::env::var("NS_PARALLELISM")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
        }
    }
}

// ---------------------------------------------------------------------------
// Task types
// ---------------------------------------------------------------------------

/// A task as read from the chain pending-task queue.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NorthernTask {
    pub id: TaskId,
    /// IPFS CID or other content-addressable URI for the payload.
    /// Supported schemes: `ipfs://<CID>`, `hex:<hex-bytes>`.
    pub payload_uri: String,
    /// Block number at which this task was submitted on-chain.
    pub submitted_at_block: u64,
    pub kind: TaskKind,
    pub status: TaskStatus,
    /// X3 Lang bytecode hash, present when the task was compiled via RC4.
    pub x3_bytecode_hash: Option<String>,
}

/// Payload body fetched from the content-addressed store.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskPayload {
    pub task_id: TaskId,
    /// Raw deterministic bytecode or script body.
    pub body: Vec<u8>,
    /// Key-value parameters injected at execution time.
    pub params: HashMap<String, serde_json::Value>,
    /// Optional secondary input dataset URI.
    pub input_uri: Option<String>,
}

/// High-level task categories.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Pure computation — no external I/O; must be fully deterministic.
    Compute,
    /// Off-chain data fetch + transform; result hash must be stable.
    DataFetch,
    /// AI/ML inference job.
    AiInference,
    /// X3 Lang compiled agent job (RC4).
    X3LangAgent,
    /// Custom / future extension kind.
    Other(String),
}

/// Task lifecycle state, mirrored from the RC2 on-chain pallet enum.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Claimed,
    Running,
    Completed,
    Failed,
    Disputed,
}

// ---------------------------------------------------------------------------
// Execution result
// ---------------------------------------------------------------------------

/// Result produced after a task execution round.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub task_id: TaskId,
    pub executor_id: ExecutorId,
    /// SHA-256 digest of the raw output bytes, hex-encoded.
    /// This is the value committed on-chain.
    pub result_hash: String,
    /// Full raw output — kept locally; only the hash is submitted on-chain.
    pub output: Vec<u8>,
    /// Proof bundle for RC3 quorum comparison.
    pub proof: ProofBundle,
    pub status: ExecutionStatus,
}

/// Execution outcome classification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    /// Executor produced output but it was non-deterministic; will not submit.
    NonDeterministic,
    /// Hard failure; executor will self-report and may be slashed.
    Failed(String),
}

// ---------------------------------------------------------------------------
// Proof bundle (RC3)
// ---------------------------------------------------------------------------

/// Proof bundle attached to every execution result.
///
/// In RC3: 3+ executors each produce a bundle.  Those whose
/// `output_hash` matches the quorum majority are rewarded; the
/// minority are slashed or lose reputation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofBundle {
    pub task_id: TaskId,
    pub executor_id: ExecutorId,
    /// SHA-256 of the full input payload body.
    pub input_hash: String,
    /// SHA-256 of the execution output.
    pub output_hash: String,
    /// UNIX timestamp (seconds) when execution completed.
    pub executed_at: i64,
    /// Wall-clock execution duration in milliseconds.
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Executor profile (RC2 on-chain registry)
// ---------------------------------------------------------------------------

/// Executor profile registered on-chain via `pallet-northern-swarm`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutorProfile {
    pub executor_id: ExecutorId,
    pub hardware: HardwareProfile,
    /// Amount staked in the smallest token denomination.
    /// This is the slashing target for dishonest execution.
    pub stake: u128,
    /// Consecutive successful executions — used as a reputation proxy.
    pub reputation: u64,
}

/// Advertised hardware capabilities of an executor node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub cpu_cores: u32,
    /// GPU VRAM in MiB; 0 if no GPU.
    pub gpu_vram_mib: u32,
    /// Available RAM in MiB.
    pub ram_mib: u64,
    /// Advertised network bandwidth in Mbps.
    pub bandwidth_mbps: u32,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum NorthernSwarmError {
    #[error("chain RPC error: {0}")]
    ChainRpc(String),

    #[error("payload fetch error (uri={uri}): {source}")]
    PayloadFetch { uri: String, source: String },

    #[error("execution failed for task {task_id}: {reason}")]
    ExecutionFailed { task_id: TaskId, reason: String },

    #[error("result submission failed for task {task_id}: {reason}")]
    SubmitFailed { task_id: TaskId, reason: String },

    #[error("serialisation error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
