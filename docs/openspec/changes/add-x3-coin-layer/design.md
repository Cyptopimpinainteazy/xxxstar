## Context
X3 already has kernel settlement and proof scaffolding, but no canonical coin definition or mirror projection path. Launch requires runtime-native authority and a deterministic, replay-safe proof bridge to EVM.

## Goals / Non-Goals
- Goals:
  - Runtime-native X3 coin as canonical asset (single source of truth).
  - Proof-based mirror projections for EVM, SVM, and BTC with deterministic serialization and replay protection.
  - Single proof protocol and domain separation across all mirror domains.
  - Threshold signatures via BLS aggregation with a dedicated signer-set pallet.
- Non-Goals:
  - Permissionless bridge validation before proof aggregation stability is proven.

## Decisions
- Canonical asset lives in Substrate runtime and is referenced by `pallet-x3-kernel`/`pallet-x3-settlement-engine`.
- Mirror mints/burns only on threshold-signed proofs (no operator wallets) across EVM/SVM/BTC.
- Proofs must include domain separation: `{x3_chain_id, mirror_chain_id, nonce, intent_hash}`.
- Replay protection requires on-chain proof hash registry and monotonic nonce per domain.
 - Use BLS aggregation with a 2/3 threshold of the active signer set (managed by a dedicated pallet).
 - Genesis allocations are fixed and mapped to wallet-derived recipient accounts.

## Risks / Trade-offs
- Proof verification cost on EVM/SVM may limit throughput; mitigate with batching and aggregate signatures.
- BTC confirmation delays and reorg risk complicate timeouts; mitigate with explicit depth thresholds and refund windows.
- Mis-specified serialization breaks determinism; mitigate with explicit encoding tests and fixtures.
- Cross-chain outages could stall mirror updates; mitigate with queueing + retry logic.
 - BLS verification cost may be high for EVM; mitigate with batched verification and precompiles where available.

## Migration Plan
1. Introduce canonical asset and runtime API without mirror exposure.
2. Deploy EVM/SVM/BTC mirror components with mint/burn disabled.
3. Enable proof-based mint/burn behind feature flag after validation.

## Open Questions
- Which BLS curve + verification library is used across EVM/SVM/BTC tooling.
- Required proof payload size limits for EVM/SVM verification costs.
