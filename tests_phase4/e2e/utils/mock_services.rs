//! Mock Services for E2E Testing
//!
//! Provides mock implementations of external services and APIs
//! to enable isolated E2E testing without dependencies on external systems.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use warp::{http::StatusCode, reply, Filter};

/// Mock external blockchain API responses
#[derive(Debug, Clone)]
pub struct MockExternalChain {
    pub chain_id: u64,
    pub name: String,
    pub base_url: String,
    pub port: u16,
}

/// Mock DNS server for testing
pub struct MockDNSServer {
    pub records: Arc<Mutex<HashMap<String, String>>>,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Mock GPU swarm services
pub struct MockGPUSwarm {
    pub nodes: Arc<Mutex<HashMap<String, MockGPUNode>>>,
    pub coordinator_url: String,
}

/// Mock external price oracles
pub struct MockPriceOracle {
    pub prices: Arc<Mutex<HashMap<String, f64>>>,
    pub update_interval: Duration,
}

/// Mock blockchain explorers
pub struct MockBlockExplorer {
    pub transactions: Arc<Mutex<HashMap<String, MockTransaction>>>,
    pub blocks: Arc<Mutex<HashMap<u64, MockBlock>>>,
}

/// Mock GPU node for testing
#[derive(Debug, Clone)]
pub struct MockGPUNode {
    pub node_id: String,
    pub gpu_type: String,
    pub vram_gb: u64,
    pub available: bool,
    pub tasks_completed: u64,
}

/// Mock transaction for explorer
#[derive(Debug, Clone)]
pub struct MockTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas_used: u64,
    pub block_number: u64,
}

/// Mock block for explorer
#[derive(Debug, Clone)]
pub struct MockBlock {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub transactions: Vec<String>,
}

