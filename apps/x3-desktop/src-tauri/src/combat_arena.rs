use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/* ─── Agent State ─────────────────────────────── */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatAgent {
    pub id: String,
    pub name: String,
    pub health: f64,
    pub pnl: f64,
    pub xp: u64,
    pub color: String,
    pub entity_type: String,
    pub position: AgentPosition,
    pub status: String,
    pub last_action: String,
    pub strategy_id: String,
    pub chain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEvent {
    pub id: String,
    pub height: u64,
    pub agent_id: String,
    pub status: String,
    pub timestamp: u64,
    pub position: BlockPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPosition {
    pub x: f64,
    pub z: f64,
}

/* ─── Shared State ─────────────────────────────── */

pub struct CombatArenaState {
    pub agents: Arc<RwLock<Vec<CombatAgent>>>,
    pub blocks: Arc<RwLock<Vec<BlockEvent>>>,
    pub running: Arc<RwLock<bool>>,
}

impl CombatArenaState {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(seed_agents())),
            blocks: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }
}

fn seed_agents() -> Vec<CombatAgent> {
    vec![
        CombatAgent {
            id: "agent-1".into(),
            name: "Alpha Arbitrage".into(),
            health: 100.0,
            pnl: 24580.50,
            xp: 1500,
            color: "#00ccff".into(),
            entity_type: "diamond".into(),
            position: AgentPosition { x: -4.0, y: 0.0, z: 0.0 },
            status: "idle".into(),
            last_action: "Flashloan swap on Uniswap V3".into(),
            strategy_id: "strat-arb-1".into(),
            chain: "ethereum".into(),
        },
        CombatAgent {
            id: "agent-2".into(),
            name: "Gamma Yield".into(),
            health: 100.0,
            pnl: 12340.75,
            xp: 1100,
            color: "#00ff88".into(),
            entity_type: "sphere".into(),
            position: AgentPosition { x: 4.0, y: 0.0, z: 0.0 },
            status: "idle".into(),
            last_action: "Harvested x3LP rewards".into(),
            strategy_id: "strat-yield-1".into(),
            chain: "ethereum".into(),
        },
        CombatAgent {
            id: "agent-3".into(),
            name: "Delta Hedge".into(),
            health: 100.0,
            pnl: 8920.30,
            xp: 800,
            color: "#ff8800".into(),
            entity_type: "icosahedron".into(),
            position: AgentPosition { x: 0.0, y: 0.0, z: -4.0 },
            status: "idle".into(),
            last_action: "Opened GLP short position".into(),
            strategy_id: "strat-hedge-1".into(),
            chain: "arbitrum".into(),
        },
        CombatAgent {
            id: "agent-4".into(),
            name: "Omega MEV".into(),
            health: 100.0,
            pnl: 45230.00,
            xp: 3200,
            color: "#ff3366".into(),
            entity_type: "torus".into(),
            position: AgentPosition { x: 0.0, y: 0.0, z: 4.0 },
            status: "idle".into(),
            last_action: "Sandwich attack on GMX".into(),
            strategy_id: "strat-mev-1".into(),
            chain: "ethereum".into(),
        },
        CombatAgent {
            id: "agent-5".into(),
            name: "Sigma LPs".into(),
            health: 100.0,
            pnl: 6730.15,
            xp: 600,
            color: "#aa66ff".into(),
            entity_type: "cylinder".into(),
            position: AgentPosition { x: -6.0, y: 0.0, z: -4.0 },
            status: "idle".into(),
            last_action: "Added liquidity to Curve tricrypto".into(),
            strategy_id: "strat-lp-1".into(),
            chain: "polygon".into(),
        },
        CombatAgent {
            id: "agent-6".into(),
            name: "Zeta Governance".into(),
            health: 100.0,
            pnl: 1520.00,
            xp: 400,
            color: "#ffff44".into(),
            entity_type: "cone".into(),
            position: AgentPosition { x: 6.0, y: 0.0, z: -4.0 },
            status: "idle".into(),
            last_action: "Voted on AIP-42 proposal".into(),
            strategy_id: "strat-gov-1".into(),
            chain: "ethereum".into(),
        },
    ]
}

