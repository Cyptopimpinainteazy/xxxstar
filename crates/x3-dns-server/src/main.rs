//! X3 Chain DNS Server - Main Entry Point
//!
//! Authoritative DNS server for the .x3 TLD with blockchain integration
//! Provides DNS resolution for X3 Chain ecosystem services

use log::{error, info};
use std::sync::Arc;
use x3_dns_server::config::DnsConfig;
use x3_dns_server::error::DnsResult;
use x3_dns_server::server::AtlasDnsServer;

#[tokio::main]
async fn main() -> DnsResult<()> {
    // Initialize logging
    env_logger::init();

    info!("🚀 Starting X3 Chain DNS Server...");
    info!("📡 Authoritative DNS server for .x3 TLD");
    info!("🔗 Blockchain-integrated domain management");

    // Load configuration (use default for now to avoid config file issues)
    let config = DnsConfig::default();

    info!("⚙️  Configuration loaded: {:?}", config.server);

    // Initialize DNS server
    let dns_server = Arc::new(AtlasDnsServer::new(config.clone()).await?);

    info!(
        "🏗️  DNS server initialized with {} zones",
        dns_server.get_zone_count().await
    );

    // Clone Arc for spawning
    let server_clone = Arc::clone(&dns_server);

    // Start DNS server
    let dns_handle = tokio::spawn(async move {
        if let Err(e) = server_clone.start().await {
            error!("DNS server error: {}", e);
        }
    });

    // Start management API server
    let api_config = config.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = x3_dns_server::api::start_management_api(api_config).await {
            error!("API server error: {}", e);
        }
    });

    info!("✅ X3 Chain DNS Server is running!");
    info!("🌐 DNS server listening on: {}", config.server.bind_address);
    info!(
        "🔧 Management API listening on: {}",
        config.api.bind_address
    );
    info!(
        "📊 Metrics available at: http://{}/metrics",
        config.api.bind_address
    );

    // Handle shutdown gracefully
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("🛑 Received Ctrl+C, shutting down gracefully...");
        }
        result = dns_handle => {
            if let Err(e) = result {
                error!("DNS server task panicked: {}", e);
            }
        }
        result = api_handle => {
            if let Err(e) = result {
                error!("API server task panicked: {}", e);
            }
        }
    }

    // Stop DNS server
    dns_server.stop().await?;

    info!("🔚 X3 Chain DNS Server shutdown complete");
    Ok(())
}
