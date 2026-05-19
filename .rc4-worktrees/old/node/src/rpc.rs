//! X3 Chain node RPC module wiring.
//!
//! Assembles the full JSON-RPC module used by the service.
//! Merges substrate system RPCs, transaction-payment RPCs, chain RPCs,
//! and the Frontier-compatible ETH/SVM RPC provided by `rpc_frontier`.

use flash_finality::FlashFinalityGadget;
use jsonrpsee::{types::ErrorObjectOwned, RpcModule};
use sc_client_api::BlockBackend;
use sc_rpc::chain::ChainApiServer;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use std::sync::{Arc, Mutex};
use x3_atomic_trade::{AMMPool, SwapRPCServer};
use x3_chain_runtime::{opaque::Block, AccountId, AssetId, Balance};
use x3_common::{
    signing::{Ed25519Signer, KeyType, Secp256k1Signer, Signer, Sr25519Signer},
    weight_metering::{ComputeMeter, GasMeter, Operation, WeightConfig, WeightMeter},
};
use x3_rpc::{
    SwapRequest, ValidatorRpcApi, WalletDexApi, WalletDexRpc, WalletServiceApi, WalletServiceRpc,
};

use crate::rpc_middleware::RateLimiter;
use crate::service::FullClient;

type RpcError = Box<dyn std::error::Error + Send + Sync>;
type JsonRpseeError = ErrorObjectOwned;

/// Helper to create custom JSON-RPC errors.
fn custom_error(message: impl Into<String>) -> JsonRpseeError {
    ErrorObjectOwned::owned(-32603, message.into(), None::<()>)
}

