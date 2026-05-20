//! RPC Client Configuration for External Chains
//!
//! This module provides unified RPC endpoint management across all supported chains,
//! with fallback chains, load balancing, and automatic retry logic.

use serde::{Deserialize, Serialize};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

/// RPC endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpoint {
    pub url: String,
    pub priority: u8,
    pub timeout_ms: u32,
    pub max_retries: u32,
}

impl RpcEndpoint {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            priority: 50,
            timeout_ms: 30000,
            max_retries: 3,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// WebSocket endpoint for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEndpoint {
    pub url: String,
    pub max_subscriptions: u32,
    pub ping_interval_ms: u32,
}

impl WsEndpoint {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_subscriptions: 100,
            ping_interval_ms: 30000,
        }
    }
}

/// Flash loan provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanProvider {
    pub name: String,
    pub contract_address: String,
    pub fee_bps: u32,        // Fee in basis points (100 = 1%)
    pub max_liquidity: u128, // Wei/token units
    pub enabled: bool,
}

impl FlashLoanProvider {
    pub fn new(name: impl Into<String>, contract: impl Into<String>, fee_bps: u32) -> Self {
        Self {
            name: name.into(),
            contract_address: contract.into(),
            fee_bps,
            max_liquidity: u128::MAX,
            enabled: true,
        }
    }

    pub fn with_liquidity(mut self, max_liquidity: u128) -> Self {
        self.max_liquidity = max_liquidity;
        self
    }
}

/// DEX Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexRouter {
    pub name: String,
    pub protocol: String,
    pub router_address: String,
    pub factory_address: Option<String>,
    pub enabled: bool,
}

impl DexRouter {
    pub fn new(
        name: impl Into<String>,
        protocol: impl Into<String>,
        router: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            protocol: protocol.into(),
            router_address: router.into(),
            factory_address: None,
            enabled: true,
        }
    }

    pub fn with_factory(mut self, factory: impl Into<String>) -> Self {
        self.factory_address = Some(factory.into());
        self
    }
}

/// Per-chain RPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainRpcConfig {
    pub chain_id: u64,
    pub chain_name: String,
    pub rpc_endpoints: Vec<RpcEndpoint>,
    pub ws_endpoints: Vec<WsEndpoint>,
    pub flashloan_providers: Vec<FlashLoanProvider>,
    pub dex_routers: Vec<DexRouter>,
    pub block_explorer_url: String,
    pub average_block_time_ms: u32,
    pub finality_depth: u32,
}

impl ChainRpcConfig {
    pub fn new(chain_id: u64, chain_name: impl Into<String>) -> Self {
        Self {
            chain_id,
            chain_name: chain_name.into(),
            rpc_endpoints: Vec::new(),
            ws_endpoints: Vec::new(),
            flashloan_providers: Vec::new(),
            dex_routers: Vec::new(),
            block_explorer_url: String::new(),
            average_block_time_ms: 2000,
            finality_depth: 12,
        }
    }

    pub fn add_rpc(mut self, endpoint: RpcEndpoint) -> Self {
        self.rpc_endpoints.push(endpoint);
        self
    }

    pub fn add_ws(mut self, endpoint: WsEndpoint) -> Self {
        self.ws_endpoints.push(endpoint);
        self
    }

    pub fn add_flashloan(mut self, provider: FlashLoanProvider) -> Self {
        self.flashloan_providers.push(provider);
        self
    }

    pub fn add_dex(mut self, router: DexRouter) -> Self {
        self.dex_routers.push(router);
        self
    }

    pub fn with_explorer(mut self, url: impl Into<String>) -> Self {
        self.block_explorer_url = url.into();
        self
    }

    pub fn with_block_time(mut self, ms: u32) -> Self {
        self.average_block_time_ms = ms;
        self
    }

    pub fn with_finality(mut self, depth: u32) -> Self {
        self.finality_depth = depth;
        self
    }

    /// Get primary RPC endpoint (highest priority)
    pub fn primary_rpc(&self) -> Option<&RpcEndpoint> {
        self.rpc_endpoints.iter().max_by_key(|e| e.priority)
    }

