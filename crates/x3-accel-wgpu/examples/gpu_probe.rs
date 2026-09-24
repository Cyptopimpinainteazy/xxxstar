//! Fresh-machine GPU bring-up probe for the wgpu accelerator backend.
//!
//! `GPU_VALIDATOR_HONEST_AUDIT.md` recorded that no accelerator claim could be
//! made because no GPU existed on the audit host: the wgpu test passed only
//! through its *fails-closed* branch. This probe is the missing evidence tool.
//! It reports the adapter that actually answered the request, runs the real
//! WGSL SHA256 kernel against CPU `sha2`, and refuses to print a speedup unless
//! every accelerated batch matched the CPU baseline byte for byte.
//!
//! ```
//! cargo run -p x3-accel-wgpu --example gpu_probe --release
//! ```
//!
//! Exit codes: `0` verified, `2` no adapter (fails closed, nothing claimed),
//! `3` GPU output diverged from CPU.

use std::process::ExitCode;
use std::time::Instant;

use sha2::{Digest, Sha256};
use x3_accel_wgpu::WgpuBackend;

/// Message lengths the parity check covers. These straddle SHA256's padding
/// boundaries (55/56 and 63/64 bytes) because that is where a kernel that
/// mishandles padding disagrees with the CPU baseline.
const PARITY_LENGTHS: [usize; 13] = [0, 1, 3, 31, 55, 56, 63, 64, 65, 120, 512, 1024, 4096];

/// Messages per throughput batch. Large enough that kernel launch overhead is
/// not the dominant term, small enough to run on an 8 GB card without concern.
const THROUGHPUT_BATCH: usize = 8192;

/// Message size for the throughput batch.
const THROUGHPUT_MESSAGE_BYTES: usize = 32;

/// Soak iterations. Sustained-correctness evidence, not just a single batch:
/// a kernel that works once and drifts under repeated dispatch is the failure
/// mode a one-shot test cannot see.
const SOAK_ITERATIONS: usize = 20;

