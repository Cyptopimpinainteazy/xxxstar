# Change: Add cross-chain GPU validator stack

## Why
X3 Chain needs a production-ready cross-chain validator stack that can validate Solana and Ethereum testnets with GPU acceleration and atomic guarantees, enabling real-world performance and correctness validation for P5.

## What Changes
- Add a cross-chain GPU validator service with EVM GPU kernels (secp256k1, keccak256) and an SVM validation pipeline.
- Add an atomic swap orchestrator that enforces dual-chain commit/rollback semantics with Redis-based state synchronization.
- Add a testnet deployment and benchmarking harness for Solana + Ethereum.
- Add an operator dashboard and metrics pipeline for TPS, success rate, rollback counts, GPU health, and network latency.
- Add security checks, failure handling, and audit hooks for atomic guarantees.

## Impact
- Affected specs: new `cross-chain-gpu-validator` capability.
- Affected code: new service modules under `src/`, `kernels/`, `dashboard/`, `deployment/`, `docs/` (new package).
- Performance: introduces GPU kernels and batching pipelines; requires CUDA-capable nodes.
- Risk: coordination failures across chains; mitigated by strict atomic invariants and timeouts.
