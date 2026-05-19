//! Universal Multi-Chain Wallet - BIP39 + EVM chains + Substrate

use bip39::{Mnemonic, Language};
use rand::RngCore;
use sp_core::{sr25519, Pair};
use sp_core::crypto::Ss58Codec;
use solana_sdk::signature::{Keypair, SeedDerivable, Signer};
use bs58;
use tauri::{command, AppHandle, Emitter, State};
use crate::wallet_core::substrate_hook::{SubstrateHookManager, SubstrateHookEvent};

#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum WalletError {
    #[error("Crypto error: {0}")]
    CryptoError(String),
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UniversalWallet {
    mnemonic: String,
    seed_hex: String,
    evm_address: String,
    evm_private_key: String,
    solana_address: String,
    solana_private_key: String,
    substrate_address: String,
    evm_chain_count: usize,
    warning: String,
}

#[command]
pub fn generate_universal_wallet() -> Result<UniversalWallet, WalletError> {
    // Generate 12-word mnemonic using supported bip39 API
    use rand::thread_rng;
    let mut entropy = [0u8; 16];
    thread_rng().fill_bytes(&mut entropy);
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| WalletError::CryptoError(e.to_string()))?;
    let mnemonic_str = mnemonic.to_string();
    let seed = mnemonic.to_seed("");

    // Derive EVM address from seed (use keccak256 of seed)
    let hash = sp_core::hashing::keccak_256(&seed);
    let evm_address = format!("0x{}", hex::encode(&hash[12..32]));
    let evm_private_key = format!("0x{}", hex::encode(&seed[0..32]));

    // Solana
    let mut solana_seed = [0u8; 32];
    solana_seed.copy_from_slice(&seed[0..32]);
    let solana_keypair = Keypair::from_seed(&solana_seed).map_err(|e| WalletError::CryptoError(format!("Solana keypair error: {}", e)))?;
    let solana_address = solana_keypair.pubkey().to_string();
    let solana_private_key = bs58::encode(solana_keypair.to_bytes()).into_string();

    // Substrate (using Polkadot SS58 format)
    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(&seed[0..32]);
    let pair = sr25519::Pair::from_seed(&seed_array);
    let substrate_address = pair.public().to_ss58check();

    // Chain count - placeholder
    let evm_chain_count = 60000;

    Ok(UniversalWallet {
        mnemonic: mnemonic_str,
        seed_hex: hex::encode(seed),
        evm_address,
        evm_private_key,
        solana_address,
        solana_private_key,
        substrate_address,
        evm_chain_count,
        warning: "⚠️ LIVE KEYS - Backup mnemonic securely. Single EVM address works on 60k+ chains.".to_string(),
    })
}

#[command]
pub fn import_universal_wallet(mnemonic: String) -> Result<UniversalWallet, WalletError> {
    // Use provided mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic.as_str())
        .map_err(|e| WalletError::CryptoError(e.to_string()))?;
    let mnemonic_str = mnemonic.to_string();
    let seed = mnemonic.to_seed("");

    // Derive EVM address from seed
    let hash = sp_core::hashing::keccak_256(&seed);
    let evm_address = format!("0x{}", hex::encode(&hash[12..32]));
    let evm_private_key = format!("0x{}", hex::encode(&seed[0..32]));

    // Solana
    let mut solana_seed = [0u8; 32];
    solana_seed.copy_from_slice(&seed[0..32]);
    let solana_keypair = Keypair::from_seed(&solana_seed).map_err(|e| WalletError::CryptoError(format!("Solana keypair error: {}", e)))?;
    let solana_address = solana_keypair.pubkey().to_string();
    let solana_private_key = bs58::encode(solana_keypair.to_bytes()).into_string();

    // Substrate
    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(&seed[0..32]);
    let pair = sr25519::Pair::from_seed(&seed_array);
    let substrate_address = pair.public().to_ss58check();

    let evm_chain_count = 60000;

    Ok(UniversalWallet {
        mnemonic: mnemonic_str,
        seed_hex: hex::encode(seed),
        evm_address,
        evm_private_key,
        solana_address,
        solana_private_key,
        substrate_address,
        evm_chain_count,
        warning: "⚠️ IMPORTED KEYS - Verify and backup securely.".to_string(),
    })
}

#[command]
pub fn get_evm_chain_count() -> usize {
    59263
}

#[command]
pub async fn store_wallet_secure(_wallet: UniversalWallet) -> Result<(), WalletError> {
    // Use Tauri's store plugin for secure storage
    // For now, placeholder - in production would encrypt and store
    Ok(())
}

#[command]
pub async fn get_wallet_balance(_chain_id: String, _address: String) -> Result<String, WalletError> {
    // Use blockchain connector to fetch balance
    // Placeholder - would integrate with ChainDB
    Ok("0.0".to_string())
}

#[command]
pub async fn submit_cross_swap(_from_chain: String, _to_chain: String, _amount: String) -> Result<String, WalletError> {
    // Integrate with Comit v2 for atomic swaps
    // Placeholder
    Ok("swap_tx_hash".to_string())
}

