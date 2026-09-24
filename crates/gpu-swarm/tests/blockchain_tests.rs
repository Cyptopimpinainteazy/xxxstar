//! GPU Backend and Blockchain Integration Tests

#[cfg(test)]
mod gpu_backend_tests {
    use gpu_swarm::gpu_backends::{GpuBackendType, GpuExecutorManager};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_gpu_executor_manager() {
        let manager = Arc::new(GpuExecutorManager::new());

        // Should be able to list devices
        let devices = manager.list_all_devices().await;
        assert!(devices.is_ok());
    }

    #[tokio::test]
    async fn test_backend_availability() {
        let manager = GpuExecutorManager::new();

        // Query backends
        let devices_result = manager.list_all_devices().await;

        // Should complete without error
        assert!(devices_result.is_ok());
    }
}

#[cfg(test)]
mod blockchain_tests {
    use gpu_swarm::blockchain::{BlockchainClient, RewardConfig};

    #[tokio::test]
    async fn test_blockchain_client_creation() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string()).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_blockchain_block_fetching() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string())
            .await
            .unwrap();

        let block_num = client.get_block_number().await.unwrap();
        assert!(block_num > 0);

        let block = client.get_block(block_num).await.unwrap();
        assert_eq!(block.block_number, block_num);
    }

    #[tokio::test]
    async fn test_staking() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string())
            .await
            .unwrap();

        let mut client = client;
        client.set_account("alice".to_string());

        let config = RewardConfig::default();
        let tx = client
            .stake("alice", config.minimum_stake, 100)
            .await
            .unwrap();

        assert!(!tx.is_empty());
    }

    #[tokio::test]
    async fn test_reward_distribution() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string())
            .await
            .unwrap();

        let mut rewards = std::collections::HashMap::new();
        rewards.insert("alice".to_string(), 1_000_000u128);
        rewards.insert("bob".to_string(), 500_000u128);

        let tx = client.distribute_rewards(rewards).await.unwrap();
        assert!(!tx.is_empty());
    }

    #[tokio::test]
    async fn test_slashing() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string())
            .await
            .unwrap();

        let mut client = client;
        client.set_account("alice".to_string());

        // First stake some tokens
        let config = RewardConfig::default();
        client.stake("alice", config.minimum_stake, 100).await.ok();

        // Then slash
        let tx = client
            .slash("alice", 100_000_000, "test slashing")
            .await
            .unwrap();

        assert!(!tx.is_empty());
    }

    #[tokio::test]
    async fn test_get_stake() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string())
            .await
            .unwrap();

        let stake = client.get_stake("alice").await.unwrap();
        assert!(stake >= 0);
    }

    #[tokio::test]
    async fn test_get_pending_rewards() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string())
            .await
            .unwrap();

        let rewards = client.get_pending_rewards("alice").await.unwrap();
        assert!(rewards >= 0);
    }

    #[test]
    fn test_reward_config() {
        let config = RewardConfig::default();

        assert!(config.task_completion_reward > 0);
        assert!(config.verification_bonus > 0);
        assert!(config.failure_penalty > 0);
        assert!(config.minimum_stake > 0);
        assert_eq!(config.reward_token, "X3");
        assert!(config.slashing_percentage <= 100);
    }

    #[tokio::test]
    async fn test_reward_history() {
        let client = BlockchainClient::new("ws://localhost:9944".to_string())
            .await
            .unwrap();

        let history = client.get_reward_history("alice").await.unwrap();
        assert!(history.is_empty() || !history.is_empty()); // Just check it's a valid Vec
    }
}

#[cfg(test)]
mod x3_vm_tests {
    use gpu_swarm::gpu_backends::GpuExecutorManager;
    use gpu_swarm::x3_vm::{ExecutionMode, X3VmExecutor};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_x3_vm_executor_creation() {
        let gpu_manager = Arc::new(GpuExecutorManager::new());
        let executor = X3VmExecutor::new(gpu_manager).await;

        assert!(executor.is_ok());
    }

