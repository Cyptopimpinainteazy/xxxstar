## Context
X3 Chain already models dual-VM execution, but P5 requires a concrete validator stack that can validate Solana and Ethereum testnets with GPU acceleration and atomic guarantees. This stack runs off-chain as a validator service and does not alter runtime consensus logic.

## Goals / Non-Goals
- Goals:
  - GPU-accelerated EVM validation (secp256k1 + keccak256) with batch processing.
  - Atomic swap orchestrator enforcing dual-chain commit/rollback with timeouts.
  - Deterministic observability and auditability (metrics, logs, reports).
  - Testnet deployment and stress benchmarking for combined TPS targets.
- Non-Goals:
  - Modify on-chain runtime logic or consensus.
  - Provide production mainnet rollout in this change.

## Decisions
- Decision: Implement an off-chain validator service with GPU kernels and a Redis-backed atomic registry.
  - Rationale: isolates risk, avoids runtime modifications, and allows rapid iteration.
- Decision: Use batch-oriented GPU pipelines for secp256k1 verification and keccak256 hashing.
  - Rationale: batch size amortizes PCIe transfer costs and improves throughput.
- Decision: Enforce a strict atomic invariant with timeout-based rollback.
  - Rationale: safety-first to prevent partial execution across chains.

## Risks / Trade-offs
- GPU driver and CUDA compatibility: mitigate with explicit version checks and graceful failover to CPU only on error.
- Cross-chain latency spikes: mitigate with bounded timeout and retry policy.
- Redis availability: mitigate with local persistence and fail-closed behavior when registry is unavailable.

## Migration Plan
1. Add new `cross-chain-gpu-validator` package structure.
2. Implement GPU kernels and batch pipelines.
3. Implement orchestrator, registry, and operator dashboard.
4. Add deployment scripts and testnet benchmarks.
5. Run validation and produce reports.

## Open Questions
- None for proposal; implementation will follow tasks once approved.
