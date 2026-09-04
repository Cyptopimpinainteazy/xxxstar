# Skill: X3 Cross-VM Reviewer

## Purpose
Review EVM/SVM/X3VM/BTC/CosmWasm routing and adapter correctness. Cross-VM operations must be atomic or have proven rollback.

## Use When
- Reviewing bridge contracts, adapters, or settlement engine.
- Before claiming cross-chain features as working.
- When adding a new VM adapter.

## Inputs To Inspect
- `bridges/` — bridge contracts.
- `adapters/` — VM adapters.
- `crates/cross-vm-bridge/` — Rust bridge crate.
- `crates/atomic-swap-orchestrator/` — atomic swap logic.
- `X3-contracts/evm/` — EVM contracts.
- `X3-contracts/svm/` — SVM programs.

## Checks To Perform
- HTLC or two-phase commit for atomicity.
- Timeout and refund paths exist on both chains.
- Replay protection present.
- Finality handling correct per chain.
- Message format versioning.
- Malformed message handling.
- Reorg handling.

## Proof To Require
- Cross-VM integration tests pass.
- Failure injection tests pass.
- No stubs in bridge adapter paths.

## Output Format
- VMs reviewed: [list]
- Atomicity: HTLC / two-phase-commit / NONE (BLOCKER)
- Timeout+Refund: PRESENT / MISSING
- Replay Protection: PRESENT / MISSING
- Verdict: PASS / FAIL