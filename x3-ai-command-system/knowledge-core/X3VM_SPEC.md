# X3VM Specification

## Overview

X3VM is the custom execution virtual machine for X3 Chain. It is designed to work alongside EVM and SVM as a first-class execution environment, providing intent-based execution with deterministic guarantees.

## Core Principles

1. **Deterministic execution** — Same input always produces same output
2. **Intent-based** — X3VM executes intents (declarative goals) rather than imperative transactions
3. **Canonical receipts** — All cross-VM operations produce canonical receipts
4. **Atomic coordination** — X3VM coordinates with EVM and SVM for atomic cross-VM execution
5. **Safety-first** — All operations preserve the Universal Asset Kernel invariant

## X3VM Execution Model

### Intents

An intent is a declarative specification of what the user wants to achieve:
- Source asset and amount
- Destination asset and chain/VM
- Slippage tolerance
- Deadline
- Replay nonce

### Execution Path

```
User submits intent
  → X3-Lang compiles intent into execution plan
  → Router selects optimal path across VMs
  → Executor validates and executes each step
  → Canonical receipt produced at each step
  → Finality check on each leg
  → Intent completed or refund initiated
```

### Canonical Receipts

Every cross-VM operation produces a canonical receipt containing:
- Intent ID (unique, non-replayable)
- Source VM and chain
- Destination VM and chain
- Asset type and amount
- Execution status
- Timestamp
- Replay nonce
- Timeout deadline

### Deterministic Ordering

Cross-VM operations must execute in deterministic order:
- Operations are ordered by (block_number, extrinsic_index, intent_nonce)
- No operation may depend on non-deterministic state
- All state transitions must be verifiable

## X3VM Opcodes

X3VM provides opcodes for:
- Intent submission and validation
- Cross-VM routing
- Asset transfer (lock/mint/burn/release)
- Receipt generation and verification
- Timeout and refund handling
- Finality verification

## Invariants

1. `canonical_supply == native + evm + svm + x3vm + cosmwasm + btc_locked + external_locked + pending`
2. No cross-VM operation may complete without a canonical receipt
3. No receipt may be replayed (nonce must be strictly increasing)
4. No operation may proceed past its deadline without explicit timeout handling
5. Every lock must have a corresponding release or refund path

## X3VM and Other VMs

| VM | Execution Model | Finality | State Model |
|---|---|---|---|
| EVM | Imperative (transactions) | Probabilistic | Account-based |
| SVM | Imperative (transactions) | Probabilistic | Account-based |
| X3VM | Intent-based | Deterministic | Receipt-based |
| Substrate | Runtime (extrinsics) | Deterministic | Storage-based |

X3VM coordinates between these models, translating intents into appropriate operations on each VM.