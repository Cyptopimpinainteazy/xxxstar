use clap::{Args, Parser, Subcommand};
use sc_cli::{
    ChainSpec, CheckBlockCmd, ExportBlocksCmd, ExportStateCmd, ImportBlocksCmd, PurgeChainCmd,
    RevertCmd, RunCmd, SubstrateCli,
};
use sp_core::H256;
use std::path::PathBuf;

/// Command line options for the X3 Chain node binary.
///
/// This CLI mirrors the ergonomics of other Substrate-based chains while
/// highlighting the dual-VM X3 Chain architecture.
#[derive(Debug, Parser)]
#[command(
    name = "X3 Chain Node",
    bin_name = "x3-chain-node",
    author,
    version,
    about = "Run and manage the 3i X3 Chain L1 blockchain node",
    long_about = "X3 Chain is a dual-VM (EVM + SVM) Layer-1 built \
    for atomic cross-chain operations and native asset orchestration. \
    Use this CLI to operate validator, collator, and archival nodes, or \
    to inspect and craft chain specifications.",
    propagate_version = true,
    disable_help_subcommand = true,
    next_display_order = None
)]
pub struct Cli {
    /// Subcommand invoked by the user.
    #[command(subcommand)]
    pub subcommand: Option<Commands>,

    /// Run command parameters shared with most subcommands.
    #[command(flatten)]
    pub run: RunCmd,

    /// Node-level feature gates for staged production rollout.
    #[command(flatten)]
    pub features: NodeFeatureFlags,
}

#[derive(Debug, Clone, Args, Default)]
/// Feature flags controlling optional node behavior.
///
/// IMPORTANT: All feature flags default to OFF for stability. Enable only after
/// thorough testing and validation in canary/staging environments.
///
/// Example usage:
///   x3-chain-node --enable-flash-finality --enable-poh
///   CHAIN_SPEC=staging x3-chain-node --enable-flash-finality
pub struct NodeFeatureFlags {
    /// Enable the parallel proposer pipeline.
    ///
    /// The parallel proposer pipeline allows multiple block proposals to be
    /// processed concurrently, improving block authorship throughput. Currently
    /// in SHADOW MODE - enable only for testing, not for block inclusion.
    ///
    /// Requirements:
    /// - Deterministic scheduler enabled in runtime
    /// - Min 4 CPU cores recommended
    ///
    /// Default: false
    #[arg(long, default_value_t = false)]
    pub enable_parallel_proposer: bool,

    /// Enable Flash Finality tasks.
    ///
    /// Flash Finality is an alternative finality mechanism providing faster
    /// consensus commitment than GRANDPA. When enabled, GRANDPA is automatically
    /// disabled. Operates in SHADOW MODE by default - consensus remains driven
    /// by GRANDPA while Flash Finality runs in parallel for testing.
    ///
    /// Mutually exclusive with GRANDPA. Set --disable-grandpa=false to run both.
    ///
    /// Requirements:
    /// - Native support for flash finality certificates
    /// - Network with 2/3+ validators supporting Flash Finality
    ///
    /// Default: false
    #[arg(long, default_value_t = false)]
    pub enable_flash_finality: bool,

    /// Enable PoH (Proof of History) digest validation path.
    ///
    /// PoH digests provide verifiable time proofs integrated with block headers.
    /// When enabled, the node validates PoH digests during block import and
    /// includes PoH tickets in authored blocks. Currently in VALIDATION-ONLY mode.
    ///
    /// Requirements:
    /// - PoH generator service enabled
    /// - Additional ~50ms latency per block for validation
    ///
    /// Default: false
    #[arg(long, default_value_t = false)]
    pub enable_poh: bool,

    /// Enable atomic kernel/transaction orchestration paths.
    ///
    /// This flag enables the runtime atomic kernel pallet and sequencer
    /// behavior for multi-VM atomic settlement while keeping the feature
    /// gate in explicit control for staged rollouts.
    #[arg(long, default_value_t = false)]
    pub enable_atomic_kernel: bool,

