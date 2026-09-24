# X3 Agent Guardrails (binding — identical to AGENTS.md)

`AGENTS.md` in the repository root is the single source of truth for every agent working in this
repo, Roo Code included. Read it before acting. This file repeats its non-negotiables so a mode
that never opens `AGENTS.md` cannot miss them. If the two ever disagree, `AGENTS.md` wins.

## Prime directive

Fix real code. Do not update documents instead of implementing working systems.

## Forbidden — do not create

- fake adapters
- fake relayers
- fake proofs
- no-op execution paths
- placeholder logic
- TODO-only work
- mocks outside test-only modules
- silent fallbacks in security code

Do not delete a failing test just to make the suite pass.

## Required proof before claiming completion

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

Run the fake-code scan over what you touched:

```bash
grep -rIn "TODO\|FIXME\|stub\|mock\|fake\|placeholder\|dummy\|unimplemented!\|todo!\|panic!(\"not implemented" . --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.venv --exclude-dir='.wt-*'
```

The repo's own gates are `make guard` (`scripts/agent_guard.py`, `scripts/no_stub_guard.py`,
`scripts/test_cheat_guard.py`), `make audit` (`scripts/invariant_guard.py`,
`scripts/mainnet_release_gate.py`), `make mainnet-check`, and `make fresh-machine-check`. A gate that
cannot run on this host is reported as BLOCKED with the reason; it is never reported as passing.

Command output outranks markdown. A file existing is not proof it is wired; a green test is not
proof the feature is reachable.

## Completion report — mandatory, every task

End every task with exactly this block:

```
Files changed:
Commands run:
Proof result:
Remaining blockers:
Next 10 tasks:
Completion percent:
```

## Critical X3 systems

Treat these as high-risk surfaces requiring tests plus invariant coverage:

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

## Task tracking

View, add, update, or delete project tasks with the TODO MCP tools (`todo_get_tasks`,
`todo_add_tasks`, `todo_update_tasks`, `todo_delete_tasks`, `todo_clear_category`,
`todo_move_category`) on server `todo-mcp` or `todo-extension`.

Never edit the `.todo` file directly with file tools.

## Skills

Repo skills live in `.agents/skills/` (`todo-tracker`, `featureproof`,
`ask-yourself-this-before-you-ask-me-any-questions`). When a task matches one, use it and say so.
