# Universal Asset Kernel (UAK) — Knowledge Core

## Overview

The Universal Asset Kernel (UAK) is the canonical accounting enforcement layer for X3. It ensures that the supply invariant holds across all virtual machines and external chains at all times. Every transfer, lock, mint, burn, bridge, and refund must pass through the UAK's validation logic.

## The Invariant

```
canonical_supply == native + evm + svm + x3vm + cosmwasm + btc_locked + external_locked + pending
```

This invariant is the single source of truth for asset existence on X3. No operation may cause any term to change without a corresponding, equal-and-opposite change that preserves the equation.

### Invariant Terms

| Term | Definition |
|------|-----------|
| `canonical_supply` | The authoritative total supply as defined by the X3 ledger protocol. This is the ground truth. |
| `native` | Balance held in X3 native (Substrate-based) accounts. |
| `evm` | Balance held in EVM contract storage (ERC-20, ERC-721, etc.) on X3. |
| `svm` | Balance held in SVM accounts (SPL tokens, native lamports) on X3. |
| `x3vm` | Balance held in X3VM state (WASM-based contracts) on X3. |
| `cosmwasm` | Balance held in CosmWasm contract state on X3. |
| `btc_locked` | BTC locked in custody addresses, verifiable on the Bitcoin blockchain. |
| `external_locked` | Assets locked in custody on external chains (Ethereum L1, Solana, Cosmos Hub, etc.), verifiable on those chains. |
| `pending` | Assets in transit — locked on source, not yet confirmed on destination. This term is zero only when no routes are in flight. |

## Core Rules

### Rule 1: No Wrapped-Token Shortcuts Without Explicit Paths

Wrapped tokens are permitted only when all of the following are true:

- A **custodian** is defined: a specific address or multisig that holds the original asset.
- A **mint path** is defined: how the wrapped token is created (deposit original -> mint wrapped).
- A **burn path** is defined: how the wrapped token is destroyed (burn wrapped -> release original).
- A **lock path** is defined: how the original is locked when the wrapped token is minted.
- A **release path** is defined: how the original is released when the wrapped token is burned.
- A **refund path** is defined: what happens if the lock succeeds but the mint fails, or vice versa.
- The **custody proof** is verifiable on-chain at all times.
- The **supply** of the wrapped token never exceeds the locked original.

Any wrapped-token implementation that lacks any of these is a critical bug.

### Rule 2: Every Transfer Preserves the Invariant

A transfer moves value from one term to another (or within the same term). The sum must not change.

| Operation | Accounting Effect |
|-----------|-------------------|
| Native -> EVM | `native -= amount`, `evm += amount` |
| EVM -> SVM | `evm -= amount`, `svm += amount` |
| X3 -> External chain | `evm -= amount`, `external_locked += amount`, then `external_locked -= amount`, `pending += amount` until confirmation |
| External chain -> X3 | `pending += amount` until confirmation, then `pending -= amount`, `evm += amount` |
| Mint (backed by lock) | `btc_locked += amount`, `native += amount` |
| Burn (release from lock) | `btc_locked -= amount`, `native -= amount` |
| Refund (failed bridge) | `pending -= amount`, `native += amount` (or whichever VM the refund returns to) |

### Rule 3: Cross-VM Transfers Require Seven Checks

No cross-VM transfer may be committed without all seven of the following:

1. **Source verification** — The source VM has confirmed the deduction. The sender's balance has been reduced. The source state is finalized.
2. **Canonical receipt** — The UAK has recorded the transfer in the canonical ledger. The canonical_supply invariant is checked.
3. **Deterministic ordering** — The transfer has a globally unique, deterministic nonce (chain ID + VM ID + sequence number). No two transfers may share a nonce.
4. **Timeout/refund** — A timeout is defined. If the destination does not confirm within the timeout, the source is refunded. The refund path is tested and deterministic.
5. **Replay protection** — The nonce is consumed on both source and destination. A replayed message is rejected.
6. **Finality check** — The source chain/VM has reached finality (or the required confirmation depth) before the destination considers the transfer valid.
7. **Destination confirmation** — The destination VM has confirmed the receipt. The destination state is finalized. Only then is the transfer considered complete and `pending` reduced.

