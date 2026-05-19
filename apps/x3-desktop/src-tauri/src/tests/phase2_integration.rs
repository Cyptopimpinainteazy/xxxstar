//! Phase 2 Integration Tests for Tauri Desktop Core
//!
//! This module tests the integration of:
//! - IPC communication
//! - Substrate hooks
//! - Wallet store
//! - x3ChainService

use crate::wallet_core::substrate_hook::{SubstrateHookHandler, SubstrateHookManager};
use crate::wallet_core::wallet_store::WalletStore;
use crate::wallet_core::x3_chain_service::{X3ChainService, X3ChainServiceConfig};
use crate::wallet_core::ipc::{IntentDraft, AssetRequirement, ChainType, FeeCap};

#[test]
fn test_ipc_intent_draft_serialization() {
    let intent = IntentDraft {
        id: "test-intent-123".to_string(),
        parties: vec!["party1".to_string(), "party2".to_string()],
        assets: vec![AssetRequirement {
            chain: ChainType::EVM,
            chain_id: "1".to_string(),
            token_address: "0x0000000000000000000000000000000000000000".to_string(),
            amount: "1000000000000000000".to_string(),
            slippage_basis_points: 100,
        }],
        fee_caps: vec![FeeCap {
            chain: "EVM".to_string(),
            max_fee: "10000000000".to_string(),
        }],
        expiry_timestamp_ms: 1234567890,
    };

    // Test serialization
    let serialized = serde_json::to_string(&intent).expect("Failed to serialize intent");
    assert!(serialized.contains("test-intent-123"));

    // Test deserialization
    let deserialized: IntentDraft = serde_json::from_str(&serialized).expect("Failed to deserialize intent");
    assert_eq!(deserialized.id, "test-intent-123");
    assert_eq!(deserialized.parties.len(), 2);
}

#[test]
fn test_substrate_hook_handler() {
    let config = crate::wallet_core::substrate_hook::SubstrateHookConfig {
        rpc_url: "ws://127.0.0.1:9944".to_string(),
        subscription_timeout_ms: 30000,
        reconnect_delay_ms: 5000,
        max_retries: 3,
    };

    let mut handler = SubstrateHookHandler::new(config);

    // Test initial state
    assert!(!handler.is_connected());
    assert_eq!(handler.get_last_block_number(), None);

    // Test block processing
    handler.on_new_block(
        sp_core::H256::from([0u8; 32]),
        12345,
        sp_core::H256::from([1u8; 32]),
        1234567890,
    );

    assert!(handler.is_connected());
    assert_eq!(handler.get_last_block_number(), Some(12345));
}

#[test]
fn test_substrate_hook_manager() {
    let mut manager = SubstrateHookManager::new("ws://127.0.0.1:9944");

    let handler1 = manager.get_handler("handler1");
    assert!(handler1.is_connected());

    let handler2 = manager.get_handler("handler2");
    assert_eq!(manager.handler_count(), 2);

    manager.remove_handler("handler1");
    assert_eq!(manager.handler_count(), 1);
}

#[test]
fn test_wallet_store() {
    let mut store = WalletStore::new();
    store.initialize();

    // Test wallet storage
    store
        .store_wallet(
            "test_wallet",
            "test mnemonic words for testing",
            "test seed hex",
            "m/44'/60'/0'/0/0",
        )
        .expect("Failed to store wallet");

    assert_eq!(store.wallet_count(), 1);

    // Test wallet retrieval
    let (mnemonic, seed) = store.retrieve_wallet("test_wallet").expect("Failed to retrieve wallet");
    assert_eq!(mnemonic, "test mnemonic words for testing");
    assert_eq!(seed, "test seed hex");

    // Test wallet deletion
    store.delete_wallet("test_wallet").expect("Failed to delete wallet");
    assert_eq!(store.wallet_count(), 0);
}

#[test]
fn test_x3_chain_service() {
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
fn test_full_integration_flow() {
    // This test simulates a full integration flow:
    // 1. Create an intent
    // 2. Verify intent through quorum
    // 3. Store wallet
    // 4. Query chain state
    // 5. Execute transaction

    // Step 1: Create intent
    let intent = IntentDraft {
        id: "integration-test-intent".to_string(),
        parties: vec!["sender".to_string(), "receiver".to_string()],
        assets: vec![AssetRequirement {
            chain: ChainType::EVM,
            chain_id: "1".to_string(),
            token_address: "0x0000000000000000000000000000000000000000".to_string(),
            amount: "1000000000000000000".to_string(),
            slippage_basis_points: 100,
        }],
        fee_caps: vec![FeeCap {
            chain: "EVM".to_string(),
            max_fee: "10000000000".to_string(),
        }],
        expiry_timestamp_ms: 1234567890,
    };

    // Step 2: Verify intent (simulated)
    let intent_hash = format!(
        "0x{}",
        hex::encode(sp_core::hashing::sha2_256(
            &serde_json::to_vec(&intent).unwrap()
        ))
    );

    // Step 3: Store wallet
    let mut store = WalletStore::new();
    store.initialize();
    store
        .store_wallet(
            "integration_test_wallet",
            "test mnemonic for integration",
            "test seed for integration",
            "m/44'/60'/0'/0/0",
        )
        .expect("Failed to store wallet");

    // Step 4: Query chain state
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

    // Step 5: Execute transaction (simulated)
    let tx_hash = format!(
        "0x{}",
        hex::encode(sp_core::hashing::keccak_256(
            intent_hash.as_bytes()
        ))
    );

    // Verify the flow
    assert!(!intent_hash.is_empty());
    assert_eq!(store.wallet_count(), 1);
    assert!(!tx_hash.is_empty());
}
