# Atomic Swap Orchestrator

The `atomic-swap-orchestrator` is a high-performance cross-chain execution engine designed for the X3 Chain validator ecosystem. It enables atomic multi-chain transactions, specifically between EVM-compatible chains and the SVM (Solana) ecosystem, using the X3 VM and GPU-accelerated validator kernels.

## Overview

This crate provides the `AtomicSwapOrchestrator`, which manages the lifecycle of a cross-chain swap:
1.  **Preparation**: Gathering transaction data for both source and destination chains.
2.  **Simulation**: Running the swap in the X3 VM to verify outcomes.
3.  **GPU Acceleration**: Utilizing CUDA kernels for cryptographic verification and instruction parallelization.
4.  **Execution**: Submitting transactions to the respective chains.
5.  **Rollback**: Handling failures gracefully to ensure atomicity.

## Core Components

-   **`Orchestrator`**: The main entry point for processing swaps.
-   **`AtomicPair`**: Encapsulates the operations required on two different chains.
-   **`ExecutionStatus`**: Represents the final result of an atomic swap.

## Integration with X3 Bot

The `x3-bot` uses this orchestrator to execute real-time arbitrage and liquidity rebalancing opportunities identified across EVM and SVM chains.

## Development

### Build

```bash
cargo build -p atomic-swap-orchestrator
```

### Testing

```bash
cargo test -p atomic-swap-orchestrator
```

## Security

Atomic swaps rely on the X3 VM's sandboxed execution environment and the X3 Chain's validator consensus to ensure that funds are never at risk during the cross-chain transition.
