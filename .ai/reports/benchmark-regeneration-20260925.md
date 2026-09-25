# The weight-regeneration path was broken, and `submit_comit_v2` has no benchmark

Date: 2026-09-25. Scope: `runtime/Cargo.toml`, `runtime/src/lib.rs` (`benches`),
`pallets/x3-kernel/src/benchmarks` + `weights.rs`. Motive: `submit_comit_v2`'s weight is a
hand-written placeholder ("Conservative: same base weight as submit_comit until benchmarks are
re-run"), and this session changed that extrinsic (the X3 payload is now the program, and the comit
persists an execution receipt), so the placeholder needed either a measurement or a reason.

## What was actually wrong, in the order it surfaced

1. **`cargo build --release --features runtime-benchmarks` did not compile at all.** It failed with
   `E0046: not all trait items implemented, missing: try_successful_origin` at
   `pallets/x3-custody/src/lib.rs:249`. The pallet *has* that method — behind
   `#[cfg(feature = "runtime-benchmarks")]` — but the runtime's `runtime-benchmarks` feature did not
   forward `pallet-x3-custody/runtime-benchmarks`, so the cfg was off and the benchmark-only trait
   method was absent.
2. **It was not one pallet.** Of the 52 runtime dependencies that declare a `runtime-benchmarks`
   feature, the runtime forwarded it for 33 (after this change) and **19 have never compiled with it
   on**: 15 fail in a std build and 4 more fail in the wasm/no_std build. Each is recorded in
   `runtime/Cargo.toml` with its first error. The pattern is that nothing ever compiled them:
   `E0046 try_successful_origin` (solvency, reservation, rebalance, partner, treasury-policy),
   `E0433 std` / undeclared `Box` (x3-coin, x3-automation, governance, agent-accounts, agent-memory),
   missing types/items (`BalanceOf`, `MinProviderStake::get`, `Balance::max_value`,
   `Config::RuntimeEvent`, `impl_test_function`), one parse error, one `x3-vrf` failure, and two
   packages outside the root workspace (`pallet-x3-wallet-pallet`, `pallet-svm-runtime`).
3. **The kernel was not registered for benchmarking.** `define_benchmarks!` listed four pallets and
   `pallet_x3_kernel` was not one, so even with the build fixed the CLI answered "No benchmarks found
   which match your input". Registering it needs the `construct_runtime!` alias (`AtlasKernel`), not
   an arbitrary one: the macro's `list_benchmarks!`/`add_benchmarks!` expansions resolve at crate
   root, where only that alias exists.
4. **A stale wasm can answer with the previous benchmark list.** `wasm-builder` did not rebuild the
   blob when only the runtime's source changed, so `--list --all` kept showing the old four pallets;
   removing `target/release/wbuild/x3-chain-runtime` and rebuilding produced the kernel's nine.
5. **`submit_comit_v2` has no benchmark.** With the build fixed and the pallet registered, the CLI
   lists these nine kernel benchmarks: `add_authority`, `authorize_account`, `deauthorize_account`,
   `enact_authority_change`, `register_asset`, `remove_authority`, `schedule_authority_change`,
   `submit_comit`, `update_canonical_balance`. `submit_comit_v2` is not among them, which is exactly
   what its own comment admits: there is nothing to re-run.

## Why writing that benchmark is not a five-minute copy

`submit_comit_v2` executes an X3 payload through `T::X3Adapter`, and on this runtime that adapter is
the real `X3VmAdapter`, which validates the payload as X3BC (magic, version gate, checksum). A
benchmark that passed a synthetic payload would fail on the runtime — and would have *passed*
against the pallet's own mock, whose `TestX3Adapter::validate` returns `Ok(())` for anything. So the
benchmark needs a real compiled artifact as its fixture (small, but genuinely compiled), and the
EVM/SVM payloads need to be the valid packet forms `verify_payloads_v2` demands. That is a piece of
work with its own correctness standard, not a copy of `submit_comit`'s body.

## What this change lands

* `runtime/Cargo.toml`: forwards `runtime-benchmarks` for the 33 dependencies whose benchmark code
  compiles, and records the 19 that do not with their first error (measured, per pallet, with
  `cargo check -p <crate> --features runtime-benchmarks`, and the four extra that only fail in the
  wasm build found by iterating `cargo build --features runtime-benchmarks`).
* `runtime/src/lib.rs`: registers `pallet_x3_kernel` in `define_benchmarks!`, so the kernel's nine
  benchmarks are runnable for the first time.
* Nothing about the default build changes: the `benches` module and these forwards are all behind
  `runtime-benchmarks`, which is off by default. Verified: `cargo check --workspace`,
  `cargo test -p x3-chain-runtime --lib` (53), `cargo test -p pallet-x3-kernel` (218) all green.

## What is still open

1. Write the `submit_comit_v2` benchmark (needs a compiled X3BC fixture + valid packet payloads), then
   generate its weight and delete the placeholder comment in `weights.rs`.
2. Fix the 19 benchmark modules that have never compiled, or say per pallet why not.
3. `wasm-builder` not tracking runtime source changes is a footgun for anyone measuring weights: the
   measured blob can be stale without any error. Worth a note in the release docs, or a gate.

## Second pass (same day): the benchmark exists, and what it now stops on

The missing `submit_comit_v2` benchmark is written, and the fixtures it needs are in place:

* `pallets/x3-kernel/src/bench_fixtures.rs` holds a **compiled** X3 artifact (63 bytes, from
  `fn main() -> i64 { return 42; }`) beside the source it came from, with two tests that run in the
  pallet's suite: the bytes must equal `compile_source(source)` today, and the production adapter
  (`X3VmAdapter::validate`) must accept them. A hand-written byte blob would not have been evidence —
  TICKET-108 is the story of fixtures that were not what the compiler emits.
* The benchmark module's EVM and SVM payload helpers now build **valid packets** through
  `test_helpers` (shared with the benchmark build), instead of `0xa9059cbb` + zeros and ELF magic +
  zeros. Measured before the change: `Benchmark pallet_x3_kernel::submit_comit failed:
  InvalidEvmPacket`. That benchmark had been registered and had never run, so nothing had said so.
* `runtime/src/lib.rs` registers `pallet-x3-kernel` for benchmarking, so both extrinsics are listed.

With those in place the CLI runs both benchmarks and they fail at the **next** stage:

```
Benchmark pallet_x3_kernel::submit_comit    failed: SvmExecutionFailed
Benchmark pallet_x3_kernel::submit_comit_v2 failed: SvmExecutionFailed
```

That is the real state of this benchmark module: its fixtures are *shaped* right now (they validate as
packets) but they are not *executable* by the runtime's adapters. `submit_comit`/`submit_comit_v2`
execute each non-empty payload through `T::EvmAdapter`/`T::SvmAdapter`, which on this runtime are the
real adapters, not the mock: an EVM leg needs a signed RLP transaction and an SVM leg needs an
instruction payload those adapters can run. So the weight table's `submit_comit_v2` entry stays a
placeholder, and now for a *specific* reason rather than a vague one.

### What the next pass needs

1. An executable EVM fixture (a signed transaction the frontier stack accepts, with the benchmark
   caller funded) and an executable SVM instruction payload for the runtime's SVM adapter.
2. Then `benchmark pallet --pallet=pallet_x3_kernel --extrinsic=submit_comit_v2 --output=...` can
   produce a real weight, and the placeholder comment in `weights.rs` can go.
3. Until then, the honest statement in the row is: the benchmark exists, it is registered, and it
   fails on the payload fixtures — not "it needs re-running".
