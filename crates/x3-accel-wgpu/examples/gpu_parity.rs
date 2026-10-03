//! Physical-GPU parity and throughput run for the wgpu Keccak-256 / SHA-256 kernels.
//!
//! ```text
//! cargo run --release -p x3-accel-wgpu --example gpu_parity -- [--seed N] [--random N] [--out FILE]
//! ```
//!
//! For every hardware adapter (software rasterizers excluded) it checks every
//! GPU digest byte-for-byte against the CPU reference (`keccak-hash`, `sha2`):
//! boundary lengths around each block size, seeded random vectors, repeated
//! values, and a large transaction-sized batch; reruns the same batch to prove
//! determinism; measures GPU vs single-thread CPU throughput per batch size;
//! and splits one batch across all adapters concurrently, merging in input
//! order. Exits 1 on any mismatch or if no hardware adapter exists, so it can
//! gate. Results are labelled PHYSICAL: they come from the real cards.

use std::time::Instant;

use sha2::{Digest, Sha256};
use x3_accel_wgpu::WgpuBackend;

type Digests = Vec<[u8; 32]>;

#[derive(Clone, Copy)]
enum Algo {
    Keccak,
    Sha256,
}

impl Algo {
    fn name(self) -> &'static str {
        match self {
            Algo::Keccak => "keccak256",
            Algo::Sha256 => "sha256",
        }
    }

    fn cpu(self, inputs: &[Vec<u8>]) -> Digests {
        inputs
            .iter()
            .map(|input| match self {
                Algo::Keccak => keccak_hash::keccak(input).0,
                Algo::Sha256 => Sha256::digest(input).into(),
            })
            .collect()
    }

    fn gpu(self, backend: &WgpuBackend, inputs: &[Vec<u8>]) -> Result<Digests, String> {
        match self {
            Algo::Keccak => backend.keccak256_batch(inputs),
            Algo::Sha256 => backend.sha256_batch(inputs),
        }
        .map_err(|err| err.to_string())
    }
}

/// xorshift64*: deterministic, dependency-free, good enough for test vectors.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[values.len() / 2]
}

/// Compare and describe the first mismatch, if any.
fn compare(name: &str, inputs: &[Vec<u8>], gpu: &Digests, cpu: &Digests) -> Option<String> {
    if gpu.len() != cpu.len() {
        return Some(format!(
            "{name}: GPU returned {} digests for {} inputs",
            gpu.len(),
            cpu.len()
        ));
    }
    let bad: Vec<usize> = (0..cpu.len()).filter(|&i| gpu[i] != cpu[i]).collect();
    bad.first().map(|&i| {
        format!(
            "{name}: {} of {} mismatched; first at index {i} (len {}): gpu {} cpu {}",
            bad.len(),
            cpu.len(),
            inputs[i].len(),
            hex(&gpu[i]),
            hex(&cpu[i])
        )
    })
}

fn vector_sets(seed: u64, random: usize) -> Vec<(&'static str, Vec<Vec<u8>>)> {
    let mut rng = Rng(seed | 1);
    // Every length where a padding or block-absorb bug would show: around
    // SHA-256's 64-byte block (55/56 is where the length no longer fits) and
    // Keccak-256's 136-byte rate, and multiples of both.
    let boundary: Vec<Vec<u8>> = [
        0usize, 1, 31, 32, 33, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129, 135, 136, 137, 191,
        192, 255, 256, 271, 272, 273, 407, 408, 409, 1023, 1024, 1025, 4096,
    ]
    .iter()
    .map(|&len| rng.bytes(len))
    .collect();
    let random_vectors: Vec<Vec<u8>> = (0..random)
        .map(|_| {
            let len = (rng.next() % 2049) as usize;
            rng.bytes(len)
        })
        .collect();
    let repeated = vec![rng.bytes(77); 512];
    let patterned: Vec<Vec<u8>> = (0..=255u8).map(|b| vec![b; 200]).collect();
    // 32-byte payloads are the transaction-hash / signing-preimage shape.
    let tx_sized: Vec<Vec<u8>> = (0..50_000).map(|_| rng.bytes(32)).collect();
    vec![
        ("boundary_lengths", boundary),
        ("random_0_2048", random_vectors),
        ("repeated_value", repeated),
        ("byte_patterns", patterned),
        ("tx_sized_32b_x50000", tx_sized),
        ("empty_batch", Vec::new()),
        // 70 MiB of data: more than one 64 MiB dispatch, so the batch is split
        // and the parts must come back in order.
        (
            "split_70x1mib",
            (0..70).map(|_| rng.bytes(1 << 20)).collect(),
        ),
        // Above 65535 workgroups x 64 threads = 4,194,240 messages, so the
        // kernel's 2-D index fold is exercised. Distinct 4-byte counters.
        (
            "two_d_dispatch_4300000x4b",
            (0..4_300_000u32)
                .map(|i| i.to_le_bytes().to_vec())
                .collect(),
        ),
    ]
}

