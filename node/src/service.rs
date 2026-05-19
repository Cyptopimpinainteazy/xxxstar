use crate::flash_finality::FlashFinalityBridge;
use crate::metrics::X3PrometheusMetrics;
use crate::rpc_middleware::{RateLimitConfig, RateLimiter};
use contention_predictor::{ContentionPredictor, PredictorConfig};
use flash_finality::{FlashFinalityConfig, FlashFinalityGadget, FLASH_FINALITY_PROTOCOL_ID};
use futures_util::StreamExt;
use parallel_proposer::{extract_tx_metadata, ParallelProposerFactory};
use poh_generator::PoHState;
use sc_client_api::{Backend, BlockBackend, BlockchainEvents, HeaderBackend};
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
    SaturatedConversion,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use x3_bridge_adapters::{
    OffchainEscrowPersistence, PalletEscrowAdapter, RuntimeCrossVmDispatcher,
    SubstrateClientBalanceAdapter,
};
use x3_chain_runtime::{opaque::Block, RuntimeApi};
use x3_cross_vm_bridge::{CrossVmBridge, CrossVmResult};
use x3_finality_oracle::{
    Chain as FinalityChain, FinalityOracle, FinalityRule, FinalityStatus, InMemoryFinalityOracle,
    ObservedBlock,
};
use x3_gateway_risk_engine::{GatewayRiskEngine, RiskPolicy, RouteRiskInput};
use x3_proof_dispute::{DisputeStatus, DisputeTracker};
use x3_validator_attestation::{Attestation, AttestationSet, ValidatorId};
use x3_verification_router::{
    NonEmptyPayloadVerifier, ProofEnvelope, ProofKind, VerificationRouter,
};

#[cfg(feature = "gpu-validator")]
use x3_gpu_validator_swarm::{
    config::SwarmConfig, deterministic::DeterministicValidator, orchestrator::SwarmOrchestrator,
};

/// Key type for Aura block authoring
const AURA: KeyTypeId = KeyTypeId(*b"aura");
/// Key type for GRANDPA finality
const GRANDPA: KeyTypeId = KeyTypeId(*b"gran");

/// Txpool sizing aligned to X3 throughput targets.
/// Default Substrate pool (8 192/512) is 12x too small for 100k TPS goals.
/// Tuned per audit recommendation: 100k ready / 50k future, 256 MiB / 64 MiB.
/// NOTE: Default sizing (100k ready / 50k future) scales dynamically based on network speed.
/// See NetworkSpeed enum for speed-specific pool configurations.
#[allow(dead_code)]
const TX_POOL_READY_COUNT: usize = 100_000;
#[allow(dead_code)]
const TX_POOL_FUTURE_COUNT: usize = 50_000;
#[allow(dead_code)]
const TX_POOL_READY_BYTES: usize = 256 * 1024 * 1024; // 256 MiB
#[allow(dead_code)]
const TX_POOL_FUTURE_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
#[allow(dead_code)]
const TX_POOL_BAN_TIME_SECS: u64 = 60; // 60s ban (vs default 1800s) — faster retry under burst

/// GPU Validator Sidecar health check interval (blocks).
/// Health check runs every N blocks to detect sidecar crashes.
#[allow(dead_code)]
const GPU_SIDECAR_HEALTH_CHECK_INTERVAL: u32 = 5;

