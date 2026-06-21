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
grep -RIn "TODO\|FIXME\|stub\|mock\|fake\|placeholder\|dummy\|unimplemented!\|todo!\|panic!(\"not implemented" . --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules
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
