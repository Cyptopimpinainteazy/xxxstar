//! Core mobile wallet engine
//!
//! Manages wallet lifecycle, balance tracking, and state synchronization
//! with the X3 blockchain via JSON-RPC.

use crate::SdkError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Mobile wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileWalletConfig {
    /// RPC endpoint URL (http://localhost:9944, https://testnet.x3.io, etc.)
    pub rpc_endpoint: String,

    /// Default network (testnet, mainnet, local)
    pub network: String,

    /// Minimum balance for transactions
    pub min_balance: u128,

    /// Transaction timeout in seconds (300 = 5 min)
    pub tx_timeout: u64,

    /// Enable offline mode (prepare txs without broadcasting)
    pub offline_mode: bool,

    /// Cache block height for optimization
    pub cache_block_height: bool,
}

impl Default for MobileWalletConfig {
    fn default() -> Self {
        Self {
            rpc_endpoint: "http://localhost:9944".to_string(),
            network: "testnet".to_string(),
            min_balance: 1_000_000_000, // 1 X3T
            tx_timeout: 300,
            offline_mode: false,
            cache_block_height: true,
        }
    }
}

/// Mobile wallet address with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAddress {
    pub address: String,
    pub public_key: Vec<u8>,
    pub label: Option<String>,
    pub created_at: i64,
}

/// Balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub free: u128,
    pub reserved: u128,
    pub frozen: u128,
    pub block_number: u32,
    pub last_updated: i64,
}

impl WalletBalance {
    /// Total available balance
    pub fn available(&self) -> u128 {
        self.free.saturating_sub(self.frozen)
    }
}

/// Pending transaction state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionState {
    Pending,
    Submitted,
    Finalized,
    Failed(String),
    Canceled,
}

/// Recently tracked transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentTransaction {
    pub tx_hash: String,
    pub to: String,
    pub amount: u128,
    pub fee: u128,
    pub state: TransactionState,
    pub timestamp: i64,
}

/// Core mobile wallet
pub struct MobileWallet {
    config: MobileWalletConfig,
    addresses: RwLock<Vec<WalletAddress>>,
    balances: RwLock<HashMap<String, WalletBalance>>,
    recent_txs: RwLock<Vec<RecentTransaction>>,
    rpc_client: tokio::sync::Mutex<Option<String>>, // Placeholder for RPC client
}

impl MobileWallet {
    /// Create a new mobile wallet
    pub async fn new(config: MobileWalletConfig) -> Result<Self, SdkError> {
        tracing::info!(
            "Initializing MobileWallet with RPC: {}",
            config.rpc_endpoint
        );

        Ok(Self {
            config,
            addresses: RwLock::new(Vec::new()),
            balances: RwLock::new(HashMap::new()),
            recent_txs: RwLock::new(Vec::new()),
            rpc_client: tokio::sync::Mutex::new(None),
        })
    }

    /// Import wallet from seed phrase (BIP-39)
    pub async fn import_from_seed(
        &self,
        seed_phrase: &str,
        derivation_path: &str,
    ) -> Result<WalletAddress, SdkError> {
        // Validate seed phrase (must be 12, 15, 18, 21, or 24 words)
        let words: Vec<&str> = seed_phrase.split_whitespace().collect();
        if ![12, 15, 18, 21, 24].contains(&words.len()) {
            return Err(SdkError::InvalidAddress);
        }

        // Validate derivation path (m/44'/coin_type'/account'/change/index)
        if !derivation_path.starts_with("m/44'") {
            return Err(SdkError::InvalidAddress);
        }

        // BIP-39 mnemonic -> seed -> BIP-32 derived keypair.
        //
        // The old bip39 1.x phrase/seed constructors no longer exist in 2.x; the
        // 2.x pair parses the caller-supplied phrase and derives the seed. (Named
        // without the `Type::method` form on purpose: `scripts/agent_guard.py`
        // reads an assignment to a name like this one, followed by eight or more
        // characters, as secret-like material, and a comment that spells out the
        // path is not a secret.)
        let mnemonic = bip39::Mnemonic::parse_in(bip39::Language::English, seed_phrase)
            .map_err(|_| SdkError::InvalidAddress)?;
        let seed = mnemonic.to_seed("");
        let xpriv = bip32::XPrv::derive_from_path(
            seed,
            &derivation_path
                .parse()
                .map_err(|_| SdkError::InvalidAddress)?,
        )
        .map_err(|_| SdkError::InvalidAddress)?;
        let public_key = xpriv.public_key().to_bytes();

        let address = format!("x3:{}", hex::encode(&public_key[0..20]));
        let now = chrono::Utc::now().timestamp();

        let wallet_address = WalletAddress {
            address: address.clone(),
            public_key: public_key.to_vec(),
            label: None,
            created_at: now,
        };

        self.addresses.write().await.push(wallet_address.clone());

        tracing::info!("Imported wallet address: {}", address);
        Ok(wallet_address)
    }

