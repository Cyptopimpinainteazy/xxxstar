/// Chain connection management.
///
/// `ChainManager` holds concurrent connections to multiple chains
/// (Substrate / EVM / local dev) behind a `tokio::sync::RwLock`.
/// Reconnection uses exponential backoff with jitter.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::error::AppError;

// ---------------------------------------------------------------------------
// Shared types (mirroring TypeScript types/chain.ts)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub chain_id: String,
    pub network_phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatus {
    pub connected: bool,
    pub block_height: u64,
    pub peer_count: u32,
    pub sync_state: String,
    pub latency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub hash: String,
    pub parent_hash: String,
    pub number: u64,
    pub timestamp: u64,
    pub extrinsic_count: u32,
    pub event_count: u32,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningIntent {
    pub chain_id: String,
    pub account_id: String,
    pub module: String,
    pub method: String,
    pub args: Vec<serde_json::Value>,
    pub tip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyageState {
    pub schema_version: u32,
    pub game_state: serde_json::Value,
    pub economy: serde_json::Value,
    pub ship: serde_json::Value,
    pub saved_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneExport {
    pub format: String,
    pub scene_json: String,
    pub include_textures: bool,
}

// ---------------------------------------------------------------------------
// Connection state per chain
// ---------------------------------------------------------------------------

struct ChainConnection {
    config: ChainConfig,
    status: ChainStatus,
    block_height: u64,
    reconnect_attempts: u32,
}

impl ChainConnection {
    fn new(config: ChainConfig) -> Self {
        Self {
            config,
            status: ChainStatus {
                connected: false,
                block_height: 0,
                peer_count: 0,
                sync_state: "offline".into(),
                latency: 0.0,
            },
            block_height: 0,
            reconnect_attempts: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// ChainManager
// ---------------------------------------------------------------------------

/// Manages connections to multiple blockchains concurrently.
pub struct ChainManager {
    connections: Arc<RwLock<HashMap<String, ChainConnection>>>,
}

impl ChainManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to a chain. For now, simulates connection establishment.
    /// In production, this opens WebSocket (Substrate) or JSON-RPC (EVM).
    pub async fn connect(&self, chain_id: &str, config: ChainConfig) -> Result<ChainStatus, AppError> {
        info!(chain_id, rpc_url = %config.rpc_url, "connecting to chain");

        let mut conn = ChainConnection::new(config);
        // Simulate successful connection
        conn.status = ChainStatus {
            connected: true,
            block_height: 1,
            peer_count: 3,
            sync_state: "synced".into(),
            latency: 45.0,
        };
        conn.reconnect_attempts = 0;

        let status = conn.status.clone();
        let mut conns = self.connections.write().await;
        conns.insert(chain_id.to_string(), conn);

        info!(chain_id, "chain connected");
        Ok(status)
    }

    /// Disconnect from a chain.
    pub async fn disconnect(&self, chain_id: &str) -> Result<(), AppError> {
        let mut conns = self.connections.write().await;
        if conns.remove(chain_id).is_none() {
            return Err(AppError::NotFound(format!("chain not connected: {chain_id}")));
        }
        info!(chain_id, "chain disconnected");
        Ok(())
    }

    /// Get current status for a connected chain.
    pub async fn get_status(&self, chain_id: &str) -> Result<ChainStatus, AppError> {
        let conns = self.connections.read().await;
        conns
            .get(chain_id)
            .map(|c| c.status.clone())
            .ok_or_else(|| AppError::NotFound(format!("chain not connected: {chain_id}")))
    }

    /// Fetch blocks from a chain. Returns mock blocks for the scaffold.
    pub async fn fetch_blocks(
        &self,
        chain_id: &str,
        from_height: u64,
        limit: u32,
    ) -> Result<Vec<Block>, AppError> {
        let conns = self.connections.read().await;
        if !conns.contains_key(chain_id) {
            return Err(AppError::NotFound(format!("chain not connected: {chain_id}")));
        }

        let blocks: Vec<Block> = (0..limit)
            .map(|i| {
                let num = from_height + i as u64;
                Block {
                    hash: format!("0x{:064x}", num),
                    parent_hash: format!("0x{:064x}", num.saturating_sub(1)),
                    number: num,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    extrinsic_count: (num % 20) as u32,
                    event_count: (num % 50) as u32,
                    author: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY".into(),
                }
            })
            .collect();

        Ok(blocks)
    }

    /// Attempt reconnection with exponential backoff.
    pub async fn try_reconnect(&self, chain_id: &str) -> Result<ChainStatus, AppError> {
        let mut conns = self.connections.write().await;
        let conn = conns
            .get_mut(chain_id)
            .ok_or_else(|| AppError::NotFound(format!("chain unknown: {chain_id}")))?;

        conn.reconnect_attempts += 1;
        let delay_secs = Self::backoff_delay(conn.reconnect_attempts);
        warn!(chain_id, attempt = conn.reconnect_attempts, delay_secs, "reconnecting");

        // In production, attempt actual reconnection here.
        conn.status.connected = true;
        conn.status.sync_state = "syncing".into();
        conn.reconnect_attempts = 0;

        Ok(conn.status.clone())
    }

    /// Exponential backoff: min(initial * 2^attempts, max) + jitter.
    fn backoff_delay(attempts: u32) -> f64 {
        let base: f64 = 1.0 * 2.0_f64.powi(attempts as i32);
        let capped = base.min(60.0);
        let jitter = rand::random::<f64>() * 0.3 * capped;
        capped + jitter
    }
}

impl Default for ChainManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_and_disconnect() {
        let mgr = ChainManager::new();
        let cfg = ChainConfig {
            rpc_url: "ws://127.0.0.1:9944".into(),
            chain_id: "atlas-dev".into(),
            network_phase: "devnet".into(),
        };
        let status = mgr.connect("atlas-dev", cfg).await.unwrap();
        assert!(status.connected);

        mgr.disconnect("atlas-dev").await.unwrap();
        assert!(mgr.get_status("atlas-dev").await.is_err());
    }

    #[test]
    fn backoff_bounded() {
        for i in 0..20 {
            let d = ChainManager::backoff_delay(i);
            assert!(d <= 80.0, "backoff exceeded ceiling: {d}");
        }
    }
}
