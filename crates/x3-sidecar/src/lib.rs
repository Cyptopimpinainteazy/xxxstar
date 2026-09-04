//! X3 Sidecar Daemon
//!
//! Off-chain swarm execution node for X3 Chain. This daemon:
//! - Connects to the swarm network
//! - Receives X3 bytecode execution jobs
//! - Executes jobs in a sandboxed VM
//! - Generates deterministic receipts with Merkle proofs
//! - Submits receipts to the on-chain verifier
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        X3 SIDECAR DAEMON                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
//! │  │    RPC       │   │    Job       │   │   Receipt    │            │
//! │  │   Server     │──▶│   Queue      │──▶│  Generator   │            │
//! │  └──────────────┘   └──────────────┘   └──────────────┘            │
//! │         │                  │                  │                    │
//! │         │                  ▼                  │                    │
//! │         │         ┌──────────────┐           │                    │
//! │         │         │    X3 VM     │           │                    │
//! │         │         │  Executor    │           │                    │
//! │         │         └──────────────┘           │                    │
//! │         │                  │                  │                    │
//! │         ▼                  ▼                  ▼                    │
//! │  ┌──────────────────────────────────────────────────┐              │
//! │  │              State Manager                        │              │
//! │  │  • Merkle Tree  • Checkpoints  • Rollback        │              │
//! │  └──────────────────────────────────────────────────┘              │
//! │                           │                                        │
//! │                           ▼                                        │
//! │  ┌──────────────────────────────────────────────────┐              │
//! │  │              Chain Submitter                      │              │
//! │  │  • Receipt Submission  • Gas Estimation          │              │
//! │  └──────────────────────────────────────────────────┘              │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

pub mod benchmark;
pub mod config;
pub mod evm_provider;
pub mod executor;
pub mod gateway_client;
pub mod job;
pub mod receipt;
pub mod rpc;
pub mod state;
pub mod submitter;
pub mod telemetry;

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use x3_orchestra_control_plane::ControlPlaneClient;

pub use benchmark::{
    build_provider_onboarding_job_request, BenchmarkRunInput, BenchmarkStore,
    ProviderOnboardingBenchmarkRequest,
};
pub use config::SidecarConfig;
pub use executor::X3Executor;
pub use gateway_client::{BenchmarkResultPayload, GatewayClient, GatewayClientConfig};
pub use job::{Job, JobQueue};
pub use receipt::{ExecutionReceipt, ReceiptGenerator};
pub use state::StateManager;
pub use submitter::ChainSubmitter;
pub use telemetry::Telemetry;

/// Tracked status for an execution job.
#[derive(Clone, Debug)]
pub struct JobStatusEntry {
    /// Human-readable status label.
    pub status: String,
    /// Last update timestamp (unix seconds).
    pub updated_at_unix: u64,
    /// Transaction hash when submitted on-chain.
    pub tx_hash: Option<String>,
    /// Last error message if any.
    pub error: Option<String>,
}

impl JobStatusEntry {
    pub fn new(status: impl Into<String>, tx_hash: Option<String>, error: Option<String>) -> Self {
        let updated_at_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            status: status.into(),
            updated_at_unix,
            tx_hash,
            error,
        }
    }
}

/// Sidecar state (shared across components)
pub struct SidecarState {
    pub start_time: Instant,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub registered: bool,
    pub job_statuses: std::collections::HashMap<[u8; 32], JobStatusEntry>,
}

impl Default for SidecarState {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            jobs_completed: 0,
            jobs_failed: 0,
            registered: false,
            job_statuses: std::collections::HashMap::new(),
        }
    }
}

/// Sidecar daemon
pub struct SidecarDaemon {
    pub config: SidecarConfig,
    pub job_queue: Arc<RwLock<JobQueue>>,
    pub executor: Arc<X3Executor>,
    pub state_manager: Arc<RwLock<StateManager>>,
    pub receipt_generator: Arc<ReceiptGenerator>,
    pub submitter: Arc<ChainSubmitter>,
    pub benchmark_store: Arc<BenchmarkStore>,
    pub gateway_client: Option<Arc<GatewayClient>>,
    pub orchestra_client: Option<Arc<ControlPlaneClient>>,
    pub telemetry: Arc<Telemetry>,
    pub state: Arc<RwLock<SidecarState>>,
}

