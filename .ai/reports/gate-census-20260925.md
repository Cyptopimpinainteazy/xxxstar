# The gate census: what has tests, and what no gate runs

2026-09-25. Last cycle's gate finding generalised: `check-registry-tests-are-gated.py` requires a gate
for every crate the readiness registry *cites*, and a crate nobody cites can still carry a suite. This
is the measurement of that wider question, the one defect it turned up, and — the reason this ships as
a tool rather than a gate — the four ways the measurement itself was wrong before it was believed.

## What the census says

```
root workspace:             194 crates, 183 with tests, 22 of those in the fast set
outside the root workspace: 31 crates with tests, 22 that this census cannot match to a gate
```

The first line is the one worth reading twice. **161 of the 183 root crates with tests are not in the
fast set.** They are not ungated: `test workspace:` in `GATES_DEEP` is `cargo test --workspace`, and
that runs them. The distinction is how long a push takes, not whether anything runs — and getting it
wrong is exactly the mistake this census was written to correct.

## Correction to the previous cycle

The SVM fix reported that `x3-svm-integration` had "no gate" and that the defects were found "within
minutes of the suite being run for the first time". Both halves are wrong:

* the crate is a root workspace member, so `cargo test --workspace` (the `test workspace` gate) runs
  it. What it lacked was a *fast-set* gate;
* the 33 existing tests passed with both defects present. They were found by the new tests, not by
  running the old ones.

A suite that runs and does not check the broken thing is a different problem from a suite that never
runs. The claim has been corrected in `scripts/local-ci.sh` and in
`.ai/reports/svm-interp-accounting-20260925.md`, where it was made.

## The defect: 69 tests compiled on every run and never executed

The `nested workspaces` gate read:

```
for d in crates/x3-swarm-core services/x3-swarm-api services/x3-swarm-worker services/x3-solvency-sidecar;
do … cargo check --locked --all-targets --manifest-path "$d/Cargo.toml" …
```

`cargo check --all-targets` **compiles a crate's tests and runs none of them.** Measured:
`x3-swarm-core` has 69 test attributes, all passing, and the whole suite takes about eight seconds. It
now has `test x3-swarm-core`, a real `cargo test` gate.

The other three stay on `check`, with the reason stated in the gate's comment: `services/x3-swarm-api`
and `services/x3-swarm-worker` have no tests at all, and `services/x3-solvency-sidecar`'s suite
contains `state::tests::record_fill_time_ema_after_window`, which was still running after 60 seconds
and kept the whole run going for minutes. That is a finding about the test, not a reason to skip the
crate — see the ticket below.

## Why this is a tool and not a gate

Deciding "is this crate gated" by reading shell was wrong four times:

1. `--all-features` matched a `--all\b` pattern, so the first version declared a workspace-wide test
   gate and every crate gated by construction;
2. a workspace-level `--manifest-path` gate (the form `test x3-lang` uses) names no `-p`, so the six
   `x3-lang` crates — ~1,300 tests — were reported ungated;
3. `nested workspaces` passes its manifest through a shell variable (`--manifest-path "$d/Cargo.toml"`),
   which a regex over the command text cannot follow;
4. `cargo check --all-targets` counts as a gate to any text-matching scheme while running nothing.

Each is fixable, and each fix moved the number without changing the tree. A checker that needs four
corrections before it is believed is not evidence, so `scripts/check-crate-tests-are-gated.py` is
shipped as a census with those four hazards written into its docstring, and `KNOWN_UNGATED` is left
empty rather than filled with guesses.

## Tickets

1. **Make the census sound.** Have `local-ci.sh` record the packages each gate actually tests — from
   `cargo metadata`, once, when the gate runs — instead of a consumer re-deriving it from the command
   text. Then "every crate with tests is run by some gate" becomes a checkable invariant, and the
   census becomes a gate.
2. **`services/x3-solvency-sidecar`: `state::tests::record_fill_time_ema_after_window` runs for
   minutes.** A test that slow cannot be in the fast set, so the crate's 14 tests stay unrun on every
   push. Either the test needs a fake clock or the crate needs a separate gate.