/* ─── Tauri Commands ──────────────────────────── */

use tauri::State;

#[tauri::command]
pub fn get_arena_agents(state: State<'_, CombatArenaState>) -> Result<Vec<CombatAgent>, String> {
    state.agents.read().map(|a| a.clone()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_arena_blocks(state: State<'_, CombatArenaState>) -> Result<Vec<BlockEvent>, String> {
    state.blocks.read().map(|b| b.clone()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_arena_ai(state: State<'_, CombatArenaState>) -> Result<bool, String> {
    let mut running = state.running.write().map_err(|e| e.to_string())?;
    *running = true;
    Ok(true)
}

#[tauri::command]
pub fn stop_arena_ai(state: State<'_, CombatArenaState>) -> Result<bool, String> {
    let mut running = state.running.write().map_err(|e| e.to_string())?;
    *running = false;
    Ok(false)
}

#[tauri::command]
pub fn get_arena_status(state: State<'_, CombatArenaState>) -> Result<bool, String> {
    state.running.read().map(|r| *r).map_err(|e| e.to_string())
}

use crate::chain_rpc;

/// Attempt a real JSON-RPC call. Returns an error if the node is unreachable.
async fn try_rpc_or_err<F, T>(rpc_fn: F) -> Result<T, String>
where
    F: std::future::Future<Output = Option<T>>,
{
    rpc_fn.await.ok_or_else(|| "RPC call failed: node unreachable or returned no result".to_string())
}

#[tauri::command]
pub async fn connect_chain(chain: String) -> Result<String, String> {
    let config = chain_rpc::ChainRpcConfig::default();
    let url = chain_rpc::rpc_url_for_chain(&config, &chain);
    // Try a real RPC call to verify connectivity
    match chain_rpc::fetch_chain_id(url).await {
        Some(_) => Ok("ok".to_string()),
        None => Err(format!(
            "Cannot connect to chain \"{chain}\": RPC endpoint at {url} is unreachable"
        )),
    }
}

#[tauri::command]
pub async fn disconnect_chain(chain: String) -> Result<String, String> {
    Ok("ok".to_string())
}

#[tauri::command]
pub async fn fetch_chain_status(chain: String) -> Result<serde_json::Value, String> {
    let config = chain_rpc::ChainRpcConfig::default();
    let url = chain_rpc::rpc_url_for_chain(&config, &chain);

    // Try live RPC
    if let (Some(block_number), Some(chain_id), Some(peer_count)) = (
        chain_rpc::fetch_block_number(url).await,
        chain_rpc::fetch_chain_id(url).await,
        chain_rpc::fetch_peer_count(url).await,
    ) {
        return Ok(serde_json::json!({
            "chainId": chain_id,
            "blockHeight": block_number,
            "peers": peer_count,
            "synced": true,
            "avgBlockTimeMs": 12000
        }));
    }

    // RPC unreachable — return a clear error
    Err(format!(
        "Cannot fetch chain status for \"{chain}\": RPC endpoint at {url} is unreachable or returned no data"
    ))
}

#[tauri::command]
pub async fn fetch_blocks(chain: String, limit: u32) -> Result<Vec<serde_json::Value>, String> {
    let config = chain_rpc::ChainRpcConfig::default();
    let url = chain_rpc::rpc_url_for_chain(&config, &chain);

    // Try real RPC — fetch block number then walk backwards
    if let Some(latest) = chain_rpc::fetch_block_number(url).await {
        let mut blocks = Vec::new();
        for i in 0..limit.min(10) {
            let block_num = latest.saturating_sub(i as u64);
            if let Some(block) = chain_rpc::fetch_block_by_number(url, block_num).await {
                let hash = block.get("hash").and_then(|h| h.as_str()).unwrap_or("0x0");
                let timestamp_hex = block.get("timestamp").and_then(|t| t.as_str()).unwrap_or("0x0");
                let timestamp = u64::from_str_radix(timestamp_hex.trim_start_matches("0x"), 16).unwrap_or(0);
                let tx_count = block.get("transactions").and_then(|t| t.as_array()).map(|a| a.len()).unwrap_or(0);
                let state_root = block.get("stateRoot").and_then(|s| s.as_str()).unwrap_or("0x0");

                blocks.push(serde_json::json!({
                    "hash": hash,
                    "height": block_num,
                    "timestamp": timestamp * 1000,
                    "txCount": tx_count,
                    "stateRoot": state_root,
                }));
            }
        }
        if !blocks.is_empty() {
            return Ok(blocks);
        }
    }

    // RPC unreachable — return a clear error
    Err(format!(
        "Cannot fetch blocks for chain \"{chain}\": RPC endpoint at {url} is unreachable"
    ))
}

#[tauri::command]
pub async fn fetch_mempool(chain: String) -> Result<Vec<serde_json::Value>, String> {
    let config = chain_rpc::ChainRpcConfig::default();
    let url = chain_rpc::rpc_url_for_chain(&config, &chain);

    // Try real RPC — eth_getBlockByNumber with "pending" flag
    if let Some(pending_block) = chain_rpc::raw_rpc_call(url, "eth_getBlockByNumber", serde_json::json!(["pending", true])).await {
        if let Some(txs) = pending_block.get("transactions").and_then(|t| t.as_array()) {
            if !txs.is_empty() {
                let mempool: Vec<serde_json::Value> = txs.iter().take(20).map(|tx| {
                    serde_json::json!({
                        "hash": tx.get("hash").and_then(|h| h.as_str()).unwrap_or("0x0"),
                        "blockHeight": 0,
                        "from": tx.get("from").and_then(|f| f.as_str()).unwrap_or("0x0"),
                        "to": tx.get("to").and_then(|t| t.as_str()).unwrap_or("0x0"),
                        "value": tx.get("value").and_then(|v| v.as_str()).unwrap_or("0x0"),
                        "status": "pending",
                        "timestamp": chrono::Utc::now().timestamp_millis(),
                    })
                }).collect();
                return Ok(mempool);
            }
        }
    }

    // RPC unreachable — return a clear error
    Err(format!(
        "Cannot fetch mempool for chain \"{chain}\": RPC endpoint at {url} is unreachable"
    ))
}

#[tauri::command]
pub async fn sign_and_send_tx(chain: String, raw_tx: String) -> Result<String, String> {
    // Try to send via real RPC first
    let config = chain_rpc::ChainRpcConfig::default();
    let url = chain_rpc::rpc_url_for_chain(&config, &chain);
    if let Some(result) = chain_rpc::raw_rpc_call(url, "eth_sendRawTransaction", serde_json::json!([raw_tx])).await {
        if let Some(tx_hash) = result.as_str() {
            return Ok(tx_hash.to_string());
        }
    }
    // RPC unreachable — return a clear error
    Err(format!(
        "Cannot send transaction on \"{chain}\": RPC endpoint at {url} is unreachable"
    ))
}

#[tauri::command]
pub async fn get_balance(chain: String, address: String) -> Result<String, String> {
    let config = chain_rpc::ChainRpcConfig::default();
    let url = chain_rpc::rpc_url_for_chain(&config, &chain);
    if let Some(result) = chain_rpc::raw_rpc_call(url, "eth_getBalance", serde_json::json!([address, "latest"])).await {
        if let Some(balance_hex) = result.as_str() {
            let wei = u128::from_str_radix(balance_hex.trim_start_matches("0x"), 16).unwrap_or(0);
            let eth = wei as f64 / 1e18;
            return Ok(format!("{:.4} ETH", eth));
        }
    }
    // RPC unreachable — return a clear error
    Err(format!(
        "Cannot fetch balance on \"{chain}\": RPC endpoint at {url} is unreachable"
    ))
}
