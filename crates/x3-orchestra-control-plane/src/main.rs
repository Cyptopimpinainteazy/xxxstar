use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use x3_orchestra_control_plane::{create_router, MemoryCrmAdapter, OrchestraControlPlane};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:9961")]
    bind: String,

    #[arg(long, default_value = "./x3-orchestra-control-plane-data")]
    state_dir: PathBuf,

    #[arg(long = "voter")]
    voters: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let service = Arc::new(OrchestraControlPlane::open_persistent(&args.state_dir)?);
    let crm = Arc::new(MemoryCrmAdapter::new(args.voters));
    let router = create_router(service, crm);
    let listener = tokio::net::TcpListener::bind(&args.bind).await?;

    info!(bind = %args.bind, state_dir = %args.state_dir.display(), "starting orchestra control-plane server");
    axum::serve(listener, router).await?;
    Ok(())
}
