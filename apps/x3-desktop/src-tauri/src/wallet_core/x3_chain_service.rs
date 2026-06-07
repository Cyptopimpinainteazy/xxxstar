//! x3ChainService for Tauri Desktop Core
//!
//! This module provides the service layer for X3 chain operations with:
//!
//! - Chain operation methods (queries, transactions, subscriptions)
//! - Error handling with detailed error types
//! - Caching mechanism for performance
//! - Configuration support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// X3 chain operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainOperation {
    QueryBlock {
        block_number: Option<u64>,
        block_hash: Option<String>,
    },
    QueryAccount {
        address: String,
        at_block: Option<u64>,
    },
    QueryBalance {
        address: String,
        asset_id: Option<String>,
    },
    QueryStorage {
        key: String,
        at_block: Option<u64>,
    },
    SubmitExtrinsic {
        call: String,
        signer: String,
        nonce: Option<u64>,
        tip: Option<u64>,
    },
    SubscribeNewHeads,
    SubscribeEvents,
    EstimateGas {
        call: String,
        signer: String,
    },
    SimulateTransaction {
        call: String,
        signer: String,
    },
}

/// Chain operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOperationResult {
    pub success: bool,
    pub operation: String,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Cache entry for chain data
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub timestamp: Instant,
    pub ttl: Duration,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }
}

/// Chain cache for performance optimization
pub struct ChainCache {
    entries: HashMap<String, CacheEntry<serde_json::Value>>,
    default_ttl: Duration,
}

impl ChainCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.entries.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(&entry.value)
            }
        })
    }

    pub fn set(&mut self, key: &str, value: serde_json::Value, ttl: Option<Duration>) {
        let ttl = ttl.unwrap_or(self.default_ttl);
        self.entries.insert(key.to_string(), CacheEntry::new(value, ttl));
    }

    pub fn remove(&mut self, key: &str) {
        self.entries.remove(key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }
}

/// x3ChainService error
#[derive(Debug, thiserror::Error)]
pub enum X3ChainServiceError {
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// x3ChainService configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X3ChainServiceConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub timeout_ms: u64,
    pub cache_ttl_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

/// x3ChainService - manages X3 chain operations
pub struct X3ChainService {
    config: X3ChainServiceConfig,
    cache: ChainCache,
    connection_status: ConnectionStatus,
    operation_stats: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub last_connected: Option<u64>,
    pub last_disconnected: Option<u64>,
    pub block_number: Option<u64>,
    pub chain_name: Option<String>,
}

impl X3ChainService {
    /// Create a new x3ChainService
    pub fn new(config: X3ChainServiceConfig) -> Self {
        let cache_ttl_ms = config.cache_ttl_ms;
        Self {
            config,
            cache: ChainCache::new(Duration::from_millis(cache_ttl_ms)),
            connection_status: ConnectionStatus {
                connected: false,
                last_connected: None,
                last_disconnected: None,
                block_number: None,
                chain_name: None,
            },
            operation_stats: HashMap::new(),
        }
    }

    /// Execute a chain operation
    pub async fn execute_operation(
        &mut self,
        operation: ChainOperation,
    ) -> Result<ChainOperationResult, X3ChainServiceError> {
        let start_time = Instant::now();
        let operation_name = format!("{:?}", &operation);

        let result = match operation {
            ChainOperation::QueryBlock { block_number, block_hash } => {
                self.execute_query_block(block_number, block_hash).await
            }
            ChainOperation::QueryAccount { address, at_block } => {
                self.execute_query_account(&address, at_block).await
            }
            ChainOperation::QueryBalance { address, asset_id } => {
                self.execute_query_balance(&address, asset_id).await
            }
            ChainOperation::QueryStorage { key, at_block } => {
                self.execute_query_storage(&key, at_block).await
            }
            ChainOperation::SubmitExtrinsic {
                call,
                signer,
                nonce,
                tip,
            } => self.execute_submit_extrinsic(&call, &signer, nonce, tip).await,
            ChainOperation::SubscribeNewHeads => self.execute_subscribe_new_heads().await,
            ChainOperation::SubscribeEvents => self.execute_subscribe_events().await,
            ChainOperation::EstimateGas { call, signer } => {
                self.execute_estimate_gas(&call, &signer).await
            }
            ChainOperation::SimulateTransaction { call, signer } => {
                self.execute_simulate_transaction(&call, &signer).await
            }
        };

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        *self.operation_stats.entry(operation_name.clone()).or_insert(0) += 1;

        match result {
            Ok(data) => Ok(ChainOperationResult {
                success: true,
                operation: operation_name,
                data: Some(data),
                error: None,
                execution_time_ms,
            }),
            Err(e) => Ok(ChainOperationResult {
                success: false,
                operation: operation_name,
                data: None,
                error: Some(e.to_string()),
                execution_time_ms,
            }),
        }
    }

    /// Execute a block query
    async fn execute_query_block(
        &mut self,
        block_number: Option<u64>,
        block_hash: Option<String>,
    ) -> Result<serde_json::Value, X3ChainServiceError> {
        // Check cache first
        let cache_key = format!("block_{}_{}", block_number.unwrap_or(0), block_hash.clone().unwrap_or("latest".to_string()));
        
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // In production, this would make an actual RPC call
        // For now, return a mock response
        let result = serde_json::json!({
            "number": block_number.unwrap_or(12345),
            "hash": block_hash.unwrap_or("0x1234...".to_string()),
            "parentHash": "0x5678...".to_string(),
            "timestamp": 1234567890,
            "extrinsics": [],
            "events": []
        });

        self.cache.set(&cache_key, result.clone(), None);
        Ok(result)
    }

