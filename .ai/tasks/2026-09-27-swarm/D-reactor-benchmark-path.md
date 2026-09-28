# Workstream D — X3 Reactor: a benchmark job that is submitted, run and published (P2)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read
`AGENTS.md` first, then this brief.

## What the row says today

`FEATURE_REGISTRY.toml` → `[x3_reactor]`: `readiness_score = 40`, crate `crates/x3-bench`, mode
`LIVE_TESTNET`, required tests `create_and_compare_reports` and `test_compile_optimized`, and three
blockers:

```text
Benchmark infrastructure not in CI critical path
GPU benchmark path depends on optional sidecar
No test covers a benchmark job submit/publish path: the previously cited
benchmark_job_submits / benchmark_result_publishes never existed anywhere in the repository.
```

The third blocker is the honest one and it is your job: there is no submission/publication path at
all, only report comparison. Do **not** satisfy it by adding two empty test functions named after the
missing ones — that is the exact failure this row is complaining about. Implement the path, then the
tests that drive it.

## Your files

`crates/x3-bench/**` and one new gate line in `scripts/local-ci.sh`. Do not edit `pallets/**`,
`runtime/**`, `crates/northern-swarm/**`, `crates/x3-swarm-core/**`, or `.maintain/**` — other lanes
own those right now, and a srtool re-attestation of the runtime is in flight.

## The work

1. **A benchmark job with a lifecycle**: submitted → queued/claimed → run → result published, with a
   typed record for each transition and a stable identity for the job. It must be able to run the
   crate's existing compile/report pipeline as its "run" step so this is a real path, not a stub.
2. **Publication is real**: a published result carries the inputs it was produced from (source
   revision, parameters, measurements) so a third party can tell two results apart and detect a
   stale or mismatched one. Refuse to publish a result whose inputs do not match the job.
3. **Deterministic scheduling/selection** where the row's block "GPU benchmark path depends on an
   optional sidecar" bites: when the accelerator is unavailable, the job must say so explicitly
   (`Unavailable`-style typed error) rather than silently reporting a CPU number as if it were a GPU
   one (`AGENTS.md` §18).
4. **Tests** that fail if the path is removed: happy path end to end; publish without a run; publish
   twice (idempotent or refused — pick one and pin it); publish a result whose inputs were changed
   (must be refused); accelerator unavailable (typed refusal, no silent CPU substitution).
5. **A CI gate** so the path is no longer outside CI: add a `test x3-bench` line to
   `scripts/local-ci.sh` in the fast set (follow the shape of the existing `test x3-swarm-core`
   entry), and make sure it is green.

## Rules

* `cargo test -p x3-bench --all-targets` passes; `cargo clippy -p x3-bench --all-targets -- -D warnings`
  clean for what you touch. Use `CARGO_TARGET_DIR=/tmp/x3-target-reactor`.
* No `unwrap()`/`expect()` on production paths (the panic ratchet counts them:
  `python3 scripts/audit/panic_unwrap_scan.py`). No `todo!()`, no `unimplemented!()`.
* Break-it-first: remove the input-binding check, watch the "changed inputs" test go red, restore,
  watch it go green. Record it.
* **Never `git add -A`.** Stage only your own paths. **Do not push**; report to `/root`.

Report to `/root`: commit hash, file list, exact commands and results, what the job/publish records
contain, and the break-it-first evidence.
