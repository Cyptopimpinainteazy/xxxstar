use tauri::command;
use serde::Serialize;
use std::process::Command;

/// Allowed system commands — allowlist-only execution to prevent command injection.
/// Each entry maps a human-readable ID to a fixed command + args.
const ALLOWED_COMMANDS: &[(&str, &str, &[&str])] = &[
    ("disk-usage",       "df",       &["-h"]),
    ("processes",        "ps",       &["aux", "--sort=-pcpu"]),
    ("network-ifaces",   "ip",       &["addr", "show"]),
    ("docker-containers","docker",   &["ps", "-a", "--format", "table {{.ID}}\\t{{.Names}}\\t{{.Status}}\\t{{.Ports}}"]),
    ("docker-stats",     "docker",   &["stats", "--no-stream", "--format", "table {{.Name}}\\t{{.CPUPerc}}\\t{{.MemUsage}}"]),
    ("uptime",           "uptime",   &[]),
    ("hostname",         "hostname", &[]),
    ("free-memory",      "free",     &["-h"]),
    ("journal-recent",   "journalctl", &["--no-pager", "-n", "100", "--user"]),
    ("gpu-info",         "nvidia-smi", &["--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu", "--format=csv,noheader"]),
    ("system-info",      "uname",    &["-a"]),
];

#[derive(Serialize)]
pub struct AllowedCommand {
    pub id: String,
    pub label: String,
}

/// Returns the list of allowed command IDs so the frontend can display buttons.
#[command]
pub fn admin_list_commands() -> Vec<AllowedCommand> {
    ALLOWED_COMMANDS
        .iter()
        .map(|(id, bin, _)| AllowedCommand {
            id: id.to_string(),
            label: format!("{} ({})", id, bin),
        })
        .collect()
}

/// Execute a command from the allowlist by its ID. Rejects anything not in the list.
#[command]
pub async fn run_system_command(cmd: String) -> Result<String, String> {
    let entry = ALLOWED_COMMANDS
        .iter()
        .find(|(id, _, _)| *id == cmd.as_str())
        .ok_or_else(|| format!("Command '{}' is not in the allowed list", cmd))?;

    let (_, bin, args) = entry;
    let output = Command::new(bin)
        .args(*args)
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", bin, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Ok(format!("{}\n{}", stdout, stderr))
    }
}

/// Get system overview — CPU, memory, disk, uptime in a structured format.
#[command]
pub async fn admin_system_overview() -> Result<AdminSystemOverview, String> {
    let uptime_out = Command::new("uptime")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let hostname = Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let kernel = Command::new("uname")
        .arg("-r")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    Ok(AdminSystemOverview {
        hostname,
        kernel,
        uptime: uptime_out,
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    })
}

#[derive(Serialize)]
pub struct AdminSystemOverview {
    pub hostname: String,
    pub kernel: String,
    pub uptime: String,
    pub platform: String,
    pub arch: String,
}

/// Check health of local services by probing their HTTP endpoints.
#[command]
pub async fn admin_check_services() -> Result<Vec<ServiceHealth>, String> {
    let services = vec![
        ("TPS Bridge",          9999, "/stats"),
        ("Validator Registry",  7001, "/health"),
        ("RPC Proxy",           8899, "/stats"),
        ("Admin API",           7777, "/health"),
        ("GPU Lane 1",          9001, "/health"),
        ("GPU Lane 2",          9002, "/health"),
        ("GPU Lane 3",          9003, "/health"),
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for (name, port, path) in services {
        let url = format!("http://127.0.0.1:{}{}", port, path);
        let start = std::time::Instant::now();
        let (healthy, status_code) = match client.get(&url).send().await {
            Ok(r) => (r.status().is_success(), r.status().as_u16()),
            Err(_) => (false, 0),
        };
        let latency_ms = start.elapsed().as_millis() as u32;
        results.push(ServiceHealth {
            name: name.to_string(),
            port,
            healthy,
            status_code,
            latency_ms,
        });
    }

    Ok(results)
}

#[derive(Serialize)]
pub struct ServiceHealth {
    pub name: String,
    pub port: u16,
    pub healthy: bool,
    pub status_code: u16,
    pub latency_ms: u32,
}
