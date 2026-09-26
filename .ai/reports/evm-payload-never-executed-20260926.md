# The EVM arm of the triple-VM submit path reports success without executing

Date: 2026-09-26
Scope: `pallets/x3-kernel/src/lib.rs::submit_comit_v2`, `pallets/x3-kernel/src/wasm_adapters.rs`
Matrix: `X3-LANG-004` (blocker added)
Severity: false success with a persisted receipt — the shape AGENTS.md forbids.

## How it was found

The previous turn proved the X3 gas budget on a live chain. The obvious next step was the same proof
for the EVM and SVM budgets, since the kernel passes `DefaultEvmGasLimit` and
`DefaultSvmComputeLimit` the same way. Building that test meant answering "what is an EVM payload?",
and the two halves of the answer disagree.

## The two halves

```
submit_comit_v2                    pallets/x3-kernel/src/lib.rs
  if !evm_payload.is_empty() {
      match deserialize_packet(&evm_payload) {      // SCALE-encoded Packet
          Ok(packet) => { let mask = get_domain_mask(&packet); ensure EVM bit .. }
          Err(_) => Err(InvalidEvmPacket)
      }
  }

the adapter the wasm runtime uses  pallets/x3-kernel/src/wasm_adapters.rs
  WasmEvmAdapter::execute(payload, gas_limit)
      -> x3_evm_integration::mini_evm::execute_evm(payload, ..)   // payload AS EVM CODE
```

`runtime/src/lib.rs` selects `WasmEvmAdapter` for every build that is not `std+frontier`, so this is
the adapter a live chain runs. The kernel's expected payload is a semantic operation; the adapter's
expected payload is bytecode.

## Measured, not inferred

A temporary probe in `wasm_adapters.rs` (since replaced by the ignored test below) ran the real
adapter over the real fixture — `crate::test_helpers::wrap_evm_payload(&[0xAA; 64])`, which is exactly
what the pallet's own benchmark submits:

```text
PROBE evm payload bytes = 124
PROBE evm head = [00, 00, 6b, cb, 44, 6f, 34, 8c]
PROBE evm execute = Ok((true, 22576))     <- success, gas charged, nothing executed
PROBE evm validate = Ok(())
PROBE svm execute = Err(Other("SVM execution failed"))
PROBE x3 execute = Ok((true, 3))          <- the X3 arm really executes
```

The first byte of a SCALE-encoded `Packet::Evm(..)` is the enum discriminant `0x00` — EVM `STOP`. The
interpreter halts immediately, reports success, and charges base gas. The kernel then persists a
receipt for it.

So the EVM arm does not merely fail like the SVM arm (which fails closed and was already recorded);
it **succeeds without doing anything**, and the receipt says so. A caller cannot tell an executed EVM
operation from one that was never attempted.

## Why nothing caught it

* `bench_submit_comit_v2` passes, because the pallet's mock adapters accept anything; the earlier note
  on this row recorded that real adapters fail the payloads, but not that the EVM one *passes* them
  while the SVM one fails.
* The live cross-domain EVM gate passes, because it goes through the atomic-swap transport and the
  settlement engine, not through `submit_comit_v2`'s EVM arm.
* No test asserts that an accepted EVM payload produced an effect.

## Regression test (KNOWN-RED, deliberately)

`pallets/x3-kernel/src/wasm_adapters.rs`:
`accepted_evm_payload_must_execute::an_accepted_evm_payload_is_executed_or_refused_never_reported_as_success`

It is `#[ignore]`d so the crate's suite stays green, and it **fails by design** when run:

```
$ cargo test -p pallet-x3-kernel --lib an_accepted_evm_payload_is_executed_or_refused -- --ignored
panicked at pallets/x3-kernel/src/wasm_adapters.rs:
the adapter reported success for a payload it cannot have executed: the packet starts with its enum
discriminant (0x00), which is EVM STOP
```

**Acceptance criterion for the fix: that test passes when un-ignored.**

## What the fix has to decide

The X3 arm was resolved on 2026-09-25 by making the payload the thing the adapter executes ("the
payload is the program, and validation is the adapter's own"). Applying the same rule to EVM/SVM
means the kernel stops requiring a `Packet` for these two payloads and validates with
`T::EvmAdapter::validate` / `T::SvmAdapter::validate` instead. That is a convention change with
ripple — kernel tests that submit packets, the benchmark fixtures, and the routing helpers that
consume `Packet` — which is why it is recorded here with evidence rather than rushed.

The alternative — leaving the payload a packet and having the adapter build and execute the call it
describes — is a larger implementation and needs its own design.

Either way the invariant is the same and is what the test encodes: *a payload the kernel accepts must
be executed or refused, never reported as a success.*

## Remaining

* SVM arm: fails closed today (`SVM execution failed`), so it is non-functional rather than unsafe;
  it needs the same convention decision.
* The static gas-budget refusal (`GasBudgetExceeded` in the verifier) is still not live-proven.
* PRIORITY 2's public-testnet half remains the standing external blocker.
