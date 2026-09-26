# The EVM/SVM payload convention: a payload is the artifact its adapter executes

Date: 2026-09-26
Scope: `pallets/x3-kernel/src/{lib,mock,test_helpers,wasm_adapters,tests,packet_integration_tests}.rs`,
`runtime/src/lib.rs`
Commit: `a3d918abb fix(x3-kernel): a payload is the artifact its adapter executes (X3-LANG-004)`
Predecessor: `.ai/reports/evm-payload-never-executed-20260926.md` (the false success this closes)

## The decision

The X3 arm of `submit_comit` settled this on 2026-09-25: **the payload is the artifact the adapter
executes, and the adapter that does the work is the component that validates it**
(`T::X3Adapter::validate`). This change applies the same rule to the EVM and SVM arms.

Before it, the kernel demanded a SCALE-encoded `Packet` carrying the slot's domain bit while every
adapter handed the same bytes to an interpreter as code. `Packet`'s first byte is the enum
discriminant `0x00`, which is EVM `STOP`, so the shape the kernel accepted was exactly the shape that
could not be executed — measured in the predecessor report as `execute -> Ok((success: true,
gas_used: 22576))` for an operation that never ran, with a receipt persisted for it.

The alternative — keep the payload a `Packet` and have each adapter translate it into work — is a
design change, not a convention change: the adapter interface here is `(payload, limit) ->
ExecutionReceipt`, which has nowhere to put the target contract, the accounts, or the value a packet
names. That work is not required to remove the false success, so it was not taken.

## What changed

| site | before | after |
|---|---|---|
| `AtlasKernel::verify_domain_payloads` (`lib.rs`) | `deserialize_packet` + domain-bit check | asks `T::EvmAdapter::validate` / `T::SvmAdapter::validate` |
| `submit_comit` / `submit_comit_v2` | two inlined copies of the packet check | both call `verify_domain_payloads` |
| `wasm_adapters::{WasmEvmAdapter,WasmSvmAdapter}::validate` | `validate_evm` / `interp_validate_program` only | also refuse a `Packet` by name |
| `runtime::native_vm_adapters::{NativeEvmAdapter,NativeSvmAdapter}::validate` | accepted anything non-empty / program-shaped | also refuse a `Packet` by name |
| `mock::{TestEvmAdapter,TestSvmAdapter}::validate` | `Ok(())` for everything | mirror the live validators (packet refusal + `validate_evm` / `interp_validate_program`) |
| `test_helpers::{wrap_evm_payload,wrap_svm_payload}` | built `Packet::Evm(Call)` / `Packet::Svm(Invoke)` | build EVM bytecode / an eBPF program |

The mock and the fixtures are part of the fix, not cleanup: a permissive mock is how the two halves
disagreed for two days while every kernel test passed.

## Measured

```text
cargo test -p pallet-x3-kernel                                  -> 225 passed; 0 failed
cargo clippy -p pallet-x3-kernel --all-targets -- -D warnings    -> clean
cargo fmt --all -- --check                                      -> clean
cargo check -p pallet-x3-kernel --features runtime-benchmarks    -> compiles
     (one `dead_code` warning for `wrap_x3_payload`, a function this change does not touch and
      whose only callers are `#[cfg(test)]` modules — pre-existing)
```

### The tests fail if the convention is reverted

`verify_domain_payloads` was temporarily put back to the retired rule (packet decode + domain bit)
and the module run:

```text
cargo test -p pallet-x3-kernel payload_convention
  -> 3 passed; 2 failed
     a_short_payload_is_bytecode_and_is_accepted         InvalidEvmPacket
     evm_bytecode_and_svm_program_payloads_are_accepted  InvalidEvmPacket
```

Both failures name the acceptance check, not an incidental panic. The rule was then restored and the
file is byte-identical to the version this commit carries (`cmp` against a pristine copy, and an
empty `git diff` for the path); the module is green again, 5 passed of 5.

### The fixtures are the chain's own artifacts

`wrap_evm_payload` is `PUSH1 0; PUSH1 0; RETURN` followed by the test's intent; `wrap_svm_payload` is
`MOV64_IMM r1, imm` per four bytes of intent, then `r0 = 0`, then `EXIT`. Each has a test that runs
the **production** validator over it (`x3_evm_integration::mini_evm::validate_evm` — what
`WasmEvmAdapter::validate` calls — and `x3_svm_integration::interp_validate_program`) and asserts
`!payload_is_packet`, so a fixture the live chain refuses, or one that has drifted back into being a
packet, fails its own precondition instead of passing quietly.

## What this does not prove, and what is still open

1. **The EVM/SVM arms are now honest, not functional.** A `Packet` that names a target contract or
   program is refused by name; nothing translates one into work. Those two domains are inert
   (loudly) until the translation is designed. The X3 arm is the one that executes end to end.
2. **`real_adapters::FrontierEvmAdapter` is still a false success.** `pallets/x3-kernel/src/adapters.rs`
   returns `ExecutionReceipt { success: true, .. }` from a gas figure computed off the payload length,
   without executing. Reachability, measured: nothing wires it — `runtime/src/lib.rs` selects
   `native_vm_adapters::NativeEvmAdapter` (1672, 1766) or, in the wasm build, `WasmEvmAdapter` — so it
   is dormant rather than live, but it is `pub use`d from the pallet and should be deleted or made to
   refuse.
3. **No live gas/accounting proof for the EVM and SVM budgets**, which is what started this thread:
   the X3 budget was proven on a chain on 2026-09-25, the other two still are not.
4. **The runtime WASM record.** These kernel sources are in the wasm blob, so the record at
   `docs/reports/runtime-wasm-hashes.json` has to move with the revision; that rebuild is a separate
   step (two from-scratch srtool builds) recorded in its own commit.
