# x3-mc — probabilistic adversarial simulation engine

`x3-mc` samples scenarios from parameter distributions, executes them, checks
invariants, and hands back a seed for anything it breaks. It is the search layer
over `x3-sim`, which is itself a harness over the **real** coordinator.

```text
distributions -> run seed -> sample -> EXECUTE -> invariants -> failure?
                                                      |            |
                                                     no           yes
                                                      |            |
                                                   next run    save seed
                                                                  |
                                                            replay + minimize
                                                                  |
                                                           regression test
```

## What it is not

Monte Carlo is **not** a replacement for proof, deterministic tests, fuzzing,
state-machine fuzzing, mutation testing, formal verification or real validator
testing. It is an extra adversarial search layer. A campaign that completes a
million runs has sampled a million scenarios from the distributions it was
given:

* it does **not** show that a failure is impossible;
* it **does not** speak for a distribution it never sampled;
* `observed failure rate` is a measurement, not `true failure probability`.

`summary.json` in every evidence bundle says this in machine-readable form,
alongside the list of axes the campaign does not cover.

## Architecture: choosing versus executing

```
MONTE CARLO GENERATOR   (this crate: distributions, seeds, minimization)
        v
X3-SIM                  (virtual clock, lossy network, fault plan)
        v
DETERMINISTIC EXECUTION (SwapCoordinator — the real state machine)
        v
INVARIANT CHECKER       (coordinator violations + campaign assertions)
```

The engine contributes randomness and bookkeeping; it never contributes
semantics. `scenarios::AtomicSettlement` calls `x3_sim::run_with`, whose every
state transition comes from `x3_cross_vm_coordinator::SwapCoordinator`.

## Running

```bash
cargo run --manifest-path crates/x3-mc/Cargo.toml -- \
    run crates/x3-mc/campaigns/mc-x3-atomic-001.json --runs 2000 --out /tmp/mc
```

| Command | Purpose |
| --- | --- |
| `x3-mc run <campaign.json>` | Execute a campaign. `--runs`, `--seed`, `--out`, `--no-minimize`, `--json` |
| `x3-mc replay <campaign.json> --index N` | Re-execute one run by index. `--master-seed` pins the campaign seed |
| `x3-mc minimize <campaign.json> --index N` | Shrink a failing run to its smallest reproducer |
| `x3-mc report <directory>` | Summarize an evidence bundle |

Exit codes: `0` every invariant held, `1` a violation was found, `2` usage or
environment error.

## Determinism

Run `i` uses `derive_seed(campaign_seed, i)`, not the `i`-th draw of a shared
generator. Consequences, all of which are tested:

* run 900,000 replays without replaying the 899,999 before it;
* `--runs 1000` and `--runs 10000` agree on runs 0..1000;
* adding a worker cannot renumber anything, which is what makes a distributed
  farm a scaling change rather than a correctness change.

The engine, the RNG and every distribution are free of wall-clock reads. `seed:
auto` resolves through entropy supplied by the CLI and the resolved value is
written into `campaign.json` in the bundle, so an auto-seeded run is still
replayable afterwards.

## Campaign format

A campaign is JSON. Variable order in the file does not matter: the draw order
is the sorted key order, so reformatting a file cannot silently change what a
seed produces.

```json
{
  "campaign": { "name": "MC-X3-ATOMIC-001", "runs": 2000, "seed": "auto" },
  "variables": {
    "partition_events": { "distribution": "discrete", "values": [0, 1, 2, 3, 4] },
    "packet_loss":      { "distribution": "uniform", "min": 0, "max": 30 },
    "tx_rate":          { "distribution": "log_uniform", "min": 10, "max": 50000 },
    "restart":          { "distribution": "bernoulli", "p": 0.05 },
    "latency_ms":       { "distribution": "log_normal", "mean": 4, "std_dev": 1,
                          "min": 1, "max": 1500 },
    "traffic_mix":      { "distribution": "weighted_categorical",
                          "values": [1, 2, 3], "weights": [3, 1, 1] }
  }
}
```

