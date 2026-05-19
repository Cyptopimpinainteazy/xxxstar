//! X3 GPU Validator Swarm
//!
//! A deterministic GPU validator swarm with CPU verification, replay mode,
//! and quarantine/fallback mechanisms for production deployments.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    X3 GPU Validator Swarm                                │
//! │  ┌─────────────────────────────────────────────────────────────────────┐ │
//! │  │                     Swarm Orchestrator                               │ │
//! │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐  │ │
//! │  │  │ Task Queue  │  │ Scheduler   │  │ Verification Engine         │  │ │
//! │  │  └──────┬──────┘  └──────┬──────┘  └───────────┬───────────────┘  │ │
//! │  └─────────┼────────────────┼─────────────────────┼──────────────────┘ │
//! │            │                │                     │                     │
//! │      ┌─────▼─────┬──────────▼──────────┬──────────▼─────┐             │
//! │      │           │                     │                │             │
//! │  ┌───▼───┐   ┌───▼───┐            ┌────▼────┐      ┌────▼────┐        │
//! │  │Validator│  │Validator│   ...    │Validator│      │Validator│        │
//! │  │GPU:A   │  │GPU:B   │            │GPU:X    │      │GPU:Y    │        │
//! │  └───┬───┘   └───┬───┘            └────┬────┘      └────┬────┘        │
//! │      │           │                     │                │             │
//! │      └───────────┴─────────────────────┴────────────────┘             │
//! │                         │                                             │
//! │              ┌───────────▼───────────┐                                  │
//! │              │  CPU Verification    │                                  │
//! │              │  + Replay Mode       │                                  │
//! │              └─────────────────────┘                                  │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Deterministic GPU Execution**: Bit-for-bit deterministic outputs
//! - **CPU Verification**: Every GPU result verified by CPU
//! - **Replay Mode**: Re-run computation for divergence detection
//! - **Quarantine System**: Isolate misbehaving validators
//! - **Fallback Mechanism**: Automatic CPU fallback on divergence
//! - **Swarm Orchestration**: Coordinate multiple validators
//! - **One-Command Onboarding**: Install, run, join, and benchmark with single commands
//! - **JSON Benchmarks**: Machine-readable performance reports
//! - **Full Telemetry**: Prometheus metrics and health monitoring

pub mod config;
pub mod cpu_validator;
pub mod crypto;
pub mod deterministic;
pub mod error;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod gpu_bytecode;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod gpu_fallback_chain;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod gpu_memory_pool;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod gpu_receipt;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod health;
pub mod metrics;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod multi_gpu_dispatcher;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod network;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod orchestrator;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod payment;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod proof_aggregator;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod proof_integration;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod protocol;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod quarantine;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod state_merkle_proof;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod telemetry;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod unified_proof;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod validator;
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub mod x3_kernel_versioning;

pub use config::{SwarmConfig, ValidatorConfig};
pub use cpu_validator::{
    validate_cpu, validate_cpu_batch, validate_cpu_with, CpuTaskResult, CpuValidator,
    CpuValidatorMetrics, EasyCpuValidator,
};
pub use crypto::{
    blake2b, compute_hash, keccak256, keccak256_batch, sha256, HashAlgorithm,
    HashAlgorithm as CryptoHashAlgorithm, HashOutput, SignatureOutput, VerificationResult,
};
pub use deterministic::{DeterministicEngine, ExecutionMode, VerificationLevel};
pub use error::{SwarmError, SwarmResult};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use gpu_fallback_chain::{DegradationStrategy, FallbackChain, FallbackStats};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use gpu_memory_pool::{GpuMemoryManager, GpuMemoryPool, MemoryPoolStats, SlabHandle};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use gpu_receipt::{GpuClass, GpuReceipt, GpuReceiptValidator, ProofType};
pub use metrics::{HealthCheck, HealthStatus, MetricsCollector, SwarmMetrics, ValidatorHealth};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use multi_gpu_dispatcher::{GpuDeviceInfo, JobResult, MultiGpuDispatcher, PerformanceStats};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use network::{
    Network, NetworkConfig, NetworkEvent, NetworkManager, NetworkMessage, NetworkPeer,
};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use orchestrator::{OrchestratorEvent, SwarmOrchestrator};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use payment::{PaymentSystem, ProviderAccount, ProviderStatus, WorkRecord, WorkType};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use proof_aggregator::{AggregationState, AggregatorStats, ProofAggregator};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use protocol::{SwarmMessage, TaskAssignment, TaskResult, ValidatorMessage, ValidatorProof};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use quarantine::{DivergenceRecord, QuarantineManager, QuarantineReason};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use state_merkle_proof::{
    compute_merkle_root, generate_merkle_proof, MerkleNode, MerkleProofPath, StateMerkleProof,
    StateRootVerification,
};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use telemetry::{TelemetryConfig, TelemetrySink};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use unified_proof::{
    AtomicVmProof, ByzantineConsensus, GpuValidatorAttestation, ProofHeader, ProofValidationResult,
    UnifiedProof,
};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use validator::{Validator, ValidatorEvent, ValidatorState};
#[cfg(any(
    feature = "cuda",
    feature = "opencl",
    feature = "metal",
    feature = "vulkan"
))]
pub use x3_kernel_versioning::{X3KernelManifest, X3KernelRegistry, X3KernelRuntime};

/// Current version of the X3 GPU Validator Swarm protocol
pub const PROTOCOL_VERSION: u32 = 3;

/// Maximum task payload size (16 MB)
pub const MAX_TASK_SIZE: usize = 16 * 1024 * 1024;

/// Default task timeout (5 minutes)
pub const DEFAULT_TASK_TIMEOUT_SECS: u64 = 300;

/// Minimum stake required to participate as a validator (in X3 tokens)
pub const MIN_VALIDATOR_STAKE: u64 = 1000;

/// Maximum number of validators in the swarm
pub const MAX_VALIDATORS: usize = 256;

/// Quarantine duration for divergence (30 minutes)
pub const QUARANTINE_DURATION_SECS: u64 = 1800;

/// Maximum replay attempts before permanent quarantine
pub const MAX_REPLAY_ATTEMPTS: u32 = 3;
