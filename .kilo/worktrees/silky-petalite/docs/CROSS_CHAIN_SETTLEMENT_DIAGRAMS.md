# Cross-Chain Settlement Coordinator - Architecture & Flow Diagrams

## 1. System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                              X3 CHAIN PLATFORM                                       │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│  ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│  │                           NEXT.JS FRONT-END + API GATEWAY                        │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │    │
│  │  │ Portfolio    │  │ Swap Flow    │  │ Advanced     │  │ WebSocket Feed       │ │    │
│  │  │ View         │  │ Widget       │  │ Settlement   │  │ (Real-time Events)   │ │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────────────┘ │    │
│  └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                           │                                              │
│                                           ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│  │                           COMIT KERNEL (CORE MODULE)                             │    │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────────┐   │    │
│  │  │ Trading Core /   │  │ Accounting       │  │ Settlement Coordinator       │   │    │
│  │  │ Matching Engine  │  │ Engine           │  │ (State Machine)              │   │    │
│  │  │                  │  │                  │  │                              │   │    │
│  │  │ • Order intake   │  │ • Balance mgmt   │  │ • HTLC sequencing           │   │    │
│  │  │ • Shard assign   │  │ • Provisional    │  │ • Timeout handling          │   │    │
│  │  │ • Event sourcing │  │ • Confirmed      │  │ • Secret propagation        │   │    │
│  │  └──────────────────┘  └──────────────────┘  └──────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                           │                                              │
│                    ┌──────────────────────┼──────────────────────┐                      │
│                    │                      │                      │                      │
│                    ▼                      ▼                      ▼                      │
│  ┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────────────────┐      │
│  │   BTC ADAPTER        │  │   EVM ADAPTER        │  │   L2 / ALT-CHAIN         │      │
│  │                      │  │                      │  │   ADAPTERS               │      │
│  │ • UTXO selection     │  │ • ERC20 handling     │  │                          │      │
│  │ • HTLC scripts       │  │ • Contract escrow    │  │ • Arbitrum               │      │
│  │ • SPV proofs         │  │ • Event watching     │  │ • Base                   │      │
│  │ • 6 confirmations    │  │ • Reorg detection    │  │ • Polygon                │      │
│  └──────────────────────┘  └──────────────────────┘  │ • 100+ more chains       │      │
│            │                        │                └──────────────────────────┘      │
│            │                        │                            │                      │
└────────────┼────────────────────────┼────────────────────────────┼──────────────────────┘
             │                        │                            │
             ▼                        ▼                            ▼
    ┌─────────────────┐     ┌─────────────────┐          ┌─────────────────┐
    │   BITCOIN       │     │   ETHEREUM      │          │   L2 NETWORKS   │
    │   NETWORK       │     │   + EVM CHAINS  │          │   (Fast Final)  │
    └─────────────────┘     └─────────────────┘          └─────────────────┘
```

---

## 2. Atomic Swap State Machine

```
                                    ┌─────────┐
                                    │  OPEN   │
                                    │         │
                                    └────┬────┘
                                         │ SwapIntentCreated
                                         │ (secret hash generated)
                                         ▼
                              ┌──────────────────────┐
                              │  FUNDED_SLOW_CHAIN   │
                              │  (BTC / slower L1)   │
                              └──────────┬───────────┘
                                         │ EscrowFunded (slow)
                      ┌──────────────────┼──────────────────┐
                      │                  │                  │
                      │ TIMEOUT          │                  │
                      ▼                  ▼                  │
          ┌───────────────────┐  ┌──────────────────────┐   │
          │ REFUNDED_SLOW     │  │  FUNDED_FAST_CHAIN   │   │
          │ (maker recovers)  │  │  (ARB/BASE/etc)      │   │
          └───────────────────┘  └──────────┬───────────┘   │
                                            │               │
                      ┌─────────────────────┼───────────────┤
                      │                     │               │
                      │ TIMEOUT             │ CLAIM         │
                      ▼                     ▼               │
          ┌───────────────────┐  ┌──────────────────────┐   │
          │ REFUNDED_FAST     │  │      CLAIMED         │   │
          │ (taker recovers)  │  │  (secret revealed)   │   │
          └───────────────────┘  └──────────┬───────────┘   │
                                            │               │
                                            │ Both legs     │
                                            │ completed     │
                                            ▼               │
                                 ┌──────────────────────┐   │
                                 │     COMPLETED        │◄──┘
                                 │  (SwapCompleted)     │
                                 └──────────────────────┘
