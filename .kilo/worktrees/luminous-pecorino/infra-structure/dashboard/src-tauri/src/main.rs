#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::{Builder, generate_handler};

#[derive(Serialize, Deserialize)]
struct ServiceStatus {
    name: String,
    port: u16,
    healthy: bool,
}

#[tauri::command]
async fn check_services() -> Result<Vec<ServiceStatus>, String> {
    let services = vec![
        ("GPU Lane 1", 9001),
        ("GPU Lane 2", 9002),
        ("GPU Lane 3", 9003),
        ("TPS Bridge", 9999),
        ("Validator Registry", 7001),
        ("RPC Proxy", 8899),
        ("Admin API", 7777),
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for (name, port) in services {
        let url = format!("http://127.0.0.1:{}/health", port);
        let healthy = client.get(&url).send().await.map(|r| r.status().is_success()).unwrap_or(false);
        results.push(ServiceStatus {
            name: name.to_string(),
            port,
            healthy,
        });
    }

    Ok(results)
}

#[tauri::command]
fn get_app_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": "Inferstructor Dashboard",
        "version": "1.0.0",
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }))
}

fn main() {
    Builder::default()
        .invoke_handler(generate_handler![
            check_services,
            get_app_info,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Inferstructor Dashboard");
}