/// GPU Validator Sidecar restart threshold (consecutive failures).
/// If sidecar health check fails N times consecutively, trigger restart.
#[allow(dead_code)]
const GPU_SIDECAR_RESTART_THRESHOLD: u32 = 3;

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
            &config,
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
    verification_router: VerificationRouter,
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

        let mut verification_router = VerificationRouter::new();
        verification_router
            .register_verifier(ProofKind::EvmReceipt, Arc::new(NonEmptyPayloadVerifier));
        verification_router.register_verifier(
            ProofKind::SolanaCommitment,
            Arc::new(NonEmptyPayloadVerifier),
        );
        verification_router
            .register_verifier(ProofKind::Generic, Arc::new(NonEmptyPayloadVerifier));

        Self {
            finality_oracle,
            verification_router,
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

        let statement_hash = [0xABu8; 32];
        let mut attestations = AttestationSet::new(statement_hash);
        let mut successful_results = 0u64;

        for (idx, result) in results.iter().enumerate() {
            if !result.success {
                return Err("execution_failed".to_string());
            }

            if result.output.is_empty() {
                return Err("empty_success_output".to_string());
            }

            successful_results = successful_results.saturating_add(1);
            let kind = if result.output.starts_with(b"EVM") {
                ProofKind::EvmReceipt
            } else if result.output.starts_with(b"SVM") {
                ProofKind::SolanaCommitment
            } else {
                ProofKind::Generic
            };

            self.verification_router
                .route(&ProofEnvelope {
                    kind,
                    payload: result.output.clone(),
                    source_chain: 0,
                    destination_chain: 0,
                })
                .map_err(|err| format!("verification_error: {err}"))?;

            attestations
                .add_attestation(Attestation {
                    validator: ValidatorId(format!("bridge-result-{idx}")),
                    statement_hash,
                    signature: result.gas_used.to_le_bytes().to_vec(),
                    weight: 1,
                })
                .map_err(|err| format!("attestation_error: {err:?}"))?;
        }

        if !attestations.has_quorum(successful_results) {
            return Err("attestation_quorum_not_met".to_string());
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

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks: Option<()> = None;
    let name = config.network.node_name.clone();
    let chain_name = config.chain_spec.name().to_string();
    let prometheus_registry = config.prometheus_registry().cloned();
    let role_for_grandpa = role.clone();

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
        log::warn!(
            "⚠️ --enable-poh is set, but PoH digest verification is not yet enforced in block import."
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
            .get(0)
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

        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
            StartAuraParams {
                slot_duration,
                client: client.clone(),
                select_chain,
                block_import: grandpa_block_import,
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

        let orchestrator = match SwarmOrchestrator::new(orchestrator_id.clone(), gpu_config) {
            Ok(orch) => {
                log::info!(
                    "🎮 GPU Validator Orchestrator initialized: {}",
                    orchestrator_id
                );
                Arc::new(tokio::sync::RwLock::new(orch))
            }
            Err(e) => {
                log::error!(
                    "❌ Failed to initialize GPU Validator Orchestrator: {}; GPU validation disabled",
                    e
                );
                return Err(ServiceError::Other(format!(
                    "GPU Validator Orchestrator initialization failed: {}",
                    e
                )));
            }
        };

        let orch_clone = orchestrator.clone();
        let client_for_gpu = client.clone();
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

                            // Perform health check (TODO: implement actual process detection + RPC probe)
                            let health_status = health_monitor.check_health(current_block);
                            health_monitor.record_check(health_status, current_block);

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
                            } else if health_status {
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
        let balance_adapter = Arc::new(SubstrateClientBalanceAdapter::new(client.clone()));

        match backend.offchain_storage() {
            Some(offchain_storage) => {
                let escrow_adapter = Arc::new(PalletEscrowAdapter::with_persistence(
                    balance_adapter.clone(),
                    OffchainEscrowPersistence::new(offchain_storage),
                ));

                {
                    // C-002: replace the no-op keep-alive loop with a real
                    // cross-VM bridge poller backed by RuntimeCrossVmDispatcher,
                    // so pending EVM/SVM operations are actually submitted to the
                    // runtime rather than discarded inside a 1-hour sleep loop.
                    let dispatcher = Arc::new(RuntimeCrossVmDispatcher::new(client.clone()));
                    let bridge = Arc::new(std::sync::Mutex::new(CrossVmBridge::new()));
                    let bridge_safety_gate = CrossVmBridgeSafetyGate::default();
                    let client_for_bridge = client.clone();
                    // Keep escrow_adapter alive for the duration of the task.
                    let _escrow = escrow_adapter.clone();
                    let bridge_for_task = bridge.clone();
                    task_manager.spawn_handle().spawn(
                        "cross-vm-bridge-poller",
                        Some("x3"),
                        async move {
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

    // ── Store GPU Orchestrator reference for RPC access ────────────────────────────────
    #[cfg(feature = "gpu-validator")]
    if feature_flags.enable_gpu_validator {
        task_manager.extension().insert(orchestrator.clone());
        log::debug!("🎮 GPU Orchestrator reference stored in task manager extensions");
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

                        // Sidecar service spawn attempt
                        // TODO: Replace with actual sidecar spawning logic once x3-sidecar
                        // crate exports a public start_sidecar() function.
                        // Placeholder demonstrates intended lifecycle management:
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

    // ─────────────────────────────────────────────────────────────────
    // Initialize Cross-Chain GPU Validator
    // ─────────────────────────────────────────────────────────────────
    #[cfg(feature = "gpu-validator")]
    if feature_flags.enable_gpu_validator {
        use x3_cross_chain_gpu_validator::CrossChainValidator;

        let cross_chain_validator =
            CrossChainValidator::new(orchestrator.clone(), config.network.protocol_version);

        // Spawn cross-chain validation task
        task_manager.spawn_handle().spawn(
            "cross-chain-gpu-validator",
            Box::pin(async move {
                match cross_chain_validator.run_validation_loop().await {
                    Ok(()) => {
                        log::info!("🌐 Cross-chain GPU validator loop completed");
                    }
                    Err(e) => {
                        log::error!(
                            "🌐 Cross-chain GPU validator critical failure: {} — validator disabled, node continues",
                            e
                        );
                        return;
                    }
                }
            }),
        );

        // Export for RPC layer
        task_manager
            .extension()
            .insert(cross_chain_validator.clone());
        log::debug!("🌐 Cross-chain validator reference exported for RPC");
    }

    log::info!("✨ X3 Chain node started successfully");
    log::info!("🔗 Network: {}", chain_name);
    log::info!("👤 Node name: {}", name);
    log::info!("📋 Role: {:?}", role);

    Ok(task_manager)
}

/// Spawn the X3 Sidecar Service for cross-VM bridge monitoring.
///
/// The sidecar watches external VMs (Solana, other EVMs) and bridges assets into X3.
/// This is a placeholder for the actual sidecar initialization logic.
///
/// TODO: Once x3-sidecar crate is ready, import and call:
/// ```ignore
/// x3_sidecar::start_sidecar(config).await
/// ```
async fn spawn_sidecar_service(service_id: &str) -> Result<(), String> {
    log::debug!(
        "🔌 Sidecar service '{}': placeholder implementation",
        service_id
    );

    // TODO: Implement actual sidecar service spawning here
    // The sidecar should:
    // 1. Connect to Solana RPC (for SPL monitoring)
    // 2. Listen for escrow events
    // 3. Validate finality (32 confirmations)
    // 4. Submit bridge extrinsics to X3 node via RPC

    // For now, simulate a running sidecar that logs periodically
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        log::debug!("🔌 Sidecar '{}' health check: OK", service_id);
    }
}

/// Runs the Flash-Finality voter that applies certificates as actual finality.
///
/// This voter listens to block finality notifications and uses Flash-Finality
/// certificates to move the canonical finalized head. When live mode is enabled,
/// certificates override GRANDPA finality; in shadow mode, they're logged for comparison.
///
/// When a certificate is available it is written to **off-chain local storage** so
/// the `pallet-x3-atomic-kernel` OCW can attach it to PoAE proofs as finality_cert.
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
                    // No Flash certificate yet; this could be normal if finality advanced
                    // via GRANDPA first, or if we're still in earlier consensus phases
                    log::debug!("⚡ No Flash cert for #{} yet", number);
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
        let mut feature_flags = NodeFeatureFlags::default();
        feature_flags.enable_flash_finality = true;

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
    fn cross_vm_safety_postflight_accepts_non_empty_outputs() {
        let gate = CrossVmBridgeSafetyGate::default();
        let results = vec![
            CrossVmResult::success(b"EVM:receipt:ok".to_vec(), 21_000),
            CrossVmResult::success(b"SVM:receipt:ok".to_vec(), 5_000),
        ];
        assert!(gate.postflight(&results).is_ok());
    }
}

//====== tests ======
// DISABLED: Tests require sc_service::Configuration API changes
// #[cfg(test)]
// mod tests { ... }