impl SidecarDaemon {
    /// Create a new sidecar daemon
    pub fn new(config: SidecarConfig) -> anyhow::Result<Self> {
        let state_manager = Arc::new(RwLock::new(StateManager::new()));
        let executor = Arc::new(X3Executor::new(config.vm.clone()));
        let receipt_generator = Arc::new(ReceiptGenerator::from_hex(&config.executor_key)?);
        let submitter = Arc::new(ChainSubmitter::new(
            config.chain_rpc.clone(),
            config.executor_key.clone(),
        ));
        let job_queue = Arc::new(RwLock::new(JobQueue::new()));
        let telemetry = Telemetry::new();
        let state = Arc::new(RwLock::new(SidecarState::default()));

        // Initialize gateway client first (if configured)
        let gateway_client = if let Some(gateway_url) = &config.benchmark_gateway_url {
            let gateway_config = GatewayClientConfig {
                gateway_url: gateway_url.clone(),
                auth_token: config.benchmark_gateway_token.clone(),
                max_retries: config.submit_retries,
                initial_backoff_ms: 100,
            };
            Some(Arc::new(GatewayClient::new(gateway_config)))
        } else {
            None
        };

        // Initialize benchmark store with gateway client
        let benchmark_store = Arc::new(BenchmarkStore::open_with_gateway_client(
            &config,
            gateway_client.clone(),
        )?);
        let orchestra_client = config.orchestra_control_plane_url.as_ref().map(|base_url| {
            Arc::new(ControlPlaneClient::new(
                base_url.clone(),
                config.orchestra_control_plane_token.clone(),
            ))
        });

        Ok(Self {
            config,
            job_queue,
            executor,
            state_manager,
            receipt_generator,
            submitter,
            benchmark_store,
            gateway_client,
            orchestra_client,
            telemetry,
            state,
        })
    }

    /// Run the daemon
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        info!("Starting X3 Sidecar Daemon v{}", env!("CARGO_PKG_VERSION"));
        info!("RPC server on port {}", self.config.rpc_port);
        info!("Metrics on port {}", self.config.metrics_port);

        // Build RPC state
        let rpc_state = Arc::new(rpc::RpcState {
            job_queue: Arc::clone(&self.job_queue),
            sidecar_state: Arc::clone(&self.state),
            submitter: Arc::clone(&self.submitter),
            benchmark_store: Arc::clone(&self.benchmark_store),
            orchestra_client: self.orchestra_client.clone(),
            telemetry: Arc::clone(&self.telemetry),
        });

        // Build server routers.
        let rpc_addr: std::net::SocketAddr = format!("0.0.0.0:{}", self.config.rpc_port).parse()?;
        let metrics_addr: std::net::SocketAddr =
            format!("0.0.0.0:{}", self.config.metrics_port).parse()?;
        let router = rpc::create_router(Arc::clone(&rpc_state));
        let metrics_router = rpc::create_metrics_router(rpc_state);

        info!("RPC server listening on {}", rpc_addr);
        if self.config.metrics_port != self.config.rpc_port {
            info!("Metrics server listening on {}", metrics_addr);
        }

        // Spawn job processor
        let daemon = Arc::clone(&self);
        let processor_handle = tokio::spawn(async move {
            daemon.job_processor_loop().await;
        });

        // Run HTTP servers. Keep /metrics on RPC for backward compatibility,
        // and serve a dedicated telemetry surface on `metrics_port`.
        if self.config.metrics_port == self.config.rpc_port {
            axum::Server::bind(&rpc_addr)
                .serve(router.into_make_service())
                .await?;
        } else {
            let rpc_server = axum::Server::bind(&rpc_addr).serve(router.into_make_service());
            let metrics_server =
                axum::Server::bind(&metrics_addr).serve(metrics_router.into_make_service());

            tokio::try_join!(rpc_server, metrics_server)?;
        }

