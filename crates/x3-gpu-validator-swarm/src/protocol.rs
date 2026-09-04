//! Protocol definitions for X3 GPU Validator Swarm

use crate::crypto::{HashOutput, SignatureOutput};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Swarm message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SwarmMessage {
    /// Task assignment
    TaskAssignment(TaskAssignment),
    /// Task result
    TaskResult(TaskResult),
    /// Validator announcement
    ValidatorAnnounce(ValidatorAnnouncement),
    /// Heartbeat
    Heartbeat(Heartbeat),
    /// Challenge (for verification)
    Challenge(Challenge),
    /// Challenge response
    ChallengeResponse(ChallengeResponse),
    /// Sync request
    SyncRequest(SyncRequest),
    /// Sync response
    SyncResponse(SyncResponse),
}

/// Task assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Assignment ID
    pub assignment_id: String,
    /// Task type
    pub task_type: String,
    /// Input data
    pub inputs: Vec<Vec<u8>>,
    /// Expected output count
    pub expected_count: usize,
    /// Deadline timestamp
    pub deadline: i64,
    /// Validator to assign to
    pub validator_id: String,
}

impl TaskAssignment {
    /// Create a new task assignment
    pub fn new(
        task_type: String,
        inputs: Vec<Vec<u8>>,
        validator_id: String,
        deadline: i64,
    ) -> Self {
        Self {
            assignment_id: Uuid::new_v4().to_string(),
            task_type,
            inputs,
            expected_count: 0,
            deadline,
            validator_id,
        }
    }
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Assignment ID
    pub assignment_id: String,
    /// Validator ID
    pub validator_id: String,
    /// Output hashes
    pub outputs: Vec<HashOutput>,
    /// Execution time (ms)
    pub execution_time_ms: u64,
    /// Verification result
    pub verification_result: String,
    /// Whether CPU fallback was used
    pub cpu_fallback: bool,
    /// Signature
    pub signature: Option<SignatureOutput>,
    /// Timestamp
    pub timestamp: i64,
}

/// Validator announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorAnnouncement {
    /// Validator ID
    pub validator_id: String,
    /// Validator address
    pub address: String,
    /// Stake amount
    pub stake: u64,
    /// GPU capabilities
    pub gpu_info: String,
    /// Region
    pub region: Option<String>,
    /// Timestamp
    pub timestamp: i64,
}

/// Heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Validator ID
    pub validator_id: String,
    /// Current load
    pub load: f64,
    /// Tasks processed
    pub tasks_processed: u64,
    /// Timestamp
    pub timestamp: i64,
}

/// Challenge for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// Challenge ID
    pub challenge_id: String,
    /// Challenge data
    pub data: Vec<u8>,
    /// Expected result hash
    pub expected_hash: HashOutput,
    /// Deadline
    pub deadline: i64,
}

impl Challenge {
    /// Create a new challenge
    pub fn new(data: Vec<u8>, expected_hash: HashOutput, deadline: i64) -> Self {
        Self {
            challenge_id: Uuid::new_v4().to_string(),
            data,
            expected_hash,
            deadline,
        }
    }
}

/// Challenge response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    /// Challenge ID
    pub challenge_id: String,
    /// Validator ID
    pub validator_id: String,
    /// Result hash
    pub result_hash: HashOutput,
    /// Execution time (ms)
    pub execution_time_ms: u64,
    /// Whether result matches expected
    pub matches: bool,
    /// Timestamp
    pub timestamp: i64,
}

/// Sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Request ID
    pub request_id: String,
    /// Validator ID
    pub validator_id: String,
    /// Sync type
    pub sync_type: String,
    /// From block height
    pub from_block: u64,
}

/// Sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Request ID
    pub request_id: String,
    /// Data
    pub data: Vec<u8>,
    /// Block height
    pub block_height: u64,
    /// Timestamp
    pub timestamp: i64,
}

/// Validator message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorMessage {
    /// Message ID
    pub message_id: String,
    /// Sender validator ID
    pub sender_id: String,
    /// Sequence number
    pub sequence: u64,
    /// Payload
    pub payload: SwarmMessage,
    /// Signature
    pub signature: Option<SignatureOutput>,
}

impl ValidatorMessage {
    /// Create a new validator message
    pub fn new(sender_id: String, payload: SwarmMessage) -> Self {
        Self {
            message_id: Uuid::new_v4().to_string(),
            sender_id,
            sequence: 0,
            payload,
            signature: None,
        }
    }
}

/// Validator proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorProof {
    /// Proof ID
    pub proof_id: String,
    /// Validator ID
    pub validator_id: String,
    /// Task ID
    pub task_id: String,
    /// Output hash
    pub output_hash: HashOutput,
    /// Verification hash
    pub verification_hash: HashOutput,
    /// Timestamp
    pub timestamp: i64,
    /// Block height
    pub block_height: u64,
    /// Signature
    pub signature: SignatureOutput,
}