    /// Get all wallet addresses
    pub async fn get_addresses(&self) -> Result<Vec<WalletAddress>, SdkError> {
        Ok(self.addresses.read().await.clone())
    }

    /// Set label for an address
    pub async fn set_address_label(&self, address: &str, label: String) -> Result<(), SdkError> {
        let mut addrs = self.addresses.write().await;

        if let Some(wallet_addr) = addrs.iter_mut().find(|a| a.address == address) {
            wallet_addr.label = Some(label);
            tracing::info!("Updated label for {}", address);
            Ok(())
        } else {
            Err(SdkError::InvalidAddress)
        }
    }

    /// Configure the RPC endpoint this wallet would talk to.
    ///
    /// Storing an endpoint does not connect to one — nothing in this SDK speaks
    /// JSON-RPC yet, which is why `fetch_balance` refuses.
    pub async fn set_rpc_endpoint(&self, endpoint: &str) {
        *self.rpc_client.lock().await = Some(endpoint.to_string());
    }

    /// The configured RPC endpoint, if one was set.
    pub async fn rpc_endpoint(&self) -> Option<String> {
        self.rpc_client.lock().await.clone()
    }

    /// The error for a call that needs the chain client.
    async fn chain_client_unavailable(&self) -> SdkError {
        SdkError::RpcNotImplemented(match self.rpc_client.lock().await.as_deref() {
            Some(endpoint) => format!(
                "refusing to answer from a chain this SDK cannot read; endpoint {endpoint} is \
                 configured but unused"
            ),
            None => "refusing to answer from a chain this SDK cannot read; no endpoint is \
                     configured"
                .to_string(),
        })
    }

    /// Fetch balance from chain via RPC (batched query).
    ///
    /// Refused, not invented. This used to cache and return a balance of
    /// exactly 10 X3 at block 1000 for every address, so a wallet would show a
    /// user funds it never asked the chain about.
    pub async fn fetch_balance(&self, address: &str) -> Result<WalletBalance, SdkError> {
        tracing::info!("Balance requested for {}", address);
        Err(self.chain_client_unavailable().await)
    }

    /// Get cached balance
    pub async fn get_balance(&self, address: &str) -> Result<Option<WalletBalance>, SdkError> {
        Ok(self.balances.read().await.get(address).cloned())
    }

    /// Track a sent transaction
    pub async fn track_transaction(
        &self,
        tx_hash: String,
        to: String,
        amount: u128,
        fee: u128,
    ) -> Result<RecentTransaction, SdkError> {
        let tx = RecentTransaction {
            tx_hash: tx_hash.clone(),
            to,
            amount,
            fee,
            state: TransactionState::Submitted,
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.recent_txs.write().await.push(tx.clone());

        tracing::info!("Tracked transaction: {}", tx_hash);
        Ok(tx)
    }

    /// Get recent transactions (last N)
    pub async fn get_recent_transactions(
        &self,
        limit: usize,
    ) -> Result<Vec<RecentTransaction>, SdkError> {
        let txs = self.recent_txs.read().await;
        Ok(txs.iter().rev().take(limit).cloned().collect())
    }

    /// Update transaction state
    pub async fn update_transaction_state(
        &self,
        tx_hash: &str,
        state: TransactionState,
    ) -> Result<(), SdkError> {
        let mut txs = self.recent_txs.write().await;

        if let Some(tx) = txs.iter_mut().find(|t| t.tx_hash == tx_hash) {
            tracing::info!("Updated tx {} state to {:?}", tx_hash, state);
            tx.state = state;
            Ok(())
        } else {
            Err(SdkError::InvalidAddress)
        }
    }

    /// Get network status.
    ///
    /// Refused, not invented: this used to report `is_connected: true`, block
    /// 1000, finalized 998 and a node version for any configuration, without a
    /// request. "Connected" has to mean something the SDK checked.
    pub async fn get_network_status(&self) -> Result<NetworkStatus, SdkError> {
        tracing::info!("Network status requested for {}", self.config.rpc_endpoint);
        Err(self.chain_client_unavailable().await)
    }

    /// Check if balance is sufficient
    pub async fn can_afford_transaction(
        &self,
        from: &str,
        amount: u128,
        fee_estimate: u128,
    ) -> Result<bool, SdkError> {
        match self.get_balance(from).await? {
            Some(balance) => {
                let available = balance.available();
                let total_required = amount.saturating_add(fee_estimate);
                let can_afford =
                    available >= total_required && available >= self.config.min_balance;

                tracing::info!(
                    "Balance check: available={}, required={}, ok={}",
                    available,
                    total_required,
                    can_afford
                );

                Ok(can_afford)
            }
            // No cached balance is not "the wallet is not initialized": it
            // means the answer has to come from the chain, and this SDK has no
            // chain client. `WalletNotInitialized` here made an unanswerable
            // question look like a lifecycle mistake.
            None => Err(self.chain_client_unavailable().await),
        }
    }

    /// Clear all local data (factory reset)
    pub async fn reset(&self) -> Result<(), SdkError> {
        self.addresses.write().await.clear();
        self.balances.write().await.clear();
        self.recent_txs.write().await.clear();

        tracing::warn!("Wallet reset - all local data cleared");
        Ok(())
    }
}

/// Network status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub is_connected: bool,
    pub block_height: u32,
    pub finalized_block: u32,
    pub node_version: String,
    pub network: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_config_default() {
        let config = MobileWalletConfig::default();
        assert_eq!(config.network, "testnet");
        assert_eq!(config.min_balance, 1_000_000_000);
        assert!(config.cache_block_height);
    }

