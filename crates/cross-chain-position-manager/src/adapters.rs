//! Universal chain adapters for cross-chain position management
//!
//! This module provides:
//! - Chain registry integration
//! - Universal chain adapter wrapper
//! - Cross-chain communication adapters

use crate::config::{ChainConfig, PositionManagerConfig};
use crate::error::{PositionManagerError, Result};
use crate::types::{ChainSpecifics, H160, H256, U256};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

/// Chain information from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    pub chain_id: u64,
    pub name: String,
    pub chain_type: ChainType,
    pub rpc_url: String,
    pub explorer_url: Option<String>,
    pub native_token: NativeToken,
    pub block_time_ms: u64,
    pub confirmations_required: u64,
    pub is_testnet: bool,
}

/// Chain type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainType {
    Evm,
    Substrate,
    Cosmos,
    Solana,
    Bitcoin,
    Other,
}

/// Native token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeToken {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub address: Option<H160>,
}

/// Chain registry adapter for managing chain connections
#[derive(Debug, Clone)]
pub struct ChainRegistryAdapter {
    /// Registered chains
    chains: sp_std::collections::btree_map::BTreeMap<u64, ChainInfo>,
    /// Active connections
    active_connections: Vec<u64>,
    /// Configuration
    config: PositionManagerConfig,
}

impl ChainRegistryAdapter {
    /// Create a new chain registry adapter
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        let mut chains = sp_std::collections::btree_map::BTreeMap::new();

        // Register default chains from config
        for (chain_id, chain_config) in &config.chain_configs {
            let chain_info = ChainInfo {
                chain_id: *chain_id,
                name: format!("Chain_{}", chain_id),
                chain_type: ChainType::Evm,
                rpc_url: chain_config.rpc_url.clone(),
                explorer_url: None,
                native_token: NativeToken {
                    symbol: "ETH".to_string(),
                    name: "Ether".to_string(),
                    decimals: 18,
                    address: None,
                },
                block_time_ms: 12000,
                confirmations_required: chain_config.confirmations_required,
                is_testnet: false,
            };
            chains.insert(*chain_id, chain_info);
        }

        Ok(Self {
            chains,
            active_connections: Vec::new(),
            config: config.clone(),
        })
    }

    /// Register a new chain
    pub fn register_chain(&mut self, chain_info: ChainInfo) -> Result<()> {
        if self.chains.contains_key(&chain_info.chain_id) {
            return Err(PositionManagerError::ChainAlreadyRegistered(
                chain_info.chain_id,
            ));
        }
        self.chains.insert(chain_info.chain_id, chain_info);
        Ok(())
    }

    /// Get chain information
    pub fn get_chain_info(&self, chain_id: u64) -> Option<&ChainInfo> {
        self.chains.get(&chain_id)
    }

    /// List all registered chains
    pub fn list_chains(&self) -> Vec<&ChainInfo> {
        self.chains.values().collect()
    }

    /// Connect to a chain
    pub async fn connect_to_chain(&mut self, chain_id: u64) -> Result<()> {
        if !self.chains.contains_key(&chain_id) {
            return Err(PositionManagerError::ChainNotFound(chain_id));
        }

        if !self.active_connections.contains(&chain_id) {
            self.active_connections.push(chain_id);
        }

        Ok(())
    }

    /// Disconnect from a chain
    pub async fn disconnect_from_chain(&mut self, chain_id: u64) -> Result<()> {
        self.active_connections.retain(|&id| id != chain_id);
        Ok(())
    }

    /// Check if connected to a chain
    pub fn is_connected(&self, chain_id: u64) -> bool {
        self.active_connections.contains(&chain_id)
    }

    /// Get all active connections
    pub fn active_connections(&self) -> &[u64] {
        &self.active_connections
    }

    /// Get chain configuration
    pub fn get_chain_config(&self, chain_id: u64) -> Option<&ChainConfig> {
        self.config.chain_configs.get(&chain_id)
    }
}

/// Cross-chain adapter for communication between chains
#[derive(Debug, Clone)]
pub struct CrossChainAdapter {
    /// Source chain ID
    source_chain: u64,
    /// Target chain ID
    target_chain: u64,
    /// Bridge contract address
    bridge_address: H160,
    /// Message fee
    message_fee: U256,
    /// Timeout in milliseconds
    timeout_ms: u64,
}

impl CrossChainAdapter {
    /// Create a new cross-chain adapter
    pub fn new(
        source_chain: u64,
        target_chain: u64,
        bridge_address: H160,
        message_fee: U256,
        timeout_ms: u64,
    ) -> Self {
        Self {
            source_chain,
            target_chain,
            bridge_address,
            message_fee,
            timeout_ms,
        }
    }

    /// Send message to target chain
    pub async fn send_message(&self, message_type: MessageType, payload: Vec<u8>) -> Result<H256> {
        // Validate message
        if payload.is_empty() {
            return Err(PositionManagerError::InvalidMessage(
                "Empty payload".to_string(),
            ));
        }

        // Generate message ID
        let message_id = self.generate_message_id(&message_type, &payload);

        // In a real implementation, this would:
        // 1. Encode the message
        // 2. Call the bridge contract
        // 3. Pay the message fee
        // 4. Wait for confirmation

        Ok(message_id)
    }

