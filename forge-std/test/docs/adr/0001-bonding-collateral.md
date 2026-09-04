# ADR 0001 — Bonding & Collateral Design

Status: Proposed

Date: 2026-02-08

## Context

X3 requires a robust bonding/collateral system to enable permissionless agents to post bonds, provide risk alignment for arbitrage execution, and support deterministic slashing via the Court subsystem. This ADR defines scope, custody model, bond types, lifecycle, and initial implementation strategy (PoC → production).

## Decision

Adopt a hybrid approach:

- On-chain storage for canonical bond state (pallet `x3-settlement-engine`) to ensure immutable, verifiable records.
- Off-chain ledger & reconciliation for fast balance reflections and operator views, with on-chain settlement as the source-of-truth.
- Custody: protocol-managed escrow (on-chain via FRAME pallet) with operator tooling using HSM/Vault to sign maintenance operations.

## Bond types

- InitialMargin — required to open exposure
- MaintenanceMargin — ongoing collateral to avoid forced liquidation
- PerformanceBond — posted to guarantee correct behavior; slashed on proven violation

## Lifecycle

1. DepositBond — reserves funds and creates a Bond record (Locked)
2. RequestWithdraw — request and time-lock
3. FinalizeWithdraw — release funds to owner after time-lock and checks
4. Slash — mark bond Slashed and record SlashProof (integration with `x3-proof`)

## Acceptance Criteria

- Deposits create on-chain Bond record and off-chain ledger entry in < T_sync.
- Withdraw follows request + time-lock pattern; cannot be finalized before time-lock.
- Slashing is deterministic (Court verdict) and recorded immutably in the pallet.

## Implementation Notes

Start with a minimal `CollateralManager` trait and `InMemoryCollateral` PoC in `pallets/x3-settlement-engine/src/collateral.rs` (done). Next: storage-backed implementation (StorageMap), events, extrinsics for deposit/withdraw, and unit+integration tests.

Update 2026-02-08: Implemented storage-backed `Bond` storage (`Bonds`, `BondsByOwner`, `BondCounter`), internal helpers (`create_bond`, `request_withdrawal`, `finalize_withdraw`, `slash_bond`), and public FRAME extrinsics: `deposit_bond`, `request_bond_withdraw`, `finalize_bond_withdraw`, and `slash_bond`. Unit tests exercise both internal helpers and dispatchable flows. Remaining work: SDK RPC bindings, UI wiring, e2e tests, and OpenTelemetry/metrics.

"Done" requires: ADR merged, pallet extrinsics implemented and covered by unit tests, SDK bindings, e2e deterministic test passing.
