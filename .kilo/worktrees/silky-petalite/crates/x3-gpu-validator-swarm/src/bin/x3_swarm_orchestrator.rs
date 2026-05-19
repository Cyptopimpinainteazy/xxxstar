//! X3 Swarm Orchestrator Binary
//!
//! Main entry point for running the X3 Swarm Orchestrator.

use std::sync::Arc;
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    crypto::HashAlgorithm,
    deterministic::{DeterministicTask, TaskType},
    orchestrator::SwarmOrchestrator,
    validator::Validator,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "run" => run_orchestrator().await,
        "status" => show_status().await,
        "add-validator" => add_validator().await,
        "benchmark" => run_benchmark().await,
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("X3 Swarm Orchestrator");
    println!();
    println!("Usage:");
    println!("  x3-swarm-orchestrator run            - Run the orchestrator");
    println!("  x3-swarm-orchestrator status         - Show swarm status");
    println!("  x3-swarm-orchestrator add-validator - Add a validator");
    println!("  x3-swarm-orchestrator benchmark     - Run swarm benchmark");
}

async fn run_orchestrator() {
    println!("Starting X3 Swarm Orchestrator...");

    let config = SwarmConfig::default();
    let orchestrator = SwarmOrchestrator::new(config);

    println!("Orchestrator ID: {}", orchestrator.id());
    println!(
        "Assignment strategy: {:?}",
        orchestrator.get_all_validator_metrics().len()
    );

    // Add some validators
    for i in 1..=4 {
        let validator = Arc::new(Validator::new(
            SwarmConfig::default(),
            format!("validator-{}", i),
        ));
        validator.initialize().unwrap();
        orchestrator.register_validator(validator);
    }

    println!(
        "Registered {} validators",
        orchestrator.get_active_validators()
    );

    // Submit test tasks
    println!("\nSubmitting test tasks...");
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"test data".to_vec()],
        HashAlgorithm::Keccak256,
    );

    for _ in 0..10 {
        orchestrator.submit_task(task.clone());
    }

    println!("Submitted {} tasks", orchestrator.pending_task_count());

    // Process tasks
    println!("\nProcessing tasks...");
    let processed = orchestrator.process_pending_tasks();
    println!("Processed {} tasks", processed);
    println!("Completed: {}", orchestrator.completed_task_count());

    // Show state
    let state_json = orchestrator.export_state_json().unwrap();
    println!("\nSwarm State:\n{}", state_json);

    println!("\nOrchestrator running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await.unwrap();
    println!("Orchestrator stopped.");
}

async fn show_status() {
    println!("X3 Swarm Orchestrator Status");
    println!("=============================");

    let config = SwarmConfig::default();
    let orchestrator = SwarmOrchestrator::new(config);

    println!("Orchestrator ID: {}", orchestrator.id());
    println!("Uptime: {:?}", orchestrator.uptime());
    println!("\nTasks:");
    println!("  Pending: {}", orchestrator.pending_task_count());
    println!("  Completed: {}", orchestrator.completed_task_count());

    let metrics = orchestrator.get_swarm_metrics();
    println!("\nMetrics:");
    println!("  Total validators: {}", metrics.total_validators);
    println!("  Active validators: {}", metrics.active_validators);
    println!("  Total tasks: {}", metrics.total_tasks);
    println!("  Throughput: {:.2} tasks/s", metrics.tasks_per_second);
}

async fn add_validator() {
    println!("Adding validator to swarm...");

    let config = SwarmConfig::default();
    let orchestrator = SwarmOrchestrator::new(config);

    let validator = Arc::new(Validator::new(
        SwarmConfig::default(),
        "new-validator".to_string(),
    ));
    validator.initialize().unwrap();

    orchestrator.register_validator(validator);

    println!("Validator added successfully");
    println!("Total validators: {}", orchestrator.get_active_validators());
}

async fn run_benchmark() {
    println!("Running Swarm Benchmark...");

    let config = SwarmConfig::default();
    let orchestrator = SwarmOrchestrator::new(config);

    // Add validators
    for i in 1..=4 {
        let validator = Arc::new(Validator::new(
            SwarmConfig::default(),
            format!("bench-validator-{}", i),
        ));
        validator.initialize().unwrap();
        orchestrator.register_validator(validator);
    }

    // Benchmark
    let task_counts = vec![10, 50, 100, 500];

    println!("\nBenchmarking task distribution:");
    println!(
        "{:<15} {:<15} {:<15} {:<15}",
        "Task Count", "Pending", "Processed", "Time (ms)"
    );
    println!("{}", "-".repeat(60));

    for count in task_counts {
        // Submit tasks
        for i in 0..count {
            let task = DeterministicTask::new(
                TaskType::BatchHash,
                vec![format!("test data {}", i).into_bytes()],
                HashAlgorithm::Keccak256,
            );
            orchestrator.submit_task(task);
        }

        let pending = orchestrator.pending_task_count();
        let start = std::time::Instant::now();
        let processed = orchestrator.process_pending_tasks();
        let elapsed = start.elapsed();

        println!(
            "{:<15} {:<15} {:<15} {:<15.2}",
            count,
            pending,
            processed,
            elapsed.as_millis() as f64
        );
    }

    println!("\nBenchmark complete.");

    // Show final metrics
    let metrics = orchestrator.get_swarm_metrics();
    println!("\nFinal Metrics:");
    println!("  Total tasks: {}", metrics.total_tasks);
    println!("  Successful: {}", metrics.successful_tasks);
    println!("  Avg latency: {:.2} ms", metrics.avg_task_latency_ms);
    println!("  Throughput: {:.2} tasks/s", metrics.tasks_per_second);
}
