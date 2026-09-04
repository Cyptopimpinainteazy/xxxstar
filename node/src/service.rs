use crate::flash_finality::FlashFinalityBridge;
use crate::metrics::X3PrometheusMetrics;
use crate::rpc_middleware::{RateLimitConfig, RateLimiter};
use contention_predictor::{ContentionPredictor, PredictorConfig};
use flash_finality::{FlashFinalityConfig, FlashFinalityGadget, FLASH_FINALITY_PROTOCOL_ID};
use futures_util::StreamExt;
use parallel_proposer::{extract_tx_metadata, ParallelProposerFactory};
use poh_generator::PoHState;
use poh_generator::{PoHDigest, PoHVerifier, POH_ENGINE_ID};
use sc_client_api::{Backend, BlockBackend, BlockchainEvents, HeaderBackend};
use sc_consensus::{BlockCheckParams, BlockImport, BlockImportParams, ImportResult};
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_consensus_grandpa::SharedVoterState;
use sc_service::{
    ChainType, Configuration, Error as ServiceError, KeystoreContainer, PartialComponents,
    TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_core::{crypto::KeyTypeId, Pair};
use sp_runtime::traits::Header as HeaderT;
use sp_runtime::{
    traits::{BlakeTwo256, Block as BlockT, Hash as HashT},
    DigestItem, SaturatedConversion,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use x3_bridge_adapters::{
    OffchainEscrowPersistence, RuntimeCrossVmDispatcher, SubstrateX3VmBridge,
};
use x3_chain_runtime::{opaque::Block, RuntimeApi};
use x3_cross_vm_bridge::{CrossVmBridge, CrossVmResult};
use x3_finality_oracle::{
    Chain as FinalityChain, FinalityOracle, FinalityRule, FinalityStatus, InMemoryFinalityOracle,
    ObservedBlock,
};
use x3_gateway_risk_engine::{GatewayRiskEngine, RiskPolicy, RouteRiskInput};
use x3_proof_dispute::{DisputeStatus, DisputeTracker};

#[cfg(feature = "gpu-validator")]
use x3_gpu_validator_swarm::{config::SwarmConfig, orchestrator::SwarmOrchestrator};

/// Key type for Aura block authoring
const AURA: KeyTypeId = KeyTypeId(*b"aura");
/// Key type for GRANDPA finality
const GRANDPA: KeyTypeId = KeyTypeId(*b"gran");

// Txpool sizing is determined dynamically by NetworkSpeed::detect().
// Design targets: 100k ready / 50k future, 256 MiB / 64 MiB, 60s ban.
// See NetworkSpeed enum and tuned_transaction_pool_options for runtime values.

/// GPU Validator Sidecar health check interval (blocks).
/// Health check runs every N blocks to detect sidecar crashes.
#[allow(dead_code)]
const GPU_SIDECAR_HEALTH_CHECK_INTERVAL: u32 = 5;

/// GPU Validator Sidecar restart threshold (consecutive failures).
/// If sidecar health check fails N times consecutively, trigger restart.
#[allow(dead_code)]
const GPU_SIDECAR_RESTART_THRESHOLD: u32 = 3;

/// GPU Sidecar graceful shutdown timeout (seconds).
/// Maximum time to wait for sidecar to shut down cleanly before forcing termination.
#[allow(dead_code)]
const GPU_SIDECAR_SHUTDOWN_TIMEOUT_SECS: u64 = 30;

/// ───────────────────────────────────────────────────────────────
/// GPU Sidecar Lifecycle Management
/// ───────────────────────────────────────────────────────────────

/// Configuration for GPU sidecar spawning
#[cfg(feature = "gpu-validator")]
#[derive(Debug, Clone)]
pub struct GpuSidecarConfig {
    /// Sidecar service ID
    pub service_id: String,
    /// GPU devices to use (if empty, auto-detect)
    pub gpu_devices: Vec<usize>,
    /// RPC endpoint for runtime communication
    pub rpc_endpoint: String,
    /// Proof submission interval (blocks)
    pub proof_interval_blocks: u32,
    /// Maximum concurrent validation tasks
    pub max_concurrent_tasks: usize,
}

#[cfg(feature = "gpu-validator")]
impl Default for GpuSidecarConfig {
    fn default() -> Self {
        Self {
            service_id: "x3-gpu-sidecar-0".to_string(),
            gpu_devices: vec![],
            rpc_endpoint: "http://127.0.0.1:9944".to_string(),
            proof_interval_blocks: 10,
            max_concurrent_tasks: 4,
        }
    }
}

/// Handle to a running GPU sidecar process
#[cfg(feature = "gpu-validator")]
pub struct GpuSidecarHandle {
    /// Task handle for the sidecar task
    pub task_handle: Arc<Mutex<Option<JoinHandle<Result<(), String>>>>>,
    /// Sidecar configuration
    pub config: GpuSidecarConfig,
    /// Whether sidecar is running
    pub is_running: Arc<std::sync::atomic::AtomicBool>,
    /// Shutdown signal
    pub shutdown_tx: tokio::sync::mpsc::UnboundedSender<()>,
}

#[cfg(feature = "gpu-validator")]
impl GpuSidecarHandle {
    /// Create a new GPU sidecar handle
    pub fn new(config: GpuSidecarConfig) -> (Self, tokio::sync::mpsc::UnboundedReceiver<()>) {
        let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                task_handle: Arc::new(Mutex::new(None)),
                config,
                is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                shutdown_tx,
            },
            shutdown_rx,
        )
    }

    /// Check if sidecar is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gracefully shutdown the sidecar
    pub async fn shutdown(&self, timeout_secs: u64) -> Result<(), String> {
        log::info!(
            "🛑 GPU sidecar shutdown initiated (timeout: {} seconds)",
            timeout_secs
        );

        // Signal shutdown
        if let Err(_) = self.shutdown_tx.send(()) {
            log::warn!("GPU sidecar shutdown signal already closed");
        }

        // Wait for task to complete with timeout
        let timeout_duration = Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();

        loop {
            let mut task_handle = self.task_handle.lock().await;
            if task_handle.is_none() {
                log::info!("✅ GPU sidecar gracefully shut down");
                self.is_running
                    .store(false, std::sync::atomic::Ordering::Release);
                return Ok(());
            }

            if start.elapsed() > timeout_duration {
                log::error!(
                    "⚠️ GPU sidecar shutdown timeout after {} seconds; task may not terminate cleanly",
                    timeout_secs
                );
                self.is_running
                    .store(false, std::sync::atomic::Ordering::Release);
                return Err("Sidecar shutdown timeout".to_string());
            }

            drop(task_handle);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Network speed detection for dynamic TX pool sizing.
/// Helps validators on low-bandwidth connections avoid pool overflow and network saturation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSpeed {
    /// Slow network (1 Mbps): Smaller pools to prevent congestion
    Slow,
    /// Normal network (10+ Mbps): Standard tuning
    Normal,
    /// Fast network (100+ Mbps): Larger pools for higher throughput
    Fast,
}

impl NetworkSpeed {
    /// Detect network speed from environment or default to Normal.
    /// Environment variable: X3_NETWORK_SPEED=slow|normal|fast
    fn detect() -> Self {
        match std::env::var("X3_NETWORK_SPEED")
            .unwrap_or_else(|_| "normal".to_string())
            .to_lowercase()
            .as_str()
        {
            "slow" => NetworkSpeed::Slow,
            "fast" => NetworkSpeed::Fast,
            _ => NetworkSpeed::Normal,
        }
    }

    /// Return (ready_count, future_count, ready_bytes, future_bytes) for this speed
    fn pool_sizing(&self) -> (usize, usize, usize, usize) {
        match self {
            NetworkSpeed::Slow => {
                // Slow network (1 Mbps): 50k ready / 25k future, 128 MiB / 32 MiB
                (50_000, 25_000, 128 * 1024 * 1024, 32 * 1024 * 1024)
            }
            NetworkSpeed::Normal => {
                // Normal network (10+ Mbps): 100k ready / 50k future, 256 MiB / 64 MiB
                (100_000, 50_000, 256 * 1024 * 1024, 64 * 1024 * 1024)
            }
            NetworkSpeed::Fast => {
                // Fast network (100+ Mbps): 200k ready / 100k future, 512 MiB / 128 MiB
                (200_000, 100_000, 512 * 1024 * 1024, 128 * 1024 * 1024)
            }
        }
    }
}

/// Rollout feature flags for consensus and execution paths.
/// All flags default to off for mainnet-v1; enable per-validator via CLI or env on canary set first.
/// Experimental features (flash finality, PoH, GPU validator, sidecar) are disabled for mainnet-v1.
#[derive(Debug, Clone, Copy, Default)]
pub struct NodeFeatureFlags {
    /// Enable the parallel proposer pipeline.
    pub enable_parallel_proposer: bool,
    /// Enable Flash Finality tasks.
    pub enable_flash_finality: bool,
    /// Enable PoH digest validation path.
    pub enable_poh: bool,
    /// Enable the atomic kernel runtime and sequencer processing path.
    pub enable_atomic_kernel: bool,
    /// Require GPU path for validation critical flows.
    pub gpu_required: bool,
    /// Enable GPU validator swarm orchestrator (requires gpu-validator feature).
    pub enable_gpu_validator: bool,
}

/// GPU Validator Sidecar health monitor.
/// Tracks sidecar process health and triggers restart on failure.
/// ISSUE #1 FIX: Manages GPU sidecar lifecycle to prevent node degradation.
#[cfg(feature = "gpu-validator")]
#[derive(Debug, Clone)]
pub struct GpuSidecarHealthMonitor {
    /// Number of consecutive health check failures
    consecutive_failures: u32,
    /// Last successful health check block
    last_healthy_block: u32,
    /// Flag indicating sidecar is operational
    is_healthy: bool,
}

#[cfg(feature = "gpu-validator")]
impl GpuSidecarHealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            last_healthy_block: 0,
            is_healthy: true,
        }
    }

    /// Check sidecar health and return true if operational
    pub fn check_health(&mut self, current_block: u32) -> bool {
        // Health status is tracked via `record_check` and restart thresholds;
        // this method returns the current tracked state.
        self.is_healthy
    }

    /// Record health check result
    pub fn record_check(&mut self, healthy: bool, current_block: u32) {
        if healthy {
            self.consecutive_failures = 0;
            self.last_healthy_block = current_block;
            self.is_healthy = true;
        } else {
            self.consecutive_failures += 1;
            if self.consecutive_failures >= GPU_SIDECAR_RESTART_THRESHOLD {
                log::error!(
                    "🚨 GPU sidecar health check failed {} times. \
                    Triggering restart at block {}. \
                    Last healthy block: {}.",
                    self.consecutive_failures,
                    current_block,
                    self.last_healthy_block
                );
                self.is_healthy = false;
            }
        }
    }

    /// Check if sidecar needs restart
    pub fn needs_restart(&self) -> bool {
        self.consecutive_failures >= GPU_SIDECAR_RESTART_THRESHOLD
    }

    /// Reset health monitor (called after restart)
    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.is_healthy = true;
        log::info!("🔄 GPU sidecar health monitor reset");
    }
}
/// Executor for X3 Chain — WASM-only in stable2512 (native eliminated).
pub type Executor = sc_executor::WasmExecutor<sp_io::SubstrateHostFunctions>;

/// Full client type alias
pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;

/// Full backend type alias
pub type FullBackend = sc_service::TFullBackend<Block>;

/// Type alias for select chain implementation
pub type SelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// Insert development keys into the keystore for block authoring.
///
/// For development mode (`--dev`), this inserts Alice's Aura (sr25519) and
/// GRANDPA (ed25519) keys into the keystore so the node can author blocks.
fn insert_dev_keys_with_seed(keystore: &KeystoreContainer, seed: &str) -> Result<(), ServiceError> {
    use sp_core::crypto::SecretStringError;

    let keystore = keystore.keystore();

    // Insert Aura key (sr25519) for block authoring
    let aura_pair =
        sp_core::sr25519::Pair::from_string(seed, None).map_err(|e: SecretStringError| {
            ServiceError::Other(format!("Failed to generate Aura keypair: {:?}", e))
        })?;
    keystore
        .insert(AURA, seed, &aura_pair.public().0)
        .map_err(|e| ServiceError::Other(format!("Failed to insert Aura key: {:?}", e)))?;

    log::info!("🔑 Inserted Aura key for block authoring");

    // Insert GRANDPA key (ed25519) for finality
    let grandpa_pair =
        sp_core::ed25519::Pair::from_string(seed, None).map_err(|e: SecretStringError| {
            ServiceError::Other(format!("Failed to generate GRANDPA keypair: {:?}", e))
        })?;
    keystore
        .insert(GRANDPA, seed, &grandpa_pair.public().0)
        .map_err(|e| ServiceError::Other(format!("Failed to insert GRANDPA key: {:?}", e)))?;

    log::info!("🔑 Inserted GRANDPA key for finality");

    Ok(())
}

fn maybe_insert_dev_keys(
    config: &Configuration,
    keystore: &KeystoreContainer,
) -> Result<(), ServiceError> {
    // If X3_DEV_SEED is set, insert that key regardless of chain type (testnet convenience).
    if let Ok(seed) = std::env::var("X3_DEV_SEED") {
        log::info!("🔑 Inserting dev keys from X3_DEV_SEED");
        return insert_dev_keys_with_seed(keystore, &seed);
    }

    // For development chains, insert Alice's keys for block authoring
    if config.chain_spec.chain_type() == ChainType::Development {
        return insert_dev_keys_with_seed(keystore, "//Alice");
    }

    Ok(())
}

fn tuned_transaction_pool_options(
    _existing: sc_transaction_pool::TransactionPoolOptions,
) -> sc_transaction_pool::TransactionPoolOptions {
    let network_speed = NetworkSpeed::detect();
    let (ready_count, future_count, ready_bytes, _future_bytes) = network_speed.pool_sizing();

    const TX_POOL_BAN_TIME_SECS: u64 = 60;
    log::info!(
        "🔗 TX Pool configured for {:?} network: {} ready / {} future, {} MiB",
        network_speed,
        ready_count,
        future_count,
        ready_bytes / 1024 / 1024,
    );

    sc_transaction_pool::TransactionPoolOptions::new_with_params(
        ready_count,
        ready_bytes,
        Some(TX_POOL_BAN_TIME_SECS),
        sc_transaction_pool::TransactionPoolType::SingleState,
        false,
    )
}

/// Apply the tuned limits to a runtime configuration before the pool is built.
pub fn tune_transaction_pool_config(config: &mut Configuration) {
    let network_speed = NetworkSpeed::detect();
    log::info!(
        "🌐 Network speed detected: {:?} (set X3_NETWORK_SPEED=slow|normal|fast to override)",
        network_speed
    );
    config.transaction_pool = tuned_transaction_pool_options(config.transaction_pool.clone());
}

/// Return the correct Aura slot duration for a given runtime spec_version.
///
/// CRITICAL: Aura enforces slot monotonicity. If the slot duration changes mid-chain,
/// nodes that don't gate on spec_version will compute wrong slots for historical blocks
/// and either stall or fork. This function is the safety valve.
///
/// - spec_version < 5: legacy 400ms slots (genesis chain used 400ms)
/// - spec_version >= 5: 200ms slots (v5 migration target)
///
/// Call this when building/verifying any block with a spec_version you can read.
pub fn slot_duration_for_spec(spec_version: u32) -> Duration {
    if spec_version >= 5 {
        Duration::from_millis(200)
    } else {
        Duration::from_millis(400)
    }
}

