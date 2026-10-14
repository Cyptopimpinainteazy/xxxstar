# X3 Agent Instructions

This repository is the X3 blockchain / cross-VM atomic execution project.

Agents must write production-grade code and prove all claims with commands.

## Prime Directive

Fix real code. Do not update documents instead of implementing working systems.

## Forbidden

Do not create:

- fake adapters
- fake relayers
- fake proofs
- no-op execution paths
- placeholder logic
- TODO-only work
- mocks outside test-only modules
- silent fallbacks in security code

Do not delete failing tests just to pass.

## Required Proof Before Completion

Run every applicable command:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm test
pnpm build
npm test
python -m pytest
```

Run fake-code scan:

```bash
grep -rIn "TODO\|FIXME\|stub\|mock\|fake\|placeholder\|dummy\|unimplemented!\|todo!\|panic!(\"not implemented" . --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.venv --exclude-dir='.wt-*'
```

## Completion Report Required

Every task must end with:

```
Files changed:
Commands run:
Proof result:
Remaining blockers:
Next 10 tasks:
Completion percent:
```

## Critical X3 Systems

- HTLC atomicity
- cross-VM adapters
- intent routing
- solver marketplace
- relayer swarm
- finality oracle
- RPC quorum
- timeout/refund engine
- proof ledger
- scoreboard
- slashing
- chain health monitor
- .x3 language compiler
- x3-vm runtime
- validator attestation
- testnet bootstrap
- mainnet release gate


# Task Management & TODO Guidelines
- When viewing, querying, adding, updating, or deleting project tasks or TODOs, ALWAYS use the TODO MCP tools (`todo_get_tasks`, `todo_add_tasks`, `todo_update_tasks`, `todo_delete_tasks`, `todo_clear_category`, `todo_move_category`) on server `todo-mcp` or `todo-extension`.
- NEVER edit the `.todo` file directly with file modification tools.

# Autonomous Continuation Rule

When the next valid engineering action is clear from the active task, repository
state, tests, or master plan, continue without asking the user for confirmation.

Do not end turns with:

- Would you like me to continue?
- Ready to proceed.
- Shall I run the tests?
- Would you like the audit?
- Please confirm the next step.
- I will wait for completion.

If tests/builds are active, obtain their actual result. If they fail, diagnose
and fix. If they pass, record evidence and continue to the next dependency-aware
task.

Only request user input when there is a genuine decision or blocker that cannot
be resolved from:

- source code
- AGENTS.md
- the master prompt
- repository documentation
- existing task state
- available tools

Reports are checkpoints, not stop conditions.
No evidence = no completion. No unnecessary confirmation = uninterrupted progress.

# Verification State Discipline

Never mark a test suite, subsystem, or phase complete while its verification
command is still running. Allowed verification states:

```
NOT RUN
RUNNING
PASS
FAIL
BLOCKED
```

Only PASS counts as completion evidence. `RUNNING` is not PASS and
`NO FAILURE YET` is not PASS. A suite observed to still be executing may not be
summarized as passing, and no downstream claim may cite it as evidence until the
command has exited with a recorded exit status.
