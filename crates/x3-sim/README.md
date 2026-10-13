# x3-sim — deterministic fault-injection simulator

`x3-sim` runs the **real** cross-VM atomic swap coordinator
(`crates/cross-vm-coordinator`) against a virtual clock, a seeded RNG, a
lossy/partitionable network and a scheduled fault plan. A run is pinned by its
seed, and a failing run prints the exact command that reproduces it.

```bash
cargo run --manifest-path crates/x3-sim/Cargo.toml -- --seed 948218671 --scenario partition-storm
```

It is its own cargo workspace root, for the same reason the coordinator is: it
path-depends on a crate the parent Substrate workspace excludes.

## Why it exists

The coordinator passes 117 unit tests that call one operation at a time. That
cannot express the failures that actually hurt: a claim racing a refund across
a partition, a duplicate delivery after a restart, a persisted write that
disappears. `x3-sim` schedules those orderings deterministically instead of
hoping a human thinks of them.

It does **not** re-implement any transition. If it did, a passing run would only
prove the simulator agrees with itself. Every state change comes from the
coordinator crate; `x3-sim` chooses the order of calls and judges the resulting
state.

## Scenarios

| Scenario | What it schedules |
| --- | --- |
| `happy-path` | Every session walks the lifecycle on an ordered, lossless network. The positive control. |
| `claim-refund-race` | Claims and refunds scheduled against each other, plus a partition. |
| `partition-storm` | Repeated partitions, a slow link and message loss. |
| `crash-recovery` | Two crash-and-restarts from persistence, plus a lost write. |

## Options

| Flag | Meaning |
| --- | --- |
| `--seed <u64>` | Seed for the run. Replay uses the same value. |
| `--scenario <name>` | One of the scenarios above. |
| `--sessions <n>` | Atomic swap sessions in the ledger (default 8). |
| `--steps <n>` | Scheduler steps (default 120). |
| `--nodes <n>` | Node 0 is the coordinator, the rest are clients (default 4). |
| `--out <dir>` | Write an evidence bundle (JSON + trace) to this directory. |
| `--packet <dir>` | Write the failure packet (JSON + Markdown) here when a run fails. |
| `--hunt <n>` | Run `n` consecutive seeds from `--seed`; report every distinct failure. |
| `--max-failures <n>` | With `--hunt`, stop collecting after `n` failures (default 5). |
| `--no-minimize` | Do not minimize a failing config automatically. |
| `--root-cause` | Dispatch every written packet to the root-cause agent. |
| `--json` | Print one JSON document: evidence for a run, hunt summary for `--hunt`, the packet for a failure. |

Exit codes: `0` all invariants held, `1` a violation was found, `2` usage,
I/O, evidence- or packet-writing error. `root_cause.py` has its own codes: `2`
an unreachable router (nothing stored), `3` an answer that is not the required
JSON contract (the raw answer is kept).

## Failure packets

A failing run does not just print `FAIL`. It produces a **failure packet**:

```text
seed, scenario, config        what to re-run
invariant, session, detail    what broke
first_bad_step / first_bad_op which step introduced it
state_before / state_after    the violating session around that step
active_faults                 partitions/restarts/loss that had fired
suspected_code                coordinator symbols the invariant implicates
all_violations                every invariant the run broke, not just the first
trace_digest / state_digest   fingerprints of the exact run
replay_command                the one-line reproduction
minimized                     the smallest verified reproducer
required_regression_test      the exact test that must exist after the fix
branch / worktree_dirty       where the checkout stood; a dirty packet's commit
                              no longer describes the whole tree, and
                              worktree_dirty is null when git cannot say
```

```bash
cargo run --manifest-path crates/x3-sim/Cargo.toml -- --seed 948218671 \
    --scenario partition-storm --packet /tmp/x3-packets
```

Every field is derived from the run. The commit is `X3_COMMIT` when the caller
sets it, otherwise the checkout's `HEAD`, otherwise `unknown` — it never
guesses. The branch and worktree dirtiness are recorded next to it, because a
failure from uncommitted code is still real evidence but the commit hash alone
would overstate what it pins.

## Automatic minimization

A failing run of 200 sessions and 5,000 steps is a mystery; the same violation
in 1 session and 9 steps is a bug report. When a run fails, the minimizer
shrinks `sessions`, `steps` and `nodes` — keeping a candidate only while the
**same invariant** still fires — and re-runs the smallest candidate once more
to verify it before reporting `verified: true`. The search is bounded
(`DEFAULT_MAX_RUNS`, 160 runs) and deterministic: the same seed minimises to
the same config. `--no-minimize` turns it off.

The scan is honest about what it can prove. Every candidate value changes the
seeded schedule, so the failure predicate is not monotone: a config that fails
at a large value says nothing about smaller ones. Instead of a binary search
that could walk past a smaller reproducer, each dimension is scanned upwards
from its floor for a bounded number of runs and only observed results are
trusted. Passes repeat until a whole pass changes nothing, so a shrink in one
dimension still gets the chance to unlock a smaller value in another. The
result is the smallest reproducer *found*, verified by a fresh run — not a
claim of global minimality.

## Hunting

```bash
cargo run --manifest-path crates/x3-sim/Cargo.toml -- --hunt 500 \
    --scenario crash-recovery --packet /tmp/x3-packets
```