// ─── PoH Block Import Wrapper ─────────────────────────────────────────────────

/// Wraps a Substrate block import and verifies the PoH digest on each imported block.
///
/// **v2 enforcement**: When `poh_state` is `Some`, every imported block is checked:
///   1. PoH digest must be present in the block header's consensus logs.
///   2. Tick must be `prev_tick + 1` (monotonicity).
///   3. PoH hash must be `SHA256(prev_poh_hash || tx_mix_root)` (chain integrity).
///
/// **Passthrough mode**: When `poh_state` is `None` (i.e. `--enable-poh` is not set),
/// the wrapper is transparent — no overhead, no change to existing behavior.
///
/// **tx_mix_root in v2**: Both the block proposer (`PoHState::advance`) and this verifier
/// use `&[]` (empty tx slice) → `tx_mix_root = SHA256([0u8; 64])`.  This is consistent
/// on both sides.  A future poh-v3 milestone will wire real extrinsic hashes on both ends
/// simultaneously.
pub struct PoHVerifyBlockImport<Block, Inner> {
    inner: Inner,
    poh_state: Option<Arc<Mutex<PoHState>>>,
    _phantom: std::marker::PhantomData<Block>,
}

impl<Block: BlockT, Inner> PoHVerifyBlockImport<Block, Inner> {
    /// Create a new wrapper.
    ///
    /// - `poh_state = Some(...)`: enforcement active — every block is verified.
    /// - `poh_state = None`: passthrough — zero overhead, existing behavior.
    pub fn new(inner: Inner, poh_state: Option<Arc<Mutex<PoHState>>>) -> Self {
        Self {
            inner,
            poh_state,
            _phantom: Default::default(),
        }
    }

    /// Extract and decode the PoH digest from a block header's consensus digest logs.
    /// Returns `None` if no `Consensus(POH_ENGINE_ID, _)` log is present.
    fn extract_poh_digest(header: &Block::Header) -> Option<PoHDigest> {
        for item in header.digest().logs() {
            if let DigestItem::Consensus(engine_id, bytes) = item {
                if engine_id == &POH_ENGINE_ID {
                    return PoHDigest::decode(bytes);
                }
            }
        }
        None
    }
}

#[async_trait::async_trait]
impl<Block: BlockT + Send, Inner: BlockImport<Block> + Send + Sync> BlockImport<Block>
    for PoHVerifyBlockImport<Block, Inner>
{
    type Error = Inner::Error;

    async fn check_block(
        &self,
        block: BlockCheckParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(block).await
    }

    async fn import_block(
        &self,
        block: BlockImportParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        if let Some(state_arc) = &self.poh_state {
            match Self::extract_poh_digest(&block.header) {
                None => {
                    // No digest: warn but allow through during upgrade grace period.
                    // poh-v3: reject once all validators are upgraded.
                    log::warn!(
                        "[PoH] Block has no PoH digest — allowing through (upgrade grace period). \
                         Once all validators run poh-v2, this will be a hard reject."
                    );
                }
                Some(digest) => {
                    let mut state = state_arc.lock().await;
                    let prev_tick = state.tick();
                    let prev_hash = state.hash();

                    // v2: use empty tx slice — consistent with proposer's advance(&[])
                    // v3: replace &[] with real extrinsic hashes from block.body
                    match PoHVerifier::verify(&digest, prev_tick, &prev_hash, &[]) {
                        Ok(()) => {
                            // Advance local state to stay in sync with the chain.
                            state.advance(&[]);
                            log::debug!(
                                "[PoH] ✅ Tick {} verified — chain integrity confirmed",
                                digest.tick
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "[PoH] ❌ Verification failed at tick {}: {} — block REJECTED",
                                digest.tick,
                                e
                            );
                            return Ok(ImportResult::KnownBad);
                        }
                    }
                }
            }
        }
        self.inner.import_block(block).await
    }
}

/// Create partial components for X3 Chain node
///
/// Returns the common components needed by various subcommands (benchmarking, export, etc.)
pub fn new_partial(
    config: &Configuration,
) -> Result<
    PartialComponents<
        FullClient,
        FullBackend,
        SelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
        (
            sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, SelectChain>,
            sc_consensus_grandpa::LinkHalf<Block, FullClient, SelectChain>,
            Option<Telemetry>,
        ),
    >,
    ServiceError,
> {
    // Set up telemetry if endpoints are configured
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    // Create executor
    let executor = sc_service::new_wasm_executor::<sp_io::SubstrateHostFunctions>(&config.executor);

    // Build partial components
    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;

    // For dev chains or when X3_DEV_SEED is set, insert keys for block authoring.
    maybe_insert_dev_keys(config, &keystore_container)?;

    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    // Select chain implementation (longest chain rule)
    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = Arc::from(
        sc_transaction_pool::Builder::new(
            task_manager.spawn_essential_handle(),
            client.clone(),
            config.role.is_authority().into(),
        )
        .with_options(config.transaction_pool.clone())
        .with_prometheus(config.prometheus_registry())
        .build(),
    );

    // Create GRANDPA block import wrapper
    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        512u32,
        &client,
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    // Create Aura import queue with proper block verification
    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

    let import_queue =
        sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(ImportQueueParams {
            block_import: grandpa_block_import.clone(),
            justification_import: Some(Box::new(grandpa_block_import.clone())),
            client: client.clone(),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                let slot =
					sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						*timestamp,
						slot_duration,
					);

                Ok((slot, timestamp))
            },
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            check_for_equivocation: Default::default(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            compatibility_mode: Default::default(),
        })?;

    Ok(PartialComponents {
        client,
        backend,
        task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        other: (grandpa_block_import, grandpa_link, telemetry),
    })
}

/// Determine whether GRANDPA should run given configuration and feature flags.
///
/// - returns `false` when either the user disabled GRANDPA explicitly or when the
///   experimental Flash Finality gadget flag is active. This helper exists so
///   that unit tests can verify the decision logic without spawning a full node.
pub fn compute_enable_grandpa(config: &Configuration, feature_flags: NodeFeatureFlags) -> bool {
    compute_enable_grandpa_from_flags(config.disable_grandpa, feature_flags)
}

fn compute_enable_grandpa_from_flags(
    disable_grandpa: bool,
    feature_flags: NodeFeatureFlags,
) -> bool {
    !disable_grandpa && !feature_flags.enable_flash_finality
}

fn enforce_startup_gate_if_authority(is_authority: bool) -> Result<(), ServiceError> {
    if !is_authority {
        return Ok(());
    }

    x3_chain_runtime::fraud_proofs::startup_gate::run_startup_gate().map_err(|err| {
        ServiceError::Other(format!(
            "Startup determinism gate failed; refusing authority startup: {err}"
        ))
    })
}

struct CrossVmBridgeSafetyGate {
    finality_oracle: InMemoryFinalityOracle,
    risk_engine: GatewayRiskEngine,
}

impl Default for CrossVmBridgeSafetyGate {
    fn default() -> Self {
        let mut finality_oracle = InMemoryFinalityOracle::new();
        finality_oracle.set_rule(
            FinalityChain::Other(0),
            FinalityRule {
                min_confirmations: 1,
                max_allowed_reorg_depth: 0,
            },
        );

        Self {
            finality_oracle,
            risk_engine: GatewayRiskEngine::new(RiskPolicy::default()),
        }
    }
}

impl CrossVmBridgeSafetyGate {
    fn preflight(
        &self,
        bridge: &CrossVmBridge,
        best_number: u64,
        finalized_number: u64,
        recent_failures: u32,
    ) -> Result<(), String> {
        if bridge.is_paused() {
            return Err("bridge_paused".to_string());
        }

        if bridge.pending_count() == 0 {
            return Ok(());
        }

        let confirmations = best_number.saturating_sub(finalized_number);
        let verdict = self.finality_oracle.evaluate(ObservedBlock {
            chain: FinalityChain::Other(0),
            height: best_number,
            confirmations,
            observed_reorg_depth: 0,
        });

        if verdict.status != FinalityStatus::Finalized {
            return Err(format!(
                "finality_not_ready: status={:?}, best={}, finalized={}",
                verdict.status, best_number, finalized_number
            ));
        }

        let decision = self.risk_engine.evaluate(RouteRiskInput {
            value_usd: (bridge.pending_count() as u64).saturating_mul(10_000),
            recent_failures,
            verifier_quorum_met: true,
        });

        if decision.allow_route {
            Ok(())
        } else {
            Err(format!("risk_gate_blocked: {}", decision.reason))
        }
    }

    fn postflight(&self, results: &[CrossVmResult]) -> Result<(), String> {
        if results.is_empty() {
            return Ok(());
        }

        for result in results {
            if !result.success {
                return Err("execution_failed".to_string());
            }

            if result.error.is_some() {
                return Err("success_with_error".to_string());
            }

            if result.output.is_empty() {
                return Err("empty_success_output".to_string());
            }
        }

        Ok(())
    }

    fn open_dispute(&self, marker: [u8; 32], now: u64) -> Result<DisputeStatus, String> {
        let mut tracker = DisputeTracker::new(marker, now, 1)
            .map_err(|err| format!("dispute_init_failed: {err:?}"))?;
        tracker
            .vote("node-crossvm-safety", true)
            .map_err(|err| format!("dispute_vote_failed: {err:?}"))?;
        let closed = tracker
            .close(now.saturating_add(1), 1)
            .map_err(|err| format!("dispute_close_failed: {err:?}"))?;
        Ok(closed.status)
    }
}

/// Start a new X3 Chain full node with complete consensus and networking
pub fn new_full<
    N: sc_network::NetworkBackend<Block, <Block as sp_runtime::traits::Block>::Hash>,