    /// Receive message from source chain
    pub async fn receive_message(&self, message_id: H256) -> Result<Vec<u8>> {
        // In a real implementation, this would:
        // 1. Query the bridge contract for the message
        // 2. Verify the message signature
        // 3. Decode the payload
        // 4. Return the payload

        Ok(Vec::new())
    }

    /// Generate message ID
    fn generate_message_id(&self, message_type: &MessageType, payload: &[u8]) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        hasher.hash(&self.source_chain.to_le_bytes());
        hasher.hash(&self.target_chain.to_le_bytes());
        hasher.hash(&self.bridge_address.as_bytes());
        hasher.hash(&self.message_fee.as_bytes());
        hasher.hash(&self.timeout_ms.to_le_bytes());
        hasher.hash(&(message_type.clone() as u8).to_le_bytes());
        hasher.hash(&(payload.len() as u64).to_le_bytes());
        for byte in payload.iter().take(32) {
            hasher.hash(&[*byte]);
        }
        H256::from_slice(hasher.finish().as_ref())
    }

    /// Get source chain ID
    pub fn source_chain(&self) -> u64 {
        self.source_chain
    }

    /// Get target chain ID
    pub fn target_chain(&self) -> u64 {
        self.target_chain
    }

    /// Get bridge address
    pub fn bridge_address(&self) -> H160 {
        self.bridge_address
    }

    /// Get message fee
    pub fn message_fee(&self) -> U256 {
        self.message_fee
    }
}

/// Message types for cross-chain communication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    PositionUpdate,
    MigrationRequest,
    MigrationComplete,
    RiskAlert,
    PriceUpdate,
    StateSync,
}

/// Universal chain adapter for managing multiple chains
#[derive(Debug, Clone)]
pub struct UniversalChainAdapter {
    /// Chain registry
    registry: ChainRegistryAdapter,
    /// Cross-chain adapters
    cross_chain_adapters: sp_std::collections::btree_map::BTreeMap<(u64, u64), CrossChainAdapter>,
    /// Configuration
    config: PositionManagerConfig,
}

impl UniversalChainAdapter {
    /// Create a new universal chain adapter
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        let registry = ChainRegistryAdapter::new(config)?;

        Ok(Self {
            registry,
            cross_chain_adapters: sp_std::collections::btree_map::BTreeMap::new(),
            config: config.clone(),
        })
    }

    /// Connect to all configured chains
    pub async fn connect_all(&mut self) -> Result<()> {
        for chain_id in self.config.chain_configs.keys() {
            self.registry.connect_to_chain(*chain_id).await?;
        }
        Ok(())
    }

    /// Disconnect from all chains
    pub async fn disconnect_all(&mut self) -> Result<()> {
        for chain_id in self.config.chain_configs.keys() {
            self.registry.disconnect_from_chain(*chain_id).await?;
        }
        Ok(())
    }

    /// Get chain registry
    pub fn registry(&self) -> &ChainRegistryAdapter {
        &self.registry
    }

    /// Get mutable chain registry
    pub fn registry_mut(&mut self) -> &mut ChainRegistryAdapter {
        &mut self.registry
    }

    /// Add cross-chain adapter
    pub fn add_cross_chain_adapter(
        &mut self,
        source_chain: u64,
        target_chain: u64,
        adapter: CrossChainAdapter,
    ) {
        self.cross_chain_adapters
            .insert((source_chain, target_chain), adapter);
    }

    /// Get cross-chain adapter
    pub fn get_cross_chain_adapter(
        &self,
        source_chain: u64,
        target_chain: u64,
    ) -> Option<&CrossChainAdapter> {
        self.cross_chain_adapters.get(&(source_chain, target_chain))
    }

    /// Send message between chains
    pub async fn send_cross_chain_message(
        &self,
        source_chain: u64,
        target_chain: u64,
        message_type: MessageType,
        payload: Vec<u8>,
    ) -> Result<H256> {
        let adapter = self
            .get_cross_chain_adapter(source_chain, target_chain)
            .ok_or_else(|| {
                PositionManagerError::CrossChainAdapterNotFound(source_chain, target_chain)
            })?;

        adapter.send_message(message_type, payload).await
    }

    /// Get chain specifics
    pub fn get_chain_specifics(&self, chain_id: u64) -> Option<ChainSpecifics> {
        self.config
            .chain_configs
            .get(&chain_id)
            .map(|config| ChainSpecifics {
                chain_id,
                gas_price_multiplier: config.gas_price_multiplier,
                min_gas_price: config.min_gas_price,
                max_gas_price: config.max_gas_price,
                bridge_timeout_ms: config.bridge_timeout_ms,
                confirmations_required: config.confirmations_required,
                native_token_decimals: 18,
                supports_eip1559: true,
            })
    }

    /// List all supported chains
    pub fn supported_chains(&self) -> Vec<u64> {
        self.config.chain_configs.keys().cloned().collect()
    }

    /// Check if chain is supported
    pub fn is_chain_supported(&self, chain_id: u64) -> bool {
        self.config.chain_configs.contains_key(&chain_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_registry_adapter() {
        let config = PositionManagerConfig::default();
        let registry = ChainRegistryAdapter::new(&config).unwrap();

        assert!(registry.list_chains().len() > 0);
    }

    #[test]
    fn test_cross_chain_adapter() {
        let adapter = CrossChainAdapter::new(1, 137, H160::random(), U256::from(1000), 300000);

        assert_eq!(adapter.source_chain(), 1);
        assert_eq!(adapter.target_chain(), 137);
    }
}
