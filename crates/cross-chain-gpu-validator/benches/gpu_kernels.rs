use criterion::{criterion_group, criterion_main, Criterion};
use cross_chain_gpu_validator::kernels::{Keccak256Kernel, Secp256k1Kernel};

fn benchmark_keccak256_batch(c: &mut Criterion) {
    c.bench_function("keccak256_batch_32", |b| {
        let kernel = Keccak256Kernel::new(256, false);
        b.iter(|| {
            let strs: Vec<String> = (0..32).map(|i| format!("test_input_{i}")).collect();
            let inputs: Vec<&[u8]> = strs.iter().map(|s| s.as_bytes()).collect();
            kernel.hash_batch_cpu(&inputs)
        })
    });

    c.bench_function("keccak256_batch_256", |b| {
        let kernel = Keccak256Kernel::new(256, false);
        b.iter(|| {
            let strs: Vec<String> = (0..256).map(|i| format!("test_input_{i}")).collect();
            let inputs: Vec<&[u8]> = strs.iter().map(|s| s.as_bytes()).collect();
            kernel.hash_batch_cpu(&inputs)
        })
    });

    c.bench_function("keccak256_parity_check", |b| {
        let kernel = Keccak256Kernel::new(256, false);
        b.iter(|| {
            let strs: Vec<String> = (0..64).map(|i| format!("parity_check_{i}")).collect();
            let inputs: Vec<&[u8]> = strs.iter().map(|s| s.as_bytes()).collect();
            kernel.verify_parity(&inputs)
        })
    });
}

fn benchmark_secp256k1_verification(c: &mut Criterion) {
    c.bench_function("secp256k1_batch_verify_32", |b| {
        let kernel = Secp256k1Kernel::new(32, false);
        b.iter(|| {
            let messages = vec![b"test_message".as_slice(); 32];
            let sig_data = vec![0u8; 64];
            let pubkey_data = vec![0u8; 33];
            let signatures: Vec<&[u8]> = (0..32).map(|_| sig_data.as_slice()).collect();
            let pubkeys: Vec<&[u8]> = (0..32).map(|_| pubkey_data.as_slice()).collect();
            kernel.verify_batch_cpu(&messages, &signatures, &pubkeys)
        })
    });
}

criterion_group!(
    benches,
    benchmark_keccak256_batch,
    benchmark_secp256k1_verification
);
criterion_main!(benches);