```

### State Descriptions

| State | Description | Events |
|-------|-------------|--------|
| `OPEN` | Swap intent created, secret hash committed | `SwapIntentCreated` |
| `FUNDED_SLOW_CHAIN` | Slower chain (BTC/L1) escrow confirmed | `EscrowFunded` |
| `FUNDED_FAST_CHAIN` | Faster chain (L2) escrow confirmed | `EscrowFunded` |
| `CLAIMED` | One party claimed with secret | `EscrowClaimed` |
| `REFUNDED_SLOW` | Slow chain refund after timeout | `EscrowRefunded` |
| `REFUNDED_FAST` | Fast chain refund after timeout | `EscrowRefunded` |
| `COMPLETED` | Both legs resolved successfully | `SwapCompleted` |

---

## 3. End-to-End Sequence Diagram

```
┌─────────┐      ┌───────────┐     ┌─────────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│  MAKER  │      │ MATCHING  │     │ SETTLEMENT  │    │   BTC   │    │   ARB   │    │  TAKER  │
│         │      │  ENGINE   │     │ COORDINATOR │    │ ADAPTER │    │ ADAPTER │    │         │
└────┬────┘      └─────┬─────┘     └──────┬──────┘    └────┬────┘    └────┬────┘    └────┬────┘
     │                 │                  │                │              │              │
     │  1. Submit      │                  │                │              │              │
     │  Swap Order     │                  │                │              │              │
     │────────────────►│                  │                │              │              │
     │                 │                  │                │              │              │
     │                 │  2. Match Found  │                │              │              │
     │                 │─────────────────►│                │              │              │
     │                 │                  │                │              │              │
     │                 │   3. SwapIntentCreated            │              │              │
     │◄────────────────┼──────────────────┼────────────────┼──────────────┼─────────────►│
     │                 │                  │                │              │              │
     │  4. Fund BTC    │                  │                │              │              │
     │  HTLC Escrow    │                  │                │              │              │
     │─────────────────┼──────────────────┼───────────────►│              │              │
     │                 │                  │                │              │              │
     │                 │                  │  5. Watch      │              │              │
     │                 │                  │◄───────────────│              │              │
     │                 │                  │  (6 confirms)  │              │              │
     │                 │                  │                │              │              │
     │                 │   6. EscrowFunded (BTC)           │              │              │
     │◄────────────────┼──────────────────┼────────────────┼──────────────┼─────────────►│
     │                 │                  │                │              │              │
     │                 │                  │                │              │  7. Fund ARB │
     │                 │                  │                │              │  Contract    │
     │                 │                  │                │              │◄─────────────│
     │                 │                  │                │              │              │
     │                 │                  │  8. Watch      │              │              │
     │                 │                  │◄───────────────┼──────────────│              │
     │                 │                  │  (1 confirm)   │              │              │
     │                 │                  │                │              │              │
     │                 │   9. EscrowFunded (ARB)           │              │              │
     │◄────────────────┼──────────────────┼────────────────┼──────────────┼─────────────►│
     │                 │                  │                │              │              │
     │                 │                  │ 10. Claim ARB  │              │              │
     │                 │                  │ (reveal secret)│              │              │
     │─────────────────┼──────────────────┼────────────────┼─────────────►│              │
     │                 │                  │                │              │              │
     │                 │  11. EscrowClaimed (ARB, secret)  │              │              │
     │◄────────────────┼──────────────────┼────────────────┼──────────────┼─────────────►│
     │                 │                  │                │              │              │
     │                 │                  │                │ 12. Claim    │              │
     │                 │                  │                │ BTC (secret) │              │
     │                 │                  │                │◄─────────────┼──────────────│
     │                 │                  │                │              │              │
     │                 │  13. EscrowClaimed (BTC)          │              │              │
     │◄────────────────┼──────────────────┼────────────────┼──────────────┼─────────────►│
     │                 │                  │                │              │              │
     │                 │  14. SwapCompleted                │              │              │
     │◄────────────────┼──────────────────┼────────────────┼──────────────┼─────────────►│
     │                 │                  │                │              │              │
