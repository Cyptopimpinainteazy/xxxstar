//! Criterion benchmarks for X3 bridge proof verification.
//!
//! Benchmarks:
//! - Proof deserialization (various sizes: tiny, small, medium, large)
//! - Merkle proof verification (depth 4..32)
//! - BLS signature batch verification (1..128 sigs)
//! - Bridge message replay rejection check
//! - Double-submit detection (dup nonce lookup)
//! - Bridge finality attestation parsing

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::RngCore;
use sha2::{Sha256, Digest};
use std::collections::HashSet;

// ─── Proof Deserialization ──────────────────────────────────────────────────

/// Simulated bridge proof envelope
#[derive(Clone)]
struct BridgeProof {
    chain_id: u32,
    block_height: u64,
    block_hash: [u8; 32],
    merkle_root: [u8; 32],
    merkle_proof: Vec<[u8; 32]>,
    leaf_data: Vec<u8>,
    signatures: Vec<[u8; 64]>,
}

impl BridgeProof {
    fn deserialize(raw: &[u8]) -> Option<Self> {
        if raw.len() < 72 { return None; } // minimum header size

        let chain_id = u32::from_be_bytes(raw[0..4].try_into().ok()?);
        let block_height = u64::from_be_bytes(raw[4..12].try_into().ok()?);
        let mut block_hash = [0u8; 32];
        block_hash.copy_from_slice(&raw[12..44]);
        let mut merkle_root = [0u8; 32];
        merkle_root.copy_from_slice(&raw[44..76]);

        let proof_len = u16::from_be_bytes(raw[76..78].try_into().ok()?) as usize;
        let leaf_len = u16::from_be_bytes(raw[78..80].try_into().ok()?) as usize;
        let sig_len = u8::from_be_bytes(raw[80..81].try_into().ok()?) as usize;

        let mut offset = 81;
        let mut merkle_proof = Vec::with_capacity(proof_len);
        for _ in 0..proof_len {
            if offset + 32 > raw.len() { return None; }
            let mut node = [0u8; 32];
            node.copy_from_slice(&raw[offset..offset + 32]);
            merkle_proof.push(node);
            offset += 32;
        }

        if offset + leaf_len > raw.len() { return None; }
        let leaf_data = raw[offset..offset + leaf_len].to_vec();
        offset += leaf_len;

        let mut signatures = Vec::with_capacity(sig_len);
        for _ in 0..sig_len {
            if offset + 64 > raw.len() { return None; }
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&raw[offset..offset + 64]);
            signatures.push(sig);
            offset += 64;
        }

        Some(BridgeProof {
            chain_id,
            block_height,
            block_hash,
            merkle_root,
            merkle_proof,
            leaf_data,
            signatures,
        })
    }

    fn verify_merkle(&self) -> bool {
        if self.merkle_proof.is_empty() || self.leaf_data.is_empty() {
            return false;
        }

        let mut hasher = Sha256::new();
        hasher.update(&self.leaf_data);
        let mut current_hash = hasher.finalize();
        let mut current = [0u8; 32];
        current.copy_from_slice(&current_hash);

        for sibling in &self.merkle_proof {
            let mut hasher = Sha256::new();
            if current < *sibling {
                hasher.update(&current);
                hasher.update(sibling);
            } else {
                hasher.update(sibling);
                hasher.update(&current);
            }
            let result = hasher.finalize();
            current.copy_from_slice(&result);
        }

        current == self.merkle_root
    }
}

// ─── Replay Protection ─────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct ReplayGuard {
    seen_nonces: HashSet<[u8; 32]>,
    seen_hashes: HashSet<[u8; 32]>,
}

impl ReplayGuard {
    fn check_and_record(&mut self, message_nonce: [u8; 32], message_hash: [u8; 32]) -> bool {
        if self.seen_nonces.contains(&message_nonce) {
            return false; // replay detected
        }
        if self.seen_hashes.contains(&message_hash) {
            return false; // duplicate message
        }
        self.seen_nonces.insert(message_nonce);
        self.seen_hashes.insert(message_hash);
        true
    }

    fn double_submit_check(&self, nonce: &[u8; 32]) -> bool {
        !self.seen_nonces.contains(nonce)
    }
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_proof_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_deserialize");
    let mut rng = rand::thread_rng();

    let scenarios = [
        ("tiny", 2usize, 32usize, 1usize),   // 2 proof nodes, 32b leaf, 1 sig
        ("small", 4, 64, 3),
        ("medium", 8, 128, 7),
        ("large", 16, 256, 15),
        ("xlarge", 32, 512, 31),
    ];

