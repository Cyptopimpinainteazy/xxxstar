//! Configuration for X3 GPU Validator Swarm

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for the X3 GPU Validator Swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Validator identity configuration
    pub identity: IdentityConfig,

    /// Network configuration
    pub network: NetworkConfig,

    /// GPU configuration
    pub gpu: GpuConfig,

    /// Verification configuration
    pub verification: VerificationConfig,

    /// Quarantine configuration
    pub quarantine: QuarantineConfig,

    /// Telemetry configuration
    pub telemetry: TelemetryConfig,

    /// Benchmark configuration
    pub benchmark: BenchmarkConfig,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig::default(),
            network: NetworkConfig::default(),
            gpu: GpuConfig::default(),
            verification: VerificationConfig::default(),
            quarantine: QuarantineConfig::default(),
            telemetry: TelemetryConfig::default(),
            benchmark: BenchmarkConfig::default(),
        }
    }
}

impl SwarmConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, crate::SwarmError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| crate::SwarmError::ConfigError(e.to_string()))
    }

    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &std::path::Path) -> Result<(), crate::SwarmError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::SwarmError::ConfigError(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Validator identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Path to the validator's keypair file
    pub keypair_path: PathBuf,

    /// Validator's display name
    pub display_name: Option<String>,

    /// Validator's region (for locality-aware scheduling)
    pub region: Option<String>,

    /// Validator's stake amount (in X3 tokens)
    pub stake_amount: u64,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            keypair_path: PathBuf::from("~/.x3-validator/validator.key"),
            display_name: None,
            region: None,
            stake_amount: 1000,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen addresses for P2P
    pub listen_addresses: Vec<String>,

    /// Bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<String>,

    /// Orchestrator endpoint (for centralized mode)
    pub orchestrator_endpoint: Option<String>,

    /// Maximum number of peer connections
    pub max_peers: usize,

    /// Enable mDNS for local discovery
    pub enable_mdns: bool,

    /// Enable Kademlia DHT
    pub enable_dht: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/30334".to_string(),
                "/ip4/0.0.0.0/udp/30334/quic-v1".to_string(),
            ],
            bootstrap_nodes: vec![],
            orchestrator_endpoint: Some("https://orchestrator.x3.x3-chain.io".to_string()),
            max_peers: 100,
            enable_mdns: true,
            enable_dht: true,
        }
    }
}

/// GPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// Enable CUDA backend
    pub enable_cuda: bool,

    /// Enable OpenCL backend
    pub enable_opencl: bool,

    /// GPU device indices to use (empty = all available)
    pub device_indices: Vec<u32>,

    /// Maximum memory usage per GPU (MB, 0 = unlimited)
    pub max_memory_mb: u64,

    /// GPU power limit percentage (0-100, 0 = no limit)
    pub power_limit_percent: u8,

    /// Enable deterministic mode (slower but reproducible)
    pub deterministic_mode: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            enable_cuda: true,
            enable_opencl: false,
            device_indices: vec![],
            max_memory_mb: 0,
            power_limit_percent: 0,
            deterministic_mode: true,
        }
    }
}

/// Verification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Enable CPU verification
    pub cpu_verification_enabled: bool,

    /// Enable replay mode for divergence detection
    pub replay_mode_enabled: bool,

    /// Number of replay attempts before quarantining
    pub max_replay_attempts: u32,

    /// Verification level (Basic, Standard, Strict)
    pub verification_level: VerificationLevelConfig,

    /// Timeout for verification (seconds)
    pub verification_timeout_secs: u64,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            cpu_verification_enabled: true,
            replay_mode_enabled: true,
            max_replay_attempts: 3,
            verification_level: VerificationLevelConfig::Standard,
            verification_timeout_secs: 60,
        }
    }
}

/// Verification level configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum VerificationLevelConfig {
    /// Only verify first and last result
    Basic,
    /// Verify all results
    #[default]
    Standard,
    /// Verify all results with multiple CPU implementations
    Strict,
}

/// Quarantine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineConfig {
    /// Enable quarantine system
    pub enabled: bool,

    /// Quarantine duration (seconds)
    pub quarantine_duration_secs: u64,

    /// Maximum divergence count before permanent ban
    pub max_divergence_count: u32,

    /// Enable automatic fallback to CPU on divergence
    pub auto_fallback_cpu: bool,
}

impl Default for QuarantineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            quarantine_duration_secs: 1800,
            max_divergence_count: 3,
            auto_fallback_cpu: true,
        }
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable telemetry
    pub enabled: bool,

    /// Telemetry endpoint
    pub endpoint: Option<String>,

    /// Telemetry interval (seconds)
    pub interval_secs: u64,

    /// Include detailed metrics
    pub detailed_metrics: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: Some("https://telemetry.x3.x3-chain.io".to_string()),
            interval_secs: 30,
            detailed_metrics: false,
        }
    }
}

/// Benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Enable benchmarking
    pub enabled: bool,

    /// Benchmark output directory
    pub output_dir: PathBuf,

    /// Number of iterations for benchmarks
    pub iterations: u32,

    /// Batch sizes to test
    pub batch_sizes: Vec<usize>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output_dir: PathBuf::from("./benchmark-results"),
            iterations: 100,
            batch_sizes: vec![1, 10, 100, 1000, 10000],
        }
    }
}

/// Configuration for a single validator (loaded from orchestrator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Validator ID
    pub validator_id: String,

    /// Validator address
    pub address: String,

    /// Stake amount
    pub stake_amount: u64,

    /// Region
    pub region: Option<String>,

    /// GPU capabilities
    pub gpu_capabilities: GpuCapabilities,
}

/// GPU capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapabilities {
    /// Number of GPUs
    pub gpu_count: u32,

    /// Total memory (MB)
    pub total_memory_mb: u64,

    /// Compute capability
    pub compute_capability: (u32, u32),

    /// Supported operations
    pub supported_operations: Vec<String>,
}
