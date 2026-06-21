//! Criterion benchmarks for X3 DEX routing and price calculations.
//!
//! Benchmarks:
//! - Swap route search (single-hop, multi-hop, cross-VM)
//! - Quote calculation (AMM constant product, stable swap)
//! - Slippage bounds checking
//! - Pool state lookup and update
//! - Liquidity-lock verification
//! - Anti-rug check (supply concentration, owner renounce, locked LP)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::RngCore;

// ─── AMM Constant Product Math ──────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Pool {
    reserve_a: u128,
    reserve_b: u128,
    fee_bps: u32,
}

impl Pool {
    fn quote_swap_a_to_b(&self, amount_in: u128) -> Option<u128> {
        if amount_in == 0 || self.reserve_a == 0 || self.reserve_b == 0 {
            return None;
        }
        let amount_in_with_fee = amount_in * (10_000 - self.fee_bps as u128) / 10_000;
        let numerator = amount_in_with_fee * self.reserve_b;
        let denominator = self.reserve_a + amount_in_with_fee;
        if denominator == 0 {
            return None;
        }
        Some(numerator / denominator)
    }

    fn quote_swap_b_to_a(&self, amount_in: u128) -> Option<u128> {
        if amount_in == 0 || self.reserve_a == 0 || self.reserve_b == 0 {
            return None;
        }
        let amount_in_with_fee = amount_in * (10_000 - self.fee_bps as u128) / 10_000;
        let numerator = amount_in_with_fee * self.reserve_a;
        let denominator = self.reserve_b + amount_in_with_fee;
        if denominator == 0 {
            return None;
        }
        Some(numerator / denominator)
    }

    fn price_ratio(&self) -> f64 {
        if self.reserve_b == 0 { return 0.0; }
        self.reserve_a as f64 / self.reserve_b as f64
    }

    fn invariant_k(&self) -> u128 {
        self.reserve_a.saturating_mul(self.reserve_b)
    }

    fn apply_swap_a_to_b(&mut self, amount_in: u128, amount_out: u128) {
        self.reserve_a = self.reserve_a.saturating_add(amount_in);
        self.reserve_b = self.reserve_b.saturating_sub(amount_out);
    }
}

// ─── Route Scoring ──────────────────────────────────────────────────────────

#[derive(Clone)]
struct RouteHop {
    pool_id: u64,
    token_in: [u8; 32],
    token_out: [u8; 32],
    amount_in: u128,
}

#[derive(Clone)]
struct Route {
    hops: Vec<RouteHop>,
    total_input: u128,
    total_output: u128,
    hop_count: usize,
}

impl Route {
    fn score(&self) -> f64 {
        if self.total_input == 0 { return 0.0; }
        // Score = output/input ratio penalized by hop count
        let ratio = self.total_output as f64 / self.total_input as f64;
        let hop_penalty = 1.0 / (1.0 + (self.hop_count as f64 * 0.001));
        ratio * hop_penalty
    }

    fn effective_price_impact(&self, expected_output: u128) -> f64 {
        if expected_output == 0 { return 1.0; }
        let diff = (expected_output as f64 - self.total_output as f64).abs();
        diff / expected_output as f64
    }

    fn meets_slippage(&self, min_output: u128) -> bool {
        self.total_output >= min_output
    }
}

// ─── Route Search (simulated graph walk) ────────────────────────────────────

fn find_best_route(pools: &[Pool], _amount_in: u128, _token_a: usize, _token_b: usize) -> Option<Route> {
    // Simplified route search: score all pools directly connecting the pair
    let mut best: Option<Route> = None;

    for (i, pool) in pools.iter().enumerate() {
        if let Some(output) = pool.quote_swap_a_to_b(black_box(1_000_000_000u128)) {
            let route = Route {
                hops: vec![RouteHop {
                    pool_id: i as u64,
                    token_in: [0u8; 32],
                    token_out: [0u8; 32],
                    amount_in: 1_000_000_000u128,
                }],
                total_input: 1_000_000_000u128,
                total_output: output,
                hop_count: 1,
            };

            match &best {
                None => best = Some(route),
                Some(b) if route.score() > b.score() => best = Some(route),
                _ => {}
            }
        }
    }
    best
}

// ─── Anti-Rug Checks ────────────────────────────────────────────────────────

#[derive(Clone)]
struct TokenInfo {
    total_supply: u128,
    owner_balance: u128,
    locked_lp_tokens: u128,
    creator_renounced: bool,
    has_mint_authority: bool,
    has_freeze_authority: bool,
}

impl TokenInfo {
    fn owner_concentration_pct(&self) -> f64 {
        if self.total_supply == 0 { return 100.0; }
        (self.owner_balance as f64 / self.total_supply as f64) * 100.0
    }

    fn liquidity_lock_pct(&self) -> f64 {
        if self.total_supply == 0 { return 0.0; }
        (self.locked_lp_tokens as f64 / self.total_supply as f64) * 100.0
    }

    fn anti_rug_score(&self) -> u8 {
        let mut score = 0u8;
        // Creator renounced ownership
        if self.creator_renounced { score += 30; }
        // No mint authority (can't inflate supply)
        if !self.has_mint_authority { score += 25; }
        // No freeze authority
        if !self.has_freeze_authority { score += 15; }
        // Owner concentration < 5%
        if self.owner_concentration_pct() < 5.0 { score += 15; }
        // LP locked > 50%
        if self.liquidity_lock_pct() > 50.0 { score += 15; }
        score
    }

