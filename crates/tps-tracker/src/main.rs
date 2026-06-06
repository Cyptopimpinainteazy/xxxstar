use std::env;
/// X3 Chain TPS Tracker Binary
///
/// Runs the TPS tracking service that polls the blockchain and stores metrics in InfluxDB
use tps_tracker::{TpsTracker, TpsTrackerConfig};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load config from environment or use defaults
    let config = TpsTrackerConfig {
        rpc_url: env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:9944".to_string()),
        influx_url: env::var("INFLUX_URL").unwrap_or_else(|_| "http://localhost:8086".to_string()),
        influx_db: env::var("INFLUX_DB").unwrap_or_else(|_| "x3_chain_tps".to_string()),
        influx_token: env::var("INFLUX_TOKEN").unwrap_or_else(|_| "x3-chain-key".to_string()),
        poll_interval_secs: env::var("POLL_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        buffer_size: env::var("BUFFER_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
    };

    let mut tracker = TpsTracker::new(config);
    tracker.run().await
}