Supported: `constant`, `uniform`, `normal`, `log_normal`, `log_uniform`,
`bernoulli`, `categorical` (alias `discrete`), `weighted_categorical`
(alias `weighted`), `poisson`, `integer` (alias `bounded_integer`),
`empirical`.

Every distribution is bounded. An unbounded normal can propose a negative
validator count, and a scenario that has to defend against its own sampler is
testing the sampler. Truncation bounds are optional; a degenerate input
produces a finite value rather than a panic.

YAML is not supported yet. The schema is the one above; adding a YAML front end
is a parser dependency, not a design change.

## What the atomic campaign varies

`MC-X3-ATOMIC-001` maps its variables onto the seams `x3-sim` actually has:

| Axis | Variable |
| --- | --- |
| Network delay, jitter, loss | `latency_ms`, `jitter_ms`, `drop_percent` |
| Duplicate delivery and replayed messages | `duplicate_percent` |
| Partitions and recovery windows | `partition_events`, `slow_link_events` |
| Node restart timing and duration | `crash_events`, `crash_duration_ms` |
| A persisted write disappearing | `stale_write_events` |
| Lock / claim / refund interleaving | `clock_step_ms`, `clock_jitter_ms` |
| Topology and load | `nodes`, `sessions`, `steps` |

Invariants come from the coordinator's own checker — `CLAIM_REFUND_MIX`,
`DOUBLE_SETTLE`, `REFUND_AFTER_CLAIM` and the rest — plus three campaign-level
assertions:

* `NO_PROGRESS_BUT_TRAFFIC` — messages were delivered and nothing was accepted,
  which means the harness stopped reaching the code under test and a clean
  sweep would be meaningless;
* `FAULT_WITHOUT_EFFECT` — a lost write fired in a run where nothing else
  happened;
* `NONDETERMINISTIC_REPLAY` — re-running an identical point produced a
  different state digest. Sampled at `verify_determinism` rather than run
  always, because it doubles the cost of the runs it covers.

**Not covered by this campaign, and not claimed:** proof timing and freshness,
per-VM execution latency, timeouts as a separate axis, RPC failure as distinct
from message loss, consensus across validators, storage durability beyond one
lost write, and economics. Each needs a seam in the simulator first; naming a
variable in a campaign file is not the same as testing the thing.

## Evidence bundle

```text
campaign.json     the campaign as executed, with the resolved seed
environment.json  tool version and target
summary.json      counts, coverage, interval, interpretation, not-covered list
failures/         one file per failing run, minimized reproducer included
seeds.txt         replayable seed database for the campaign
```

## The control problem

A campaign that reports zero failures is only worth something if the same
engine would have reported a failure had there been one. `tests/engine.rs`
holds that control: a scenario with a known broken region, so detection,
seeding, replay, clustering and minimization are checked against a defect that
is definitely present. `crates/x3-sim/tests/params.rs` pins the fault plans the
fixed scenarios produced before they were parameterized, so a refactor cannot
quietly re-tune every seeded run.

If you add a campaign and it has never once failed, that is not yet evidence of
anything. Break something on purpose and confirm it is caught.

## Gotcha worth knowing about

Enums deserialized from JSON here are hand-written rather than
`#[serde(tag = ...)]` or `#[serde(untagged)]`. The Substrate crates pulled in
through `x3-cross-vm-bridge` enable `serde_json`'s `arbitrary_precision`
feature, and under it serde's *buffered* representation of a JSON number is a
map rather than a scalar. The derived internally-tagged path therefore fails on
every `f64` field with `invalid type: map, expected f64` while `i64` fields
load fine. Deserializing through `serde_json::Value` avoids the buffered form
entirely. Any new enum in this crate that carries floats needs the same
treatment.

## Test suite

```bash
cargo test --manifest-path crates/x3-mc/Cargo.toml
```

## Scope

v0 ships the engine and one campaign, `MC-X3-ATOMIC-001`, against the real
coordinator. The next campaigns — consensus/network and performance — need
seams that do not exist yet: the simulator does not model consensus across
validators, and there is no load harness wired in. Building those campaigns
before their seams exist would produce run counts with nothing behind them.
