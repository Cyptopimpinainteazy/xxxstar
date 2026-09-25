# X3 VM: an unenforced gas limit, and two readers that allocate from input

2026-09-25. Working the X3Lang/VM prove-items from the roadmap that do not need a live network.
`cargo test -p x3-x3-integration --features compile` was green before this, which is exactly why
the two defects below were worth going looking for: neither had a test that could have noticed.

## What was already covered, measured rather than assumed

* `crates/x3-integration/tests/bytecode_version_compat.rs` — old artifacts against a newer loader:
  patch bumps readable, newer minor / next major / future-loader-demand refused *by name*, a body
  edit refused with both checksums named, and the loader bounds asserted.
* `crates/x3-integration/tests/bc_const_pool_parity.rs` — the two readers agree about what a
  well-formed module means: constant pool, call arguments, globals, trailing debug sections and an
  unknown constant tag.
* Replay and rollback have coverage in the pallet: `submit_evm_transaction_rejects_replay_without_overwriting_stored_records`,
  `submit_comit_rejects_duplicate_comit_id`, `nonce_replay_attack_fails`,
  `admit_x3vm_call_inserts_and_rejects_duplicate`, `evm_failure_causes_full_rollback`,
  `rollback_restores_overlay_ledger_from_stored_leg_receipt`.

So the gaps were gas accounting and the readers' behaviour on *malformed* input.

## Defect 1: `X3Executor::execute` never applied its own gas limit

`X3ExecutorConfig` carries `gas_limit`, `max_call_depth`, `max_stack_size` and `trace`. The `std`
executor built the VM with `VM::from_bytes(bytecode)`, which uses `VMConfig::default()` — so every
one of those fields was discarded. `on_chain()` (500 000) and `simulation()` (10 000 000) both ran
under the VM's own 1 000 000, and a caller that asked for *less* got more.

Reproduced before the fix (`crates/x3-integration/tests/gas_accounting.rs`):

```
a 100 gas limit admitted a 2000-instruction program:
success=true gas_used=2002 instructions_executed=2001
```

The limit is what the caller is charged against and what the pallet reports, so it has to be the
bound the VM stops at. The executor now builds the VM with `VM::with_config(module, VMConfig {
gas_limit, max_call_depth, max_stack_size, trace })`, and the same program under the same limit
returns `GasExhausted { used, limit }`.

The `no_std` path was already correct — `mini_x3::execute_x3bc` takes the limit as an argument and
enforces it in its own loop — which is why this survived: the chain executes the `no_std` reader,
and the `std` path is reached by the node's adapters, simulations and tests. The test file pins both
to the same policy.

## Defect 2: a four-byte field could ask for a 95 GB allocation

Both readers read a table count out of the module and then call `Vec::with_capacity(count)` *before*
checking anything against the bytes that remain. The new mutation sweep
(`crates/x3-integration/tests/bytecode_robustness.rs`) hit it on the first run:

```
crates/x3-integration/src/mini_x3.rs   const pool:  memory allocation of 102676561944 bytes failed
crates/x3-backend/src/bc_format.rs     functions:   memory allocation of 171127603240 bytes failed
```

The mutants differ by one byte with the envelope checksum recomputed, so the failure is not the
checksum gate: `const_count = 0xFF00_0001` is a legal `u32`, and 4 278 190 081 entries × the size of
one entry is the number above.

The `mini_x3` one is the serious one. That reader is what `pallets/x3-kernel` executes on chain,
through `WasmX3Adapter`, so a module with a large count in the body is an allocation request the
runtime has to satisfy — a denial of service on block production, from input an attacker chooses.
The `x3-backend` copy is reached by the verifier and the `std` VM.

Fixed at every site of that shape by bounding the pre-allocation with the input that is left (no
entry is smaller than a byte, so this is a sound upper bound) — three tables in each reader, plus
the call-argument vector in the interpreter, which is `u16`-bounded and so smaller but the same
shape. The loops still report EOF on their own terms; nothing that used to be accepted is refused.

## The tests this produced

`crates/x3-integration/tests/gas_accounting.rs` — a limit below what the program needs stops it and
reports the caller's limit; a limit above admits it; the stricter `on_chain()` profile is applied
rather than replaced by the VM default.

`crates/x3-integration/tests/bytecode_robustness.rs` — the other half of the parity file:

* the fixture corpus is valid before it is damaged (so the rest cannot pass vacuously);
* every truncation of two modules is refused by all three readers (on-chain, `std`, verifier);
* bad magic, a future major, a newer minor, a future loader demand and a body that does not match
  its checksum are each refused by all three — the version-compat file does this for the `std`
  reader and `mini_x3` by name; this covers the whole triple for every header field;
* every body byte with four replacement values each, checksum recomputed so the reader underneath
  runs, must not panic — this is the sweep that found defect 2;
* the on-chain reader and the `std` reader must **agree** about every one of those damaged modules.
  A disagreement would mean a module the toolchain accepts could execute with different semantics
  on chain than off it. It holds today.

## Still open (ticket)

1. **The mutation sweep is a fixed corpus, not a fuzzer.** It is deterministic and runs in
   milliseconds, which is why it can be a gate; a `cargo-fuzz` target over the same two readers
   would cover inputs this does not. `x3-dns-server` and the SVM BPF reader are not covered by
   either.
2. **Other `with_capacity`-from-input sites elsewhere in the tree are unexamined.** The sweep found
   four in two files; the same pattern (a length read from input, then an allocation) is worth
   grepping for across the VM, the packet decoder and the bridge.
3. **`gas_used` on the `std` failure path is reported as the VM's count, not the caller's limit.**
   `GasExhausted { used, limit }` now carries both, which is honest, but a caller that assumed
   `used == limit` on exhaustion would be reading the wrong number.
