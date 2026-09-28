# Workstream A — Northern Swarm pallet: the adversarial matrix (P0, PR #519)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`, current branch
already contains the merge of `release-gate/swarm-reactor-onchain-v1` (PR #519). Read `AGENTS.md`
first, then this brief.

## Context you must know before touching anything

* The primary agent is **regenerating `pallets/northern-swarm/src/weights.rs` right now** (the
  committed one came from the generator's no-template fallback: it has no proof sizes and does not
  compile). Until that lands, `cargo test -p pallet-northern-swarm` **will not compile** — that is
  expected, not your bug. Write your tests first; run them when the primary agent tells you the
  weights are in, or after you see `cargo check -p pallet-northern-swarm` succeed.
* Do **not** edit `pallets/northern-swarm/src/weights.rs`, `pallets/northern-swarm/src/lib.rs`,
  `runtime/src/lib.rs`, or `scripts/mainnet/swarm_reactor_gate.py` — those are the primary agent's
  lane for this task. Your files are `pallets/northern-swarm/src/tests.rs` and
  `pallets/northern-swarm/src/mock.rs`.
* The gate `scripts/mainnet/swarm_reactor_gate.py` requires these three test names to exist (they
  do; keep them): `quorum_requires_matching_results`,
  `task_reward_moves_reserved_balance_to_winner`, `task_reward_preserves_total_issuance`.

## Your job

The pallet has **5 tests**. The release prompt's adversarial list is much longer, and these are the
cases that decide whether the on-chain compute path is real:

```text
2 matching results + 1 conflicting      (finalizes for the matching pair, and says so)
3 conflicting results                   (no winner, task ends disputed, stakes intact)
duplicate result from the same executor
result from an executor that never claimed the task
result from a suspended/deregistered executor
result after the task is already finalized
claim after finalization
claim past the per-executor claim limit
submit_result by a non-claimant
reward: executor account reaped / reward exceeds the reserved amount
reward: exact-balance boundary (reserved == reward)
executor deregisters while a task is active and claimed
heartbeat: executor marked inactive after the configured gap
```

For each: drive it through the pallet's real dispatchables with the mock runtime, assert the
*state transition* (task status, executor state, balances, events) — not just that a call returns
`Err`. Where a case should be refused, assert the specific `Error::<Test>::…` variant. Where a case
must not move funds, assert the balances and `TotalIssuance` are unchanged, and where it must move
them, assert the exact delta.

Then prove the important ones are load-bearing: break the check in the pallet (in a scratch copy),
watch the test go red, restore it byte-identically, watch it go green, and record the evidence.

## Rules

- Do not weaken, delete or `#[ignore]` an existing test. Do not add a test that asserts a constant.
- No `unwrap()`/`expect()` in the pallet's production paths (the panic ratchet is live:
  `python3 scripts/audit/panic_unwrap_scan.py`); test code is exempt.
- Commit only your two files. **Never `git add -A`. Do not push** — message me (`/root`) with the
  commit hash, the test names you added, and the exact commands and results.