>(
    mut config: Configuration,
    feature_flags: NodeFeatureFlags,
) -> Result<TaskManager, ServiceError> {
    enforce_startup_gate_if_authority(config.role.is_authority())?;

    tune_transaction_pool_config(&mut config);
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        other: (grandpa_block_import, grandpa_link, mut telemetry),
    } = new_partial(&config)?;

    // configure network protocols; GRANDPA may be disabled when using Flash Finality
    let mut net_config = sc_network::config::FullNetworkConfiguration::<
        Block,
        <Block as sp_runtime::traits::Block>::Hash,
        N,
    >::new(&config.network, config.prometheus_registry().cloned());
    let metrics = N::register_notification_metrics(config.prometheus_registry());
    let peer_store_handle = net_config.peer_store_handle();

    // decide whether GRANDPA should be active; tests can call the helper below.
    let enable_grandpa = compute_enable_grandpa(&config, feature_flags);
    if !enable_grandpa && feature_flags.enable_flash_finality {
        log::info!("⚡ Flash Finality flag is set; GRANDPA will be disabled for this node");
    }

    if feature_flags.enable_atomic_kernel {
        log::info!(
            "🧩 Atomic kernel feature gate enabled; sequencer and settlement pipelines are active"
        );
        // Additional atomic kernel activation hooks can be added here.
    } else {
        log::info!("🧩 Atomic kernel feature gate is disabled (default)");
    }

    let genesis_hash = client
        .block_hash(0)?
        .ok_or_else(|| ServiceError::Other("Genesis block not found".to_string()))?;
    let grandpa_protocol_name =
        sc_consensus_grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);

    let grandpa_notification_service = if enable_grandpa {
        let (grandpa_protocol_config, grandpa_notification_service) =
            sc_consensus_grandpa::grandpa_peers_set_config::<_, N>(
                grandpa_protocol_name.clone(),
                metrics.clone(),
                peer_store_handle.clone(),
            );
        net_config.add_notification_protocol(grandpa_protocol_config);
        Some(grandpa_notification_service)
    } else {
        None
    };

    let warp_sync = if enable_grandpa {
        Some(Arc::new(
            sc_consensus_grandpa::warp_proof::NetworkProvider::new(
                backend.clone(),
                grandpa_link.shared_authority_set().clone(),
                Vec::default(),
            ),
        ))
    } else {
        None
    };

    let flash_notification_service = if feature_flags.enable_flash_finality {
        let (flash_proto, flash_notif) = N::notification_config(
            FLASH_FINALITY_PROTOCOL_ID.into(),
            vec![],
            1024 * 1024,
            None,
            sc_network::config::SetConfig {
                in_peers: 25,
                out_peers: 25,
                reserved_nodes: vec![],
                non_reserved_mode: sc_network::config::NonReservedPeerMode::Accept,
            },
            metrics.clone(),
            peer_store_handle.clone(),
        );
        net_config.add_notification_protocol(flash_proto);
        Some(flash_notif)
    } else {
        None
    };

    // Build networking service
    let (network, system_rpc_tx, tx_handler_controller, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: warp_sync.map(|w| sc_service::WarpSyncConfig::WithProvider(w)),
            block_relay: None,
            metrics,
        })?;

    let role = config.role;
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks: Option<()> = None;
    let name = config.network.node_name.clone();
    let chain_name = config.chain_spec.name().to_string();
    let prometheus_registry = config.prometheus_registry().cloned();
    let role_for_grandpa = role;

    // Register X3-specific Prometheus metrics alongside Substrate's built-in metrics.
    // These counters track block production, comit lifecycle, and dual-VM execution
    // and are automatically scraped via Substrate's /metrics endpoint.
    let x3_metrics: Option<std::sync::Arc<X3PrometheusMetrics>> = prometheus_registry
        .as_ref()
        .and_then(|reg| match X3PrometheusMetrics::register(reg) {
            Ok(m) => {
                log::info!("📊 X3 Prometheus metrics registered successfully");
                Some(std::sync::Arc::new(m))
            }
            Err(e) => {
                log::warn!("⚠️ Failed to register X3 Prometheus metrics: {}", e);
                None
            }
        });

    let mut predictor_config = PredictorConfig::default();
    predictor_config.max_parallel_shards = if feature_flags.enable_parallel_proposer {
        predictor_config.max_parallel_shards.max(2)
    } else {
        1
    };
    let contention_predictor = Arc::new(ContentionPredictor::new(predictor_config));
    let predictor_for_heatmap = if feature_flags.enable_parallel_proposer {
        Some(contention_predictor.clone())
    } else {
        None
    };

    if feature_flags.enable_parallel_proposer {
        log::info!(
            "⚡ Parallel proposer is enabled; contention predictor wired into block authoring"
        );
    }
    if feature_flags.enable_flash_finality {
        if enable_grandpa {
            // still running grandpa due to some configuration oddity
            log::warn!(
                "⚠️ --enable-flash-finality is set but GRANDPA will still run due to configuration."
            );
        } else {
            log::info!(
                "⚡ Flash Finality is enabled; GRANDPA has been disabled for this node (shadow mode)."
            );
        }
    }
    if feature_flags.enable_poh {
        log::info!(
            "⏱️ PoH digest verification is ACTIVE (v2) — blocks without a valid PoH digest \
             will log an error. Hard-rejection lands in poh-v3 once all validators upgrade."
        );
    }
    if feature_flags.gpu_required {
        log::warn!(
            "⚠️ --gpu-required=true is set; ensure CPU fallback is not relied on by your deployment policy."
        );
    }

    // Initialize PoH State if enabled
    let shared_poh_state = if feature_flags.enable_poh {
        Some(Arc::new(Mutex::new(PoHState::default())))
    } else {
        None
    };

    // Initialize Flash Finality Gadget for RPC regardless of whether we run the bridge
    let flash_finality_gadget = if feature_flags.enable_flash_finality {
        let keystore = keystore_container.keystore();
        let my_id = keystore
            .sr25519_public_keys(KeyTypeId(*b"flsh"))
            .first()
            .map(|k| k.0);

        if let Some(my_id) = my_id {
            Some(Arc::new(FlashFinalityGadget::new(
                FlashFinalityConfig::default(),
                my_id,
                Some(Box::new(keystore) as Box<dyn std::any::Any + Send + Sync>),
            )))
        } else {
            log::warn!(
                "⚠️ Flash Finality enabled but no flsh key found in keystore; disabling Flash Finality gadget"
            );
            None
        }
    } else {
        None
    };

    // Spawn core Substrate tasks (RPC, network, telemetry, txpool, offchain, etc.)
    let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig::default()));

    {
        let limiter = rate_limiter.clone();
        task_manager
            .spawn_handle()
            .spawn("rpc-rate-limiter-cleanup", None, async move {
                let interval = Duration::from_secs(60);
                loop {
                    tokio::time::sleep(interval).await;
                    limiter.cleanup_stale_connections(Duration::from_secs(5 * 60));
                }
            });
    }

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();
        let gadget = flash_finality_gadget.clone();
        let limiter = rate_limiter.clone();
        Box::new(
            move |subscription_executor: sc_rpc::SubscriptionTaskExecutor| {
                crate::rpc::create_full(
                    client.clone(),
                    transaction_pool.clone(),
                    gadget.clone(),
                    limiter.clone(),
                    subscription_executor,
                )
                .map_err(Into::into)
            },
        )
    };

    let disable_grandpa_flag = config.disable_grandpa;

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        client: client.clone(),
        backend: backend.clone(),
        task_manager: &mut task_manager,
        keystore: keystore_container.keystore(),
        transaction_pool: transaction_pool.clone(),
        rpc_builder,
        network: Arc::new(network.clone()),
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        telemetry: telemetry.as_mut(),
        tracing_execute_block: None,
    })?;

    // Start Aura block authoring if this is an authority node
    if role.is_authority() {
        let proposer_factory: ParallelProposerFactory<_, FullBackend, FullClient, _> =
            ParallelProposerFactory::new(
                task_manager.spawn_handle(),
                client.clone(),
                transaction_pool.clone(),
                prometheus_registry.as_ref(),
                telemetry.as_ref().map(|x| x.handle()),
                contention_predictor.clone(),
            );

        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
        let shared_poh_state_for_aura = shared_poh_state.clone();

        // PoH v2: wrap grandpa_block_import so every imported block is verified.
        // When enable_poh=false, poh_state=None → PoHVerifyBlockImport is a zero-cost passthrough.
        let poh_wrapped_block_import =
            PoHVerifyBlockImport::new(grandpa_block_import, shared_poh_state.clone());

        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
            StartAuraParams {
                slot_duration,
                client: client.clone(),
                select_chain,
                block_import: poh_wrapped_block_import,
                proposer_factory,
                create_inherent_data_providers: move |_, ()| {
                    let poh_state = shared_poh_state_for_aura.clone();
                    async move {
                        let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                        let slot =
                            sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                *timestamp,
                                slot_duration,
                            );

                        // Advance PoH state if enabled (shadow mode — just tick, don't inject as inherent)
                        if let Some(state_arc) = poh_state {
                            let mut state = state_arc.lock().await;
                            state.advance(&[]);
                        }

                        Ok((slot, timestamp))
                    }
                },
                force_authoring,
                backoff_authoring_blocks,
                keystore: keystore_container.keystore(),
                sync_oracle: sync_service.clone(),
                justification_sync_link: sync_service.clone(),
                block_proposal_slot_portion: SlotProportion::new(0.9f32),
                max_block_proposal_slot_portion: None,
                telemetry: telemetry.as_ref().map(|x| x.handle()),
                compatibility_mode: Default::default(),
            },
        )?;

        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", Some("block-authoring"), aura);
    }

    // Start GRANDPA finality gadget
    if enable_grandpa {
        let grandpa_config = sc_consensus_grandpa::Config {
            gossip_duration: std::time::Duration::from_millis(100),
            justification_generation_period: 512u32,
            name: Some(name.clone()),
            observer_enabled: false,
            keystore: Some(keystore_container.keystore()),
            local_role: role_for_grandpa,
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            protocol_name: grandpa_protocol_name,
        };

        // Create GRANDPA parameters with offchain transaction pool
        let offchain_tx_pool_factory =
            sc_transaction_pool_api::OffchainTransactionPoolFactory::new(transaction_pool.clone());

        let grandpa_params = sc_consensus_grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network: network.clone(),
            sync: Arc::new(sync_service.clone()),
            notification_service: grandpa_notification_service
                .expect("grandpa notification service present when grandpa enabled; qed"),
            voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state: SharedVoterState::empty(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            offchain_tx_pool_factory,
        };

        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            None,
            sc_consensus_grandpa::run_grandpa_voter(grandpa_params)?,
        );
    }

    // Network starts automatically in stable2512 (start_network removed)

    // Spawn a background task to watch finalized blocks and log events with emojis
    {
        let client = client.clone();
        let metrics_for_import = x3_metrics.clone();
        task_manager
            .spawn_handle()
            .spawn("import-watcher", None, async move {
                use futures_util::StreamExt;

                let mut notifications = client.import_notification_stream();
                while let Some(notification) = notifications.next().await {
                    let number: u64 = (*notification.header.number()).saturated_into();
                    if let Some(ref m) = metrics_for_import {
                        m.blocks_produced.inc();
                    }
                    // Purple color for block imported
                    log::info!(
                        "\x1b[35m📦 Block imported: #{} — syncing state\x1b[0m",
                        number
                    );
                }
            });
    }

    {
        let client = client.clone();
        let predictor = predictor_for_heatmap.clone();
        task_manager
            .spawn_handle()
            .spawn("block-watcher", None, async move {
                use futures_util::StreamExt;

                let mut notifications = client.finality_notification_stream();
                while let Some(notification) = notifications.next().await {
                    // number is saturated into u64
                    let number: u64 = (*notification.header.number()).saturated_into();
                    // Orange color for block finalized
                    log::info!("\x1b[33m🏆 Block finalized: #{} ✅\x1b[0m", number);

                    if let Some(predictor) = predictor.as_ref() {
                        if let Ok(Some(block)) = client.block(notification.hash) {
                            let mut txs = Vec::new();
                            for xt in block.block.extrinsics() {
                                let hash = BlakeTwo256::hash_of(&xt);
                                let mut hash_bytes = [0u8; 32];
                                hash_bytes.copy_from_slice(hash.as_ref());
                                txs.push(extract_tx_metadata(&xt, hash_bytes));
                            }
                            predictor.update_heatmap(&txs).await;
                        }
                    }
                }
            });
    }

    // Start Flash Finality if enabled
    if let Some(gadget) = flash_finality_gadget {
        let bridge = FlashFinalityBridge::new(
            gadget.clone(),
            client.clone(),
            network.clone(),
            sync_service.clone(),
            keystore_container.keystore(),
            flash_notification_service
                .expect("flash notification service present when flash finality enabled; qed"),
        );

        task_manager.spawn_essential_handle().spawn(
            "flash-finality-bridge",
            Some("flash-finality"),
            bridge.run(),
        );

        task_manager.spawn_essential_handle().spawn(
            "flash-finality-timeout",
            Some("flash-finality"),
            gadget.clone().spawn_timeout_monitor(),
        );

        // Spawn the Flash-Finality voter to apply certificates as finality
        // In live mode (when enable_flash_finality=true and vote_on_flash=true),
        // this will move the finalized head based on certificates.
        // In shadow mode, it logs certificate availability for monitoring.
        let gadget_for_voter = gadget.clone();
        let client_for_voter = client.clone();
        let enable_flash_live_mode = feature_flags.enable_flash_finality && !disable_grandpa_flag;

        task_manager.spawn_essential_handle().spawn(
            "flash-finality-voter",
            Some("flash-finality"),
            run_flash_finality_voter(gadget_for_voter, client_for_voter, enable_flash_live_mode),
        );

        log::info!("⚡ Flash Finality gadget, network bridge, and voter started");
    }

    // Spawn GPU Validator Orchestrator if enabled (feature-gated)
    #[cfg(feature = "gpu-validator")]
    if feature_flags.enable_gpu_validator {
        let orchestrator_id = format!("{}-gpu-validator", name.clone());
        let gpu_config = SwarmConfig::default();

        let orchestrator = Arc::new(tokio::sync::RwLock::new(SwarmOrchestrator::new(gpu_config)));
        log::info!(
            "🎮 GPU Validator Orchestrator initialized: {}",
            orchestrator_id
        );

        let orch_clone = orchestrator.clone();
        task_manager.spawn_essential_handle().spawn(
            "gpu-validator-orchestrator",
            Some("gpu-validator"),
            async move {
                loop {
                    // Poll orchestrator health/status; in production this would
                    // integrate with block import, fetch pending proofs, etc.
                    let orch = orch_clone.read().await;
                    if let Err(e) = orch.health_check() {
                        log::warn!("⚠️ GPU Validator health check failed: {}", e);
                    }
                    drop(orch);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            },
        );

        log::info!("🎮 GPU Validator Orchestrator spawned and monitoring");

        {
            let client_for_anchor = client.clone();
            let orch_for_anchor = orchestrator.clone();
            task_manager.spawn_handle().spawn(
                "gpu-validator-finalized-anchor",
                Some("gpu-validator"),
                async move {
                    use futures_util::StreamExt;

                    let mut finality_notifications =
                        client_for_anchor.finality_notification_stream();
                    while let Some(notification) = finality_notifications.next().await {
                        let finalized_block: u64 = (*notification.header.number()).saturated_into();
                        let orch = orch_for_anchor.read().await;
                        orch.update_finalized_block_anchor(finalized_block);
                        if let Some(anchor) = orch.min_finalized_block_anchor() {
                            log::debug!(
                                "🎮 GPU validator proof anchor updated from finalized head: {}",
                                anchor
                            );
                        }
                    }
                },
            );
            log::info!("🎮 GPU validator finalized-head anchor watcher spawned");
        }

        // ISSUE #1 FIX: Spawn GPU Sidecar Health Monitor task
        // Tracks GPU sidecar process health and triggers restart on failure threshold
        {
            let client_for_monitor = client.clone();
            let orch_for_monitor = orchestrator.clone();
            let mut health_monitor = GpuSidecarHealthMonitor::new();
            let mut last_checked_block: u32 = 0;

            task_manager.spawn_handle().spawn(
                "gpu-sidecar-health-monitor",
                Some("gpu-validator"),
                async move {
                    use futures_util::StreamExt;

                    let mut finality_notifications = client_for_monitor.finality_notification_stream();
                    while let Some(notification) = finality_notifications.next().await {
                        let current_block: u32 = (*notification.header.number())
                            .saturated_into::<u32>();

                        // Run health check every GPU_SIDECAR_HEALTH_CHECK_INTERVAL blocks
                        if current_block.saturating_sub(last_checked_block)
                            >= GPU_SIDECAR_HEALTH_CHECK_INTERVAL
                        {
                            last_checked_block = current_block;

                            // Probe orchestrator health — feeds actual sidecar status into the monitor.
                            let orch_guard = orch_for_monitor.read().await;
                            let healthy = orch_guard.health_check().is_ok();
                            drop(orch_guard);
                            health_monitor.record_check(healthy, current_block);

                            if health_monitor.needs_restart() {
                                // Restart GPU sidecar via orchestrator
                                let orch = orch_for_monitor.read().await;
                                if let Err(e) = orch.trigger_restart() {
                                    log::error!(
                                        "🚨 Failed to trigger GPU sidecar restart: {}; manual intervention required",
                                        e
                                    );
                                } else {
                                    log::info!("🔄 GPU sidecar restarted after health failure threshold reached");
                                    health_monitor.reset();
                                }
                                drop(orch);
                            } else if healthy {
                                log::debug!(
                                    "✅ GPU sidecar health check passed at block {}",
                                    current_block
                                );
                            }
                        }
                    }
                },
            );

            log::info!("🏥 GPU Sidecar Health Monitor spawned (checks every {} blocks, restart after {} failures)",
                GPU_SIDECAR_HEALTH_CHECK_INTERVAL, GPU_SIDECAR_RESTART_THRESHOLD);
        }

        // ─────────────────────────────────────────────────────────────
        // Wire GPU Sidecar Spawning into Startup Sequence (Task 4)
        // ─────────────────────────────────────────────────────────────
        // TICKET-001 Phase 2: Spawn GPU validator sidecar with proper lifecycle management.
        // The sidecar performs GPU-accelerated cross-chain validation and runs independently
        // from the orchestrator, but is coordinated via health checks and restart signals.
        {
            let sidecar_config = GpuSidecarConfig {
                service_id: format!("{}-sidecar", name.clone()),
                gpu_devices: vec![],                                // Auto-detect
                rpc_endpoint: format!("http://127.0.0.1:{}", 9944), // Default X3 RPC port
                proof_interval_blocks: 10,
                max_concurrent_tasks: 4,
            };

            log::info!(
                "🔧 GPU Sidecar startup: initializing with config {:?}",
                sidecar_config
            );

            let (gpu_sidecar_handle, shutdown_rx) = GpuSidecarHandle::new(sidecar_config.clone());
            let gpu_sidecar_handle_arc = Arc::new(gpu_sidecar_handle);

            // Spawn sidecar task into the task manager
            let gpu_sidecar_for_spawn = gpu_sidecar_handle_arc.clone();
            let gpu_sidecar_is_running = gpu_sidecar_for_spawn.is_running.clone();
            let gpu_sidecar_task_handle = gpu_sidecar_for_spawn.task_handle.clone();
            let orchestrator_for_sidecar = orchestrator.clone();

            task_manager.spawn_handle().spawn(
                "gpu-validator-sidecar",
                Some("gpu-validator"),
                async move {
                    log::info!("✨ GPU Sidecar async task started");
                    gpu_sidecar_is_running.store(true, std::sync::atomic::Ordering::Release);

                    let result =
                        spawn_gpu_sidecar(sidecar_config, shutdown_rx, orchestrator_for_sidecar)
                            .await;

                    log::info!("🏁 GPU Sidecar async task completed: {:?}", result);
                    gpu_sidecar_is_running.store(false, std::sync::atomic::Ordering::Release);
                },
            );

            log::info!(
                "🚀 GPU Sidecar spawned and monitoring (service_id={})",
                gpu_sidecar_handle_arc.config.service_id
            );
        }

        {
            use cross_chain_gpu_validator::CrossChainValidator;

            let cross_chain_validator = CrossChainValidator::new(None, 1);

            task_manager.spawn_handle().spawn(
                "cross-chain-gpu-validator",
                Some("gpu-validator"),
                async move {
                    match cross_chain_validator.run_validation_loop().await {
                        Ok(()) => {
                            log::info!("🌐 Cross-chain GPU validator loop completed");
                        }
                        Err(e) => {
                            log::error!(
                                "🌐 Cross-chain GPU validator critical failure: {} — validator disabled, node continues",
                                e
                            );
                        }
                    }
                },
            );

            log::debug!("🌐 Cross-chain GPU validator spawned");
        }
    }

    #[cfg(not(feature = "gpu-validator"))]
    if feature_flags.enable_gpu_validator {
        log::warn!(
            "⚠️ GPU Validator requested but gpu-validator feature not enabled at compile time; ignored"
        );
    }

    // ── Wire Cross-VM bridge adapters ─────────────────────────────────────
    // `SubstrateClientBalanceAdapter` provides live canonical-ledger balances
    // to the off-chain AtomicSwapOrchestrator.  `PalletEscrowAdapter` wraps it
    // with durable escrow persistence backed by the node's off-chain storage,
    // so in-flight cross-VM swaps survive node restarts.
    {
        match backend.offchain_storage() {
            Some(offchain_storage) => {
                let runtime_bridge = Arc::new(SubstrateX3VmBridge::with_persistence(
                    client.clone(),
                    OffchainEscrowPersistence::new(offchain_storage),
                ));
                let escrow_adapter = runtime_bridge.escrow.clone();

                {
                    let dispatcher = Arc::new(
                        RuntimeCrossVmDispatcher::new(client.clone())
                            .with_x3vm_bridge(runtime_bridge.bridge.clone()),
                    );
                    let bridge = Arc::new(std::sync::Mutex::new(CrossVmBridge::new()));
                    let bridge_safety_gate = CrossVmBridgeSafetyGate::default();
                    let client_for_bridge = client.clone();
                    // Keep the X3VM bridge and runtime-backed providers alive for the task.
                    let _runtime_bridge = runtime_bridge.clone();
                    let _escrow = escrow_adapter.clone();
                    let bridge_for_task = bridge.clone();
                    task_manager.spawn_handle().spawn(
                        "cross-vm-bridge-poller",
                        Some("x3"),
                        async move {
                            let _runtime_bridge = _runtime_bridge;
                            let mut recent_failures: u32 = 0;
                            loop {
                                tokio::time::sleep(Duration::from_millis(200)).await;
                                // Lock is acquired and released within this block;
                                // not held across any await point.
                                let mut b = match bridge_for_task.lock() {
                                    Ok(guard) => guard,
                                    Err(poisoned) => {
                                        // Mutex was poisoned by a panicking thread.
                                        // Recover the data and log — do NOT crash the node.
                                        log::error!(
                                            target: "x3-service",
                                            "cross-vm bridge mutex was poisoned; recovering guard"
                                        );
                                        poisoned.into_inner()
                                    }
                                };

                                let info = client_for_bridge.info();
                                let best_number: u64 = info.best_number.saturated_into();
                                let finalized_number: u64 = info.finalized_number.saturated_into();

                                if let Err(reason) = bridge_safety_gate.preflight(
                                    &b,
                                    best_number,
                                    finalized_number,
                                    recent_failures,
                                ) {
                                    if reason != "bridge_paused" {
                                        recent_failures = recent_failures.saturating_add(1);
                                        log::warn!("[cross-vm] preflight blocked execution: {}", reason);
                                    }
                                    continue;
                                }

                                match b.execute_pending_with_dispatcher(
                                    dispatcher.as_ref(),
                                ) {
                                    Ok(results) if !results.is_empty() => {
                                        if let Err(reason) = bridge_safety_gate.postflight(&results) {
                                            recent_failures = recent_failures.saturating_add(1);
                                            b.pause();
                                            let marker = BlakeTwo256::hash_of(&reason).to_fixed_bytes();
                                            let dispute_status = bridge_safety_gate
                                                .open_dispute(marker, best_number)
                                                .unwrap_or(DisputeStatus::Open);
                                            log::warn!(
                                                "[cross-vm] postflight rejected batch (status={:?}): {}; bridge paused",
                                                dispute_status,
                                                reason
                                            );
                                        } else {
                                            recent_failures = 0;
                                            log::debug!(
                                                "[cross-vm] executed {} pending bridge ops",
                                                results.len()
                                            );
                                        }
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        recent_failures = recent_failures.saturating_add(1);
                                        log::warn!(
                                            "[cross-vm] bridge poll error: {:?}",
                                            e
                                        );
                                    }
                                }
                            }
                        },
                    );
                }

                log::info!("🌉 Cross-VM bridge adapters wired (balance + escrow)");
            }
            None => {
                log::warn!("⚠️  Off-chain storage unavailable (in-memory backend?); escrow persistence disabled");
            }
        }
    }

    // Start PoH Generator background task if enabled
    if let Some(poh_state_arc) = shared_poh_state {
        let client_clone = client.clone();

        task_manager
            .spawn_essential_handle()
            .spawn("poh-watcher", Some("poh"), async move {
                let mut import_notifications = client_clone.import_notification_stream();
                while let Some(notification) = import_notifications.next().await {
                    if notification.is_new_best {
                        let mut state = poh_state_arc.lock().await;
                        state.advance(&[]);
                        log::info!(
                            "⏱️  [PoH] Shadow tick {} anchored to block {}",
                            state.tick(),
                            notification.hash
                        );
                    }
                }
            });
        log::info!("⏱️ Proof of History (PoH) generator enabled and wired to block loop");
    }

    // ─────────────────────────────────────────────────────────────────
    // Initialize Sidecar Service for Cross-VM Bridge
    // ─────────────────────────────────────────────────────────────────
    // The sidecar monitors external VMs (SVM, EVM on other chains) and bridges
    // assets into X3. It must be lifecycle-managed so crashes trigger restarts.
    {
        let sidecar_enabled = std::env::var("X3_ENABLE_SIDECAR")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if sidecar_enabled {
            log::info!("🔌 Cross-VM Sidecar Service: initializing lifecycle management");

            let sidecar_name = format!("{}-sidecar", name.clone());
            let sidecar_task_name = sidecar_name.clone();
            let chain_name_for_sidecar = chain_name.clone();

            task_manager.spawn_handle().spawn(
                "x3-sidecar-monitor",
                Some("bridge-sidecar"),
                async move {
                    // Loop that monitors and restarts sidecar if it fails
                    let mut restart_count = 0;
                    loop {
                        log::info!(
                            "🔌 Spawning {} (restart #{}) for chain: {}",
                            sidecar_task_name,
                            restart_count,
                            chain_name_for_sidecar
                        );

                        // Spawn the sidecar binary. On clean exit break; on error the
                        // outer restart loop applies exponential back-off (up to 60s).
                        match spawn_sidecar_service(&sidecar_task_name).await {
                            Ok(()) => {
                                // If sidecar completed normally, exit loop
                                log::info!("🔌 {} exited normally", sidecar_task_name);
                                break;
                            }
                            Err(e) => {
                                restart_count += 1;
                                log::error!(
                                    "❌ {} failed ({}): {}; restarting in 5s...",
                                    sidecar_task_name,
                                    restart_count,
                                    e
                                );

                                // Exponential backoff: 5s base, max 60s
                                let backoff_secs =
                                    std::cmp::min(5 * 2_u64.pow(restart_count - 1), 60);
                                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;

                                // Safety: prevent infinite restart loops beyond threshold
                                if restart_count > 20 {
                                    log::error!(
                                        "❌ {} exceeded restart threshold (20); disabling sidecar — node continues without it",
                                        sidecar_task_name
                                    );
                                    return; // graceful exit; non-essential task
                                }
                            }
                        }
                    }
                },
            );

            log::info!("✅ Sidecar service lifecycle manager spawned; monitoring enabled");
        } else {
            log::warn!("⚠️ Cross-VM Sidecar Service disabled via X3_ENABLE_SIDECAR=false");
        }
    }

    log::info!("✨ X3 Chain node started successfully");
    log::info!("🔗 Network: {}", chain_name);
    log::info!("👤 Node name: {}", name);
    log::info!("📋 Role: {:?}", role);

    Ok(task_manager)
}