```

---

## 4. Message Flow Detail

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         MESSAGE FLOW TIMELINE                               │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  T+0ms     SwapIntentCreated                                               │
│            ├── swapId: "abc-123"                                           │
│            ├── maker: "bc1q..."                                            │
│            ├── taker: "0x742d..."                                          │
│            ├── assetA: { chain: "BTC", amount: 0.5 }                       │
│            ├── assetB: { chain: "ARB", amount: 1200 }                      │
│            ├── secretHash: "sha256:a1b2c3..."                              │
│            └── timelocks: { slow: 12h, fast: 6h }                          │
│                                                                             │
│  T+10min   EscrowFunded (BTC)                                              │
│            ├── swapId: "abc-123"                                           │
│            ├── chain: "BTC"                                                │
│            ├── txHash: "4a5b6c..."                                         │
│            ├── blockHeight: 820,456                                        │
│            └── confirmations: 6                                            │
│                                                                             │
│  T+12min   EscrowFunded (ARB)                                              │
│            ├── swapId: "abc-123"                                           │
│            ├── chain: "ARB"                                                │
│            ├── txHash: "0x789..."                                          │
│            ├── blockHeight: 15,234,567                                     │
│            └── confirmations: 1                                            │
│                                                                             │
│  T+13min   EscrowClaimed (ARB) ← Maker claims fast chain first             │
│            ├── swapId: "abc-123"                                           │
│            ├── chain: "ARB"                                                │
│            ├── txHash: "0xabc..."                                          │
│            └── secret: "preimage:d4e5f6..."  ← SECRET REVEALED             │
│                                                                             │
│  T+15min   EscrowClaimed (BTC) ← Taker uses revealed secret                │
│            ├── swapId: "abc-123"                                           │
│            ├── chain: "BTC"                                                │
│            ├── txHash: "7d8e9f..."                                         │
│            └── secret: "preimage:d4e5f6..."                                │
│                                                                             │
│  T+16min   SwapCompleted                                                   │
│            ├── swapId: "abc-123"                                           │
│            ├── status: "completed"                                         │
│            └── finalBalances:                                              │
│                ├── maker: { BTC: 0.0, ARB: +1200 }                         │
│                └── taker: { BTC: +0.5, ARB: 0.0 }                          │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Chain Adapter Interface

```typescript
interface ChainAdapter {
  // Watch for deposits to escrow addresses
  watchDeposits(addresses: string[]): Observable<DepositEvent>;
  
  // Estimate finality and reorg risk
  estimateFinality(txHash: string): Promise<{
    confidence: number;      // 0.0 - 1.0
    reorgRisk: number;       // probability of reorg
    confirmations: number;   // current confirmations
    requiredConfs: number;   // chain-specific threshold
  }>;
  
  // Build escrow transaction (HTLC or smart contract)
  buildEscrow(swap: SwapIntent): Promise<UnsignedTx>;
  
  // Verify escrow state on-chain
  verifyEscrow(txHash: string): Promise<EscrowState>;
  
  // Claim escrow with revealed secret
  claimEscrow(swapId: string, secret: string): Promise<SignedTx>;
  
  // Refund escrow after timeout
  refundEscrow(swapId: string): Promise<SignedTx>;
  
  // Broadcast signed transaction
  broadcast(signedTx: string): Promise<TxReceipt>;
}

