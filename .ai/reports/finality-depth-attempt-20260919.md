# `finality` guard kind: an attempt, reverted unpushed — 2026-09-19

## Why this file exists

I started the last unbacked guard kind with corpus weight (nine examples, twelve
guards) and **reverted it**, because two suites went red and my remaining context was
not enough to diagnose them properly. Nothing was pushed: `origin/master` is at
`52f79a4b9`, and the tree is green (72 suites, 800 passed; corpus 17/23, 16/23
warning-free against the control's 16/22, 15/22).

If you pick this up, start from the design below rather than from scratch, and expect
these two failures.

## The design (it compiled and the corpus sweep passed)

1. `FinalityPolicy` gains `blocks: Option<u32>`; the parser accepts `blocks <n>` **before**
   the terse `<chain> require <mode>` arm (or `blocks 32` is read as a chain named
   `blocks`); the formatter writes it back.
2. `semantic::verify_finality_guards_declared` decides both guard shapes:
   - a depth claim (`require finality.X >= N`) needs the declared depth for X to be at
     least N — a guard *below* it settles before the program's own policy says the chain
     is safe, which is the cross-chain loss the requirement exists to prevent; a guard
     above it is stricter and fine;
   - a mode claim (`require finality.sol == finalized`) needs the declared `requirement`
     to be that word;
   - a chain with no `finality_policy` is refused, naming the clause to add.
3. Chain names compare case-insensitively: eight of the corpus's twelve guards spell the
   chain differently from the way a declaration does (`finality Ethereum`).
4. The nine examples declare what their guards already require: for each chain a guard
   names, a `finality_policy strict { chain <as-the-guard-spells-it> requirement finalized
   blocks <the guard's own number> }`. Generated from the guards, so no number is
   invented — the guard states it. Two of the twelve are mode claims (`== finalized`) and
   get the requirement clause alone. Inserted before the first top-level item, as the
   route-score declarations were.

Chunk 1 + 2 compiled; with chunk 3 the corpus sweep was **17/23 check, 17/23 build,
16/23 warning-free, 17/23 run** — i.e. the check passed for all nine examples, and
`test_guard_declarations.rs` was 16 tests green (including an invariant that every
example containing `require finality` also declares `blocks`).

## The two failures, unverified as to cause

With all three chunks in the tree:

    compiler/tests/b52_test.rs
      disassembly_lists_every_emitted_instruction  left: 6, right: 31
      parse_roundtrip_ir_bytecode                  "disassembly should contain BRIDGE"

    python -m pytest  ->  13 failed, 1 passed      (it is 14 passed without the changes)

Both pass again on the reverted tree, so they are caused by the change and not
pre-existing. Most proximate explanation, **not verified**:

- the **Python harness** (`x3-lang/tests/*.py`) has its own reader of the examples and of
  the `finality_policy` shape; a new `blocks` clause (and nine edited examples) would need
  it. This is the more serious of the two: a language change that the repository's own
  second implementation cannot read is not finished.
- the **b52 disassembly test** counts trace lines against IR operations, and the nine
  examples' added policies change those counts for the fixture it reads
  (`examples/flagship_b52.x3`). A count that collapses to 6 rather than shifting by a few
  suggests something else is going on — worth reading that test before changing the
  examples again.

## Suggested next attempt

1. Change the language first (`blocks`, the check) with **no example edits**, and run
   `cargo test --workspace` plus the Python suite: they should both stay green, and the
   corpus will drop (the nine guards become unbacked) — that drop is the measure of what
   the example edits are for.
2. Teach the Python harness the clause (or establish that it does not read the examples'
   `finality_policy` at all), then edit the examples and re-run everything.
3. Only then remove `finality` from TICKET-049's remaining list.

---

# Second attempt, same outcome — and the real blocker, 2026-09-19

Applied the **language only** first (AST `blocks`, parser, formatter, check), as this
note suggested, then the example declarations, then the harness fix. Result:

- **`python -m pytest` → 14 passed** with the language change *and* every example edit.
  The suite's 13 failures last time were **not** the language: the harness's `cli.py`
  required the file's *first* line to be `intent <name> {`, and a `finality_policy`
  block ahead of the intent broke it. Fixed properly in `cli.py`: a small
  `_SKIPPED_DECLARATIONS` set of top-level declarations whose bodies the reader steps
  over, with anything else still an error. That is a real improvement — the legacy
  reader can now read a program that declares something before its intent.
- `b52_test`'s `test_check_mode_mainnet_rejects_unsafe_intent` and
  `test_conformance`'s `valid_internal_swap` were **fixtures** whose finality guards
  became unbacked: both are programs, and both now declare what they require (the
  conformance one through the example it includes).
- **The blocker**: `compiler/src/emitter.rs`'s
  `emitter::tests::a_fixed_frame_operator_with_a_non_zero_operand_does_not_truncate_the_walk`
  and the two `b52_test` disassembly tests fail once a program's stream starts with a
  guard. Changing the disassembler's fixed-frame advance from `pc + 4` to
  `align4(pc + 3)` — the rule TICKET-053 gave the verifier — fixes the b52 tests and
  breaks the emitter test, whose fixture places the instruction after a fixed frame at
  `pc + 4`.

**So the repository holds two contradictory statements about one byte layout**, and I
could not settle which is right in the room I had. Either:

- the emitter writes fixed frames **contiguously** (4 bytes each, `pc + 4`), in which
  case TICKET-053's `align4(pc + 3)` in the verifier is wrong and only appeared to fix
  things — it made two probes run that happened to be padded — or
- the emitter pads **every** instruction to the next absolute multiple of four (which
  is what `align4(pc + 3)` implies), in which case the emitter test's fixture is not
  shaped like emitted bytecode and needs to move.

Which it is can be read off `emit_operation` and `pad_to_4` directly — that is the next
step, and it is a *format* question that should be settled with the emitter's own code
plus a round-trip test (`emit` → `disassemble` → every operation appears exactly once),
not by fitting either rule to whichever tests are failing.

Both attempts are reverted unpushed: `origin/master` is `52f79a4b9` and the tree is
green (72 suites, 800 passed; python 14; corpus 17/23, 16/23 warning-free).
