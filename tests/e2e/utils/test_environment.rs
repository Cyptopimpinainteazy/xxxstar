//! Test Environment Management
//!
//! Provides utilities for managing test environments including
//! blockchain node setup, database configuration, and service orchestration.

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::{TestConfig, TestResult};

/// Test environment state
#[derive(Debug, Clone)]
pub struct EnvironmentState {
    pub blockchain_running: bool,
    pub gpu_swarm_running: bool,
    pub dns_server_running: bool,
    pub external_apis_mocked: bool,
    pub test_data_initialized: bool,
}

/// Manages the test environment lifecycle
pub struct TestEnvironment {
    config: TestConfig,
    state: Arc<Mutex<EnvironmentState>>,
    processes: Arc<Mutex<Vec<std::process::Child>>>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub async fn new(config: TestConfig) -> TestResult<Self> {
        info!("Initializing test environment");

        let env = Self {
            config: config.clone(),
            state: Arc::new(Mutex::new(EnvironmentState {
                blockchain_running: false,
                gpu_swarm_running: false,
                dns_server_running: false,
                external_apis_mocked: false,
                test_data_initialized: false,
            })),
            processes: Arc::new(Mutex::new(Vec::new())),
        };

        // Start blockchain node
        env.start_blockchain_node().await?;

        // Initialize test data
        env.initialize_test_data().await?;

        Ok(env)
    }

    /// Start the blockchain node
    async fn start_blockchain_node(&self) -> TestResult<()> {
        info!("Starting blockchain node");

        let mut child = Command::new("./target/release/x3-chain")
            .args(&[
                "--dev",
                "--rpc-port",
                "9933",
                "--ws-port",
                "9944",
                "--port",
                "30333",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start blockchain node: {}", e))?;

        // Wait for node to start
        sleep(Duration::from_secs(10)).await;

        // Check if node is responding
        let client = reqwest::Client::new();
        for i in 0..30 {
            match client
                .get(&format!("{}/health", self.config.rpc_url))
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    info!("Blockchain node started successfully");
                    self.update_state(|state| state.blockchain_running = true);
                    self.add_process(child);
                    return Ok(());
                }
                _ => {
                    if i == 29 {
                        return Err("Blockchain node failed to start".into());
                    }
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Ok(())
    }

    /// Initialize test data and accounts
    async fn initialize_test_data(&self) -> TestResult<()> {
        info!("Initializing test data");

        // Create test accounts
        self.create_test_accounts().await?;

        // Deploy base contracts
        self.deploy_base_contracts().await?;

        self.update_state(|state| state.test_data_initialized = true);
        Ok(())
    }

    /// Create test accounts with funded balances
    async fn create_test_accounts(&self) -> TestResult<()> {
        info!("Creating test accounts");

        // Fund test accounts using RPC calls
        let client = reqwest::Client::new();

        let accounts = vec![
            "test_user_1",
            "test_user_2",
            "test_lender",
            "test_borrower",
            "test_gpu_miner",
            "test_ai_trader",
        ];

        for account in accounts {
            let response = client
                .post(&format!("{}/rpc", self.config.rpc_url))
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "dev_newAccount",
                    "params": [account],
                    "id": 1
                }))
                .send()
                .await?;

            if !response.status().is_success() {
                warn!("Failed to create test account: {}", account);
            }
        }

        Ok(())
    }

    /// Deploy base smart contracts
    async fn deploy_base_contracts(&self) -> TestResult<()> {
        info!("Deploying base smart contracts");

        // Deploy lending protocol contracts
        self.deploy_lending_contracts().await?;

        // Deploy AI swarm contracts
        self.deploy_ai_swarm_contracts().await?;

        // Deploy evolution contracts
        self.deploy_evolution_contracts().await?;

        Ok(())
    }