/// Manages all mock services for E2E testing
pub struct MockServiceManager {
    pub external_chains: HashMap<String, MockExternalChain>,
    pub dns_server: Option<MockDNSServer>,
    pub gpu_swarm: Option<MockGPUSwarm>,
    pub price_oracle: Option<MockPriceOracle>,
    pub block_explorer: Option<MockBlockExplorer>,
    running_services: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl MockServiceManager {
    /// Create a new mock service manager
    pub fn new() -> Self {
        Self {
            external_chains: Self::setup_external_chains(),
            dns_server: None,
            gpu_swarm: None,
            price_oracle: None,
            block_explorer: None,
            running_services: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Setup mock external chains
    fn setup_external_chains() -> HashMap<String, MockExternalChain> {
        let mut chains = HashMap::new();

        chains.insert(
            "avalanche".to_string(),
            MockExternalChain {
                chain_id: 43114,
                name: "Avalanche".to_string(),
                base_url: "http://localhost:9650".to_string(),
                port: 9650,
            },
        );

        chains.insert(
            "bsc".to_string(),
            MockExternalChain {
                chain_id: 56,
                name: "Binance Smart Chain".to_string(),
                base_url: "http://localhost:8545".to_string(),
                port: 8545,
            },
        );

        chains.insert(
            "arbitrum".to_string(),
            MockExternalChain {
                chain_id: 42161,
                name: "Arbitrum".to_string(),
                base_url: "http://localhost:8547".to_string(),
                port: 8547,
            },
        );

        chains.insert(
            "base".to_string(),
            MockExternalChain {
                chain_id: 8453,
                name: "Base".to_string(),
                base_url: "http://localhost:8549".to_string(),
                port: 8549,
            },
        );

        chains
    }

    /// Start all mock services
    pub async fn start_all_services(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting all mock services");

        // Start DNS server
        self.start_mock_dns_server().await?;

        // Start GPU swarm
        self.start_mock_gpu_swarm().await?;

        // Start price oracle
        self.start_mock_price_oracle().await?;

        // Start block explorer
        self.start_mock_block_explorer().await?;

        // Start external chain mocks
        self.start_external_chain_mocks().await?;

        info!("All mock services started successfully");
        Ok(())
    }

    /// Start mock DNS server
    async fn start_mock_dns_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting mock DNS server");

        let records = Arc::new(Mutex::new(HashMap::from([
            ("x3-chain.test".to_string(), "127.0.0.1".to_string()),
            (
                "api.x3-chain.test".to_string(),
                "127.0.0.1:8080".to_string(),
            ),
            (
                "explorer.x3-chain.test".to_string(),
                "127.0.0.1:3000".to_string(),
            ),
        ])));

        // Start DNS server using warp
        let dns_routes =
            warp::path!("resolve" / String)
                .and(warp::get())
                .and_then(move |domain: String| {
                    let records = records.clone();
                    async move {
                        if let Ok(records) = records.lock() {
                            if let Some(ip) = records.get(&domain) {
                                Ok(reply::json(&serde_json::json!({
                                    "domain": domain,
                                    "ip": ip,
                                    "status": "success"
                                })))
                            } else {
                                Ok(reply::json(&serde_json::json!({
                                    "domain": domain,
                                    "status": "not_found"
                                })))
                            }
                        } else {
                            Ok(reply::json(&serde_json::json!({
                                "status": "error"
                            })))
                        }
                    }
                });

        let (_, server) = warp::serve(dns_routes).bind_ephemeral(([127, 0, 0, 1], 5353));

        let server_handle = tokio::spawn(server);

        self.dns_server = Some(MockDNSServer {
            records: records.clone(),
            server_handle: Some(server_handle),
        });

        // Wait for server to start
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Start mock GPU swarm
    async fn start_mock_gpu_swarm(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting mock GPU swarm");

        let nodes = Arc::new(Mutex::new(HashMap::new()));

        // Add mock GPU nodes
        let mut node_map = nodes.lock().unwrap();
        node_map.insert(
            "gpu-node-1".to_string(),
            MockGPUNode {
                node_id: "gpu-node-1".to_string(),
                gpu_type: "RTX 4090".to_string(),
                vram_gb: 24,
                available: true,
                tasks_completed: 0,
            },
        );

        node_map.insert(
            "gpu-node-2".to_string(),
            MockGPUNode {
                node_id: "gpu-node-2".to_string(),
                gpu_type: "RTX 4080".to_string(),
                vram_gb: 16,
                available: true,
                tasks_completed: 0,
            },
        );

        // Start swarm coordinator API
        let swarm_routes = warp::path!("swarm" / "nodes")
            .and(warp::get())
            .and_then(move || {
                let nodes = nodes.clone();
                async move {
                    if let Ok(nodes) = nodes.lock() {
                        let node_list: Vec<_> = nodes.values().cloned().collect();
                        Ok(reply::json(&node_list))
                    } else {
                        Ok(reply::json(&Vec::<MockGPUNode>::new()))
                    }
                }
            });

        let (_, server) = warp::serve(swarm_routes).bind_ephemeral(([127, 0, 0, 1], 8080));

        let server_handle = tokio::spawn(server);

        self.gpu_swarm = Some(MockGPUSwarm {
            nodes: nodes.clone(),
            coordinator_url: "http://127.0.0.1:8080".to_string(),
            server_handle: Some(server_handle),
        });

        sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Start mock price oracle
    async fn start_mock_price_oracle(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting mock price oracle");

        let prices = Arc::new(Mutex::new(HashMap::from([
            ("ETH".to_string(), 2000.0),
            ("BTC".to_string(), 45000.0),
            ("USDC".to_string(), 1.0),
            ("X3".to_string(), 0.5),
        ])));

        // Start price update loop
        let update_prices = prices.clone();
        let price_update_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                if let Ok(mut prices) = update_prices.lock() {
                    // Simulate price movements
                    for price in prices.values_mut() {
                        *price *= 1.0 + (rand::random::<f64>() - 0.5) * 0.02; // ±1% movement
                    }
                }
            }
        });

        // Start price API
        let price_routes =
            warp::path!("price" / String)
                .and(warp::get())
                .and_then(move |symbol: String| {
                    let prices = prices.clone();
                    async move {
                        if let Ok(prices) = prices.lock() {
                            if let Some(&price) = prices.get(&symbol) {
                                Ok(reply::json(&serde_json::json!({
                                    "symbol": symbol,
                                    "price": price,
                                    "timestamp": std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs()
                                })))
                            } else {
                                Ok(reply::json(&serde_json::json!({
                                    "symbol": symbol,
                                    "status": "not_found"
                                })))
                            }
                        } else {
                            Ok(reply::json(&serde_json::json!({
                                "status": "error"
                            })))
                        }
                    }
                });

        let (_, server) = warp::serve(price_routes).bind_ephemeral(([127, 0, 0, 1], 8090));

        let server_handle = tokio::spawn(server);

        self.price_oracle = Some(MockPriceOracle {
            prices: prices.clone(),
            update_interval: Duration::from_secs(5),
            server_handle: Some(server_handle),
        });

        // Add to running services
        self.running_services
            .lock()
            .unwrap()
            .push(price_update_handle);

        sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Start mock block explorer
    async fn start_mock_block_explorer(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting mock block explorer");

        let transactions = Arc::new(Mutex::new(HashMap::new()));
        let blocks = Arc::new(Mutex::new(HashMap::new()));

        // Add some mock data
        let mut block_map = blocks.lock().unwrap();
        block_map.insert(
            1,
            MockBlock {
                number: 1,
                hash: "0x1234567890abcdef".to_string(),
                parent_hash: "0x0000000000000000".to_string(),
                timestamp: 1640995200,
                transactions: vec!["0xtx123".to_string()],
            },
        );

        let mut tx_map = transactions.lock().unwrap();
        tx_map.insert(
            "0xtx123".to_string(),
            MockTransaction {
                hash: "0xtx123".to_string(),
                from: "0xabc123".to_string(),
                to: "0xdef456".to_string(),
                value: "1000000000000000000".to_string(), // 1 ETH
                gas_used: 21000,
                block_number: 1,
            },
        );

        // Start explorer API
        let explorer_routes = warp::path!("tx" / String)
            .and(warp::get())
            .and_then(move |tx_hash: String| {
                let transactions = transactions.clone();
                async move {
                    if let Ok(transactions) = transactions.lock() {
                        if let Some(tx) = transactions.get(&tx_hash) {
                            Ok(reply::json(tx))
                        } else {
                            Ok(reply::with_status(
                                reply::json(&serde_json::json!({"status": "not_found"})),
                                StatusCode::NOT_FOUND,
                            ))
                        }
                    } else {
                        Ok(reply::with_status(
                            reply::json(&serde_json::json!({"status": "error"})),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        ))
                    }
                }
            })
            .or(warp::path!("block" / u64)
                .and(warp::get())
                .and_then(move |block_num: u64| {
                    let blocks = blocks.clone();
                    async move {
                        if let Ok(blocks) = blocks.lock() {
                            if let Some(block) = blocks.get(&block_num) {
                                Ok(reply::json(block))
                            } else {
                                Ok(reply::with_status(
                                    reply::json(&serde_json::json!({"status": "not_found"})),
                                    StatusCode::NOT_FOUND,
                                ))
                            }
                        } else {
                            Ok(reply::with_status(
                                reply::json(&serde_json::json!({"status": "error"})),
                                StatusCode::INTERNAL_SERVER_ERROR,
                            ))
                        }
                    }
                }));

        let (_, server) = warp::serve(explorer_routes).bind_ephemeral(([127, 0, 0, 1], 3000));

        let server_handle = tokio::spawn(server);

        self.block_explorer = Some(MockBlockExplorer {
            transactions: transactions.clone(),
            blocks: blocks.clone(),
            server_handle: Some(server_handle),
        });

        sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Start external chain mocks
    async fn start_external_chain_mocks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting external chain mocks");

        for (name, chain) in &self.external_chains {
            info!("Mocking {} at {}", chain.name, chain.base_url);

            // Start mock RPC endpoints for each chain
            let chain_rpc_routes =
                warp::path!("rpc")
                    .and(warp::post())
                    .and_then(move || async move {
                        Ok(reply::json(&serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "result": "0x1234567890abcdef"
                        })))
                    });

            let (_, server) =
                warp::serve(chain_rpc_routes).bind_ephemeral(([127, 0, 0, 1], chain.port));

            let server_handle = tokio::spawn(server);
            self.running_services.lock().unwrap().push(server_handle);
        }

        Ok(())
    }