3. **The remaining 22 crates the census cannot match** to a gate, one at a time — the SVM programs
   under `X3-contracts/svm/programs/` and `programs/svm/` (`x3-core`, `x3-vm-erc20`,
   `x3-atomic-swap-solana`, `x3-receipt-verifier`, `x3-kernel-bridge`, `x3-external-gateway`), the two
   parity crates, `loom-concurrency`, the two fuzz workspaces, `x3-live-auditor`, `x3-regression-engine`
   and `svm-counter-test`. Each needs the same two questions the fast set answered for `x3-swarm-core`:
   does its suite pass, and how long does it take.

---

# Second pass, same day: the split, and what walking the list turned up

## The loops are gone

Both loop-shaped gates were replaced with one entry per workspace. `nested workspaces` became four
entries (`check x3-swarm-api`, `check x3-swarm-worker`, `check x3-solvency-sidecar`, `test x3-sidecar`)
and `js sdk tests` became twelve (eleven packages plus the pnpm `apps/x3-studio`). A loop hides the two
things a gate list is for: *which* workspace failed when it fails, and *what it covers* when a reader
asks. It is also the reason hazard 3 above existed at all — with no loops left, the census's gate
reading is sound for every gate in the file.

## Six gates added, each measured before it was added

| gate | tests | what it was |
| --- | --- | --- |
| `test x3-swarm-core` | 69 | `cargo check --all-targets` in the nested loop: compiled, never ran |
| `test x3 svm programs` | 12 + 11 + 15 + 21 + 20 across five packages | `test x3-htlc` selects one package out of `X3-contracts/svm`; these five were never run |
| `test svm atomic swap` | 10 | `programs/svm/x3_atomic_swap`, both packages of its workspace |
| `test x3 parity-core` | 6 | `X3-contracts/shared/parity-core` — the CPU/GPU agreement tests |
| `test x3 gpu-parity-core` | 7 | same, GPU side |
| `test x3-adapters` | 20 | `adapters/` had a workspace, a lock and a suite no gate named |

That is **191 test attributes** moved from "nothing runs them" to gated on every push, at a cost of
about a minute of wall clock (the five-package SVM gate is 46 s cold, the rest are seconds).

The census after the walk: **22 → 11 ungated** crates outside the root workspace.

## The eleven that remain, and why

* `tests/loom-concurrency` (9) — **has no committed `Cargo.lock`**, so `cargo test --locked` fails
  before it compiles anything.
* `apps/x3-desktop/src-tauri` (63) and `apps/inferstructor-dashboard/src-tauri` — **stale committed
  lockfiles**, same failure.
* `services/x3-solvency-sidecar` (14) — a test that runs for minutes
  (`state::tests::record_fill_time_ema_after_window`), which cannot go in a fast set as it stands.
* `x3-live-auditor` (2) and `x3-regression-engine` (1) — the `x3-autonomic-core` tree.
* `svm-counter-test` (1, twice), the two `pallet-*/fuzz` workspaces (1 each), and two patch-tree
  crates the census's vendored-path filter does not catch (`sc-allocator`, `sp-maybe-compressed-blob`).

## New measurement: 17 of 36 nested lockfiles are stale

`cargo metadata --locked` over every committed lockfile outside the vendored trees:

```
committed lockfiles: 36
of those, --locked FAILS: 17
  13  pallets/*/fuzz/Cargo.lock
   3  apps/*/src-tauri/Cargo.lock   (x3-desktop, inferstructor-dashboard, infra-structure/dashboard)
   1  launch-gates/sources/pack-04-invariant/integration-tests/svm-counter-test/Cargo.lock
```

plus `tests/loom-concurrency`, which has no lockfile at all. That is 18 of 36 nested workspaces whose
tests cannot be run with `--locked` as the tree stands — the same class this file recorded for the fuzz
trees two cycles ago, now measured across all of them rather than one at a time.

## Tickets

1. **Decide what the 15 fuzz and Tauri workspaces are for.** Either they are built and gated, or they
   are not, and the stale lockfiles go with them. Refreshing one cascades — measured earlier: the
   narrowest `cargo update` in a fuzz workspace moved 525 packages to 637 — which is why this is a
   decision, not a chore.
2. **`tests/loom-concurrency` needs a committed lockfile**, or the directory needs a reason to exist
   without one.
3. **`services/x3-solvency-sidecar`'s minutes-long test** needs a fake clock, or the crate needs a gate
   that is not the fast set.
4. **Walk the last five crates** (`x3-live-auditor`, `x3-regression-engine`, `svm-counter-test` ×2,
   `sc-allocator`) the same way this pass walked six: does the suite pass, how long does it take, then
   gate it.

