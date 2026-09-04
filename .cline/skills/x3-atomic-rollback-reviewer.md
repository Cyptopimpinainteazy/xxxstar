# Skill: X3 Atomic Rollback Reviewer

## Purpose
Verify rollback, timeout, replay protection, refund paths, and finality handling for cross-chain operations.

## Use When
- Reviewing swap, bridge, or settlement code.
- Before claiming atomic operations are safe.
- When timeouts or finality are changed.

## Inputs To Inspect
- `pallets/atomic-trade-engine/` — trade engine pallet.
- `pallets/x3-settlement-engine/` — settlement pallet.
- `crates/atomic-swap-orchestrator/` — orchestrator.
- `crates/flash-finality/` — finality handling.
- `bridges/AtomicBridge.sol` — bridge contract.

## Checks To Perform
- Timeout mechanism exists and is enforced.
- Refund path exists and is tested.
- Replay protection on both source and destination chains.
- Rollback does not leave partial state.
- Finality assumptions are chain-appropriate.
- Concurrent operations don't conflict.

## Proof To Require
- Timeout tests pass.
- Refund tests pass.
- Replay attack tests pass.
- Concurrent operation tests pass.

## Output Format
- Operation: <name>
- Timeout: PRESENT at <code> / MISSING
- Refund: PRESENT at <code> / MISSING
- Replay Protection: PRESENT / MISSING
- Verdict: PASS / FAIL