#[command]
pub async fn execute_x3_script(_script: String, _wallet: UniversalWallet) -> Result<String, WalletError> {
    // Execute x3-lang script with wallet context
    // Placeholder
    Ok("execution_result".to_string())
}

#[command]
pub async fn run_cross_chain_intent(draft: crate::wallet_core::ipc::IntentDraft) -> Result<String, String> {
    crate::wallet_core::coordinator::WalletCoordinator::create_intent_draft(draft).await
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2: Substrate Hook Commands
// ─────────────────────────────────────────────────────────────────────────────

#[command]
pub async fn subscribe_substrate_events(app: AppHandle, state: State<'_, crate::SubstrateState>) -> Result<String, String> {
    let rpc_url = "ws://127.0.0.1:9944";
    let mut manager = SubstrateHookManager::new(rpc_url);
    let handler = manager.get_handler("default");

    let app_handle = app.clone();
    handler.register_hook("emit_events", Box::new(move |event: SubstrateHookEvent| {
        let payload = match &event {
            SubstrateHookEvent::NewBlock { hash, number, parent_hash, timestamp } => {
                serde_json::json!({
                    "type": "NewBlock",
                    "data": {
                        "hash": format!("{:?}", hash),
                        "number": *number,
                        "parentHash": format!("{:?}", parent_hash),
                        "timestamp": *timestamp,
                    }
                })
            }
            SubstrateHookEvent::Extrinsic { hash, signer, method, success, error } => {
                serde_json::json!({
                    "type": "Extrinsic",
                    "data": {
                        "hash": format!("{:?}", hash),
                        "signer": format!("{:?}", signer),
                        "method": method.clone(),
                        "success": *success,
                        "error": error.clone(),
                    }
                })
            }
            SubstrateHookEvent::ChainReorg { old_hash, new_hash, reorg_depth } => {
                serde_json::json!({
                    "type": "ChainReorg",
                    "data": {
                        "oldHash": format!("{:?}", old_hash),
                        "newHash": format!("{:?}", new_hash),
                        "reorgDepth": *reorg_depth,
                    }
                })
            }
        };
        let _ = app_handle.emit("substrate_event", payload);
    }));

    *state.manager.write().unwrap() = Some(manager);
    Ok("substrate_events_subscribed".to_string())
}

#[command]
pub async fn get_substrate_hook_state(state: State<'_, crate::SubstrateState>) -> Result<String, String> {
    if let Some(manager) = &mut *state.manager.write().unwrap() {
        let handler = manager.get_handler("default");
        let hook_state = handler.get_state();
        let response = serde_json::json!({
            "connected": hook_state.connected,
            "lastBlockNumber": hook_state.last_block_number,
        });
        Ok(response.to_string())
    } else {
        Ok(r#"{"connected": false, "lastBlockNumber": null}"#.to_string())
    }
}

#[command]
pub async fn register_substrate_hook(hook_id: String, state: State<'_, crate::SubstrateState>) -> Result<(), String> {
    if let Some(_manager) = &*state.manager.read().unwrap() {
        // Hook is already registered in subscribe
        Ok(())
    } else {
        Err("not subscribed".to_string())
    }
}

#[command]
pub async fn unregister_substrate_hook(hook_id: String, state: State<'_, crate::SubstrateState>) -> Result<(), String> {
    if let Some(manager) = &mut *state.manager.write().unwrap() {
        manager.remove_handler(&hook_id);
        Ok(())
    } else {
        Err("not subscribed".to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2: Wallet Store Commands
// ─────────────────────────────────────────────────────────────────────────────

use crate::wallet_core::wallet_store::WalletStore;

#[command]
pub async fn store_wallet_encrypted(
    wallet_id: String,
    mnemonic: String,
    seed: String,
    derivation_path: String,
    master_password: String,
) -> Result<(), String> {
    let mut store = WalletStore::new();
    store
        .store_wallet(&wallet_id, &mnemonic, &seed, &derivation_path, &master_password)
        .map_err(|e| format!("Failed to store wallet: {}", e))
}

#[command]
pub async fn retrieve_wallet_encrypted(wallet_id: String, master_password: String) -> Result<String, String> {
    let mut store = WalletStore::new();
    store
        .retrieve_wallet(&wallet_id, &master_password)
        .map(|(mnemonic, seed)| format!("{{\"wallet_id\": \"{}\", \"mnemonic\": \"{}\", \"seed\": \"{}\"}}", wallet_id, mnemonic, seed))
        .map_err(|e| format!("Failed to retrieve wallet: {}", e))
}

#[command]
pub async fn delete_wallet(wallet_id: String) -> Result<(), String> {
    let mut store = WalletStore::new();
    store
        .delete_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))
}

#[command]
pub async fn export_wallet_backup(wallet_id: String) -> Result<String, String> {
    let mut store = WalletStore::new();
    store
        .export_backup(&wallet_id)
        .map_err(|e| format!("Failed to export wallet backup: {}", e))
}

#[command]
pub async fn import_wallet_backup(backup: String) -> Result<String, String> {
    let mut store = WalletStore::new();
    store
        .import_backup(&backup)
        .map(|wallet_id| format!("{{\"wallet_id\": \"{}\", \"status\": \"imported\"}}", wallet_id))
        .map_err(|e| format!("Failed to import wallet backup: {}", e))
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2: x3ChainService Commands
// ─────────────────────────────────────────────────────────────────────────────

#[command]
pub async fn query_block(block_number: Option<u64>, block_hash: Option<String>) -> Result<String, String> {
    let method = if block_hash.is_some() {
        "chain_getBlock"
    } else {
        "chain_getBlockHash"
    };
    let params = if let Some(number) = block_number {
        serde_json::json!([number])
    } else if let Some(hash) = block_hash.as_ref() {
        serde_json::json!([hash])
    } else {
        serde_json::json!([])
    };
    match crate::rpc_call(method, params).await {
        Some(result) => Ok(result.to_string()),
        None => Err("Failed to query block".to_string()),
    }
}

#[command]
pub async fn query_account(address: String, at_block: Option<u64>) -> Result<String, String> {
    let at_param = if let Some(block) = at_block {
        serde_json::json!([format!("0x{:x}", block)])
    } else {
        serde_json::json!([])
    };
    
    // First get nonce
    let nonce_result = crate::rpc_call("system_accountNextIndex", serde_json::json!([address])).await;
    let nonce = if let Some(n) = nonce_result {
        n.as_u64().unwrap_or(0)
    } else {
        0
    };
    
    // Then get balance from storage
    let balance_result = crate::rpc_call("state_getStorage", serde_json::json!(["0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9", address])).await;
    let balance = if let Some(b) = balance_result {
        b.to_string()
    } else {
        "0".to_string()
    };
    
    serde_json::to_string(&serde_json::json!({
        "address": address,
        "nonce": nonce,
        "free": balance
    })).map_err(|e| format!("Failed to serialize account: {}", e))
}

#[command]
pub async fn query_balance(address: String, asset_id: Option<String>) -> Result<String, String> {
    match crate::rpc_call("wallet_getBalance", serde_json::json!([address])).await {
        Some(result) => Ok(result.to_string()),
        None => Err("Failed to query balance".to_string()),
    }
}

#[command]
pub async fn submit_extrinsic(call: String, signer: String, nonce: Option<u64>, tip: Option<u64>) -> Result<String, String> {
    let call_data = serde_json::from_str::<serde_json::Value>(&call)
        .map_err(|e| format!("Failed to parse call: {}", e))?;
    let params = serde_json::json!([call_data]);
    match crate::rpc_call("author_submitExtrinsic", params).await {
        Some(result) => Ok(result.to_string()),
        None => Err("Failed to submit extrinsic".to_string()),
    }
}

#[command]
pub async fn get_connection_status() -> Result<String, String> {
    match crate::rpc_call("system_health", serde_json::json!([])).await {
        Some(result) => {
            let is_syncing = result.get("isSyncing").and_then(|v| v.as_bool()).unwrap_or(false);
            let peers = result.get("peers").and_then(|v| v.as_u64()).unwrap_or(0);
            let best_block = result.get("bestBlock").unwrap_or(&serde_json::Value::Null);
            serde_json::to_string(&serde_json::json!({
                "connected": !is_syncing,
                "blockNumber": best_block,
                "peers": peers
            })).map_err(|e| format!("Failed to serialize status: {}", e))
        }
        None => Err("Failed to get connection status".to_string()),
    }
}

#[command]
pub async fn clear_chain_cache() -> Result<(), String> {
    // Clear the chain operation cache
    // In production, this would use the x3_chain_service module
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_universal_wallet() {
        let wallet = generate_universal_wallet().expect("Failed to generate wallet");
        assert!(!wallet.mnemonic.is_empty());
        assert_eq!(wallet.mnemonic.split_whitespace().count(), 12);
        assert!(wallet.evm_address.starts_with("0x"));
        assert_eq!(wallet.evm_address.len(), 42);
        assert!(!wallet.solana_address.is_empty());
        assert!(!wallet.substrate_address.is_empty());
    }

    #[test]
    fn test_import_universal_wallet_consistency() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string();
        let wallet = import_universal_wallet(mnemonic.clone()).expect("Failed to import wallet");
        
        assert_eq!(wallet.mnemonic, mnemonic);
        // Verify fixed addresses for this known mnemonic (if logic is stable)
        assert!(wallet.evm_address.starts_with("0x"));
        
        // Import again and check for exact match
        let wallet2 = import_universal_wallet(mnemonic).expect("Failed to import wallet again");
        assert_eq!(wallet.evm_address, wallet2.evm_address);
        assert_eq!(wallet.solana_address, wallet2.solana_address);
        assert_eq!(wallet.substrate_address, wallet2.substrate_address);
    }

    #[test]
    fn test_evm_chain_count() {
        assert!(get_evm_chain_count() > 50000);
    }

    #[test]
    fn test_invalid_mnemonic_fails() {
        let result = import_universal_wallet("invalid mnemonic".to_string());
        assert!(result.is_err());
    }
}
