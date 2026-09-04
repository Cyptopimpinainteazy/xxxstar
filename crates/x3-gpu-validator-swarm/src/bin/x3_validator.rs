//! X3 GPU Validator Binary
//!
//! Main entry point for running an X3 GPU Validator.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    crypto::HashAlgorithm,
    deterministic::{DeterministicTask, TaskType},
    proof_aggregator::ProofAggregator,
    unified_proof::UnifiedProof,
    validator::Validator,
    SwarmError, SwarmResult,
};

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "run" => run_validator().await,
        "live-probe" => {
            if let Err(e) = run_live_probe().await {
                eprintln!("live-probe failed: {}", e);
                std::process::exit(1);
            }
        }
        "aggregate-probes" => {
            if let Err(e) = aggregate_probes().await {
                eprintln!("aggregate-probes failed: {}", e);
                std::process::exit(1);
            }
        }
        "benchmark" => run_benchmark().await,
        "status" => show_status().await,
        "test" => test_validator().await,
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("X3 GPU Validator");
    println!();
    println!("Usage:");
    println!("  x3-validator run        - Run the validator");
    println!("  x3-validator live-probe - Wait for node finality and emit a signed proof JSON");
    println!("  x3-validator aggregate-probes - Verify and aggregate live-probe JSON files");
    println!("  x3-validator benchmark  - Run benchmarks");
    println!("  x3-validator status    - Show validator status");
    println!("  x3-validator test      - Run tests");
    println!();
    println!("Options:");
    println!("  --config <path>   - Path to config file");
    println!("  --validator-id    - Validator ID");
    println!("  --key-path <path> - Path to 32-byte validator signing key material");
    println!("  --rpc-url <url>   - Node JSON-RPC URL for live-probe");
    println!("  --probe <path>    - live-probe JSON file for aggregate-probes");
    println!("  --task-id <id>    - Deterministic task ID for live-probe consensus tests");
    println!("  --cpu-only        - Run in CPU-only mode");
}

#[derive(Debug, Serialize, Deserialize)]
struct LiveProbeOutput {
    validator_id: String,
    validator_pubkey_hex: String,
    finalized_block: u64,
    bundle_id_hex: String,
    legs_hash_hex: String,
    signature_len: usize,
    proof: UnifiedProof,
}