    /// Execute an account query
    async fn execute_query_account(
        &mut self,
        address: &str,
        at_block: Option<u64>,
    ) -> Result<serde_json::Value, X3ChainServiceError> {
        let cache_key = format!("account_{}_{}", address, at_block.unwrap_or(0));

        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let result = serde_json::json!({
            "address": address,
            "nonce": 0,
            "data": {
                "free": 1_000_000_000_000i64,
                "reserved": 0,
                "misc_frozen": 0,
                "fee_frozen": 0
            },
            "providers": 1,
            "consumers": 0,
            "sufficients": 0,
            "authored_blocks": 0
        });

        self.cache.set(&cache_key, result.clone(), None);
        Ok(result)
    }

    /// Execute a balance query
    async fn execute_query_balance(
        &mut self,
        address: &str,
        asset_id: Option<String>,
    ) -> Result<serde_json::Value, X3ChainServiceError> {
        let asset = asset_id.unwrap_or_else(|| "X3".to_string());
        let cache_key = format!("balance_{}_{}", address, asset);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let result = serde_json::json!({
            "address": address,
            "asset_id": asset,
            "free": 1_000_000_000_000i64,
            "reserved": 0,
            "frozen": 0,
            "flags": []
        });

        self.cache.set(&cache_key, result.clone(), None);
        Ok(result)
    }

    /// Execute a storage query
    async fn execute_query_storage(
        &mut self,
        key: &str,
        at_block: Option<u64>,
    ) -> Result<serde_json::Value, X3ChainServiceError> {
        let cache_key = format!("storage_{}_{}", key, at_block.unwrap_or(0));

        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let result = serde_json::json!({
            "key": key,
            "value": "0x12345678",
            "at_block": at_block.unwrap_or(12345)
        });

        self.cache.set(&cache_key, result.clone(), None);
        Ok(result)
    }

    /// Execute an extrinsic submission
    async fn execute_submit_extrinsic(
        &mut self,
        call: &str,
        signer: &str,
        nonce: Option<u64>,
        tip: Option<u64>,
    ) -> Result<serde_json::Value, X3ChainServiceError> {
        let result = serde_json::json!({
            "tx_hash": format!("0x{}", hex::encode(sp_core::hashing::keccak_256(call.as_bytes()))),
            "signer": signer,
            "nonce": nonce.unwrap_or(0),
            "tip": tip.unwrap_or(0),
            "status": "pending",
            "block_hash": null
        });

        Ok(result)
    }

    /// Execute a new heads subscription
    async fn execute_subscribe_new_heads(&mut self) -> Result<serde_json::Value, X3ChainServiceError> {
        self.connection_status.connected = true;
        self.connection_status.last_connected = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );

        let result = serde_json::json!({
            "subscription_id": format!("new_heads_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
            "status": "subscribed"
        });

        Ok(result)
    }

    /// Execute an events subscription
    async fn execute_subscribe_events(&mut self) -> Result<serde_json::Value, X3ChainServiceError> {
        let result = serde_json::json!({
            "subscription_id": format!("events_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
            "status": "subscribed"
        });

        Ok(result)
    }

    /// Execute a gas estimation
    async fn execute_estimate_gas(
        &mut self,
        call: &str,
        signer: &str,
    ) -> Result<serde_json::Value, X3ChainServiceError> {
        let result = serde_json::json!({
            "gas_limit": 1000000,
            "gas_price": 1000000000,
            "total_fee": 1_000_000_000_000i64,
            "weight": 500000000
        });

        Ok(result)
    }

    /// Execute a transaction simulation
    async fn execute_simulate_transaction(
        &mut self,
        call: &str,
        signer: &str,
    ) -> Result<serde_json::Value, X3ChainServiceError> {
        let result = serde_json::json!({
            "success": true,
            "weight_used": 500000000,
            "class": "Normal",
            "pays_fee": "Yes",
            "events": []
        });

        Ok(result)
    }

    /// Get the connection status
    pub fn get_connection_status(&self) -> &ConnectionStatus {
        &self.connection_status
    }

    /// Get operation statistics
    pub fn get_operation_stats(&self) -> &HashMap<String, u64> {
        &self.operation_stats
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn get_cache_size(&self) -> usize {
        self.cache.size()
    }

    /// Get the service configuration
    pub fn get_config(&self) -> &X3ChainServiceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x3_chain_service_creation() {
        let config = X3ChainServiceConfig {
            rpc_url: "http://127.0.0.1:9933".to_string(),
            ws_url: "ws://127.0.0.1:9944".to_string(),
            timeout_ms: 30000,
            cache_ttl_ms: 60000,
            max_retries: 3,
            retry_delay_ms: 1000,
        };

        let service = X3ChainService::new(config);
        assert!(!service.get_connection_status().connected);
        assert_eq!(service.get_cache_size(), 0);
    }

    #[test]
    fn test_cache_ttl() {
        let config = X3ChainServiceConfig {
            rpc_url: "http://127.0.0.1:9933".to_string(),
            ws_url: "ws://127.0.0.1:9944".to_string(),
            timeout_ms: 30000,
            cache_ttl_ms: 100, // 100ms TTL
            max_retries: 3,
            retry_delay_ms: 1000,
        };

        let mut service = X3ChainService::new(config);
        
        // Set a cache entry
        service.cache.set("test_key", serde_json::json!("test_value"), None);
        assert_eq!(service.get_cache_size(), 1);

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(150));
        
        // Entry should be expired
        assert_eq!(service.get_cache_size(), 0);
    }
}