### Rule 4: Pending Must Converge to Zero

The `pending` term must converge to zero over time. This means:

- Every in-flight transfer has a timeout.
- Timed-out transfers are refunded automatically or via a claim mechanism.
- The UAK must be able to reconcile `pending` against a list of active routes.
- If `pending` grows without bound, that is a critical bug or an active exploit.

### Rule 5: No Phantom Minting

An asset may only be minted if:

- It is a native creation (defined by protocol rules, e.g., block rewards, staking rewards).
- It is backed by a lock on another chain (bridge minting).
- It is a wrapped token with a verified custodian and proof of reserves.

Any mint that does not fall into one of these categories is a phantom mint and is a critical bug.

### Rule 6: No Unaccounted Burns

An asset may only be burned if:

- It is a protocol-defined burn (e.g., fee burn, slashing).
- It releases a corresponding lock on another chain (bridge burn).
- It is a wrapped token being redeemed for the original, with proof of release on the source.

Any burn that does not have a corresponding effect is an unaccounted burn and is a critical bug.

### Rule 7: Double-Counting Is a Critical Bug

The same asset must not be counted in two terms simultaneously. Specifically:

- An asset locked in BTC custody must not also appear in `native` or `evm` without a corresponding mint entry.
- An asset bridged to an external chain must not also appear in `native` or `evm` without being in `external_locked` or `pending`.
- An asset in `pending` must not also appear in the destination term until confirmation.

### Rule 8: Custody Proofs Must Be Verifiable On-Chain

Every lock on an external chain must have a proof that is verifiable within X3's consensus:

- **BTC**: SPV proof or Merkle proof against a known block header.
- **Ethereum**: Merkle-Patricia proof against a finalized block header.
- **Solana**: Proof against a confirmed slot with required confirmation depth.
- **Cosmos/IBC**: IBC proof against a trusted header.

The UAK must be able to verify these proofs on-chain. Trust assumptions must be explicit and minimized.

## Failure Modes

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Source deduction without destination credit | `pending` grows without convergence | Timeout triggers refund to source |
| Destination credit without source deduction | `canonical_supply` invariant violation | Reverse the credit, alert operators |
| Double-spend across VMs | Same nonce appears in two VMs | Reject the second, log the attempt |
| Stuck pending (no timeout) | `pending` entry exceeds max age | Force-refund after governance timeout |
| Custody proof invalid | On-chain verification fails | Halt bridge, alert operators |
| Unbacked mint detected | Supply exceeds locked reserves | Freeze the asset, alert operators |

## Implementation Requirements

1. **Every bridge contract must call the UAK** before minting or releasing assets.
2. **Every cross-VM transfer must go through the UAK** — no direct VM-to-VM transfers bypassing the kernel.
3. **The UAK must be upgradeable only via governance** with a time-locked upgrade path.
4. **The UAK must emit events for every state change**: lock, release, mint, burn, transfer, pending, confirm, refund.
5. **The UAK must have a pause mechanism** that halts all new transfers but does not block refunds.
6. **The UAK must have an invariant check** that runs on every block (or every N blocks) and alerts if `canonical_supply != sum(terms)`.

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — Defines the invariant that the UAK enforces.
- **CROSS_VM_ROUTING.md** — Defines the route types that the UAK validates.
- **EVM_RULES.md** through **COSMWASM_IBC_RULES.md** — Each VM's rules must comply with the UAK.
- **MAINNET_READINESS.md** — The UAK invariant check must pass before mainnet deployment.
- **FORBIDDEN_PATTERNS.md** — Phantom minting, unbacked bridging, and double-counting are explicitly forbidden.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*