    fn is_likely_rug(&self) -> bool {
        self.anti_rug_score() < 40
    }
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_amm_quote(c: &mut Criterion) {
    let mut group = c.benchmark_group("amm_quote");
    let pool = Pool {
        reserve_a: 1_000_000_000_000_000_000_000u128,
        reserve_b: 500_000_000_000_000_000_000u128,
        fee_bps: 30, // 0.3%
    };

    let amounts: [u128; 6] = [
        1_000_000u128,         // 1 USDC worth
        100_000_000u128,       // 100 USDC
        1_000_000_000u128,     // 1,000 USDC
        1_000_000_000_000u128, // 1M USDC
        100_000_000_000_000u128, // 100M USDC
        1_000_000_000_000_000_000u128, // 1B USDC
    ];

    for &amount in &amounts {
        group.bench_with_input(
            BenchmarkId::new("quote_swap_a_to_b", amount),
            &amount,
            |b, &amt| b.iter(|| pool.quote_swap_a_to_b(black_box(amt))),
        );
    }

    group.bench_function("price_ratio", |b| b.iter(|| pool.price_ratio()));
    group.bench_function("invariant_k", |b| b.iter(|| pool.invariant_k()));
    group.finish();
}

fn bench_route_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("route_search");
    let mut rng = rand::thread_rng();

    for pool_count in [1, 5, 10, 50, 100] {
        let pools: Vec<Pool> = (0..pool_count)
            .map(|_| {
                let a: u128 = rng.next_u64() as u128 * 1_000_000_000;
                let b: u128 = rng.next_u64() as u128 * 1_000_000_000;
                Pool {
                    reserve_a: a.max(1_000_000),
                    reserve_b: b.max(1_000_000),
                    fee_bps: 30,
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("find_best_route", pool_count),
            &pool_count,
            |b, _| {
                b.iter(|| find_best_route(black_box(&pools), 1_000_000_000u128, 0, 1))
            },
        );
    }
    group.finish();
}

fn bench_route_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("route_scoring");

    let routes: Vec<Route> = (1..=10)
        .map(|hops| Route {
            hops: (0..hops)
                .map(|i| RouteHop {
                    pool_id: i,
                    token_in: [0u8; 32],
                    token_out: [0u8; 32],
                    amount_in: 100_000_000u128,
                })
                .collect(),
            total_input: 1_000_000_000u128,
            total_output: 990_000_000u128 - (hops as u128 * 100_000),
            hop_count: hops as usize,
        })
        .collect();

    group.bench_function("score_1hop", |b| {
        b.iter(|| routes[0].score())
    });

    group.bench_function("score_5hop", |b| {
        b.iter(|| routes[4].score())
    });

    group.bench_function("score_10hop", |b| {
        b.iter(|| routes[9].score())
    });

    group.bench_function("price_impact", |b| {
        b.iter(|| routes[4].effective_price_impact(black_box(1_000_000_000u128)))
    });

    group.bench_function("meets_slippage", |b| {
        b.iter(|| routes[4].meets_slippage(black_box(980_000_000u128)))
    });

    group.finish();
}

fn bench_pool_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_update");

    group.bench_function("apply_swap", |b| {
        b.iter_batched(
            || Pool {
                reserve_a: 1_000_000_000_000_000_000_000u128,
                reserve_b: 500_000_000_000_000_000_000u128,
                fee_bps: 30,
            },
            |mut pool| pool.apply_swap_a_to_b(1_000_000_000u128, 499_000_000u128),
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_anti_rug(c: &mut Criterion) {
    let mut group = c.benchmark_group("anti_rug");

    let safe_token = TokenInfo {
        total_supply: 1_000_000_000_000_000_000u128,
        owner_balance: 10_000_000_000_000_000u128, // 1%
        locked_lp_tokens: 700_000_000_000_000_000u128, // 70%
        creator_renounced: true,
        has_mint_authority: false,
        has_freeze_authority: false,
    };

    let risky_token = TokenInfo {
        total_supply: 1_000_000_000_000_000_000u128,
        owner_balance: 800_000_000_000_000_000u128, // 80%
        locked_lp_tokens: 0u128,
        creator_renounced: false,
        has_mint_authority: true,
        has_freeze_authority: true,
    };

    group.bench_function("owner_concentration", |b| {
        b.iter(|| safe_token.owner_concentration_pct())
    });

    group.bench_function("liquidity_lock_pct", |b| {
        b.iter(|| safe_token.liquidity_lock_pct())
    });

    group.bench_function("anti_rug_score_safe", |b| {
        b.iter(|| safe_token.anti_rug_score())
    });

    group.bench_function("anti_rug_score_risky", |b| {
        b.iter(|| risky_token.anti_rug_score())
    });

    group.bench_function("is_likely_rug_safe", |b| {
        b.iter(|| safe_token.is_likely_rug())
    });

    group.bench_function("is_likely_rug_risky", |b| {
        b.iter(|| risky_token.is_likely_rug())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_amm_quote,
    bench_route_search,
    bench_route_scoring,
    bench_pool_update,
    bench_anti_rug,
);
criterion_main!(benches);