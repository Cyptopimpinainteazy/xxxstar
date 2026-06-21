//! Live RPC integration tests.
//! Run with: cargo test -p x3-atomic-swap --features std --test rpc_live_test -- --ignored
//! Set RPC_URL env var or it uses a public fallback.

use x3_atomic_swap::error::SwapError;
use x3_atomic_swap::rpc_client::RpcClient;

/// Get RPC URL from env or use public sepolia endpoint
fn get_rpc_url() -> String {
    std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://ethereum-sepolia.publicnode.com".to_string())
}

/// Test making a real eth_blockNumber call against a testnet
#[ignore]
#[test]
fn test_live_eth_block_number() {
    let url = get_rpc_url();
    eprintln!("[LIVE] Testing eth_blockNumber against: {}", url);

    let mut client = RpcClient::new(url.clone(), 11155111);
    let result = client.get_block_number();

    match result {
        Ok(block_num) => {
            eprintln!("[LIVE] ✓ eth_blockNumber = {}", block_num);
            assert!(block_num > 0, "Block number must be > 0");
            assert!(block_num > 1000000, "Sepolia should have > 1M blocks");
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[LIVE] ⚠ RPC error (may be network/permissions): {}", msg);
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
    }
}

/// Test getting a specific address balance
#[ignore]
#[test]
fn test_live_eth_get_balance() {
    let url = get_rpc_url();
    eprintln!("[LIVE] Testing eth_getBalance against: {}", url);

    let mut client = RpcClient::new(url.clone(), 11155111);
    // Vitalik's public address on Sepolia - might have some test ETH
    let address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    let result = client.get_balance(address);

    match result {
        Ok(balance) => {
            eprintln!("[LIVE] ✓ Balance of {} = {} wei", address, balance);
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[LIVE] ⚠ RPC error: {}", msg);
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
    }
}

/// Test getting a transaction receipt
#[ignore]
#[test]
fn test_live_eth_get_transaction_receipt() {
    let url = get_rpc_url();
    eprintln!("[LIVE] Testing eth_getTransactionReceipt against: {}", url);

    let mut client = RpcClient::new(url.clone(), 11155111);
    // Use a zero hash - should return null/None
    let tx_hash = "0x0000000000000000000000000000000000000000000000000000000000000000";
    let result = client.get_transaction_receipt(tx_hash);

    match result {
        Ok(Some(receipt)) => {
            eprintln!("[LIVE] ✓ Transaction receipt found");
            assert!(receipt.is_object(), "Receipt should be an object");
        }
        Ok(None) => {
            eprintln!("[LIVE] ✓ Transaction not found (expected for zero hash)");
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[LIVE] ⚠ RPC error: {}", msg);
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
    }
}

/// Test estimating gas for a simple ETH transfer
#[ignore]
#[test]
fn test_live_eth_estimate_gas() {
    let url = get_rpc_url();
    eprintln!("[LIVE] Testing eth_estimateGas against: {}", url);

    let mut client = RpcClient::new(url.clone(), 11155111);
    let from = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    let to = "0x0000000000000000000000000000000000000000";
    // Empty data = simple ETH transfer
    let data = "0x";
    let result = client.estimate_gas(from, to, data);

    match result {
        Ok(gas) => {
            eprintln!("[LIVE] ✓ Estimated gas = {}", gas);
            assert!(gas > 0, "Gas estimate must be > 0");
            assert!(gas < 100000, "Simple transfer should cost < 100k gas");
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[LIVE] ⚠ RPC error: {}", msg);
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
    }
}

/// Test getting a block by number
#[ignore]
#[test]
fn test_live_eth_get_block_by_number() {
    let url = get_rpc_url();
    eprintln!("[LIVE] Testing eth_getBlockByNumber against: {}", url);

    let mut client = RpcClient::new(url.clone(), 11155111);
    // Block 1 on Sepolia (genesis is 0)
    let result = client.get_block_by_number(1, false);

    match result {
        Ok(block) => {
            eprintln!("[LIVE] ✓ Block 1 found: {:?}", block.get("hash"));
            assert!(block.is_object(), "Block should be an object");
            let number = block.get("number").and_then(|v| v.as_str());
            assert!(number.is_some(), "Block should have a number field");
            eprintln!("[LIVE] ✓ Block number field: {:?}", number);
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[LIVE] ⚠ RPC error: {}", msg);
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
    }
}

/// Test that the chain_id matches Sepolia
#[ignore]
#[test]
fn test_live_eth_chain_id() {
    let url = get_rpc_url();
    eprintln!("[LIVE] Testing eth_chainId against: {}", url);

    let mut client = RpcClient::new(url.clone(), 11155111);
    let params = vec![];
    let result = client.call("eth_chainId", params);

    match result {
        Ok(resp) => {
            if let Some(result_val) = resp.result {
                let chain_id_hex = result_val.as_str().unwrap_or("0x0");
                let chain_id =
                    u64::from_str_radix(chain_id_hex.trim_start_matches("0x"), 16).unwrap_or(0);
                eprintln!("[LIVE] ✓ chainId = {} ({})", chain_id, chain_id_hex);
                assert_eq!(chain_id, 11155111, "Should be Sepolia (11155111)");
            }
        }
        Err(SwapError::RpcError(msg)) => {
            eprintln!("[LIVE] ⚠ RPC error: {}", msg);
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
    }
}
