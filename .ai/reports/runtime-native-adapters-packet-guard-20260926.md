# The native adapters' packet guard had no test — and no gate ran the tests that could have caught that

Date: 2026-09-26. Scope: `runtime/src/lib.rs`, `runtime/Cargo.toml`, `scripts/local-ci.sh`.
Motive: `aaece429d` ("refuse a packet instead of reporting a false EVM/SVM success") put the same
guard at four adapter sites, but the regression tests it added cover only two of them — the two
`wasm_adapters` in `pallets/x3-kernel`. The runtime's own `NativeEvmAdapter` / `NativeSvmAdapter`
got the guard and nothing else.

## What was wrong

1. **The guard's tests could not reach the guard.** `native_vm_adapters` and `vm_adapter_tests` are
   both `#[cfg(all(test, feature = "std", feature = "frontier"))]`, and `frontier` is off by default
   (`runtime/Cargo.toml`: "Real Frontier EVM execution path (post-RC1; OFF by default)"). The gate
   named `test runtime` is `cargo test -p x3-chain-runtime`, so it compiles neither the module nor
   its tests.
2. **The one gate that passes the feature filters the tests out**:
   `scripts/check-runtime-variants.sh` runs `cargo test -p x3-chain-runtime --no-default-features
   --features std,frontier runtime_upgrade_rehearsal` — the feature is on, but only the migration
   rehearsal tests run. Measured: the module compiled, 0 of its tests executed.
3. **One of those tests could not fail.** `test_x3_adapter_real_execution` ended in
   `assert!(result.is_ok() || result.is_err())`, which is true for every possible verdict, next to a
   comment admitting the author did not know the answer ("may return Ok or Err for partial
   payload"). A test that cannot fail is not coverage, and it sat in the one module the chain's only
   real EVM path lives in.

## What landed

* `runtime/src/lib.rs`: `a_packet_is_refused_by_the_native_adapters`.
  * It builds the fixtures the kernel's own suite builds — `EvmPacket::Call { args: intent }`,
    `SvmPacket::Invoke { data: intent }` — via `x3-packet-schema` (new **dev**-dependency), so a
    fixture that stopped being a packet fails its own precondition instead of testing nothing.
  * Each test asserts the precondition (`deserialize_packet` accepts it, `get_domain_mask` matches
    the slot, and for EVM that the first byte is the `Packet` discriminant `0x00` = EVM `STOP`),
    then requires the adapter to refuse by name.
  * `the_frontier_create_path_the_guard_replaced_reports_success_for_the_same_bytes` hands those
    same bytes to the Frontier `create` call the guard sits in front of, and requires the
    *successful* contract creation with no code that the pre-guard path reported. The guard's
    rationale is therefore measured in the same run rather than asserted in a commit message, and
    deleting the guard turns the other two tests red.
* `runtime/src/lib.rs`: `test_x3_adapter_real_execution` now requires the refusal by name
  (`Invalid X3 bytecode`) for a 4-byte magic-only artifact — the verifier's `parse` refuses anything
  shorter than 6 bytes with `UnexpectedEof` before it reads a version
  (`crates/x3-vm/src/bytecode.rs`).
* `scripts/local-ci.sh`: `test runtime frontier` — `env SKIP_WASM_BUILD=1 cargo test -p
  x3-chain-runtime --no-default-features --features std,frontier`, the whole suite under the feature
  set the `frontier` variant builds, in the fast set. This is the gate that executes the native
  adapters for the first time.

## Measured

```text
env SKIP_WASM_BUILD=1 cargo test -p x3-chain-runtime --no-default-features --features std,frontier
  -> 60 passed + 8 passed + 21 passed; 0 failed; 0 ignored
     (was: the lib suite could not compile under the default feature set, and the variant run
      executed 0 of these)

env SKIP_WASM_BUILD=1 cargo test -p x3-chain-runtime            # the existing `test runtime` gate
  -> 53 passed + 8 passed + 21 passed; unchanged

cargo fmt --all -- --check                                       -> clean
cargo clippy -p x3-chain-runtime --no-default-features --features std,frontier --lib --tests
  -> 5 warnings, all in code this change does not touch
     (unused `codec::Encode` at 3632, `keccak_256` at 4111, `H256` at 4114, `BlakeTwo256` at 4115,
      `FrontierPrecompiles::new` without `Default` at precompiles.rs:16)
```

## What remains open

1. **The payload convention itself (X3-LANG-004).** Both arms still disagree with themselves: the
   kernel demands a SCALE-encoded `Packet` with the domain bit and the adapters execute bytes, so
   every non-empty EVM/SVM payload in `submit_comit_v2` is now refused at execution. The X3 arm
   settled this in `aaece429d`'s sibling change — "ask the component that will do the work", i.e.
   `T::X3Adapter::validate` — and the EVM/SVM arms still ask the packet decoder instead. Applying
   that precedent to all three arms (and updating `packet_integration_tests`, which pins the packet
   requirement for EVM/SVM) is an architecture decision, not a cleanup; it also moves the runtime's
   WASM bytes and so needs its own re-attestation.
2. **`FrontierEvmAdapter` may be the same false success, still live.** Found while reading
   `pallets/x3-kernel/src/adapters.rs` for this change:
   `real_adapters::FrontierEvmAdapter::execute` computes a gas figure from the payload length and
   returns `ExecutionReceipt { success: true, .. }` without executing anything ("For native
   execution, we perform basic validation and return a success receipt"). If any runtime config
   wires that adapter, it is the exact shape `aaece429d` removed — needs a reachability check and, if
   reachable, the same refusal treatment.
3. **The `frontier` build is not warning-clean, and nothing says so.** The five warnings above are
   pre-existing and only appear with `--features frontier`; a clippy gate with that feature and
   `-D warnings` would fail today. Adding one is the natural follow-up to `test runtime frontier`.
4. **`generate_proof`/`--features dev` variants** are still only exercised for
   `runtime_upgrade_rehearsal`; the `frontier` gate added here covers one variant's *whole* suite,
   the other five variants' non-rehearsal tests remain unexecuted.

## The fork this run could not resolve

This change deliberately stays inside test code, the manifest, and the gate list. It does not touch
`pallets/x3-kernel` or the runtime's production paths, because another agent in this worktree is
mid-flight re-attesting the runtime (`docs/reports/runtime-wasm-hashes.json`,
`docs/reports/runtime-wasm-reproducibility.md`, `reports/rc6/*` are dirty under its hand): any
source change that moves the WASM bytes would invalidate the record it is writing.
