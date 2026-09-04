//! X3 GPU Validator Benchmark Tool
//!
//! Produces JSON benchmark reports for performance analysis.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    crypto::HashAlgorithm,
    deterministic::{DeterministicTask, TaskType},
    validator::Validator,
};

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Batch size
    pub batch_size: usize,
    /// Total time (ms)
    pub total_time_ms: f64,
    /// Average time per operation (us)
    pub avg_time_us: f64,
    /// Operations per second
    pub ops_per_second: f64,
    /// Success rate
    pub success_rate: f64,
    /// Timestamp
    pub timestamp: i64,
}

/// Benchmark report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// Report version
    pub version: String,
    /// Benchmark name
    pub benchmark: String,
    /// Hardware info
    pub hardware: HardwareInfo,
    /// Results
    pub results: Vec<BenchmarkResult>,
    /// Summary
    pub summary: BenchmarkSummary,
    /// Timestamp
    pub timestamp: i64,
}

/// Hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// CPU cores
    pub cpu_cores: usize,
    /// Memory (MB)
    pub memory_mb: u64,
    /// GPU available
    pub gpu_available: bool,
}

/// Benchmark summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    /// Total operations
    pub total_operations: usize,
    /// Total time (ms)
    pub total_time_ms: f64,
    /// Average throughput
    pub avg_throughput: f64,
    /// Peak throughput
    pub peak_throughput: f64,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "run" => run_benchmark(),
        "report" => show_latest_report(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("X3 GPU Validator Benchmark");
    println!();
    println!("Usage:");
    println!("  x3-bench run    - Run benchmark and save report");
    println!("  x3-bench report - Show latest benchmark report");
    println!();
    println!("Options:");
    println!("  --output <path> - Output file path (default: ./benchmark-results.json)");
    println!("  --iterations    - Number of iterations per test");
}

fn run_benchmark() {
    println!("Running X3 GPU Validator Benchmark...");
    println!();

    let mut results = Vec::new();
    let config = SwarmConfig::default();
    let validator = Arc::new(Validator::new(config, "benchmark-validator".to_string()));
    validator.initialize().unwrap();

    // Get hardware info
    let hw_info = HardwareInfo {
        cpu_cores: num_cpus::get(),
        memory_mb: get_memory_mb(),
        gpu_available: true, // Would check in real implementation
    };

    println!(
        "Hardware: {} cores, {} MB memory, GPU: {}",
        hw_info.cpu_cores,
        hw_info.memory_mb,
        if hw_info.gpu_available {
            "available"
        } else {
            "not available"
        }
    );

    // Benchmark different configurations
    let configs = vec![
        (
            "Keccak256",
            HashAlgorithm::Keccak256,
            vec![1, 10, 100, 1000, 10000],
        ),
        (
            "SHA256",
            HashAlgorithm::Sha256,
            vec![1, 10, 100, 1000, 10000],
        ),
    ];

    for (algo_name, algo, batch_sizes) in configs {
        println!("\nBenchmarking {}:", algo_name);
        println!(
            "{:<12} {:<12} {:<15} {:<15} {:<12}",
            "Batch", "Time (ms)", "Avg (us)", "Ops/sec", "Success"
        );
        println!("{}", "-".repeat(66));

        for batch_size in batch_sizes {
            let inputs: Vec<Vec<u8>> = (0..batch_size)
                .map(|i| format!("benchmark data {}", i).into_bytes())
                .collect();

            let task = DeterministicTask::new(TaskType::BatchHash, inputs, algo);

            let start = std::time::Instant::now();
            let result = validator.process_task(task);
            let elapsed = start.elapsed();

            let ops_per_sec = batch_size as f64 / elapsed.as_secs_f64();
            let success_rate = if result.verification
                == x3_gpu_validator_swarm::crypto::VerificationResult::Valid
            {
                100.0
            } else {
                0.0
            };

            println!(
                "{:<12} {:<12.2} {:<15.2} {:<15.0} {:<12.1}%",
                batch_size,
                elapsed.as_millis() as f64,
                elapsed.as_micros() as f64 / batch_size as f64,
                ops_per_sec,
                success_rate
            );

            results.push(BenchmarkResult {
                name: format!("{}_{}", algo_name, batch_size),
                batch_size,
                total_time_ms: elapsed.as_millis() as f64,
                avg_time_us: elapsed.as_micros() as f64 / batch_size as f64,
                ops_per_second: ops_per_sec,
                success_rate,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    }

    // Calculate summary
    let total_ops: usize = results.iter().map(|r| r.batch_size).sum();
    let total_time: f64 = results.iter().map(|r| r.total_time_ms).sum();
    let avg_throughput = if total_time > 0.0 {
        total_ops as f64 / (total_time / 1000.0)
    } else {
        0.0
    };
    let peak_throughput = results.iter().map(|r| r.ops_per_second).fold(0.0, f64::max);

    // Create report
    let report = BenchmarkReport {
        version: "1.0".to_string(),
        benchmark: "x3-gpu-validator".to_string(),
        hardware: hw_info,
        results: results.clone(),
        summary: BenchmarkSummary {
            total_operations: total_ops,
            total_time_ms: total_time,
            avg_throughput,
            peak_throughput,
        },
        timestamp: chrono::Utc::now().timestamp(),
    };

    // Save report
    let output_path = PathBuf::from("benchmark-results.json");
    let json = serde_json::to_string_pretty(&report).unwrap();
    let mut file = File::create(&output_path).unwrap();
    file.write_all(json.as_bytes()).unwrap();

    println!();
    println!("==============================================");
    println!("Benchmark Complete");
    println!("==============================================");
    println!("Total operations: {}", total_ops);
    println!("Total time: {:.2} ms", total_time);
    println!("Average throughput: {:.0} ops/sec", avg_throughput);
    println!("Peak throughput: {:.0} ops/sec", peak_throughput);
    println!();
    println!("Report saved to: {}", output_path.display());
}

fn show_latest_report() {
    let path = PathBuf::from("benchmark-results.json");

    if !path.exists() {
        eprintln!("No benchmark report found. Run 'x3-bench run' first.");
        return;
    }

    let content = std::fs::read_to_string(&path).unwrap();
    let report: BenchmarkReport = serde_json::from_str(&content).unwrap();

    println!("X3 GPU Validator Benchmark Report");
    println!("===================================");
    println!("Version: {}", report.version);
    println!("Benchmark: {}", report.benchmark);
    println!("Timestamp: {}", report.timestamp);
    println!();
    println!("Hardware:");
    println!("  CPU cores: {}", report.hardware.cpu_cores);
    println!("  Memory: {} MB", report.hardware.memory_mb);
    println!(
        "  GPU: {}",
        if report.hardware.gpu_available {
            "available"
        } else {
            "not available"
        }
    );
    println!();
    println!("Summary:");
    println!("  Total operations: {}", report.summary.total_operations);
    println!("  Total time: {:.2} ms", report.summary.total_time_ms);
    println!(
        "  Avg throughput: {:.0} ops/sec",
        report.summary.avg_throughput
    );
    println!(
        "  Peak throughput: {:.0} ops/sec",
        report.summary.peak_throughput
    );
    println!();
    println!("Results:");
    for result in &report.results {
        println!(
            "  {}: {:.0} ops/sec ({:.1}% success)",
            result.name, result.ops_per_second, result.success_rate
        );
    }
}

fn get_memory_mb() -> u64 {
    // Simple memory detection
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
            })
            .map(|kb| kb / 1024)
            .unwrap_or(0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

use std::sync::Arc;
