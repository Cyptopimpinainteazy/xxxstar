//! GPU Swarm Node Binary
//!
//! Run a GPU swarm node that can execute distributed compute tasks.

#![allow(unused, dead_code, deprecated)]

use gpu_swarm::{
    admin::{run_admin, AdminState},
    config::SwarmConfig,
    coordinator::{CoordinatorConfig, SwarmCoordinator},
    gpu_backends::{
        cuda::CudaExecutor, vulkan::VulkanExecutor, webgpu::WebGpuExecutor, GpuExecutor,
    },
    network::{NetworkConfig, NetworkManager},
    node::{GpuBackend, GpuCapabilities, SwarmNode},
};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting GPU Swarm Node v{}", env!("CARGO_PKG_VERSION"));

    // Load or create configuration
    let config_path = PathBuf::from("swarm-config.toml");
    let config = if config_path.exists() {
        tracing::info!("Loading config from {:?}", config_path);
        SwarmConfig::from_file(&config_path)?
    } else {
        tracing::info!("Using default configuration");
        SwarmConfig::default()
    };

    // Detect GPU capabilities
    let gpu = detect_gpu_capabilities().await;
    tracing::info!(
        "Detected GPU: {} ({} VRAM, {} compute units)",
        gpu.device_name,
        format_bytes(gpu.total_vram),
        gpu.compute_units
    );

    // Create swarm node
    let node = SwarmNode::new(&config, gpu)?;
    tracing::info!("Node ID: {}", hex::encode(&node.id[..16]));

    // Create network manager
    let net_config = NetworkConfig::default();
    let mut network = NetworkManager::new(net_config)?;

    // Start network
    network.start().await?;
    tracing::info!("Network started");

    // Spawn local admin GUI server on 127.0.0.1:9101
    let admin_state = std::sync::Arc::new(tokio::sync::Mutex::new(AdminState::default()));
    let admin_state_clone = admin_state.clone();
    let admin_addr: std::net::SocketAddr = "127.0.0.1:9101".parse().unwrap();
    tokio::spawn(async move {
        tracing::info!("Starting admin UI on http://127.0.0.1:9101");
        run_admin(admin_state_clone, admin_addr).await;
    });

    // Simulate updating uptime, rewards and history in background
    let metrics_state = admin_state.clone();
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let mut s = metrics_state.lock().await;
            s.uptime_seconds = start.elapsed().as_secs();
            if s.enabled {
                // Simulate rewards accrual based on gpu level
                let rate = match s.gpu_level.as_str() {
                    "low" => 0.01,
                    "high" => 0.05,
                    _ => 0.02,
                };
                s.rewards += rate;
                // append to reward history every 5 seconds
                if (s.uptime_seconds % 5) == 0 {
                    let t = chrono::Utc::now().timestamp() as u64;
                    let r = s.rewards;
                    s.rewards_history
                        .push(gpu_swarm::admin::RewardPoint { t, rewards: r });
                    // keep only last 200 points
                    let rh_len = s.rewards_history.len();
                    if rh_len > 200 {
                        let remove = rh_len - 200;
                        s.rewards_history.drain(0..remove);
                    }
                }
                // simple scoring: base + rewards*10
                s.score = (100 + (s.rewards * 10.0) as u32) as u32;
            }
        }
    });

    tracing::info!("Node ready. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down...");
    network.stop();

    Ok(())
}

/// Detect GPU capabilities via configured backends.
async fn detect_gpu_capabilities() -> GpuCapabilities {
    let mut backends = Vec::new();
    let mut device_name = "Unknown GPU".to_string();
    let mut vendor = "Unknown".to_string();
    let mut total_vram = 0u64;
    let mut available_vram = 0u64;
    let mut compute_capability = None;

    if let Ok(cuda) = CudaExecutor::new().await {
        if cuda.is_available().await {
            if let Ok(devices) = cuda.list_devices().await {
                if let Some(d) = devices.first() {
                    backends.push(GpuBackend::Cuda);
                    device_name = d.name.clone();
                    total_vram = d.total_memory;
                    available_vram = d.available_memory;
                    vendor = if d.name.to_lowercase().contains("nvidia") {
                        "NVIDIA".to_string()
                    } else {
                        vendor
                    };
                    if let Some((major, minor)) = d
                        .compute_capability
                        .split_once('.')
                        .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
                    {
                        compute_capability = Some((major, minor));
                    }
                }
            }
        }
    }

    if let Ok(vulkan) = VulkanExecutor::new().await {
        if vulkan.is_available().await {
            backends.push(GpuBackend::Vulkan);
            if device_name == "Unknown GPU" {
                if let Ok(devices) = vulkan.list_devices().await {
                    if let Some(d) = devices.first() {
                        device_name = d.name.clone();
                        total_vram = d.total_memory;
                        available_vram = d.available_memory;
                    }
                }
            }
        }
    }

    if let Ok(webgpu) = WebGpuExecutor::new().await {
        if webgpu.is_available().await {
            backends.push(GpuBackend::WebGpu);
        }
    }

    if backends.is_empty() {
        backends.push(GpuBackend::Vulkan);
        total_vram = 8 * 1024 * 1024 * 1024;
        available_vram = 6 * 1024 * 1024 * 1024;
    }

    GpuCapabilities {
        backends,
        device_name,
        vendor,
        total_vram,
        available_vram,
        compute_units: 32,
        max_workgroup_size: 1024,
        max_threads: 32768,
        compute_capability,
        supports_fp64: true,
        supports_fp16: true,
        supports_tensor_cores: true,
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