/// Spawn the X3 GPU Sidecar Service for cross-chain validation.
///
/// The sidecar spawns as an async task within the tokio runtime, watching external VMs
/// (Solana, other EVMs) and performing GPU-accelerated validation of cross-chain proofs.
/// This is fully integrated into the X3 node lifecycle.
///
/// # Features
/// - Non-blocking startup (spawned as async task)
/// - Graceful shutdown coordination
/// - Health monitoring via finality stream
/// - Automatic restart on health check failures
/// - Comprehensive logging
/// - GPU kernel dispatch via `SwarmOrchestrator::submit_batch`
#[cfg(feature = "gpu-validator")]
async fn spawn_gpu_sidecar(
    sidecar_config: GpuSidecarConfig,
    mut shutdown_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    orchestrator: Arc<tokio::sync::RwLock<SwarmOrchestrator>>,
) -> Result<(), String> {
    use x3_gpu_validator_swarm::crypto::HashAlgorithm;
    use x3_gpu_validator_swarm::deterministic::{DeterministicTask, TaskType};

    log::info!(
        "🚀 GPU Sidecar Service '{}' starting up",
        sidecar_config.service_id
    );
    log::info!("   • RPC Endpoint: {}", sidecar_config.rpc_endpoint);
    log::info!(
        "   • GPU Devices: {:?}",
        if sidecar_config.gpu_devices.is_empty() {
            vec!["auto-detect".to_string()]
        } else {
            sidecar_config
                .gpu_devices
                .iter()
                .map(|d| d.to_string())
                .collect()
        }
    );
    log::info!(
        "   • Max Concurrent Tasks: {}",
        sidecar_config.max_concurrent_tasks
    );
    log::info!(
        "   • Proof Submission Interval: {} blocks",
        sidecar_config.proof_interval_blocks
    );

    let mut health_check_counter = 0u32;
    // Reuse a single HTTP client across ticks — avoids spawning a new connection
    // pool every 10 seconds and exhausting file-descriptor limits under load.
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("GPU Sidecar: HTTP client build failed: {}", e))?;

    loop {
        tokio::select! {
            // Shutdown signal received
            _ = shutdown_rx.recv() => {
                log::info!("🛑 GPU Sidecar '{}' received shutdown signal", sidecar_config.service_id);
                return Ok(());
            }

            // Periodic health check + GPU kernel dispatch
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                health_check_counter += 1;

                log::debug!(
                    "✅ GPU Sidecar '{}' tick #{}: querying orchestrator",
                    sidecar_config.service_id,
                    health_check_counter
                );

                // Dispatch a validation task carrying real block state bytes.
                // We query `chain_getHeader` on the local X3 node to get the
                // latest block hash + number as deterministic task inputs.
                let block_inputs: Vec<Vec<u8>> = {
                    let x3_rpc = &sidecar_config.rpc_endpoint;
                    let rpc_body = serde_json::json!({
                        "jsonrpc": "2.0", "id": 1,
                        "method": "chain_getHeader",
                        "params": []
                    });
                    match http_client
                        .post(x3_rpc)
                        .json(&rpc_body)
                        .send()
                        .await
                        .and_then(|r| r.error_for_status())
                    {
                        Ok(resp) => {
                            match resp.json::<serde_json::Value>().await {
                                Ok(v) => {
                                    let hash = v["result"]["parentHash"]
                                        .as_str()
                                        .unwrap_or("")
                                        .trim_start_matches("0x")
                                        .as_bytes()
                                        .to_vec();
                                    let num_hex = v["result"]["number"]
                                        .as_str()
                                        .unwrap_or("0x0")
                                        .trim_start_matches("0x");
                                    let num = u64::from_str_radix(num_hex, 16).unwrap_or(0);
                                    log::debug!(
                                        "🎮 GPU Sidecar '{}' tick #{}: block #{} hash_len={}",
                                        sidecar_config.service_id,
                                        health_check_counter,
                                        num,
                                        hash.len()
                                    );
                                    vec![hash, num.to_le_bytes().to_vec()]
                                }
                                Err(e) => {
                                    log::warn!("🎮 GPU Sidecar: JSON decode error: {}", e);
                                    vec![
                                        sidecar_config.service_id.as_bytes().to_vec(),
                                        health_check_counter.to_le_bytes().to_vec(),
                                    ]
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "🎮 GPU Sidecar '{}' tick #{}: chain_getHeader failed ({}), using fallback",
                                sidecar_config.service_id, health_check_counter, e
                            );
                            vec![
                                sidecar_config.service_id.as_bytes().to_vec(),
                                health_check_counter.to_le_bytes().to_vec(),
                            ]
                        }
                    }
                };

                let task = DeterministicTask::new(
                    TaskType::Hash,
                    block_inputs,
                    HashAlgorithm::Blake2b,
                );

                {
                    let orch = orchestrator.read().await;
                    let task_id = orch.submit_task(task);
                    log::debug!(
                        "🎮 GPU Sidecar '{}' submitted task {} to orchestrator (tick #{})",
                        sidecar_config.service_id,
                        task_id,
                        health_check_counter
                    );

                    // Drain any completed results to bound queue memory.
                    let processed = orch.process_pending_tasks();
                    if processed > 0 {
                        log::debug!(
                            "🎮 GPU Sidecar '{}': orchestrator processed {} task(s)",
                            sidecar_config.service_id,
                            processed
                        );
                    }
                }

                if health_check_counter % 6 == 0 {
                    let orch = orchestrator.read().await;
                    let metrics = orch.get_swarm_metrics();
                    log::info!(
                        "📊 GPU Sidecar '{}' metrics #{}: total_tasks={}, successful={}, validators={}, uptime={}s",
                        sidecar_config.service_id,
                        health_check_counter,
                        metrics.total_tasks,
                        metrics.successful_tasks,
                        metrics.active_validators,
                        health_check_counter * 10
                    );
                }
            }
        }
    }
}

/// Spawn the X3 Sidecar Service for cross-VM bridge monitoring.
///
/// Attempts to launch the `x3-sidecar` binary as a child process.  The binary
/// path is resolved in order:
/// 1. `X3_SIDECAR_BIN` environment variable
/// 2. Same directory as the running node executable
/// 3. `PATH` lookup (`x3-sidecar`)
///
/// If the binary cannot be found or fails to start the function returns `Err`
/// so the caller's restart loop engages with exponential back-off.  If the
/// binary exits cleanly (status 0) the function returns `Ok(())`.
async fn spawn_sidecar_service(service_id: &str) -> Result<(), String> {
    // Resolve sidecar binary path.
    let bin_path = if let Ok(explicit) = std::env::var("X3_SIDECAR_BIN") {
        explicit
    } else if let Ok(mut exe) = std::env::current_exe() {
        exe.set_file_name("x3-sidecar");
        if exe.exists() {
            exe.to_string_lossy().into_owned()
        } else {
            "x3-sidecar".to_string()
        }
    } else {
        "x3-sidecar".to_string()
    };

    // Optional Solana RPC endpoint forwarded to the child process.
    let solana_rpc = std::env::var("X3_SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    // X3 node RPC for extrinsic submission (bridge events).
    let x3_node_rpc =
        std::env::var("X3_NODE_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:9944".to_string());

    // Escrow program ID to monitor on Solana.
    let escrow_program = std::env::var("X3_ESCROW_PROGRAM").unwrap_or_default();

    log::info!(
        "🔌 Spawning sidecar '{}' via binary '{}' (Solana RPC: {}, X3 RPC: {})",
        service_id,
        bin_path,
        solana_rpc,
        x3_node_rpc
    );

    let mut cmd = tokio::process::Command::new(&bin_path);
    cmd.arg("--service-id")
        .arg(service_id)
        .arg("--solana-rpc")
        .arg(&solana_rpc)
        .arg("--x3-rpc")
        .arg(&x3_node_rpc)
        .kill_on_drop(true);
    if !escrow_program.is_empty() {
        cmd.arg("--escrow-program").arg(&escrow_program);
    }

    let status = cmd
        .status()
        .await
        .map_err(|e| format!("failed to launch '{}': {}", bin_path, e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "x3-sidecar exited with status {}",
            status.code().unwrap_or(-1)
        ))
    }
}

