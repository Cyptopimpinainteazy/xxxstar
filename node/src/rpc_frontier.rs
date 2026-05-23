//! Runtime-backed Frontier and SVM RPC wiring.
//!
//! This module provides optional Ethereum-compatible and SVM-compatible
//! JSON-RPC endpoints backed by runtime API calls.
//! When `feature = "frontier"` is enabled for the node crate, this module
//! will create and merge additional Ethereum-compatible RPC handlers. These
//! can later be extended with `fc-rpc`/`fp-rpc` once the Frontier version
//! compatibility is resolved.

use hex;
use jsonrpsee::RpcModule;
use pallet_x3_kernel::AtlasKernelRuntimeApi;
use sc_client_api::BlockBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::DigestItem;
use std::sync::Arc;
use x3_chain_runtime::{opaque::Block, AccountId, AssetId, Balance};
use x3_common::{
    signing::{Ed25519Signer, KeyType, Secp256k1Signer, Signer, Sr25519Signer},
    weight_metering::{ComputeMeter, GasMeter, Operation, WeightConfig, WeightMeter},
};

/// Decode a SVM pubkey from either a 0x-prefixed hex string (32 bytes) or
/// a base58-encoded Solana-style pubkey.
fn decode_svm_pubkey(s: &str) -> Result<Vec<u8>, jsonrpsee::types::ErrorObjectOwned> {
    if let Some(hex_str) = s.strip_prefix("0x") {
        let bytes = hex::decode(hex_str).map_err(|e| {
            jsonrpsee::types::ErrorObjectOwned::owned(
                -32603,
                format!("Invalid hex pubkey: {}", e),
                None::<()>,
            )
        })?;
        if bytes.len() != 32 {
            return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                -32603,
                "SVM pubkey must be 32 bytes".to_string(),
                None::<()>,
            ));
        }
        return Ok(bytes);
    }
    // base58 decode
    let bytes = bs58::decode(s).into_vec().map_err(|e| {
        jsonrpsee::types::ErrorObjectOwned::owned(
            -32603,
            format!("Invalid base58 pubkey: {}", e),
            None::<()>,
        )
    })?;
    if bytes.len() != 32 {
        return Err(jsonrpsee::types::ErrorObjectOwned::owned(
            -32603,
            "SVM pubkey must be 32 bytes".to_string(),
            None::<()>,
        ));
    }
    Ok(bytes)
}

/// Helper: decode a hex EVM address string to a 20-byte Vec
fn decode_address(s: &str) -> Result<Vec<u8>, jsonrpsee::types::ErrorObjectOwned> {
    let stripped = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(stripped).map_err(|e| {
        jsonrpsee::types::ErrorObjectOwned::owned(
            -32603,
            format!("Invalid address: {}", e),
            None::<()>,
        )
    })?;
    if bytes.len() != 20 {
        return Err(jsonrpsee::types::ErrorObjectOwned::owned(
            -32603,
            "Address must be 20 bytes".to_string(),
            None::<()>,
        ));
    }
    Ok(bytes)
}

fn parse_gas_limit(tx_obj: &serde_json::Value) -> Result<u64, jsonrpsee::types::ErrorObjectOwned> {
    let Some(raw_gas) = tx_obj.get("gas") else {
        return Ok(10_000_000);
    };

    if let Some(gas_u64) = raw_gas.as_u64() {
        return Ok(gas_u64);
    }

    if let Some(gas_str) = raw_gas.as_str() {
        if let Some(stripped) = gas_str.strip_prefix("0x") {
            return u64::from_str_radix(stripped, 16).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid gas: {}", e),
                    None::<()>,
                )
            });
        }

        return gas_str.parse::<u64>().map_err(|e| {
            jsonrpsee::types::ErrorObjectOwned::owned(
                -32603,
                format!("Invalid gas: {}", e),
                None::<()>,
            )
        });
    }

    Err(jsonrpsee::types::ErrorObjectOwned::owned(
        -32603,
        "Invalid gas value: expected integer or string".to_string(),
        None::<()>,
    ))
}

fn decode_u64_block_param(s: &str) -> Result<u64, jsonrpsee::types::ErrorObjectOwned> {
    if s == "latest" {
        return Err(jsonrpsee::types::ErrorObjectOwned::owned(
            -32603,
            "latest must be handled separately".to_string(),
            None::<()>,
        ));
    }
    if s == "earliest" {
        return Ok(0);
    }
    if let Some(stripped) = s.strip_prefix("0x") {
        return u64::from_str_radix(stripped, 16).map_err(|e| {
            jsonrpsee::types::ErrorObjectOwned::owned(
                -32603,
                format!("Invalid block number: {}", e),
                None::<()>,
            )
        });
    }
    Err(jsonrpsee::types::ErrorObjectOwned::owned(
        -32603,
        "Invalid block number format".to_string(),
        None::<()>,
    ))
}

fn to_hex_quantity_u64(v: u64) -> String {
    format!("0x{:x}", v)
}

fn to_hex_quantity_u128(v: u128) -> String {
    format!("0x{:x}", v)
}

fn to_hex_hash_32(bytes: &[u8]) -> Option<String> {
    if bytes.len() != 32 {
        return None;
    }
    Some(format!("0x{}", hex::encode(bytes)))
}

fn to_hex_address_20(bytes: &[u8]) -> Option<String> {
    if bytes.len() != 20 {
        return None;
    }
    Some(format!("0x{}", hex::encode(bytes)))
}

fn block_timestamp_from_header(header: &x3_chain_runtime::opaque::Header) -> u64 {
    for log in &header.digest.logs {
        let data = match log {
            DigestItem::PreRuntime(_, data)
            | DigestItem::Consensus(_, data)
            | DigestItem::Seal(_, data)
            | DigestItem::Other(data) => data,
            _ => continue,
        };

        if data.is_empty() {
            continue;
        }

        if let Ok(raw_ts) = <u64 as codec::Decode>::decode(&mut &data[..]) {
            // Ethereum expects seconds; Substrate timestamp payloads are usually milliseconds.
            return if raw_ts > 10_000_000_000 {
                raw_ts / 1000
            } else {
                raw_ts
            };
        }
    }
    0
}

