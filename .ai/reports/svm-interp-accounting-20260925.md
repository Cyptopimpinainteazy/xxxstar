# The SVM interpreter over-charged every call above its fuel cap

2026-09-25. Third pass of the same technique — damage a parser with attacker-shaped input and see
what comes back — applied to the SVM interpreter `pallet-x3-kernel` reaches on chain. The previous
two passes found a gas limit that was never applied (`X3Executor`) and two readers that allocated
from an untrusted count (`mini_x3`, `x3-backend`). This one found a compute figure that was wrong
whenever the caller's limit was above the interpreter's own cap.

**Correction, added 2026-09-25 after the gate census.** An earlier draft of this report said the crate
had "no gate" and that the defects were found "within minutes of the suite being run for the first
time". Both halves are wrong. `x3-svm-integration` is a root workspace member, so `test workspace:`
in `GATES_DEEP` (`cargo test --workspace`) runs it; what it lacked was a *fast-set* gate. And the 33
existing tests passed with both defects present — they were found by the new tests in this file, not
by running the old ones. The distinction matters: a suite that runs and does not check the thing that
is broken is a different problem from a suite that never runs at all. The second is what
`scripts/check-crate-tests-are-gated.py` was written to find, and the census below is why it is
shipped as a measurement tool rather than a gate.

## What was checked first, and was already right

The interpreter's handling of untrusted bytes is careful, and worth recording as such:

* `elf_find_text` — the ELF64 section walk — uses `slice::get` and `checked_add` / `checked_mul`
  for every offset and count. An ELF whose `e_shoff`, `e_shnum` or a section's `sh_offset` points
  past the end answers `None` rather than panicking, and the loop over section headers is bounded
  by the `u16` count.
* `mem_read_bytes` / `mem_write_bytes` check `checked_add`ed ends against `STACK_SIZE` and the heap
  length before touching memory, and allocate only after those checks.
* `sol_memset_` charges fuel for `n` bytes **before** `vec![val; n]`, so the allocation is bounded
  by the fuel the program actually has.
* The `rbpf` path (`crates/svm-integration/src/rbpf.rs`) accumulates `compute_units_used` by
  `saturating_add` of what it charges, rather than deriving it from the limit.

## Defect: units used were derived from the wrong baseline

`execute_bpf` caps its internal fuel at `MAX_INSN_FUEL` (1,000,000):

```rust
let fuel = config.compute_unit_limit.min(MAX_INSN_FUEL);
```

and then reported usage against the **caller's** limit:

```rust
let compute_units = config.compute_unit_limit - vm.fuel;
```

Whenever `compute_unit_limit` is above the cap those are different numbers, and the gap counted as
if it had been burned. Reproduced before the fix:

```
two instructions under a 2000000 unit limit reported 1000002 units used
```

The default config (200,000) hides it; any caller above 1,000,000 hits it, and `WasmSvmAdapter`
passes the transaction's own compute limit straight through — so the pallet charges and records a
number the program never spent. Fixed by reporting `fuel - vm.fuel` against the fuel actually
granted.

## Defect: validation accepted a payload execution could not run

`validate_program` is the pallet's precondition for `execute_bpf`, and for an ELF it only asked
whether a `.text` section *exists*:

```rust
elf_find_text(payload).ok_or(SvmError::InvalidPayload)?;
return Ok(());
```

`execute_bpf` additionally requires that section to be non-empty and a whole number of 8-byte
instructions, so an ELF whose `.text` is empty or 9 bytes long validated and then failed at
execution — for a reason validation could have seen. Both now apply the same two checks, and
`a_text_section_that_is_not_whole_instructions_is_refused_by_both_paths` pins it.

This is the same shape as the `X3Executor` finding from two cycles ago: the validator and the
executor had different ideas about what the input must be.

## Coverage added

`crates/svm-integration/tests/interpreter_robustness.rs` — seven tests over a real two-instruction
program and a hand-built ELF64 that wraps it:

* the fixture validates, executes, and reports a handful of units rather than a limit-shaped number;
* a limit above the interpreter's cap does not inflate the units reported (1,000,000, 2,000,000 and
  50,000,000);
* an unterminated loop is stopped by the compute limit at three different limits;
* every truncation of a program is handled without panicking, through both entry points;
* every single-byte mutation at seven replacement values must not panic, and any mutant that
  executes must report units inside the limit it was given;
* an ELF-wrapped program runs, and damaging every byte of the ELF header and the section headers
  must not panic the parser;
* a `.text` section that is not whole instructions is refused by validation *and* execution.

Two of these failed on their first run because of mistakes in the fixtures rather than the
interpreter, and both are recorded in the file: the "never terminates" program had an `EXIT` before
its backward jump, so it returned immediately; and the hand-built ELF named its string-table
section `.text` as well, so the parser returned the 9-byte string table as the program. A test that
cannot fail for the right reason is not evidence.

## Runtime

`crates/svm-integration` is in `pallet-x3-kernel`'s graph, so the runtime WASM is re-attested with
this change.
