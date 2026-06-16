#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_system = "windows")]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{Emitter, Manager, State, generate_handler};
use tokio::time::sleep;
use sysinfo::System;

/* ─── Shared state for OS-level monitoring ─────── */

#[derive(Clone)]
struct OsState {
    node_status: Arc<RwLock<NodeStatusData>>,
    swarm_tasks: Arc<RwLock<Vec<SwarmTask>>>,
    system_metrics: Arc<RwLock<SystemMetricsData>>,
    sys_handle: Arc<std::sync::Mutex<System>>,
}

impl OsState {
    fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let initial_metrics = read_system_metrics(&sys);
        Self {
            node_status: Arc::new(RwLock::new(NodeStatusData {
                running: false,
                pid: None,
                block_height: 0,
                peer_count: 0,
                updated_at: chrono::Utc::now().to_rfc3339(),
            })),
            swarm_tasks: Arc::new(RwLock::new(vec![])),
            system_metrics: Arc::new(RwLock::new(initial_metrics)),
            sys_handle: Arc::new(std::sync::Mutex::new(sys)),
        }
    }
}

/* ─── Error type ──────────────────────────────── */

#[derive(Debug, Serialize)]
struct IpcError {
    code: &'static str,
    message: String,
    details: Option<String>,
}

impl IpcError {
    fn new(code: &'static str, message: &str, details: Option<String>) -> Self {
        Self {
            code,
            message: message.to_string(),
            details,
        }
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for IpcError {}

/* ─── Node status ───────────────────────────────── */

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NodeStatusData {
    running: bool,
    pid: Option<u32>,
    block_height: u64,
    peer_count: u32,
    updated_at: String,
}

/* ─── Swarm task models ────────────────────────── */

#[derive(Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwarmTask {
    id: String,
    name: String,
    status: String,
    agent: String,
    priority: u8,
    created_at: String,
}

/* ─── System metrics ────────────────────────────── */

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SystemMetricsData {
    cpu: CpuMetrics,
    memory: MemoryMetrics,
    disk: Vec<DiskMetrics>,
    updated_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CpuMetrics {
    usage_percent: f32,
    cores: u32,
    frequency: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MemoryMetrics {
    used: u64,
    total: u64,
    usage_percent: f32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiskMetrics {
    name: String,
    used: u64,
    total: u64,
    usage_percent: f32,
}

/* ─── Tauri commands ────────────────────────────── */

#[tauri::command]
async fn get_node_status(state: State<'_, OsState>) -> Result<NodeStatusData, IpcError> {
    let status = state.node_status.read().expect("node_status read lock");
    Ok(status.clone())
}

#[tauri::command]
async fn launch_node() -> Result<String, IpcError> {
    // Spawn x3-chain-node as a background process via shell plugin.
    // The frontend invokes this command; the actual spawning happens
    // through the tauri-plugin-shell scope configured in tauri.conf.json.
    // For a direct spawn: std::process::Command::new("x3-chain-node")
    //     .args(["--chain", "local", "--tmp"])
    //     .spawn()
    //     .map_err(|e| IpcError::new("NODE_SPAWN", &format!("Failed to launch node: {e}"), None))?;

    // Return success; the node status monitor will pick up the running process.
    Ok("node_launch_requested".to_string())
}

#[tauri::command]
async fn stop_node() -> Result<String, IpcError> {
    Ok("node_stop_requested".to_string())
}

#[tauri::command]
async fn swarm_get_tasks(state: State<'_, OsState>) -> Result<Vec<SwarmTask>, IpcError> {
    // Try to fetch from x3-swarm-api at :8787; fall back to cached state.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|e| IpcError::new("HTTP_CLIENT", &format!("{e}"), None))?;

    if let Ok(resp) = client
        .get("http://127.0.0.1:8787/tasks")
        .send()
        .await
    {
        if let Ok(tasks) = resp.json::<Vec<SwarmTask>>().await {
            let mut cache = state.swarm_tasks.write().expect("swarm_tasks write lock");
            *cache = tasks.clone();
            return Ok(tasks);
        }
    }

    // Fall back to cached tasks.
    let cache = state.swarm_tasks.read().expect("swarm_tasks read lock");
    Ok(cache.clone())
}

#[tauri::command]
async fn swarm_get_health() -> Result<serde_json::Value, IpcError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|e| IpcError::new("HTTP_CLIENT", &format!("{e}"), None))?;

    let resp = client
        .get("http://127.0.0.1:8787/health")
        .send()
        .await
        .map_err(|e| IpcError::new("SWARM_API", &format!("Swarm health endpoint unreachable: {e}"), None))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| {
        IpcError::new("SWARM_API", &format!("Invalid health response: {e}"), None)
    })?;

    Ok(json)
}

#[tauri::command]
async fn get_system_metrics(state: State<'_, OsState>) -> Result<SystemMetricsData, IpcError> {
    Ok(state.system_metrics.read().expect("system_metrics lock").clone())
}

#[tauri::command]
async fn swarm_approve_task(task_id: String) -> Result<String, IpcError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|e| IpcError::new("HTTP_CLIENT", &format!("{e}"), None))?;

    let resp = client
        .post(format!("http://127.0.0.1:8787/approve/{}", task_id))
        .send()
        .await
        .map_err(|e| IpcError::new("SWARM_API", &format!("Approve failed: {e}"), None))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| {
        IpcError::new("SWARM_API", &format!("Invalid approve response: {e}"), None)
    })?;

    Ok(json.to_string())
}