/// Create a Frontier-compatible JSON-RPC module backed by runtime API calls.
/// Provides eth_getBalance, eth_getCode, eth_getStorageAt,
/// eth_getTransactionCount (nonce), eth_call, and eth_estimateGas.
pub fn create_frontier_rpc<C>(
    client: Arc<C>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: Send
        + Sync
        + 'static
        + ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + BlockBackend<Block>,
    C::Api: pallet_x3_kernel::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>,
{
    let mut module = RpcModule::new(());

    // eth_getBalance — returns native balance for an EVM address as hex wei
    let c = client.clone();
    module.register_method(
        "eth_getBalance",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (address_hex, _block): (String, serde_json::Value) =
                params.parse().unwrap_or_else(|_| {
                    let s: String = params.one().unwrap_or_default();
                    (s, serde_json::Value::Null)
                });
            let bytes = decode_address(&address_hex)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let balance: Balance = api
                .get_evm_balance(at, bytes, 0u32)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?
                .unwrap_or_default();
            Ok(serde_json::Value::String(format!("0x{:x}", balance)))
        },
    )?;

    // eth_getCode — returns contract bytecode for an EVM address as 0x-prefixed hex
    let c = client.clone();
    module.register_method(
        "eth_getCode",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (address_hex, _block): (String, serde_json::Value) =
                params.parse().unwrap_or_else(|_| {
                    let s: String = params.one().unwrap_or_default();
                    (s, serde_json::Value::Null)
                });
            let bytes = decode_address(&address_hex)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let code: Vec<u8> = api.get_evm_code(at, bytes).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(code)
            )))
        },
    )?;

    // eth_getStorageAt — returns EVM storage value at (address, slot) as 0x-prefixed hex
    let c = client.clone();
    module.register_method(
        "eth_getStorageAt",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (address_hex, slot_hex, _block): (String, String, serde_json::Value) =
                params.parse().map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
                })?;
            let addr_bytes = decode_address(&address_hex)?;
            let slot_stripped = slot_hex.strip_prefix("0x").unwrap_or(&slot_hex);
            let slot_bytes = hex::decode(slot_stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid slot: {}", e),
                    None::<()>,
                )
            })?;
            if slot_bytes.len() > 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Slot must be ≤32 bytes".to_string(),
                    None::<()>,
                ));
            }
            let mut key = [0u8; 32];
            let offset = 32 - slot_bytes.len();
            key[offset..].copy_from_slice(&slot_bytes);
            let storage_key = sp_core::H256::from(key);
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let val: Option<sp_core::H256> = api
                .get_evm_storage(at, addr_bytes, storage_key)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(val.unwrap_or_default().as_bytes())
            )))
        },
    )?;

    // eth_getTransactionCount — returns account nonce as hex
    let c = client.clone();
    module.register_method(
        "eth_getTransactionCount",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (address_hex, _block): (String, serde_json::Value) =
                params.parse().unwrap_or_else(|_| {
                    let s: String = params.one().unwrap_or_default();
                    (s, serde_json::Value::Null)
                });
            let bytes = decode_address(&address_hex)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let nonce: u64 = api.get_evm_nonce(at, bytes).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::Value::String(format!("0x{:x}", nonce)))
        },
    )?;

    // eth_call — execute a read-only EVM call and return raw output bytes.
    let c = client.clone();
    module.register_method(
        "eth_call",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (tx_obj, _block): (serde_json::Value, serde_json::Value) =
                params.parse().unwrap_or_else(|_| {
                    let tx: serde_json::Value = params
                        .one()
                        .unwrap_or(serde_json::Value::Object(Default::default()));
                    (tx, serde_json::Value::Null)
                });

            let target = tx_obj.get("to").and_then(|v| v.as_str()).ok_or_else(|| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Missing to address".to_string(),
                    None::<()>,
                )
            })?;
            let target_bytes = decode_address(target)?;
            let caller = tx_obj
                .get("from")
                .and_then(|v| v.as_str())
                .map(decode_address)
                .transpose()?;

            let data_hex = tx_obj.get("data").and_then(|v| v.as_str()).unwrap_or("0x");
            let data_stripped = data_hex.strip_prefix("0x").unwrap_or(data_hex);
            let input_data = hex::decode(data_stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid data: {}", e),
                    None::<()>,
                )
            })?;
            let gas_limit = parse_gas_limit(&tx_obj)?;

            let api = c.runtime_api();
            let at = c.info().best_hash;
            let result: Result<Vec<u8>, Vec<u8>> = api
                .call_evm(at, caller, target_bytes, input_data, gas_limit)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("EVM API error: {}", e),
                        None::<()>,
                    )
                })?;

            match result {
                Ok(output) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(output)
                ))),
                Err(err) => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("EVM call failed: {}", String::from_utf8_lossy(&err)),
                    None::<()>,
                )),
            }
        },
    )?;

    // eth_estimateGas — estimate gas using runtime EVM dry-run logic.
    let c = client.clone();
    module.register_method(
        "eth_estimateGas",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let tx_obj: serde_json::Value = params.one().map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid params: {}", e),
                    None::<()>,
                )
            })?;

            let target_bytes = if let Some(target) = tx_obj.get("to").and_then(|v| v.as_str()) {
                decode_address(target)?
            } else {
                vec![0u8; 20] // Contract creation
            };
            let caller = tx_obj
                .get("from")
                .and_then(|v| v.as_str())
                .map(decode_address)
                .transpose()?;

            let data_hex = tx_obj.get("data").and_then(|v| v.as_str()).unwrap_or("0x");
            let data_stripped = data_hex.strip_prefix("0x").unwrap_or(data_hex);
            let input_data = hex::decode(data_stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid data: {}", e),
                    None::<()>,
                )
            })?;
            let gas_limit = parse_gas_limit(&tx_obj)?;

            let api = c.runtime_api();
            let at = c.info().best_hash;
            let result: Result<u64, Vec<u8>> = api
                .estimate_evm_gas(at, caller, target_bytes, input_data, gas_limit)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("EVM API error: {}", e),
                        None::<()>,
                    )
                })?;

            match result {
                Ok(gas) => Ok(serde_json::Value::String(format!("0x{:x}", gas))),
                Err(err) => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Gas estimation failed: {}", String::from_utf8_lossy(&err)),
                    None::<()>,
                )),
            }
        },
    )?;

    // eth_sendRawTransaction — submit a signed RLP-encoded Ethereum transaction
    // Executes via the X3 kernel EVM adapter and returns the keccak256 tx hash.
    let c = client.clone();
    module.register_method(
        "eth_sendRawTransaction",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let raw_hex: String = params.one()?;
            let stripped = raw_hex.strip_prefix("0x").unwrap_or(&raw_hex);
            let raw_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid hex: {}", e),
                    None::<()>,
                )
            })?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let result: Result<Vec<u8>, Vec<u8>> =
                api.submit_evm_transaction(at, raw_bytes).map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match result {
                Ok(tx_hash) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(tx_hash)
                ))),
                Err(err_bytes) => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!(
                        "EVM execution failed: {}",
                        String::from_utf8_lossy(&err_bytes)
                    ),
                    None::<()>,
                )),
            }
        },
    )?;

    // eth_getTransactionByHash — returns EVM transaction object by hash
    let c = client.clone();
    module.register_method(
        "eth_getTransactionByHash",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let tx_hash_hex: String = params.one()?;
            let stripped = tx_hash_hex.strip_prefix("0x").unwrap_or(&tx_hash_hex);
            let tx_hash_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid tx hash: {}", e),
                    None::<()>,
                )
            })?;
            if tx_hash_bytes.len() != 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction hash must be 32 bytes".to_string(),
                    None::<()>,
                ));
            }
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let tx_opt: Option<Vec<u8>> = api
                .get_evm_transaction_by_hash(at, tx_hash_bytes.clone())
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match tx_opt {
                Some(tx_bytes) => {
                    use codec::Decode;
                    let tx =
                        pallet_x3_kernel::pallet::EvmTransactionData::decode(&mut &tx_bytes[..])
                            .map_err(|e| {
                                jsonrpsee::types::ErrorObjectOwned::owned(
                                    -32603,
                                    format!("Decode error: {:?}", e),
                                    None::<()>,
                                )
                            })?;

                    let receipt_opt: Option<Vec<u8>> = api
                        .get_evm_receipt(at, tx_hash_bytes.clone())
                        .map_err(|e| {
                            jsonrpsee::types::ErrorObjectOwned::owned(
                                -32603,
                                format!("Runtime error: {:?}", e),
                                None::<()>,
                            )
                        })?;

                    let block_number_opt = receipt_opt
                        .and_then(|receipt_bytes| {
                            pallet_x3_kernel::ExecutionReceipt::decode(&mut &receipt_bytes[..]).ok()
                        })
                        .and_then(|receipt| receipt.logs.first().map(|l| l.block_number));

                    let (block_number_json, block_hash_json, tx_index_json) =
                        if let Some(block_number) = block_number_opt {
                            if let Ok(block_hash_opt) = c.hash(block_number as u32) {
                                let block_hash_json = block_hash_opt
                                    .map(|h| {
                                        serde_json::Value::String(format!(
                                            "0x{}",
                                            hex::encode(h.as_bytes())
                                        ))
                                    })
                                    .unwrap_or(serde_json::Value::Null);
                                (
                                    serde_json::Value::String(to_hex_quantity_u64(block_number)),
                                    block_hash_json,
                                    serde_json::Value::String("0x0".to_string()),
                                )
                            } else {
                                (
                                    serde_json::Value::String(to_hex_quantity_u64(block_number)),
                                    serde_json::Value::Null,
                                    serde_json::Value::Null,
                                )
                            }
                        } else {
                            (
                                serde_json::Value::Null,
                                serde_json::Value::Null,
                                serde_json::Value::Null,
                            )
                        };

                    let from = to_hex_address_20(&tx.from).unwrap_or_else(|| {
                        "0x0000000000000000000000000000000000000000".to_string()
                    });
                    let to = to_hex_address_20(&tx.to)
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null);

                    let chain_id = api.chain_id(at).map_err(|e| {
                        jsonrpsee::types::ErrorObjectOwned::owned(
                            -32603,
                            format!("Runtime error: {:?}", e),
                            None::<()>,
                        )
                    })?;

                    Ok(serde_json::json!({
                        "hash": to_hex_hash_32(&tx_hash_bytes).unwrap_or_default(),
                        "nonce": to_hex_quantity_u64(tx.nonce),
                        "blockHash": block_hash_json,
                        "blockNumber": block_number_json,
                        "transactionIndex": tx_index_json,
                        "from": from,
                        "to": to,
                        "value": to_hex_quantity_u128(tx.value),
                        "gasPrice": to_hex_quantity_u128(tx.gas_price),
                        "maxPriorityFeePerGas": serde_json::Value::Null,
                        "maxFeePerGas": serde_json::Value::Null,
                        "gas": to_hex_quantity_u64(tx.gas),
                        "input": format!("0x{}", hex::encode(&tx.input)),
                        "v": serde_json::Value::Null,
                        "r": serde_json::Value::Null,
                        "s": serde_json::Value::Null,
                        "type": "0x0",
                        "accessList": serde_json::Value::Null,
                        "chainId": to_hex_quantity_u64(chain_id)
                    }))
                }
                None => Ok(serde_json::Value::Null),
            }
        },
    )?;

    // eth_getTransactionReceipt — returns EVM transaction receipt by hash (Ethereum-compatible format)
    let c = client.clone();
    module.register_method("eth_getTransactionReceipt", move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
        let tx_hash_hex: String = params.one()?;
        let stripped = tx_hash_hex.strip_prefix("0x").unwrap_or(&tx_hash_hex);
        let tx_hash_bytes = hex::decode(stripped)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Invalid tx hash: {}", e), None::<()>))?;
        if tx_hash_bytes.len() != 32 {
            return Err(jsonrpsee::types::ErrorObjectOwned::owned(-32603, "Transaction hash must be 32 bytes".to_string(), None::<()>));
        }
        let api = c.runtime_api();
        let at = c.info().best_hash;
        let receipt_opt: Option<Vec<u8>> = api
            .get_evm_receipt(at, tx_hash_bytes.clone())
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Runtime error: {:?}", e), None::<()>))?;
        match receipt_opt {
            Some(receipt_bytes) => {
                use codec::Decode;
                let receipt = pallet_x3_kernel::ExecutionReceipt::decode(&mut &receipt_bytes[..])
                    .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Decode error: {:?}", e), None::<()>))?;
                let tx_hash = format!("0x{}", hex::encode(&tx_hash_bytes));
                let logs: Vec<serde_json::Value> = receipt.logs.iter().map(|log| {
                    serde_json::json!({
                        "address": format!("0x{}", hex::encode(&log.address)),
                        "topics": log.topics.iter().map(|t| format!("0x{}", hex::encode(t.as_bytes()))).collect::<Vec<_>>(),
                        "data": format!("0x{}", hex::encode(&log.data))
                    })
                }).collect();
                Ok(serde_json::json!({
                    "transactionHash": tx_hash,
                    "status": if receipt.success { "0x1" } else { "0x0" },
                    "gasUsed": format!("0x{:x}", receipt.gas_used),
                    "logs": logs
                }))
            }
            None => Ok(serde_json::Value::Null),
        }
    })?;

    // eth_getLogs — returns EVM logs matching a filter
    let c = client.clone();
    module.register_method("eth_getLogs", move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
        let filter: serde_json::Value = params.one().unwrap_or_else(|_| serde_json::Value::Null);
        let latest_block = c.info().best_number as u64;
        let from_block = filter.get("fromBlock")
            .and_then(|v| v.as_str())
            .map(|s| {
                if s == "latest" {
                    return Ok(latest_block);
                }
                let stripped = s.strip_prefix("0x").unwrap_or(s);
                u64::from_str_radix(stripped, 16)
                    .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Invalid fromBlock: {}", e), None::<()>))
            }).unwrap_or(Ok(0))?;
        let to_block = filter.get("toBlock")
            .and_then(|v| v.as_str())
            .map(|s| {
                if s == "latest" {
                    return Ok(latest_block);
                }
                let stripped = s.strip_prefix("0x").unwrap_or(s);
                u64::from_str_radix(stripped, 16)
                    .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Invalid toBlock: {}", e), None::<()>))
            }).unwrap_or(Ok(latest_block))?;
        let address = filter.get("address")
            .and_then(|v| v.as_str())
            .map(decode_address)
            .transpose()?;
        // Encode filter as SCALE tuple: (from_block: u64, to_block: u64, address: Option<[u8; 20]>)
        // SCALE encoding: u64 (8 bytes LE) + u64 (8 bytes LE) + Option tag (0x00/0x01) + [u8; 20] (if Some)
        let mut filter_bytes = Vec::new();
        filter_bytes.extend_from_slice(&from_block.to_le_bytes());
        filter_bytes.extend_from_slice(&to_block.to_le_bytes());
        match address {
            Some(addr) => {
                filter_bytes.push(0x01); // Some tag
                filter_bytes.extend_from_slice(&addr);
            }
            None => {
                filter_bytes.push(0x00); // None tag
            }
        }
        let api = c.runtime_api();
        let at = c.info().best_hash;
        let logs_bytes: Vec<Vec<u8>> = api
            .get_evm_logs(at, filter_bytes)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Runtime error: {:?}", e), None::<()>))?;
        let logs: Vec<serde_json::Value> = logs_bytes.iter()
            .filter_map(|bytes| {
                use codec::Decode;
                let log = pallet_x3_kernel::ExecutionLog::decode(&mut &bytes[..]).ok()?;
                // Filter logs based on block range
                if log.block_number >= from_block && log.block_number <= to_block {
                    Some(serde_json::json!({
                        "address": format!("0x{}", hex::encode(&log.address)),
                        "topics": log.topics.iter().map(|t| format!("0x{}", hex::encode(t.as_bytes()))).collect::<Vec<_>>(),
                        "data": format!("0x{}", hex::encode(&log.data)),
                        "blockNumber": format!("0x{:x}", log.block_number)
                    }))
                } else {
                    None
                }
            })
            .collect();
        Ok(serde_json::Value::Array(logs))
    })?;

    // x3_getEvmTransaction — returns raw EVM transaction receipt by hash
    let c = client.clone();
    module.register_method(
        "x3_getEvmTransaction",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let tx_hash_hex: String = params.one()?;
            let stripped = tx_hash_hex.strip_prefix("0x").unwrap_or(&tx_hash_hex);
            let tx_hash_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid tx hash: {}", e),
                    None::<()>,
                )
            })?;
            if tx_hash_bytes.len() != 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction hash must be 32 bytes".to_string(),
                    None::<()>,
                ));
            }
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let receipt_opt: Option<Vec<u8>> =
                api.get_evm_transaction(at, tx_hash_bytes).map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match receipt_opt {
                Some(receipt_bytes) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(receipt_bytes)
                ))),
                None => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction not found".to_string(),
                    None::<()>,
                )),
            }
        },
    )?;

    // x3_getEvmReceipt — returns raw EVM transaction receipt by hash (alternative endpoint)
    let c = client.clone();
    module.register_method(
        "x3_getEvmReceipt",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let tx_hash_hex: String = params.one()?;
            let stripped = tx_hash_hex.strip_prefix("0x").unwrap_or(&tx_hash_hex);
            let tx_hash_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid tx hash: {}", e),
                    None::<()>,
                )
            })?;
            if tx_hash_bytes.len() != 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction hash must be 32 bytes".to_string(),
                    None::<()>,
                ));
            }
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let receipt_opt: Option<Vec<u8>> =
                api.get_evm_receipt(at, tx_hash_bytes).map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match receipt_opt {
                Some(receipt_bytes) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(receipt_bytes)
                ))),
                None => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction receipt not found".to_string(),
                    None::<()>,
                )),
            }
        },
    )?;

    // x3_getEvmLogs — returns raw EVM logs matching a filter
    let c = client.clone();
    module.register_method(
        "x3_getEvmLogs",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let filter: serde_json::Value =
                params.one().unwrap_or_else(|_| serde_json::Value::Null);
            let from_block = filter
                .get("fromBlock")
                .and_then(|v| v.as_str())
                .map(|s| {
                    let stripped = s.strip_prefix("0x").unwrap_or(s);
                    u64::from_str_radix(stripped, 16).map_err(|e| {
                        jsonrpsee::types::ErrorObjectOwned::owned(
                            -32603,
                            format!("Invalid fromBlock: {}", e),
                            None::<()>,
                        )
                    })
                })
                .unwrap_or(Ok(0))?;
            let to_block = filter
                .get("toBlock")
                .and_then(|v| v.as_str())
                .map(|s| {
                    let stripped = s.strip_prefix("0x").unwrap_or(s);
                    u64::from_str_radix(stripped, 16).map_err(|e| {
                        jsonrpsee::types::ErrorObjectOwned::owned(
                            -32603,
                            format!("Invalid toBlock: {}", e),
                            None::<()>,
                        )
                    })
                })
                .unwrap_or(Ok(0))?;
            let address = filter
                .get("address")
                .and_then(|v| v.as_str())
                .map(decode_address)
                .transpose()?;
            // Encode filter as SCALE tuple: (from_block: u64, to_block: u64, address: Option<[u8; 20]>)
            // SCALE encoding: u64 (8 bytes LE) + u64 (8 bytes LE) + Option tag (0x00/0x01) + [u8; 20] (if Some)
            let mut filter_bytes = Vec::new();
            filter_bytes.extend_from_slice(&from_block.to_le_bytes());
            filter_bytes.extend_from_slice(&to_block.to_le_bytes());
            match address {
                Some(addr) => {
                    filter_bytes.push(0x01); // Some tag
                    filter_bytes.extend_from_slice(&addr);
                }
                None => {
                    filter_bytes.push(0x00); // None tag
                }
            }
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let logs_bytes: Vec<Vec<u8>> = api.get_evm_logs(at, filter_bytes).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            let logs: Vec<String> = logs_bytes
                .iter()
                .map(|bytes| format!("0x{}", hex::encode(bytes)))
                .collect();
            Ok(serde_json::json!({ "logs": logs }))
        },
    )?;

    // x3_getEvmTransactionByHash — returns formatted EVM transaction by hash
    let c = client.clone();
    module.register_method(
        "x3_getEvmTransactionByHash",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let tx_hash_hex: String = params.one()?;
            let stripped = tx_hash_hex.strip_prefix("0x").unwrap_or(&tx_hash_hex);
            let tx_hash_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid tx hash: {}", e),
                    None::<()>,
                )
            })?;
            if tx_hash_bytes.len() != 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction hash must be 32 bytes".to_string(),
                    None::<()>,
                ));
            }
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let receipt_opt: Option<Vec<u8>> = api
                .get_evm_transaction(at, tx_hash_bytes.clone())
                .map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            match receipt_opt {
                Some(receipt_bytes) => {
                    use codec::Decode;
                    let receipt =
                        pallet_x3_kernel::ExecutionReceipt::decode(&mut &receipt_bytes[..])
                            .map_err(|e| {
                                jsonrpsee::types::ErrorObjectOwned::owned(
                                    -32603,
                                    format!("Decode error: {:?}", e),
                                    None::<()>,
                                )
                            })?;
                    let tx_hash = format!("0x{}", hex::encode(&tx_hash_bytes));
                    Ok(serde_json::json!({
                        "transactionHash": tx_hash,
                        "success": receipt.success,
                        "gasUsed": format!("0x{:x}", receipt.gas_used),
                        "returnData": format!("0x{}", hex::encode(&receipt.return_data)),
                        "logsCount": receipt.logs.len()
                    }))
                }
                None => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction not found".to_string(),
                    None::<()>,
                )),
            }
        },
    )?;

    // x3_getEvmReceiptByHash — returns formatted EVM receipt by hash
    let c = client.clone();
    module.register_method(
        "x3_getEvmReceiptByHash",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let tx_hash_hex: String = params.one()?;
            let stripped = tx_hash_hex.strip_prefix("0x").unwrap_or(&tx_hash_hex);
            let tx_hash_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid tx hash: {}", e),
                    None::<()>,
                )
            })?;
            if tx_hash_bytes.len() != 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction hash must be 32 bytes".to_string(),
                    None::<()>,
                ));
            }
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let receipt_opt: Option<Vec<u8>> = api
                .get_evm_receipt(at, tx_hash_bytes.clone())
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match receipt_opt {
                Some(receipt_bytes) => {
                    use codec::Decode;
                    let receipt =
                        pallet_x3_kernel::ExecutionReceipt::decode(&mut &receipt_bytes[..])
                            .map_err(|e| {
                                jsonrpsee::types::ErrorObjectOwned::owned(
                                    -32603,
                                    format!("Decode error: {:?}", e),
                                    None::<()>,
                                )
                            })?;
                    let tx_hash = format!("0x{}", hex::encode(&tx_hash_bytes));
                    Ok(serde_json::json!({
                        "transactionHash": tx_hash,
                        "success": receipt.success,
                        "gasUsed": format!("0x{:x}", receipt.gas_used),
                        "logsCount": receipt.logs.len()
                    }))
                }
                None => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Transaction receipt not found".to_string(),
                    None::<()>,
                )),
            }
        },
    )?;

    // eth_blockNumber — returns the current block number as hex
    let c = client.clone();
    module.register_method(
        "eth_blockNumber",
        move |_, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let info = c.info();
            Ok(serde_json::Value::String(format!(
                "0x{:x}",
                info.best_number
            )))
        },
    )?;

    // eth_chainId — returns the chain ID as hex
    let c = client.clone();
    module.register_method(
        "eth_chainId",
        move |_, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let chain_id = c.runtime_api().chain_id(c.info().best_hash).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::Value::String(format!("0x{:x}", chain_id)))
        },
    )?;

    // net_version — returns the network version as string
    let c = client.clone();
    module.register_method(
        "net_version",
        move |_, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let chain_id = c.runtime_api().chain_id(c.info().best_hash).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::Value::String(format!("{}", chain_id)))
        },
    )?;

    // eth_getBlockByNumber — returns block info by block number
    let c = client.clone();
    module.register_method(
        "eth_getBlockByNumber",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (block_param, _full): (serde_json::Value, bool) = params
                .parse()
                .unwrap_or_else(|_| (serde_json::Value::String("latest".to_string()), false));

            let block_number = match block_param {
                serde_json::Value::String(s) => {
                    if s == "latest" {
                        c.info().best_number as u64
                    } else {
                        decode_u64_block_param(&s)?
                    }
                }
                serde_json::Value::Number(n) => n.as_u64().ok_or_else(|| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        "Invalid block number".to_string(),
                        None::<()>,
                    )
                })?,
                _ => {
                    return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        "Invalid block number type".to_string(),
                        None::<()>,
                    ))
                }
            };

            if block_number > u32::MAX as u64 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Block number exceeds supported range".to_string(),
                    None::<()>,
                ));
            }

            let block = c
                .block(
                    c.hash((block_number as u32).into())
                        .map_err(|e| {
                            jsonrpsee::types::ErrorObjectOwned::owned(
                                -32603,
                                format!("Hash lookup error: {:?}", e),
                                None::<()>,
                            )
                        })?
                        .ok_or_else(|| {
                            jsonrpsee::types::ErrorObjectOwned::owned(
                                -32602,
                                "Block not found".to_string(),
                                None::<()>,
                            )
                        })?,
                )
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;

            let header = match block {
                Some(b) => b.block.header.clone(),
                None => return Ok(serde_json::Value::Null),
            };
            let hash = header.hash();
            let parent_hash = header.parent_hash;
            let state_root = header.state_root;
            let extrinsics_root = header.extrinsics_root;
            let number = header.number;
            let timestamp = block_timestamp_from_header(&header);

            Ok(serde_json::json!({
                "number": format!("0x{:x}", number),
                "hash": format!("0x{}", hex::encode(hash.as_bytes())),
                "parentHash": format!("0x{}", hex::encode(parent_hash.as_bytes())),
                "stateRoot": format!("0x{}", hex::encode(state_root.as_bytes())),
                "extrinsicsRoot": format!("0x{}", hex::encode(extrinsics_root.as_bytes())),
                "logsBloom": "0x0",
                "transactionsRoot": format!("0x{}", hex::encode(hash.as_bytes())),
                "miner": "0x0000000000000000000000000000000000000000",
                "gasLimit": format!("0xc350"),
                "gasUsed": format!("0x0"),
                "timestamp": format!("0x{:x}", timestamp),
                "difficulty": format!("0x0"),
                "nonce": "0x0000000000000000",
                "size": format!("0x0"),
                "extraData": "0x",
                "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }))
        },
    )?;

    // eth_getBlockByHash — returns block info by block hash
    let c = client.clone();
    module.register_method(
        "eth_getBlockByHash",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (hash_hex, _full): (String, bool) =
                params.parse().unwrap_or_else(|_| ("0x".to_string(), false));

            let stripped = hash_hex.strip_prefix("0x").unwrap_or(&hash_hex);
            let hash_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid hash: {}", e),
                    None::<()>,
                )
            })?;
            if hash_bytes.len() != 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Block hash must be 32 bytes".to_string(),
                    None::<()>,
                ));
            }

            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_bytes);
            let block_hash = sp_core::H256::from(hash);

            let block = c.block(block_hash).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;

            let header = match block {
                Some(b) => b.block.header.clone(),
                None => return Ok(serde_json::Value::Null),
            };
            let hash = header.hash();
            let parent_hash = header.parent_hash;
            let state_root = header.state_root;
            let extrinsics_root = header.extrinsics_root;
            let number = header.number;
            let timestamp = block_timestamp_from_header(&header);

            Ok(serde_json::json!({
                "number": format!("0x{:x}", number),
                "hash": format!("0x{}", hex::encode(hash.as_bytes())),
                "parentHash": format!("0x{}", hex::encode(parent_hash.as_bytes())),
                "stateRoot": format!("0x{}", hex::encode(state_root.as_bytes())),
                "extrinsicsRoot": format!("0x{}", hex::encode(extrinsics_root.as_bytes())),
                "logsBloom": "0x0",
                "transactionsRoot": format!("0x{}", hex::encode(hash.as_bytes())),
                "miner": "0x0000000000000000000000000000000000000000",
                "gasLimit": format!("0xc350"),
                "gasUsed": format!("0x0"),
                "timestamp": format!("0x{:x}", timestamp),
                "difficulty": format!("0x0"),
                "nonce": "0x0000000000000000",
                "size": format!("0x0"),
                "extraData": "0x",
                "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }))
        },
    )?;

    // eth_getTransactionLogs — returns EVM logs for a specific transaction by hash
    let c = client.clone();
    module.register_method("eth_getTransactionLogs", move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
        let tx_hash_hex: String = params.one()?;
        let stripped = tx_hash_hex.strip_prefix("0x").unwrap_or(&tx_hash_hex);
        let tx_hash_bytes = hex::decode(stripped)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Invalid tx hash: {}", e), None::<()>))?;
        if tx_hash_bytes.len() != 32 {
            return Err(jsonrpsee::types::ErrorObjectOwned::owned(-32603, "Transaction hash must be 32 bytes".to_string(), None::<()>));
        }
        let api = c.runtime_api();
        let at = c.info().best_hash;
        let logs_bytes: Vec<Vec<u8>> = api
            .get_evm_transaction_logs(at, tx_hash_bytes)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Runtime error: {:?}", e), None::<()>))?;
        let logs: Vec<serde_json::Value> = logs_bytes.iter()
            .filter_map(|bytes| {
                use codec::Decode;
                let log = pallet_x3_kernel::ExecutionLog::decode(&mut &bytes[..]).ok()?;
                Some(serde_json::json!({
                    "address": format!("0x{}", hex::encode(&log.address)),
                    "topics": log.topics.iter().map(|t| format!("0x{}", hex::encode(t.as_bytes()))).collect::<Vec<_>>(),
                    "data": format!("0x{}", hex::encode(&log.data)),
                    "blockNumber": format!("0x{:x}", log.block_number)
                }))
            })
            .collect();
        Ok(serde_json::Value::Array(logs))
    })?;

    Ok(module)
}

