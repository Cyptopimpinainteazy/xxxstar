/// Quantum Kernel Voyager — Tauri v2 library crate.
///
/// Configures the app builder with system tray, window, plugins,
/// and all IPC command handlers.
mod chains;
mod commands;
mod error;
mod keystore;
mod x3_integration;

use commands::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            std::fs::create_dir_all(&data_dir).ok();

            let keystore_path = data_dir.join("keystore.json");
            let keystore = keystore::EncryptedKeystore::new(&keystore_path, "default-dev-password")
                .expect("failed to initialize keystore");

            let state = AppState {
                chain_manager: chains::ChainManager::new(),
                keystore: Arc::new(tokio::sync::Mutex::new(keystore)),
                data_dir,
            };

            app.manage(state);

            tracing::info!("application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect_chain,
            commands::disconnect_chain,
            commands::fetch_chain_status,
            commands::fetch_blocks,
            commands::sign_and_send_tx,
            commands::create_account,
            commands::list_accounts,
            commands::export_scene,
            commands::save_voyage,
            commands::load_voyage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
