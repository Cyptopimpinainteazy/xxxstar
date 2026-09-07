#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Tokenomics & DEX Spotlight

== The Supply Ledger: A Model for How to Do This Right

`pallets/x3-supply-ledger` enforces `represented_total ≤ canonical_supply` at five separate mutation call sites (`do_mint_canonical`, `do_burn_canonical`, `debit_source_to_pending`, `credit_destination_from_pending` twice) *and* redundantly sweeps every registered asset every block in `on_finalize`. This was executed live in this audit: `cargo test -p pallet-x3-supply-ledger -- --nocapture` passed 33/33, including a fuzz test (`fuzz_all_operations_preserve_invariant`) that drives randomized operation sequences against the invariant rather than only fixed examples. The violation policy (`InvariantViolationPolicy::EventAndPause`) is fail-closed: on any detected violation, `TransferHalted` is set and further transfers/mints/swaps are blocked until governance intervenes.

This is the pattern every other money-handling pallet in this codebase should match. One does not.

== The DEX: Where That Pattern Breaks Down

`crates/x3-dex/src/amm_pools.rs` computes every swap price and every LP mint/burn/add/remove-liquidity ratio using `f64` floating-point arithmetic. This is HIGH-02, and it is worth spending a full chapter section on because it is easy to under-rate from a one-line description.

```rust
// crates/x3-dex/src/amm_pools.rs (paraphrased from the audited lines)
let fee = (amount_in as f64) * (pool.fee_basis_points as f64) / 10000.0;
let amount_in_after_fee = (amount_in as f64) - fee;
let amount_out = ((amount_in_after_fee * (reserve_out as f64))
    / ((reserve_in as f64) + amount_in_after_fee)) as u128;
```

Two independent problems compound here:

+ *Precision loss.* `u128 as f64` silently truncates to 53 bits of mantissa. Reserves or swap amounts above roughly 9×10^15 — well within normal range for an 18-decimal token, representing under 0.01 whole tokens — lose precision on *every* swap. Repeated small trades against a large pool can systematically extract value through rounding bias, a well-documented AMM attack class.
+ *Determinism.* This code compiles into the WASM runtime every validator executes. Under normal conditions IEEE-754 float math is deterministic across conformant backends, but Substrate nodes support toggling native vs. WASM execution strategy, and floating-point behavior at edge cases (subnormals, NaN payloads) is implementation-defined at exactly the boundary where different backends could diverge — precisely the class of bug blockchain engineering practice exists to forbid in state-transition code.

All 14 pallet tests for the DEX pass. This is not reassuring: the tests assert the float implementation is *self-consistent* (e.g. `test_invariant_preservation`), never that it matches an integer/decimal reference or produces identical output across execution modes. A passing test suite here is a false signal of correctness — exactly the audit standard this document holds the rest of the repository to.

#callout(kind: "critical", title: "Fix pattern already exists in this same codebase")[
  The fix is not novel: `pallets/x3-supply-ledger` already demonstrates the correct pattern one pallet over — `checked_mul`/`checked_div`/`saturating_*` on `u128`, widened to `U256` where an intermediate product could overflow. HIGH-02's recommendation is to port that exact pattern into `amm_pools.rs`'s pricing functions, replacing every `as f64` conversion.
]

== Everything Else in Tokenomics Checked Out

- *LP anti-rug locks* (`pallets/x3-lp-locker`): a real block-number check (`ensure!(current_block >= record.unlock_at_block, ...)`), not a boolean flag. 19/19 tests passed live, including `unlock_lp_rejects_before_expiry` and `extend_lock_rejects_shorten`.
- *Wallet mint authorization* (`pallets/x3-wallet-pallet::mint_tokens`): matches its own `BUILD_VERIFICATION_REPORT.md` claim exactly — `ensure_minter` plus `checked_add`.
- *Validator "staking" language*: the repository's own claim that permissionless staking/bonding/nomination is "NOT implemented, deferred to M3" is accurate — confirmed by a repo-wide grep for `pallet_staking` returning nothing. A real (but non-permissionless) slashing/stake-bookkeeping system exists instead via `pallets/x3-consensus`.
- One low-severity hygiene item remains: `pallets/x3-wallet-pallet/src/lib.rs:249` credits a receiver's balance with raw `+` rather than `checked_add` (LOW-02) — not currently exploitable given the ledger's bounded mint invariants elsewhere, but inconsistent with the codebase's own stated discipline.
