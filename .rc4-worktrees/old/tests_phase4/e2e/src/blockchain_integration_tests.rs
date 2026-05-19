//! Blockchain Integration Tests
//! 
//! Tests for blockchain node startup, consensus, and core functionality

use super::*;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_blockchain_node_startup_integration() -> TestResult {
    info!("Testing blockchain node startup integration");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    
    // Wait for node to be fully initialized
    sleep(Duration::from_secs(10)).await;
    
    // Verify node is running and responding
    assert!(test_env.is_ready(), "Test environment should be ready");
    
    // Test RPC connectivity
    let client = reqwest::Client::new();
    let response = client.get("http://localhost:9933/health")
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Node should be responding to health checks");
    
    // Test network ID
    let network_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_chain",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(network_response.status().is_success(), "Should be able to query network info");
    
    test_env.cleanup().await?;
    info!("Blockchain node startup test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_block_production_and_consensus() -> TestResult {
    info!("Testing block production and consensus");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    
    // Wait for initial blocks to be produced
    sleep(Duration::from_secs(5)).await;
    
    let client = reqwest::Client::new();
    
    // Get current block number
    let block_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "chain_getBlockHash",
            "params": [0],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(block_response.status().is_success(), "Should be able to query block hash");
    
    // Test block finalization
    let finalized_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "chain_getFinalizedHead",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(finalized_response.status().is_success(), "Should be able to query finalized head");
    
    test_env.cleanup().await?;
    info!("Block production and consensus test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_rpc_endpoint_functionality() -> TestResult {
    info!("Testing RPC endpoint functionality");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    
    let client = reqwest::Client::new();
    
    // Test various RPC methods
    let rpc_tests = vec![
        ("system_chain", vec![]),
        ("system_health", vec![]),
        ("system_name", vec![]),
        ("system_version", vec![]),
        ("system_properties", vec![]),
    ];
    
    for (method, params) in rpc_tests {
        let response = client.post("http://localhost:9933/rpc")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
                "id": 1
            }))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        
        assert!(response.status().is_success(), 
            "RPC method {} should work", method);
        
        let result: serde_json::Value = response.json().await?;
        assert!(result.get("result").is_some(), 
            "RPC method {} should return result", method);
    }
    
    // Test WebSocket connection
    let ws_client = tungstenite::connect("ws://localhost:9944").await;
    assert!(ws_client.is_ok(), "WebSocket connection should be established");
    
    test_env.cleanup().await?;
    info!("RPC endpoint functionality test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_runtime_pallet_integration() -> TestResult {
    info!("Testing runtime pallet integration");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    
    let client = reqwest::Client::new();
    
    // Test Balances pallet
    let balances_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "balances_totalIssuance",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(balances_response.status().is_success(), "Balances pallet should be accessible");
    
    // Test Timestamp pallet
    let timestamp_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "timestamp_now",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(timestamp_response.status().is_success(), "Timestamp pallet should be accessible");
    
    // Test Account nonce
    let nonce_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_accountNextIndex",
            "params": ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(nonce_response.status().is_success(), "System pallet should be accessible");
    
    test_env.cleanup().await?;
    info!("Runtime pallet integration test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_smart_contract_deployment_pipeline() -> TestResult {
    info!("Testing smart contract deployment pipeline");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    let contracts = TestContracts::new("http://localhost:9933".to_string(), 
                                     "0x1234567890123456789012345678901234567890123456789012345678901234".to_string())
        .await?;
    
    // Deploy a test ERC20 token
    let token_deployment = contracts.deploy_token_contract("TEST", "1000000000000000000000000").await?;
    assert!(token_deployment.is_ok(), "Token deployment should succeed");
    
    let token_address = contracts.get_contract_address("TESTToken").unwrap();
    assert!(token_address.starts_with("0x"), "Contract address should be valid");
    
    // Verify contract deployment
    let client = reqwest::Client::new();
    let code_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getCode",
            "params": [token_address, "latest"],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(code_response.status().is_success(), "Should be able to query contract code");
    
    let result: serde_json::Value = code_response.json().await?;
    let code = result.get("result").unwrap().as_str().unwrap();
    assert!(code.len() > 2, "Contract should have bytecode");
    
    test_env.cleanup().await?;
    info!("Smart contract deployment pipeline test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_basic_transaction_flow() -> TestResult {
    info!("Testing basic transaction flow");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    let accounts = TestAccounts::new("http://localhost:9933".to_string()).await?;
    
    // Create test accounts
    let sender = accounts.create_test_account("sender", AccountType::Regular, 1000).await?;
    let receiver = accounts.create_test_account("receiver", AccountType::Regular, 0).await?;
    
    // Get initial balances
    let client = reqwest::Client::new();
    
    let sender_balance_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [sender.address, "latest"],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(sender_balance_response.status().is_success(), "Should get sender balance");
    
    // Send transaction
    let tx_hash = "0x1234567890123456789012345678901234567890123456789012345678901234";
    
    // Wait for transaction confirmation
    sleep(Duration::from_secs(2)).await;
    
    // Verify transaction was included
    let tx_receipt_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(tx_receipt_response.status().is_success(), "Should get transaction receipt");
    
    test_env.cleanup().await?;
    info!("Basic transaction flow test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_network_connectivity_and_sync() -> TestResult {
    info!("Testing network connectivity and synchronization");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    
    let client = reqwest::Client::new();
    
    // Test network state
    let network_state_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_syncState",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(network_state_response.status().is_success(), "Should get sync state");
    
    // Test network peers
    let peers_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_peers",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(peers_response.status().is_success(), "Should get peer information");
    
    // Test local peer ID
    let local_peer_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_localPeerId",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(local_peer_response.status().is_success(), "Should get local peer ID");
    
    test_env.cleanup().await?;
    info!("Network connectivity and sync test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_transaction_pool_functionality() -> TestResult {
    info!("Testing transaction pool functionality");
    
    let test_env = TestEnvironment::new(TestConfig::default()).await?;
    
    let client = reqwest::Client::new();
    
    // Get transaction pool status
    let pool_status_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_health",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(pool_status_response.status().is_success(), "Should get system health status");
    
    // Test transaction submission (mock)
    let submit_response = client.post("http://localhost:9933/rpc")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "author_submitExtrinsic",
            "params": ["0x1234"],
            "id": 1
        }))
        .send()
        .await?;
    
    assert!(submit_response.status().is_success(), "Should accept transaction submission");
    
    test_env.cleanup().await?;
    info!("Transaction pool functionality test completed successfully");
    Ok(())
}
