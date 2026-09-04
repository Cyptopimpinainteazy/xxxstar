//! E2E Test Utilities and Infrastructure
//!
//! This module provides common utilities, test fixtures, and infrastructure
//! for end-to-end integration tests across the X3-X3-Sphere ecosystem.

pub mod assertions;
pub mod mock_services;
pub mod test_accounts;
pub mod test_contracts;
pub mod test_environment;

pub use assertions::*;
pub use mock_services::*;
pub use test_accounts::*;
pub use test_contracts::*;
pub use test_environment::*;

use std::sync::Once;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static TRACING_INIT: Once = Once::new();

/// Initialize tracing for E2E tests
pub fn init_test_logging() {
    TRACING_INIT.call_once(|| {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "e2e_tests=debug,tower_http=debug,axum=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}

/// Common test result type
pub type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Test timeout configuration
#[derive(Debug, Clone)]
pub struct TestTimeout {
    pub blockchain_startup: std::time::Duration,
    pub contract_deployment: std::time::Duration,
    pub transaction_confirmation: std::time::Duration,
    pub frontend_load: std::time::Duration,
    pub gpu_task_execution: std::time::Duration,
}

impl Default for TestTimeout {
    fn default() -> Self {
        Self {
            blockchain_startup: std::time::Duration::from_secs(30),
            contract_deployment: std::time::Duration::from_secs(60),
            transaction_confirmation: std::time::Duration::from_secs(15),
            frontend_load: std::time::Duration::from_secs(10),
            gpu_task_execution: std::time::Duration::from_secs(120),
        }
    }
}

/// Test configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub network_name: String,
    pub rpc_url: String,
    pub websocket_url: String,
    pub chain_id: u64,
    pub private_key: String,
    pub timeout: TestTimeout,
    pub parallel_tests: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            network_name: "x3-chain-testnet".to_string(),
            rpc_url: "http://localhost:9933".to_string(),
            websocket_url: "ws://localhost:9944".to_string(),
            chain_id: 9999,
            private_key: "0x1234567890123456789012345678901234567890123456789012345678901234"
                .to_string(),
            timeout: TestTimeout::default(),
            parallel_tests: false,
        }
    }
}
