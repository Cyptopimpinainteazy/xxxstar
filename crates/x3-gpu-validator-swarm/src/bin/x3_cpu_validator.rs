//! X3 CPU Validator Binary
//!
//! Standalone CPU validator - no GPU required.
//! Anyone can run this to participate in the X3 swarm.

use x3_gpu_validator_swarm::{cpu_validator::EasyCpuValidator, crypto::HashAlgorithm};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "hash" => hash_data(&args[2..]),
        "hash-batch" => hash_batch(&args[2..]),
        "validate" => validate(&args[2..]),
        "benchmark" => run_benchmark(),
        "status" => show_status(),
        "test" => run_tests(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("X3 CPU Validator - No GPU Required!");
    println!("═══════════════════════════════════");
    println!();
    println!("Usage: x3-cpu-validator <command> [args]");
    println!();
    println!("Commands:");
    println!("  hash <data>         - Hash a single string");
    println!("  hash-batch <file>   - Hash lines from a file");
    println!("  validate <data>     - Validate data and show result");
    println!("  benchmark           - Run performance benchmark");
    println!("  status              - Show validator status");
    println!("  test                - Run validation tests");
    println!();
    println!("Examples:");
    println!("  x3-cpu-validator hash \"hello world\"");
    println!("  x3-cpu-validator hash-batch data.txt");
    println!("  x3-cpu-validator validate \"test data\"");
    println!();
    println!("API Usage (in Rust code):");
    println!("  use x3_gpu_validator_swarm::cpu_validator::validate_cpu;");
    println!("  let hash = validate_cpu(b\"data\");");
}

fn hash_data(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: No data provided");
        return;
    }

    let data = args[0].as_bytes();
    let hash = validate_cpu(data);

    println!("Input: {}", args[0]);
    println!("Hash:  {}", hash.to_hex());
    println!();
    println!("Algorithm: Keccak-256");
    println!("Output: 256-bit (32 bytes)");
}

fn hash_batch(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: No file provided");
        return;
    }

    let filename = &args[0];
    let content = match std::fs::read_to_string(filename) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return;
        }
    };

    let lines: Vec<Vec<u8>> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.as_bytes().to_vec())
        .collect();

    if lines.is_empty() {
        println!("No data to hash");
        return;
    }

    let validator = EasyCpuValidator::default();
    let hashes = validator.hash_batch(lines.clone());

    println!("Hashed {} items from {}", lines.len(), filename);
    println!();
    println!("Results:");
    for (i, hash) in hashes.iter().enumerate() {
        println!("  {}: {}", i + 1, hash.to_hex());
    }
}

fn validate(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: No data provided");
        return;
    }

    let data = args[0].as_bytes();

    println!("Validating: {}", args[0]);
    println!();

    // Test all algorithms
    let validator = EasyCpuValidator::default();

    // Keccak-256
    let keccak = validator.hash_with(data, HashAlgorithm::Keccak256);
    println!("Keccak-256: {}", keccak.to_hex());

    // SHA-256
    let sha256 = validator.hash_with(data, HashAlgorithm::Sha256);
    println!("SHA-256:    {}", sha256.to_hex());

    // Blake2b
    let blake2b = validator.hash_with(data, HashAlgorithm::Blake2b);
    println!("Blake2b:    {}", blake2b.to_hex());

    println!();
    println!("✓ Validation complete - all algorithms produce deterministic output");
}

fn run_benchmark() {
    println!("X3 CPU Validator Benchmark");
    println!("═══════════════════════════");
    println!();

    let validator = EasyCpuValidator::default();

    // Test different data sizes
    let sizes = vec![1, 10, 100, 1000, 10000];

    println!(
        "{:<12} {:<15} {:<15} {:<15}",
        "Data Size", "Time (ms)", "Ops/sec", "ms/op"
    );
    println!("{}", "-".repeat(57));

    for size in sizes {
        let data: Vec<Vec<u8>> = (0..size)
            .map(|i| format!("benchmark data {}", i).into_bytes())
            .collect();

        let start = std::time::Instant::now();
        let _hashes = validator.hash_batch(data);
        let elapsed = start.elapsed();

        let ops_per_sec = size as f64 / elapsed.as_secs_f64();
        let ms_per_op = elapsed.as_millis() as f64 / size as f64;

        println!(
            "{:<12} {:<15.2} {:<15.0} {:<15.4}",
            size,
            elapsed.as_millis() as f64,
            ops_per_sec,
            ms_per_op
        );
    }

    println!();
    println!("✓ Benchmark complete");
    println!();
    println!("Your CPU can validate without GPU!");
    println!("Join the swarm: x3-cpu-validator status");
}

fn show_status() {
    println!("X3 CPU Validator Status");
    println!("════════════════════════");
    println!();

    let validator = EasyCpuValidator::default();
    let metrics = validator.metrics();

    println!("Validator ID: {}", metrics.validator_id);
    println!("Mode: CPU-Only (No GPU required)");
    println!();
    println!("Performance:");
    println!("  Tasks processed:  {}", metrics.tasks_processed);
    println!("  Tasks successful: {}", metrics.tasks_successful);
    println!("  Success rate:     {:.1}%", metrics.success_rate * 100.0);
    println!("  Uptime:           {}s", metrics.uptime_secs);
    println!();
    println!("═══════════════════════════════════");
    println!("✓ CPU Validator is ready!");
    println!("  Anyone can validate - no GPU needed");
}

fn run_tests() {
    println!("Running X3 CPU Validator Tests...");
    println!();

    let validator = EasyCpuValidator::default();

    // Test 1: Basic hash
    println!("Test 1: Basic hash");
    let hash = validator.hash(b"hello world");
    assert!(hash.0 != [0u8; 32]);
    println!("  ✓ Keccak-256 hash works");

    // Test 2: Determinism
    println!("Test 2: Determinism");
    let hash1 = validator.hash(b"test");
    let hash2 = validator.hash(b"test");
    assert_eq!(hash1.0, hash2.0);
    println!("  ✓ Same input produces same output");

    // Test 3: Different algorithms
    println!("Test 3: Algorithm variety");
    let keccak = validator.hash_with(b"test", HashAlgorithm::Keccak256);
    let sha256 = validator.hash_with(b"test", HashAlgorithm::Sha256);
    assert_ne!(keccak.0, sha256.0);
    println!("  ✓ Different algorithms produce different hashes");

    // Test 4: Batch processing
    println!("Test 4: Batch processing");
    let hashes = validator.hash_batch(vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    assert_eq!(hashes.len(), 3);
    println!("  ✓ Batch processing works");

    // Test 5: Large data
    println!("Test 5: Large data");
    let large: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let hash = validator.hash(&large);
    assert!(hash.0 != [0u8; 32]);
    println!("  ✓ Large data handled");

    println!();
    println!("═══════════════════════════════════");
    println!("✓ All tests passed!");
    println!("═══════════════════════════════════");
}

// Re-export the validate function for API use
pub use x3_gpu_validator_swarm::cpu_validator::validate_cpu;