    /// Get all enabled flashloan providers
    pub fn enabled_flashloans(&self) -> Vec<&FlashLoanProvider> {
        self.flashloan_providers
            .iter()
            .filter(|p| p.enabled)
            .collect()
    }

    /// Get all enabled DEX routers
    pub fn enabled_dexes(&self) -> Vec<&DexRouter> {
        self.dex_routers.iter().filter(|r| r.enabled).collect()
    }
}

/// Global RPC registry for all chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRegistry {
    chains: BTreeMap<u64, ChainRpcConfig>,
}

impl RpcRegistry {
    pub fn new() -> Self {
        Self {
            chains: BTreeMap::new(),
        }
    }

    pub fn register(mut self, config: ChainRpcConfig) -> Self {
        self.chains.insert(config.chain_id, config);
        self
    }

    pub fn get(&self, chain_id: u64) -> Option<&ChainRpcConfig> {
        self.chains.get(&chain_id)
    }

    pub fn all_chains(&self) -> Vec<&ChainRpcConfig> {
        self.chains.values().collect()
    }

    pub fn supported_chain_ids(&self) -> Vec<u64> {
        self.chains.keys().cloned().collect()
    }
}

impl Default for RpcRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-configured Arbitrum mainnet RPC setup
pub fn arbitrum_mainnet_config() -> ChainRpcConfig {
    ChainRpcConfig::new(42161, "Arbitrum One")
        // Primary RPC endpoints (from your config)
        .add_rpc(RpcEndpoint::new("https://arb-mainnet.g.alchemy.com/v2/Fe5T2pGsX76ml9kDCwVRZhtmkdixfrDQ")
            .with_priority(100)
            .with_timeout(25000))
        .add_rpc(RpcEndpoint::new("https://lb.drpc.org/arbitrum/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE")
            .with_priority(90))
        .add_rpc(RpcEndpoint::new("https://rpc.ankr.com/arbitrum/648269110992d35fb12b490f3e9d00e18141ad9212081909344f15ec1c342a3c")
            .with_priority(80))
        .add_rpc(RpcEndpoint::new("https://arb1.arbitrum.io/rpc")
            .with_priority(70))
        // WebSocket endpoints
        .add_ws(WsEndpoint::new("wss://lb.drpc.org/arbitrum/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE"))
        .add_ws(WsEndpoint::new("wss://rpc.ankr.com/arbitrum/ws/648269110992d35fb12b490f3e9d00e18141ad9212081909344f15ec1c342a3c"))
        // Flash loan providers
        .add_flashloan(FlashLoanProvider::new(
            "Aave V3",
            "0x794a61358D6845594F94dc1DB02A252b5b4814aD",
            3,
        ).with_liquidity(100_000_000 * 10u128.pow(18)))
        .add_flashloan(FlashLoanProvider::new(
            "Balancer V2",
            "0xBA12222222228d8Ba445958a75a0704d566BF2C8",
            5,
        ).with_liquidity(250_000_000 * 10u128.pow(18)))
        .add_flashloan(FlashLoanProvider::new(
            "Radiant Capital",
            "0xF4B1486DD74D07706052A33d31d7c0AAFD0659E1",
            4,
        ).with_liquidity(50_000_000 * 10u128.pow(18)))
        .add_flashloan(FlashLoanProvider::new(
            "dForce",
            "0x0988f3C0cFd7F0326475fA7fDa7F64e0663B70F0",
            5,
        ).with_liquidity(40_000_000 * 10u128.pow(18)))
        // DEX Routers
        .add_dex(DexRouter::new("Uniswap V3", "AMM", "0xE592427A0AEce92De3Edee1F18E0157C05861564")
            .with_factory("0x1F98431c8aD98523631AE4a59f267346ea31F984"))
        .add_dex(DexRouter::new("SushiSwap V2", "AMM", "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"))
        .add_dex(DexRouter::new("Camelot V2", "AMM", "0xc873fEcbd354f5A56E00E710B90EF4201db2448d")
            .with_factory("0x6EcCab422D763aC031210895C81787E87B43A652"))
        .add_dex(DexRouter::new("Trader Joe V2", "AMM", "0xb4315eDB925C2c89bFdE53d243b4db61b5D0a4e2")
            .with_factory("0x8e42f2F4101563bF679975178e880FD87d3eFd4e"))
        .add_dex(DexRouter::new("Ramses V2", "AMM", "0xAAA87963EFeB6f7E0a2711F397663105Acb1805e")
            .with_factory("0xAAA20D08e59F6561f242b08513D36266C5A29415"))
        .add_dex(DexRouter::new("Chronos", "AMM", "0xE708aa9E887980750C040a6A2Cb901c37aA63434")
            .with_factory("0xCEFb89f8103fC792B03C15d8c722a48A5C049660"))
        .with_explorer("https://arbiscan.io")
        .with_block_time(1000)  // Arbitrum L2: ~1s
        .with_finality(1) // Arbitrum finalizes quickly
}