#[tauri::command]
async fn swarm_reject_task(task_id: String) -> Result<String, IpcError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|e| IpcError::new("HTTP_CLIENT", &format!("{e}"), None))?;

    let resp = client
        .post(format!("http://127.0.0.1:8787/reject/{}", task_id))
        .send()
        .await
        .map_err(|e| IpcError::new("SWARM_API", &format!("Reject failed: {e}"), None))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| {
        IpcError::new("SWARM_API", &format!("Invalid reject response: {e}"), None)
    })?;

    Ok(json.to_string())
}

/* ─── System metrics helpers ──────────────────── */

fn read_system_metrics(sys: &System) -> SystemMetricsData {
    let cpu_info = sys.global_cpu_info();
    let cpu_usage = cpu_info.cpu_usage();
    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();

    SystemMetricsData {
        cpu: CpuMetrics {
            usage_percent: cpu_usage,
            cores: sys.cpus().len() as u32,
            frequency: sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
        },
        memory: MemoryMetrics {
            used: used_memory * 1024,
            total: total_memory * 1024,
            usage_percent: if total_memory > 0 {
                (used_memory as f32 / total_memory as f32) * 100.0
            } else {
                0.0
            },
        },
        disk: vec![DiskMetrics {
            name: "System".into(),
            used: used_memory * 1024,
            total: total_memory * 1024,
            usage_percent: if total_memory > 0 {
                (used_memory as f32 / total_memory as f32) * 100.0
            } else {
                0.0
            },
        }],
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

/* ─── Background monitor tick ──────────────────── */

fn start_os_monitor(app: tauri::AppHandle, state: OsState) {
    tauri::async_runtime::spawn(async move {
        loop {
            sleep(Duration::from_millis(5000)).await;

            // Refresh system metrics.
            {
                let mut sys = state.sys_handle.lock().expect("sys_handle lock");
                sys.refresh_cpu();
                sys.refresh_memory();
                let metrics = read_system_metrics(&sys);
                drop(sys);
                *state.system_metrics.write().expect("system_metrics lock") = metrics;
            }

            // Poll node status via RPC.
            if let Ok(client) = reqwest::Client::builder()
                .timeout(Duration::from_millis(800))
                .build()
            {
                let rpc_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "system_health",
                    "params": []
                });

                if let Ok(resp) = client
                    .post("http://127.0.0.1:9933")
                    .json(&rpc_body)
                    .send()
                    .await
                {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let peers = json
                            .get("result")
                            .and_then(|r| r.get("peers"))
                            .and_then(|p| p.as_u64())
                            .unwrap_or(0) as u32;
                        let is_syncing = json
                            .get("result")
                            .and_then(|r| r.get("isSyncing"))
                            .and_then(|s| s.as_bool())
                            .unwrap_or(true);

                        let mut status = state.node_status.write().expect("node_status write lock");
                        status.running = !is_syncing || peers > 0;
                        status.peer_count = peers;
                        status.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                }
            }

            // Emit status update to frontend.
            let status_snapshot = state.node_status.read().expect("node_status read lock").clone();
            let _ = app.emit("os:node_status", status_snapshot);

            let metrics_snapshot = state.system_metrics.read().expect("system_metrics lock").clone();
            let _ = app.emit("os:system_metrics", metrics_snapshot);
        }
    });
}

/* ─── Application entry point ─────────────────── */

fn main() {
    let os_state = OsState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .manage(os_state.clone())
        .invoke_handler(generate_handler![
            get_node_status,
            launch_node,
            stop_node,
            swarm_get_tasks,
            swarm_get_health,
            get_system_metrics,
            swarm_approve_task,
            swarm_reject_task,
        ])
        .setup(move |app| {
            start_os_monitor(app.handle().clone(), os_state.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run tauri-os application");
}