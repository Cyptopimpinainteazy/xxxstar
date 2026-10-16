/// Quantum Kernel Voyager — Tauri v2 desktop entry point.
///
/// Thin main binary that delegates to the library crate.

use tracing_subscriber::{fmt, EnvFilter};

fn main() {
    // Structured logging — controlled by VOYAGER_LOG_LEVEL env var
    let filter = EnvFilter::try_from_env("VOYAGER_LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    tracing::info!("Quantum Kernel Voyager starting");

    quantum_kernel_voyager_lib::run();
}
