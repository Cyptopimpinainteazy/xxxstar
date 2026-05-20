//! Environment Variable Configuration for RPC and Chain Settings
//!
//! Loads configuration from `.env` or environment variables for:
//! - Network selection (Arbitrum, Base, Polygon, etc.)
//! - RPC endpoints and API keys
//! - Wallet configuration
//! - Flash loan and DEX provider credentials

use serde::{Deserialize, Serialize};
use std::collections::btree_map::BTreeMap;

/// Network environment enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum NetworkEnv {
    #[serde(rename = "arbitrum")]
    Arbitrum,
    #[serde(rename = "base")]
    Base,
    #[serde(rename = "polygon")]
    Polygon,
    #[serde(rename = "avalanche")]
    Avalanche,
    #[serde(rename = "bsc")]
    Bsc,
}

impl NetworkEnv {
    /// Parse from environment string
    pub fn from_env(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "arbitrum" => Some(NetworkEnv::Arbitrum),
            "base" => Some(NetworkEnv::Base),
            "polygon" => Some(NetworkEnv::Polygon),
            "avalanche" => Some(NetworkEnv::Avalanche),
            "bsc" => Some(NetworkEnv::Bsc),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkEnv::Arbitrum => "arbitrum",
            NetworkEnv::Base => "base",
            NetworkEnv::Polygon => "polygon",
            NetworkEnv::Avalanche => "avalanche",
            NetworkEnv::Bsc => "bsc",
        }
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            NetworkEnv::Arbitrum => 42161,
            NetworkEnv::Base => 8453,
            NetworkEnv::Polygon => 137,
            NetworkEnv::Avalanche => 43114,
            NetworkEnv::Bsc => 56,
        }
    }
}

/// API Provider credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentials {
    pub alchemy_api_key: Option<String>,
    pub drpc_api_key: Option<String>,
    pub ankr_api_key: Option<String>,
}

impl ProviderCredentials {
    pub fn new() -> Self {
        Self {
            alchemy_api_key: None,
            drpc_api_key: None,
            ankr_api_key: None,
        }
    }

    pub fn with_alchemy(mut self, key: String) -> Self {
        self.alchemy_api_key = Some(key);
        self
    }

    pub fn with_drpc(mut self, key: String) -> Self {
        self.drpc_api_key = Some(key);
        self
    }

    pub fn with_ankr(mut self, key: String) -> Self {
        self.ankr_api_key = Some(key);
        self
    }
}

impl Default for ProviderCredentials {
    fn default() -> Self {
        Self::new()
    }
}

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub private_key: String,
    pub address: String,
}

impl WalletConfig {
    pub fn new(private_key: String, address: String) -> Self {
        Self {
            private_key,
            address,
        }
    }
}

/// MEV/Flash loan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanConfig {
    pub enabled: bool,
    pub max_slippage_bps: u32,
    pub min_profit_threshold: u64,
    pub check_interval_ms: u64,
}

impl Default for FlashLoanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_slippage_bps: 100,       // 1%
            min_profit_threshold: 50000, // 0.5 USD in wei
            check_interval_ms: 15000,    // 15s
        }
    }
}

/// Bot/Arbitrage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub port: u16,
    pub check_interval_ms: u64,
    pub max_slippage_percent: f64,
    pub min_profit_threshold: f64,
    pub max_gas_price_gwei: u64,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            port: 3001,
            check_interval_ms: 15000,
            max_slippage_percent: 1.0,
            min_profit_threshold: 0.5,
            max_gas_price_gwei: 100,
        }
    }
}

/// Complete environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub network: NetworkEnv,
    pub rpc_urls: BTreeMap<String, Vec<String>>,
    pub provider_credentials: ProviderCredentials,
    pub wallet: Option<WalletConfig>,
    pub flashloan: FlashLoanConfig,
    pub bot: BotConfig,
    pub api_keys: BTreeMap<String, String>,
}

impl EnvConfig {
    /// Create a new default configuration
    pub fn new(network: NetworkEnv) -> Self {
        let mut rpc_urls = BTreeMap::new();

        // Default RPC URLs based on network
        match network {
            NetworkEnv::Arbitrum => {
                rpc_urls.insert(
                    "primary".to_string(),
                    vec![
                        "https://arb-mainnet.g.alchemy.com/v2/Fe5T2pGsX76ml9kDCwVRZhtmkdixfrDQ"
                            .to_string(),
                    ],
                );
                rpc_urls.insert("fallback".to_string(), vec![
                    "https://lb.drpc.org/arbitrum/ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE".to_string(),
                    "https://rpc.ankr.com/arbitrum/648269110992d35fb12b490f3e9d00e18141ad9212081909344f15ec1c342a3c".to_string(),
                    "https://arb1.arbitrum.io/rpc".to_string(),
                ]);
            }
            NetworkEnv::Base => {
                rpc_urls.insert(
                    "primary".to_string(),
                    vec![
                        "https://base-mainnet.g.alchemy.com/v2/Fe5T2pGsX76ml9kDCwVRZhtmkdixfrDQ"
                            .to_string(),
                    ],
                );
                rpc_urls.insert(
                    "fallback".to_string(),
                    vec!["https://mainnet.base.org".to_string()],
                );
            }
            _ => {}
        }

        Self {
            network,
            rpc_urls,
            provider_credentials: ProviderCredentials::new(),
            wallet: None,
            flashloan: FlashLoanConfig::default(),
            bot: BotConfig::default(),
            api_keys: BTreeMap::new(),
        }
    }

