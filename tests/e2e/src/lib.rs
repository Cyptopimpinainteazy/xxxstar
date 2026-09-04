//! End-to-End Integration Tests for X3-X3-Sphere
//!
//! This module provides comprehensive E2E testing for the entire X3-X3-Sphere ecosystem,
//! with focus on deterministic triple-run validation for consensus, GPU coordination, and settlement.
//!
//! Invariants: CHAIN-CONSENSUS-001, GPU-COORD-001, SETTLEMENT-001
//! See: docs/adr/0002-e2e-determinism-triple-run.md

#[cfg(feature = "full_e2e")]
#[path = "../utils/mod.rs"]
pub mod utils;

#[cfg(all(test, feature = "full_e2e"))]
pub mod deterministic_integration_tests;
#[cfg(feature = "full_e2e")]
pub mod wait_for_rpc;

// Re-export utility modules for easier access in tests
/// Internal mainnet happy path E2E tests
/// Tests critical flows: asset transfer, swap, refund, halt/restart, replay protection
#[path = "internal_mainnet_happy_path.rs"]
pub mod internal_mainnet_happy_path;

#[cfg(feature = "full_e2e")]
pub use utils::assertions::*;
#[cfg(feature = "full_e2e")]
pub use utils::mock_services::*;
#[cfg(feature = "full_e2e")]
pub use utils::test_accounts::*;
#[cfg(feature = "full_e2e")]
pub use utils::test_contracts::*;
#[cfg(feature = "full_e2e")]
pub use utils::test_environment::*;

#[cfg(feature = "full_e2e")]
use tokio::runtime::Runtime;

/// Test result type for E2E tests
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Global test runtime for async operations
#[cfg(feature = "full_e2e")]
static TEST_RUNTIME: once_cell::sync::OnceCell<Runtime> = once_cell::sync::OnceCell::new();

/// Initialize the global test runtime
#[cfg(feature = "full_e2e")]
pub fn init_test_runtime() -> &'static Runtime {
    TEST_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Failed to create test runtime")
    })
}

#[cfg(all(test, feature = "full_e2e"))]
mod tests {
    use super::*;
    use tracing::info;

    /// Basic test to verify E2E test infrastructure is working
    #[tokio::test]
    async fn test_e2e_infrastructure_setup() -> TestResult {
        info!("Starting E2E infrastructure test...");

        // Initialize test runtime
        let runtime = init_test_runtime();

        // Create a basic test environment
        let test_env = TestEnvironment::new().await?;
        info!("Test environment created successfully");

        // Verify test environment is functional
        assert!(test_env.is_running());
        info!("Test environment is running");

        // Test basic account creation
        let alice = test_env.create_test_account("alice")?;
        assert!(alice.is_valid());
        info!("Test account 'alice' created successfully");

        // Test mock services
        let mock_services = MockServices::new().await?;
        assert!(mock_services.is_running());
        info!("Mock services initialized successfully");

        // Cleanup
        test_env.cleanup().await?;
        info!("Test environment cleaned up");

        info!("E2E infrastructure test completed successfully!");
        Ok(())
    }

