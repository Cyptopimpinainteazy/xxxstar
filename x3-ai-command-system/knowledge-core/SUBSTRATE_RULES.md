# Substrate Rules — Knowledge Core

## Overview

These are the mandatory security rules for all Substrate/Rust runtime code in the X3 ecosystem. The Substrate runtime is the foundation of X3's native layer, and its correctness directly affects consensus, staking, governance, and all VM coordination. Substrate runtime code must be deterministic, safe, and upgradeable without breaking state.

## Safety Fundamentals

### Rule SUB-1: No Unsafe Without Justification

- `unsafe` blocks are forbidden in runtime code unless there is a written justification reviewed by a security auditor.
- Every `unsafe` block must have a safety comment explaining why the operation is safe.
- `unsafe` must not be used to bypass runtime checks (bounds, overflow, type casts).
- If a dependency uses `unsafe`, it must be audited and the audit must be documented.
- Prefer safe abstractions over raw `unsafe` operations.

### Rule SUB-2: No Panics in Runtime

- Runtime code must never panic. Panics in the runtime can halt block production.
- Do not use `unwrap()`, `expect()`, or `indexing` operations that can panic. Use `ok()`, `map_err()`, and checked operations instead.
- Do not use `assert!()` for conditions that depend on user input. Use `ensure!()` which returns an error.
- Do not use `Vec::remove()`, `Vec::swap_remove()`, or `Vec::insert()` with unbounded indices. Use bounded collections.
- All arithmetic must use checked or saturating operations. No `+`, `-`, `*` on integer types without explicit overflow handling.
- `#[pallet::hooks]` must not panic. All hook logic must be fallible or silently skip invalid states.

### Rule SUB-3: Result Types

- All extrinsics must return `DispatchResult` or `DispatchResultWithPostInfo`.
- All runtime API calls must return `Result<T, E>` where `E` is a well-defined error type.
- All storage operations must return `Result`. Use `Option<T>` where absence is expected, not as an error type.
- Error types must be exhaustive and documented. No catch-all error variants.
- Do not discard errors with `.ok()` or `let _ =`. Log or propagate all errors.

## Bounded Collections

### Rule SUB-4: Bounded Collections

- All vectors, B-trees, and maps in runtime storage must be bounded.
- Use `BoundedVec<T, S>`, `BoundedBTreeMap<K, V, S>`, and `BoundedBTreeSet<T, S>` instead of unbounded collections.
- The bound `S` must be a reasonable maximum that prevents unbounded state growth.
- Iterating over large collections must use pagination. Do not iterate over the entire collection in a single block.
- Do not store unbounded `Vec<T>` in storage. Convert to `BoundedVec<T, S>` or use a separate storage map with a count.

## Deterministic Execution

### Rule SUB-5: Deterministic Execution

- Runtime code must produce identical results on all validators. No floating-point arithmetic, no random number generation without a deterministic seed, no timestamps (use block numbers instead), no network I/O.
- Do not use `std::time`, `std::thread`, `std::net`, or any `std` I/O in runtime code.
- Hashing must use the runtime's configured hash function (`Hashing`), not a hardcoded algorithm.
- Sorting must be deterministic. Use `BTreeMap` instead of `HashMap` for ordered iteration.
- Do not rely on `HashMap` iteration order. It is non-deterministic across runs.

## Weight and Benchmark

### Rule SUB-6: Weight and Benchmark Implications

- Every extrinsic must have a `#[pallet::weight]` annotation.
- Weight must account for the worst-case execution path, not the average case.
- Weight must be benchmarked using `frame-benchmarking`. Do not guess weights.
- Weight must include the cost of storage reads and writes, not just computation.
- Weight must be proportional to the size of the data being processed. Unbounded data means unbounded weight, which is a bug.
- Do not use `Weight::from_ref_time(0)` for non-trivial operations. Zero-weight extrinsics that do meaningful work can be used to attack the network.
- Weight refunds must be accurate. Overestimating is acceptable; underestimating is not.

