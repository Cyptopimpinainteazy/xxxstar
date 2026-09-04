//! Gateway RPC endpoints — query cross-chain gateway state.
//!
//! Provides JSON-RPC methods for reading route configs, transfer status,
//! pending transfers, and withdrawal status from the
//! `pallet-x3-crosschain-gateway` storage.
//!
//! # Wiring
//!
//! ```ignore
//! use x3_rpc::gateway_rpc::create_gateway_rpc;
//! use std::sync::Arc;
//!
//! let rpc = create_gateway_rpc(Arc::new(move |key, _hash| {
//!     client.storage(key, hash)
//!         .map(|opt| opt.map(|d| d.0))
//!         .map_err(|e| e.to_string())
//! }), Arc::new(move |prefix, _hash| {
//!     client.storage_keys(prefix, hash)
//!         .map(|keys| keys.into_iter().map(|k| k.0).collect())
//!         .map_err(|e| e.to_string())
//! }));
//! ```
//!
//! # Endpoints
//!
//! * `gateway_getRouteConfig(route_id: HexStr) → RouteConfigResponse`
//! * `gateway_getPendingTransfers(limit: Option<u32>, offset: Option<u32>) → Vec<TransferSummaryResponse>`
//! * `gateway_getTransferStatus(transfer_id: HexStr) → TransferSummaryResponse`
//! * `gateway_getWithdrawalStatus(withdrawal_id: HexStr) → WithdrawalSummaryResponse`

use codec::Decode;
use jsonrpsee::{types::ErrorObjectOwned, RpcModule};
use sp_core::storage::StorageKey;
use std::sync::Arc;

type JsonRpseeError = ErrorObjectOwned;

/// Storage read callback: `(storage_key, block_hash) -> Result<Option<raw_bytes>, error_msg>`
pub type StorageReadFn =
    Arc<dyn Fn(StorageKey, [u8; 32]) -> Result<Option<Vec<u8>>, String> + Send + Sync>;

/// Storage keys iterator callback: `(prefix, block_hash) -> Result<Vec<raw_keys>, error_msg>`
pub type StorageKeysFn =
    Arc<dyn Fn(StorageKey, [u8; 32]) -> Result<Vec<Vec<u8>>, String> + Send + Sync>;

// ── Storage key helpers ────────────────────────────────────────────────────

fn storage_prefix(pallet: &str, storage: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    let pallet_hash = sp_core::twox_128(pallet.as_bytes());
    let storage_hash = sp_core::twox_128(storage.as_bytes());
    key[..16].copy_from_slice(&pallet_hash);
    key[16..32].copy_from_slice(&storage_hash);
    key
}

fn blake2_128_concat(key: &[u8]) -> Vec<u8> {
    let hash = sp_core::blake2_128(key);
    let mut out = Vec::with_capacity(16 + key.len());
    out.extend_from_slice(&hash);
    out.extend_from_slice(key);
    out
}

fn map_storage_key(pallet: &str, storage: &str, map_key: &[u8]) -> StorageKey {
    let prefix = storage_prefix(pallet, storage);
    let hashed_key = blake2_128_concat(map_key);
    let mut full = Vec::with_capacity(prefix.len() + hashed_key.len());
    full.extend_from_slice(&prefix);
    full.extend_from_slice(&hashed_key);
    StorageKey(full)
}

fn map_storage_prefix(pallet: &str, storage: &str) -> StorageKey {
    StorageKey(storage_prefix(pallet, storage).to_vec())
}

