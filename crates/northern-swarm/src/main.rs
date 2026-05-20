use northern_swarm::{ChainWatcher, Config, NorthernSwarmError};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), NorthernSwarmError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("northern_swarm=info".parse().unwrap()),
        )
        .init();

    let cfg = Config::from_env();
    info!(
        chain_rpc  = %cfg.chain_rpc_url,
        ipfs       = %cfg.ipfs_gateway,
        parallelism = cfg.parallelism,
        "Northern Swarm executor starting",
    );

    let watcher = ChainWatcher::new(cfg);
    watcher.run().await
}
