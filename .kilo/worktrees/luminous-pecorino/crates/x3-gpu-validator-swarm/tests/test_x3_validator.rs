//! Tests for X3 GPU Validator Swarm

use std::sync::Arc;

use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    crypto::{HashAlgorithm, VerificationResult},
    deterministic::{DeterministicEngine, DeterministicTask, ExecutionMode, TaskType},
    metrics::HealthStatus,
    quarantine::{AuthToken, DivergenceRecord, QuarantineManager, QuarantineReason},
    validator::Validator,
};

/// Test deterministic engine basic operation
#[test]
fn test_deterministic_engine_basic() {
    let engine = DeterministicEngine::new();

    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"hello world".to_vec()],
        HashAlgorithm::Keccak256,
    );

    let result = engine.execute(task);

    assert_eq!(result.verification, VerificationResult::Valid);
    assert_eq!(result.outputs.len(), 1);
}

/// Test deterministic engine batch processing
#[test]
fn test_deterministic_engine_batch() {
    let engine = DeterministicEngine::new();

    let inputs = vec![
        b"hello".to_vec(),
        b"world".to_vec(),
        b"test".to_vec(),
        b"data".to_vec(),
        b"batch".to_vec(),
    ];

    let task = DeterministicTask::new(TaskType::BatchHash, inputs, HashAlgorithm::Keccak256);

    let result = engine.execute(task);

    assert_eq!(result.verification, VerificationResult::Valid);
    assert_eq!(result.outputs.len(), 5);
}

/// Test different hash algorithms
#[test]
fn test_hash_algorithms() {
    let engine = DeterministicEngine::new();
    let input = b"test data".to_vec();

    // Keccak256
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![input.clone()],
        HashAlgorithm::Keccak256,
    );
    let result = engine.execute(task);
    assert_eq!(result.verification, VerificationResult::Valid);

    // SHA256
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![input.clone()],
        HashAlgorithm::Sha256,
    );
    let result = engine.execute(task);
    assert_eq!(result.verification, VerificationResult::Valid);
}

/// Test CPU fallback mode
#[test]
fn test_cpu_fallback() {
    let engine = DeterministicEngine::new();
    engine.set_mode(ExecutionMode::CpuFallback);

    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"test".to_vec()],
        HashAlgorithm::Keccak256,
    );

    let result = engine.execute(task);

    assert!(result.cpu_fallback_used);
    assert_eq!(result.verification, VerificationResult::Valid);
}

/// Test replay mode
#[test]
fn test_replay_mode() {
    let engine = DeterministicEngine::new();
    engine.set_replay_mode(true);

    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"test".to_vec()],
        HashAlgorithm::Keccak256,
    );

    // Execute multiple times - should be deterministic
    let result1 = engine.execute(task.clone());
    let result2 = engine.execute(task);

    assert_eq!(result1.outputs, result2.outputs);
}

/// Test quarantine manager
#[test]
fn test_quarantine_manager() {
    let manager = QuarantineManager::new(3, 60, true);

    assert!(!manager.is_quarantined("validator1"));

    manager.quarantine("validator1".to_string(), QuarantineReason::Divergence);

    assert!(manager.is_quarantined("validator1"));

    let status = manager.get_status("validator1").unwrap();
    assert_eq!(status.divergence_count, 1);

    // Release
    let token = AuthToken {
        token: "test-token".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
    };
    manager.register_orchestrator("orchestrator-1".to_string(), token.clone());
    assert!(manager
        .release("validator1", "orchestrator-1", &token)
        .unwrap());
}

/// Test divergence recording
#[test]
fn test_divergence_recording() {
    let manager = QuarantineManager::new(3, 60, true);

    let record = DivergenceRecord::new(
        "validator1".to_string(),
        "task1".to_string(),
        b"gpu_output".to_vec(),
        b"cpu_output".to_vec(),
    );

    manager.record_divergence(record);

    let records = manager.get_divergences("validator1");
    assert_eq!(records.len(), 1);
}

/// Test validator creation
#[test]
fn test_validator_creation() {
    let config = SwarmConfig::default();
    let validator = Validator::new(config, "test-validator".to_string());

    assert_eq!(validator.id(), "test-validator");
    assert_eq!(
        validator.state(),
        x3_gpu_validator_swarm::validator::ValidatorState::Starting
    );
}

/// Test validator initialization
#[test]
fn test_validator_initialization() {
    let config = SwarmConfig::default();
    let validator = Validator::new(config, "test-validator".to_string());

    validator.initialize().unwrap();

    assert_eq!(
        validator.state(),
        x3_gpu_validator_swarm::validator::ValidatorState::Running
    );
}

/// Test validator task processing
#[test]
fn test_validator_task_processing() {
    let config = SwarmConfig::default();
    let validator = Validator::new(config, "test-validator".to_string());
    validator.initialize().unwrap();

    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"hello world".to_vec()],
        HashAlgorithm::Keccak256,
    );

    let result = validator.process_task(task);

    assert_eq!(result.verification, VerificationResult::Valid);
}