// BTC-specific extension
interface BtcAdapter extends ChainAdapter {
  utxoSelector(amount: bigint): Promise<UTXO[]>;
  buildHtlcScript(secretHash: string, timeout: number): Script;
}
```

---

## 6. UI State Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SWAP WIDGET STATES                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐  │
│  │   PENDING    │───►│ IN SETTLEMENT│───►│    ON-CHAIN          │  │
│  │              │    │              │    │                      │  │
│  │ "Matching    │    │ "Funding     │    │ "Waiting for         │  │
│  │  your order" │    │  escrows..." │    │  confirmations..."   │  │
│  │              │    │              │    │  [████████░░] 6/6    │  │
│  │ [Cancel]     │    │ [View Tx]    │    │  [View on Explorer]  │  │
│  └──────────────┘    └──────────────┘    └──────────────────────┘  │
│                                                    │                │
│                                                    ▼                │
│                                          ┌──────────────────────┐  │
│                                          │    FINALIZED         │  │
│                                          │                      │  │
│                                          │ ✓ Swap Complete!     │  │
│                                          │ +0.5 BTC received    │  │
│                                          │ -1200 USDC sent      │  │
│                                          │                      │  │
│                                          │ [View Details]       │  │
│                                          └──────────────────────┘  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 7. Observability Dashboard

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    SETTLEMENT COORDINATOR METRICS                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Active Swaps: 47          Completed (24h): 1,234      Failed: 3        │
│  ════════════════════════════════════════════════════════════════════   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  SWAP STATES DISTRIBUTION                                        │    │
│  │                                                                   │    │
│  │  OPEN              ████████████░░░░░░░░░░░░░░░░░░░░░  23 (49%)   │    │
│  │  FUNDED_SLOW       ████████░░░░░░░░░░░░░░░░░░░░░░░░░  12 (26%)   │    │
│  │  FUNDED_FAST       ████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   8 (17%)   │    │
│  │  CLAIMED           ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   4 (8%)    │    │
│  │                                                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  ALERTS                                                          │    │
│  │                                                                   │    │
│  │  ⚠️  Swap abc-123: BTC escrow funding timeout in 2h              │    │
│  │  ⚠️  Chain reorg detected on Polygon (depth: 2 blocks)          │    │
│  │  ✓  All systems operational                                      │    │
│  │                                                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  CHAIN HEALTH                                                    │    │
│  │                                                                   │    │
│  │  BTC      ✓ 820,456  │  6 confs  │  ~10 min/block               │    │
│  │  ETH      ✓ 18.2M    │  12 confs │  ~12 sec/block               │    │
│  │  ARB      ✓ 15.2M    │  1 conf   │  ~0.25 sec/block             │    │
│  │  BASE     ✓ 8.1M     │  1 conf   │  ~2 sec/block                │    │
│  │  POLYGON  ⚠ 52.1M    │  128 confs│  ~2 sec/block (reorg risk)   │    │
│  │                                                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 8. Implementation Checklist

### Phase 1: Core Infrastructure
- [ ] Settlement Coordinator state machine (Rust)
- [ ] Event store (append-only, event-sourced)
- [ ] Message bus (Kafka/NATS)

### Phase 2: Chain Adapters
- [ ] BTC Adapter (HTLC scripts, UTXO selection)
- [ ] EVM Adapter (escrow contracts, event watching)
- [ ] L2 Adapters (Arbitrum, Base, Polygon)

### Phase 3: Integration
- [ ] Wire to Comit Kernel
- [ ] Connect to existing 103-chain registry
- [ ] Custody/signing service interface

### Phase 4: UI/UX
- [ ] WebSocket event subscription
- [ ] Swap flow widget
- [ ] Settlement tracking view
- [ ] Observability dashboard

### Phase 5: Testing & Hardening
- [ ] Testnet BTC ↔ ARB swap
- [ ] Chaos testing (delayed confirmations, reorgs)
- [ ] Security audit (HTLC scripts, timeout handling)

---

*Architecture Document | X3 Chain Cross-Chain Settlement | December 2025*