Runs 500 seeds, writes one packet per **distinct** failure — the same violation
found again is counted as a duplicate, not a second packet, and does not
consume `--max-failures` — and exits `1` when anything failed, so it drops
straight into CI. With `--out`, each failing run also gets its evidence bundle
(JSON + trace) and the hunt writes a `hunt-summary-<scenario>-<seed>.json`
recording how many seeds ran, how many passed, how many were duplicates, and
each failure's invariant and replay command. `--json` prints that summary as
one JSON document instead of the human lines.

## Root-cause dispatch

A packet is written to be handed to an investigation agent:

```bash
python3 crates/x3-sim/scripts/root_cause.py /tmp/x3-packets/x3-failure-<id>-<invariant>.json \
    --out reports/root-cause
```

`root_cause.py` takes exactly one packet per invocation; a hunt that writes
several packets gets one dispatch (and one stored answer) per packet.

`--root-cause` runs that dispatcher for you as part of the failing run (it
writes the packet to `target/x3-failure-packets` when `--packet` is not given);
dispatch failing never hides the violation, it just leaves the packet on disk.

The dispatcher posts the packet to the X3 AI router (`/v1/chat/completions`,
default `http://127.0.0.1:11435`), requires a JSON answer of the shape
`{"causes":[{symbol, file, confidence, reasoning}], "first_experiment", ...}`,
and stores it with provenance (model, request id, token usage) next to the
packet reference. It fails closed: an unreachable router is exit 2 and stores
nothing; a non-conforming answer is exit 3 and is kept as raw text, never
promoted into a "cause". Causes are ranked hypotheses to verify, not verdicts.

## Gate failures in other subsystems

The atomic kernel, settlement engine, supply ledger and runtime fail as *tests*,
not as simulated invariants. The same packet shape is produced for them by the
repository-level wrapper:

```bash
scripts/x3-failure-packet.sh --label pallet-x3-supply-ledger --root-cause -- \
    cargo test -p pallet-x3-supply-ledger
```

It runs the gate, and on failure extracts the first error, the failing tests,
the `file:line` each panic points at, the commit, the measured duration, and
the replay command (prefixed with `cd <checkout> &&`, so the replay runs
against the same source) into `failure-packets/<utc>-<label>-<id>.json`
(+ `.md`). A passing gate writes nothing, a usage error is exit `2`, and a
broken packet builder never replaces the gate's own exit code.

## Invariants checked

After every step, and across sessions:

| Code | Meaning |
| --- | --- |
| `CLAIM_REFUND_MIX` | One leg claimed while the other was refunded. |
| `COMPLETE_WITHOUT_BOTH_CLAIMS` | Phase `Complete` without both legs claimed. |
| `REFUNDED_WITH_A_CLAIM` | Phase `Refunded` with a claimed leg still recorded. |
| `PHASE_WITHOUT_FAST_HTLC` | A phase past locking with no fast-chain record. |
| `TIMELOCK_ORDER_INVERTED` | Slow-chain timelock not after the fast one. |
| `DUPLICATE_JOURNAL_ENTRY` | One semantic operation recorded twice with the same evidence. |
| `REFUND_AFTER_CLAIM` | The durable journal records a claim and then a refund. |
| `DOUBLE_SETTLE` | One hash lock completed two swaps. |

`REFUND_AFTER_CLAIM` reads the operation journal rather than the leg statuses,
because `record_refunds` overwrites the statuses — the journal is the only place
the earlier claim survives.

## What it found

`SwapCoordinator::abort` set `phase = Aborting` without consulting the transition
table every other mutator uses. A swap that had already completed (both legs
`CLAIMED`) could be aborted and then refunded, paying both sides twice. The
reproduction lives in `tests/refund_after_claim.rs`; the fix is in
`crates/cross-vm-coordinator/src/state_machine.rs`, with the regression test next
to it.

## Test suite

```bash
cargo test --manifest-path crates/x3-sim/Cargo.toml
python3 crates/x3-sim/scripts/test_root_cause.py
bash scripts/test_x3_failure_packet.sh
```

* `tests/determinism.rs` — the same seed reproduces the same trace and state;
  the seed really does change the schedule; the happy path completes.
* `tests/invariants.rs` — the checker is not vacuous: each invariant fires on a
  hand-built state that a bug would produce.
* `tests/refund_after_claim.rs` — the reproduced defect, plus a positive control
  that a legitimate refund still works.
* `tests/packet.rs` — a passing run produces no packet; the minimizer shrinks
  real runs and verifies its result; the packet's replay command pins every
  dimension and the CLI reach the same trace digest and verdict.
* `tests/minimize.rs` — `--hunt` over clean seeds exits 0 and writes nothing;
  `--json` hunts print one parseable document; usage errors are exit 2.
* `scripts/test_root_cause.py` — the dispatcher's prompt, fail-closed behaviour
  on an unreachable router, bad answers (HTTP error, non-JSON, malformed
  envelope) kept as raw text, contract validation, and provenance storage.
* `scripts/test_x3_failure_packet.sh` — a failing gate produces an accurate
  packet; a passing gate produces none; a broken packet builder preserves the
  gate's exit code.

## Scope

v0 simulates the coordinator lifecycle. It does not yet simulate consensus
across validators, EVM/SVM execution, or the full runtime — those need their own
harnesses, and the simulator's seams (`VirtualNetwork`, `FaultPlan`,
`invariants`) are where they plug in.