/// Test validator metrics
#[test]
fn test_validator_metrics() {
    let config = SwarmConfig::default();
    let validator = Validator::new(config, "test-validator".to_string());
    validator.initialize().unwrap();

    // Process some tasks
    for i in 0..5 {
        let task = DeterministicTask::new(
            TaskType::BatchHash,
            vec![format!("test {}", i).into_bytes()],
            HashAlgorithm::Keccak256,
        );
        validator.process_task(task);
    }

    let metrics = validator.get_metrics();
    assert_eq!(metrics.total_tasks, 5);
    assert_eq!(metrics.successful_tasks, 5);
}

/// Test CPU mode switching
#[test]
fn test_validator_cpu_mode() {
    let config = SwarmConfig::default();
    let validator = Validator::new(config, "test-validator".to_string());
    validator.initialize().unwrap();

    // Switch to CPU mode
    validator.enable_cpu_mode();
    assert_eq!(validator.current_mode(), ExecutionMode::CpuFallback);

    // Switch back to GPU mode
    validator.enable_gpu_mode();
    assert_eq!(
        validator.current_mode(),
        ExecutionMode::GpuWithCpuVerification
    );
}

/// Test validator health
#[test]
fn test_validator_health() {
    let config = SwarmConfig::default();
    let validator = Validator::new(config, "test-validator".to_string());
    validator.initialize().unwrap();

    // Record heartbeat
    validator.record_heartbeat();

    // Check health
    let health = validator.health_status();
    assert_eq!(health, HealthStatus::Healthy);
}

/// Test orchestrator creation
#[test]
fn test_orchestrator_creation() {
    let config = SwarmConfig::default();
    let orchestrator = x3_gpu_validator_swarm::orchestrator::SwarmOrchestrator::new(config);

    assert!(!orchestrator.id().is_empty());
    assert_eq!(orchestrator.pending_task_count(), 0);
}

/// Test orchestrator task submission
#[test]
fn test_orchestrator_task_submission() {
    let config = SwarmConfig::default();
    let orchestrator = x3_gpu_validator_swarm::orchestrator::SwarmOrchestrator::new(config);

    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"test".to_vec()],
        HashAlgorithm::Keccak256,
    );

    let task_id = orchestrator.submit_task(task);
    assert!(!task_id.is_empty());
    assert_eq!(orchestrator.pending_task_count(), 1);
}

/// Test orchestrator validator registration
#[test]
fn test_orchestrator_validator_registration() {
    use std::sync::Arc;

    let config = SwarmConfig::default();
    let orchestrator = x3_gpu_validator_swarm::orchestrator::SwarmOrchestrator::new(config);

    let validator = Arc::new(Validator::new(
        SwarmConfig::default(),
        "test-validator".to_string(),
    ));
    validator.initialize().unwrap();

    orchestrator.register_validator(validator);

    assert_eq!(orchestrator.get_active_validators(), 1);
}

/// Test orchestrator state export
#[test]
fn test_orchestrator_state_export() {
    let config = SwarmConfig::default();
    let orchestrator = x3_gpu_validator_swarm::orchestrator::SwarmOrchestrator::new(config);

    let state_json = orchestrator.export_state_json().unwrap();
    assert!(!state_json.is_empty());
}

/// Integration test with multiple validators
#[test]
fn test_multi_validator_scenario() {
    use std::sync::Arc;

    let config = SwarmConfig::default();
    let orchestrator = x3_gpu_validator_swarm::orchestrator::SwarmOrchestrator::new(config);

    // Register validators
    for i in 0..3 {
        let validator = Arc::new(Validator::new(
            SwarmConfig::default(),
            format!("validator-{}", i),
        ));
        validator.initialize().unwrap();
        orchestrator.register_validator(validator);
    }

    // Submit tasks
    for i in 0..10 {
        let task = DeterministicTask::new(
            TaskType::BatchHash,
            vec![format!("test task {}", i).into_bytes()],
            HashAlgorithm::Keccak256,
        );
        orchestrator.submit_task(task);
    }

    // Process tasks
    orchestrator.process_pending_tasks();

    // Check results
    let metrics = orchestrator.get_swarm_metrics();
    assert!(metrics.total_tasks <= 10);
}

/// Test performance benchmarks
#[tokio::test]
async fn test_performance_benchmark() {
    let config = SwarmConfig::default();
    let validator = Arc::new(Validator::new(config, "bench-validator".to_string()));
    validator.initialize().unwrap();

    // Benchmark batch sizes
    let batch_sizes = vec![1, 10, 100];

    for batch_size in batch_sizes {
        let inputs: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| format!("bench data {}", i).into_bytes())
            .collect();

        let task = DeterministicTask::new(TaskType::BatchHash, inputs, HashAlgorithm::Keccak256);

        let start = std::time::Instant::now();
        validator.process_task(task);
        let elapsed = start.elapsed();

        // Just verify it completes
        assert!(elapsed.as_millis() < 60000); // Should complete within 1 minute
    }
}

/// Test telemetry recording
#[test]
fn test_telemetry_recording() {
    use std::collections::HashMap;
    use x3_gpu_validator_swarm::telemetry::{TelemetryConfig, TelemetrySink};

    let config = TelemetryConfig::default();
    let sink = TelemetrySink::new(config, "test-validator".to_string());

    let mut data = HashMap::new();
    data.insert("key".to_string(), serde_json::json!("value"));

    sink.record("test_event".to_string(), "test-validator".to_string(), data);

    let events = sink.get_pending_events();
    assert_eq!(events.len(), 1);
}
