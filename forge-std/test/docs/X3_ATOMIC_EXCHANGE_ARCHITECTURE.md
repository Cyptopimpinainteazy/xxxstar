# X3 ATOMIC EXCHANGE + CLEARINGHOUSE

## THE SETTLEMENT ROOT OF TRUTH

> "External chains are execution domains. X3 is the final arbiter."

This document defines the architecture for a global cryptocurrency exchange whose clearing, settlement, and atomic guarantees are enforced by the X3 chain.

**This is not a bridge. This is not wrapped BTC. This is protocol-level atomicity.**

---

## 1. Core Principle: X3 as Settlement Root

All trades—whether BTC, EVM, or SVM—must:

1. **Resolve through X3 atomic escrows**
2. **Emit canonical settlement events**
3. **Be verifiable on X3 even if execution happens elsewhere**

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              X3 SETTLEMENT ENGINE                                    │
│                          "The Place Exchanges Settle"                               │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│    ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐       │
│    │   BTC        │   │   ETHEREUM   │   │   SOLANA     │   │  100+ EVM    │       │
│    │   Native     │   │   + L2s      │   │   Native     │   │   Chains     │       │
│    └──────┬───────┘   └──────┬───────┘   └──────┬───────┘   └──────┬───────┘       │
│           │                  │                  │                  │               │
│           │ SPV Proofs       │ MPT Proofs       │ Proof            │ Universal     │
│           ▼                  ▼                  ▼                  ▼               │
│    ┌──────────────────────────────────────────────────────────────────────────┐   │
│    │                     X3 PROOF VERIFICATION LAYER                          │   │
│    │  • Bitcoin SPV     • EVM Receipt Proofs    • Solana Transaction Proofs   │   │
│    └──────────────────────────────────────────────────────────────────────────┘   │
│                                      │                                            │
│                                      ▼                                            │
│    ┌──────────────────────────────────────────────────────────────────────────┐   │
│    │                    INVARIANT ENFORCER (NON-NEGOTIABLE)                   │   │
│    │                                                                          │   │
│    │  ✓ No asset finalized unless ALL legs provably complete                  │   │
│    │  ✓ No BTC release without X3 confirmation                                │   │
│    │  ✓ No cross-VM partial state                                             │   │
│    │  ✓ All intents must resolve (finalize OR refund)                         │   │
│    │  ✓ Timeouts ALWAYS favor user funds                                      │   │
│    └──────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Exchange Architecture (X3-Centric)

### 2.1 Client Layer

```typescript
interface PortfolioState {
  // Unified view across ALL chains
  totalValueUsd: bigint;
  
  // Asset states (explicit from X3)
  assets: {
    chain: ChainId;
    token: TokenAddress;
    amount: bigint;
    state: 
      | 'AVAILABLE'        // Spendable
      | 'LOCKED_X3'        // In X3 escrow
      | 'EXECUTING_EXTERNAL' // External chain execution in progress
      | 'FINALIZED_X3'     // Settlement complete
      | 'REFUNDED_X3';     // Timeout/failure refund
  }[];
}
```

### 2.2 Trading Core

