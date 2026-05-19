#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    non_snake_case,
    unexpected_cfgs,
    unused_parens,
    non_camel_case_types,
    deprecated,
    clippy::all
)]

//! X3 Validator RPC Server
//!
//! JSON-RPC endpoints for validator management, leaderboard queries, and metrics collection.

use jsonrpsee::{types::ErrorObjectOwned, RpcModule};
type JsonRpseeError = ErrorObjectOwned;
use sc_client_api::BlockBackend;
use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use std::sync::{Arc, Mutex};
use x3_chain_runtime::{opaque::Block, AccountId, AssetId, Balance};

/// Validator status enum
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidatorStatus {
    Online,
    Syncing,
    Offline,
    Inactive,
}

/// Validator information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidatorInfo {
    pub account_id: String,
    pub status: ValidatorStatus,
    pub score: u64,
    pub blocks_produced: u64,
    pub blocks_finalized: u64,
    pub uptime: f64,
    pub last_seen: u64,
    pub session_key: Option<String>,
}

/// Leaderboard entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub account_id: String,
    pub score: u64,
    pub blocks_produced: u64,
    pub blocks_finalized: u64,
    pub uptime: f64,
    pub tps: f64,
    pub latency_ms: u64,
    pub gas_efficiency: f64,
}

/// Metrics snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub block_height: u64,
    pub validator_count: u32,
    pub active_validators: u32,
    pub avg_tps: f64,
    pub avg_latency_ms: u64,
    pub total_gas_used: u64,
    pub gas_efficiency_score: f64,
}

/// Validator RPC API
pub trait ValidatorRpcApi {
    /// Get current validator set
    fn validator_get_validators(&self) -> Result<Vec<ValidatorInfo>, JsonRpseeError>;

    /// Get validator by account ID
    fn validator_get_validator(&self, account_id: String) -> Result<ValidatorInfo, JsonRpseeError>;

    /// Get leaderboard with optional filters
    fn validator_get_leaderboard(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<LeaderboardEntry>, JsonRpseeError>;

    /// Get metrics snapshot
    fn validator_get_metrics(&self) -> Result<MetricsSnapshot, JsonRpseeError>;

    /// Get validator stats for a specific range
    fn validator_get_stats(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<MetricsSnapshot, JsonRpseeError>;
}

/// Validator RPC implementation
pub struct ValidatorRpc;

impl ValidatorRpc {
    pub fn new() -> Self {
        Self
    }

    /// Get current authorities from runtime (stub)
    fn get_authorities(&self) -> Result<Vec<AccountId>, JsonRpseeError> {
        Ok(vec![])
    }
}

impl ValidatorRpcApi for ValidatorRpc {
    fn validator_get_validators(&self) -> Result<Vec<ValidatorInfo>, JsonRpseeError> {
        let authorities = self.get_authorities()?;

        // TODO: Fetch actual validator status from chain state
        // For now, return all authorities as online with mock data
        Ok(authorities
            .into_iter()
            .map(|account| ValidatorInfo {
                account_id: format!("{:?}", account),
                status: ValidatorStatus::Online,
                score: 100,
                blocks_produced: 0,
                blocks_finalized: 0,
                uptime: 100.0,
                last_seen: 0,
                session_key: None,
            })
            .collect())
    }

    fn validator_get_validator(&self, account_id: String) -> Result<ValidatorInfo, JsonRpseeError> {
        let authorities = self.get_authorities()?;

        // Parse account ID (simplified - in production would use proper decoding)
        let target_account = authorities
            .into_iter()
            .find(|a| format!("{:?}", a) == account_id);

        target_account
            .map(|account| ValidatorInfo {
                account_id: format!("{:?}", account),
                status: ValidatorStatus::Online,
                score: 100,
                blocks_produced: 0,
                blocks_finalized: 0,
                uptime: 100.0,
                last_seen: 0,
                session_key: None,
            })
            .ok_or_else(|| {
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Validator not found: {}", account_id),
                    None::<()>,
                )
            })
    }

    fn validator_get_leaderboard(
        &self,
        _limit: Option<u32>,
        _offset: Option<u32>,
    ) -> Result<Vec<LeaderboardEntry>, JsonRpseeError> {
        // TODO: Implement leaderboard query from pallet-x3-kernel or custom storage
        // For now, return mock data
        Ok(vec![
            LeaderboardEntry {
                rank: 1,
                account_id: "0x1234...".to_string(),
                score: 1000,
                blocks_produced: 100,
                blocks_finalized: 99,
                uptime: 99.9,
                tps: 1000.0,
                latency_ms: 100,
                gas_efficiency: 0.95,
            },
            LeaderboardEntry {
                rank: 2,
                account_id: "0x5678...".to_string(),
                score: 950,
                blocks_produced: 95,
                blocks_finalized: 94,
                uptime: 99.5,
                tps: 950.0,
                latency_ms: 110,
                gas_efficiency: 0.92,
            },
        ])
    }

    fn validator_get_metrics(&self) -> Result<MetricsSnapshot, JsonRpseeError> {
        // TODO: Fetch actual metrics from chain state
        Ok(MetricsSnapshot {
            timestamp: 0,
            block_height: 0,
            validator_count: 0,
            active_validators: 0,
            avg_tps: 0.0,
            avg_latency_ms: 0,
            total_gas_used: 0,
            gas_efficiency_score: 0.0,
        })
    }

    fn validator_get_stats(
        &self,
        _start_block: u64,
        _end_block: u64,
    ) -> Result<MetricsSnapshot, JsonRpseeError> {
        // TODO: Fetch metrics for specific block range
        Ok(MetricsSnapshot {
            timestamp: 0,
            block_height: 0,
            validator_count: 0,
            active_validators: 0,
            avg_tps: 0.0,
            avg_latency_ms: 0,
            total_gas_used: 0,
            gas_efficiency_score: 0.0,
        })
    }
}

fn err_to_rpc<E: std::fmt::Display>(e: E) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
}

/// Create validator RPC module
pub fn create_validator_rpc(
    _client: Arc<dyn std::any::Any + Send + Sync>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>> {
    let mut module = RpcModule::new(());
    let validator_rpc = std::sync::Arc::new(ValidatorRpc::new());

    {
        let vr = validator_rpc.clone();
        module.register_method("validator_getValidators", move |_, _, _| {
            vr.validator_get_validators()
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    {
        let vr = validator_rpc.clone();
        module.register_method("validator_getValidator", move |params, _, _| {
            let account_id: String = params.parse::<(String,)>().map(|(s,)| s)?;
            vr.validator_get_validator(account_id)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    {
        let vr = validator_rpc.clone();
        module.register_method("validator_getLeaderboard", move |params, _, _| {
            let (limit, offset): (Option<u32>, Option<u32>) =
                params.parse().unwrap_or((None, None));
            vr.validator_get_leaderboard(limit, offset)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    {
        let vr = validator_rpc.clone();
        module.register_method("validator_getMetrics", move |_, _, _| {
            vr.validator_get_metrics()
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    {
        let vr = validator_rpc.clone();
        module.register_method("validator_getStats", move |params, _, _| {
            let (start_block, end_block): (u64, u64) = params.parse()?;
            vr.validator_get_stats(start_block, end_block)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    Ok(module)
}
