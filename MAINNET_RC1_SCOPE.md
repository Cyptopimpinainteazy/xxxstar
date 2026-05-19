# MAINNET RC-1 Scope

This document defines the v0.4 internal-only Mainnet RC-1 scope for X3 Atomic Star.
It is intended to be the authoritative scope statement for all RC-1 launch messaging.

## Enabled in RC-1
The following capabilities are part of the shipped RC-1 product surface:

- Universal Asset Kernel with supply-ledger invariants
- Internal X3Native / X3Evm / X3Svm domains
- Internal cross-VM routing with atomic source-debit / destination-credit semantics
- Packet standard lifecycle with message commitment, replay protection, and timeout handling
- IXL MVP bundle execution and receipt emission
- LiquidityCore spot swap path and LP lock behavior
- Kernel invariants and atomic rollback/refund logic
- Internal proof-backed launch gate automation
- Launch validator and operator tooling for an internal testnet
- Proof taxonomy and receipt generation for critical path claims

## Explicitly gated out of RC-1
These features are intentionally disabled for RC-1 and must remain off until a later audited phase:

- `external-gateway`
- `parallel-executor`
- `appzone-factory`
- `pq-experimental`
- `advanced-dex`
- `ai-optimizer`
- `gpu-acceleration`

## Enforced compile-time scope gates
The RC-1 scope is enforced in `pallets/x3-cross-vm-router/src/lib.rs` by compile-time errors whenever `mainnet-rc1` is enabled together with any forbidden feature. The source of truth is the `mainnet-rc1` feature list in `pallets/x3-cross-vm-router/Cargo.toml`.

### RC-1 enabled feature set
- `internal-ixl`
- `packet-standard`
- `liquidity-core`
- `kernel-invariants`

### RC-1 forbidden feature set
The following features are gated and must not be active in a `mainnet-rc1` build:

- `external-gateway`
- `parallel-executor`
- `appzone-factory`
- `pq-experimental`
- `advanced-dex`
- `ai-optimizer`
- `gpu-acceleration`

## Scope interpretation
- **Internal atomic settlement** is the core shipped value proposition.
- **Real external bridges, connected L1s, PQ-on-chain signatures, AI optimizer consensus, advanced DEX, and GPU-critical validator paths are roadmap items, not RC-1 launch claims.**

## Public messaging caution
This document is an internal RC-1 scope statement. It is not a public-mainnet readiness certification. Public launch language must stay limited to the RC-1 feature set above and should explicitly avoid claims about external bridge activation, PQ production readiness, AI consensus, or GPU-critical validator acceleration.

## Usage
Use this document for launch messaging, release notes, and audit checklists. If a public communication describes X3 as mainnet-ready, it must only refer to the RC-1 feature set above.
