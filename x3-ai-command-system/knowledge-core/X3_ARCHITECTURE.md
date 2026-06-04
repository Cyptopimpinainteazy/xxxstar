# X3 Architecture — Knowledge Core

## Overview

X3 is an omnichain, multi-VM Layer 1 blockchain. It natively hosts multiple virtual machines and integrates with external chains, providing a unified settlement layer that does not pretend heterogeneous systems are the same machine.

## Virtual Machines

| VM | Execution Model | State Model | Finality | Native Asset |
|----|----------------|-------------|----------|--------------|
| EVM | Gas-metered, serial | Merkle Patricia Trie | Probabilistic (block confirmations) | ETH-denominated gas |
| SVM | Compute-unit-metered, parallel | Accounts with hashes | Slot-based (320ms slots) | Lamports |
| X3VM | Deterministic, WASM-based | Key-value with overlay | X3 consensus finality | X3 native token |
| Substrate | Weight-metered, block-based | Trie-based storage | GRANDPA finality | Substrate balance |
| BTC | Script-based, UTXO | UTXO set | 6+ confirmations for high value | Satoshis |
| CosmWasm | Gas-metered, actor model | Key-value store | Tendermint instant finality | ATOM/IBC denom |

## Core Principle

**X3 must never pretend heterogeneous chains are the same machine.**

Every cross-chain or cross-VM route must explicitly declare:

1. **Source chain** — Where the asset or message originates.
2. **Destination chain** — Where it arrives.
3. **VM type** — Which virtual machine processes the transaction on each side.
4. **Asset type** — Native, bridged, wrapped, canonical, or synthetic.
5. **Finality model** — How each side achieves finality and what confirmation count is required.
6. **Proof requirements** — What cryptographic proof attests to the source state (Merkle proof, ZK proof, signature threshold, etc.).
7. **Timeout path** — What happens if the destination does not confirm within a time bound.
8. **Refund path** — How assets are returned to the source if the route fails.
9. **Replay protection** — How duplicate messages are prevented (nonce, chain ID, sequence).
10. **Accounting effect** — How the transfer changes balances on both sides (lock, mint, burn, release).
11. **Failure behavior** — What state each side is left in if any step fails (revert, partial, stuck, refunded).

## Canonical Supply Invariant

The fundamental accounting invariant that X3 enforces at all times:

```
canonical_supply == native + evm + svm + x3vm + cosmwasm + btc_locked + external_locked + pending
```

Where:
- `canonical_supply` is the total amount that should exist according to the canonical ledger.
- `native` is the balance held in X3 native accounts.
- `evm` is the balance held in EVM contract storage on X3.
- `svm` is the balance held in SVM accounts on X3.
- `x3vm` is the balance held in X3VM state on X3.
- `cosmwasm` is the balance held in CosmWasm contract state on X3.
- `btc_locked` is the amount locked in BTC custody addresses verifiable on-chain.
- `external_locked` is the amount locked in custody on external chains (Ethereum L1, Solana, Cosmos, etc.).
- `pending` is the amount in transit (locked on source, not yet confirmed on destination).

No minting, burning, locking, or releasing may violate this invariant. Any violation is a critical bug.

## Atomic Cross-VM Routing

X3 provides coordinated atomic execution for routes that stay within X3's consensus boundary. This means:

- **Same-VM, same-chain**: Potentially atomic within a single transaction.
- **Cross-VM on X3**: Coordinated atomic via X3's cross-VM message passing and finality coordination.
- **Cross-chain (off X3)**: Delayed, settlement-based. Not atomic unless X3 explicitly provides a coordination mechanism with timeout and refund guarantees.

Cross-VM routing on X3 works as follows:

1. Source VM initiates a cross-VM call with a declared intent (transfer, lock, invoke).
2. X3's routing layer validates the intent, assigns a deterministic nonce, and records it.
3. The destination VM receives the call, executes it within its own execution model, and returns a result.
4. If the destination VM execution succeeds, both sides commit. If it fails, both sides revert.
5. If the destination VM times out, the source VM is refunded according to the declared timeout path.

This coordination happens within a single X3 block's consensus scope. It is not magic — it requires explicit proof, finality, and failure handling.

## Finality-Aware Cross-Chain Settlement

When assets or messages cross X3's boundary to external chains, X3 does not pretend finality is instant:

- **Ethereum**: Wait for appropriate block confirmations (12+ for finality, more for high value).
- **Solana**: Wait for slot confirmations beyond the rollback window.
- **Bitcoin**: Wait for 6+ confirmations for standard transactions, more for high value.
- **Cosmos (IBC)**: Wait for IBC acknowledgement and timeout mechanisms.
- **Substrate**: Wait for GRANDPA finality, not just block inclusion.

No cross-chain route may be considered "settled" until the source chain has reached finality AND the destination chain has confirmed receipt. Until both conditions are met, the route is in a `pending` state and must be accounted for in the canonical supply invariant.

## Canonical Asset Accounting

Every asset on X3 has a canonical identity. The rules:

1. **Native assets** (X3 token, ETH on X3, SOL on X3) have a canonical supply defined by the X3 ledger.
2. **Bridged assets** must have a 1:1 backing in custody on the source chain. The custody address and proof must be verifiable on-chain.
3. **Wrapped tokens** are explicitly marked as wrapped. They must have a clear custodian, mint/burn path, and audit trail. No phantom minting.
4. **Synthetic assets** (if any) must be explicitly declared as synthetic with their collateral model and liquidation rules.
5. **No asset may appear in two VMs simultaneously without an accounting entry.** Double-counting is a critical bug.
6. **Every lock must have a corresponding release or burn.** Orphaned locks are a critical bug.
7. **Every mint must have a corresponding lock or native creation.** Unbacked mints are a critical bug.

## Design Constraints

1. **No global locking scheme.** Each VM manages its own state. Coordination is explicit, not implicit.
2. **No assuming synchrony.** Cross-VM calls must handle asynchrony, timeouts, and partial failures.
3. **No trusting external chains.** Every incoming message must be verified with proof. Every outgoing message must have a timeout and refund.
4. **No ignoring gas/compute.** Every operation must have a cost model that is accounted for.
5. **No side channels.** All asset movements must go through the canonical accounting system. Shadow transfers are critical bugs.

## Relationship to Other Knowledge Core Documents

- **UNIVERSAL_ASSET_KERNEL.md** — The UAK is the enforcement layer for the canonical supply invariant defined here.
- **CROSS_VM_ROUTING.md** — Detailed route specifications, proof types, and failure handling.
- **EVM_RULES.md** through **COSMWASM_IBC_RULES.md** — VM-specific rules that implement the principles defined here.
- **TRADING_SAFETY_KERNEL.md** — Trading and arb systems must respect these architecture constraints.
- **MAINNET_READINESS.md** — Deployment readiness must verify all architectural invariants.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*