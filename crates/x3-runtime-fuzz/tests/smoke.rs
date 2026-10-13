//! Seeded smoke run of the whole-runtime harness, cheap enough for every CI run.
//!
//! Random bytes almost never decode into a valid `RuntimeCall`, so this starts from
//! encoded real calls and applies deterministic byte mutations (flip, insert, delete,
//! truncate, splice). It is not a substitute for a coverage-guided campaign; it proves
//! the harness runs on this runtime and pins any invariant violation a mutation hits.
//! `X3_RUNTIME_FUZZ_ITERS` raises the iteration count for longer local runs.

use pallet_balances::Call as BalancesCall;
use sp_runtime::MultiAddress;
use x3_chain_runtime::RuntimeCall;
use x3_runtime_fuzz::{encode_step, genesis, process_input};

/// xorshift64*: deterministic, dependency-free.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn seeds(accounts: &[x3_chain_runtime::AccountId]) -> Vec<Vec<u8>> {
    let to = |i: usize| MultiAddress::Id(accounts[i % accounts.len()].clone());
    let calls: Vec<(bool, u8, RuntimeCall)> = vec![
        (
            false,
            0,
            RuntimeCall::Balances(BalancesCall::transfer_allow_death {
                dest: to(1),
                value: 1_000_000,
            }),
        ),
        (
            false,
            1,
            RuntimeCall::Balances(BalancesCall::transfer_keep_alive {
                dest: to(2),
                value: 10u128.pow(20),
            }),
        ),
        (
            true,
            2,
            RuntimeCall::Balances(BalancesCall::transfer_all {
                dest: to(0),
                keep_alive: false,
            }),
        ),
        (
            false,
            4,
            RuntimeCall::Balances(BalancesCall::transfer_allow_death {
                dest: to(3),
                value: u128::MAX,
            }),
        ),
        (
            true,
            3,
            RuntimeCall::Balances(BalancesCall::burn {
                value: 12_345,
                keep_alive: true,
            }),
        ),
    ];
    let mut out: Vec<Vec<u8>> = calls
        .iter()
        .map(|(a, o, c)| encode_step(*a, *o, c))
        .collect();
    out.push(
        calls
            .iter()
            .flat_map(|(a, o, c)| encode_step(*a, *o, c))
            .collect(),
    );
    out
}

fn mutate(rng: &mut Rng, pool: &[Vec<u8>]) -> Vec<u8> {
    let mut data = pool[rng.below(pool.len())].clone();
    for _ in 0..1 + rng.below(4) {
        match rng.below(5) {
            0 if !data.is_empty() => {
                let i = rng.below(data.len());
                data[i] ^= 1 << rng.below(8);
            }
            1 => {
                let i = rng.below(data.len() + 1);
                data.insert(i, rng.next() as u8);
            }
            2 if data.len() > 1 => {
                let i = rng.below(data.len());
                data.remove(i);
            }
            3 if !data.is_empty() => data.truncate(rng.below(data.len()) + 1),
            _ => data.extend_from_slice(&pool[rng.below(pool.len())]),
        }
    }
    data
}

#[test]
fn genesis_builds_from_the_dev_preset() {
    let g = genesis();
    assert!(g.accounts.len() >= 2, "need at least two endowed accounts");
    // Visible in the test log: which preset sections this runtime build cannot load.
    println!("dropped preset sections: {:?}", g.dropped_sections);
}

#[test]
fn seeds_execute_and_keep_every_invariant() {
    let g = genesis();
    let pool = seeds(&g.accounts);
    for seed in &pool {
        assert!(process_input(&g, seed) > 0, "a seed decoded to no call");
    }
}

#[test]
fn mutated_inputs_keep_every_invariant() {
    let g = genesis();
    let pool = seeds(&g.accounts);
    let iters: usize = std::env::var("X3_RUNTIME_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000);
    let mut rng = Rng(0x5EED_0000_0000_0003);
    let mut dispatched = 0;
    for _ in 0..iters {
        dispatched += process_input(&g, &mutate(&mut rng, &pool));
    }
    // Most mutations break decoding; the run is only meaningful if many still dispatch.
    assert!(
        dispatched > iters / 4,
        "only {dispatched} calls dispatched in {iters} mutated inputs"
    );
}
