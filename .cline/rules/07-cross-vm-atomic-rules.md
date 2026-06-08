# Rule: Cross-VM Atomic Rules

## Purpose
Cross-VM operations (EVM, SVM, BTC, CosmWasm, X3VM) must be atomic or have proven rollback. Partial execution across chains is an invariant violation.

## Required Behavior
- Every cross-chain transfer must have a timeout and refund path.
- HTLC or similar two-phase commit must be used for cross-VM atomicity.
- Bridge adapters must handle reorgs, finality delays, and gas spikes.
- Settlement engine must reconcile state across all involved VMs.
- Cross-VM message format must be versioned and backward-compatible.

## Forbidden Behavior
- Do NOT ship one-way cross-chain transfers that can lose funds on failure.
- Do NOT hardcode finality assumptions (e.g., "EVM final after 1 block").
- Do NOT skip timeout handling in bridge contracts.
- Do NOT deploy untested cross-VM paths to testnet.
- Do NOT claim cross-VM atomicity without testing failure injection.

## Proof Required
- Cross-VM integration tests must pass.
- HTLC timeout/refund tests must pass.
- Reorg simulation must not break invariants.
- Bridge adapter must handle malformed messages gracefully.