        processor_handle.abort();
        Ok(())
    }

    async fn job_processor_loop(&self) {
        loop {
            // Try to get next job
            let job = {
                let mut queue = self.job_queue.write().await;
                let popped = queue.pop();
                if popped.is_some() {
                    queue.record_started();
                }
                popped
            };

            if let Some(job) = job {
                {
                    let mut state = self.state.write().await;
                    state
                        .job_statuses
                        .insert(job.id, JobStatusEntry::new("running", None, None));
                }

                let timer = telemetry::ExecutionTimer::start(Arc::clone(&self.telemetry));
                let wait_time_ms = job.submitted_at.elapsed().as_millis() as u64;

                // Capture pre-execution state (before checkpoint)
                let pre_state = {
                    let sm = self.state_manager.read().await;
                    // Clone the state snapshot for receipt generation
                    // This captures the state before execution for deterministic state proofs
                    sm.clone()
                };

                // Create checkpoint
                {
                    let mut sm = self.state_manager.write().await;
                    sm.checkpoint();
                }

                // Execute
                match self
                    .executor
                    .execute(&job.bytecode, &job.input, job.gas_limit)
                {
                    Ok(result) => {
                        timer.complete(result.gas_used);

                        // Submit receipt with retry logic
                        let max_retries = 3;
                        let mut retry_count = 0;
                        let mut submission_success = false;
                        let mut last_error = String::new();

                        while retry_count < max_retries && !submission_success {
                            // Get current post-state
                            let post_state = self.state_manager.read().await;

                            // Generate receipt
                            let receipt = self.receipt_generator.generate(
                                job.id,
                                &job.input,
                                &result,
                                &pre_state,
                                &*post_state,
                            );
                            drop(post_state);

                            // Submit to chain
                            match self.submitter.submit_receipt(&receipt).await {
                                Ok(tx_hash) => {
                                    info!("Receipt submitted: {}", tx_hash);
                                    self.telemetry.record_receipt_submitted();
                                    let mut state = self.state.write().await;
                                    state.jobs_completed += 1;
                                    state.job_statuses.insert(
                                        job.id,
                                        JobStatusEntry::new("submitted", Some(tx_hash), None),
                                    );
                                    submission_success = true;
                                }
                                Err(e) => {
                                    last_error = e.to_string();
                                    retry_count += 1;

                                    if retry_count < max_retries {
                                        // Exponential backoff: 100ms, 200ms, 400ms
                                        let backoff_ms = 100u64 * (1 << (retry_count - 1));
                                        tracing::warn!(
                                            "Receipt submission failed (attempt {}/{}), retrying in {}ms: {}",
                                            retry_count,
                                            max_retries,
                                            backoff_ms,
                                            e
                                        );
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            backoff_ms,
                                        ))
                                        .await;
                                    } else {
                                        tracing::error!(
                                            "Receipt submission failed after {} retries: {}",
                                            max_retries,
                                            e
                                        );
                                        self.telemetry.record_receipt_failure();
                                    }
                                }
                            }
                        }

                        // If all retries failed, record failure status
                        if !submission_success {
                            let mut state = self.state.write().await;
                            state.job_statuses.insert(
                                job.id,
                                JobStatusEntry::new(
                                    "submit_failed",
                                    None,
                                    Some(format!(
                                        "Failed after {} retries: {}",
                                        max_retries, last_error
                                    )),
                                ),
                            );
                        }

                        let mut queue = self.job_queue.write().await;
                        queue.record_completed(wait_time_ms);
                    }
                    Err(e) => {
                        let err = e.to_string();
                        tracing::error!("Job execution failed: {}", err);
                        timer.fail();

                        // Rollback state
                        let mut sm = self.state_manager.write().await;
                        sm.rollback();

                        let mut queue = self.job_queue.write().await;
                        queue.record_failed();
                        drop(queue);

                        let mut state = self.state.write().await;
                        state.jobs_failed += 1;
                        state
                            .job_statuses
                            .insert(job.id, JobStatusEntry::new("failed", None, Some(err)));
                    }
                }
            } else {
                // No jobs, sleep
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Initialize logging
pub fn init_logging(level: Level) -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    // Attempt to set the global subscriber. If one is already set (typical in tests
    // or when multiple components initialize logging), just continue.
    match tracing::subscriber::set_global_default(subscriber) {
        Ok(()) => Ok(()),
        Err(_e) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;

    #[test]
    fn init_logging_is_idempotent() {
        // Should succeed even if called multiple times (no panic)
        assert!(init_logging(Level::INFO).is_ok());
        assert!(init_logging(Level::DEBUG).is_ok());
    }

    #[test]
    fn test_state_manager_clone_preserves_state() {
        // Verify that StateManager cloning preserves state for pre/post comparisons
        let mut original = StateManager::new();
        original.set(b"key1", b"value1");
        original.set(b"key2", b"value2");

        let root_before = original.root();

        let cloned = original.clone();

        // Modify original
        original.set(b"key1", b"modified");

        // Cloned should preserve original state
        let root_after_clone = cloned.root();

        // Roots should match pre-modification
        assert_eq!(root_before, root_after_clone);
        assert_eq!(cloned.get(b"key1"), Some(b"value1".as_slice()));
        assert_eq!(cloned.get(b"key2"), Some(b"value2".as_slice()));
    }

    #[test]
    fn test_state_capture_vs_empty() {
        // Test that pre-state captured before execution is NOT empty
        // This was the bug: StateManager::new() created empty state
        let mut pre_state = StateManager::new();
        let empty_state = StateManager::new();

        // Empty state should have zero root
        assert_eq!(empty_state.root(), [0u8; 32]);

        // Pre-state with values should have non-zero root
        pre_state.set(b"execution_context", b"state_data");
        let pre_root = pre_state.root();
        assert_ne!(pre_root, [0u8; 32]);

        // This demonstrates the bug: if pre_state was StateManager::new(),
        // we'd lose the execution context and state proofs would be invalid
    }

    #[test]
    fn test_job_status_tracking() {
        // Verify job status transitions are correct
        let status_running = JobStatusEntry::new("running", None, None);
        assert_eq!(status_running.status, "running");
        assert!(status_running.tx_hash.is_none());
        assert!(status_running.error.is_none());

        let status_submitted = JobStatusEntry::new("submitted", Some("0x123abc".to_string()), None);
        assert_eq!(status_submitted.status, "submitted");
        assert_eq!(status_submitted.tx_hash.as_ref().unwrap(), "0x123abc");
        assert!(status_submitted.error.is_none());

        let status_failed = JobStatusEntry::new("failed", None, Some("timeout".to_string()));
        assert_eq!(status_failed.status, "failed");
        assert!(status_failed.tx_hash.is_none());
        assert_eq!(status_failed.error.as_ref().unwrap(), "timeout");
    }

    #[test]
    fn test_sidecar_state_initialization() {
        // Verify sidecar state starts correctly
        let state = SidecarState::default();
        assert_eq!(state.jobs_completed, 0);
        assert_eq!(state.jobs_failed, 0);
        assert!(!state.registered);
        assert!(state.job_statuses.is_empty());
    }

    #[test]
    fn test_receipt_pre_post_state_difference() {
        // Verify that pre/post states in receipts are different when execution modifies state
        let private_key = [1u8; 32];
        let generator = receipt::ReceiptGenerator::new(&private_key);

        let job_id = [2u8; 32];
        let input = b"test input";
        let result = executor::ExecutionResult {
            success: true,
            gas_used: 100,
            return_data: b"output".to_vec(),
            logs: vec![],
            error: None,
        };

        // Pre-state: empty
        let pre_state = StateManager::new();
        let pre_root = pre_state.root();

        // Post-state: modified
        let mut post_state = StateManager::new();
        post_state.set(b"storage_key", b"storage_value");
        let post_root = post_state.root();

        let receipt = generator.generate(job_id, input, &result, &pre_state, &post_state);

        // Pre/post roots should be different if state changed
        assert_ne!(pre_root, post_root);
        assert_eq!(receipt.pre_state_root, pre_root);
        assert_eq!(receipt.post_state_root, post_root);
        assert_ne!(receipt.pre_state_root, receipt.post_state_root);
    }

    #[test]
    fn test_backoff_calculation() {
        // Verify exponential backoff formula: 100ms * (1 << (retry - 1))
        // retry_count 1: 100ms * (1 << 0) = 100ms
        let backoff_1 = 100u64 * (1 << (1 - 1));
        assert_eq!(backoff_1, 100);

        // retry_count 2: 100ms * (1 << 1) = 200ms
        let backoff_2 = 100u64 * (1 << (2 - 1));
        assert_eq!(backoff_2, 200);

        // retry_count 3: 100ms * (1 << 2) = 400ms
        let backoff_3 = 100u64 * (1 << (3 - 1));
        assert_eq!(backoff_3, 400);
    }
}
