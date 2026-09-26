# Gas accounting, proven on the chain instead of in the adapter

Date: 2026-09-26
Scope: `node/tests/x3vm_live_lifecycle.rs`, matrix row `X3-LANG-004`
Directive: PRIORITY 1 — "prove gas accounting ... do not mark complete until the runtime path is exercised."

## What was missing

The live suite only ever ran programs that return immediately. Every receipt it checked reported
`gas_used > 0`, which shows gas is *metered*; nothing showed the chain *stops* a program that will not
finish inside its budget. The unit-level `test_gas_exhausted` covers the adapter in isolation, and the
matrix said so — but "the chain refuses an unbounded program" is a different claim, and an attacker
who could evade it would buy unbounded interpretation time per block.

The budget is the runtime's own constant, not something the submitter picks:

```
runtime/src/lib.rs:  pub const DefaultX3GasLimit: u64 = 6_000_000;
pallets/x3-kernel/src/lib.rs:  let x3_gas_limit = T::DefaultX3GasLimit::get();
                              T::X3Adapter::execute(tx, x3_gas_limit) -> Err => X3ExecutionFailed
pallets/x3-kernel/src/adapters.rs:  X3Executor::execute(..).map_err(|e| match e { GasExhausted {..} => "X3 out of gas", .. })
```

## The test

`a_program_past_the_gas_limit_is_refused_and_leaves_no_receipt` compiles

```x3
fn burn(n: i64) -> i64 { let mut i = 0; let mut total = 0;
                         while (i < n) { total = total + i; i = i + 1; } return total; }
fn main() -> i64 { return burn(3000000); }
```

submits it live, and asserts four things:

1. the extrinsic **is** included in a finalized block (it is signed and well-formed — inclusion is not
   success, which is the point);
2. the dispatch failed, and the reason is the kernel's own variant — `X3ExecutionFailed`, computed in
   the test from `pallet_x3_kernel::Error::<Runtime>::X3ExecutionFailed.encode()` rather than pinned
   as a byte, because the runtime reports `ModuleError { index: 11, error: [variant, 0, 0, 0],
   message: None }` with no name;
3. no receipt was stored for that comit (`X3ExecutionReceipts` is written only after every acceptance
   check);
4. a cheap program submitted afterwards still succeeds and gets its receipt, so a refusal that broke
   the path would fail the test.

The loop is sized so it *cannot* finish inside the budget — the interpreter stops at the limit, so a
larger iteration count costs no extra time and removes any doubt about the intent.

## Evidence

```
$ cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored --exact \
      a_program_past_the_gas_limit_is_refused_and_leaves_no_receipt
test result: ok. 1 passed; 0 failed; ... finished in 43.50s

$ bash scripts/local-ci.sh --cross --only x3-native-lifecycles
PASS X3-native lifecycles   366s
  test result: ok. 7 passed; 0 failed; ... finished in 363.06s

$ bash scripts/local-ci.sh --jobs 4
87 gates, 0 failures   (no gate was added this turn)
```

Measured on the way: the first version asserted the error text contained `X3ExecutionFailed`. It does
not — the runtime's metadata carries no message, so the test would have been asserting something that
is never true. The refusal is now checked against the encoded variant, which is what the chain
actually reports.

## Remaining

* The other half of the gas story is not live-proven: a *static* budget excess is refused by the
  verifier (`GasBudgetExceeded` in `crates/x3-vm/src/verifier.rs::verify_gas`), which runs inside
  `validate` before execution. Only the dynamic path is covered here.
* Cross-VM gas (`DefaultEvmGasLimit`, `DefaultSvmComputeLimit`) has its own budget constants and the
  same question open on the live path.
* PRIORITY 2's public-testnet half remains the standing external blocker.
