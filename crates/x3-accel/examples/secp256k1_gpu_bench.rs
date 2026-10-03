//! secp256k1 verification throughput: GPU (per adapter) vs CPU, 1 and all threads.
//!
//! cargo run --release -p x3-accel --features wgpu --example secp256k1_gpu_bench [-- --out FILE]
//!
//! Every GPU result is asserted valid (the jobs are all good signatures);
//! correctness across invalid inputs is `tests/secp256k1_gpu_parity.rs`.
use std::time::Instant;

use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use x3_accel::{AccelBackend, CpuBackend, Secp256k1VerifyJob};

fn jobs(count: usize) -> Vec<Secp256k1VerifyJob> {
    let secp = Secp256k1::new();
    (0..count as u32)
        .map(|i| {
            let sk = SecretKey::from_slice(&Sha256::digest(i.to_le_bytes())).unwrap();
            let z: [u8; 32] = Sha256::digest(i.to_be_bytes()).into();
            Secp256k1VerifyJob {
                message_hash: z,
                signature: secp
                    .sign_ecdsa(&Message::from_digest(z), &sk)
                    .serialize_compact(),
                public_key: PublicKey::from_secret_key(&secp, &sk).serialize().to_vec(),
            }
        })
        .collect()
}

fn main() {
    let mut out = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = Some(args.next().expect("--out FILE")),
            other => panic!("unknown argument {other}"),
        }
    }

    let all = jobs(65_536);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let cpu = CpuBackend::new();
    let t = Instant::now();
    cpu.verify_secp256k1_batch(&all[..8192]).unwrap();
    let cpu1 = 8192.0 / t.elapsed().as_secs_f64();
    let t = Instant::now();
    std::thread::scope(|scope| {
        for part in all.chunks(all.len().div_ceil(threads)) {
            scope.spawn(move || CpuBackend::new().verify_secp256k1_batch(part).unwrap());
        }
    });
    let cpu_all = all.len() as f64 / t.elapsed().as_secs_f64();
    println!("cpu 1 thread: {cpu1:.0} verifies/s   cpu {threads} threads: {cpu_all:.0} verifies/s");

    let mut rows = Vec::new();
    for (index, info) in x3_accel_wgpu::WgpuBackend::hardware_adapters()
        .iter()
        .enumerate()
    {
        let gpu = x3_accel::WgpuBackend::from_adapter(index).unwrap();
        let t = Instant::now();
        gpu.verify_secp256k1_batch(&all[..1]).unwrap();
        let first = t.elapsed();
        println!(
            "== {}: first call (shader compile + 1 job) {first:.2?}",
            info.name
        );
        for count in [64usize, 1024, 8192, 65_536] {
            let t = Instant::now();
            let ok = gpu.verify_secp256k1_batch(&all[..count]).unwrap();
            let secs = t.elapsed().as_secs_f64();
            assert!(ok.iter().all(|v| *v));
            let rate = count as f64 / secs;
            println!(
                "   {count:>6} jobs: {:>8.1} ms  {rate:>9.0} verifies/s  vs cpu{threads}t {:.3}x",
                secs * 1e3,
                rate / cpu_all
            );
            rows.push(format!(
                "{{\"gpu\":\"{}\",\"first_call_s\":{:.2},\"batch\":{count},\"ms\":{:.2},\"verifies_s\":{rate:.0},\"vs_cpu_all\":{:.3}}}",
                info.name,
                first.as_secs_f64(),
                secs * 1e3,
                rate / cpu_all
            ));
        }
    }
    // Both cards at once, split by measured speed.
    let multi = x3_accel::MultiDevice::calibrated_wgpu(x3_accel::DEFAULT_MIN_SPLIT).unwrap();
    println!(
        "== all {} adapters split by weight {:?}",
        multi.device_count(),
        multi
            .weights()
            .iter()
            .map(|w| w.round())
            .collect::<Vec<_>>()
    );
    for count in [8192usize, 65_536] {
        let t = Instant::now();
        let ok = multi.verify_secp256k1_batch(&all[..count]).unwrap();
        let secs = t.elapsed().as_secs_f64();
        assert!(ok.iter().all(|v| *v));
        let rate = count as f64 / secs;
        println!(
            "   {count:>6} jobs: {:>8.1} ms  {rate:>9.0} verifies/s  vs cpu{threads}t {:.3}x",
            secs * 1e3,
            rate / cpu_all
        );
        rows.push(format!(
            "{{\"gpu\":\"all-adapters-split\",\"batch\":{count},\"ms\":{:.2},\"verifies_s\":{rate:.0},\"vs_cpu_all\":{:.3}}}",
            secs * 1e3,
            rate / cpu_all
        ));
    }
    if let Some(path) = out {
        let report = format!(
            "{{\"label\":\"PHYSICAL\",\"cpu_1t_verifies_s\":{cpu1:.0},\"cpu_threads\":{threads},\"cpu_all_verifies_s\":{cpu_all:.0},\"gpu\":[{}]}}\n",
            rows.join(",")
        );
        std::fs::write(path, report).expect("write report");
    }
}
