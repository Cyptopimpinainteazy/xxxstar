//! Core court types aligned with X3 Design Booklet Section 3.5

use serde::{Deserialize, Serialize};
use x3_proof::types::{AgentIdentity, BlockHeight, Hash256, IntentId};

/// Resource usage vector (mirrors x3-fees::ResourceVector but Court owns its copy)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceVector {
    pub cpu_cycles: u64,
    pub gpu_cycles: u64,
    pub memory_bytes: u64,
    pub io_ops: u64,
    pub storage_reads: u64,
    pub storage_writes: u64,
}

impl ResourceVector {
    pub fn exceeds(&self, other: &Self) -> bool {
        self.cpu_cycles > other.cpu_cycles
            || self.gpu_cycles > other.gpu_cycles
            || self.memory_bytes > other.memory_bytes
            || self.io_ops > other.io_ops
            || self.storage_reads > other.storage_reads
            || self.storage_writes > other.storage_writes
    }
}

/// Dispute identifier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DisputeId(pub u64);

/// Dispute lifecycle state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisputeState {
    /// Dispute filed, pending replay.
    Filed,
    /// Replay in progress.
    Replaying,
    /// Verdict rendered.
    Resolved,
    /// Dispute dismissed (invalid filing).
    Dismissed,
}

/// Challenge type - strictly typed set (Closed Set per design)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChallengeType {
    /// Block execution diverged from claimed state
    InvalidExecution,
    /// Action DAG structure invalid or mismatched
    InvalidDag,
    /// Execution order hash mismatch
    InvalidOrder,
    /// Receipt hash mismatch
    ReceiptMismatch,
    /// Resource usage mismatch (CPU/GPU/Memory)
    ResourceMismatch,
    /// Proposer signed two different blocks at same height
    ProposerEquivocation,
    /// Agent fraud (output mismatch, kernel mismatch)
    AgentFraud,
    /// False challenge (challenger provided bad proof)
    InvalidChallenge,
}

/// Challenge payload - typed proof data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChallengePayload {
    /// Receipt mismatch: action ID, expected vs observed hashes
    ReceiptMismatch {
        action_id: u64,
        expected: Hash256,
        observed: Hash256,
    },
    /// Resource mismatch: agent ID, claimed vs actual resources
    ResourceMismatch {
        agent_id: u64,
        claimed: ResourceVector,
        actual: ResourceVector,
    },
    /// DAG conflict: two actions with invalid dependency
    DagConflict { action_a: u64, action_b: u64 },
    /// Proposer equivocation: two competing blocks at same height
    Equivocation { block_a: Hash256, block_b: Hash256 },
    /// GPU execution mismatch (kernel/output/resource)
    GpuFraud {
        gpu_receipt_hash: Hash256,
        mismatch_type: GpuMismatchType,
    },
}

/// GPU-specific mismatch types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuMismatchType {
    OutputCommitmentMismatch,
    KernelHashMismatch,
    ResourceUnderreport,
    NonDeterministicExecution,
}

/// The type of dispute being filed (per design booklet)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisputeType {
    /// Execution diverged from deterministic expectation.
    ExecutionDivergence {
        /// The proof chain being disputed.
        proof_chain_hash: Hash256,
    },
    /// Invalid execution (receipt, order, DAG)
    InvalidExecution {
        challenge_type: ChallengeType,
        payload: ChallengePayload,
    },
    /// Agent submitted invalid proof or computation.
    InvalidProof {
        /// The specific proof hash being contested.
        proof_hash: Hash256,
    },
    /// Agent executed the same intent twice.
    DoubleExecution {
        intent_id: IntentId,
        first_proof: Hash256,
        second_proof: Hash256,
    },
    /// Execution result doesn't match the proof chain.
    ResultMismatch {
        intent_id: IntentId,
        claimed_result: Hash256,
        actual_result: Hash256,
    },
    /// Resource accounting fraud (underclaim, overuse)
    ResourceFraud {
        agent_id: u64,
        claimed: ResourceVector,
        actual: ResourceVector,
    },
    /// Proposer equivocation (two blocks at same height)
    ProposerEquivocation { block_a: Hash256, block_b: Hash256 },
}

/// A dispute filed with the court.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    /// Unique dispute ID.
    pub id: DisputeId,
    /// Type of dispute.
    pub dispute_type: DisputeType,
    /// Agent being disputed (or proposer/validator).
    pub respondent: AgentIdentity,
    /// Block at which the dispute was filed.
    pub filed_at: BlockHeight,
    /// Deadline for resolution (finality window).
    pub deadline: BlockHeight,
    /// Current state.
    pub state: DisputeState,
    /// The verdict, once resolved.
    pub verdict: Option<VerdictRecord>,
    /// Bond posted by challenger (burned if invalid challenge)
    pub challenger_bond: u128,
}

/// Verdict outcome.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerdictOutcome {
    /// Respondent is guilty — slashing enforced.
    Guilty,
    /// Respondent is not guilty — dispute dismissed.
    NotGuilty,
    /// Dispute is invalid (malformed, wrong target, etc.).
    InvalidDispute,
}

/// Verdict record — immutable and final.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerdictRecord {
    /// The dispute this verdict resolves.
    pub dispute_id: DisputeId,
    /// The outcome.
    pub outcome: VerdictOutcome,
    /// Block at which the verdict was rendered.
    pub rendered_at: BlockHeight,
    /// Hash of the replay proof (if applicable).
    pub replay_proof_hash: Option<Hash256>,
    /// Amount ordered slashed (0 if acquitted).
    pub slash_amount: u128,
    /// Hash of the verdict record.
    pub verdict_hash: Hash256,
}

/// Court configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtConfig {
    /// Maximum blocks allowed for dispute resolution.
    pub finality_window: u64,
    /// Minimum bond required to file a dispute (anti-spam).
    pub dispute_bond: u128,
    /// Whether auto-slashing is enabled on guilty verdicts.
    pub auto_slash: bool,
}

impl Default for CourtConfig {
    fn default() -> Self {
        Self {
            finality_window: 100,
            dispute_bond: 100_000,
            auto_slash: true,
        }
    }
}