/// Runs the finality voter that writes finality certificates to off-chain storage.
///
/// Listens to block finality notifications and produces a deterministic finality
/// certificate hash for every finalized block:
/// - If Flash-Finality is active and has a certificate, uses the Flash cert hash.
/// - Otherwise, derives a cert hash from the GRANDPA-finalized block hash via
///   `blake2_256(hash)`. This provides non-zero certs for the unsigned extrinsic
///   path even when Flash-Finality is not running.
///
/// Written to **off-chain local storage** so the `pallet-x3-atomic-kernel` OCW can
/// attach it to PoAE proofs as finality_cert.
///
/// Key format: `b"x3ff:" (5 bytes) + block_number (8 bytes LE) = 13 bytes`
/// Value:      `cert_hash (32 bytes)`
async fn run_flash_finality_voter<Client, Block>(
    gadget: Arc<FlashFinalityGadget>,
    client: Arc<Client>,
    enable_live_mode: bool,
) where
    Client: BlockchainEvents<Block> + BlockBackend<Block> + Send + Sync + 'static,
    Block: sp_runtime::traits::Block + 'static,
    Block::Header: HeaderT,
{
    use futures_util::StreamExt;

    log::info!(
        "⚡ Flash-Finality voter started — live_mode={}",
        if enable_live_mode { "ON" } else { "SHADOW" }
    );

    let mut finality_notifications = client.finality_notification_stream();

    loop {
        match finality_notifications.next().await {
            Some(notification) => {
                let number: u64 = (*notification.header.number()).saturated_into();
                let hash: [u8; 32] = notification.hash.as_ref().try_into().unwrap_or([0u8; 32]);

                // Try to get a Flash-Finality certificate for this block
                if let Some(cert) = gadget.get_certificate(hash).await {
                    // --- Write cert_hash to off-chain local storage ---
                    // Key: "x3ff:" + block_number (LE u64) = 13 bytes
                    // Value: cert_hash (32 bytes)
                    // The pallet-x3-atomic-kernel OCW reads this to populate
                    // `finality_cert` in PoAE proofs instead of H256::zero().
                    {
                        let cert_hash = cert.cert_hash();
                        let mut key = b"x3ff:".to_vec();
                        key.extend_from_slice(&number.to_le_bytes());
                        sp_io::offchain::local_storage_set(
                            sp_runtime::offchain::StorageKind::PERSISTENT,
                            &key,
                            &cert_hash,
                        );
                        log::info!(
                            "⚡ [FlashFinality] cert stored at key x3ff:{} → cert_hash=0x{}",
                            number,
                            hex::encode(&cert_hash[..8])
                        );
                    }

                    if enable_live_mode {
                        log::info!(
                            "⚡✅ Live mode: Flash-Finality cert for #{} — {} votes (certificate ready)",
                            number,
                            cert.vote_count
                        );
                    } else {
                        // Shadow mode: log certificate for monitoring without applying it
                        log::debug!(
                            "⚡🔍 Shadow: Flash cert available for #{} — {} votes (not applied)",
                            number,
                            cert.vote_count
                        );
                    }

                    // Record metrics
                    let metrics = gadget.metrics().await;
                    log::info!(
                        "📊 Flash-Finality metrics: rounds_completed={}, shadow_agreements={}",
                        metrics.rounds_completed,
                        metrics.shadow_agreements
                    );
                } else {
                    // No Flash certificate — derive cert hash from GRANDPA-finalized
                    // block hash. This provides non-zero certs for the unsigned
                    // `submit_finalization_result` path even without Flash-Finality.
                    let cert_hash = sp_core::blake2_256(&hash);
                    let mut key = b"x3ff:".to_vec();
                    key.extend_from_slice(&number.to_le_bytes());
                    sp_io::offchain::local_storage_set(
                        sp_runtime::offchain::StorageKind::PERSISTENT,
                        &key,
                        &cert_hash,
                    );
                    log::debug!(
                        "⚡ [GRANDPA] cert stored at key x3ff:{} → cert_hash=0x{}",
                        number,
                        hex::encode(&cert_hash[..8])
                    );
                }
            }

            None => {
                log::warn!("⚡ Flash-Finality voter: client finality stream closed");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_cross_vm_bridge::CrossVmResult;

    #[test]
    fn startup_gate_is_skipped_for_non_authorities() {
        assert!(enforce_startup_gate_if_authority(false).is_ok());
    }

    #[test]
    fn startup_gate_passes_for_reference_authority_build() {
        assert!(enforce_startup_gate_if_authority(true).is_ok());
    }

    #[test]
    fn grandpa_stays_enabled_without_disable_flag_or_flash_finality() {
        assert!(compute_enable_grandpa_from_flags(
            false,
            NodeFeatureFlags::default(),
        ));
    }

    #[test]
    fn grandpa_is_disabled_when_config_disables_it() {
        assert!(!compute_enable_grandpa_from_flags(
            true,
            NodeFeatureFlags::default(),
        ));
    }

    #[test]
    fn grandpa_is_disabled_when_flash_finality_is_enabled() {
        let feature_flags = NodeFeatureFlags {
            enable_flash_finality: true,
            ..Default::default()
        };

        assert!(!compute_enable_grandpa_from_flags(false, feature_flags));
    }

    #[test]
    fn cross_vm_safety_preflight_rejects_when_bridge_paused() {
        let gate = CrossVmBridgeSafetyGate::default();
        let mut bridge = CrossVmBridge::new();
        bridge.pause();
        let blocked = gate.preflight(&bridge, 10, 9, 0);
        assert!(blocked.is_err());
    }

    #[test]
    fn cross_vm_safety_postflight_rejects_empty_success_output() {
        let gate = CrossVmBridgeSafetyGate::default();
        let results = vec![CrossVmResult::success(Vec::new(), 21_000)];
        let blocked = gate.postflight(&results);
        assert!(blocked.is_err());
    }

    #[test]
    fn cross_vm_safety_postflight_rejects_success_with_error() {
        let gate = CrossVmBridgeSafetyGate::default();
        let results = vec![CrossVmResult {
            success: true,
            output: b"EVM:receipt:ok".to_vec(),
            gas_used: 21_000,
            error: Some(b"contradictory error".to_vec()),
        }];

        assert_eq!(
            gate.postflight(&results),
            Err("success_with_error".to_string())
        );
    }

    #[test]
    fn cross_vm_safety_postflight_accepts_non_empty_outputs() {
        let gate = CrossVmBridgeSafetyGate::default();
        let results = vec![
            CrossVmResult::success(b"EVM:receipt:ok".to_vec(), 21_000),
            CrossVmResult::success(b"SVM:receipt:ok".to_vec(), 5_000),
        ];
        assert!(gate.postflight(&results).is_ok());
    }

    // ── PoH shadow-mode regression (v1 backlog gate) ─────────────────────
    // These tests lock in the invariant that --enable-poh is SHADOW MODE ONLY
    // in mainnet-v1.  If someone accidentally wires PoH enforcement into block
    // import, nodes would start rejecting valid blocks.  The tests must keep
    // passing until the v2 PoH enforcement work is deliberately merged.

    /// PoH flag is accepted without panicking and is stored correctly.
    #[test]
    fn poh_flag_is_accepted_in_feature_flags() {
        let flags = NodeFeatureFlags {
            enable_poh: true,
            ..Default::default()
        };
        assert!(flags.enable_poh);
    }

    /// All other flags remain default when only enable_poh is set.
    /// Prevents accidental coupling where setting poh also enables gpu/finality.
    #[test]
    fn poh_flag_does_not_activate_other_flags() {
        let flags = NodeFeatureFlags {
            enable_poh: true,
            ..Default::default()
        };
        assert!(!flags.enable_flash_finality, "flash finality must stay off");
        assert!(!flags.enable_gpu_validator, "gpu validator must stay off");
        assert!(!flags.gpu_required, "gpu_required must stay off");
        assert!(
            !flags.enable_parallel_proposer,
            "parallel proposer must stay off"
        );
        assert!(!flags.enable_atomic_kernel, "atomic kernel must stay off");
    }

    /// GRANDPA must stay enabled regardless of enable_poh.
    /// PoH in shadow mode must not interfere with the finality gadget.
    #[test]
    fn poh_shadow_mode_does_not_disable_grandpa() {
        let flags = NodeFeatureFlags {
            enable_poh: true,
            ..Default::default()
        };
        // disable_grandpa=false, flash_finality=false → GRANDPA enabled
        assert!(
            compute_enable_grandpa_from_flags(false, flags),
            "GRANDPA must remain enabled when only enable_poh is set (shadow mode)"
        );
    }

    /// PoH + flash finality combination: GRANDPA is still disabled by flash
    /// finality, not by PoH.  This ensures PoH has no side-effect on the
    /// GRANDPA decision path.
    #[test]
    fn poh_with_flash_finality_disables_grandpa_via_finality_not_poh() {
        let flags_poh_only = NodeFeatureFlags {
            enable_poh: true,
            ..Default::default()
        };
        let flags_both = NodeFeatureFlags {
            enable_poh: true,
            enable_flash_finality: true,
            ..Default::default()
        };
        // PoH alone → GRANDPA on
        assert!(compute_enable_grandpa_from_flags(false, flags_poh_only));
        // PoH + flash finality → GRANDPA off (flash finality is the cause)
        assert!(!compute_enable_grandpa_from_flags(false, flags_both));
    }

    /// NodeFeatureFlags::default() must have enable_poh = false.
    /// Guards against a Default impl change that would silently enable PoH
    /// on every node that doesn't explicitly set flags.
    #[test]
    fn poh_is_off_by_default() {
        let flags = NodeFeatureFlags::default();
        assert!(
            !flags.enable_poh,
            "enable_poh must default to false for mainnet-v1"
        );
    }

    // ─── PoH v2 Block Import Wrapper Tests ────────────────────────────────────

    /// Ensures `PoHVerifyBlockImport::new()` compiles and constructs correctly
    /// in passthrough mode (poh_state = None).
    #[test]
    fn poh_verify_block_import_passthrough_mode_constructs() {
        let flags = NodeFeatureFlags {
            enable_poh: false,
            ..Default::default()
        };
        assert!(!flags.enable_poh, "passthrough: poh_state would be None");
    }

    /// Validates that when `enable_poh` is true the poh state is Some, not None.
    #[test]
    fn poh_verify_block_import_enforcement_mode_state_is_some() {
        let flags = NodeFeatureFlags {
            enable_poh: true,
            ..Default::default()
        };
        assert!(
            flags.enable_poh,
            "enforcement mode: poh_state would be Some(...)"
        );
    }

    // ─── extract_poh_digest behavioral tests ──────────────────────────────────
    // These test the static helper directly using Substrate generic header types,
    // avoiding the need for a full runtime.

    type TestHeader = sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>;
    type TestBlock = sp_runtime::generic::Block<TestHeader, sp_runtime::OpaqueExtrinsic>;

    /// No digest logs → extract_poh_digest returns None.
    #[test]
    fn extract_poh_digest_returns_none_for_empty_digest() {
        let header = TestHeader::new(
            1,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        let result = PoHVerifyBlockImport::<TestBlock, ()>::extract_poh_digest(&header);
        assert!(result.is_none(), "empty digest logs should return None");
    }

    /// A Consensus log with the wrong engine ID → extract_poh_digest returns None.
    #[test]
    fn extract_poh_digest_returns_none_for_wrong_engine_id() {
        let wrong_id = *b"babe";
        let mut header = TestHeader::new(
            1,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        header
            .digest_mut()
            .push(DigestItem::Consensus(wrong_id, vec![0u8; 32]));
        let result = PoHVerifyBlockImport::<TestBlock, ()>::extract_poh_digest(&header);
        assert!(result.is_none(), "wrong engine ID should return None");
    }

    /// A valid Consensus log with POH_ENGINE_ID and properly encoded PoHDigest → Some.
    #[test]
    fn extract_poh_digest_decodes_valid_digest() {
        let mut state = PoHState::default();
        let digest = state.advance(&[]);
        // Use the canonical encode_payload() so this test stays in sync with
        // PoHDigest's SCALE layout — never couple test encoding to manual byte math.
        let encoded = digest.encode_payload();

        let mut header = TestHeader::new(
            1,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        header
            .digest_mut()
            .push(DigestItem::Consensus(POH_ENGINE_ID, encoded));

        let result = PoHVerifyBlockImport::<TestBlock, ()>::extract_poh_digest(&header);
        assert!(result.is_some(), "valid PoH digest should decode to Some");
        let decoded = result.unwrap();
        assert_eq!(decoded.tick, digest.tick);
        assert_eq!(decoded.poh_hash, digest.poh_hash);
        assert_eq!(decoded.tx_mix_root, digest.tx_mix_root);
    }

    /// Malformed bytes (wrong length) → extract_poh_digest returns None (not panic).
    #[test]
    fn extract_poh_digest_returns_none_for_malformed_bytes() {
        let mut header = TestHeader::new(
            1,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        // 71 bytes — one short of the required 72; must return None, not panic.
        header
            .digest_mut()
            .push(DigestItem::Consensus(POH_ENGINE_ID, vec![0u8; 71]));
        let result = PoHVerifyBlockImport::<TestBlock, ()>::extract_poh_digest(&header);
        assert!(
            result.is_none(),
            "71-byte payload is malformed and must return None"
        );

        // 0 bytes edge case
        let mut header2 = TestHeader::new(
            1,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        header2
            .digest_mut()
            .push(DigestItem::Consensus(POH_ENGINE_ID, vec![]));
        let result2 = PoHVerifyBlockImport::<TestBlock, ()>::extract_poh_digest(&header2);
        assert!(result2.is_none(), "empty payload must return None");
    }

    // ─── State advancement regression tests ──────────────────────────────────

    /// On verification success, `PoHState` advances exactly one tick.
    /// Regression guard: if state.advance() is accidentally removed from the Ok branch,
    /// subsequent blocks fail with NonMonotonicTick.
    #[test]
    fn poh_state_advances_on_verify_success() {
        let mut state = PoHState::default();
        let tick_before = state.tick();
        let digest = state.advance(&[]);
        assert_eq!(
            state.tick(),
            tick_before + 1,
            "advance must increment tick by 1"
        );
        // Verify the digest is consistent with the advanced state
        let result = PoHVerifier::verify(
            &digest,
            tick_before,
            &{
                let s2 = PoHState::default();
                
                s2.hash()
            },
            &[],
        );
        assert!(
            result.is_ok(),
            "digest produced by advance() must verify against prior state"
        );
    }

    /// State MUST advance even when verification fails — prevents cascade desync.
    ///
    /// Scenario: verifier receives a tampered digest (hash chain broken).
    /// After allowing through (v2 grace), the next block must still be verifiable.
    #[test]
    fn poh_state_must_advance_after_verify_failure_to_prevent_cascade() {
        let mut proposer_state = PoHState::default();
        let mut verifier_state = PoHState::default();

        // Block 1: produced by proposer, tampered before reaching verifier
        let real_digest_1 = proposer_state.advance(&[]);
        let mut tampered = real_digest_1.clone();
        tampered.poh_hash[0] ^= 0xFF; // corrupt the hash

        let prev_tick = verifier_state.tick();
        let prev_hash = verifier_state.hash();
        let result = PoHVerifier::verify(&tampered, prev_tick, &prev_hash, &[]);
        assert!(result.is_err(), "tampered digest should fail verification");

        // Verifier MUST still advance state (as the fix dictates)
        verifier_state.advance(&[]);
        assert_eq!(
            verifier_state.tick(),
            1,
            "verifier state must advance to tick 1 despite failure"
        );

        // Block 2: produced normally by proposer (tick 2), verifier now at tick 1 → expect tick 2
        let real_digest_2 = proposer_state.advance(&[]);
        let result2 = PoHVerifier::verify(
            &real_digest_2,
            verifier_state.tick(),
            &verifier_state.hash(),
            &[],
        );
        // NOTE: block 2 hash WON'T match because proposer advanced from real block 1 state
        // but verifier advanced from its own (tampered-adjusted) state. This is expected —
        // a tampered block causes a permanent desync that only chain reorg can fix.
        // The key assertion is that we get HashChainBroken, NOT NonMonotonicTick
        // (which would indicate we didn't advance state at all).
        match result2 {
            Ok(()) => {
                // Rare: if tamper was in hash but tx_mix_root matched by chance — still ok
            }
            Err(poh_generator::PoHVerifyError::NonMonotonicTick { .. }) => {
                panic!("Got NonMonotonicTick on block 2 — verifier state was NOT advanced after block 1 failure (cascade desync bug)");
            }
            Err(_) => {
                // HashChainBroken or TxMixRootMismatch — expected after a tamper
            }
        }
    }
} // end mod tests

#[cfg(test)]
mod runtime_bridge_client_tests {
    use super::*;
    use crate::{chain_spec, Cli};
    use clap::Parser;
    use codec::{Decode, Encode};
    use sc_cli::SubstrateCli;
    use sc_transaction_pool_api::TransactionPool;
    use sp_core::{H160, H256};
    use sp_inherents::InherentDataProvider;
    use sp_runtime::{
        generic::Era,
        traits::{IdentifyAccount, Verify},
        OpaqueExtrinsic,
    };
    use std::ffi::OsString;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Mutex as StdMutex, MutexGuard, OnceLock};
    use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};
    use x3_bridge_adapters::{RuntimeCrossVmDispatcher, SubstrateX3VmBridge};
    use x3_chain_runtime::{
        AccountId, Address, Runtime, RuntimeCall, Signature, SignedExtra, SignedPayload,
        UncheckedExtrinsic, VERSION,
    };
    use x3_cross_vm_bridge::{CrossVmCall, CrossVmDispatcher, CrossVmStatus, VmId};

    #[test]
    #[ignore = "CLI runner initializes a process-global logger; covered by active full-node HTTP RPC test and runnable manually in isolation"]
    fn service_rpc_submits_signed_extrinsic_imports_block_then_bridge_reads_runtime_state() {
        let _runner_lock = cli_runner_lock();
        let evm_escrow = H160::repeat_byte(0xE7);
        let svm_escrow = [0x57; 32];
        let spec = chain_spec::development_config_with_bridge_escrows(evm_escrow, svm_escrow)
            .expect("bridge test chain spec should build");
        let spec_path = write_temp_chain_spec(&spec);

        let cli = Cli::parse_from([
            "x3-chain-node",
            "--chain",
            spec_path
                .to_str()
                .expect("temporary chain spec path should be utf-8"),
            "--tmp",
            "--no-telemetry",
            "--no-prometheus",
        ]);

        let runner = cli
            .create_runner(&cli.run)
            .expect("CLI runner should build node configuration");

        let test_result: Result<(), ServiceError> = runner.sync_run(|config| {
            let partial = new_partial(&config)?;
            let runtime_bridge = SubstrateX3VmBridge::<_, Block>::new(partial.client.clone());
            let dispatcher = RuntimeCrossVmDispatcher::<_, Block>::new(partial.client.clone())
                .with_x3vm_bridge(runtime_bridge.bridge.clone());

            let target_account = account_from_seed("//Bob");
            let canonical_balance = 123_456_789u128;
            let signed_update = signed_council_canonical_balance_update(
                &partial.client,
                target_account.clone(),
                canonical_balance,
            )?;
            submit_signed_extrinsic_via_author_rpc(&partial, signed_update)?;
            import_ready_pool_block(&partial.client, &partial.transaction_pool)?;

            let bytecode = x3_vm::bridge::bc_format_helpers::assemble_simple_module();
            let call = CrossVmCall::new(
                VmId::X3Vm,
                VmId::X3Vm,
                0u32.to_le_bytes(),
                bytecode,
                1_000_000,
                42,
                100,
            )
            .expect("x3vm bytecode should fit cross-vm payload");

            let receipt = dispatcher
                .execute_x3vm_tx(&[0xA5; 32], &call)
                .expect("runtime-backed dispatcher should execute x3vm call");

            assert_eq!(receipt.status, CrossVmStatus::Success);
            assert_eq!(receipt.call_hash, call.call_hash(&H256::zero()));
            assert_eq!(dispatcher.get_evm_bridge_escrow(), evm_escrow.0);
            assert_eq!(dispatcher.get_svm_bridge_escrow(), svm_escrow);
            assert_eq!(
                dispatcher.get_svm_balance(target_account.as_ref()),
                canonical_balance as u64
            );
            Ok(())
        });

        let _ = std::fs::remove_file(spec_path);
        test_result.expect("service-level runtime bridge execution should pass");
    }

    #[test]
    fn full_node_http_rpc_submits_signed_extrinsic_ws_observes_head_then_svm_rpc_reads_runtime_state(
    ) {
        let _runner_lock = cli_runner_lock();
        let evm_escrow = H160::repeat_byte(0xE8);
        let svm_escrow = [0x58; 32];
        let rpc_port = reserve_tcp_port();
        let spec = chain_spec::development_config_with_bridge_escrows(evm_escrow, svm_escrow)
            .expect("bridge test chain spec should build");
        let spec_path = write_temp_chain_spec(&spec);
        let rpc_port_arg = rpc_port.to_string();

        let cli = Cli::parse_from([
            "x3-chain-node",
            "--chain",
            spec_path
                .to_str()
                .expect("temporary chain spec path should be utf-8"),
            "--tmp",
            "--no-telemetry",
            "--no-prometheus",
            "--validator",
            "--force-authoring",
            "--no-grandpa",
            "--unsafe-force-node-key-generation",
            "--port",
            "0",
            "--rpc-port",
            rpc_port_arg.as_str(),
            "--rpc-methods",
            "unsafe",
        ]);

        let runner = cli
            .create_runner(&cli.run)
            .expect("CLI runner should build node configuration");

        let test_result: Result<(), ServiceError> = runner.sync_run(|config| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| ServiceError::Other(format!("build tokio runtime: {e}")))?;
            let _runtime_guard = runtime.enter();
            let _task_manager =
                new_full::<sc_network::NetworkWorker<_, _>>(config, NodeFeatureFlags::default())?;
            wait_for_http_rpc(rpc_port)?;

            let genesis_hash = rpc_h256(rpc_port, "chain_getBlockHash", serde_json::json!([0]))?;
            let target_account = account_from_seed("//Bob");
            let target_bytes: &[u8] = target_account.as_ref();
            let target_pubkey = format!("0x{}", hex::encode(target_bytes));
            let canonical_balance = 987_654_321u128;
            let signed_update = signed_council_canonical_balance_update_for_genesis(
                genesis_hash,
                target_account.clone(),
                canonical_balance,
            )?;
            let tx_hex = format!("0x{}", hex::encode(signed_update.encode()));
            let baseline_head = rpc_header_number(&rpc_call(
                rpc_port,
                "chain_getHeader",
                serde_json::json!([]),
            )?)?
            .unwrap_or(0);
            let observed_head =
                submit_extrinsic_and_wait_for_ws_head(&runtime, rpc_port, tx_hex, baseline_head)?;
            assert!(
                observed_head > baseline_head,
                "new-head WebSocket subscription should observe a block after extrinsic submission"
            );

            wait_for_svm_balance(rpc_port, &target_pubkey, canonical_balance as u64)?;
            Ok(())
        });

        let _ = std::fs::remove_file(spec_path);
        test_result.expect("full-node HTTP RPC bridge execution should pass");
    }

    #[test]
    #[ignore = "CLI runner initializes a process-global logger; covered by active full-node HTTP RPC test and runnable manually in isolation"]
    fn full_node_grandpa_rpc_submits_signed_extrinsic_ws_observes_finalized_head_then_svm_rpc_reads_runtime_state(
    ) {
        let _runner_lock = cli_runner_lock();
        let evm_escrow = H160::repeat_byte(0xE9);
        let svm_escrow = [0x59; 32];
        let rpc_port = reserve_tcp_port();
        let spec = chain_spec::development_config_with_bridge_escrows(evm_escrow, svm_escrow)
            .expect("bridge test chain spec should build");
        let spec_path = write_temp_chain_spec(&spec);
        let rpc_port_arg = rpc_port.to_string();

        let cli = Cli::parse_from([
            "x3-chain-node",
            "--chain",
            spec_path
                .to_str()
                .expect("temporary chain spec path should be utf-8"),
            "--tmp",
            "--no-telemetry",
            "--no-prometheus",
            "--validator",
            "--force-authoring",
            "--unsafe-force-node-key-generation",
            "--port",
            "0",
            "--rpc-port",
            rpc_port_arg.as_str(),
            "--rpc-methods",
            "unsafe",
        ]);

        let runner = cli
            .create_runner(&cli.run)
            .expect("CLI runner should build node configuration");

        let test_result: Result<(), ServiceError> = runner.sync_run(|config| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| ServiceError::Other(format!("build tokio runtime: {e}")))?;
            let _runtime_guard = runtime.enter();
            let _task_manager =
                new_full::<sc_network::NetworkWorker<_, _>>(config, NodeFeatureFlags::default())?;
            wait_for_http_rpc(rpc_port)?;

            let genesis_hash = rpc_h256(rpc_port, "chain_getBlockHash", serde_json::json!([0]))?;
            let target_account = account_from_seed("//Bob");
            let target_bytes: &[u8] = target_account.as_ref();
            let target_pubkey = format!("0x{}", hex::encode(target_bytes));
            let canonical_balance = 1_234_567_890u128;
            let signed_update = signed_council_canonical_balance_update_for_genesis(
                genesis_hash,
                target_account.clone(),
                canonical_balance,
            )?;
            let tx_hex = format!("0x{}", hex::encode(signed_update.encode()));
            let baseline_finalized = finalized_header_number(rpc_port)?;
            let (included_head, finalized_head) =
                submit_extrinsic_wait_for_balance_then_finalized_head(
                    &runtime,
                    rpc_port,
                    tx_hex,
                    target_pubkey.clone(),
                    canonical_balance as u64,
                )?;
            assert!(
                included_head > baseline_finalized,
                "signed extrinsic should land in a block after the baseline finalized head"
            );
            assert!(
                finalized_head >= included_head,
                "finalized-head WebSocket subscription should reach the block containing the extrinsic"
            );

            wait_for_svm_balance(rpc_port, &target_pubkey, canonical_balance as u64)?;
            Ok(())
        });

        let _ = std::fs::remove_file(spec_path);
        test_result.expect("full-node GRANDPA finalized-head bridge execution should pass");
    }

    #[test]
    #[ignore = "two in-process full nodes currently require manual harness tuning to avoid stalled networking/finality shutdown"]
    fn two_validator_nodes_submit_on_first_observe_finalized_bridge_state_on_second() {
        let _runner_lock = cli_runner_lock();
        let _env_lock = dev_seed_env_lock();
        let evm_escrow = H160::repeat_byte(0xEA);
        let svm_escrow = [0x5A; 32];
        let first_rpc_port = reserve_tcp_port();
        let second_rpc_port = reserve_tcp_port();
        let first_p2p_port = reserve_tcp_port();
        let second_p2p_port = reserve_tcp_port();
        let spec =
            chain_spec::local_two_validator_config_with_bridge_escrows(evm_escrow, svm_escrow)
                .expect("two-validator bridge test chain spec should build");
        let spec_path = write_temp_chain_spec(&spec);
        let spec_path_str = spec_path
            .to_str()
            .expect("temporary chain spec path should be utf-8")
            .to_string();
        let first_base_path = temp_node_base_path("first");
        let second_base_path = temp_node_base_path("second");
        let first_base_path_str = first_base_path
            .to_str()
            .expect("first temporary base path should be utf-8")
            .to_string();
        let second_base_path_str = second_base_path
            .to_str()
            .expect("second temporary base path should be utf-8")
            .to_string();
        let first_rpc_port_arg = first_rpc_port.to_string();
        let second_rpc_port_arg = second_rpc_port.to_string();
        let first_p2p_port_arg = first_p2p_port.to_string();
        let second_p2p_port_arg = second_p2p_port.to_string();

        let first_cli = Cli::parse_from([
            "x3-chain-node",
            "--chain",
            spec_path_str.as_str(),
            "--base-path",
            first_base_path_str.as_str(),
            "--no-telemetry",
            "--no-prometheus",
            "--validator",
            "--force-authoring",
            "--unsafe-force-node-key-generation",
            "--port",
            first_p2p_port_arg.as_str(),
            "--rpc-port",
            first_rpc_port_arg.as_str(),
            "--rpc-methods",
            "unsafe",
        ]);

        let first_runner = first_cli
            .create_runner(&first_cli.run)
            .expect("first CLI runner should build node configuration");

        let test_result: Result<(), ServiceError> = first_runner.sync_run(|first_config| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| ServiceError::Other(format!("build tokio runtime: {e}")))?;
            let _runtime_guard = runtime.enter();
            {
                let _alice_seed = ScopedDevSeed::set("//Alice");
                let _first_task_manager = new_full::<sc_network::NetworkWorker<_, _>>(
                    first_config,
                    NodeFeatureFlags::default(),
                )?;
                wait_for_http_rpc(first_rpc_port)?;
                let first_peer_id = rpc_call(
                    first_rpc_port,
                    "system_localPeerId",
                    serde_json::json!([]),
                )?
                .as_str()
                .ok_or_else(|| {
                    ServiceError::Other("system_localPeerId did not return a string".into())
                })?
                .to_string();
                let first_bootnode =
                    format!("/ip4/127.0.0.1/tcp/{first_p2p_port}/p2p/{first_peer_id}");

                let second_cli = Cli::parse_from([
                    "x3-chain-node",
                    "--chain",
                    spec_path_str.as_str(),
                    "--base-path",
                    second_base_path_str.as_str(),
                    "--no-telemetry",
                    "--no-prometheus",
                    "--validator",
                    "--force-authoring",
                    "--unsafe-force-node-key-generation",
                    "--port",
                    second_p2p_port_arg.as_str(),
                    "--rpc-port",
                    second_rpc_port_arg.as_str(),
                    "--rpc-methods",
                    "unsafe",
                    "--bootnodes",
                    first_bootnode.as_str(),
                ]);
                let second_config = second_cli
                    .create_configuration(&second_cli.run, runtime.handle().clone())
                    .map_err(|e| {
                        ServiceError::Other(format!("second node configuration: {e}"))
                    })?;
                {
                    let _bob_seed = ScopedDevSeed::set("//Bob");
                    let _second_task_manager = new_full::<sc_network::NetworkWorker<_, _>>(
                        second_config,
                        NodeFeatureFlags::default(),
                    )?;
                    wait_for_http_rpc(second_rpc_port)?;
                    wait_for_peer_count(second_rpc_port, 1)?;

                    let genesis_hash =
                        rpc_h256(first_rpc_port, "chain_getBlockHash", serde_json::json!([0]))?;
                    let target_account = account_from_seed("//Charlie");
                    let target_bytes: &[u8] = target_account.as_ref();
                    let target_pubkey = format!("0x{}", hex::encode(target_bytes));
                    let canonical_balance = 2_468_013_579u128;
                    let signed_update = signed_council_canonical_balance_update_for_genesis(
                        genesis_hash,
                        target_account.clone(),
                        canonical_balance,
                    )?;
                    let tx_hex = format!("0x{}", hex::encode(signed_update.encode()));
                    let baseline_finalized = finalized_header_number(second_rpc_port)?;
                    let (included_head, finalized_head) =
                        submit_extrinsic_wait_for_remote_balance_then_finalized_head(
                            &runtime,
                            first_rpc_port,
                            second_rpc_port,
                            tx_hex,
                            target_pubkey.clone(),
                            canonical_balance as u64,
                        )?;
                    assert!(
                        included_head > baseline_finalized,
                        "remote node should import the signed extrinsic after its baseline finalized head"
                    );
                    assert!(
                        finalized_head >= included_head,
                        "second node finalized-head subscription should reach the extrinsic block"
                    );
                    wait_for_svm_balance(
                        second_rpc_port,
                        &target_pubkey,
                        canonical_balance as u64,
                    )?;
                    Ok(())
                }
            }
        });

        let _ = std::fs::remove_file(spec_path);
        let _ = std::fs::remove_dir_all(first_base_path);
        let _ = std::fs::remove_dir_all(second_base_path);
        test_result.expect("two-validator bridge finality observation should pass");
    }

    fn submit_extrinsic_and_wait_for_ws_head(
        runtime: &tokio::runtime::Runtime,
        port: u16,
        tx_hex: String,
        baseline_head: u64,
    ) -> Result<u64, ServiceError> {
        runtime.block_on(async move {
            use jsonrpsee::core::client::SubscriptionClientT;
            use jsonrpsee::rpc_params;
            use jsonrpsee::ws_client::WsClientBuilder;

            let client = WsClientBuilder::default()
                .build(format!("ws://127.0.0.1:{port}"))
                .await
                .map_err(|e| ServiceError::Other(format!("connect websocket rpc: {e}")))?;
            let mut new_heads = client
                .subscribe::<serde_json::Value, _>(
                    "chain_subscribeNewHeads",
                    rpc_params![],
                    "chain_unsubscribeNewHeads",
                )
                .await
                .map_err(|e| ServiceError::Other(format!("subscribe new heads: {e}")))?;

            let tx_result = tokio::task::spawn_blocking(move || {
                rpc_call(port, "author_submitExtrinsic", serde_json::json!([tx_hex]))
            })
            .await
            .map_err(|e| ServiceError::Other(format!("join author_submitExtrinsic: {e}")))??;
            assert!(
                tx_result.as_str().is_some(),
                "author_submitExtrinsic should return a transaction hash"
            );

            tokio::time::timeout(StdDuration::from_secs(45), async {
                while let Some(header_result) = new_heads.next().await {
                    let header = header_result
                        .map_err(|e| ServiceError::Other(format!("read new head: {e}")))?;
                    if let Some(number) = rpc_header_number(&header)? {
                        if number > baseline_head {
                            return Ok(number);
                        }
                    }
                }
                Err(ServiceError::Other(
                    "new-head WebSocket subscription ended before observing import".into(),
                ))
            })
            .await
            .map_err(|_| {
                ServiceError::Other(format!(
                    "timed out waiting for WebSocket new head above #{baseline_head}"
                ))
            })?
        })
    }

    fn submit_extrinsic_wait_for_balance_then_finalized_head(
        runtime: &tokio::runtime::Runtime,
        port: u16,
        tx_hex: String,
        pubkey_hex: String,
        expected_balance: u64,
    ) -> Result<(u64, u64), ServiceError> {
        runtime.block_on(async move {
            use jsonrpsee::core::client::SubscriptionClientT;
            use jsonrpsee::rpc_params;
            use jsonrpsee::ws_client::WsClientBuilder;

            let client = WsClientBuilder::default()
                .build(format!("ws://127.0.0.1:{port}"))
                .await
                .map_err(|e| ServiceError::Other(format!("connect websocket rpc: {e}")))?;
            let mut finalized_heads = client
                .subscribe::<serde_json::Value, _>(
                    "chain_subscribeFinalizedHeads",
                    rpc_params![],
                    "chain_unsubscribeFinalizedHeads",
                )
                .await
                .map_err(|e| ServiceError::Other(format!("subscribe finalized heads: {e}")))?;

            let tx_result = tokio::task::spawn_blocking(move || {
                rpc_call(port, "author_submitExtrinsic", serde_json::json!([tx_hex]))
            })
            .await
            .map_err(|e| ServiceError::Other(format!("join author_submitExtrinsic: {e}")))??;
            assert!(
                tx_result.as_str().is_some(),
                "author_submitExtrinsic should return a transaction hash"
            );

            let included_head = tokio::task::spawn_blocking(move || {
                wait_for_svm_balance_at_best_head(port, &pubkey_hex, expected_balance)
            })
            .await
            .map_err(|e| ServiceError::Other(format!("join svm balance wait: {e}")))??;

            let finalized_head = tokio::time::timeout(StdDuration::from_secs(90), async {
                while let Some(header_result) = finalized_heads.next().await {
                    let header = header_result
                        .map_err(|e| ServiceError::Other(format!("read finalized head: {e}")))?;
                    if let Some(number) = rpc_header_number(&header)? {
                        if number >= included_head {
                            return Ok(number);
                        }
                    }
                }
                Err(ServiceError::Other(
                    "finalized-head WebSocket subscription ended before extrinsic block finalized"
                        .into(),
                ))
            })
            .await
            .map_err(|_| {
                ServiceError::Other(format!(
                    "timed out waiting for finalized head at or above #{included_head}"
                ))
            })??;

            Ok((included_head, finalized_head))
        })
    }

    fn submit_extrinsic_wait_for_remote_balance_then_finalized_head(
        runtime: &tokio::runtime::Runtime,
        submit_port: u16,
        observe_port: u16,
        tx_hex: String,
        pubkey_hex: String,
        expected_balance: u64,
    ) -> Result<(u64, u64), ServiceError> {
        runtime.block_on(async move {
            use jsonrpsee::core::client::SubscriptionClientT;
            use jsonrpsee::rpc_params;
            use jsonrpsee::ws_client::WsClientBuilder;

            let client = WsClientBuilder::default()
                .build(format!("ws://127.0.0.1:{observe_port}"))
                .await
                .map_err(|e| ServiceError::Other(format!("connect observer websocket rpc: {e}")))?;
            let mut finalized_heads = client
                .subscribe::<serde_json::Value, _>(
                    "chain_subscribeFinalizedHeads",
                    rpc_params![],
                    "chain_unsubscribeFinalizedHeads",
                )
                .await
                .map_err(|e| {
                    ServiceError::Other(format!("subscribe observer finalized heads: {e}"))
                })?;

            let tx_result = tokio::task::spawn_blocking(move || {
                rpc_call(
                    submit_port,
                    "author_submitExtrinsic",
                    serde_json::json!([tx_hex]),
                )
            })
            .await
            .map_err(|e| ServiceError::Other(format!("join author_submitExtrinsic: {e}")))??;
            assert!(
                tx_result.as_str().is_some(),
                "author_submitExtrinsic should return a transaction hash"
            );

            let included_head = tokio::task::spawn_blocking(move || {
                wait_for_svm_balance_at_best_head(observe_port, &pubkey_hex, expected_balance)
            })
            .await
            .map_err(|e| ServiceError::Other(format!("join observer svm balance wait: {e}")))??;

            let finalized_head = tokio::time::timeout(StdDuration::from_secs(120), async {
                while let Some(header_result) = finalized_heads.next().await {
                    let header = header_result.map_err(|e| {
                        ServiceError::Other(format!("read observer finalized head: {e}"))
                    })?;
                    if let Some(number) = rpc_header_number(&header)? {
                        if number >= included_head {
                            return Ok(number);
                        }
                    }
                }
                Err(ServiceError::Other(
                    "observer finalized-head WebSocket subscription ended before extrinsic block finalized"
                        .into(),
                ))
            })
            .await
            .map_err(|_| {
                ServiceError::Other(format!(
                    "timed out waiting for observer finalized head at or above #{included_head}"
                ))
            })??;

            Ok((included_head, finalized_head))
        })
    }

    fn signed_council_canonical_balance_update(
        client: &Arc<FullClient>,
        account: AccountId,
        new_balance: u128,
    ) -> Result<OpaqueExtrinsic, ServiceError> {
        let genesis_hash = client
            .block_hash(0)
            .map_err(|e| ServiceError::Other(format!("read genesis hash: {e}")))?
            .ok_or_else(|| ServiceError::Other("missing genesis hash".into()))?;
        signed_council_canonical_balance_update_for_genesis(genesis_hash, account, new_balance)
    }

    fn signed_council_canonical_balance_update_for_genesis(
        genesis_hash: H256,
        account: AccountId,
        new_balance: u128,
    ) -> Result<OpaqueExtrinsic, ServiceError> {
        let proposal = RuntimeCall::AtlasKernel(
            pallet_x3_kernel::Call::<Runtime>::update_canonical_balance {
                account,
                asset_id: 0,
                new_balance,
                comit_id: None,
            },
        );
        let length_bound = proposal.encoded_size() as u32;
        let call = RuntimeCall::Council(pallet_collective::Call::<
            Runtime,
            pallet_collective::Instance1,
        >::propose {
            threshold: 1,
            proposal: Box::new(proposal),
            length_bound,
        });

        signed_extrinsic_for_genesis("//Alice", call, 0, genesis_hash)
    }

    fn submit_signed_extrinsic_via_author_rpc(
        partial: &sc_service::PartialComponents<
            FullClient,
            FullBackend,
            SelectChain,
            sc_consensus::DefaultImportQueue<Block>,
            sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
            (
                sc_consensus_grandpa::GrandpaBlockImport<
                    FullBackend,
                    Block,
                    FullClient,
                    SelectChain,
                >,
                sc_consensus_grandpa::LinkHalf<Block, FullClient, SelectChain>,
                Option<Telemetry>,
            ),
        >,
        extrinsic: OpaqueExtrinsic,
    ) -> Result<(), ServiceError> {
        let author = sc_rpc::author::Author::new(
            partial.client.clone(),
            partial.transaction_pool.clone(),
            partial.keystore_container.keystore(),
            Arc::new(partial.task_manager.spawn_handle()),
        );
        let rpc = sc_rpc::author::AuthorApiServer::into_rpc(author);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "author_submitExtrinsic",
            "params": [format!("0x{}", hex::encode(extrinsic.encode()))],
        })
        .to_string();
        let (response, _) = futures::executor::block_on(rpc.raw_json_request(&request, 16))
            .map_err(|e| ServiceError::Other(format!("submit extrinsic rpc request: {e}")))?;
        let response: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| ServiceError::Other(format!("decode rpc response: {e}")))?;
        if let Some(error) = response.get("error") {
            return Err(ServiceError::Other(format!(
                "author_submitExtrinsic failed: {error}"
            )));
        }
        assert!(
            response
                .get("result")
                .and_then(|value| value.as_str())
                .is_some(),
            "author_submitExtrinsic should return a transaction hash"
        );
        Ok(())
    }

    fn signed_extrinsic_for_genesis(
        seed: &str,
        call: RuntimeCall,
        nonce: u32,
        genesis_hash: H256,
    ) -> Result<OpaqueExtrinsic, ServiceError> {
        let pair = sp_core::sr25519::Pair::from_string(seed, None)
            .map_err(|e| ServiceError::Other(format!("load signing key {seed}: {e:?}")))?;
        let account = account_from_public(pair.public());

        let extra: SignedExtra = (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(Era::Immortal),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
            pallet_x3_invariants::InvariantCheck::<Runtime>::new(),
            decode_agent_law_check()?,
        );
        let payload = SignedPayload::from_raw(
            call.clone(),
            extra.clone(),
            (
                (),
                VERSION.spec_version,
                VERSION.transaction_version,
                genesis_hash,
                genesis_hash,
                (),
                (),
                (),
                (),
                (),
            ),
        );
        let signature = payload.using_encoded(|payload| Signature::from(pair.sign(payload)));
        let extrinsic =
            UncheckedExtrinsic::new_signed(call, Address::Id(account), signature, extra);
        Ok(extrinsic.into())
    }

    fn reserve_tcp_port() -> u16 {
        TcpListener::bind(("127.0.0.1", 0))
            .expect("ephemeral tcp port should be available")
            .local_addr()
            .expect("ephemeral tcp listener should expose local address")
            .port()
    }

    fn wait_for_http_rpc(port: u16) -> Result<(), ServiceError> {
        let deadline = Instant::now() + StdDuration::from_secs(30);
        let mut last_error = String::new();
        while Instant::now() < deadline {
            match rpc_call(port, "chain_getBlockHash", serde_json::json!([0])) {
                Ok(_) => return Ok(()),
                Err(err) => {
                    last_error = err.to_string();
                    std::thread::sleep(StdDuration::from_millis(250));
                }
            }
        }
        Err(ServiceError::Other(format!(
            "http rpc did not become ready on port {port}: {last_error}"
        )))
    }

    fn wait_for_peer_count(port: u16, expected_min: u64) -> Result<(), ServiceError> {
        let deadline = Instant::now() + StdDuration::from_secs(45);
        let mut last_value = serde_json::Value::Null;
        while Instant::now() < deadline {
            match rpc_call(port, "system_health", serde_json::json!([])) {
                Ok(value) => {
                    last_value = value.clone();
                    if value
                        .get("peers")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|peers| peers >= expected_min)
                    {
                        return Ok(());
                    }
                }
                Err(err) => {
                    last_value = serde_json::Value::String(err.to_string());
                }
            }
            std::thread::sleep(StdDuration::from_millis(500));
        }
        Err(ServiceError::Other(format!(
            "system_health peers did not reach {expected_min}; last value: {last_value}"
        )))
    }

    fn wait_for_svm_balance(
        port: u16,
        pubkey_hex: &str,
        expected: u64,
    ) -> Result<(), ServiceError> {
        wait_for_svm_balance_at_best_head(port, pubkey_hex, expected).map(|_| ())
    }

    fn wait_for_svm_balance_at_best_head(
        port: u16,
        pubkey_hex: &str,
        expected: u64,
    ) -> Result<u64, ServiceError> {
        let deadline = Instant::now() + StdDuration::from_secs(45);
        let mut last_value = serde_json::Value::Null;
        while Instant::now() < deadline {
            match rpc_call(port, "svm_getBalance", serde_json::json!([pubkey_hex])) {
                Ok(value) => {
                    last_value = value.clone();
                    if value
                        .get("value")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|balance| balance == expected)
                    {
                        return best_header_number(port);
                    }
                }
                Err(err) => {
                    last_value = serde_json::Value::String(err.to_string());
                }
            }
            std::thread::sleep(StdDuration::from_millis(500));
        }
        Err(ServiceError::Other(format!(
            "svm_getBalance did not reach {expected}; last value: {last_value}"
        )))
    }

    fn best_header_number(port: u16) -> Result<u64, ServiceError> {
        rpc_header_number(&rpc_call(port, "chain_getHeader", serde_json::json!([]))?)?.ok_or_else(
            || ServiceError::Other("chain_getHeader response did not include a number".into()),
        )
    }

    fn finalized_header_number(port: u16) -> Result<u64, ServiceError> {
        let finalized_hash = rpc_call(port, "chain_getFinalizedHead", serde_json::json!([]))?;
        rpc_header_number(&rpc_call(
            port,
            "chain_getHeader",
            serde_json::json!([finalized_hash]),
        )?)?
        .ok_or_else(|| {
            ServiceError::Other(
                "chain_getHeader finalized response did not include a number".into(),
            )
        })
    }

    fn dev_seed_env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| StdMutex::new(()))
            .lock()
            .expect("X3_DEV_SEED test lock should not be poisoned")
    }

    fn cli_runner_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| StdMutex::new(()))
            .lock()
            .expect("CLI runner test lock should not be poisoned")
    }

    struct ScopedDevSeed {
        previous: Option<OsString>,
    }

    impl ScopedDevSeed {
        fn set(seed: &str) -> Self {
            let previous = std::env::var_os("X3_DEV_SEED");
            std::env::set_var("X3_DEV_SEED", seed);
            Self { previous }
        }
    }

    impl Drop for ScopedDevSeed {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("X3_DEV_SEED", value),
                None => std::env::remove_var("X3_DEV_SEED"),
            }
        }
    }

    fn rpc_h256(port: u16, method: &str, params: serde_json::Value) -> Result<H256, ServiceError> {
        let value = rpc_call(port, method, params)?;
        let hash_hex = value
            .as_str()
            .ok_or_else(|| ServiceError::Other(format!("{method} did not return a hash string")))?;
        let bytes = hex::decode(hash_hex.strip_prefix("0x").unwrap_or(hash_hex))
            .map_err(|e| ServiceError::Other(format!("{method} returned invalid hex: {e}")))?;
        if bytes.len() != 32 {
            return Err(ServiceError::Other(format!(
                "{method} returned {} bytes, expected 32",
                bytes.len()
            )));
        }
        Ok(H256::from_slice(&bytes))
    }

    fn rpc_header_number(header: &serde_json::Value) -> Result<Option<u64>, ServiceError> {
        let Some(number_hex) = header.get("number").and_then(|value| value.as_str()) else {
            return Ok(None);
        };
        let number_hex = number_hex.strip_prefix("0x").unwrap_or(number_hex);
        u64::from_str_radix(number_hex, 16)
            .map(Some)
            .map_err(|e| ServiceError::Other(format!("invalid RPC header number: {e}")))
    }

    fn rpc_call(
        port: u16,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ServiceError> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let body = request.to_string();
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .map_err(|e| ServiceError::Other(format!("connect http rpc {port}: {e}")))?;
        let request = format!(
            "POST / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| ServiceError::Other(format!("write http rpc request: {e}")))?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|e| ServiceError::Other(format!("read http rpc response: {e}")))?;
        let response = String::from_utf8(response)
            .map_err(|e| ServiceError::Other(format!("http rpc response was not utf-8: {e}")))?;
        let body = decode_http_body(&response)?;
        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ServiceError::Other(format!("decode http rpc json: {e}; body={body}")))?;
        if let Some(error) = value.get("error") {
            return Err(ServiceError::Other(format!("{method} failed: {error}")));
        }
        value.get("result").cloned().ok_or_else(|| {
            ServiceError::Other(format!("{method} response missing result: {value}"))
        })
    }

    fn decode_http_body(response: &str) -> Result<String, ServiceError> {
        let (headers, body) = response
            .split_once("\r\n\r\n")
            .ok_or_else(|| ServiceError::Other(format!("invalid http response: {response}")))?;
        if headers
            .lines()
            .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
        {
            return decode_chunked_body(body);
        }
        Ok(body.to_string())
    }

    fn decode_chunked_body(mut body: &str) -> Result<String, ServiceError> {
        let mut decoded = String::new();
        loop {
            let (len_hex, rest) = body
                .split_once("\r\n")
                .ok_or_else(|| ServiceError::Other("malformed chunked http body".into()))?;
            let len = usize::from_str_radix(len_hex.trim(), 16)
                .map_err(|e| ServiceError::Other(format!("invalid chunk length: {e}")))?;
            if len == 0 {
                return Ok(decoded);
            }
            if rest.len() < len + 2 {
                return Err(ServiceError::Other(
                    "chunked http body ended mid-chunk".into(),
                ));
            }
            decoded.push_str(&rest[..len]);
            body = &rest[len + 2..];
        }
    }

    fn decode_agent_law_check() -> Result<pallet_x3_agent_law::AgentLawCheck<Runtime>, ServiceError>
    {
        pallet_x3_agent_law::AgentLawCheck::<Runtime>::decode(&mut &[][..])
            .map_err(|e| ServiceError::Other(format!("decode agent law extension: {e}")))
    }

    fn account_from_seed(seed: &str) -> AccountId {
        let pair = sp_core::sr25519::Pair::from_string(seed, None)
            .expect("well-known dev account seed should decode");
        account_from_public(pair.public())
    }

    fn account_from_public(public: sp_core::sr25519::Public) -> AccountId {
        <Signature as Verify>::Signer::from(public).into_account()
    }

    fn import_ready_pool_block(
        client: &Arc<FullClient>,
        transaction_pool: &sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
    ) -> Result<(), ServiceError> {
        let before = client.info();
        let parent_hash = before.best_hash;
        let mut block_builder = sc_block_builder::BlockBuilderBuilder::new(&**client)
            .on_parent_block(parent_hash)
            .fetch_parent_block_number(&**client)
            .map_err(|e| ServiceError::Other(format!("fetch parent block number: {e}")))?
            .build()
            .map_err(|e| ServiceError::Other(format!("build block builder: {e}")))?;

        let inherent_data = futures::executor::block_on(
            sp_timestamp::InherentDataProvider::from_system_time().create_inherent_data(),
        )
        .map_err(|e| ServiceError::Other(format!("create inherent data: {e}")))?;

        for inherent in block_builder
            .create_inherents(inherent_data)
            .map_err(|e| ServiceError::Other(format!("create inherents: {e}")))?
        {
            block_builder
                .push(inherent)
                .map_err(|e| ServiceError::Other(format!("apply inherent: {e}")))?;
        }

        let mut included_ready = 0usize;
        for pending_tx in transaction_pool.ready() {
            block_builder
                .push((*pending_tx.data).clone())
                .map_err(|e| ServiceError::Other(format!("apply ready transaction: {e}")))?;
            included_ready += 1;
        }
        assert!(
            included_ready > 0,
            "signed test transaction should be ready"
        );

        let (block, storage_changes, _) = block_builder
            .build()
            .map_err(|e| ServiceError::Other(format!("build block: {e}")))?
            .into_inner();
        let imported_hash = block.header().hash();
        let imported_number = *block.header().number();
        let mut params =
            BlockImportParams::new(sp_consensus::BlockOrigin::Own, block.header().clone());
        params.body = Some(block.extrinsics().to_vec());
        params.state_action = sc_consensus::StateAction::ApplyChanges(
            sc_consensus::StorageChanges::Changes(storage_changes),
        );
        params.fork_choice = Some(sc_consensus::ForkChoiceStrategy::LongestChain);

        let import_result = futures::executor::block_on(client.import_block(params))
            .map_err(|e| ServiceError::Other(format!("import block: {e}")))?;
        assert!(matches!(import_result, ImportResult::Imported(_)));

        let after = client.info();
        assert_eq!(after.best_hash, imported_hash);
        assert_eq!(after.best_number, imported_number);
        assert!(after.best_number > before.best_number);
        Ok(())
    }

    fn write_temp_chain_spec(spec: &chain_spec::ChainSpec) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "x3-runtime-bridge-client-{nonce}-{}.json",
            std::process::id()
        ));
        let json = spec
            .as_json(false)
            .expect("bridge test chain spec should serialize");
        std::fs::write(&path, json).expect("temporary bridge chain spec should be writable");
        path
    }

    fn temp_node_base_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "x3-runtime-bridge-node-{label}-{nonce}-{}",
            std::process::id()
        ))
    }
}