    /// Test blockchain node startup simulation
    #[tokio::test]
    async fn test_blockchain_node_startup() -> TestResult {
        info!("Testing blockchain node startup simulation...");

        let test_env = TestEnvironment::new().await?;

        // Simulate node startup process
        let startup_result = test_env.simulate_node_startup().await?;
        assert!(startup_result.success);
        assert!(startup_result.rpc_endpoint.is_some());

        info!("Blockchain node startup test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test smart contract deployment simulation
    #[tokio::test]
    async fn test_contract_deployment() -> TestResult {
        info!("Testing smart contract deployment simulation...");

        let test_env = TestEnvironment::new().await?;
        let contracts = TestContracts::new(&test_env).await?;

        // Deploy a simple test contract
        let deployment_result = contracts.deploy_test_contract().await?;
        assert!(deployment_result.success);
        assert!(deployment_result.contract_address.is_some());

        info!("Smart contract deployment test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test transaction creation and submission
    #[tokio::test]
    async fn test_transaction_flow() -> TestResult {
        info!("Testing transaction creation and submission...");

        let test_env = TestEnvironment::new().await?;
        let accounts = TestAccounts::new(&test_env).await?;

        // Create test accounts
        let alice = accounts.create_lender("alice")?;
        let bob = accounts.create_borrower("bob")?;

        // Simulate transaction
        let tx_result = test_env.simulate_transaction(&alice, &bob, 1000).await?;
        assert!(tx_result.success);
        assert_eq!(tx_result.amount, 1000);

        info!("Transaction flow test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test DNS server integration
    #[tokio::test]
    async fn test_dns_server_integration() -> TestResult {
        info!("Testing DNS server integration...");

        let test_env = TestEnvironment::new().await?;
        let dns_server = test_env.start_mock_dns_server().await?;

        // Test DNS resolution
        let resolution_result = dns_server.resolve_domain("test.x3").await?;
        assert!(resolution_result.success);

        info!("DNS server integration test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test GPU swarm integration
    #[tokio::test]
    async fn test_gpu_swarm_integration() -> TestResult {
        info!("Testing GPU swarm integration...");

        let test_env = TestEnvironment::new().await?;
        let gpu_swarm = test_env.start_mock_gpu_swarm().await?;

        // Test GPU task submission
        let task_result = gpu_swarm.submit_task("test_task", 100).await?;
        assert!(task_result.success);
        assert!(task_result.task_id.is_some());

        info!("GPU swarm integration test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test external chain integration
    #[tokio::test]
    async fn test_external_chain_integration() -> TestResult {
        info!("Testing external chain integration...");

        let test_env = TestEnvironment::new().await?;
        let external_chains = test_env.start_mock_external_chains().await?;

        // Test Avalanche chain connection
        let avalanche_result = external_chains.test_avalanche_connection().await?;
        assert!(avalanche_result.success);

        // Test BSC chain connection
        let bsc_result = external_chains.test_bsc_connection().await?;
        assert!(bsc_result.success);

        info!("External chain integration test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test cross-chain position manager
    #[tokio::test]
    async fn test_cross_chain_position_manager() -> TestResult {
        info!("Testing cross-chain position manager...");

        let test_env = TestEnvironment::new().await?;
        let accounts = TestAccounts::new(&test_env).await?;

        // Create test user
        let trader = accounts.create_trader("trader")?;

        // Simulate cross-chain position
        let position_result = test_env.simulate_cross_chain_position(&trader).await?;
        assert!(position_result.success);
        assert!(position_result.position_id.is_some());

        info!("Cross-chain position manager test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test X3 language execution
    #[tokio::test]
    async fn test_x3_language_execution() -> TestResult {
        info!("Testing X3 language execution...");

        let test_env = TestEnvironment::new().await?;

        // Execute simple X3 script
        let x3_code = r#"
            function main() {
                let result = 42;
                return result;
            }
        "#;

        let execution_result = test_env.execute_x3_script(x3_code).await?;
        assert!(execution_result.success);
        assert_eq!(execution_result.return_value, "42");

        info!("X3 language execution test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test protocol integration workflows
    #[tokio::test]
    async fn test_lending_protocol_workflow() -> TestResult {
        info!("Testing lending protocol workflow...");

        let test_env = TestEnvironment::new().await?;
        let accounts = TestAccounts::new(&test_env).await?;
        let contracts = TestContracts::new(&test_env).await?;

        // Setup lending protocol
        let lender = accounts.create_lender("lender")?;
        let borrower = accounts.create_borrower("borrower")?;
        let lending_pool = contracts.deploy_lending_pool().await?;

        // Simulate lending workflow
        let workflow_result = test_env
            .simulate_lending_workflow(&lender, &borrower, &lending_pool)
            .await?;
        assert!(workflow_result.success);
        assert!(workflow_result.transaction_hash.is_some());

        info!("Lending protocol workflow test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test AI swarm protocol workflow
    #[tokio::test]
    async fn test_ai_swarm_protocol_workflow() -> TestResult {
        info!("Testing AI swarm protocol workflow...");

        let test_env = TestEnvironment::new().await?;
        let accounts = TestAccounts::new(&test_env).await?;
        let gpu_swarm = test_env.start_mock_gpu_swarm().await?;

        // Create AI agent
        let agent = accounts.create_ai_agent("ai_agent")?;

        // Submit AI task to swarm
        let task_submission = gpu_swarm
            .submit_ai_task(&agent, "model_training", 1000)
            .await?;
        assert!(task_submission.success);

        // Wait for task completion
        let task_result = gpu_swarm
            .wait_for_completion(task_submission.task_id.unwrap(), 30)
            .await?;
        assert!(task_result.success);

        info!("AI swarm protocol workflow test completed!");
        test_env.cleanup().await?;
        Ok(())
    }

    /// Test evolution protocol workflow
    #[tokio::test]
    async fn test_evolution_protocol_workflow() -> TestResult {
        info!("Testing evolution protocol workflow...");

        let test_env = TestEnvironment::new().await?;
        let accounts = TestAccounts::new(&test_env).await?;

        // Create evolution researcher
        let researcher = accounts.create_researcher("researcher")?;

        // Start evolution experiment
        let experiment_result = test_env.start_evolution_experiment(&researcher).await?;
        assert!(experiment_result.success);
        assert!(experiment_result.experiment_id.is_some());

        // Run evolution steps
        for i in 0..10 {
            let step_result = test_env.run_evolution_step(&researcher, i).await?;
            assert!(step_result.success);
        }

        // Finalize experiment
        let finalization_result = test_env.finalize_evolution_experiment(&researcher).await?;
        assert!(finalization_result.success);

        info!("Evolution protocol workflow test completed!");
        test_env.cleanup().await?;
        Ok(())
    }
}
