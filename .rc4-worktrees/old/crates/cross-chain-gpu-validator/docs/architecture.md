# Cross-Chain GPU Validator Architecture

## Overview

The cross-chain GPU validator is an off-chain service that validates Solana and Ethereum testnets with GPU acceleration and atomic guarantees.

## Components

### 1. GPU Kernels
- **Secp256k1 Kernel**: GPU-accelerated batch verification of EVM signatures
- **Keccak256 Kernel**: GPU-accelerated batch hashing for EVM state roots
- **CPU Fallback**: Identical implementations for failover and parity verification

### 2. Validators
- **EVM Validator**: Validates Ethereum state roots using GPU-batched keccak256
- **SVM Validator**: Validates Solana transactions and block hashes
- **Failover Manager**: GPU → CPU failover with health checks

### 3. Orchestration
- **Atomic Registry**: Redis-backed registry for swap state synchronization
- **Atomic Swap Orchestrator**: Enforces dual-chain commit/rollback with timeouts
- **Phase Management**: Tracks swap lifecycle (Pending → ValidatingEvm → ValidatingSvm → Committed/RolledBack)

### 4. Observability
- **Operator Dashboard**: Live metrics for TPS, success rate, rollback counts, GPU health, RPC latency
- **Metrics Pipeline**: Aggregation and reporting

## Design Principles

1. **Atomicity First**: All or nothing semantics across chains
2. **GPU Acceleration**: Batch processing for PCIe efficiency
3. **Safe Failover**: CPU fallback preserves atomic invariants
4. **Observability**: Comprehensive metrics and health checks
5. **Testnet Ready**: Deployment scripts for Solana + Ethereum testnets

## Data Flow

```
Swap Request
    ↓
Registry: Create record (Pending)
    ↓
Phase: ValidatingEvm
    ↓
EVM Validator (GPU → CPU failover)
    ↓
Phase: ValidatingSvm (if EVM passed)
    ↓
SVM Validator
    ↓
Phase: Committed (if both passed) / RolledBack (if either failed)
    ↓
Registry: Clean up
```

## Invariants

- **ATOMIC-COMMIT-001**: Both chains commit or both rollback
- **ATOMIC-TIMEOUT-002**: Timeout causes automatic rollback
- **GPU-CPU-PARITY-001**: GPU and CPU produce identical results
- **REGISTRY-CONSISTENCY-001**: Redis registry is source of truth for swap state

## Performance Targets

- Combined EVM + SVM throughput: 2-4M TPS
- GPU signature verification: 100k+ signatures/sec
- GPU keccak256 hashing: 1M+ hashes/sec
- Failover latency: <100ms

## Security Considerations

- Strict atomic invariants prevent partial execution
- Timeout-based rollback prevents deadlocks
- Fail-closed behavior when registry unavailable
- GPU health checks prevent silent corruption
- Signature verification uses standard secp256k1