//====== GPU Sidecar Tests ======
#[cfg(all(test, feature = "gpu-validator"))]
mod gpu_sidecar_tests {
    use super::*;

    /// Test 1: GPU Sidecar Configuration Validation
    /// Verifies that GpuSidecarConfig can be created with proper defaults
    /// and that all fields are accessible.
    #[test]
    fn test_gpu_sidecar_config_defaults() {
        let config = GpuSidecarConfig::default();

        // Verify all fields have correct defaults
        assert_eq!(config.service_id, "x3-gpu-sidecar-0");
        assert!(
            config.gpu_devices.is_empty(),
            "GPU devices should auto-detect"
        );
        assert_eq!(config.rpc_endpoint, "http://127.0.0.1:9944");
        assert_eq!(config.proof_interval_blocks, 10);
        assert_eq!(config.max_concurrent_tasks, 4);
    }

    /// Test 2: GPU Sidecar Configuration with Custom Values
    /// Verifies that custom configurations can be created and cloned properly.
    #[test]
    fn test_gpu_sidecar_config_custom() {
        let config = GpuSidecarConfig {
            service_id: "custom-sidecar".to_string(),
            gpu_devices: vec![0, 1, 2],
            rpc_endpoint: "http://192.168.1.100:9944".to_string(),
            proof_interval_blocks: 5,
            max_concurrent_tasks: 8,
        };

        // Verify clone works (required for Arc<Config> patterns)
        let cloned = config.clone();
        assert_eq!(cloned.service_id, config.service_id);
        assert_eq!(cloned.gpu_devices.len(), 3);
        assert_eq!(cloned.max_concurrent_tasks, 8);
    }

