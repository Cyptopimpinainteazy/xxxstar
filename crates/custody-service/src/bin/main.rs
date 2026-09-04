/// Custody Service binary
/// Runs the standalone custody microservice
use custody_service::CustodyServiceImpl;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("custody_service=info".parse()?),
        )
        .init();

    info!("Starting Custody Service");

    // Create service
    let service = CustodyServiceImpl::new().await?;
    info!("Service initialized");

    // Initialize sample vaults for demonstration
    service.init_vault(
        "settlement-float".to_string(),
        "USDC".to_string(),
        1_000_000,
        custody_service::types::VaultStatus::Active,
    )?;

    service.init_vault(
        "gas-reserve".to_string(),
        "ETH".to_string(),
        100,
        custody_service::types::VaultStatus::Active,
    )?;

    info!("Vaults initialized");
    info!("Custody Service running on 0.0.0.0:50051");

    // In production, this would start a gRPC server
    // For now, just keep the service alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down");

    Ok(())
}