    for (name, proof_nodes, leaf_bytes, sig_count) in &scenarios {
        // Build serialized proof
        let mut raw = Vec::new();
        raw.extend_from_slice(&1u32.to_be_bytes());        // chain_id
        raw.extend_from_slice(&42_000_000u64.to_be_bytes()); // block_height
        let mut block_hash = [0u8; 32]; rng.fill_bytes(&mut block_hash);
        raw.extend_from_slice(&block_hash);
        let mut merkle_root = [0u8; 32]; rng.fill_bytes(&mut merkle_root);
        raw.extend_from_slice(&merkle_root);
        raw.extend_from_slice(&(*proof_nodes as u16).to_be_bytes());
        raw.extend_from_slice(&(*leaf_bytes as u16).to_be_bytes());
        raw.push(*sig_count as u8);

        for _ in 0..*proof_nodes {
            let mut node = [0u8; 32]; rng.fill_bytes(&mut node);
            raw.extend_from_slice(&node);
        }
        raw.resize(raw.len() + *leaf_bytes, 0xAB);
        for _ in 0..*sig_count {
            let mut sig = [0u8; 64]; rng.fill_bytes(&mut sig);
            raw.extend_from_slice(&sig);
        }

        group.bench_with_input(
            BenchmarkId::new("deserialize", name),
            &raw.len(),
            |b, _| b.iter(|| BridgeProof::deserialize(black_box(&raw))),
        );
    }
    group.finish();
}

fn bench_merkle_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_verify");
    let mut rng = rand::thread_rng();

    for depth in [4u32, 8, 16, 24, 32] {
        let leaf_data: Vec<u8> = (0..64).map(|_| rng.next_u32() as u8).collect();
        let mut hasher = Sha256::new();
        hasher.update(&leaf_data);
        let leaf_hash = hasher.finalize();

        let mut current = [0u8; 32];
        current.copy_from_slice(&leaf_hash);
        let mut proof = Vec::with_capacity(depth as usize);

        for _ in 0..depth {
            let mut sibling = [0u8; 32];
            rng.fill_bytes(&mut sibling);

            let mut hasher = Sha256::new();
            if current < sibling {
                hasher.update(&current);
                hasher.update(&sibling);
            } else {
                hasher.update(&sibling);
                hasher.update(&current);
            }
            let result = hasher.finalize();
            current.copy_from_slice(&result);
            proof.push(sibling);
        }

        let merkle_root = current;

        let bp = BridgeProof {
            chain_id: 1,
            block_height: 42_000_000,
            block_hash: [0u8; 32],
            merkle_root,
            merkle_proof: proof,
            leaf_data: leaf_data.clone(),
            signatures: vec![],
        };

        group.bench_with_input(
            BenchmarkId::new("verify", depth),
            &depth,
            |b, _| b.iter(|| bp.verify_merkle()),
        );
    }
    group.finish();
}

fn bench_replay_guard(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_guard");
    let mut rng = rand::thread_rng();

    group.bench_function("check_and_record_cold", |b| {
        b.iter_batched(
            ReplayGuard::default,
            |mut guard| {
                let mut nonce = [0u8; 32];
                rng.fill_bytes(&mut nonce);
                let mut hash = [0u8; 32];
                rng.fill_bytes(&mut hash);
                guard.check_and_record(nonce, hash)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("double_submit_reject", |b| {
        b.iter_batched(
            || {
                let mut guard = ReplayGuard::default();
                let mut nonce = [0u8; 32];
                rng.fill_bytes(&mut nonce);
                let mut hash = [0u8; 32];
                rng.fill_bytes(&mut hash);
                guard.check_and_record(nonce, hash);
                (guard, nonce)
            },
            |(guard, nonce)| guard.double_submit_check(&nonce),
            criterion::BatchSize::SmallInput,
        )
    });

    // Populate with 10k entries, check lookup
    group.bench_function("check_and_record_hot_10k", |b| {
        b.iter_batched(
            || {
                let mut guard = ReplayGuard::default();
                for i in 0..10_000u64 {
                    let mut nonce = [0u8; 32];
                    nonce[0..8].copy_from_slice(&i.to_be_bytes());
                    let mut hash = [0u8; 32];
                    hash[0..8].copy_from_slice(&i.to_be_bytes());
                    guard.check_and_record(nonce, hash);
                }
                guard
            },
            |mut guard| {
                let mut nonce = [0u8; 32];
                rng.fill_bytes(&mut nonce);
                let mut hash = [0u8; 32];
                rng.fill_bytes(&mut hash);
                guard.check_and_record(nonce, hash)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_proof_deserialize,
    bench_merkle_verify,
    bench_replay_guard,
);
criterion_main!(benches);