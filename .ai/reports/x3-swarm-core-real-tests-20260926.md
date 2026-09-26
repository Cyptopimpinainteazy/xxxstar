# `x3_swarm_core`: the four fictional required-tests are now real tests

Date: 2026-09-26
Scope: `crates/x3-swarm-core/tests/agent_lifecycle.rs`, `FEATURE_REGISTRY.toml` `[x3_swarm_core]`
Closes: audit queue `REG-x3_swarm_core-1` (the `required_tests were fictional` blocker)

## What was wrong

The independent audit CRITICAL-TOK-1 (2026-09-06) found that the registry cited four
tests for `[x3_swarm_core]` — `swarm_agent_can_receive_task`,
`swarm_agent_cannot_touch_forbidden_files`, `swarm_memory_records_lesson`,
`swarm_kill_switch_stops_agents` — and that **none of them existed as a test
function anywhere under `crates/x3-swarm-core`**. The registry had cited zero
names since, which is why the feature sits at `readiness_score = 25` and why the
audit queue carried the row as a "below 40pct - highest-triage candidate".

## What was done

The four tests are written against the crate's real API, in a new integration
target `crates/x3-swarm-core/tests/agent_lifecycle.rs`. They are behavioural, not
constant assertions:

| Test | What it proves on the implementation |
| --- | --- |
| `swarm_agent_can_receive_task` | `SwarmScheduler` hands an agent the task queued for its own `AgentKind`, still `Pending`; a class with nothing queued gets nothing; `update_status(.., Running)` takes the task out of the handout queue so it cannot be delivered twice; `Passed` is recorded; an unknown task id is refused |
| `swarm_agent_cannot_touch_forbidden_files` | `ForbiddenPathGuard` composes repo policy (`evaluate_path`) with tier scope: `.env`/`keys/`/`secrets/`/`id_rsa` are `Block` for every tier, `runtime/`+`pallets/`+`bridge/` are `SecurityReview` and refused to a `DocsTestsReports` agent, `ReadOnly`/`MainnetBlocked` allow nothing, `./x` and `docs\x` normalise, `docs/../keys/x` and `""` fail closed |
| `swarm_memory_records_lesson` | `AgentMemory` stores a lesson and returns it by agent and by feature; the finding, the regression-test name, the outcome and the timestamp survive; the store is append-only; the raw-vector helpers agree with it |
| `swarm_kill_switch_stops_agents` | `SwarmAuthority` is driven through three D-class violations to `Sanction::Kill`; the genesis record is terminated, the agent is inactive, an `AgentKilled` audit entry exists, `SpawnGuard` now refuses the agent's spawn with `ParentInactive`, and further misconduct returns `AuthorityError::AgentAlreadyKilled` |

`FEATURE_REGISTRY.toml` re-cites the four names, and the stale blocker
("Experimental — not in CI critical path") is replaced with the measured fact:
the crate's tests **do** run in the fast set (`test x3-swarm-core`), but
`crates/x3-swarm-core` is excluded from the root workspace and is absent from the
runtime dependency graph `cargo metadata` reports for `x3-chain-runtime`, so the
swarm control plane is not on the deployed blockchain path.

## Why the citation is now honest

`test x3-swarm-core` is a fast gate in `scripts/local-ci.sh` and runs
`cargo test --locked --all-targets --manifest-path crates/x3-swarm-core/Cargo.toml`,
which executes this file. `scripts/check-readiness-consistency.sh` verifies that
each cited name exists as a `fn` under the feature's own `crate_or_service`, and
`scripts/check-registry-tests-are-gated.py` verifies the crate behind every
registry citation is tested by a gate. Both now pass on this citation.

## Evidence

```
$ env CARGO_TARGET_DIR=/tmp/x3-nested-x3-swarm-core cargo test --locked --all-targets \
      --manifest-path crates/x3-swarm-core/Cargo.toml
  lib             54 passed; 0 failed; 0 ignored
  agent_lifecycle  4 passed; 0 failed; 0 ignored
  guard_tests     15 passed; 0 failed; 0 ignored

$ bash scripts/check-readiness-consistency.sh
  PASS: All status documents are consistent with FEATURE_REGISTRY.toml.

$ python3 scripts/check-registry-tests-are-gated.py
  18 registry feature(s) cite a crate a gate tests, 0 are on the shrinking KNOWN_UNGATED list

$ python3 scripts/x3_audit_matrix.py && python3 scripts/x3_audit_matrix.py --check
  x3-audit-matrix check PASS: artifacts match their sources

$ env CARGO_TARGET_DIR=/tmp/x3-nested-x3-swarm-core cargo clippy --locked --all-targets \
      --manifest-path crates/x3-swarm-core/Cargo.toml -- -D warnings
  Finished `dev` profile (exit 0)
```

## Also fixed here (found while verifying this change)

`crates/x3-swarm-core/src/policy.rs:19` did not compile under
`cargo clippy -- -D warnings`: `manual_div_ceil` — the 2/3 security-council quorum
threshold was written `(total * 2 + 2) / 3`. It is pre-existing (2026-09-03 baseline)
and invisible to CI, because the nested workspace has no clippy gate in
`scripts/local-ci.sh` — only `test x3-swarm-core` runs there, and `cargo test` does
not run clippy. Changed to `(total * 2).div_ceil(3)`: the same value for every
`usize` (checked for all `n` in `0..100000`, 0 mismatches), and the crate now exits 0
under the full `--all-targets -- -D warnings` lint.

## Findings left open (not fixed here)

1. The remaining `[x3_swarm_core]` blocker stands: no multi-agent
   race-condition or partition-tolerance testing exists.
2. `services/x3-swarm-api` and `services/x3-swarm-worker` are still `cargo check`
   only — they declare no tests, so the swarm control plane has no service-level
   coverage.
3. The nested swarm workspace has no clippy gate, which is how the `manual_div_ceil`
   failure above survived two years of green CI. Propose, do not silently add: a
   `clippy x3-swarm-core` entry in `scripts/local-ci.sh` alongside `test x3-swarm-core`.