    /// Stop all mock services
    pub async fn stop_all_services(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping all mock services");

        // Stop DNS server
        if let Some(dns_server) = self.dns_server.take() {
            if let Some(handle) = dns_server.server_handle {
                handle.abort();
            }
        }

        // Stop GPU swarm
        if let Some(gpu_swarm) = self.gpu_swarm.take() {
            if let Some(handle) = gpu_swarm.server_handle {
                handle.abort();
            }
        }

        // Stop price oracle
        if let Some(price_oracle) = self.price_oracle.take() {
            if let Some(handle) = price_oracle.server_handle {
                handle.abort();
            }
        }

        // Stop block explorer
        if let Some(block_explorer) = self.block_explorer.take() {
            if let Some(handle) = block_explorer.server_handle {
                handle.abort();
            }
        }

        // Stop all running services
        let mut handles = self.running_services.lock().unwrap();
        for handle in handles.drain(..) {
            handle.abort();
        }

        info!("All mock services stopped");
        Ok(())
    }

    /// Get DNS server URL
    pub fn get_dns_url(&self) -> String {
        "http://127.0.0.1:5353".to_string()
    }

    /// Get GPU swarm coordinator URL
    pub fn get_gpu_swarm_url(&self) -> String {
        "http://127.0.0.1:8080".to_string()
    }

    /// Get price oracle URL
    pub fn get_price_oracle_url(&self) -> String {
        "http://127.0.0.1:8090".to_string()
    }

    /// Get block explorer URL
    pub fn get_block_explorer_url(&self) -> String {
        "http://127.0.0.1:3000".to_string()
    }

    /// Get external chain URL
    pub fn get_external_chain_url(&self, chain_name: &str) -> Option<String> {
        self.external_chains
            .get(chain_name)
            .map(|chain| chain.base_url.clone())
    }
}