    /// Load configuration from environment (for wasm and no_std compatible subset)
    pub fn from_env() -> Self {
        // Default to Arbitrum
        let network = NetworkEnv::Arbitrum;
        let mut config = Self::new(network);

        // Load credentials if available
        config.provider_credentials = ProviderCredentials::new()
            .with_alchemy("Fe5T2pGsX76ml9kDCwVRZhtmkdixfrDQ".to_string())
            .with_drpc("ArgUBy0RzURpos-Jlz1TqLRxbgscV2AR8JXZrqRhf0fE".to_string())
            .with_ankr(
                "648269110992d35fb12b490f3e9d00e18141ad9212081909344f15ec1c342a3c".to_string(),
            );

        // Load wallet if available
        config.wallet = Some(WalletConfig::new(
            "480c2f0730a4b305123b759f2a20ceb701643116671b232ffd5cdcbb90d4431a".to_string(),
            "0x7f1d163dBe1d42F9813820996e039E6f81D5f62c".to_string(),
        ));

        config
    }

    /// Get primary RPC URL for current network
    pub fn primary_rpc(&self) -> Option<&str> {
        self.rpc_urls
            .get("primary")
            .and_then(|urls| urls.first())
            .map(|s| s.as_str())
    }

    /// Get all fallback RPC URLs
    pub fn fallback_rpcs(&self) -> Vec<&str> {
        self.rpc_urls
            .get("fallback")
            .map(|urls| urls.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Add custom API key
    pub fn add_api_key(&mut self, name: String, key: String) {
        self.api_keys.insert(name, key);
    }

    /// Get API key by name
    pub fn get_api_key(&self, name: &str) -> Option<&str> {
        self.api_keys.get(name).map(|s| s.as_str())
    }
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Flash loan routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashloanRouteConfig {
    pub aave_v3_pool: String,
    pub balancer_vault: String,
    pub dforce_flash: String,
    pub radiant_pool: String,
    pub preferred_provider: String,
}

impl Default for FlashloanRouteConfig {
    fn default() -> Self {
        Self {
            aave_v3_pool: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string(),
            balancer_vault: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
            dforce_flash: "0x0988f3C0cFd7F0326475fA7fDa7F64e0663B70F0".to_string(),
            radiant_pool: "0xF4B1486DD74D07706052A33d31d7c0AAFD0659E1".to_string(),
            preferred_provider: "aave".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_env_parsing() {
        assert_eq!(NetworkEnv::from_env("arbitrum"), Some(NetworkEnv::Arbitrum));
        assert_eq!(NetworkEnv::from_env("base"), Some(NetworkEnv::Base));
        assert_eq!(NetworkEnv::from_env("ARBITRUM"), Some(NetworkEnv::Arbitrum));
        assert_eq!(NetworkEnv::from_env("invalid"), None);
    }

    #[test]
    fn test_network_chain_ids() {
        assert_eq!(NetworkEnv::Arbitrum.chain_id(), 42161);
        assert_eq!(NetworkEnv::Base.chain_id(), 8453);
        assert_eq!(NetworkEnv::Polygon.chain_id(), 137);
    }

    #[test]
    fn test_env_config_creation() {
        let config = EnvConfig::new(NetworkEnv::Arbitrum);
        assert_eq!(config.network, NetworkEnv::Arbitrum);
        assert!(config.primary_rpc().is_some());
    }

    #[test]
    fn test_wallet_config() {
        let wallet = WalletConfig::new("0x123".to_string(), "0xabc".to_string());
        assert_eq!(wallet.private_key, "0x123");
        assert_eq!(wallet.address, "0xabc");
    }

    #[test]
    fn test_provider_credentials() {
        let creds = ProviderCredentials::new()
            .with_alchemy("alchemy_key".to_string())
            .with_drpc("drpc_key".to_string());

        assert_eq!(creds.alchemy_api_key, Some("alchemy_key".to_string()));
        assert_eq!(creds.drpc_api_key, Some("drpc_key".to_string()));
        assert_eq!(creds.ankr_api_key, None);
    }
}
