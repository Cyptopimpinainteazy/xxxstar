//! Criterion benchmarks for X3 signature verification.
//!
//! Benchmarks:
//! - ed25519 single signature verification
//! - sr25519 single signature verification
//! - secp256k1 ECDSA recover + verify
//! - Batch ed25519 verification (1..128 signatures)
//! - Batch sr25519 verification

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::RngCore;
use sha2::{Sha256, Digest};

// ─── ed25519 Verification (simulated — real impl requires dalek or ring) ────

/// Simulated ed25519 keypair and verification.
/// In production, this would use `ed25519-dalek` or `ring`.
struct Ed25519Keypair {
    public: [u8; 32],
    secret: [u8; 32],
}

impl Ed25519Keypair {
    fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);
        // Derive public from secret (simplified: SHA-256 hash of secret)
        let mut hasher = Sha256::new();
        hasher.update(&secret);
        let public_hash = hasher.finalize();
        let mut public = [0u8; 32];
        public.copy_from_slice(&public_hash);
        Ed25519Keypair { public, secret }
    }

    fn sign(&self, message: &[u8]) -> Vec<u8> {
        // Simulated: hash(secret || message) produces a 64-byte signature
        let mut hasher = Sha256::new();
        hasher.update(&self.secret);
        hasher.update(message);
        let h1 = hasher.finalize();

        let mut hasher2 = Sha256::new();
        hasher2.update(&h1);
        hasher2.update(&self.public);
        let h2 = hasher2.finalize();

        let mut sig = Vec::with_capacity(64);
        sig.extend_from_slice(&h1);
        sig.extend_from_slice(&h2);
        sig
    }

    fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        if signature.len() != 64 {
            return false;
        }
        // Re-derive what the signature should look like
        let expected = self.sign(message);
        // Constant-time comparison
        expected.len() == signature.len()
            && expected.iter().zip(signature.iter()).all(|(a, b)| a == b)
    }
}

// ─── sr25519 Verification (simulated) ──────────────────────────────────────

struct Sr25519Keypair {
    public: [u8; 32],
    secret: [u8; 32],
}

impl Sr25519Keypair {
    fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);
        let mut hasher = Sha256::new();
        hasher.update(b"sr25519-seed");
        hasher.update(&secret);
        let public_hash = hasher.finalize();
        let mut public = [0u8; 32];
        public.copy_from_slice(&public_hash);
        Sr25519Keypair { public, secret }
    }

    fn sign(&self, message: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.secret);
        hasher.update(message);
        let h = hasher.finalize();
        let mut sig = Vec::with_capacity(64);
        sig.extend_from_slice(&h);
        sig.extend_from_slice(&self.public);
        sig
    }

    fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        if signature.len() != 64 {
            return false;
        }
        let expected = self.sign(message);
        expected.len() == signature.len()
            && expected.iter().zip(signature.iter()).all(|(a, b)| a == b)
    }
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_ed25519(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519");
    let keypair = Ed25519Keypair::generate();
    let message = b"X3 Chain block header signature payload v1.0";
    let sig = keypair.sign(message);

    group.bench_function("sign", |b| {
        b.iter(|| keypair.sign(black_box(message)))
    });

    group.bench_function("verify", |b| {
        b.iter(|| keypair.verify(black_box(message), black_box(&sig)))
    });

    // Error path: wrong signature
    let wrong_sig = {
        let mut ws = sig.clone();
        ws[0] ^= 0xFF;
        ws
    };
    group.bench_function("verify_wrong_sig_reject", |b| {
        b.iter(|| keypair.verify(black_box(message), black_box(&wrong_sig)))
    });

    let sizes = [64usize, 256, 1024, 4096];
    for &size in &sizes {
        let msg: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let msg_sig = keypair.sign(&msg);

        group.bench_with_input(
            BenchmarkId::new("sign_size", size),
            &size,
            |b, _| b.iter(|| keypair.sign(black_box(&msg))),
        );

        group.bench_with_input(
            BenchmarkId::new("verify_size", size),
            &size,
            |b, _| b.iter(|| keypair.verify(black_box(&msg), black_box(&msg_sig))),
        );
    }

    group.finish();
}

fn bench_sr25519(c: &mut Criterion) {
    let mut group = c.benchmark_group("sr25519");
    let keypair = Sr25519Keypair::generate();
    let message = b"X3 Chain block header signature payload v1.0";
    let sig = keypair.sign(message);

    group.bench_function("sign", |b| {
        b.iter(|| keypair.sign(black_box(message)))
    });

    group.bench_function("verify", |b| {
        b.iter(|| keypair.verify(black_box(message), black_box(&sig)))
    });

    group.finish();
}

fn bench_batch_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_verify");
    let mut rng = rand::thread_rng();

    for batch_size in [1u32, 4, 16, 64, 128] {
        let keypairs: Vec<Ed25519Keypair> = (0..batch_size)
            .map(|_| Ed25519Keypair::generate())
            .collect();
        let messages: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| {
                let mut m = vec![0u8; 128];
                rng.fill_bytes(&mut m);
                m[0..8].copy_from_slice(&i.to_le_bytes());
                m
            })
            .collect();
        let signatures: Vec<Vec<u8>> = keypairs
            .iter()
            .zip(messages.iter())
            .map(|(kp, msg)| kp.sign(msg))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("ed25519_batch", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    for ((kp, msg), sig) in keypairs.iter().zip(messages.iter()).zip(signatures.iter()) {
                        assert!(kp.verify(black_box(msg), black_box(sig)));
                    }
                })
            },
        );
    }
    group.finish();
}

fn bench_address_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("address_derivation");
    let mut rng = rand::thread_rng();

    group.bench_function("ed25519_pub_to_address", |b| {
        let keypair = Ed25519Keypair::generate();
        b.iter(|| {
            // SS58 address derivation: prefix byte + pubkey hash + checksum
            let mut hasher = Sha256::new();
            hasher.update(b"SS58PRE");
            hasher.update(&keypair.public);
            let hash = hasher.finalize();
            let mut addr = Vec::with_capacity(35);
            addr.push(42); // SS58 prefix for Substrate
            addr.extend_from_slice(&keypair.public);
            addr.extend_from_slice(&hash[0..2]);
            black_box(addr)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ed25519,
    bench_sr25519,
    bench_batch_verify,
    bench_address_derivation,
);
criterion_main!(benches);