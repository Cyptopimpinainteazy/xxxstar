# Cross-VM Routing — Knowledge Core

## Overview

Cross-VM routing on X3 enables asset transfers and message passing between EVM, SVM, X3VM, Substrate, BTC, and CosmWasm. Each VM has a different execution model, finality mechanism, and state representation. Routes between them are not automatically atomic — X3 must explicitly coordinate finality, proof, lock, release, timeout, and refund.

## Route Types

### Type 1: Same-Chain, Same-VM (Potentially Atomic)

Transfers within a single VM on the same chain.

- **Example**: ERC-20 transfer within the EVM on X3.
- **Atomicity**: Potentially atomic within a single transaction.
- **Proof**: Not required — both sides share the same state.
- **Finality**: X3 block finality.
- **Timeout/refund**: Not required — transaction either succeeds or reverts atomically.
- **Replay protection**: X3 nonce mechanism.

### Type 2: Cross-VM on X3 (Coordinated Atomic)

Transfers between different VMs within X3's consensus boundary.

- **Example**: Transfer from an EVM account to an SVM account, both on X3.
- **Atomicity**: Coordinated atomic via X3's cross-VM message passing.
- **Proof**: X3 internal proof (receipt root, cross-VM message ID).
- **Finality**: X3 block finality for both sides.
- **Timeout/refund**: If the destination VM execution fails, the source VM is refunded within the same block.
- **Replay protection**: Cross-VM nonce (source VM + destination VM + sequence number).
- **Coordination mechanism**:
  1. Source VM locks the asset and emits a cross-VM message.
  2. X3 routing layer validates the message and assigns a deterministic nonce.
  3. Destination VM receives the message, executes, and returns a result.
  4. If success: both sides commit. If failure: both sides revert.
  5. If timeout (destination VM does not respond within the block): source VM is refunded.

### Type 3: Cross-Chain (Delayed/Settlement-Based)

Transfers between X3 and external chains, or between two external chains mediated by X3.

- **Example**: Bridge ETH from Ethereum L1 to X3 EVM.
- **Atomicity**: Not atomic. Requires finality on both sides.
- **Proof**: Cryptographic proof from the source chain (Merkle proof, SPV proof, ZK proof, or multi-sig attestation).
- **Finality**: Source chain finality + X3 finality. Both must be achieved.
- **Timeout/refund**: Mandatory. Defined at route creation. Refund path must be deterministic and tested.
- **Replay protection**: Source chain ID + destination chain ID + nonce + source tx hash.
- **Coordination mechanism**:
  1. Source chain locks the asset (or burns the wrapped version).
  2. Prover submits proof to X3 (or X3 submits proof to destination).
  3. X3 (or destination) verifies the proof.
  4. X3 (or destination) mits/releases the asset on the destination.
  5. If proof is invalid: rejected, no state change.
  6. If timeout: source chain is refunded.

## Route Specification

Every cross-VM route must declare the following:

| Field | Description |
|-------|-------------|
| `source_chain` | Chain where the asset originates (e.g., "ethereum", "x3", "solana", "bitcoin", "cosmos") |
| `destination_chain` | Chain where the asset arrives |
| `source_vm` | VM type on the source side (EVM, SVM, X3VM, Substrate, BTC, CosmWasm) |
| `destination_vm` | VM type on the destination side |
| `asset_type` | Native, bridged, wrapped, canonical, or synthetic |
| `finality_model` | How each side achieves finality (block confirmations, slots, GRANDPA, etc.) |
| `proof_type` | What proof attests to the source state (Merkle, SPV, ZK, multi-sig, internal) |
| `timeout` | Maximum time for destination confirmation before refund triggers |
| `refund_path` | How assets are returned if the route fails or times out |
| `replay_nonce` | Deterministic nonce preventing replay (chain ID + VM ID + sequence) |
| `slippage_check` | Whether the route has a slippage/price-impact check (for DEX routes) |
| `accounting_effect` | How balances change on both sides (lock/release, mint/burn, transfer) |
| `failure_behavior` | What state each side is left in if any step fails |

## Proof Types

