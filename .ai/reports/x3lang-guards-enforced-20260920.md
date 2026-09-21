# Guards: the ones the checks could not see, and the ones the VM did not judge — 2026-09-20

TICKET-027's remaining shape. Two defects, both measured on the corpus, both of the
same kind: a guard that reads as a constraint in the source and is neither checked
nor enforced.

## Defect 1 — `require_guards` read the top level only

`semantic::require_guards` walked `intent.body.stmts` and four declarations'
`requires` lists. It did not descend into `if` / `while` / `for` / `loop` /
`atomic` blocks, did not read a `fallback` block's own list, and did not look at
functions, agents, gpu blocks, simulate/task/subscription declarations, choice
paths or parallel legs. **Thirteen checks read it** (`grep require_guards`):
the risk-policy ceiling, route score, risk, canonical supply, solver bond,
relayer quorum, proof_complete, finality, invariants, and the rest.

Measured, before:

```
$ x3c build with_policy_99.x3      # risk_policy { max_slippage 50 }, fallback guard 99
x3c build: 404 bytes (101 ops)     # accepted
$ x3c build with_policy_1.x3       # the same program, policy 1, top-level guard 50
Semantic error: X3E4027: declaration 'rebalance_with_fallback' permits a slippage of 50
  while the risk policy accepts at most 1; the guard allows what the policy forbids
```

The same guard was refused at the top level and accepted one block down. After:

```
$ x3c build with_policy_99.x3
Semantic error: X3E4027: declaration 'rebalance_with_fallback' permits a slippage of 99
  while the risk policy accepts at most 50; the guard allows what the policy forbids
```

The walk is recursive now and covers every item with a statement body. Four tests
in `test_parallel_dag.rs` had been relying on the hole — their fixtures carry
`require finality.ethereum >= 12` inside a `leg` and declare no policy for
ethereum — and their fixtures now declare the depth the guard names.

Three tests pin it in `test_guard_declarations.rs`. With the fallback and leg arms
reverted, the two refusals fail and the control (a bound the policy permits) still
passes.

## Defect 2 — an economic guard was not enforced, and its bound did not travel

The spec's rule for these two kinds is explicit:

> Native profit guards — Transaction simply refuses settlement below target profit
> Native fee/slippage guards — Economic constraints enforced by the VM

The emitter recorded them as `REQUIRE static 0`: the executor treats a static guard
as satisfied, and the operand was zero rather than the bound.

```
$ x3c build rf_7.x3 -o rf_7.x3b       # require slippage <= 7
$ x3c build rf_99.x3 -o rf_99.x3b     # require slippage <= 99
$ cmp rf_7.x3b rf_99.x3b
IDENTICAL — the guard's bound is not in the artifact
```

Two programs with different ceilings, the same bytes, and neither tested. After:

```
$ x3c explain rf_7.x3b | grep REQUIRE
  0008  0x40  REQUIRE measured slippage 7
  0009  0x40  REQUIRE measured profit 0
  0013  0x40  REQUIRE measured slippage 50
  0014  0x40  REQUIRE static 0
$ cmp rf_7.x3b rf_99.x3b
differ — the bound travels now
$ x3c run rf_7.x3b
VM error: Panic("X3_GUARD_UNMEASURED: the guard `slippage <= 50bps` needs a slippage
  the host measured, and no host reported one for this trade")
$ x3c run rf_7.x3b --measured-slippage-bps 100
VM error: Panic("X3_SLIPPAGE_ABOVE_CEILING: the trade realised 100bps and the program
  allows at most 50bps")
$ x3c run rf_7.x3b --measured-slippage-bps 0 --measured-profit-bps 10
x3c run: ok
```

`slippage <= 7` in the example's `fallback` block did not even reach the artifact:
the `Statement::RouteFallback` arm destructured the guard list away with `..`. The
block's bounds lower as guards now, after the approved list.

### The design decision this makes

A program with one of these guards needs the market outcome stated to run. That is
the fail-closed direction and it is what the spec asks for, but it is user-visible:
the sweep's run step went from 19/20 to 8/20 with nothing stated, and back to
**19/20 when the outcome is stated** — the same 19 artifacts that ran before. The
CLI already had `--measured-profit-bps` / `--measured-slippage-bps` for plan floors
(TICKET-106), so the caller's half existed; it is now required for a program's own
guards too.

The guards left alone, deliberately: `slippage >= n` and `profit <= n`. Those are
the opposite direction from what the quantity means, they are refused where they
matter, and inverting the comparison at the emitter would enforce something the
program did not write.

Old artifacts are unaffected: a `static 0` guard still reads as satisfied, so
nothing already emitted changes meaning and no version bump is needed (the version
byte is a function of the opcode set, which did not change; `POLICY_VERSION` is
carried, not compared, and the economic *schema* did not change).

## The tests that changed, and why

Nine call sites in five files now state the outcome, because a measured guard
requires it: `test_control_flow_e2e.rs`, `test_e2e_fixtures.rs`,
`test_e2e_examples.rs`, `test_nonce_replay.rs`, `test_venue_settlement.rs`, and four
cases in `crates/x3-tools/tests/cli.rs`.

One of them asserted the old design in as many words — *"a program's own guard is a
compile-time constraint and must still run unmeasured"*. It now asserts both halves
of the new one: the measured run succeeds and the unmeasured run is refused by name.
That inversion is the change, not a test bent to fit it.

A second (`cli_branches_on_a_quantity_a_host_measured`) named the branch's bound in
its refusal. With the program's own floor now evaluated first and reading the same
quantity, an unmeasured profit is refused *at the guard* — the branch is
unreachable unmeasured, which is stronger than a branch that declines to decide —
so the assertion names the guard that actually refuses.

## Verification

```
cd x3-lang
CARGO_TARGET_DIR=/tmp/x3lang-target-merge /tmp/x3lang-cargo.sh test --workspace   # 1242 passed / 0 failed
… clippy --workspace --all-targets -- -D warnings                                # clean
… fmt --all -- --check                                                           # clean
pytest tests/ -q                                                                 # 23 passed
bash /tmp/x3probe/sweep2.sh <x3-lang> <x3c>
   # files=20 check=20 build=20 warning-free=20 run-with-a-stated-outcome=19 run-unmeasured=8
local-ci: no-float-in-consensus, invariant-registry, test-integrity-diff, readiness-consistency PASS
```

## Still open

- Every non-economic guard kind with a bound (`solver_bond >= 10_000`, `route_score >= 90`)
  still records threshold 0: the bound is checked at compile time and then dropped
  from the artifact, so a reader of the artifact cannot see what the program
  required. Carrying it needs a unit per kind (the flags' unit code means profit /
  delta / slippage basis points), which is a small design decision rather than a
  copy of this one.
- The economic quantities a program can *state* are profit and slippage; a fee
  ceiling (`risk { max_total_fee_bps }`) is still compile-time only.