    /// Require GPU path for validation critical flows.
    ///
    /// When set to true, the node will enforce GPU execution for performance-critical
    /// operations. If no GPU is available, validation will fail. This is useful for
    /// performance-constrained deployments or to guarantee consistent hardware profiles.
    ///
    /// WARNING: Set to true only if GPU is guaranteed to be available. Defaults to
    /// CPU fallback for safe operation.
    ///
    /// Requirements:
    /// - Compatible NVIDIA GPU (compute capability 6.0+) or AMD GPU
    /// - CUDA 11.0+ or ROCm 4.0+ installed
    ///
    /// Default: false
    #[arg(long, default_value_t = false)]
    pub gpu_required: bool,

    /// Enable GPU validator orchestrator (requires gpu-validator feature).
    ///
    /// When enabled, the node spawns the GPU validator swarm orchestrator to handle
    /// GPU-accelerated cryptographic validation (ed25519, sr25519, keccak256, secp256k1).
    /// Requires the gpu-validator feature to be enabled at compile time.
    ///
    /// The orchestrator coordinates multiple GPU validators, manages quarantine,
    /// provides failover to CPU, and exposes health/metrics endpoints via gRPC.
    ///
    /// Requirements:
    /// - gpu-validator feature enabled in Cargo.toml
    /// - Compatible NVIDIA GPU (A100, H100, RTX 4090) or similar
    /// - CUDA 11.0+ or cuDNN 8.0+
    ///
    /// Default: false
    #[arg(long, default_value_t = false)]
    pub enable_gpu_validator: bool,

    /// EVM bridge escrow contract address (20-byte hex, with or without 0x prefix).
    ///
    /// This address is written into genesis and stored in on-chain state.
    /// It overrides the default placeholder address compiled into the runtime.
    /// Must be set to the deployed EVM bridge escrow contract before mainnet launch.
    ///
    /// Example: --evm-escrow-addr 0xdead000000000000000000000000000000000001
    #[arg(long)]
    pub evm_escrow_addr: Option<String>,

    /// SVM bridge escrow program address (32-byte pubkey, hex or base58).
    ///
    /// This address is written into genesis and stored in on-chain state.
    /// It overrides the default placeholder address compiled into the runtime.
    /// Must be set to the deployed SVM bridge escrow program before mainnet launch.
    ///
    /// Example: --svm-escrow-addr 5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d
    #[arg(long)]
    pub svm_escrow_addr: Option<String>,
}

/// X3 Chain node subcommands.
///
/// These commands provide access to node lifecycle management, chain
/// specification authoring, and runtime state inspection routines.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Build a chainspec to bootstrap new networks or inspect configuration.
    BuildSpec(sc_cli::BuildSpecCmd),
    /// Validate blocks against the runtime execution logic.
    CheckBlock(CheckBlockCmd),
    /// Export blocks to a file for archival or debugging purposes.
    ExportBlocks(ExportBlocksCmd),
    /// Export full runtime state at a given block into a snapshot file.
    ExportState(ExportStateCmd),
    /// Import blocks from a file into the local database.
    ImportBlocks(ImportBlocksCmd),
    /// Remove the local database (be careful!).
    PurgeChain(PurgeChainCmd),
    /// Revert the chain to a previous state.
    Revert(RevertCmd),
    /// Run built-in benchmarking harnesses.
    #[cfg(feature = "runtime-benchmarks")]
    #[command(subcommand)]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
    /// Execute try-runtime checks against on-chain state.
    #[cfg(feature = "try-runtime")]
    TryRuntime(try_runtime_cli::TryRuntimeCmd),
    /// Execute try-runtime checks against on-chain state.
    #[cfg(not(feature = "try-runtime"))]
    TryRuntime,
    /// Atomic swap simulation and execution commands.
    AtomicSwap(AtomicSwapCmd),
    /// Comit transaction commands for dual-VM execution.
    Comit(ComitCmd),
    /// Key management commands for validator and account keys.
    Keys(KeysCmd),
    /// Inspect canonical ledger and chain state.
    Inspect(InspectCmd),
}