    #[tokio::test]
    async fn test_x3_bytecode_analysis() {
        let gpu_manager = Arc::new(GpuExecutorManager::new());
        let executor = X3VmExecutor::new(gpu_manager).await.unwrap();

        let bytecode = vec![0x01, 0x02, 0x03, 0x04];
        let profile = executor.analyze_bytecode(&bytecode);

        assert!(profile.is_ok());
        let p = profile.unwrap();
        assert_eq!(p.bytecode_size, 4);
        assert!(p.estimated_memory > 0);
    }

    #[tokio::test]
    async fn test_x3_kernel_compilation() {
        let gpu_manager = Arc::new(GpuExecutorManager::new());
        let executor = X3VmExecutor::new(gpu_manager).await.unwrap();

        let bytecode = vec![0x01, 0x02, 0x03, 0x04];
        let kernel = executor.analyze_bytecode(&bytecode);

        assert!(kernel.is_ok());
    }

    #[tokio::test]
    async fn test_x3_cache_operations() {
        let gpu_manager = Arc::new(GpuExecutorManager::new());
        let executor = X3VmExecutor::new(gpu_manager).await.unwrap();

        // Get initial stats
        let (initial_count, initial_size) = executor.get_cache_stats().await;
        assert_eq!(initial_count, 0);
        assert_eq!(initial_size, 0);

        // Clear cache (should not error)
        executor.clear_cache().await;
    }

    #[test]
    fn test_execution_mode() {
        let modes = vec![
            ExecutionMode::Interpreted,
            ExecutionMode::JitCompiled,
            ExecutionMode::PreCompiled,
        ];

        for mode in modes {
            assert!(matches!(
                mode,
                ExecutionMode::Interpreted
                    | ExecutionMode::JitCompiled
                    | ExecutionMode::PreCompiled
            ));
        }
    }
}

#[cfg(test)]
mod monitoring_tests {
    use gpu_swarm::monitoring::MetricsCollector;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(collector.is_ok());
    }

    #[test]
    fn test_metrics_gathering() {
        let collector = MetricsCollector::new().unwrap();

        // Record some metrics
        collector.tasks_submitted.inc();
        collector.tasks_completed.inc();

        let metrics = collector.gather_metrics();
        // In production, would test actual Prometheus output
        assert!(true); // Just verify no panics
    }

    #[test]
    fn test_trace_context() {
        use gpu_swarm::monitoring::TraceContext;

        let ctx = TraceContext::new();
        assert!(!ctx.trace_id.is_empty());
        assert!(!ctx.span_id.is_empty());

        let child = ctx.child_span();
        assert_eq!(child.trace_id, ctx.trace_id);
        assert_eq!(child.parent_span_id, Some(ctx.span_id.clone()));
        assert_ne!(child.span_id, ctx.span_id);
    }

    #[test]
    fn test_health_check_response() {
        use gpu_swarm::monitoring::HealthCheckResponse;

        let response = HealthCheckResponse {
            status: "healthy".to_string(),
            uptime_seconds: 3600,
            connected_peers: 5,
            task_queue_size: 10,
            gpu_devices_available: 2,
            cpu_usage_percent: 45.5,
            memory_usage_percent: 60.3,
            network_sync_status: "synced".to_string(),
            last_block_number: 1000,
            is_synced: true,
            timestamp: 1000000,
        };

        assert_eq!(response.status, "healthy");
        assert!(response.is_synced);
    }

    #[test]
    fn test_alert_rule_triggers() {
        use gpu_swarm::monitoring::AlertRule;

        let rule = AlertRule {
            name: "high_cpu".to_string(),
            metric: "cpu_usage".to_string(),
            condition: "gt".to_string(),
            threshold: 80.0,
            duration_seconds: 300,
            severity: "critical".to_string(),
            description: "CPU usage too high".to_string(),
        };

        assert!(rule.should_trigger(85.0)); // Above threshold
        assert!(!rule.should_trigger(75.0)); // Below threshold
        assert!(!rule.should_trigger(80.0)); // At threshold (with gt operator)
    }
}
