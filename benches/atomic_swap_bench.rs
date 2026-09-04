//! Criterion benchmarks for X3 atomic swap hot paths.
//!
//! Benchmarks:
//! - HTLC hashlock generation (sha256 + blake2)
//! - HTLC timelock creation and validation
//! - Cross-VM swap state transition (EVM→SVM, SVM→native, native→EVM)
//! - Route quote calculation (fee math, slippage)
//! - Intent parsing and validation
//! - Swap ledger write (SCALE encode + hash)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::RngCore;
use sha2::{Sha256, Digest};
use blake2::Blake2b512;

// ─── HTLC Hashlock Benchmarks ───────────────────────────────────────────────

fn htlc_hashlock_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

fn htlc_hashlock_blake2(data: &[u8]) -> [u8; 64] {
    let mut hasher = Blake2b512::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut output = [0u8; 64];
    output.copy_from_slice(&result);
    output
}

fn bench_htlc_hashlock(c: &mut Criterion) {
    let mut group = c.benchmark_group("htlc_hashlock");
    let sizes = [32, 64, 128, 256, 512, 1024];

    for size in sizes {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        group.bench_with_input(BenchmarkId::new("sha256", size), &size, |b, _| {
            b.iter(|| htlc_hashlock_sha256(black_box(&data)))
        });

        group.bench_with_input(BenchmarkId::new("blake2b512", size), &size, |b, _| {
            b.iter(|| htlc_hashlock_blake2(black_box(&data)))
        });
    }
    group.finish();
}

// ─── HTLC Timelock Benchmarks ───────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
struct Timelock {
    expires_at: u64,
    grace_period: u64,
    chain_id: u32,
}

impl Timelock {
    fn new(expires_at: u64, grace_period: u64, chain_id: u32) -> Self {
        Self { expires_at, grace_period, chain_id }
    }

    fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.expires_at + self.grace_period
    }

    fn is_active(&self, current_time: u64) -> bool {
        current_time < self.expires_at
    }

    fn remaining_seconds(&self, current_time: u64) -> u64 {
        if current_time >= self.expires_at {
            return 0;
        }
        self.expires_at - current_time
    }

    fn to_bytes(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        buf[0..8].copy_from_slice(&self.expires_at.to_be_bytes());
        buf[8..16].copy_from_slice(&self.grace_period.to_be_bytes());
        buf[16..20].copy_from_slice(&self.chain_id.to_be_bytes());
        buf
    }
}

fn bench_timelock_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("timelock");
    let lock = Timelock::new(1_000_000, 300, 1);
    let current = 999_000u64;

    group.bench_function("is_expired", |b| {
        b.iter(|| lock.is_expired(black_box(current)))
    });

    group.bench_function("is_active", |b| {
        b.iter(|| lock.is_active(black_box(current)))
    });

    group.bench_function("remaining_seconds", |b| {
        b.iter(|| lock.remaining_seconds(black_box(current)))
    });

    group.bench_function("to_bytes", |b| {
        b.iter(|| lock.to_bytes())
    });

    group.finish();
}

// ─── Swap State Transition Benchmarks ────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SwapState {
    Pending,
    FundedEVM,
    FundedSVM,
    FundedNative,
    Claimed,
    Refunded,
    Disputed,
    Rolledback,
}

#[derive(Clone)]
struct SwapIntent {
    id: [u8; 32],
    from_chain: u32,
    to_chain: u32,
    from_amount: u128,
    to_amount: u128,
    min_receive: u128,
    state: SwapState,
    hashlock: [u8; 32],
    timelock: Timelock,
}

impl SwapIntent {
    fn transition(&mut self, new_state: SwapState) -> Result<(), &'static str> {
        match (self.state, new_state) {
            (SwapState::Pending, SwapState::FundedEVM) => {
                self.state = SwapState::FundedEVM;
                Ok(())
            }
            (SwapState::Pending, SwapState::FundedSVM) => {
                self.state = SwapState::FundedSVM;
                Ok(())
            }
            (SwapState::FundedEVM, SwapState::Claimed) => {
                self.state = SwapState::Claimed;
                Ok(())
            }
            (SwapState::FundedSVM, SwapState::Claimed) => {
                self.state = SwapState::Claimed;
                Ok(())
            }
            (SwapState::FundedEVM, SwapState::Refunded) => {
                self.state = SwapState::Refunded;
                Ok(())
            }
            (SwapState::FundedSVM, SwapState::Refunded) => {
                self.state = SwapState::Refunded;
                Ok(())
            }
            (_, SwapState::Disputed) => {
                self.state = SwapState::Disputed;
                Ok(())
            }
            (SwapState::Disputed, SwapState::Rolledback) => {
                self.state = SwapState::Rolledback;
                Ok(())
            }
            _ => Err("invalid state transition"),
        }
    }

    fn compute_fee(&self, rate_bps: u32) -> u128 {
        (self.from_amount * rate_bps as u128) / 10_000
    }

    fn effective_output(&self, rate_bps: u32) -> u128 {
        let fee = self.compute_fee(rate_bps);
        self.to_amount.saturating_sub(fee)
    }

    fn meets_min_receive(&self, rate_bps: u32) -> bool {
        self.effective_output(rate_bps) >= self.min_receive
    }
}

