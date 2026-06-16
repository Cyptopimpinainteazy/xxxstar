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
use sp_runtime::traits::UniqueSaturatedInto;
use std::sync::{Arc, Mutex};
use x3_chain_runtime::{opaque::Block, AccountId, AssetId, Balance};
use pallet_x3_kernel::AtlasKernelRuntimeApi;

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

/// Validator RPC implementation.
///
/// Wired to the Substrate client via `ProvideRuntimeApi` and `HeaderBackend`
/// so it queries the live authority set and block data rather than returning
/// hardcoded stubs.
pub struct ValidatorRpc<C, BlockT> {
    client: Arc<C>,
    _block: std::marker::PhantomData<BlockT>,
}

impl<C, BlockT> ValidatorRpc<C, BlockT> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _block: std::marker::PhantomData,
        }
    }
}

impl<C, BlockT> ValidatorRpc<C, BlockT>
where
    BlockT: sp_runtime::traits::Block,
    C: ProvideRuntimeApi<BlockT> + HeaderBackend<BlockT> + BlockBackend<BlockT>,
    C::Api: AtlasKernelRuntimeApi<BlockT, AccountId, Balance, AssetId>,
{
    /// Get the latest block number from the header backend.
    fn best_number(&self) -> u64 {
        self.client.info().best_number.unique_saturated_into()
    }

    /// Fetch the live authority set from the runtime API.
    fn get_authorities(&self) -> Result<Vec<AccountId>, JsonRpseeError> {
        let at = self.client.info().best_hash;
        let api = self.client.runtime_api();
        api.get_authorities(at).map_err(|e| {
            ErrorObjectOwned::owned(
                -32603,
                format!("Failed to query authorities from runtime: {e}"),
                None::<()>,
            )
        })
    }

    /// Query the x3-kernel pallet storage for the authorized executor set.
    /// Falls back to the Aura/GRANDPA authority list if the kernel storage
    /// is empty.
    fn get_authorized_executors(&self) -> Result<Vec<AccountId>, JsonRpseeError> {
        let at = self.client.info().best_hash;
        let api = self.client.runtime_api();
        let authorized = api
            .get_authorized_accounts(at)
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    format!("Failed to query authorized accounts: {e}"),
                    None::<()>,
                )
            })?;

        if authorized.is_empty() {
            // Fall back to the consensus authority set
            return self.get_authorities();
        }
        Ok(authorized)
    }
}

impl<C, BlockT> ValidatorRpcApi for ValidatorRpc<C, BlockT>
where
    BlockT: sp_runtime::traits::Block,
    C: ProvideRuntimeApi<BlockT> + HeaderBackend<BlockT> + BlockBackend<BlockT>,
    C::Api: AtlasKernelRuntimeApi<BlockT, AccountId, Balance, AssetId>,
{
    fn validator_get_validators(&self) -> Result<Vec<ValidatorInfo>, JsonRpseeError> {
        // Use the consensus authority set (get_authorities) for validator
        // identity endpoints.  AuthorizedAccounts (get_authorized_executors)
        // is reserved for executor / RBAC APIs.
        let authorities = self.get_authorities()?;
        let best_number = self.best_number();

        Ok(authorities
            .into_iter()
            .map(|account| {
                let account_str = format!("{:?}", account);
                ValidatorInfo {
                    account_id: account_str,
                    status: ValidatorStatus::Online,
                    // Per-validator performance metrics are not yet
                    // available through the live runtime.  Consumers that
                    // need scored leaderboards should source metrics from
                    // the validator-metrics subsystem once it is wired.
                    score: 0,
                    blocks_produced: 0,
                    blocks_finalized: 0,
                    uptime: 0.0,
                    last_seen: best_number,
                    session_key: None,
                }
            })
            .collect())
    }

    fn validator_get_validator(&self, account_id: String) -> Result<ValidatorInfo, JsonRpseeError> {
        let authorities = self.get_authorities()?;
        let best_number = self.best_number();

        let target = authorities
            .into_iter()
            .find(|a| format!("{:?}", a) == account_id);

        target
            .map(|account| ValidatorInfo {
                account_id: format!("{:?}", account),
                status: ValidatorStatus::Online,
                score: 0,
                blocks_produced: 0,
                blocks_finalized: 0,
                uptime: 0.0,
                last_seen: best_number,
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
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<LeaderboardEntry>, JsonRpseeError> {
        // Leaderboard metrics (tps, latency, gas_efficiency) are not yet
        // available from the live runtime.  Return the authority set with
        // zeroed scores so dashboards can list participants, but do not
        // fabricate synthetic metric values.
        let authorities = self.get_authorities()?;
        let default_limit = limit.unwrap_or(20) as usize;
        let offset_idx = offset.unwrap_or(0) as usize;

        let mut entries: Vec<LeaderboardEntry> = authorities
            .into_iter()
            .enumerate()
            .skip(offset_idx)
            .take(default_limit)
            .map(|(idx, account)| LeaderboardEntry {
                rank: (idx + offset_idx + 1) as u32,
                account_id: format!("{:?}", account),
                score: 0,
                blocks_produced: 0,
                blocks_finalized: 0,
                uptime: 0.0,
                tps: 0.0,
                latency_ms: 0,
                gas_efficiency: 0.0,
            })
            .collect();

        entries.sort_by(|a, b| a.rank.cmp(&b.rank));
        for (i, entry) in entries.iter_mut().enumerate() {
            entry.rank = (i + 1) as u32;
        }

        Ok(entries)
    }

    fn validator_get_metrics(&self) -> Result<MetricsSnapshot, JsonRpseeError> {
        let best_number = self.best_number();
        let authorities = self.get_authorities()?;
        let active_count = authorities.len() as u32;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Per-validator performance metrics (avg_tps, avg_latency_ms,
        // total_gas_used, gas_efficiency_score) are not yet wired from
        // the runtime/metrics subsystem.  The fields are explicitly
        // zeroed rather than populated with synthetic placeholder values.
        Ok(MetricsSnapshot {
            timestamp: now,
            block_height: best_number,
            validator_count: active_count,
            active_validators: active_count,
            avg_tps: 0.0,
            avg_latency_ms: 0,
            total_gas_used: 0,
            gas_efficiency_score: 0.0,
        })
    }

    fn validator_get_stats(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<MetricsSnapshot, JsonRpseeError> {
        let best_number = self.best_number();
        let authorities = self.get_authorities()?;
        let active_count = authorities.len() as u32;

        let block_height = if end_block > best_number {
            best_number
        } else {
            end_block
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Range statistics are not yet computed from on-chain data.
        // Fields are zeroed to avoid fabricating metrics.
        Ok(MetricsSnapshot {
            timestamp: now,
            block_height,
            validator_count: active_count,
            active_validators: active_count,
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

/// Create validator RPC module wired to a Substrate client.
pub fn create_validator_rpc<C, BlockT>(
    client: Arc<C>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    BlockT: sp_runtime::traits::Block + 'static,
    C: ProvideRuntimeApi<BlockT> + HeaderBackend<BlockT> + BlockBackend<BlockT> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<BlockT, AccountId, Balance, AssetId>,
{
    let mut module = RpcModule::new(());
    let validator_rpc = Arc::new(ValidatorRpc::<C, BlockT>::new(client.clone()));

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