// ── Response types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouteConfigResponse {
    pub route_id: String,
    pub external_chain_id: String,
    pub external_asset_token: String,
    pub x3_asset_id: String,
    pub destination_domain: String,
    pub enabled: bool,
    pub min_amount: String,
    pub max_amount: String,
    pub daily_limit: String,
    pub daily_deposited: String,
    pub pending_limit: u32,
    pub finality_requirement: String,
    pub verification_level: String,
    pub fee_bps: u16,
    pub mode: String,
    pub contract_address: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferSummaryResponse {
    pub transfer_id: String,
    pub route_id: String,
    pub proof_id: String,
    pub x3_asset_id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WithdrawalSummaryResponse {
    pub withdrawal_id: String,
    pub x3_asset_id: String,
    pub destination_chain: String,
    pub recipient: String,
    pub amount: String,
    pub burned: bool,
    pub released: bool,
    pub created_at: String,
}

// ── RPC trait ──────────────────────────────────────────────────────────────

pub trait GatewayRpcApi {
    fn gateway_get_route_config(
        &self,
        route_id: String,
    ) -> Result<RouteConfigResponse, JsonRpseeError>;
    fn gateway_get_pending_transfers(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<TransferSummaryResponse>, JsonRpseeError>;
    fn gateway_get_transfer_status(
        &self,
        transfer_id: String,
    ) -> Result<TransferSummaryResponse, JsonRpseeError>;
    fn gateway_get_withdrawal_status(
        &self,
        withdrawal_id: String,
    ) -> Result<WithdrawalSummaryResponse, JsonRpseeError>;
}

// ── RPC implementation ─────────────────────────────────────────────────────

pub struct GatewayRpc {
    read_storage: StorageReadFn,
    read_keys: StorageKeysFn,
    best_hash: [u8; 32],
}

impl GatewayRpc {
    pub fn new(read_storage: StorageReadFn, read_keys: StorageKeysFn, best_hash: [u8; 32]) -> Self {
        Self {
            read_storage,
            read_keys,
            best_hash,
        }
    }

    fn get_storage<T: Decode>(&self, key: &StorageKey) -> Result<T, JsonRpseeError> {
        let data = (self.read_storage)(key.clone(), self.best_hash)
            .map_err(|e| {
                ErrorObjectOwned::owned(-32603, format!("storage read failed: {e}"), None::<()>)
            })?
            .ok_or_else(|| {
                ErrorObjectOwned::owned(-32603, "entry not found".to_string(), None::<()>)
            })?;
        T::decode(&mut &data[..])
            .map_err(|e| ErrorObjectOwned::owned(-32603, format!("decode failed: {e}"), None::<()>))
    }
}

const PALETT_PREFIX: &str = "X3CrosschainGateway";

impl GatewayRpcApi for GatewayRpc {
    fn gateway_get_route_config(
        &self,
        route_id: String,
    ) -> Result<RouteConfigResponse, JsonRpseeError> {
        let route_id_bytes = hex_decode(&route_id).map_err(|e| {
            ErrorObjectOwned::owned(-32602, format!("invalid route_id hex: {e}"), None::<()>)
        })?;

        let storage_key = map_storage_key(PALETT_PREFIX, "Routes", &route_id_bytes);
        let config: pallet_x3_crosschain_gateway::RouteConfig = self.get_storage(&storage_key)?;

        Ok(RouteConfigResponse {
            route_id: hex_encode(&config.route_id),
            external_chain_id: format!("{:?}", config.external_chain_id),
            external_asset_token: hex_encode(&config.external_asset.token_address_or_mint),
            x3_asset_id: hex_encode(&config.x3_asset_id),
            destination_domain: format!("{:?}", config.destination_domain),
            enabled: config.enabled,
            min_amount: config.min_amount.to_string(),
            max_amount: config.max_amount.to_string(),
            daily_limit: config.daily_limit.to_string(),
            daily_deposited: config.daily_deposited.to_string(),
            pending_limit: config.pending_limit,
            finality_requirement: config.finality_requirement.to_string(),
            verification_level: format!("{:?}", config.verification_level),
            fee_bps: config.fee_bps,
            mode: format!("{:?}", config.mode),
            contract_address: hex_encode(&config.contract_address),
        })
    }

    fn gateway_get_pending_transfers(
        &self,
        limit: Option<u32>,
        _offset: Option<u32>,
    ) -> Result<Vec<TransferSummaryResponse>, JsonRpseeError> {
        let prefix = map_storage_prefix(PALETT_PREFIX, "Transfers");
        let keys = (self.read_keys)(prefix, self.best_hash).map_err(|e| {
            ErrorObjectOwned::owned(
                -32603,
                format!("storage_keys query failed: {e}"),
                None::<()>,
            )
        })?;

        let max = limit.unwrap_or(20).min(100) as usize;
        let mut results = Vec::with_capacity(max.min(keys.len()));

        for raw_key in keys.iter().take(max) {
            let storage_key = StorageKey(raw_key.clone());
            let data = (self.read_storage)(storage_key, self.best_hash)
                .map_err(|e| {
                    ErrorObjectOwned::owned(-32603, format!("storage read failed: {e}"), None::<()>)
                })?
                .unwrap_or_default();
            if !data.is_empty() {
                if let Ok(transfer) =
                    pallet_x3_crosschain_gateway::GatewayTransfer::decode(&mut &data[..])
                {
                    results.push(TransferSummaryResponse {
                        transfer_id: hex_encode(&transfer.transfer_id),
                        route_id: hex_encode(&transfer.route_id),
                        proof_id: hex_encode(&transfer.proof_id),
                        x3_asset_id: hex_encode(&transfer.x3_asset_id),
                        sender: hex_encode(&transfer.sender),
                        recipient: hex_encode(&transfer.recipient),
                        amount: transfer.amount.to_string(),
                        status: format!("{:?}", transfer.status),
                        created_at: transfer.created_at.to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    fn gateway_get_transfer_status(
        &self,
        transfer_id: String,
    ) -> Result<TransferSummaryResponse, JsonRpseeError> {
        let tid = hex_decode(&transfer_id).map_err(|e| {
            ErrorObjectOwned::owned(-32602, format!("invalid transfer_id hex: {e}"), None::<()>)
        })?;
        let storage_key = map_storage_key(PALETT_PREFIX, "Transfers", &tid);
        let transfer: pallet_x3_crosschain_gateway::GatewayTransfer =
            self.get_storage(&storage_key)?;
        Ok(TransferSummaryResponse {
            transfer_id: hex_encode(&transfer.transfer_id),
            route_id: hex_encode(&transfer.route_id),
            proof_id: hex_encode(&transfer.proof_id),
            x3_asset_id: hex_encode(&transfer.x3_asset_id),
            sender: hex_encode(&transfer.sender),
            recipient: hex_encode(&transfer.recipient),
            amount: transfer.amount.to_string(),
            status: format!("{:?}", transfer.status),
            created_at: transfer.created_at.to_string(),
        })
    }

    fn gateway_get_withdrawal_status(
        &self,
        withdrawal_id: String,
    ) -> Result<WithdrawalSummaryResponse, JsonRpseeError> {
        let wid = hex_decode(&withdrawal_id).map_err(|e| {
            ErrorObjectOwned::owned(
                -32602,
                format!("invalid withdrawal_id hex: {e}"),
                None::<()>,
            )
        })?;
        let storage_key = map_storage_key(PALETT_PREFIX, "Withdrawals", &wid);
        let withdrawal: pallet_x3_crosschain_gateway::WithdrawalRecord =
            self.get_storage(&storage_key)?;
        Ok(WithdrawalSummaryResponse {
            withdrawal_id: hex_encode(&withdrawal.withdrawal_id),
            x3_asset_id: hex_encode(&withdrawal.x3_asset_id),
            destination_chain: format!("{:?}", withdrawal.destination_chain),
            recipient: hex_encode(&withdrawal.recipient),
            amount: withdrawal.amount.to_string(),
            burned: withdrawal.burned,
            released: withdrawal.released,
            created_at: withdrawal.created_at.to_string(),
        })
    }
}

// ── Module factory ─────────────────────────────────────────────────────────

/// Create a gateway RPC module.
pub fn create_gateway_rpc(
    read_storage: StorageReadFn,
    read_keys: StorageKeysFn,
    best_hash: [u8; 32],
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>> {
    let mut module = RpcModule::new(());
    let gw = Arc::new(GatewayRpc::new(read_storage, read_keys, best_hash));

    {
        let gw = gw.clone();
        module.register_method("gateway_getRouteConfig", move |params, _, _| {
            let (route_id,): (String,) = params.parse()?;
            gw.gateway_get_route_config(route_id)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    {
        let gw = gw.clone();
        module.register_method("gateway_getPendingTransfers", move |params, _, _| {
            let (limit, offset): (Option<u32>, Option<u32>) =
                params.parse().unwrap_or((None, None));
            gw.gateway_get_pending_transfers(limit, offset)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    {
        let gw = gw.clone();
        module.register_method("gateway_getTransferStatus", move |params, _, _| {
            let (transfer_id,): (String,) = params.parse()?;
            gw.gateway_get_transfer_status(transfer_id)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    {
        let gw = gw.clone();
        module.register_method("gateway_getWithdrawalStatus", move |params, _, _| {
            let (withdrawal_id,): (String,) = params.parse()?;
            gw.gateway_get_withdrawal_status(withdrawal_id)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
        })?;
    }

    Ok(module)
}

// ── Hex helpers ────────────────────────────────────────────────────────────

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).map_err(|e| format!("hex decode error: {e}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_prefix_length() {
        let p = storage_prefix("X3CrosschainGateway", "Routes");
        assert_eq!(p.len(), 32);
    }

    #[test]
    fn test_blake2_128_concat_length() {
        let key = [1u8; 32];
        let result = blake2_128_concat(&key);
        assert_eq!(result.len(), 16 + 32);
        assert_eq!(&result[16..], &key);
    }

    #[test]
    fn test_hex_roundtrip() {
        let input = [0xdeu8, 0xad, 0xbe, 0xef];
        let encoded = hex_encode(&input);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_hex_decode_strips_prefix() {
        let input = "0xdeadbeef";
        let decoded = hex_decode(input).unwrap();
        assert_eq!(decoded, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_hex_decode_no_prefix() {
        let input = "deadbeef";
        let decoded = hex_decode(input).unwrap();
        assert_eq!(decoded, vec![0xde, 0xad, 0xbe, 0xef]);
    }
}
