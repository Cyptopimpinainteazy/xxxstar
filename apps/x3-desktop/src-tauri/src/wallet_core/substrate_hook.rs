//! Substrate Hook Integration for Tauri Desktop Core
//!
//! This module provides the bridge between Tauri frontend and the Substrate blockchain
//! through the x3-chain-node RPC interface. It handles:
//!
//! - Substrate event subscriptions (new heads, extrinsics, events)
//! - RPC query execution for chain state
//! - Hook registration and execution for blockchain events
//! - Error handling and retry logic

use serde::{Deserialize, Serialize};
use sp_core::{crypto::AccountId32, H256};
use std::collections::HashMap;

/// Substrate hook event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubstrateHookEvent {
    NewBlock {
        hash: H256,
        number: u64,
        parent_hash: H256,
        timestamp: u64,
    },
    Extrinsic {
        hash: H256,
        signer: AccountId32,
        method: String,
        success: bool,
        error: Option<String>,
    },
    ChainReorg {
        old_hash: H256,
        new_hash: H256,
        reorg_depth: u32,
    },
}

/// Substrate hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateHookConfig {
    pub rpc_url: String,
    pub subscription_timeout_ms: u64,
    pub reconnect_delay_ms: u64,
    pub max_retries: u32,
}

/// Substrate hook state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateHookState {
    pub connected: bool,
    pub last_block_number: Option<u64>,
    pub last_block_hash: Option<H256>,
    pub subscription_count: usize,
    pub hooks: Vec<SubstrateHookConfig>,
}

/// Substrate hook handler
pub struct SubstrateHookHandler {
    config: SubstrateHookConfig,
    state: SubstrateHookState,
    event_callbacks: HashMap<String, Box<dyn Fn(SubstrateHookEvent) + Send + Sync>>,
}

impl SubstrateHookHandler {
    /// Create a new Substrate hook handler
    pub fn new(config: SubstrateHookConfig) -> Self {
        Self {
            config,
            state: SubstrateHookState {
                connected: false,
                last_block_number: None,
                last_block_hash: None,
                subscription_count: 0,
                hooks: Vec::new(),
            },
            event_callbacks: HashMap::new(),
        }
    }

    /// Register a callback for a specific hook type
    pub fn register_hook<F>(&mut self, hook_id: &str, callback: F)
    where
        F: Fn(SubstrateHookEvent) + Send + Sync + 'static,
    {
        self.event_callbacks.insert(hook_id.to_string(), Box::new(callback));
    }

    /// Unregister a hook by ID
    pub fn unregister_hook(&mut self, hook_id: &str) {
        self.event_callbacks.remove(hook_id);
    }

    /// Notify all registered callbacks of an event
    pub fn notify_events(&self, event: SubstrateHookEvent) {
        for (_id, callback) in &self.event_callbacks {
            callback(event.clone());
        }
    }

    /// Process a new block event
    pub fn on_new_block(&mut self, hash: H256, number: u64, parent_hash: H256, timestamp: u64) {
        self.state.last_block_number = Some(number);
        self.state.last_block_hash = Some(hash);
        self.state.connected = true;

        let event = SubstrateHookEvent::NewBlock {
            hash,
            number,
            parent_hash,
            timestamp,
        };
        self.notify_events(event);
    }

    /// Process an extrinsic event
    pub fn on_extrinsic(
        &mut self,
        hash: H256,
        signer: AccountId32,
        method: &str,
        success: bool,
        error: Option<String>,
    ) {
        let event = SubstrateHookEvent::Extrinsic {
            hash,
            signer,
            method: method.to_string(),
            success,
            error,
        };
        self.notify_events(event);
    }

    /// Process a chain reorg event
    pub fn on_chain_reorg(&mut self, old_hash: H256, new_hash: H256, reorg_depth: u32) {
        let event = SubstrateHookEvent::ChainReorg {
            old_hash,
            new_hash,
            reorg_depth,
        };
        self.notify_events(event);
    }

    /// Get the current hook state
    pub fn get_state(&self) -> &SubstrateHookState {
        &self.state
    }

    /// Check if connected to the Substrate node
    pub fn is_connected(&self) -> bool {
        self.state.connected
    }

    /// Get the last block number
    pub fn get_last_block_number(&self) -> Option<u64> {
        self.state.last_block_number
    }
}

/// Substrate hook manager - manages multiple hook handlers
pub struct SubstrateHookManager {
    handlers: HashMap<String, SubstrateHookHandler>,
    default_config: SubstrateHookConfig,
}

impl SubstrateHookManager {
    /// Create a new Substrate hook manager
    pub fn new(rpc_url: &str) -> Self {
        let default_config = SubstrateHookConfig {
            rpc_url: rpc_url.to_string(),
            subscription_timeout_ms: 30000,
            reconnect_delay_ms: 5000,
            max_retries: 3,
        };

        Self {
            handlers: HashMap::new(),
            default_config,
        }
    }

    /// Create or get a hook handler
    pub fn get_handler(&mut self, handler_id: &str) -> &mut SubstrateHookHandler {
        if !self.handlers.contains_key(handler_id) {
            let handler = SubstrateHookHandler::new(self.default_config.clone());
            self.handlers.insert(handler_id.to_string(), handler);
        }
        self.handlers.get_mut(handler_id).unwrap()
    }

    /// Remove a hook handler
    pub fn remove_handler(&mut self, handler_id: &str) {
        self.handlers.remove(handler_id);
    }

    /// Get the number of active handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substrate_hook_handler_creation() {
        let config = SubstrateHookConfig {
            rpc_url: "ws://127.0.0.1:9944".to_string(),
            subscription_timeout_ms: 30000,
            reconnect_delay_ms: 5000,
            max_retries: 3,
        };

        let handler = SubstrateHookHandler::new(config);
        assert!(!handler.is_connected());
        assert_eq!(handler.get_last_block_number(), None);
    }

    #[test]
    fn test_substrate_hook_manager() {
        let mut manager = SubstrateHookManager::new("ws://127.0.0.1:9944");

        let handler1 = manager.get_handler("handler1");
        assert!(handler1.is_connected());

        let handler2 = manager.get_handler("handler2");
        assert_eq!(manager.handler_count(), 2);
    }
}
