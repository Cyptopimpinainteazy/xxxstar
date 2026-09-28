# Workstream C — X3 Swarm Core (P1 of the low-score burn-down)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read
`AGENTS.md` first, then this brief. This is priority **P1** of the operator's burn-down order, after
Northern Swarm/Reactor (P0), which is now merged and green (`swarm_reactor_gate.py` 27/27, and the
release gate includes it).

## What is actually known about this subsystem — verify all of it

`FEATURE_REGISTRY.toml` has `[x3_swarm_core]`: `mode = "GUARDED_TESTNET"`,
`crate_or_service = "crates/x3-swarm-core"`, `readiness_score = 25`, and a cleared proof report
(`reports/swarm_health_report.md` does not exist). The matrix row X3-SWARM-003 (or whichever cites
`crates/x3-swarm-core`) says the crate is **excluded from the root workspace and absent from the
runtime dependency graph**, and that its own service (`crates/x3-swarm-core/services/x3-swarm-api`)
never calls the scheduler. Read those rows yourself; do not trust this paragraph.

## The operator's requirement

> Do not create another parallel scheduler. Consolidate so there is one clear lifecycle:
> `Agent → Task → Scheduler → Executor → Evidence → Verifier → Settlement → Reputation`.
> Connect useful existing swarm concepts into the chain rather than duplicating (planner, builder,
> tester, breaker, auditor, integrator, benchmark, security, research). Target loop:
> `SCAN → SCORE → PLAN → BUILD → TEST → BREAK → FIX → VERIFY → PROVE → REPORT`.
> Agents may autonomously research, benchmark, test, audit, generate patches, run simulations and
> produce proposals. They must NOT autonomously perform sensitive production actions (mainnet
> runtime upgrade, token supply modification, validator key replacement, genesis modification,
> critical settlement-rule changes) without the required governance/human authorization.

## What to do, in order

1. **Measure, do not assume.** Enumerate the crate's real surface (scheduler, task lifecycle,
>   reputation, agent roles, the service), what calls what, and which parts are reachable from
>   anything. `cargo tree -i x3-swarm-core` (it is excluded from the root workspace — say so), `rg`
>   for callers, and read the tests that exist.
2. **One concrete, load-bearing fix.** The most valuable outcomes, in my order:
   - a path that is *unreachable* today and that the lifecycle needs (e.g. the API service that never
     calls the scheduler) — make it real and test it;
   - a *duplicate* concept where two schedulers/lifecycles disagree — collapse them, and delete the
     loser rather than leaving both;
   - the **sensitive-action guard** above: if any swarm agent can currently take a production action
     without governance, that is a security defect — fix it with a test that tries and is refused.
3. **Track honestly.** Update `[x3_swarm_core]`'s readiness score only with the evidence you produce,
   using the registry's own formula: `implemented*0.35 + tested*0.25 + mainnet_ready*0.40`, and say in
   the comment what the three numbers are and why. 25 means "scaffold / concept"; 50–70 means
   "integrated and meaningfully tested". Do not award 70+ without end-to-end and adversarial coverage.

## Rules

- Do not weaken, skip, comment out or delete a gate or a test. No `unwrap()`/`expect()` on production
  paths (the panic ratchet is live: `python3 scripts/audit/panic_unwrap_scan.py`).
- `crates/x3-swarm-core` is outside the root workspace; use its own manifest
  (`cargo test --manifest-path crates/x3-swarm-core/Cargo.toml`) and keep it building.
- Commit only your own paths. **Never `git add -A`. Do not push** — message me (`/root`) with the
  commit hash, the commands, the results, and what you deliberately left open.