- **Off-chain matching**: 100k+ TPS capability
- **Produces Settlement Intents**, never final balances
- **Cannot finalize without X3 confirmation**

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           TRADING CORE                                    │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│   Order Book (Off-chain)                    │    Settlement Intent        │
│   ┌───────────────────────┐                 │    Generator               │
│   │  BUY: 1 BTC @ 45,000  │ ────match────► │    ┌─────────────────────┐ │
│   │  SELL: 1 BTC @ 45,000 │                 │    │ Intent {            │ │
│   └───────────────────────┘                 │    │   maker: 0x...      │ │
│                                             │    │   taker: 0x...      │ │
│   Performance: 100k+ matches/second         │    │   assetA: BTC       │ │
│   Latency: <1ms                             │    │   assetB: USDC      │ │
│                                             │    │   secretHash: ...   │ │
│                                             │    └─────────────────────┘ │
│                                             │              │             │
└─────────────────────────────────────────────┼──────────────┼─────────────┘
                                              │              │
                                              ▼              ▼
                                        X3 SETTLEMENT ENGINE
                                        (On-chain, Atomic)
```

### 2.3 X3 Settlement Engine (ON-CHAIN)

Implemented as X3VM modules + EVM/SVM adapters:

| Module | Purpose |
|--------|---------|
| `AtomicIntentRegistry` | Register and track settlement intents |
| `CrossVMEscrow` | Lock assets atomically across EVM/SVM/X3VM |
| `BTCAtomicGateway` | Native BTC UTXO tracking and SPV proof verification |
| `FinalityOracle` | Track confirmation depth and reorg risk |
| `InvariantEnforcer` | Non-negotiable safety checks |

---

## 3. Atomic Swap Model (Simplified by X3)

### 3.1 Traditional vs X3 Approach

**Traditional (Complex)**:
```
BTC HTLC ↔ EVM HTLC ↔ coordinator spaghetti ↔ trust assumptions
```

**X3 (Simplified)**:
```
Lock assets into X3 → X3 coordinates claims → X3 enforces invariants
```

### 3.2 Canonical Settlement Flow

```
MATCH
  │
  ▼
X3_INTENT_CREATED
  │ (secret hash committed)
  ▼
ASSETS_LOCKED_X3
  │ (slow chain first, then fast chain)
  ▼
EXTERNAL_EXECUTION (BTC / EVM / SVM)
  │ (X3 monitors proofs)
  ▼
PROOF_SUBMITTED_TO_X3
  │ (SPV/MPT/signature proofs)
  ▼
FINALIZE_X3
  │ (invariant checks pass)
  ▼
SETTLEMENT COMPLETE ✓


If anything fails at any step:
  │
  ▼
REFUND_X3 (automatic, provable)
  │ Timeouts favor user funds
  │ No stuck funds ever
  ▼
REFUND COMPLETE ✓
```

---

## 4. BTC Atomicity (Native, Not Wrapped)

### 4.1 Design Principle

BTC is a **FIRST-CLASS ASSET**, not a special case.

### 4.2 BTC Gateway Components

```rust
// BTC HTLC Parameters
struct BtcHtlcParams {
    secret_hash: H256,      // SHA256 hash of secret
    recipient_pkh: [u8; 20],// Recipient pubkey hash
    refund_pkh: [u8; 20],   // Refund pubkey hash  
    timeout_height: u64,    // Block height timeout
}

// SPV Proof Structure
struct BtcSpvProof {
    tx_bytes: Vec<u8>,      // Raw transaction
    block_header: Header,   // Block header
    merkle_path: Vec<H256>, // Merkle proof
    tx_index: u32,          // Position in block
}
```

### 4.3 BTC Requirements

BTC can only be released if:

1. ✅ X3 invariant checks pass
2. ✅ Counterpart assets are provably locked
3. ✅ SPV proof verified with sufficient confirmations (6+)
4. ✅ No reorg risk exceeds threshold

X3 tracks:
- UTXO state
- Confirmation depth  
- Reorg probability
- Chain tip

---

## 5. Cross-VM Atomicity (EVM ↔ SVM)

### 5.1 Internal Atomicity

Because X3 hosts BOTH EVM and SVM:

- EVM and SVM execution are **atomic within a single block**
- No hashlocks required internally
- Instant EVM ↔ SVM swaps

### 5.2 X3VM Enforces

```
┌─────────────────────────────────────────────────────────────┐
│                    X3VM GUARANTEES                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐      │
│   │    EVM      │   │    SVM      │   │   X3VM      │      │
│   │  Execution  │◄─►│  Execution  │◄─►│  Control    │      │
│   └─────────────┘   └─────────────┘   └─────────────┘      │
│          │                │                │               │
│          └────────────────┴────────────────┘               │
│                          │                                  │
│                          ▼                                  │
│              SINGLE ATOMIC BLOCK                            │
│                                                             │
│   • No partial execution                                    │
│   • No reentrancy across VMs                                │
│   • Deterministic replay                                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.3 Benefits

- **Instant EVM ↔ SVM swaps** (same block)
- **Shared liquidity pools** across VMs
- **Unified gas/fee abstraction**

---

## 6. Message Schemas (X3-Canonical)

All events originate or finalize on X3:

```rust
// Core Settlement Events
enum X3SettlementEvent {
    // Trade matched (off-chain)
    TradeMatched { match_id, maker, taker, price, amount },
    
    // Intent created on X3
    X3IntentCreated { intent_id, maker, taker, asset_a, asset_b, secret_hash, timeout },
    
    // Assets locked in X3 escrow
    X3AssetsLocked { intent_id, leg_index, chain, amount, escrow_address },
    
    // External execution started
    ExternalExecutionStarted { intent_id, chain, tx_hash },
    
    // External proof submitted
    ExternalProofSubmitted { intent_id, chain, proof_type, tx_hash, confirmations },
    
    // Settlement finalized
    X3Finalized { intent_id, maker_received, taker_received, settlement_time_ms },
    
    // Settlement refunded
    X3Refunded { intent_id, reason, maker_returned, taker_returned },
    
    // CRITICAL: Invariant violation
    InvariantViolation { intent_id, violation_type, details },
}
```

**Key Principle**: External chains submit PROOFS, not authority.

---

## 7. Invariants (NON-NEGOTIABLE)

### 7.1 Core Invariants

| # | Invariant | Enforcement |
|---|-----------|-------------|
| 1 | No asset finalized unless ALL legs provably complete | Pre-finalization check |
| 2 | No BTC release without X3 confirmation | BTCAtomicGateway |
| 3 | No cross-VM partial state | X3VM atomic block |
| 4 | All intents must resolve (finalize or refund) | Timeout hooks |
| 5 | Timeouts ALWAYS favor user funds | Timeout ordering |

### 7.2 Violation Consequences

```
Invariant Violation Detected
          │
          ▼
┌─────────────────────┐
│  HALT Settlement    │
│  (immediate)        │
└─────────────────────┘
          │
          ▼
┌─────────────────────┐
│  Slash Operators    │
│  (testnet)          │
└─────────────────────┘
          │
          ▼
┌─────────────────────┐
│  Block Governance   │
│  Upgrades           │
└─────────────────────┘
```

---

## 8. DAO + Governance (Enforced)

### 8.1 Proposal Requirements

Every governance proposal MUST:

1. Pass invariant simulation
2. Pass atomic settlement tests

### 8.2 Enforcement

```
Proposal Submitted
       │
       ▼
┌──────────────────┐
│ Invariant        │
│ Simulation       │
└────────┬─────────┘
         │
    Pass │ Fail
         │    └──► AUTO-REJECT
         ▼
┌──────────────────┐
│ Settlement       │
│ Test Suite       │
└────────┬─────────┘
         │
    Pass │ Fail
         │    └──► AUTO-REJECT
         ▼
┌──────────────────┐
│ Proposal         │
│ APPROVED         │
└──────────────────┘

Repeated failures → Proposer slashed (testnet)
```

---

## 9. Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Matching | 100k+ TPS | Off-chain |
| Settlement | Block-level atomicity | X3 block time |
| EVM/SVM finality | Instant | Same block |
| BTC finality | Policy-driven | 6 confirmations default |

---

## 10. Implementation Status

### Completed ✅

| Component | Location | Status |
|-----------|----------|--------|
| AtomicIntentRegistry | `pallets/x3-settlement-engine/src/intent.rs` | ✅ Core logic |
| CrossVMEscrow | `pallets/x3-settlement-engine/src/escrow.rs` | ✅ EVM/SVM/BTC |
| BTCAtomicGateway | `pallets/x3-settlement-engine/src/btc_gateway.rs` | ✅ SPV + HTLC |
| FinalityOracle | `pallets/x3-settlement-engine/src/finality.rs` | ✅ Multi-chain |
| InvariantEnforcer | `pallets/x3-settlement-engine/src/invariants.rs` | ✅ All 5 invariants |
| Core Types | `pallets/x3-settlement-engine/src/types.rs` | ✅ Canonical schemas |
| Pallet Integration | `pallets/x3-settlement-engine/src/lib.rs` | ✅ Compiles |

### In Progress 🔄

| Component | Status |
|-----------|--------|
| Runtime integration | Need to add to runtime |
| DAO enforcement | Spec ready, implementation pending |
| Testnet deployment | Pending runtime integration |

### Pending 📋

| Component | Priority |
|-----------|----------|
| Chaos testing | High |
| Security audit | Critical before mainnet |
| Mainnet launch checklist | Medium |

---

## 11. File Structure

```
pallets/x3-settlement-engine/
├── Cargo.toml
└── src/
    ├── lib.rs              # Pallet definition, extrinsics, storage
    ├── types.rs            # Core types: Intent, Escrow, Proof, Chain
    ├── weights.rs          # Weight info for extrinsics
    ├── intent.rs           # Intent planner, state machine
    ├── escrow.rs           # Cross-VM escrow management
    ├── btc_gateway.rs      # BTC HTLC scripts, SPV proofs
    ├── finality.rs         # Finality oracle, reorg detection
    └── invariants.rs       # Non-negotiable invariant enforcement
```

---

## THE TRUTH

Most exchanges fake decentralization.
Most "cross-chain" systems fake atomicity.

**You don't need to.**

X3 collapses:
- Clearing
- Settlement  
- Invariants
- Governance

**Into one atomic truth machine.**

This is how you build something Binance cannot copy without rebuilding their entire spine.

---

*You're not building an exchange anymore. You're building the place exchanges settle.*