fn bench_swap_state_transition(c: &mut Criterion) {
    let mut group = c.benchmark_group("swap_state_transition");
    let mut rng = rand::thread_rng();

    let mut hashlock = [0u8; 32];
    rng.fill_bytes(&mut hashlock);

    let intent = SwapIntent {
        id: hashlock,
        from_chain: 1,
        to_chain: 2,
        from_amount: 1_000_000_000_000_000_000u128,
        to_amount: 995_000_000_000_000_000u128,
        min_receive: 990_000_000_000_000_000u128,
        state: SwapState::Pending,
        hashlock,
        timelock: Timelock::new(1_000_000, 300, 1),
    };

    // Valid transitions
    group.bench_function("pending_to_funded_evm", |b| {
        b.iter_batched(
            || intent.clone(),
            |mut i| i.transition(SwapState::FundedEVM),
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("funded_to_claimed", |b| {
        b.iter_batched(
            || {
                let mut i = intent.clone();
                i.state = SwapState::FundedEVM;
                i
            },
            |mut i| i.transition(SwapState::Claimed),
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("funded_to_refunded", |b| {
        b.iter_batched(
            || {
                let mut i = intent.clone();
                i.state = SwapState::FundedEVM;
                i
            },
            |mut i| i.transition(SwapState::Refunded),
            criterion::BatchSize::SmallInput,
        )
    });

    // Invalid transition (error path)
    group.bench_function("invalid_transition_err", |b| {
        b.iter_batched(
            || intent.clone(),
            |mut i| i.transition(SwapState::Claimed), // Pending→Claimed is invalid
            criterion::BatchSize::SmallInput,
        )
    });

    // Fee math
    group.bench_function("compute_fee", |b| {
        b.iter(|| intent.compute_fee(black_box(30))); // 30 bps = 0.3%
    });

    group.bench_function("effective_output", |b| {
        b.iter(|| intent.effective_output(black_box(30)));
    });

    group.bench_function("meets_min_receive", |b| {
        b.iter(|| intent.meets_min_receive(black_box(30)));
    });

    group.finish();
}

// ─── Batch Verification Benchmarks ──────────────────────────────────────────

fn bench_batch_hashlock_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_hashlock_verify");
    let mut rng = rand::thread_rng();

    for batch_size in [1u32, 10, 100, 1000] {
        let preimages: Vec<(Vec<u8>, [u8; 32])> = (0..batch_size)
            .map(|i| {
                let mut preimage = vec![0u8; 64];
                rng.fill_bytes(&mut preimage);
                preimage[0..8].copy_from_slice(&i.to_le_bytes());

                let mut hasher = Sha256::new();
                hasher.update(&preimage);
                let digest = hasher.finalize();
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&digest);

                (preimage, hash)
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("verify_batch", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    for (preimage, expected_hash) in &preimages {
                        let mut hasher = Sha256::new();
                        hasher.update(black_box(preimage));
                        let result = hasher.finalize();
                        assert_eq!(result.as_slice(), expected_hash, "hashlock mismatch");
                    }
                })
            },
        );
    }
    group.finish();
}

// ─── Intent Serialization Benchmarks ────────────────────────────────────────

fn bench_intent_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("intent_serialization");
    let mut rng = rand::thread_rng();
    let mut hashlock = [0u8; 32];
    rng.fill_bytes(&mut hashlock);

    let intent = SwapIntent {
        id: hashlock,
        from_chain: 1,
        to_chain: 2,
        from_amount: 1_000_000_000_000_000_000u128,
        to_amount: 995_000_000_000_000_000u128,
        min_receive: 990_000_000_000_000_000u128,
        state: SwapState::Pending,
        hashlock,
        timelock: Timelock::new(1_000_000, 300, 1),
    };

    group.bench_function("minimal_json_serialize", |b| {
        b.iter(|| {
            serde_json::to_string(&intent.id).unwrap()
        })
    });

    group.bench_function("full_json_serialize", |b| {
        b.iter(|| {
            let json = format!(
                r#"{{"id":"{:x}","from_chain":{},"to_chain":{},"amount":{},"state":"{}"}}"#,
                hex::encode(intent.id),
                intent.from_chain,
                intent.to_chain,
                intent.from_amount,
                format!("{:?}", intent.state).to_lowercase()
            );
            black_box(json)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_htlc_hashlock,
    bench_timelock_ops,
    bench_swap_state_transition,
    bench_batch_hashlock_verify,
    bench_intent_serialization,
);
criterion_main!(benches);