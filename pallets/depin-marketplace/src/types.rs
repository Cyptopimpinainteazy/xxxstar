//! Types for the DePIN GPU Marketplace pallet.
//!
//! Proposal: DEPIN-GPU-001

use frame_support::pallet_prelude::ConstU32;
use frame_support::BoundedVec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

/// Maximum length for GPU model name in marketplace types.
pub const MAX_GPU_MODEL_LEN: u32 = 128;

/// Unique identifier for a marketplace job (128-bit).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Default,
)]
pub struct JobId(pub [u8; 16]);

/// GPU specification advertised by a provider.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct GpuSpecification {
    /// GPU model name (e.g., "NVIDIA A100 80GB")
    pub model: BoundedVec<u8, ConstU32<MAX_GPU_MODEL_LEN>>,
    /// Total VRAM in megabytes
    pub vram_mb: u32,
    /// Number of compute units / streaming multiprocessors
    pub compute_units: u32,
    /// GPU tier
    pub tier: GpuTier,
    /// Supports tensor cores
    pub tensor_cores: bool,
    /// Supports confidential computing (NVIDIA CC)
    pub confidential_compute: bool,
    /// Benchmark score (standardized, higher = better)
    pub benchmark_score: u32,
}

/// GPU tier classification.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum GpuTier {
    /// Consumer GPUs (RTX 30/40 series)
    Consumer,
    /// Prosumer GPUs (A4000, A6000)
    Prosumer,
    /// Datacenter GPUs (A100, H100, B200)
    Datacenter,
}

/// GPU requirements for a compute job.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct GpuRequirements {
    /// Minimum GPU tier
    pub min_tier: GpuTier,
    /// Minimum VRAM in megabytes
    pub min_vram_mb: u32,
    /// Minimum compute units
    pub min_compute_units: u32,
    /// Requires tensor cores
    pub requires_tensor_cores: bool,
    /// Requires confidential compute
    pub requires_confidential: bool,
}

/// Type of DePIN compute job.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum DePinJobType {
    /// AI model inference
    AiInference {
        /// Hash of the model to load
        model_hash: sp_core::H256,
        /// Expected input size in bytes
        input_size: u64,
    },
    /// Zero-knowledge proof generation
    ZkProving {
        /// Hash of the proof circuit
        circuit_hash: sp_core::H256,
        /// Witness data size in bytes
        witness_size: u64,
    },
    /// Video transcoding
    VideoTranscode {
        /// Source codec identifier
        source_codec: u8,
        /// Target codec identifier
        target_codec: u8,
        /// Target resolution (width × height)
        target_width: u16,
        target_height: u16,
    },
    /// Protein folding simulation
    ProteinFolding {
        /// Hash of the amino acid sequence
        sequence_hash: sp_core::H256,
    },
    /// Generic GPU compute workload
    GenericCompute {
        /// Hash of the workload binary/script
        workload_hash: sp_core::H256,
        /// Estimated compute units needed
        compute_units: u64,
    },
}

/// Provider status.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum ProviderStatus {
    /// Actively accepting jobs
    Active,
    /// Self-paused (not accepting new jobs)
    Paused,
    /// Preempted for block building
    BlockBuilding,
    /// Slashed below minimum reputation
    Slashed,
}

/// Reason for job failure.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum JobFailureReason {
    /// Execution error in the workload
    ExecutionError,
    /// GPU out-of-memory
    OutOfMemory,
    /// Job exceeded its deadline
    Timeout,
    /// Provider went offline
    ProviderOffline,
    /// Verification failed — result doesn't match re-execution
    VerificationFailed,
    /// Sandbox escape attempt detected
    SandboxViolation,
}

/// Job status within the marketplace.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum JobStatus {
    /// Waiting in order book for a provider
    Pending,
    /// Assigned to a provider, executing
    Executing,
    /// Execution complete, awaiting verification
    Verifying,
    /// Successfully completed
    Completed,
    /// Failed
    Failed,
    /// Cancelled by customer
    Cancelled,
}

/// Information about a registered provider.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
#[scale_info(skip_type_params(T))]
pub struct ProviderInfo<T: frame_system::Config> {
    /// Provider account
    pub account: T::AccountId,
    /// Staked amount
    pub stake: u128,
    /// GPU specifications
    pub gpu_specs: GpuSpecification,
    /// Price charged per compute unit
    pub price_per_compute_unit: u128,
    /// Reputation score (0–10_000)
    pub reputation: u32,
    /// Total jobs completed
    pub total_jobs_completed: u64,
    /// Total revenue earned
    pub total_revenue: u128,
    /// Current status
    pub status: ProviderStatus,
    /// Block when registered
    pub registered_at: BlockNumberFor<T>,
}

/// A pending order in the order book.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
#[scale_info(skip_type_params(T))]
pub struct Order<T: frame_system::Config> {
    /// Unique job ID
    pub job_id: JobId,
    /// Customer account
    pub customer: T::AccountId,
    /// Type of compute job
    pub job_type: DePinJobType,
    /// GPU requirements
    pub gpu_requirements: GpuRequirements,
    /// Maximum price willing to pay
    pub max_price: u128,
    /// Maximum duration in blocks
    pub duration_blocks: BlockNumberFor<T>,
    /// When the order was submitted
    pub submitted_at: BlockNumberFor<T>,
}

/// An active marketplace job.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
#[scale_info(skip_type_params(T))]
pub struct MarketplaceJob<T: frame_system::Config> {
    /// Unique ID
    pub id: JobId,
    /// Customer who submitted the job
    pub customer: T::AccountId,
    /// Type of compute job
    pub job_type: DePinJobType,
    /// GPU requirements
    pub gpu_requirements: GpuRequirements,
    /// Escrowed payment
    pub escrow: u128,
    /// Assigned provider (None if unassigned)
    pub assigned_provider: Option<T::AccountId>,
    /// Current status
    pub status: JobStatus,
    /// When submitted
    pub submitted_at: BlockNumberFor<T>,
    /// When assigned
    pub assigned_at: Option<BlockNumberFor<T>>,
    /// Deadline block
    pub deadline: BlockNumberFor<T>,
    /// Hash of the result (set on completion)
    pub result_hash: Option<sp_core::H256>,
    /// Compute units consumed
    pub compute_units_used: u64,
}

use frame_system::pallet_prelude::BlockNumberFor;