/// Comit transaction CLI commands for dual-VM execution.
///
/// These commands enable submitting and querying Comit transactions
/// that execute atomically across EVM and SVM virtual machines.
#[derive(Debug, Args)]
pub struct ComitCmd {
    /// The comit subcommand to execute.
    #[command(subcommand)]
    pub command: ComitSubcommand,
}

/// Comit subcommands
#[derive(Debug, Subcommand)]
pub enum ComitSubcommand {
    /// Query a Comit transaction by its ID.
    ///
    /// Retrieves the status and details of a previously submitted Comit transaction.
    Query {
        /// Comit transaction ID (H256 hex string)
        #[arg(long, value_parser = parse_h256)]
        comit_id: H256,

        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },

    /// Query the canonical balance for an account and asset.
    ///
    /// Retrieves the current balance from the canonical ledger.
    Balance {
        /// Account address (SS58 or hex format)
        #[arg(long)]
        account: String,

        /// Asset ID to query (default: 0 for native X3)
        #[arg(long, default_value = "0")]
        asset_id: u32,

        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },

    /// List authorized accounts that can submit Comit transactions.
    Authorized {
        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },
}

/// Key management CLI commands for validator and account keys.
///
/// These commands provide utilities for managing cryptographic keys
/// used by the X3 Chain node for block authoring, finality, and
/// cross-VM operations.
#[derive(Debug, Args)]
pub struct KeysCmd {
    /// The keys subcommand to execute.
    #[command(subcommand)]
    pub command: KeysSubcommand,
}

/// Keys subcommands
#[derive(Debug, Subcommand)]
pub enum KeysSubcommand {
    /// Generate a new keypair for the specified key type.
    ///
    /// Supported key types: aura (sr25519), grandpa (ed25519), imonline (sr25519).
    Generate {
        /// Key type to generate (aura, grandpa, imonline)
        #[arg(long, default_value = "aura")]
        key_type: String,

        /// Optional seed phrase (if not provided, a random keypair is generated)
        #[arg(long)]
        seed: Option<String>,

        /// Output format (json, hex, ss58)
        #[arg(long, default_value = "ss58")]
        output: String,
    },

    /// Insert a key into the node's keystore.
    ///
    /// This is useful for setting up validator keys without exposing
    /// the seed phrase in the node's command line arguments.
    Insert {
        /// Key type (aura, grandpa, imonline)
        #[arg(long)]
        key_type: String,

        /// Seed phrase or hex-encoded private key
        #[arg(long)]
        seed: String,

        /// Keystore path (defaults to node's default keystore)
        #[arg(long)]
        keystore_path: Option<PathBuf>,
    },

    /// List all keys in the node's keystore.
    ///
    /// Displays the public keys for each key type stored in the keystore.
    List {
        /// Keystore path (defaults to node's default keystore)
        #[arg(long)]
        keystore_path: Option<PathBuf>,
    },

    /// Verify a keypair by checking if the private key matches the public key.
    Verify {
        /// Key type (aura, grandpa, imonline)
        #[arg(long)]
        key_type: String,

        /// Public key (SS58 or hex format)
        #[arg(long)]
        public: String,

        /// Seed phrase or hex-encoded private key to verify
        #[arg(long)]
        seed: String,
    },
}

/// Inspect CLI commands for querying canonical ledger and chain state.
///
/// These commands provide read-only access to the X3 Chain's canonical ledger,
/// allowing users to query account balances, asset metadata, and other
/// chain state without running a full node.
#[derive(Debug, Args)]
pub struct InspectCmd {
    /// The inspect subcommand to execute.
    #[command(subcommand)]
    pub command: InspectSubcommand,
}

/// Inspect subcommands
#[derive(Debug, Subcommand)]
pub enum InspectSubcommand {
    /// Query the canonical ledger for an account.
    ///
    /// Displays all asset balances held by the specified account.
    Account {
        /// Account address (SS58 or hex format)
        #[arg(long)]
        account: String,

        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,

        /// Output format (json, table)
        #[arg(long, default_value = "table")]
        output: String,
    },