## Storage Migrations

### Rule SUB-7: Storage Migrations with Versioning

- Every runtime upgrade that changes storage must include a storage migration.
- Storage migrations must be tested against the previous state.
- Storage migrations must be idempotent — running them twice must not corrupt state.
- Use `StorageVersion` to track the current storage version.
- Migrations must not panic. If a migration fails, it must log the error and continue (or halt the upgrade).
- Migrations must be benchmarked. Large migrations may need to be spread across multiple blocks using `OnRuntimeUpgrade` hooks.
- Document all migrations in the runtime's `MIGRATION.md` or equivalent.

## Runtime API Stability

### Rule SUB-8: Runtime API Stability

- Runtime API calls must be backward-compatible. Adding new calls is acceptable; removing or changing existing calls is not.
- Use `#[api::compile]` to ensure API compatibility.
- Use runtime versioning (`spec_version`, `impl_version`) correctly. `spec_version` must increase for any breaking change.
- Do not change the encoding of existing types. If a type needs to change, create a new type and migrate.
- All types that cross the runtime/API boundary must implement `Encode` and `Decode` correctly.

## Cross-VM Coordination (Substrate Side)

### Rule SUB-9: Cross-VM Coordination

- The Substrate runtime is the coordination layer for all VMs on X3. It must correctly dispatch cross-VM messages and handle responses.
- Cross-VM dispatch must be deterministic. The same sequence of messages must produce the same result on all validators.
- Cross-VM messages must have a timeout. If a destination VM does not respond within the timeout, the source VM must be refunded.
- The runtime must maintain the canonical supply invariant (`X3_ARCHITECTURE.md`). The `pending` term must be tracked correctly.
- Storage items that track cross-VM state must be bounded and garbage-collected.
- Cross-VM message ordering must be deterministic. Use a sequence number or Merkle root to ensure all validators process messages in the same order.

## Error Handling

### Rule SUB-10: Error Handling

- All errors must be well-defined, documented, and exhaustive.
- Use `pallet::Error` for pallet-specific errors. Do not use generic error types.
- Errors must be actionable. Include enough information for the caller to understand what went wrong and how to fix it.
- Do not log sensitive information in errors (private keys, account balances beyond what is necessary).
- Errors must not leak internal state that could be used for attacks.

## Testing Requirements

### Rule SUB-11: Testing Requirements

Every pallet must have:

- **Unit tests** for all dispatchables, hooks, and storage operations.
- **Integration tests** for cross-pallet interactions.
- **Benchmark tests** that measure the weight of every extrinsic.
- **Fuzz tests** for extrinsics that accept complex input.
- **Migration tests** that verify storage upgrades against the previous state.
- **Invariant tests** for critical invariants (total supply, account balances, staking totals, cross-VM balances).

### Rule SUB-12: Test Frameworks

- Use `#[cfg(test)]` with `frame_support::assert_ok!` and `assert_err!` for unit tests.
- Use `test-runtime` for integration tests.
- Use `frame-benchmarking` for benchmark tests.
- Use `substrate-test-runtime` for end-to-end tests.
- Use `cargo fuzz` for fuzz testing.

## Deployment Checklist

1. Runtime binary is compiled in `--no-std` mode (no `std` in runtime).
2. All weights are benchmarked and accurate.
3. Storage migrations are tested and versioned.
4. Runtime API is backward-compatible.
5. Cross-VM coordination is tested end-to-end.
6. Canonical supply invariant is verified.
7. No `unsafe` blocks without justification.
8. No `unwrap()` or `expect()` in production code.
9. All collections are bounded.
10. Events are emitted for all state changes.

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — The Substrate runtime is the coordination layer defined in the architecture.
- **UNIVERSAL_ASSET_KERNEL.md** — The UAK is enforced in the Substrate runtime.
- **CROSS_VM_ROUTING.md** — Cross-VM messages are dispatched from the Substrate runtime.
- **MAINNET_READINESS.md** — Substrate runtime must pass all readiness checks before deployment.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*