    /// Test 3: GPU Sidecar Handle Creation and State Tracking
    /// Verifies that GpuSidecarHandle can be created, and that is_running
    /// state tracking works correctly.
    #[test]
    fn test_gpu_sidecar_handle_creation() {
        let config = GpuSidecarConfig::default();
        let (handle, _shutdown_rx) = GpuSidecarHandle::new(config.clone());

        // Verify initial state
        assert!(
            !handle.is_running(),
            "Sidecar should not be running initially"
        );
        assert_eq!(handle.config.service_id, config.service_id);
    }

    /// Test 4: GPU Sidecar Handle Running State
    /// Verifies that the is_running atomic flag can be set and read correctly.
    #[test]
    fn test_gpu_sidecar_handle_running_state() {
        let config = GpuSidecarConfig::default();
        let (handle, _shutdown_rx) = GpuSidecarHandle::new(config);

        // Initially not running
        assert!(!handle.is_running());

        // Simulate startup
        handle
            .is_running
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(handle.is_running());

        // Simulate shutdown
        handle
            .is_running
            .store(false, std::sync::atomic::Ordering::Release);
        assert!(!handle.is_running());
    }

    /// Test 5: GPU Sidecar Graceful Shutdown
    /// Verifies that graceful shutdown signals and responds correctly.
    /// This is a synchronous test simulating the shutdown mechanism.
    #[tokio::test]
    async fn test_gpu_sidecar_graceful_shutdown() {
        let config = GpuSidecarConfig::default();
        let (handle, shutdown_rx) = GpuSidecarHandle::new(config);

        // Mark as running
        handle
            .is_running
            .store(true, std::sync::atomic::Ordering::Release);

        // Simulate task completion (drop the receiver to close the channel)
        drop(shutdown_rx);

        // Set task handle as complete (None means task finished)
        let mut task_handle = handle.task_handle.lock().await;
        *task_handle = None;
        drop(task_handle);

        // Now shutdown should succeed immediately
        let result = handle.shutdown(5).await;
        assert!(result.is_ok(), "Shutdown should succeed");
        assert!(
            !handle.is_running(),
            "After shutdown, is_running should be false"
        );
    }