/// Decode hex string with "0x" prefix to 32-byte array.
fn decode_hex_32(value: &str, label: &str) -> Result<[u8; 32], JsonRpseeError> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    let bytes =
        hex::decode(stripped).map_err(|e| custom_error(format!("{label} decode failed: {e}")))?;
    if bytes.len() != 32 {
        return Err(custom_error(format!(
            "{label} must be 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// Parse u128 value from JSON.
fn parse_u128_value(
    value: Option<&serde_json::Value>,
    label: &str,
) -> Result<u128, JsonRpseeError> {
    let val = value.ok_or_else(|| custom_error(format!("Missing {label}")))?;
    if let Some(s) = val.as_str() {
        s.parse::<u128>()
            .map_err(|e| custom_error(format!("{label} parse failed: {e}")))
    } else if let Some(n) = val.as_u64() {
        Ok(n as u128)
    } else {
        Err(custom_error(format!("{label} must be string or number")))
    }
}

/// Full RPC extension creation.
///
/// Called by the service to build the RPC module for each connection.
pub fn create_full<P>(
    client: Arc<FullClient>,
    pool: Arc<P>,
    _gadget: Option<Arc<FlashFinalityGadget>>,
    _limiter: Arc<RateLimiter>,
    subscription_executor: sc_rpc::SubscriptionTaskExecutor,
) -> Result<RpcModule<()>, RpcError>
where
    P: TransactionPool + Sync + Send + 'static,
    FullClient: ProvideRuntimeApi<Block>,
    FullClient: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
    FullClient: BlockBackend<Block>,
    <FullClient as ProvideRuntimeApi<Block>>::Api: BlockBuilder<Block>,
    <FullClient as ProvideRuntimeApi<Block>>::Api:
        substrate_frame_rpc_system::AccountNonceApi<Block, x3_chain_runtime::AccountId, u32>,
    <FullClient as ProvideRuntimeApi<Block>>::Api:
        pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<
            Block,
            x3_chain_runtime::Balance,
        >,
    <FullClient as ProvideRuntimeApi<Block>>::Api:
        pallet_x3_kernel::AtlasKernelRuntimeApi<Block, AccountId, Balance, AssetId>,
{
    let mut module = RpcModule::new(());

    let system_rpc = substrate_frame_rpc_system::System::new(client.clone(), pool);
    module.merge(substrate_frame_rpc_system::SystemApiServer::into_rpc(
        system_rpc,
    ))?;

    let tx_payment_rpc = pallet_transaction_payment_rpc::TransactionPayment::new(client.clone());
    module.merge(
        pallet_transaction_payment_rpc::TransactionPaymentApiServer::into_rpc(tx_payment_rpc),
    )?;

    #[cfg(feature = "frontier")]
    {
        // Merge Frontier ETH-compatible JSON-RPC endpoints.
        let frontier_module = crate::rpc_frontier::create_frontier_rpc(client.clone())?;
        module.merge(frontier_module)?;

        // Merge SVM-compatible JSON-RPC endpoints.
        let svm_module = crate::rpc_frontier::create_svm_rpc(client.clone())?;
        module.merge(svm_module)?;
    }

    // Initialize DEX RPC integration.
    let wallet_dex = Arc::new(WalletDexRpc::<Block, FullClient>::new(client.clone()));
    let swap_rpc = Arc::new(Mutex::new(SwapRPCServer::new()));

    // Register default AMM pool (X3/USDC).
    {
        let mut engine = swap_rpc
            .lock()
            .map_err(|_| custom_error("Swap engine lock poisoned"))?;

        let _ = engine.register_pool(AMMPool {
            id: "default_x3_usdc".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 10_000_000_000_000,
            reserve_b: 10_000_000_000_000,
            fee_bps: 30,
            tvl_usd: 20_000_000.0,
        });
    }

    // Register walletDex_estimateSwap RPC method.
    let wallet_dex_estimate = wallet_dex.clone();
    module.register_method(
        "walletDex_estimateSwap",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let request = SwapRequest {
                token_in: decode_hex_32(
                    req.get("token_in")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_in"))?,
                    "token_in",
                )?,
                token_out: decode_hex_32(
                    req.get("token_out")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_out"))?,
                    "token_out",
                )?,
                amount_in: parse_u128_value(req.get("amount_in"), "amount_in")?,
                min_amount_out: parse_u128_value(req.get("min_amount_out"), "min_amount_out")?,
                wallet_id: decode_hex_32(
                    req.get("wallet_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing wallet_id"))?,
                    "wallet_id",
                )?,
                require_approval: req
                    .get("require_approval")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                approval_threshold: parse_u128_value(
                    req.get("approval_threshold"),
                    "approval_threshold",
                )?,
            };

            wallet_dex_estimate
                .estimate_swap(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("walletDex_estimateSwap failed: {e}")))
        },
    )?;

    // Register walletDex_executeSwap RPC method.
    let wallet_dex_execute = wallet_dex.clone();
    module.register_method(
        "walletDex_executeSwap",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let req: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;
            let request = SwapRequest {
                token_in: decode_hex_32(
                    req.get("token_in")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_in"))?,
                    "token_in",
                )?,
                token_out: decode_hex_32(
                    req.get("token_out")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing token_out"))?,
                    "token_out",
                )?,
                amount_in: parse_u128_value(req.get("amount_in"), "amount_in")?,
                min_amount_out: parse_u128_value(req.get("min_amount_out"), "min_amount_out")?,
                wallet_id: decode_hex_32(
                    req.get("wallet_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| custom_error("Missing wallet_id"))?,
                    "wallet_id",
                )?,
                require_approval: req
                    .get("require_approval")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                approval_threshold: parse_u128_value(
                    req.get("approval_threshold"),
                    "approval_threshold",
                )?,
            };

            wallet_dex_execute
                .execute_swap(request, vec![])
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("walletDex_executeSwap failed: {e}")))
        },
    )?;

    // Initialize Wallet Service RPC
    let wallet_service = Arc::new(WalletServiceRpc::<Block, FullClient>::new(client.clone()));

    // Register wallet service RPC methods
    module.register_method("wallet_createWallet", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::CreateWalletRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .create_wallet(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_createWallet failed: {e}")))
        }
    })?;

    module.register_method("wallet_importWallet", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::ImportWalletRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .import_wallet(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_importWallet failed: {e}")))
        }
    })?;

    module.register_method("wallet_backupWallet", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::BackupWalletRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .backup_wallet(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_backupWallet failed: {e}")))
        }
    })?;

    module.register_method("wallet_getBalance", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::GetBalanceRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .get_balance(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getBalance failed: {e}")))
        }
    })?;

    module.register_method("wallet_signTransaction", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::SignTransactionRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .sign_transaction(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_signTransaction failed: {e}")))
        }
    })?;

    module.register_method("wallet_submitTransaction", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::SubmitTransactionRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .submit_transaction(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_submitTransaction failed: {e}")))
        }
    })?;

    module.register_method("wallet_getTransactions", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::GetTransactionsRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .get_transactions(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getTransactions failed: {e}")))
        }
    })?;

    module.register_method("wallet_getWalletStatus", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::GetWalletStatusRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .get_wallet_status(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getWalletStatus failed: {e}")))
        }
    })?;

    module.register_method("wallet_listWallets", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::ListWalletsRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .list_wallets(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_listWallets failed: {e}")))
        }
    })?;

    module.register_method("wallet_setNetwork", {
        let wallet_service = wallet_service.clone();
        move |params: jsonrpsee::types::Params<'_>,
              _,
              _|
              -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let params: serde_json::Value = params.parse()?;
            let request: x3_rpc::SetNetworkRequest = serde_json::from_value(params)
                .map_err(|e| custom_error(format!("Invalid request: {e}")))?;
            wallet_service
                .set_network(request)
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_setNetwork failed: {e}")))
        }
    })?;

    module.register_method("wallet_getNetworks", {
        let wallet_service = wallet_service.clone();
        move |_, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            wallet_service
                .get_networks()
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .map_err(|e| custom_error(format!("wallet_getNetworks failed: {e}")))
        }
    })?;

    // Register signing RPC methods
    module.register_method(
        "x3_sign_ed25519",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, secret_hex): (String, String) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let secret = hex::decode(secret_hex.strip_prefix("0x").unwrap_or(&secret_hex))
                .map_err(|e| custom_error(format!("Invalid secret hex: {e}")))?;

            if secret.len() != 32 {
                return Err(custom_error("Secret key must be 32 bytes"));
            }

            let mut secret_array = [0u8; 32];
            secret_array.copy_from_slice(&secret);

            let signer = Ed25519Signer::from_secret_key(&secret_array);
            let signature = signer.sign(&message);

            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(signature.as_bytes())
            )))
        },
    )?;

    module.register_method(
        "x3_sign_secp256k1",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, secret_hex): (String, String) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let secret = hex::decode(secret_hex.strip_prefix("0x").unwrap_or(&secret_hex))
                .map_err(|e| custom_error(format!("Invalid secret hex: {e}")))?;

            if secret.len() != 32 {
                return Err(custom_error("Secret key must be 32 bytes"));
            }

            let mut secret_array = [0u8; 32];
            secret_array.copy_from_slice(&secret);

            let signer = Secp256k1Signer::from_secret_key(&secret_array)
                .map_err(|e| custom_error(format!("Invalid secret key: {e}")))?;
            let signature = signer.sign(&message);

            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(signature.as_bytes())
            )))
        },
    )?;

    module.register_method(
        "x3_sign_sr25519",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, secret_hex): (String, String) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let secret = hex::decode(secret_hex.strip_prefix("0x").unwrap_or(&secret_hex))
                .map_err(|e| custom_error(format!("Invalid secret hex: {e}")))?;

            if secret.len() != 32 {
                return Err(custom_error("Secret key must be 32 bytes"));
            }

            let mut secret_array = [0u8; 32];
            secret_array.copy_from_slice(&secret);

            let signer = Sr25519Signer::from_secret_key(&secret_array);
            let signature = signer.sign(&message);

            Ok(serde_json::Value::String(format!(
                "0x{}",
                hex::encode(signature.as_bytes())
            )))
        },
    )?;

    module.register_method(
        "x3_verify_signature",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let (message_hex, signature_hex, public_key_hex, key_type_str): (
                String,
                String,
                String,
                String,
            ) = params.parse()?;
            let message = hex::decode(message_hex.strip_prefix("0x").unwrap_or(&message_hex))
                .map_err(|e| custom_error(format!("Invalid message hex: {e}")))?;
            let signature = hex::decode(signature_hex.strip_prefix("0x").unwrap_or(&signature_hex))
                .map_err(|e| custom_error(format!("Invalid signature hex: {e}")))?;
            let public_key =
                hex::decode(public_key_hex.strip_prefix("0x").unwrap_or(&public_key_hex))
                    .map_err(|e| custom_error(format!("Invalid public key hex: {e}")))?;

            let key_type = match key_type_str.to_lowercase().as_str() {
                "ed25519" => KeyType::Ed25519,
                "secp256k1" => KeyType::Secp256k1,
                "sr25519" => KeyType::Sr25519,
                _ => {
                    return Err(custom_error(
                        "Invalid key type. Must be ed25519, secp256k1, or sr25519",
                    ))
                }
            };

            let valid =
                x3_common::signing::verify_signature(&signature, &message, &public_key, key_type);

            Ok(serde_json::Value::Bool(valid))
        },
    )?;

    module.register_method(
        "x3_weight_meter",
        move |params, _, _| -> Result<serde_json::Value, jsonrpsee::types::ErrorObjectOwned> {
            let config: serde_json::Value = params.parse::<(serde_json::Value,)>().map(|(v,)| v)?;

            let max_compute_units = config
                .get("max_compute_units")
                .and_then(|v| v.as_u64())
                .unwrap_or(200_000);
            let max_gas = config
                .get("max_gas")
                .and_then(|v| v.as_u64())
                .unwrap_or(1_000_000);

            let mut meter = WeightMeter::new(WeightConfig {
                max_compute_units,
                max_gas,
                ..Default::default()
            });

            // Consume some compute units for demonstration
            meter
                .consume_compute(1000)
                .map_err(|e| custom_error(format!("Compute limit: {e}")))?;

            Ok(serde_json::json!({
                "remaining_compute": meter.remaining_compute(),
                "remaining_gas": meter.remaining_gas(),
                "consumed_compute": meter.consumed_compute(),
                "consumed_gas": meter.consumed_gas(),
            }))
        },
    )?;

    // Initialize Validator RPC
    let validator_rpc = x3_rpc::create_validator_rpc(std::sync::Arc::new(()))?;
    module.merge(validator_rpc)?;

    Ok(module)
}
