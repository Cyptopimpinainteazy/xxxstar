/// Tauri IPC command handlers.
///
/// Every public function here is annotated with `#[tauri::command]` and
/// registered in `main.rs`. Each returns `Result<T, AppError>` — never panics.
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tracing::info;

use crate::chains::{Block, ChainConfig, ChainManager, ChainStatus, SceneExport, SigningIntent, VoyageState};
use crate::error::AppError;
use crate::keystore::{EncryptedKeystore, KeyInfo};

/// Shared application state accessible by all commands.
pub struct AppState {
    pub chain_manager: ChainManager,
    pub keystore: Arc<tokio::sync::Mutex<EncryptedKeystore>>,
    pub data_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// Chain commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn connect_chain(
    chain_id: String,
    config: ChainConfig,
    state: State<'_, AppState>,
) -> Result<ChainStatus, AppError> {
    if chain_id.is_empty() {
        return Err(AppError::ValidationError("chain_id cannot be empty".into()));
    }
    state.chain_manager.connect(&chain_id, config).await
}

#[tauri::command]
pub async fn disconnect_chain(
    chain_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.chain_manager.disconnect(&chain_id).await
}

#[tauri::command]
pub async fn fetch_chain_status(
    chain_id: String,
    state: State<'_, AppState>,
) -> Result<ChainStatus, AppError> {
    state.chain_manager.get_status(&chain_id).await
}

#[tauri::command]
pub async fn fetch_blocks(
    chain_id: String,
    from_height: u64,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<Block>, AppError> {
    if limit > 1000 {
        return Err(AppError::ValidationError("limit must be <= 1000".into()));
    }
    state.chain_manager.fetch_blocks(&chain_id, from_height, limit).await
}

#[tauri::command]
pub async fn sign_and_send_tx(
    chain_id: String,
    intent: SigningIntent,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    if intent.account_id.is_empty() {
        return Err(AppError::ValidationError("account_id required".into()));
    }
    let ks = state.keystore.lock().await;
    let tx_bytes = serde_json::to_vec(&intent)?;
    let sig = ks.sign_transaction(&intent.account_id, &tx_bytes)?;
    info!(chain_id, sig_len = sig.len(), "transaction signed");
    // In production, broadcast the signed tx to the chain.
    // Return a mock tx hash for the scaffold.
    let tx_hash = format!("0x{}", &sig[..64]);
    Ok(tx_hash)
}

// ---------------------------------------------------------------------------
// Account / keystore commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn create_account(
    label: String,
    chain_type: String,
    state: State<'_, AppState>,
) -> Result<KeyInfo, AppError> {
    if label.is_empty() {
        return Err(AppError::ValidationError("label required".into()));
    }
    let mut ks = state.keystore.lock().await;
    ks.create_key(&label, &chain_type)
}

#[tauri::command]
pub async fn list_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<KeyInfo>, AppError> {
    let ks = state.keystore.lock().await;
    Ok(ks.list_keys())
}

// ---------------------------------------------------------------------------
// File / scene commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn export_scene(
    path: String,
    scene_data: SceneExport,
) -> Result<(), AppError> {
    let dest = PathBuf::from(&path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dest, &scene_data.scene_json)?;
    info!(?dest, "scene exported");
    Ok(())
}

#[tauri::command]
pub async fn save_voyage(
    voyage: VoyageState,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let path = state.data_dir.join("voyage.qkv");
    let json = serde_json::to_string_pretty(&voyage)?;
    std::fs::create_dir_all(&state.data_dir)?;
    std::fs::write(&path, json)?;
    info!(?path, "voyage saved");
    Ok(())
}

#[tauri::command]
pub async fn load_voyage(
    state: State<'_, AppState>,
) -> Result<Option<VoyageState>, AppError> {
    let path = state.data_dir.join("voyage.qkv");
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)?;
    let voyage: VoyageState = serde_json::from_str(&data)?;
    Ok(Some(voyage))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voyage_round_trip() {
        let v = VoyageState {
            schema_version: 1,
            game_state: serde_json::json!({"mode": "exploring"}),
            economy: serde_json::json!({"quantum_crystals": 100}),
            ship: serde_json::json!({"fuel": 80.0}),
            saved_at: 1700000000.0,
        };
        let json = serde_json::to_string(&v).unwrap();
        let v2: VoyageState = serde_json::from_str(&json).unwrap();
        assert_eq!(v2.schema_version, 1);
    }
}
