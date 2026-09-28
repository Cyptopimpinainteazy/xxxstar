# Lane: hand-guessed weights in runtime-registered pallets

Found by the repo scanner (`scripts/swarm/x3_repo_scan.py`, kind `pallet-call-without-weights`) on
2026-09-27. Reproduce with:

```bash
python3 scripts/swarm/x3_repo_scan.py | grep pallet-call-without-weights
```

It does not need a Bitcoin node, a validator or the network — it is a code-generation + verification
lane over `pallets/`.

## The defect

Registered pallets charge a number somebody typed:

```rust
#[pallet::weight(Weight::from_parts(10_000, 0))]
pub fn borrow(origin: OriginFor<T>, ...) -> DispatchResult { ... }
```

`crates/AGENTS.md`/`AGENTS.md` forbid exactly this, and this repository has already paid for it once:
PR #519 shipped a `weights.rs` that came from the generator's no-template fallback and did not
compile. The scanner's first run found **25** such sites, 13 of them in pallets that
`runtime/src/lib.rs` registers, so the guesses are what a live block pays.

## Deliverable

For each registered pallet in the list:

1. Add `runtime-benchmarks` plumbing if it is missing (`benchmarking.rs`, the `Config` bound,
   `#[cfg(feature = "runtime-benchmarks")]` on the module, `impl_benchmark_test_suite!`).
2. Generate real weights with the repository's own tools — not by hand:
   `BENCHMARK_STEPS=50 BENCHMARK_REPEAT=20 bash scripts/run-frame-benchmarks.sh run <pallet>`
   (build the node with `--features runtime-benchmarks` first; see
   `scripts/refresh-runtime-weights.sh`). Commit the generated `weights.rs`.
3. Replace every literal weight with the generated function:
   `#[pallet::weight(T::WeightInfo::<call_name>())]`, and add `type WeightInfo` to `Config` if it is
   absent.
4. If a call genuinely cannot be benchmarked yet, say so in a comment naming the ticket, keep the
   best available bound, and add the pallet to a shrink-only list so the *next* reader sees the
   decision. Do not silently leave a guess.

Start with the pallets on the money path: `x3-supply-ledger`, `x3-treasury-policy`, `x3-wrapped`,
`x3-token-factory`, `x3-wallet-pallet`, `x3-cross-vm-router`, `x3-settlement-engine`, `x3-kernel`.

## Proof required

* `cargo test -p <pallet> --features runtime-benchmarks` green (and the plain suite green).
* `python3 scripts/check-runtime-weights-wired.py` still OK — it fails if a config that can be wired
  to generated weights is left on `()`, and if `weights.rs` is unreachable.
* `python3 scripts/swarm/x3_repo_scan.py --check` — the `pallet-call-without-weights` count must go
  **down**; when it does, re-baseline (`--update-baseline`) in the same commit so the ratchet records
  the new floor.
* `cargo fmt --all -- --check`, and `scripts/mainnet/panic_unwrap_audit.sh` must not grow.
* Break-it-first on at least one pallet: put one literal back, show the scanner's count rise and the
  gate fail, restore byte-identically, show both green again.

## Rules

Shared tree: never `git add -A`, carve explicit paths, and do not edit `scripts/local-ci.sh` while a
local-ci run is executing. Do not re-baseline upward — if a count rises, that is the thing to fix.
