# Architecture

The cross-chain GPU validator runs as an off-chain service coordinating Solana
and Ethereum validation pipelines with an atomic swap orchestrator.

## Components

- GPU kernels: batch secp256k1 verification and keccak256 hashing.
- EVM validation pipeline: signature verification + state root validation.
- SVM validation pipeline: signature verification (placeholder).
- Atomic orchestrator: enforces dual-chain commit/rollback with strict timeout.
- Registry: Redis-backed swap registry (fail-closed if unavailable).
- Dashboard: static UI polling live metrics.

## Data Flow

1. Swap submission registers a swap in Redis.
2. Orchestrator validates SVM and EVM payloads.
3. If both sides validate within timeout, swap is approved.
4. If either fails or times out, swap is rolled back.

## Notes

CUDA kernels provided are placeholders until real crypto kernels are wired in.
Replace them with actual GPU implementations before production.