async fn run_live_probe() -> SwarmResult<()> {
    let rpc_url = arg_value("--rpc-url").unwrap_or_else(|| "http://127.0.0.1:9933".to_string());
    let timeout_secs = arg_value("--timeout-secs")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);
    let task_data = arg_value("--task-data")
        .unwrap_or_else(|| "x3 live gpu-validator finalized-head proof exchange".to_string());
    let task_id = arg_value("--task-id")
        .unwrap_or_else(|| "x3-live-gpu-validator-finalized-head-proof".to_string());
    let validator_id = validator_id_from_args().unwrap_or_else(|| "validator-1".to_string());

    let finalized_block =
        wait_for_finalized_block(&rpc_url, Duration::from_secs(timeout_secs)).await?;

    let config = load_config_from_args()?;
    let validator = Arc::new(Validator::new(config, validator_id.clone()));
    validator.initialize()?;
    validator.set_chain_block_anchor(finalized_block);

    let mut task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![task_data.into_bytes()],
        HashAlgorithm::Keccak256,
    );
    task.task_id = task_id;
    let result = validator.process_task(task);
    if result.verification != x3_gpu_validator_swarm::crypto::VerificationResult::Valid {
        return Err(SwarmError::VerificationFailed(format!(
            "probe task did not verify: {:?}",
            result.verification
        )));
    }

    let proof = validator
        .get_proof_aggregator()
        .lock()
        .latest_proof()
        .ok_or(SwarmError::ProofNotFound)?;
    if proof.header.finalized_block == 0 {
        return Err(SwarmError::VerificationFailed(
            "probe proof has finalized_block=0".to_string(),
        ));
    }
    let attestation = proof
        .gpu_attestations
        .first()
        .ok_or_else(|| SwarmError::VerificationFailed("probe proof has no attestation".into()))?;
    let pubkey = validator.public_key_bytes().ok_or_else(|| {
        SwarmError::CryptoError("validator signing key unavailable; cannot emit pubkey".into())
    })?;

    let output = LiveProbeOutput {
        validator_id,
        validator_pubkey_hex: hex::encode(pubkey),
        finalized_block: proof.header.finalized_block,
        bundle_id_hex: hex::encode(proof.header.bundle_id),
        legs_hash_hex: hex::encode(proof.header.legs_hash),
        signature_len: attestation.signature.len(),
        proof,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn aggregate_probes() -> SwarmResult<()> {
    let paths = arg_values("--probe");
    if paths.len() < 2 {
        return Err(SwarmError::InvalidInput(
            "aggregate-probes requires at least two --probe files".to_string(),
        ));
    }

    let probes: Vec<LiveProbeOutput> = paths
        .iter()
        .map(|path| {
            let bytes = std::fs::read(path)?;
            serde_json::from_slice::<LiveProbeOutput>(&bytes).map_err(SwarmError::from)
        })
        .collect::<SwarmResult<_>>()?;

    let first = probes
        .first()
        .ok_or_else(|| SwarmError::InvalidInput("no probe files supplied".to_string()))?;
    if first.finalized_block == 0 {
        return Err(SwarmError::VerificationFailed(
            "first probe finalized block is zero".to_string(),
        ));
    }

    let mut aggregator = ProofAggregator::new(probes.len() as u32);
    for probe in &probes {
        let attestation = probe.proof.gpu_attestations.first().ok_or_else(|| {
            SwarmError::VerificationFailed(format!(
                "probe {} has no GPU attestation",
                probe.validator_id
            ))
        })?;
        let pubkey = parse_hex_32(&probe.validator_pubkey_hex)?;
        aggregator.register_validator_pubkey(attestation.validator_id, pubkey);
    }

    let proof_hash = first.proof.proof_hash();
    let state_commitment = first.proof.gpu_attestations[0].receipt.output_commitment;

    for probe in &probes {
        if probe.proof.header.bundle_id != first.proof.header.bundle_id {
            return Err(SwarmError::VerificationFailed(format!(
                "probe {} bundle_id mismatch",
                probe.validator_id
            )));
        }
        if probe.proof.header.legs_hash != first.proof.header.legs_hash {
            return Err(SwarmError::VerificationFailed(format!(
                "probe {} legs_hash mismatch",
                probe.validator_id
            )));
        }
        if probe.proof.header.finalized_block == 0 {
            return Err(SwarmError::VerificationFailed(format!(
                "probe {} finalized block is zero",
                probe.validator_id
            )));
        }
        aggregator.submit_proof(probe.proof.clone())?;
    }

    for probe in &probes {
        let attestation = &probe.proof.gpu_attestations[0];
        aggregator.add_attestation(
            proof_hash,
            attestation.validator_id,
            attestation.signature.clone(),
            state_commitment,
        )?;
    }

    let (state, consensus_count, _) = aggregator.get_aggregation_state(proof_hash)?;
    if !aggregator.is_finalized(proof_hash) {
        return Err(SwarmError::VerificationFailed(format!(
            "aggregate did not finalize; state={state:?} count={consensus_count}"
        )));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "state": format!("{state:?}"),
            "consensus_count": consensus_count,
            "finalized_block": first.finalized_block,
            "bundle_id": first.bundle_id_hex,
            "probe_count": probes.len()
        }))?
    );
    Ok(())
}

async fn wait_for_finalized_block(rpc_url: &str, timeout: Duration) -> SwarmResult<u64> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| SwarmError::NetworkError(e.to_string()))?;
    let deadline = Instant::now() + timeout;

    loop {
        if let Ok(block) = finalized_block_number(&client, rpc_url).await {
            if block > 0 {
                return Ok(block);
            }
        }

        if Instant::now() >= deadline {
            return Err(SwarmError::Timeout(format!(
                "timed out waiting for finalized block > 0 from {rpc_url}"
            )));
        }

        tokio::time::sleep(Duration::from_millis(750)).await;
    }
}