| Proof Type | Used For | Verification Cost | Trust Assumption |
|-----------|---------|-------------------|------------------|
| Internal receipt | Cross-VM on X3 | Low (native verification) | X3 consensus |
| Merkle-Patricia proof | Ethereum -> X3 | Medium (EVM verification) | Ethereum finality |
| SPV proof | Bitcoin -> X3 | Medium (BTC header verification) | Bitcoin hashrate |
| ZK proof | Any chain (future) | Low (ZK verification) | ZK circuit correctness |
| Multi-sig attestation | Bridge custody | Low (signature check) | Multi-sig signer honesty |
| IBC proof | Cosmos -> X3 | Medium ( Tendermint verification) | Tendermint validator set |
| Slot proof | Solana -> X3 | Medium (SVM verification) | Solana validator set |

## Finality Requirements

| Chain | Minimum Finality | Recommended for High Value | Notes |
|-------|-------------------|---------------------------|-------|
| X3 | 1 block (BFT finality) | 3 blocks | X3 uses BFT consensus |
| Ethereum | 12 confirmations | 30+ confirmations | Post-merge, ~12 min for finality |
| Solana | 32 slots | 128 slots | Slot duration ~400ms |
| Bitcoin | 6 confirmations | 30+ confirmations | 1 hour per 6 confirmations |
| Cosmos (IBC) | IBC acknowledgement | IBC + 1 block | Tendermint instant finality |
| Substrate | GRANDPA finality | 2 GRANDPA rounds | ~30 seconds per round |

## Timeout and Refund Rules

1. **Every cross-chain route must have a timeout.** No route may remain in a pending state indefinitely.
2. **The timeout must be longer than the source chain's finality time.** Otherwise, the route may be confirmed on the source but timed out on the destination, leading to stuck funds.
3. **Refund must be deterministic.** Given a timed-out route, any party must be able to trigger the refund without relying on a specific operator.
4. **Refund must not create new attack vectors.** The refund path must have its own replay protection.
5. **Timeout must be explicit in the route specification.** No implicit or default timeouts.

## Slippage Checks

For routes that involve DEX swaps (e.g., cross-chain arb, cross-VM swaps):

1. **Every swap must declare a minimum output amount.** No unchecked swaps.
2. **Slippage must account for cross-VM/cross-chain latency.** A swap that takes 10 minutes to settle needs wider slippage tolerance than one that settles in 1 second.
3. **Slippage must be checked at execution time, not at submission time.** The actual output must be compared to the minimum.
4. **Failed slippage checks must result in a revert, not a silent loss.**

## Failure Modes and Handling

| Failure | Detection | Handling |
|---------|-----------|---------|
| Source lock succeeds, destination mint fails | Timeout on destination | Refund on source after timeout |
| Destination mint succeeds, source lock not finalized | Canonical supply check | Reverse destination mint, alert operators |
| Proof submission fails | Invalid proof verification | No state change, retry possible |
| Proof submission succeeds, but source chain reorgs | Source chain finality check after reorg | If reorg invalidates proof, halt route |
| Nonce collision | Duplicate nonce detection | Reject second submission, investigate |
| Route times out | Timeout timer fires | Execute refund path |
| Refund path fails | Manual intervention required | Alert operators, governance resolution |

## Accounting Effects by Route Type

### Lock-and-Mint (Bridge In)

```
Source: lock asset in custody -> external_locked += amount
X3:    mint wrapped asset     -> evm/native/svm += amount, pending += amount, then pending -= amount
```

### Burn-and-Release (Bridge Out)

```
X3:    burn wrapped asset     -> evm/native/svm -= amount, pending += amount
Source: release asset from custody -> external_locked -= amount, then pending -= amount
```

### Same-Chain Cross-VM Transfer

```
Source VM: deduct balance -> source_term -= amount
X3 routing: record transfer -> pending += amount
Dest VM: credit balance -> dest_term += amount, pending -= amount
```

### Failed Route Refund

```
Source: refund deducted balance -> source_term += amount, pending -= amount
```

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — Defines the canonical supply invariant and the principle that heterogeneous chains are not the same machine.
- **UNIVERSAL_ASSET_KERNEL.md** — The UAK validates the accounting effects of every route.
- **EVM_RULES.md** through **COSMWASM_IBC_RULES.md** — Each VM's rules include route-specific constraints.
- **TRADING_SAFETY_KERNEL.md** — Trading routes must comply with these route specifications.
- **MEV_DEFENSE.md** — Cross-VM routes may be vulnerable to MEV; defensive measures are required.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*