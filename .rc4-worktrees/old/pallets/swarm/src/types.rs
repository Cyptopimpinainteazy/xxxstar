//! Types for the Swarm pallet.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::prelude::*;

// ============================================================================
// Contributor Types
// ============================================================================

/// Unique on-chain contributor identifier.
pub type ContributorId = u32;

/// Task identifier (hash of submission data).
pub type TaskId = H256;

/// Jury session identifier.
pub type SessionId = H256;

/// Contributor status on-chain.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub enum ContributorStatus {
    /// Active and accepting tasks.
    #[default]
    Active,
    /// Temporarily offline / not accepting work.
    Idle,
    /// Slashed or removed by governance.
    Slashed,
    /// Voluntarily deregistered (unstaking cooldown).
    Deregistering,
}

/// GPU capability descriptor stored on-chain.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub struct GpuCapabilities {
    /// VRAM in megabytes.
    pub vram_mb: u32,
    /// Compute score (higher is better, normalized 0-10000).
    pub compute_score: u32,
    /// Whether CUDA is available.
    pub cuda: bool,
    /// Number of GPU devices.
    pub device_count: u8,
}

/// A registered swarm contributor.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct Contributor<AccountId, Balance, BlockNumber> {
    /// On-chain contributor ID.
    pub id: ContributorId,
    /// Account that controls this contributor (receives rewards).
    pub account: AccountId,
    /// Staked amount.
    pub stake: Balance,
    /// Current status.
    pub status: ContributorStatus,
    /// GPU capabilities.
    pub capabilities: GpuCapabilities,
    /// Display name (max 64 bytes).
    pub name: BoundedVec<u8, ConstU32<64>>,
    /// Reputation score (0-200, 100 = neutral).
    pub reputation: u32,
    /// Tasks completed successfully.
    pub tasks_completed: u64,
    /// Tasks failed.
    pub tasks_failed: u64,
    /// Block when registered.
    pub registered_at: BlockNumber,
    /// Last heartbeat block.
    pub last_heartbeat: BlockNumber,
    /// Block when deregistration was requested (0 = not deregistering).
    pub deregister_at: BlockNumber,
}

// ============================================================================
// Task Types
// ============================================================================

/// On-chain task priority.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
    PartialOrd,
    Ord,
)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Type of compute workload.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub enum WorkloadType {
    /// X3 bytecode execution.
    #[default]
    X3Bytecode,
    /// Mempool simulation.
    MempoolSim,
    /// Route optimization.
    RouteOptimize,
    /// ML model training.
    ModelTraining,
    /// ZK proof generation.
    ZkProving,
    /// Arbitrage search.
    ArbitrageSearch,
    /// Custom workload.
    Custom,
}

/// On-chain task status.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub enum TaskStatus {
    /// Waiting for a contributor to claim.
    #[default]
    Pending,
    /// Assigned to a contributor.
    Assigned,
    /// Result submitted, awaiting verification.
    Verifying,
    /// Verified and completed.
    Completed,
    /// Failed execution or verification.
    Failed,
    /// Timed out.
    TimedOut,
    /// Cancelled by submitter.
    Cancelled,
}

/// A compute task submitted on-chain.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct SwarmTask<AccountId, Balance, BlockNumber> {
    /// Task identifier (hash).
    pub id: TaskId,
    /// Account that submitted the task.
    pub submitter: AccountId,
    /// Workload type.
    pub workload_type: WorkloadType,
    /// Hash of the payload data (actual data stored off-chain).
    pub payload_hash: H256,
    /// Task priority.
    pub priority: TaskPriority,
    /// Reward offered for completion.
    pub reward: Balance,
    /// Current status.
    pub status: TaskStatus,
    /// Minimum VRAM required (MB).
    pub min_vram_mb: u32,
    /// Minimum compute score required.
    pub min_compute_score: u32,
    /// Required verification count.
    pub verification_count: u8,
    /// Block when submitted.
    pub submitted_at: BlockNumber,
    /// Deadline block (0 = no deadline).
    pub deadline: BlockNumber,
    /// Assigned contributor (if any).
    pub assigned_to: Option<ContributorId>,
    /// Block when assigned.
    pub assigned_at: Option<BlockNumber>,
}

// ============================================================================
// Result / Verification Types
// ============================================================================

/// A submitted task result.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct TaskResult<AccountId, BlockNumber> {
    /// The task this result is for.
    pub task_id: TaskId,
    /// Contributor who executed the task.
    pub contributor_id: ContributorId,
    /// Account of the contributor.
    pub executor: AccountId,
    /// Hash of execution output.
    pub result_hash: H256,
    /// Compute units consumed.
    pub compute_units_used: u64,
    /// Block when result was submitted.
    pub submitted_at: BlockNumber,
}

/// Jury vote commitment (commit-reveal scheme).
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct VoteCommitment<AccountId, BlockNumber> {
    /// Voter account.
    pub voter: AccountId,
    /// Commitment hash: H(vote || nonce).
    pub commitment: H256,
    /// Block when committed.
    pub committed_at: BlockNumber,
}

/// Revealed jury vote.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct VoteReveal<AccountId, BlockNumber> {
    /// Voter account.
    pub voter: AccountId,
    /// The vote (true = valid, false = invalid).
    pub vote: bool,
    /// Nonce used in commitment.
    pub nonce: H256,
    /// Block when revealed.
    pub revealed_at: BlockNumber,
}

/// Jury verification session.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct JurySession<BlockNumber> {
    /// Session identifier.
    pub id: SessionId,
    /// Task being verified.
    pub task_id: TaskId,
    /// Session phase.
    pub phase: JuryPhase,
    /// Number of commitments received.
    pub commit_count: u32,
    /// Number of reveals received.
    pub reveal_count: u32,
    /// Votes for valid.
    pub yes_votes: u32,
    /// Votes for invalid.
    pub no_votes: u32,
    /// Block when session started.
    pub started_at: BlockNumber,
    /// Block when commit phase ends.
    pub commit_deadline: BlockNumber,
    /// Block when reveal phase ends.
    pub reveal_deadline: BlockNumber,
}

/// Phase of a jury session.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Debug,
    Default,
)]
pub enum JuryPhase {
    /// Accepting vote commitments.
    #[default]
    Commit,
    /// Accepting vote reveals.
    Reveal,
    /// Session finalized.
    Closed,
}

/// Swarm configuration parameters.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Debug)]
pub struct SwarmConfig<Balance, BlockNumber> {
    /// Minimum stake to register as contributor.
    pub min_stake: Balance,
    /// Heartbeat interval (blocks). Contributors must heartbeat within this window.
    pub heartbeat_interval: BlockNumber,
    /// Unstaking cooldown period (blocks).
    pub unstake_cooldown: BlockNumber,
    /// Default task timeout (blocks).
    pub default_task_timeout: BlockNumber,
    /// Commit phase duration (blocks).
    pub commit_phase_duration: BlockNumber,
    /// Reveal phase duration (blocks).
    pub reveal_phase_duration: BlockNumber,
    /// Contributor reward share (percentage, out of 100).
    pub contributor_reward_pct: u8,
    /// Protocol fee share (percentage, out of 100).
    pub protocol_fee_pct: u8,
    /// Slash amount for failed/dishonest behavior.
    pub slash_amount: Balance,
    /// Maximum concurrent tasks per contributor.
    pub max_tasks_per_contributor: u32,
}

/// Swarm statistics for runtime API.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
pub struct SwarmStats {
    pub total_contributors: u32,
    pub active_contributors: u32,
    pub total_tasks_submitted: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub pending_tasks: u32,
    pub active_jury_sessions: u32,
}