async fn finalized_block_number(client: &reqwest::Client, rpc_url: &str) -> SwarmResult<u64> {
    let hash = rpc(client, rpc_url, "chain_getFinalizedHead", json!([])).await?;
    let hash = hash
        .as_str()
        .ok_or_else(|| SwarmError::NetworkError("finalized head was not a string".into()))?;
    let header = rpc(client, rpc_url, "chain_getHeader", json!([hash])).await?;
    let number = header
        .get("number")
        .and_then(Value::as_str)
        .ok_or_else(|| SwarmError::NetworkError("header number missing".into()))?;
    u64::from_str_radix(number.trim_start_matches("0x"), 16)
        .map_err(|e| SwarmError::NetworkError(format!("invalid finalized number: {e}")))
}

async fn rpc(
    client: &reqwest::Client,
    rpc_url: &str,
    method: &str,
    params: Value,
) -> SwarmResult<Value> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| SwarmError::NetworkError(e.to_string()))?;
    let value: Value = response
        .json()
        .await
        .map_err(|e| SwarmError::NetworkError(e.to_string()))?;
    if let Some(error) = value.get("error") {
        return Err(SwarmError::NetworkError(format!(
            "rpc {method} returned error: {error}"
        )));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| SwarmError::NetworkError(format!("rpc {method} missing result")))
}

fn parse_hex_32(value: &str) -> SwarmResult<[u8; 32]> {
    let bytes = hex::decode(value.trim_start_matches("0x"))
        .map_err(|e| SwarmError::InvalidInput(format!("invalid hex pubkey: {e}")))?;
    if bytes.len() != 32 {
        return Err(SwarmError::InvalidInput(format!(
            "expected 32-byte pubkey, got {} bytes",
            bytes.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

async fn run_validator() {
    println!("Starting X3 GPU Validator...");

    let config = match load_config_from_args() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            return;
        }
    };
    let validator_id = validator_id_from_args().unwrap_or_else(|| "validator-1".to_string());

    // Create validator
    let validator = Arc::new(Validator::new(config.clone(), validator_id));

    // Initialize
    if let Err(e) = validator.initialize() {
        eprintln!("Failed to initialize validator: {}", e);
        return;
    }

    println!("Validator initialized successfully");
    println!("Validator ID: {}", validator.id());
    println!("State: {:?}", validator.state());

    // Run some test tasks
    let test_data = vec![
        b"hello world".to_vec(),
        b"test data for hashing".to_vec(),
        b"x3 gpu validator swarm".to_vec(),
    ];

    let task = DeterministicTask::new(TaskType::BatchHash, test_data, HashAlgorithm::Keccak256);

    println!("\nProcessing test task...");
    let result = validator.process_task(task);

    println!("Task completed:");
    println!("  Verification: {:?}", result.verification);
    println!("  Outputs: {}", result.outputs.len());
    println!("  Execution mode: {:?}", result.execution_mode);
    println!("  CPU fallback: {}", result.cpu_fallback_used);

    if result.divergence_detected {
        println!("  ⚠ Divergence detected!");
    }

    // Show metrics
    let metrics = validator.get_metrics();
    println!("\nMetrics:");
    println!("  Total tasks: {}", metrics.total_tasks);
    println!("  Successful: {}", metrics.successful_tasks);
    println!("  Failed: {}", metrics.failed_tasks);
    println!("  Divergent: {}", metrics.divergent_tasks);
    println!("  Accelerator backend: {}", metrics.accelerator_backend);
    println!("  Accelerator fallbacks: {}", metrics.accelerator_fallbacks);
    println!(
        "  Accelerator parity mismatches: {}",
        metrics.accelerator_parity_mismatches
    );

    println!("\nValidator running. Press Ctrl+C to stop.");

    // Keep running
    tokio::signal::ctrl_c().await.unwrap();

    println!("Shutting down validator...");
    validator.shutdown();
    println!("Validator stopped.");
}

async fn run_benchmark() {
    println!("Running X3 GPU Validator Benchmark...");

    let config = match load_config_from_args() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            return;
        }
    };
    let validator = Arc::new(Validator::new(
        config.clone(),
        validator_id_from_args().unwrap_or_else(|| "benchmark-validator".to_string()),
    ));

    validator.initialize().unwrap();

    // Benchmark different batch sizes
    let batch_sizes = vec![1, 10, 100, 1000, 10000];

    println!("\nBenchmarking batch hash operations:");
    println!(
        "{:<10} {:<15} {:<15} {:<15}",
        "Batch Size", "Total Time (ms)", "Avg Time (us)", "Throughput/s"
    );
    println!("{}", "-".repeat(55));

    for batch_size in batch_sizes {
        let inputs: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| format!("test data {}", i).into_bytes())
            .collect();

        let task = DeterministicTask::new(TaskType::BatchHash, inputs, HashAlgorithm::Keccak256);

        let start = std::time::Instant::now();
        let _result = validator.process_task(task);
        let elapsed = start.elapsed();

        let throughput = batch_size as f64 / elapsed.as_secs_f64();

        println!(
            "{:<10} {:<15.2} {:<15.2} {:<15.0}",
            batch_size,
            elapsed.as_millis() as f64,
            elapsed.as_micros() as f64 / batch_size as f64,
            throughput
        );
    }

    println!("\nBenchmark complete.");
}