/// Create an SVM-compatible JSON-RPC module backed by runtime API calls.
/// Provides SVM-compatible endpoints for querying SVM state.
pub fn create_svm_rpc<C>(
    client: Arc<C>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: Send
        + Sync
        + 'static
        + ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + BlockBackend<Block>,
    C::Api: pallet_x3_kernel::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>,
{
    let mut module = RpcModule::new(());

    // svm_executeInstruction — execute a raw SVM instruction payload via runtime API.
    // params: [caller_hex_32, program_id_hex_32, instruction_data_hex]
    let c = client.clone();
    module.register_method(
        "svm_executeInstruction",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (caller_hex, program_hex, input_hex): (String, String, String) = params.parse()?;

            let _caller = decode_svm_pubkey(&caller_hex)?;
            let program_bytes = decode_svm_pubkey(&program_hex)?;
            let data_stripped = input_hex.strip_prefix("0x").unwrap_or(&input_hex);
            let instruction_data = hex::decode(data_stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid instruction data: {}", e),
                    None::<()>,
                )
            })?;

            let mut program_id = [0u8; 32];
            program_id.copy_from_slice(&program_bytes);

            let api = c.runtime_api();
            let at = c.info().best_hash;

            match api.submit_svm_instruction(at, program_id, instruction_data) {
                Ok(Ok(output)) => Ok(serde_json::json!({
                    "success": true,
                    "output": format!("0x{}", hex::encode(output))
                })),
                Ok(Err(err)) => Ok(serde_json::json!({
                    "success": false,
                    "error": String::from_utf8_lossy(&err).to_string()
                })),
                Err(e) => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )),
            }
        },
    )?;

    // svm_getBalance — returns lamport balance for a base58 or hex SVM pubkey
    let c = client.clone();
    module.register_method(
        "svm_getBalance",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let pubkey_str: String = params.one()?;
            let bytes = decode_svm_pubkey(&pubkey_str)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let balance: u64 = api.get_svm_balance(at, bytes).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::json!({ "value": balance }))
        },
    )?;

    // svm_isProgram — returns whether a pubkey has a deployed executable program
    let c = client.clone();
    module.register_method(
        "svm_isProgram",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let pubkey_str: String = params.one()?;
            let bytes = decode_svm_pubkey(&pubkey_str)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let is_prog: bool = api.is_svm_program(at, bytes).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::json!({ "result": is_prog }))
        },
    )?;

    // svm_getAccountInfo — returns account metadata for a SVM pubkey
    let c = client.clone();
    module.register_method(
        "svm_getAccountInfo",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let pubkey_str: String = params.one()?;
            let bytes = decode_svm_pubkey(&pubkey_str)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let balance: u64 = api.get_svm_balance(at, bytes).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::json!({
                "value": {
                    "lamports": balance,
                    "owner": "11111111111111111111111111111111", // Default program owner
                    "executable": false,
                    "rentEpoch": 0u64
                }
            }))
        },
    )?;

    // svm_getBlockByHash — returns block info by hash
    let c = client.clone();
    module.register_method(
        "svm_getBlockByHash",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (block_hash_hex, _config): (String, Option<serde_json::Value>) = params.parse()?;
            let stripped = block_hash_hex.strip_prefix("0x").unwrap_or(&block_hash_hex);
            let block_hash_bytes = hex::decode(stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid block hash: {}", e),
                    None::<()>,
                )
            })?;
            if block_hash_bytes.len() != 32 {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    "Block hash must be 32 bytes".to_string(),
                    None::<()>,
                ));
            }
            let mut block_hash = [0u8; 32];
            block_hash.copy_from_slice(&block_hash_bytes);
            let api = c.runtime_api();
            let at = c.info().best_hash;

            // First, try to get the slot from the reverse index (blockhash -> slot)
            let slot = {
                // Convert block_hash to H256 for the runtime API
                let block_hash_h256 = sp_core::H256::from(block_hash);
                let maybe_direct =
                    api.get_svm_slot_by_blockhash(at, block_hash_h256)
                        .map_err(|e| {
                            jsonrpsee::types::ErrorObjectOwned::owned(
                                -32603,
                                format!("API error: {:?}", e),
                                None::<()>,
                            )
                        })?;
                if let Some(s) = maybe_direct {
                    s
                } else {
                    // Fallback: scan the forward index (slot -> blockhash) to find the slot
                    let current_slot = api.get_svm_slot(at).map_err(|e| {
                        jsonrpsee::types::ErrorObjectOwned::owned(
                            -32603,
                            format!("API error: {:?}", e),
                            None::<()>,
                        )
                    })?;
                    (current_slot.saturating_sub(1000)..=current_slot)
                        .find_map(|s| {
                            let bh = api.get_svm_blockhash(at, s).ok()??;
                            if bh.as_bytes() == block_hash {
                                Some(s)
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| {
                            jsonrpsee::types::ErrorObjectOwned::owned(
                                -32603,
                                format!("Block hash not found: 0x{}", hex::encode(block_hash)),
                                None::<()>,
                            )
                        })?
                }
            };

            // Get the blockhash for this slot (to verify and get the actual blockhash)
            let actual_blockhash = api
                .get_svm_blockhash(at, slot)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("API error: {:?}", e),
                        None::<()>,
                    )
                })?
                .ok_or_else(|| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Blockhash not found for slot {}: runtime error", slot),
                        None::<()>,
                    )
                })?;

            // Verify the blockhash matches (sanity check)
            if actual_blockhash.as_bytes() != block_hash {
                return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!(
                        "Blockhash mismatch: expected 0x{}, got 0x{}",
                        hex::encode(block_hash),
                        hex::encode(actual_blockhash.as_bytes())
                    ),
                    None::<()>,
                ));
            }

            Ok(serde_json::json!({
                "slot": format!("0x{:x}", slot),
                "blockhash": format!("0x{}", hex::encode(actual_blockhash.as_bytes())),
                "parentSlot": format!("0x{:x}", slot.saturating_sub(1)),
                "parentBlockhash": format!("0x{}", hex::encode([0u8; 32])),
                "transactions": [],
                "signatures": [],
                "feeCalculator": {
                    "lamportsPerSignature": 0
                },
                "hash": format!("0x{}", hex::encode(actual_blockhash.as_bytes())),
                "previousBlockhash": format!("0x{}", hex::encode([0u8; 32])),
                "reward": [],
                "blockTime": serde_json::Value::Null,
                "blockHeight": serde_json::Value::Null
            }))
        },
    )?;

    // svm_getBlockByNumber — returns block info by slot number
    let c = client.clone();
    module.register_method(
        "svm_getBlockByNumber",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (slot_str, _config): (String, Option<serde_json::Value>) = params.parse()?;
            let stripped = slot_str.strip_prefix("0x").unwrap_or(&slot_str);
            let slot: u64 = u64::from_str_radix(stripped, 16).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid slot: {}", e),
                    None::<()>,
                )
            })?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            // Get the blockhash for this slot
            let blockhash = api
                .get_svm_blockhash(at, slot)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?
                .ok_or_else(|| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        "Blockhash not found for slot".to_string(),
                        None::<()>,
                    )
                })?;
            Ok(serde_json::json!({
                "slot": format!("0x{:x}", slot),
                "blockhash": format!("0x{}", hex::encode(blockhash.as_bytes())),
                "parentSlot": format!("0x{:x}", slot.saturating_sub(1)),
                "parentBlockhash": format!("0x{}", hex::encode([0u8; 32])),
                "transactions": [],
                "signatures": [],
                "feeCalculator": {
                    "lamportsPerSignature": 0
                },
                "hash": format!("0x{}", hex::encode(blockhash.as_bytes())),
                "previousBlockhash": format!("0x{}", hex::encode([0u8; 32])),
                "reward": [],
                "blockTime": serde_json::Value::Null,
                "blockHeight": serde_json::Value::Null
            }))
        },
    )?;

    // svm_getLatestBlockhash — returns the latest blockhash
    let c = client.clone();
    module.register_method(
        "svm_getLatestBlockhash",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let _config: Option<serde_json::Value> = params.one().ok();
            let api = c.runtime_api();
            let at = c.info().best_hash;
            // Get the current slot
            let slot = api.get_svm_slot(at).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            // Get the blockhash for this slot
            let blockhash = api
                .get_svm_blockhash(at, slot)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?
                .ok_or_else(|| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        "Blockhash not found for slot".to_string(),
                        None::<()>,
                    )
                })?;
            Ok(serde_json::json!({
                "slot": format!("0x{:x}", slot),
                "blockhash": format!("0x{}", hex::encode(blockhash.as_bytes())),
                "parentSlot": format!("0x{:x}", slot.saturating_sub(1)),
                "parentBlockhash": format!("0x{}", hex::encode([0u8; 32])),
                "transactions": [],
                "signatures": [],
                "feeCalculator": {
                    "lamportsPerSignature": 0
                },
                "hash": format!("0x{}", hex::encode(blockhash.as_bytes())),
                "previousBlockhash": format!("0x{}", hex::encode([0u8; 32])),
                "reward": [],
                "blockTime": serde_json::Value::Null,
                "blockHeight": serde_json::Value::Null
            }))
        },
    )?;

    // svm_getSlot — returns the current slot number
    let c = client.clone();
    module.register_method(
        "svm_getSlot",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let _config: Option<serde_json::Value> = params.one().ok();
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let slot: u64 = api.get_svm_slot(at).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::json!({ "value": slot }))
        },
    )?;

    // svm_getTransactionCount — returns transaction count for an address
    let c = client.clone();
    module.register_method(
        "svm_getTransactionCount",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let pubkey_str: String = params.one()?;
            let bytes = decode_svm_pubkey(&pubkey_str)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let count: u64 = api.get_svm_transaction_count(at, bytes).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Runtime error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(serde_json::json!({ "value": count }))
        },
    )?;

    // svm_deployContract — deploys a new EVM contract with the given bytecode
    let c = client.clone();
    module.register_method(
        "svm_deployContract",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (caller_hex, bytecode_hex, gas_limit): (String, String, u64) = params.parse()?;
            let caller_bytes = if caller_hex.is_empty() || caller_hex == "0x" {
                None
            } else {
                Some(decode_address(&caller_hex)?)
            };
            let bytecode_stripped = bytecode_hex.strip_prefix("0x").unwrap_or(&bytecode_hex);
            let bytecode = hex::decode(bytecode_stripped).map_err(|e| {
                jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!("Invalid bytecode: {}", e),
                    None::<()>,
                )
            })?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let result: Result<Vec<u8>, Vec<u8>> = api
                .deploy_evm_contract(at, caller_bytes, bytecode, gas_limit)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match result {
                Ok(contract_address) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(contract_address)
                ))),
                Err(err) => Err(jsonrpsee::types::ErrorObjectOwned::owned(
                    -32603,
                    format!(
                        "Contract deployment failed: {}",
                        String::from_utf8_lossy(&err)
                    ),
                    None::<()>,
                )),
            }
        },
    )?;

    // svm_getContractReceipt — returns the EVM contract creation receipt
    let c = client.clone();
    module.register_method(
        "svm_getContractReceipt",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let contract_address_hex: String = params.one()?;
            let contract_address_bytes = decode_address(&contract_address_hex)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let receipt_opt: Option<Vec<u8>> = api
                .get_evm_contract_receipt(at, contract_address_bytes)
                .map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match receipt_opt {
                Some(receipt_bytes) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(receipt_bytes)
                ))),
                None => Ok(serde_json::Value::Null),
            }
        },
    )?;

    // svm_getProgramData — returns the SVM program data for a deployed SVM program
    let c = client.clone();
    module.register_method(
        "svm_getProgramData",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let pubkey_str: String = params.one()?;
            let bytes = decode_svm_pubkey(&pubkey_str)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let program_data_opt: Option<Vec<u8>> =
                api.get_svm_program_data(at, bytes).map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match program_data_opt {
                Some(data) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(data)
                ))),
                None => Ok(serde_json::Value::Null),
            }
        },
    )?;

    // svm_getAccountData — returns the SVM account data for a SVM address
    let c = client.clone();
    module.register_method(
        "svm_getAccountData",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let pubkey_str: String = params.one()?;
            let bytes = decode_svm_pubkey(&pubkey_str)?;
            let api = c.runtime_api();
            let at = c.info().best_hash;
            let account_data_opt: Option<Vec<u8>> =
                api.get_svm_account_data(at, bytes).map_err(|e| {
                    jsonrpsee::types::ErrorObjectOwned::owned(
                        -32603,
                        format!("Runtime error: {:?}", e),
                        None::<()>,
                    )
                })?;
            match account_data_opt {
                Some(data) => Ok(serde_json::Value::String(format!(
                    "0x{}",
                    hex::encode(data)
                ))),
                None => Ok(serde_json::Value::Null),
            }
        },
    )?;

    // svm_getSlotHistory — returns the SVM slot history for recent blockhashes
    let c = client.clone();
    module.register_method("svm_getSlotHistory", move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
        let config: Option<serde_json::Value> = params.one().ok();
        let limit = config
            .as_ref()
            .and_then(|v| v.get("limit"))
            .and_then(|l| l.as_u64())
            .unwrap_or(50) as u32;
        let api = c.runtime_api();
        let at = c.info().best_hash;
        let slot_history: Vec<u64> = api
            .get_svm_slot_history(at, limit)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Runtime error: {:?}", e), None::<()>))?;
        Ok(serde_json::json!({ "slots": slot_history.iter().map(|s| format!("0x{:x}", s)).collect::<Vec<_>>() }))
    })?;

    // svm_getRecentBlockhashes — returns the SVM recent blockhashes
    let c = client.clone();
    module.register_method("svm_getRecentBlockhashes", move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
        let config: Option<serde_json::Value> = params.one().ok();
        let limit = config
            .as_ref()
            .and_then(|v| v.get("limit"))
            .and_then(|l| l.as_u64())
            .unwrap_or(50) as u32;
        let api = c.runtime_api();
        let at = c.info().best_hash;
        let blockhashes: Vec<sp_core::H256> = api
            .get_svm_recent_blockhashes(at, limit)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, format!("Runtime error: {:?}", e), None::<()>))?;
        Ok(serde_json::json!({ "blockhashes": blockhashes.iter().map(|bh| format!("0x{}", hex::encode(bh.as_bytes()))).collect::<Vec<_>>() }))
    })?;

    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::{decode_address, parse_gas_limit};

    #[test]
    fn decode_address_accepts_20_byte_hex() {
        let addr = format!("0x{}", "11".repeat(20));
        let decoded = decode_address(&addr).expect("address should decode");
        assert_eq!(decoded.len(), 20);
        assert!(decoded.iter().all(|b| *b == 0x11));
    }

    #[test]
    fn decode_address_rejects_wrong_length() {
        let addr = format!("0x{}", "aa".repeat(19));
        let err = decode_address(&addr).expect_err("address must be rejected");
        let text = format!("{err:?}");
        assert!(text.contains("Address must be 20 bytes"));
    }

    #[test]
    fn parse_gas_limit_accepts_hex_string() {
        let tx = serde_json::json!({ "gas": "0x5208" });
        let gas = parse_gas_limit(&tx).expect("hex gas should parse");
        assert_eq!(gas, 21_000);
    }

    #[test]
    fn parse_gas_limit_accepts_numeric_value() {
        let tx = serde_json::json!({ "gas": 42000 });
        let gas = parse_gas_limit(&tx).expect("numeric gas should parse");
        assert_eq!(gas, 42_000);
    }

    #[test]
    fn parse_gas_limit_accepts_decimal_string() {
        let tx = serde_json::json!({ "gas": "42000" });
        let gas = parse_gas_limit(&tx).expect("decimal string gas should parse");
        assert_eq!(gas, 42_000);
    }

    #[test]
    fn parse_gas_limit_rejects_invalid_type() {
        let tx = serde_json::json!({ "gas": { "value": 1 } });
        let err = parse_gas_limit(&tx).expect_err("object gas value must be rejected");
        let text = format!("{err:?}");
        assert!(text.contains("Invalid gas value"));
    }

    #[test]
    fn parse_gas_limit_uses_default_when_missing() {
        let tx = serde_json::json!({});
        let gas = parse_gas_limit(&tx).expect("default gas should be used");
        assert_eq!(gas, 10_000_000);
    }
}