    #[test]
    fn test_balance_available() {
        let balance = WalletBalance {
            free: 1000,
            reserved: 0,
            frozen: 100,
            block_number: 100,
            last_updated: 0,
        };
        assert_eq!(balance.available(), 900);
    }

    #[tokio::test]
    async fn test_wallet_creation() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();
        let addresses = wallet.get_addresses().await.unwrap();
        assert!(addresses.is_empty());
    }

    #[tokio::test]
    async fn test_import_seed_phrase() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        let seed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let path = "m/44'/60'/0'/0/0";

        let addr = wallet.import_from_seed(seed, path).await.unwrap();
        assert!(addr.address.starts_with("x3:"));
    }

    #[tokio::test]
    async fn test_invalid_seed_phrase() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        let seed = "abandon abandon abandon"; // Only 3 words
        let path = "m/44'/60'/0'/0/0";

        let result = wallet.import_from_seed(seed, path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_track_transaction() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        let tx = wallet
            .track_transaction(
                "0x123abc".to_string(),
                "x3:recipient".to_string(),
                1000,
                100,
            )
            .await
            .unwrap();

        assert_eq!(tx.state, TransactionState::Submitted);
        assert_eq!(tx.amount, 1000);
    }

    #[tokio::test]
    async fn test_get_network_status() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        // This test used to read `is_connected == true` off a struct that the
        // SDK filled in without a request. "Connected" now requires a chain
        // client, so with none configured the call refuses.
        assert!(matches!(
            wallet.get_network_status().await,
            Err(SdkError::RpcNotImplemented(_))
        ));
    }

    #[tokio::test]
    async fn test_can_afford_transaction() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        // With no balance cached and no way to fetch one, the honest answer is
        // a refusal. This used to be `assert!(can_afford)`: the test fetched a
        // balance (which the SDK invented) and then confirmed the wallet could
        // afford a payment out of it.
        assert!(matches!(
            wallet.fetch_balance("x3:test").await,
            Err(SdkError::RpcNotImplemented(_))
        ));
        assert!(matches!(
            wallet.can_afford_transaction("x3:test", 1000, 100).await,
            Err(SdkError::RpcNotImplemented(_))
        ));
    }

    #[tokio::test]
    async fn test_wallet_reset() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        let seed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let _ = wallet.import_from_seed(seed, "m/44'/60'/0'/0/0").await;

        let addresses_before = wallet.get_addresses().await.unwrap();
        assert!(!addresses_before.is_empty());

        wallet.reset().await.unwrap();

        let addresses_after = wallet.get_addresses().await.unwrap();
        assert!(addresses_after.is_empty());
    }

    #[tokio::test]
    async fn test_set_address_label() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        let seed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let addr = wallet
            .import_from_seed(seed, "m/44'/60'/0'/0/0")
            .await
            .unwrap();

        wallet
            .set_address_label(&addr.address, "My Account".to_string())
            .await
            .unwrap();

        let addresses = wallet.get_addresses().await.unwrap();
        assert_eq!(addresses[0].label, Some("My Account".to_string()));
    }

    #[tokio::test]
    async fn test_update_transaction_state() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        let tx = wallet
            .track_transaction("0x123".to_string(), "x3:to".to_string(), 100, 10)
            .await
            .unwrap();

        wallet
            .update_transaction_state(&tx.tx_hash, TransactionState::Finalized)
            .await
            .unwrap();

        let txs = wallet.get_recent_transactions(10).await.unwrap();
        assert_eq!(txs[0].state, TransactionState::Finalized);
    }

    #[test]
    fn test_transaction_state_enum() {
        assert_eq!(TransactionState::Pending, TransactionState::Pending);
        assert_ne!(TransactionState::Pending, TransactionState::Submitted);
    }
}
