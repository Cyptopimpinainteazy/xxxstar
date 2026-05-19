// crates/gpu-swarm/src/monitoring/logging.rs
// Structured logging setup for ELK stack integration

use chrono::Utc;
use serde::Serialize;
use std::error::Error;
use std::fs::OpenOptions;
use std::path::Path;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Serialize)]
pub struct StructuredLog {
    pub timestamp: String,
    pub level: String,
    pub service: String,
    pub message: String,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub context: serde_json::Value,
}

pub fn setup_logging() -> Result<(), Box<dyn Error>> {
    let log_dir = Path::new("logs");
    if !log_dir.exists() {
        std::fs::create_dir(log_dir)?;
    }

    // JSON file logging layer
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/gpu-swarm.log")?;

    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(file)
        .with_span_events(FmtSpan::ACTIVE)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // Console logging layer
    let console_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(json_layer)
        .with(console_layer)
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug,tokio=info,hyper=info".parse().unwrap()),
        )
        .init();

    tracing::info!("✅ Structured logging initialized");
    Ok(())
}

/// Structured log message for ELK ingestion
pub fn log_structured(level: &str, service: &str, message: &str, context: serde_json::Value) {
    let log = StructuredLog {
        timestamp: Utc::now().to_rfc3339(),
        level: level.to_string(),
        service: service.to_string(),
        message: message.to_string(),
        trace_id: tracing::Span::current()
            .id()
            .map(|id| format!("{:x}", id.into_u64())),
        span_id: None,
        context,
    };

    match level {
        "error" => tracing::error!("{}", serde_json::to_string(&log).unwrap_or_default()),
        "warn" => tracing::warn!("{}", serde_json::to_string(&log).unwrap_or_default()),
        "info" => tracing::info!("{}", serde_json::to_string(&log).unwrap_or_default()),
        _ => tracing::debug!("{}", serde_json::to_string(&log).unwrap_or_default()),
    }
}

/// Macro for structured logging convenience
#[macro_export]
macro_rules! log_info {
    ($service:expr, $msg:expr, $ctx:expr) => {
        $crate::monitoring::logging::log_structured("info", $service, $msg, $ctx)
    };
}

#[macro_export]
macro_rules! log_error {
    ($service:expr, $msg:expr, $ctx:expr) => {
        $crate::monitoring::logging::log_structured("error", $service, $msg, $ctx)
    };
}
