use serde_json::Value;
use std::time::Duration;

/// Chain JSON-RPC configuration — connects to real Ethereum/X3 nodes.
/// Override URLs via environment variables.
pub struct ChainRpcConfig {
    pub ethereum_rpc: String,
    pub x3_rpc: String,
    pub local_rpc: String,
}

impl Default for ChainRpcConfig {
    fn default() -> Self {
        Self {
            ethereum_rpc: std::env::var("ETH_RPC_URL")
                .unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string()),
            x3_rpc: std::env::var("X3_NODE_RPC")
                .unwrap_or_else(|_| "http://rpc.testnet.x3-chain.io:9944".to_string()),
            local_rpc: "http://127.0.0.1:8545".to_string(),
        }
    }
}

/// Perform a JSON-RPC 2.0 POST call against the given URL.
/// Returns the `result` field as a `serde_json::Value`, or `None` on error.
pub async fn raw_rpc_call(url: &str, method: &str, params: Value) -> Option<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(5000))
        .build()
        .ok()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await
        .ok()?;
    let json: Value = resp.json().await.ok()?;
    json.get("result").cloned()
}

/// Resolve the RPC URL for a given chain name.
pub fn rpc_url_for_chain(config: &ChainRpcConfig, chain: &str) -> &str {
    match chain {
        "ethereum" => &config.ethereum_rpc,
        "x3" | "x3-chain" => &config.x3_rpc,
        "local" => &config.local_rpc,
        _ => &config.local_rpc,
    }
}

/// Fetch the current block number from an Ethereum-compatible node.
pub async fn fetch_block_number(url: &str) -> Option<u64> {
    let result = raw_rpc_call(url, "eth_blockNumber", serde_json::json!([])).await?;
    let hex = result.as_str()?;
    u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok()
}

/// Fetch a block by number from an Ethereum-compatible node.
pub async fn fetch_block_by_number(url: &str, block_num: u64) -> Option<Value> {
    let hex = format!("0x{:x}", block_num);
    raw_rpc_call(url, "eth_getBlockByNumber", serde_json::json!([hex, false])).await
}

/// Fetch the chain ID from an Ethereum-compatible node.
pub async fn fetch_chain_id(url: &str) -> Option<u64> {
    let result = raw_rpc_call(url, "eth_chainId", serde_json::json!([])).await?;
    let hex = result.as_str()?;
    u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok()
}

/// Fetch peer count from an Ethereum-compatible node.
pub async fn fetch_peer_count(url: &str) -> Option<u64> {
    let result = raw_rpc_call(url, "net_peerCount", serde_json::json!([])).await?;
    let hex = result.as_str()?;
    u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok()
}

/// Fetch gas price from an Ethereum-compatible node.
pub async fn fetch_gas_price(url: &str) -> Option<String> {
    let result = raw_rpc_call(url, "eth_gasPrice", serde_json::json!([])).await?;
    result.as_str().map(|s| format!("{} wei", s))
}