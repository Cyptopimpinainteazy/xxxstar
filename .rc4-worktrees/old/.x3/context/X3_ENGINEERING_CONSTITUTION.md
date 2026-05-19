# X3 Engineering Constitution

X3 Atomic Star is a production blockchain system.

The AI may assist, but tests, invariants, launch gates, risk registers, and command evidence decide truth.

## Non-Negotiables

- No stubs.
- No fake completion.
- No skipped files.
- No weakened tests.
- No blind migration from old repo.
- No mainnet-ready claims without proof.
- No unsafe changes to runtime, bridge, VM, asset kernel, DEX, genesis, chain spec, treasury, validator, or deployment paths without risk notes and tests.

## Core Invariants

### Universal Asset Kernel

`canonical_supply == native + evm + svm + external_locked + pending`

### Atomic Cross-VM Execution

All VM legs commit or all VM legs roll back.

### Bridge / Router

- No replay.
- No expired execution.
- No wrong-domain execution.
- No unaudited external bridge enabled by default.

### DEX / Launchpad

Reserves, LP supply, fees, locks, anti-rug constraints, and accounting must stay consistent.

## Proof Standard

A feature is complete only when:

- code compiles
- tests exist
- tests pass
- feature registry updated
- risk register updated
- docs updated
- rollback plan exists
- PATCH_LOG.md updated

## Operating Rule

Many agents may analyze.
One agent patches.
Tests judge.
Auditor blocks.
Commander queues the next task.