/// Create a default RPC registry with all supported chains
pub fn create_default_registry() -> RpcRegistry {
    RpcRegistry::new()
        .register(arbitrum_mainnet_config())
        .register(
            ChainRpcConfig::new(8453, "Base Mainnet")
                .add_rpc(RpcEndpoint::new("https://mainnet.base.org").with_priority(100))
                .add_rpc(
                    RpcEndpoint::new(
                        "https://base-mainnet.g.alchemy.com/v2/Fe5T2pGsX76ml9kDCwVRZhtmkdixfrDQ",
                    )
                    .with_priority(90),
                )
                .with_explorer("https://basescan.org")
                .with_block_time(2000),
        )
        .register(
            ChainRpcConfig::new(137, "Polygon Mainnet")
                .add_rpc(RpcEndpoint::new("https://polygon-rpc.com").with_priority(100))
                .with_explorer("https://polygonscan.com")
                .with_block_time(2000)
                .with_finality(128),
        )
        .register(
            ChainRpcConfig::new(43114, "Avalanche C-Chain")
                .add_rpc(
                    RpcEndpoint::new("https://api.avax.network/ext/bc/C/rpc").with_priority(100),
                )
                .with_explorer("https://snowtrace.io")
                .with_block_time(2000)
                .with_finality(1),
        )
        .register(
            ChainRpcConfig::new(56, "BNB Smart Chain")
                .add_rpc(RpcEndpoint::new("https://bsc-dataseed.binance.org").with_priority(100))
                .with_explorer("https://bscscan.com")
                .with_block_time(3000)
                .with_finality(1),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_endpoint_builder() {
        let endpoint = RpcEndpoint::new("https://example.com")
            .with_priority(100)
            .with_timeout(15000)
            .with_retries(5);

        assert_eq!(endpoint.priority, 100);
        assert_eq!(endpoint.timeout_ms, 15000);
        assert_eq!(endpoint.max_retries, 5);
    }

    #[test]
    fn test_chain_rpc_config() {
        let config = ChainRpcConfig::new(42161, "Arbitrum One")
            .add_rpc(RpcEndpoint::new("https://rpc1.com").with_priority(100))
            .add_rpc(RpcEndpoint::new("https://rpc2.com").with_priority(50));

        assert_eq!(config.chain_id, 42161);
        assert_eq!(config.rpc_endpoints.len(), 2);
        assert_eq!(config.primary_rpc().unwrap().priority, 100);
    }

    #[test]
    fn test_flashloan_provider() {
        let provider = FlashLoanProvider::new("Aave V3", "0x123", 5).with_liquidity(1_000_000);

        assert_eq!(provider.name, "Aave V3");
        assert_eq!(provider.fee_bps, 5);
        assert_eq!(provider.max_liquidity, 1_000_000);
        assert!(provider.enabled);
    }

    #[test]
    fn test_arbitrum_config() {
        let config = arbitrum_mainnet_config();

        assert_eq!(config.chain_id, 42161);
        assert!(!config.rpc_endpoints.is_empty());
        assert!(!config.flashloan_providers.is_empty());
        assert!(!config.dex_routers.is_empty());
        assert_eq!(config.average_block_time_ms, 1000);
        assert_eq!(config.finality_depth, 1);
    }

    #[test]
    fn test_rpc_registry() {
        let registry = create_default_registry();
        let arbitrum = registry.get(42161);

        assert!(arbitrum.is_some());
        assert_eq!(arbitrum.unwrap().chain_id, 42161);

        let all = registry.all_chains();
        assert!(all.len() >= 5);
    }
}