async fn show_status() {
    println!("X3 GPU Validator Status");
    println!("========================");

    let config = match load_config_from_args() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            return;
        }
    };
    let validator = Arc::new(Validator::new(
        config,
        validator_id_from_args().unwrap_or_else(|| "status-validator".to_string()),
    ));

    println!("Validator ID: {}", validator.id());
    println!("State: {:?}", validator.state());
    println!("Uptime: {:?}", validator.uptime());
    println!("Health: {:?}", validator.health_status());

    if let Some(qs) = validator.get_quarantine_status() {
        println!("Quarantined: {}", qs.is_quarantined);
        println!("  Reason: {:?}", qs.reason);
    }

    let metrics = validator.get_metrics();
    println!("\nMetrics:");
    println!("  Total tasks: {}", metrics.total_tasks);
    println!("  Successful: {}", metrics.successful_tasks);
    println!("  Failed: {}", metrics.failed_tasks);
    println!("  Divergent: {}", metrics.divergent_tasks);
    println!("  CPU fallbacks: {}", metrics.cpu_fallbacks);
    println!("  Accelerator backend: {}", metrics.accelerator_backend);
    println!("  Accelerator fallbacks: {}", metrics.accelerator_fallbacks);
    println!(
        "  Accelerator parity mismatches: {}",
        metrics.accelerator_parity_mismatches
    );
}

async fn test_validator() {
    println!("Running X3 GPU Validator Tests...");

    let config = match load_config_from_args() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            return;
        }
    };
    let validator = Arc::new(Validator::new(
        config,
        validator_id_from_args().unwrap_or_else(|| "test-validator".to_string()),
    ));

    validator.initialize().unwrap();

    // Test 1: Basic hash
    println!("\nTest 1: Basic hash");
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"hello world".to_vec()],
        HashAlgorithm::Keccak256,
    );
    let result = validator.process_task(task);
    assert!(result.verification == x3_gpu_validator_swarm::crypto::VerificationResult::Valid);
    println!("  ✓ Passed");

    // Test 2: Multiple hashes
    println!("Test 2: Multiple hashes");
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"hello".to_vec(), b"world".to_vec(), b"test".to_vec()],
        HashAlgorithm::Keccak256,
    );
    let result = validator.process_task(task);
    assert!(result.outputs.len() == 3);
    println!("  ✓ Passed");

    // Test 3: Different algorithms
    println!("Test 3: SHA256");
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![b"test data".to_vec()],
        HashAlgorithm::Sha256,
    );
    let result = validator.process_task(task);
    assert!(result.verification == x3_gpu_validator_swarm::crypto::VerificationResult::Valid);
    println!("  ✓ Passed");

    println!("\nAll tests passed!");
}

fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(2);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }
    }
    None
}

fn arg_values(flag: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut args = std::env::args().skip(2);
    while let Some(arg) = args.next() {
        if arg == flag {
            if let Some(value) = args.next() {
                values.push(value);
            }
        }
    }
    values
}

fn load_config_from_args() -> Result<SwarmConfig, x3_gpu_validator_swarm::SwarmError> {
    let mut config = if let Some(path) = arg_value("--config") {
        SwarmConfig::from_file(std::path::Path::new(&path))
    } else {
        Ok(SwarmConfig::default())
    }?;

    if let Some(path) = arg_value("--key-path") {
        config.identity.keypair_path = PathBuf::from(path);
    }

    Ok(config)
}

fn validator_id_from_args() -> Option<String> {
    arg_value("--validator-id")
}
