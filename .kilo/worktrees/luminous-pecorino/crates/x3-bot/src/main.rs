#![allow(unused, dead_code, deprecated)]

mod api;
mod config;
mod executor;
mod market;
mod rpc_pool;
mod strategy;
mod telemetry;
mod tx_manager;
mod wallet;

use crate::config::BotConfig;
use crate::rpc_pool::RpcPool;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    info!("☢️ YOLO FINISHER — World's Fastest Profitable Bot Starting...");

    let cfg = match BotConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!("FATAL: Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let pool_evm = Arc::new(RpcPool::new(cfg.rpc_evm.clone())?);
    let pool_svm = Arc::new(RpcPool::new(cfg.rpc_svm.clone())?);

    if !pool_evm.health_check().await {
        error!("FATAL: All EVM RPC endpoints are unreachable or unhealthy.");
        std::process::exit(1);
    }

    if !pool_svm.health_check().await {
        error!("FATAL: All SVM RPC endpoints are unreachable or unhealthy.");
        std::process::exit(1);
    }

    info!("PREREQUISITES PASSED. Launching Execution Loop...");

    let prometheus_port = cfg.prometheus_port;
    tokio::spawn(async move {
        api::start_metrics_server(prometheus_port).await;
    });

    if let Err(e) = executor::run(cfg, pool_evm, pool_svm).await {
        error!("FATAL: Executor crashed: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
