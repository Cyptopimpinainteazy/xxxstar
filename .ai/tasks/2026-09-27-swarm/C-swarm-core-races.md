# Workstream C — X3 Swarm Core: the concurrency and partition evidence (P1)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read
`AGENTS.md` first, then this brief. This lane is **crate-local and test-only**; it exists because the
row's second blocker is `No multi-agent race-condition or partition-tolerance testing`.

## What the row says today

`FEATURE_REGISTRY.toml` → `[x3_swarm_core]`: `readiness_score = 25`, crate
`crates/x3-swarm-core`, six required tests (all real, in
`crates/x3-swarm-core/tests/agent_lifecycle.rs`), and two blockers:

1. the crate is excluded from the root workspace and absent from the runtime dependency graph, so the
   swarm control plane is not on the deployed blockchain path — **not your job** (the primary agent
   takes the workspace/runtime wiring, because it is a runtime-graph change and a srtool
   re-attestation is running right now);
2. **no multi-agent race-condition or partition-tolerance testing — that is your job.**

## Your files

`crates/x3-swarm-core/**` only. `crates/x3-swarm-core` is a **nested workspace of its own** (like
`x3-lang`), so build with `--manifest-path crates/x3-swarm-core/Cargo.toml`. Do not edit
`pallets/**`, `runtime/**`, the root `Cargo.toml`, or anything under `crates/northern-swarm/**` —
two other lanes own the swarm pallet and the executor right now.

## The work

Add the concurrency/partition evidence the row is missing, as **real tests that drive the crate's
real API** (not a mock of the thing under test). The list below is what the reviewers will look for;
pick the ones the crate can honestly support and say so in the test's doc comment when a case needs
infrastructure the crate does not have:

```text
two agents claim the same task          — exactly one wins, the loser gets a typed refusal
duplicate dispatch of one task          — the second dispatch is refused, no double ledger entry
kill switch fired during dispatch       — the in-flight dispatch is refused, nothing leaks to the queue
ledger consistency under N concurrent spawns — the ledger equals the sequential reference for the same input
agent never acknowledges (partition)    — the task returns to the queue, the agent is not credited
two concurrent kill switches for one agent — idempotent, one ledger entry, one event
replay of an already-applied task result — refused, balances unchanged
```

Rules for these tests:

* deterministic: seeded randomness, stable ordering, no `sleep`, no wall-clock dependence
  (`AGENTS.md` §17, `determinism-gate`);
* where a case must be refused, assert the **specific** error value, not "is_err";
* where nothing must move, assert the ledger/total is unchanged *and* assert the same call succeeding
  on a fresh instance (so "nothing happened" cannot pass because the call does nothing);
* no `#[ignore]`, no weakened assertions, no asserting a constant.

Then prove the important ones are load-bearing the way this repo requires: break the guard in a
scratch copy, watch the specific test go red, restore it byte-identically, watch the suite go green,
and record both outputs.

## Rules

* `cargo test --manifest-path crates/x3-swarm-core/Cargo.toml --all-targets` must pass; add
  `cargo clippy --manifest-path crates/x3-swarm-core/Cargo.toml --all-targets -- -D warnings` clean
  for what you touch. Use `CARGO_TARGET_DIR=/tmp/x3-target-swarm-core` so you never contend with the
  srtool build.
* **Never `git add -A`.** Stage only your own files, by explicit path. **Do not push** and do not
  touch `FEATURE_REGISTRY.toml` — report to `/root` and the primary agent will re-score the row.
* A second engineering framework works in this same tree. Files you must not touch even if they look
  broken: `pallets/northern-swarm/**`, `crates/northern-swarm/**`, `runtime/src/lib.rs`,
  `.maintain/frame-weight-template.hbs`, `scripts/mainnet/**`.

Report to `/root`: commit hash, file list, the exact commands and results, which of the cases above
you added, which you deliberately did not add and why, and the break-it-first evidence.