    /// Query asset metadata by asset ID.
    ///
    /// Displays the symbol, decimals, and other metadata for a registered asset.
    Asset {
        /// Asset ID to query
        #[arg(long)]
        asset_id: u32,

        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },

    /// List all registered assets.
    ///
    /// Displays all assets registered in the X3 Chain's asset registry.
    Assets {
        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,

        /// Output format (json, table)
        #[arg(long, default_value = "table")]
        output: String,
    },

    /// Query the current authority set.
    ///
    /// Displays the list of authorities responsible for block production.
    Authorities {
        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },

    /// Query chain information and status.
    ///
    /// Displays general chain information including block height, finality, and node status.
    ChainInfo {
        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },
}

/// Atomic swap CLI commands for simulating and executing cross-VM trades.
///
/// These commands provide offline simulation capabilities for AI agents
/// and frontends to preview trade execution before submitting transactions.
#[derive(Debug, Args)]
pub struct AtomicSwapCmd {
    /// The atomic swap subcommand to execute.
    #[command(subcommand)]
    pub command: AtomicSwapSubcommand,
}

/// Atomic swap subcommands
#[derive(Debug, Subcommand)]
pub enum AtomicSwapSubcommand {
    /// Simulate a trade path without execution.
    ///
    /// Returns estimated output, gas costs, price impact, and optimal route.
    /// Use this before submitting transactions to verify expected outcomes.
    Simulate {
        /// Input token (H256 hex string, e.g., 0x0001...0000)
        #[arg(long, value_parser = parse_h256)]
        token_in: H256,

        /// Output token (H256 hex string)
        #[arg(long, value_parser = parse_h256)]
        token_out: H256,

        /// Amount of input tokens (in smallest unit)
        #[arg(long)]
        amount: u128,

        /// Maximum slippage tolerance in basis points (default: 100 = 1%)
        #[arg(long, default_value = "100")]
        slippage_bps: u32,

        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },

    /// Get current price data for a token pair.
    Price {
        /// First token (H256 hex string)
        #[arg(long, value_parser = parse_h256)]
        token_a: H256,

        /// Second token (H256 hex string)
        #[arg(long, value_parser = parse_h256)]
        token_b: H256,

        /// RPC endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9933")]
        rpc_url: String,
    },

    /// Estimate execution costs for a multi-leg trade.
    EstimateCost {
        /// Number of trade legs
        #[arg(long)]
        legs: u32,

        /// VM types for each leg (comma-separated: evm,svm,crossvm)
        #[arg(long, value_delimiter = ',')]
        vm_types: Vec<String>,
    },
}

/// Parse H256 from hex string
fn parse_h256(s: &str) -> Result<H256, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("Invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("Expected 32 bytes, got {}", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(H256::from(arr))
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "X3 Chain Node".into()
    }

    fn impl_version() -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn description() -> String {
        "X3 Chain: Dual-VM (EVM + SVM) Layer-1 with atomic cross-chain primitives.".into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").replace(':', ", ")
    }

    fn support_url() -> String {
        "https://x3-chain.io/support".into()
    }

    fn copyright_start_year() -> i32 {
        2024
    }

    fn executable_name() -> String {
        "x3-chain-node".into()
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpec>, String> {
        let spec = match id {
            "" | "dev" => crate::chain_spec::development_config(),
            "local" => crate::chain_spec::local_testnet_config(),
            "local3" | "local-3" => crate::chain_spec::local_three_validator_config(),
            "staging" | "staging-net" => crate::chain_spec::staging_config(),
            "testnet" | "test-net" => crate::chain_spec::testnet_config(),
            "production" | "prod" => crate::chain_spec::production_config(),
            path => crate::chain_spec::ChainSpec::from_json_file(PathBuf::from(path)),
        }?;
        Ok(Box::new(spec))
    }
}