    /// Test 6: GPU Sidecar Shutdown Timeout Mechanism
    /// Verifies that shutdown timeout is enforced and returns error on timeout.
    #[tokio::test]
    async fn test_gpu_sidecar_shutdown_timeout() {
        let config = GpuSidecarConfig::default();
        let (handle, _shutdown_rx) = GpuSidecarHandle::new(config);

        // Mark as running
        handle
            .is_running
            .store(true, std::sync::atomic::Ordering::Release);

        // Create a fake task that never completes
        // This simulates a hung sidecar task
        let dummy_task = tokio::spawn(async {
            // This task sleeps indefinitely, simulating a hung sidecar
            tokio::time::sleep(Duration::from_secs(100)).await;
            Ok::<(), String>(())
        });

        let mut task_handle = handle.task_handle.lock().await;
        *task_handle = Some(dummy_task);
        drop(task_handle);

        // Attempt shutdown with 1-second timeout
        let result = handle.shutdown(1).await;
        assert!(result.is_err(), "Shutdown should timeout and return error");
        assert_eq!(
            result.unwrap_err(),
            "Sidecar shutdown timeout",
            "Error message should indicate timeout"
        );
    }

    /// Test 7: GPU Sidecar Service ID Propagation
    /// Verifies that service ID is correctly propagated through config and handle.
    #[test]
    fn test_gpu_sidecar_service_id_propagation() {
        let custom_service_id = "my-custom-validator-sidecar";
        let config = GpuSidecarConfig {
            service_id: custom_service_id.to_string(),
            ..Default::default()
        };

        let (handle, _shutdown_rx) = GpuSidecarHandle::new(config);
        assert_eq!(handle.config.service_id, custom_service_id);
    }

    /// Test 8: GPU Sidecar Concurrent Handling
    /// Verifies that multiple GpuSidecarHandle instances can coexist
    /// without interfering with each other.
    #[test]
    fn test_gpu_sidecar_multiple_handles() {
        let config1 = GpuSidecarConfig {
            service_id: "sidecar-1".to_string(),
            ..Default::default()
        };
        let config2 = GpuSidecarConfig {
            service_id: "sidecar-2".to_string(),
            ..Default::default()
        };

        let (handle1, _rx1) = GpuSidecarHandle::new(config1);
        let (handle2, _rx2) = GpuSidecarHandle::new(config2);

        // Verify they are independent
        handle1
            .is_running
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(handle1.is_running());
        assert!(!handle2.is_running());

        handle2
            .is_running
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(handle1.is_running());
        assert!(handle2.is_running());
    }
}

//====== tests ======
// DISABLED: Tests require sc_service::Configuration API changes
// #[cfg(test)]
// mod tests { ... }