fn main() -> ExitCode {
    let backend = match WgpuBackend::initialize() {
        Ok(backend) => backend,
        Err(error) => {
            // Fails closed: no adapter means no accelerator, and saying so is
            // the whole point of the audit's stop condition.
            eprintln!("ADAPTER_UNAVAILABLE: {error}");
            return ExitCode::from(2);
        }
    };

    let info = backend.adapter_info();
    println!("adapter            : {}", info.name);
    println!("backend            : {:?}", info.backend);
    println!("device_type        : {:?}", info.device_type);
    println!("driver             : {} {}", info.driver, info.driver_info);
    println!(
        "vendor / device    : {:#06x} / {:#06x}",
        info.vendor, info.device
    );
    println!();

    let parity_inputs = parity_vectors();
    let gpu_outputs = match backend.sha256_batch(&parity_inputs) {
        Ok(outputs) => outputs,
        Err(error) => {
            eprintln!("KERNEL_ERROR: {error}");
            return ExitCode::from(3);
        }
    };
    let cpu_outputs: Vec<[u8; 32]> = parity_inputs
        .iter()
        .map(|input| Sha256::digest(input).into())
        .collect();

    if gpu_outputs != cpu_outputs {
        for (index, (gpu, cpu)) in gpu_outputs.iter().zip(cpu_outputs.iter()).enumerate() {
            if gpu != cpu {
                eprintln!(
                    "PARITY_MISMATCH len={} gpu={} cpu={}",
                    parity_inputs[index].len(),
                    hex(gpu),
                    hex(cpu)
                );
            }
        }
        return ExitCode::from(3);
    }

    println!(
        "parity             : OK ({} messages, CPU match)\n",
        parity_inputs.len()
    );

    let batch = throughput_vectors();
    let input_bytes: u64 = batch.iter().map(|message| message.len() as u64).sum();

    let cpu_start = Instant::now();
    let mut cpu_sink = 0u8;
    for message in &batch {
        cpu_sink ^= Sha256::digest(message)[0];
    }
    let cpu_elapsed = cpu_start.elapsed();

    let gpu_start = Instant::now();
    let gpu_batch = match backend.sha256_batch(&batch) {
        Ok(outputs) => outputs,
        Err(error) => {
            eprintln!("KERNEL_ERROR: {error}");
            return ExitCode::from(3);
        }
    };
    let gpu_elapsed = gpu_start.elapsed();
    let gpu_sink: u8 = gpu_batch.iter().fold(0u8, |acc, digest| acc ^ digest[0]);

    if gpu_sink != cpu_sink {
        eprintln!("PARITY_MISMATCH: throughput batch diverged from CPU baseline");
        return ExitCode::from(3);
    }

    // Soak: repeated dispatch, parity re-checked every iteration.
    let soak_start = Instant::now();
    for iteration in 0..SOAK_ITERATIONS {
        let outputs = match backend.sha256_batch(&batch) {
            Ok(outputs) => outputs,
            Err(error) => {
                eprintln!("KERNEL_ERROR at soak iteration {iteration}: {error}");
                return ExitCode::from(3);
            }
        };
        if outputs != gpu_batch {
            eprintln!("PARITY_MISMATCH: soak iteration {iteration} differs from warm-up batch");
            return ExitCode::from(3);
        }
    }
    let soak_elapsed = soak_start.elapsed();

    println!(
        "batch              : {} messages x {} bytes ({} bytes total)",
        batch.len(),
        THROUGHPUT_MESSAGE_BYTES,
        input_bytes
    );
    println!(
        "cpu  sha256 (sha2) : {:>9.3} ms  {:>12.0} msg/s  {:>8.2} MB/s",
        ms(cpu_elapsed),
        per_second(batch.len(), cpu_elapsed),
        megabytes_per_second(input_bytes, cpu_elapsed)
    );
    println!(
        "gpu  sha256 (wgsl) : {:>9.3} ms  {:>12.0} msg/s  {:>8.2} MB/s",
        ms(gpu_elapsed),
        per_second(batch.len(), gpu_elapsed),
        megabytes_per_second(input_bytes, gpu_elapsed)
    );
    println!(
        "speedup            : {:.2}x (includes host<->device transfer + readback)",
        cpu_elapsed.as_secs_f64() / gpu_elapsed.as_secs_f64()
    );
    println!(
        "soak               : {} x {} messages verified, {:>8.3} s total, {:>12.0} msg/s",
        SOAK_ITERATIONS,
        batch.len(),
        soak_elapsed.as_secs_f64(),
        per_second(batch.len() * SOAK_ITERATIONS, soak_elapsed)
    );

    // Keccak-256: the Ethereum hash. Exercised here because the backend used to
    // fail closed on it, so the accelerator could not serve EVM-facing hashing
    // at all — the parity check is the evidence that it can now.
    let keccak_parity_inputs = parity_vectors();
    let keccak_gpu = match backend.keccak256_batch(&keccak_parity_inputs) {
        Ok(outputs) => outputs,
        Err(error) => {
            eprintln!("KERNEL_ERROR (keccak256): {error}");
            return ExitCode::from(3);
        }
    };
    let keccak_cpu = keccak_parity_inputs
        .iter()
        .map(|input| keccak_hash::keccak(input).0)
        .collect::<Vec<[u8; 32]>>();
    if keccak_gpu != keccak_cpu {
        eprintln!("PARITY_MISMATCH (keccak256): GPU output diverged from CPU baseline");
        return ExitCode::from(3);
    }

    let keccak_cpu_start = Instant::now();
    let mut keccak_cpu_sink = 0u8;
    for message in &batch {
        keccak_cpu_sink ^= keccak_hash::keccak(message).0[0];
    }
    let keccak_cpu_elapsed = keccak_cpu_start.elapsed();

    let keccak_gpu_start = Instant::now();
    let keccak_batch_outputs = match backend.keccak256_batch(&batch) {
        Ok(outputs) => outputs,
        Err(error) => {
            eprintln!("KERNEL_ERROR (keccak256): {error}");
            return ExitCode::from(3);
        }
    };
    let keccak_gpu_elapsed = keccak_gpu_start.elapsed();
    if keccak_batch_outputs
        .iter()
        .fold(0u8, |acc, digest| acc ^ digest[0])
        != keccak_cpu_sink
    {
        eprintln!("PARITY_MISMATCH (keccak256): throughput batch diverged from CPU baseline");
        return ExitCode::from(3);
    }

    let keccak_soak_start = Instant::now();
    for iteration in 0..SOAK_ITERATIONS {
        let outputs = match backend.keccak256_batch(&batch) {
            Ok(outputs) => outputs,
            Err(error) => {
                eprintln!("KERNEL_ERROR (keccak256) at soak iteration {iteration}: {error}");
                return ExitCode::from(3);
            }
        };
        if outputs != keccak_batch_outputs {
            eprintln!("PARITY_MISMATCH (keccak256): soak iteration {iteration} differs");
            return ExitCode::from(3);
        }
    }
    let keccak_soak_elapsed = keccak_soak_start.elapsed();

    println!();
    println!(
        "keccak256          : parity OK ({} messages, CPU match)",
        keccak_parity_inputs.len()
    );
    println!(
        "cpu  keccak (rust)  : {:>9.3} ms  {:>12.0} msg/s  {:>8.2} MB/s",
        ms(keccak_cpu_elapsed),
        per_second(batch.len(), keccak_cpu_elapsed),
        megabytes_per_second(input_bytes, keccak_cpu_elapsed)
    );
    println!(
        "gpu  keccak (wgsl)  : {:>9.3} ms  {:>12.0} msg/s  {:>8.2} MB/s",
        ms(keccak_gpu_elapsed),
        per_second(batch.len(), keccak_gpu_elapsed),
        megabytes_per_second(input_bytes, keccak_gpu_elapsed)
    );
    println!(
        "speedup            : {:.2}x (includes host<->device transfer + readback)",
        keccak_cpu_elapsed.as_secs_f64() / keccak_gpu_elapsed.as_secs_f64()
    );
    println!(
        "soak               : {} x {} messages verified, {:>8.3} s total, {:>12.0} msg/s",
        SOAK_ITERATIONS,
        batch.len(),
        keccak_soak_elapsed.as_secs_f64(),
        per_second(batch.len() * SOAK_ITERATIONS, keccak_soak_elapsed)
    );
    println!();
    println!("RESULT: VERIFIED (real adapter executed the kernel, output matched CPU)");

    ExitCode::SUCCESS
}

fn parity_vectors() -> Vec<Vec<u8>> {
    PARITY_LENGTHS
        .iter()
        .map(|&length| {
            (0..length)
                .map(|index| (index as u8).wrapping_mul(31).wrapping_add(7))
                .collect()
        })
        .collect()
}

fn throughput_vectors() -> Vec<Vec<u8>> {
    (0..THROUGHPUT_BATCH)
        .map(|index| {
            (0..THROUGHPUT_MESSAGE_BYTES)
                .map(|byte| ((index.wrapping_mul(31).wrapping_add(byte)) & 0xff) as u8)
                .collect()
        })
        .collect()
}

fn ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn per_second(count: usize, duration: std::time::Duration) -> f64 {
    count as f64 / duration.as_secs_f64().max(f64::MIN_POSITIVE)
}

fn megabytes_per_second(bytes: u64, duration: std::time::Duration) -> f64 {
    bytes as f64 / duration.as_secs_f64().max(f64::MIN_POSITIVE) / (1024.0 * 1024.0)
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
