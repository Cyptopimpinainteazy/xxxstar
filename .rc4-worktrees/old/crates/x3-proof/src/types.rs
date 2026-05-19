//! Core proof types for the X3 jurisdiction.

use serde::{Deserialize, Serialize};

/// 32-byte hash used throughout the proof system.
pub type Hash256 = [u8; 32];

/// Unique identifier for a proof within a chain.
pub type ProofId = u64;

/// Block height used as deterministic clock.
pub type BlockHeight = u64;

/// An execution proof capturing a single atomic operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionProof {
    /// Unique proof identifier.
    pub id: ProofId,
    /// Block height at which this execution occurred.
    pub block_height: BlockHeight,
    /// Hash of the X3-lang program being executed.
    pub program_hash: Hash256,
    /// Hash of the input state before execution.
    pub pre_state_hash: Hash256,
    /// Hash of the output state after execution.
    pub post_state_hash: Hash256,
    /// Ordered list of state diffs produced.
    pub state_diffs: Vec<StateDiff>,
    /// Gas consumed during execution.
    pub gas_consumed: u64,
    /// Fee charged for this execution.
    pub fee_charged: u64,
    /// Agent identity that executed this.
    pub agent_id: AgentIdentity,
    /// Intent this proof belongs to, if any.
    pub intent_id: Option<IntentId>,
    /// Hash of this proof (computed over all fields above).
    pub proof_hash: Hash256,
}

/// A single state diff representing one atomic state change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateDiff {
    /// Storage key affected.
    pub key: Vec<u8>,
    /// Previous value (None if newly created).
    pub old_value: Option<Vec<u8>>,
    /// New value (None if deleted).
    pub new_value: Option<Vec<u8>>,
}

/// Agent identity — can be ephemeral or persistent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentIdentity {
    /// Public key bytes (32 bytes for Ed25519).
    pub pubkey: [u8; 32],
    /// Whether this is an ephemeral identity (single-use).
    pub ephemeral: bool,
}

/// Unique intent identifier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct IntentId(pub u128);

/// A state proof — proves a particular state root at a given block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateProof {
    /// Block height at which this state was captured.
    pub block_height: BlockHeight,
    /// Merkle root of the state tree.
    pub state_root: Hash256,
    /// Merkle inclusion proof path.
    pub merkle_path: Vec<Hash256>,
    /// The key being proved.
    pub key: Vec<u8>,
    /// The value at that key.
    pub value: Vec<u8>,
}

/// Proof of slashing — immutable record of punishment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlashProof {
    /// The execution proof that triggered slashing.
    pub execution_proof: ExecutionProof,
    /// The replay proof confirming the violation.
    pub replay_proof: ReplayProof,
    /// Amount slashed (in base units).
    pub slash_amount: u128,
    /// Block height at which slashing was finalized.
    pub finalized_at: BlockHeight,
}

/// Proof produced by deterministic replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayProof {
    /// The original execution proof being replayed.
    pub original_proof_hash: Hash256,
    /// Hash of the replayed execution result.
    pub replay_result_hash: Hash256,
    /// Whether the replay matched the original.
    pub matches: bool,
    /// Divergence point if replay didn't match (instruction index).
    pub divergence_at: Option<u64>,
    /// Block height of the replay.
    pub replayed_at: BlockHeight,
}

/// Execution receipt — the final artifact of a completed execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionReceipt {
    /// The complete proof chain for this execution.
    pub proof_chain_hash: Hash256,
    /// Total gas consumed across all steps.
    pub total_gas: u64,
    /// Total fees charged.
    pub total_fees: u64,
    /// Final state root after execution.
    pub final_state_root: Hash256,
    /// Whether execution succeeded.
    pub success: bool,
    /// Block height at which execution was finalized.
    pub finalized_at: BlockHeight,
    /// Agent that performed the execution.
    pub agent_id: AgentIdentity,
}