fn main() {
    let mut seed = 0x5833_4750_5531u64; // "X3GPU1"
    let mut random = 4000usize;
    let mut out: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()).expect("--seed N"),
            "--random" => {
                random = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .expect("--random N")
            }
            "--out" => out = args.next(),
            other => panic!("unknown argument {other}"),
        }
    }

    let adapters = WgpuBackend::hardware_adapters();
    let mut failures: Vec<String> = Vec::new();
    if adapters.is_empty() {
        failures.push("no hardware GPU adapter found".into());
    }
    let sets = vector_sets(seed, random);
    let mut adapter_json = Vec::new();
    let mut backends = Vec::new();

    for (index, info) in adapters.iter().enumerate() {
        eprintln!(
            "== adapter {index}: {} ({:?}, driver {})",
            info.name, info.backend, info.driver_info
        );
        let backend = match WgpuBackend::initialize_adapter(index) {
            Ok(backend) => backend,
            Err(err) => {
                failures.push(format!("{}: init failed: {err}", info.name));
                continue;
            }
        };
        let mut parity_json = Vec::new();
        for algo in [Algo::Keccak, Algo::Sha256] {
            let mut vectors = 0usize;
            let mut set_names = Vec::new();
            for (set, inputs) in &sets {
                let cpu = algo.cpu(inputs);
                let label = format!("{} {} {set}", info.name, algo.name());
                match algo.gpu(&backend, inputs) {
                    Ok(gpu) => {
                        if let Some(bad) = compare(&label, inputs, &gpu, &cpu) {
                            failures.push(bad);
                        }
                    }
                    Err(err) => failures.push(format!("{label}: GPU error {err}")),
                }
                vectors += inputs.len();
                set_names.push(json_str(set));
            }
            // Determinism: the same batch three times must give identical bytes.
            let batch = &sets[1].1;
            let runs: Vec<_> = (0..3)
                .filter_map(|_| algo.gpu(&backend, batch).ok())
                .collect();
            let deterministic = runs.len() == 3 && runs.windows(2).all(|w| w[0] == w[1]);
            if !deterministic {
                failures.push(format!(
                    "{} {}: reruns differ or errored",
                    info.name,
                    algo.name()
                ));
            }
            eprintln!(
                "   {:9} parity over {vectors} vectors, deterministic={deterministic}",
                algo.name()
            );
            parity_json.push(format!(
                "{{\"algo\":{},\"vectors\":{vectors},\"sets\":[{}],\"deterministic\":{deterministic}}}",
                json_str(algo.name()),
                set_names.join(",")
            ));
        }

        // Throughput: GPU (median of 5, includes upload + readback) vs one CPU thread.
        let mut rng = Rng(seed ^ 0xbeef);
        let mut throughput_json = Vec::new();
        for algo in [Algo::Keccak, Algo::Sha256] {
            for payload in [32usize, 256] {
                for count in [1usize, 16, 256, 4096, 65_536] {
                    let inputs: Vec<Vec<u8>> = (0..count).map(|_| rng.bytes(payload)).collect();
                    let _ = algo.gpu(&backend, &inputs); // warm buffers / pipeline
                    let gpu_s = median(
                        (0..5)
                            .map(|_| {
                                let t = Instant::now();
                                algo.gpu(&backend, &inputs).expect("timed GPU batch");
                                t.elapsed().as_secs_f64()
                            })
                            .collect(),
                    );
                    let cpu_s = median(
                        (0..5)
                            .map(|_| {
                                let t = Instant::now();
                                std::hint::black_box(algo.cpu(&inputs));
                                t.elapsed().as_secs_f64()
                            })
                            .collect(),
                    );
                    // Every CPU thread, contiguous chunks: the fair baseline for a node.
                    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
                    let cpu_all_s = median(
                        (0..5)
                            .map(|_| {
                                let t = Instant::now();
                                std::thread::scope(|scope| {
                                    for part in inputs.chunks(inputs.len().div_ceil(threads)) {
                                        scope.spawn(move || std::hint::black_box(algo.cpu(part)));
                                    }
                                });
                                t.elapsed().as_secs_f64()
                            })
                            .collect(),
                    );
                    let gpu_rate = count as f64 / gpu_s;
                    let cpu_rate = count as f64 / cpu_s;
                    let cpu_all_rate = count as f64 / cpu_all_s;
                    eprintln!(
                        "   {:9} {payload:>3}B x {count:>6}: gpu {gpu_rate:>10.0}/s ({:>7.3} ms)  cpu1t {cpu_rate:>10.0}/s  cpu{threads}t {cpu_all_rate:>10.0}/s  gpu/cpu{threads}t {:.2}x",
                        algo.name(),
                        gpu_s * 1e3,
                        gpu_rate / cpu_all_rate
                    );
                    throughput_json.push(format!(
                        "{{\"algo\":{},\"payload_bytes\":{payload},\"batch\":{count},\"gpu_hashes_s\":{gpu_rate:.0},\"gpu_ms\":{:.4},\"cpu_1t_hashes_s\":{cpu_rate:.0},\"cpu_all_threads\":{threads},\"cpu_all_hashes_s\":{cpu_all_rate:.0},\"gpu_vs_cpu_all\":{:.3}}}",
                        json_str(algo.name()),
                        gpu_s * 1e3,
                        gpu_rate / cpu_all_rate
                    ));
                }
            }
        }
        adapter_json.push(format!(
            "{{\"index\":{index},\"name\":{},\"backend\":{},\"driver\":{},\"parity\":[{}],\"throughput\":[{}]}}",
            json_str(&info.name),
            json_str(&format!("{:?}", info.backend)),
            json_str(&format!("{} {}", info.driver, info.driver_info)),
            parity_json.join(","),
            throughput_json.join(",")
        ));
        backends.push(backend);
    }

    // All adapters at once: contiguous partitions, merged back in input order.
    let mut multi_json = "null".to_string();
    if backends.len() >= 2 {
        let mut rng = Rng(seed ^ 0xd0a1);
        let inputs: Vec<Vec<u8>> = (0..262_144).map(|_| rng.bytes(32)).collect();
        let cpu = Algo::Keccak.cpu(&inputs);
        let chunk = inputs.len().div_ceil(backends.len());
        let single_s = {
            let t = Instant::now();
            backends[0]
                .keccak256_batch(&inputs)
                .expect("single-GPU batch");
            t.elapsed().as_secs_f64()
        };
        let started = Instant::now();
        let parts: Vec<Result<Digests, String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = backends
                .iter()
                .zip(inputs.chunks(chunk))
                .map(|(backend, part)| scope.spawn(move || Algo::Keccak.gpu(backend, part)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("GPU thread panicked"))
                .collect()
        });
        let multi_s = started.elapsed().as_secs_f64();
        let mut merged = Vec::with_capacity(inputs.len());
        for part in parts {
            match part {
                Ok(digests) => merged.extend(digests),
                Err(err) => failures.push(format!("multi-GPU partition failed: {err}")),
            }
        }
        if let Some(bad) = compare("multi-GPU keccak256 merged", &inputs, &merged, &cpu) {
            failures.push(bad);
        }
        eprintln!(
            "== multi-GPU keccak256 32B x {}: {} adapters {:.1} ms vs adapter 0 alone {:.1} ms, merged order ok={}",
            inputs.len(),
            backends.len(),
            multi_s * 1e3,
            single_s * 1e3,
            merged == cpu
        );
        multi_json = format!(
            "{{\"algo\":\"keccak256\",\"batch\":{},\"adapters\":{},\"multi_ms\":{:.3},\"adapter0_alone_ms\":{:.3},\"merged_matches_cpu\":{}}}",
            inputs.len(),
            backends.len(),
            multi_s * 1e3,
            single_s * 1e3,
            merged == cpu
        );
    }

    let report = format!(
        "{{\"label\":\"PHYSICAL\",\"seed\":{seed},\"random_vectors\":{random},\"adapters\":[{}],\"multi_gpu\":{multi_json},\"failures\":[{}],\"pass\":{}}}\n",
        adapter_json.join(","),
        failures.iter().map(|f| json_str(f)).collect::<Vec<_>>().join(","),
        failures.is_empty()
    );
    match out {
        Some(path) => std::fs::write(&path, &report).expect("write report"),
        None => print!("{report}"),
    }
    for failure in &failures {
        eprintln!("FAIL {failure}");
    }
    eprintln!("{}", if failures.is_empty() { "PASS" } else { "FAIL" });
    std::process::exit(i32::from(!failures.is_empty()));
}