    async fn deploy_lending_contracts(&self) -> TestResult<()> {
        info!("Deploying lending contracts");

        // This would typically involve:
        // 1. Compiling contracts
        // 2. Deploying via Foundry/script
        // 3. Verifying deployment

        sleep(Duration::from_secs(5)).await; // Simulate deployment time
        Ok(())
    }
    /// Get state root for latest or specific block
    pub async fn get_state_root(&self, block_hash: Option<&str>) -> TestResult<String> {
        let client = reqwest::Client::new();
        let params = if let Some(hash) = block_hash {
            serde_json::json!([hash])
        } else {
            serde_json::json!([])
        };

        let resp = client
            .post(&format!("{}/rpc", self.config.rpc_url))
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "chain_getHeader",
                "params": params
            }))
            .send()
            .await
            .map_err(|e| format!("RPC error: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        let header = body
            .get("result")
            .and_then(|r| r.get("header"))
            .ok_or("Missing header in RPC response")?;
        let state_root = header
            .get("stateRoot")
            .and_then(|s| s.as_str())
            .ok_or("Missing stateRoot in header")?;
        Ok(state_root.to_string())
    }

    /// Export extrinsics for a range of blocks (inclusive). Returns vector of extrinsic hex strings in order by block then by index.
    pub async fn export_extrinsics(&self, from: u64, to: u64) -> TestResult<Vec<String>> {
        let client = reqwest::Client::new();
        let mut extrinsics: Vec<String> = Vec::new();

        for block_num in from..=to {
            // Get block hash
            let resp = client
                .post(&format!("{}/rpc", self.config.rpc_url))
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "chain_getBlockHash",
                    "params": [format!("0x{:x}", block_num)]
                }))
                .send()
                .await
                .map_err(|e| format!("RPC error: {}", e))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Invalid JSON: {}", e))?;
            let block_hash = body
                .get("result")
                .and_then(|r| r.as_str())
                .ok_or("Missing block hash")?
                .to_string();

            // Fetch block
            let resp = client
                .post(&format!("{}/rpc", self.config.rpc_url))
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "chain_getBlock",
                    "params": [block_hash]
                }))
                .send()
                .await
                .map_err(|e| format!("RPC error: {}", e))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Invalid JSON: {}", e))?;
            if let Some(exs) = body
                .get("result")
                .and_then(|r| r.get("block"))
                .and_then(|b| b.get("extrinsics"))
            {
                if let Some(arr) = exs.as_array() {
                    for ex in arr {
                        if let Some(hex) = ex.as_str() {
                            extrinsics.push(hex.to_string());
                        }
                    }
                }
            }
        }

        Ok(extrinsics)
    }

    /// Submit a list of signed extrinsic hex strings to a target RPC endpoint.
    pub async fn submit_extrinsics_to_rpc(
        &self,
        target_rpc: &str,
        extrinsics: &[String],
    ) -> TestResult<()> {
        let client = reqwest::Client::new();
        for ex in extrinsics {
            let resp = client
                .post(&format!("{}/rpc", target_rpc))
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "author_submitExtrinsic",
                    "params": [ex]
                }))
                .send()
                .await
                .map_err(|e| format!("RPC error submit_extrinsic: {}", e))?;

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Invalid JSON: {}", e))?;
            if body.get("error").is_some() {
                return Err(format!("Extrinsic submission error: {:?}", body).into());
            }
        }
        Ok(())
    }
    async fn deploy_ai_swarm_contracts(&self) -> TestResult<()> {
        info!("Deploying AI swarm contracts");

        sleep(Duration::from_secs(5)).await; // Simulate deployment time
        Ok(())
    }

    async fn deploy_evolution_contracts(&self) -> TestResult<()> {
        info!("Deploying evolution contracts");

        sleep(Duration::from_secs(5)).await; // Simulate deployment time
        Ok(())
    }

    /// Start GPU swarm network
    pub async fn start_gpu_swarm(&self) -> TestResult<()> {
        info!("Starting GPU swarm network");

        // Start coordinator
        let coordinator = Command::new("./target/release/swarm-coordinator")
            .args(&["--config", "./config/coordinator-config.toml"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start GPU coordinator: {}", e))?;

        // Start test nodes
        for i in 0..3 {
            let node = Command::new("./target/release/swarm-node")
                .args(&[
                    "--id",
                    &format!("test_node_{}", i),
                    "--coordinator",
                    "localhost:8080",
                    "--config",
                    "./config/node-config.toml",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start GPU node {}: {}", i, e))?;

            self.add_process(node);
        }

        self.add_process(coordinator);
        sleep(Duration::from_secs(5)).await; // Wait for swarm to initialize

        self.update_state(|state| state.gpu_swarm_running = true);
        Ok(())
    }

    /// Start DNS server
    pub async fn start_dns_server(&self) -> TestResult<()> {
        info!("Starting DNS server");

        let server = Command::new("./target/release/x3-dns-server")
            .args(&["--config", "./config/dns-test.toml"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start DNS server: {}", e))?;

        sleep(Duration::from_secs(3)).await; // Wait for server to start

        self.add_process(server);
        self.update_state(|state| state.dns_server_running = true);
        Ok(())
    }

    /// Setup external API mocks
    pub async fn setup_external_mocks(&self) -> TestResult<()> {
        info!("Setting up external API mocks");

        // Mock external blockchain APIs (Avalanche, BSC, etc.)
        self.mock_external_chains().await?;

        self.update_state(|state| state.external_apis_mocked = true);
        Ok(())
    }

    async fn mock_external_chains(&self) -> TestResult<()> {
        info!("Mocking external chain APIs");

        // Start mock servers for external chains
        // This would typically involve starting mock HTTP servers
        // that respond to chain API calls

        Ok(())
    }

    /// Update environment state
    fn update_state<F>(&self, updater: F)
    where
        F: FnOnce(&mut EnvironmentState),
    {
        if let Ok(mut state) = self.state.lock() {
            updater(&mut state);
        }
    }

    /// Add process to management
    fn add_process(&self, mut process: std::process::Child) {
        if let Ok(mut processes) = self.processes.lock() {
            processes.push(process);
        }
    }

    /// Get current environment state
    pub fn get_state(&self) -> EnvironmentState {
        self.state.lock().unwrap().clone()
    }

    /// Check if environment is ready for tests
    pub fn is_ready(&self) -> bool {
        let state = self.get_state();
        state.blockchain_running && state.test_data_initialized
    }

    /// Cleanup environment
    pub async fn cleanup(&self) -> TestResult<()> {
        info!("Cleaning up test environment");

        // Stop all managed processes
        if let Ok(mut processes) = self.processes.lock() {
            for mut process in processes.drain(..) {
                let _ = process.kill();
                let _ = process.wait();
            }
        }

        // Reset state
        self.update_state(|state| {
            state.blockchain_running = false;
            state.gpu_swarm_running = false;
            state.dns_server_running = false;
            state.external_apis_mocked = false;
            state.test_data_initialized = false;
        });

        Ok(())
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Ensure cleanup on drop
        let _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.cleanup());
    }
}
