# X3 Chain Architecture Document

## Overview

X3 Chain is a Substrate-based L1 blockchain that runs two VM families inside a single runtime: an EVM-compatible execution environment (via pallet-evm/Frontier) and an SVM-style BPF execution environment (via solana-rbpf). The **X3 Kernel** orchestrates deterministic, atomic cross-VM operations and presents a unified account/fee model.

## Goals

- **Atomic cross-VM transactions** (ACID guarantees across VMs)
- **Unified account model** (single address usable in both VMs)
- **Unified economic model** (single fee token, gas ↔ compute translation)
- **Deterministic routing** & deadlock-free account locking
- **High developer ergonomics**: EVM + SVM toolchains, single RPC surface

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         X3 Chain                                │
│  ┌─────────────────┐   ┌─────────────────────────────────────────┐  │
│  │ Client/Wallets  │──▶│ Node RPC Layer (eth_/svm_/atlasKernel_) │  │
│  └─────────────────┘   └─────────────────────────────────────────┘  │
│                                    │                                │
│                                    ▼                                │
│                        ┌──────────────────────┐                     │
│                        │    X3 Kernel      │◀── Governance       │
│                        │  (pallet_x3_kernel)                     │
│                        └──────────────────────┘                     │
│                       ╱│╲         │         ╲│╱                     │
│                        │          │          │                      │
│         ┌──────────────┘          │          └──────────────┐       │
│         ▼                         ▼                         ▼       │
│  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐   │
│  │   EVM Adapter   │   │   SVM Adapter   │   │ Native Pallets  │   │
│  │ (Frontier/EVM)  │   │ (solana-rbpf)   │   │ (balances, etc) │   │
│  └─────────────────┘   └─────────────────┘   └─────────────────┘   │
│         │                         │                         │       │
│         ▼                         ▼                         ▼       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Execution Engines (EVM VM) (SVM/BPF VM)        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                    │                                │
│                                    ▼                                │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │           State Storage (Substrate Trie / RocksDB)          │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component                  | Responsibility                                                                    |
| -------------------------- | --------------------------------------------------------------------------------- |
| **X3 Kernel**           | Orchestration, atomic contexts, account locking, Comit tracking, canonical ledger |
| **EVM Adapter**            | Gas-to-weight translation, EVM execution API via Frontier                         |
| **SVM Adapter**            | BPF loader, deterministic scheduling, compute unit tracking                       |
| **VmRouter**               | Deterministic routing (Blake2-based priority + markers)                           |
| **Unified Accounts Layer** | Maps EVM H160 & SVM pubkey → AccountId32                                          |
| **Fee & Economics Layer**  | Unified fees, reservation + distribution                                          |
| **RPC Layer**              | eth_, svm_, and atlasKernel_ endpoints                                            |

---

## Data Model & Formats

### TxEnvelope (SCALE-encoded)
```rust
pub enum TxEnvelope {
    Evm(EvmTx),
    Svm(SvmTx),
    Native(SubstrateCall),
    Comit(ComitBundle),  // Cross-VM atomic bundle
}
```

### Comit Transaction
```rust
pub struct Comit<AccountId, Balance> {
    pub comit_id: H256,           // Globally unique identifier
    pub origin: AccountId,         // Submitting account
    pub evm_payload: Vec<u8>,      // EVM execution data
    pub svm_payload: Vec<u8>,      // SVM execution data  
    pub nonce: u64,                // Sequential per-account nonce
    pub fee: Balance,              // Reserved fee
    pub prepare_root: H256,        // Dual-VM prepare phase commitment
}
```

### Execution Receipt
```rust
pub struct ExecutionReceipt {
    pub success: bool,
    pub gas_used: u64,
    pub return_data: Vec<u8>,
    pub logs: Vec<ExecutionLog>,
    pub state_changes: Vec<StateChange>,
}
```

---

## Transaction Flow

### Single-VM Transaction
```
1. Client submits TxEnvelope → Node RPC
2. VmRouter::classify determines VM target
3. Dispatch to appropriate adapter (EVM or SVM)
4. Charge fees via unified fee path
5. Execute and emit events
6. Commit state to storage
```

### Cross-VM Atomic Transaction (Comit)
```
1. Client submits Comit → AtlasKernel::submit_comit
2. Validation: payload sizes, nonce, authorization
3. PREPARE PHASE:
   - Reserve fees from origin account
   - Lock affected accounts (deadlock prevention)
   - Record operation in PendingComits
4. EXECUTE PHASE:
   - Call EVM adapter with evm_payload
   - Call SVM adapter with svm_payload
   - Collect receipts and state changes
5. VERIFY PHASE:
   - Verify prepare_root against execution inputs
   - Check both receipts indicate success
6. FINALIZE PHASE (on success):
   - Commit state changes to canonical ledger
   - Release account locks
   - Distribute fees
   - Emit ComitFinalized event
7. ROLLBACK PHASE (on failure):
   - Revert all state changes (Substrate automatic)
   - Release locks
   - Refund reserved fees
   - Emit ComitFailed event
```

---

## Cross-VM Atomicity: Invariants & Protocol

### Guarantees
- **Atomicity**: Both VM operations succeed or both fail
- **Consistency**: Canonical ledger always reflects valid state
- **Isolation**: Account locks prevent concurrent modification
- **Durability**: GRANDPA finality ensures persistence

### Prepare Phase
- Fee reservation (prevents fee griefing)
- Record operation in PendingComits storage
- Acquire account locks in deterministic order (prevents deadlock)

### Execute Phase
- Call adapters sequentially (EVM first, then SVM)
- Capture state diffs and execution receipts
- Any failure triggers immediate rollback

### Finalize Phase
- Commit state snapshots to main storage
- Release all account locks
- Distribute fees according to governance rules
- Emit lifecycle events

### Rollback Phase
- Revert temporary state changes
- Release locks
- Refund reserved fees (minus base cost)
- Emit failure event with reason

---

## Fee Model & Economics

### Primary Currency
**X3** - Native token for all fee payments

### Fee Structure
```
Total Fee = Base Fee + (EVM Gas × Gas Price) + (SVM Compute Units × Unit Price)
```

### Fee Distribution (Configurable)
| Recipient         | Percentage |
| ----------------- | ---------- |
| Validators        | 60%        |
| Treasury          | 20%        |
| Burn              | 15%        |
| Kernel Operations | 5%         |

---

## Security Model & Attack Surfaces

| Attack Vector          | Mitigation                                       |
| ---------------------- | ------------------------------------------------ |
| Reentrancy across VMs  | Account locking per atomic context               |
| Fee griefing           | Fee reservation + minimum thresholds             |
| Denial of Service      | Weight accounting, RPC rate limiting             |
| Consensus disagreement | Deterministic routing, on-chain constants        |
| WASM mismatches        | Strict toolchain pinning, CI reproducible builds |

---

## API Reference

### X3 Kernel RPC
- `atlasKernel_getCanonicalBalance(account, asset_id)` → Balance
- `atlasKernel_getAssetMetadata(asset_id)` → AssetMetadata
- `atlasKernel_isAuthorized(account)` → bool
- `atlasKernel_getNonce(account)` → u64

### EVM (Frontier Compatible)
- `eth_sendRawTransaction`, `eth_call`, `eth_estimateGas`
- `eth_getBalance`, `eth_getTransactionReceipt`

### SVM
- `svm_sendTransaction`, `svm_simulate`, `svm_getAccountInfo`

---

## File Structure

```
/x3-chain
├── /runtime                    # Substrate runtime crate
├── /node                       # Node binary, RPC wiring
├── /pallets
│   └── x3-kernel/           # Core orchestration pallet
│       └── src/
│           ├── lib.rs          # Pallet definition
│           ├── adapters.rs     # VM adapter traits
│           ├── authority.rs    # Authority management
│           └── tests.rs        # Unit tests
├── /crates
│   ├── evm-integration/        # EVM adapter (Frontier)
│   └── svm-integration/        # SVM adapter (solana-rbpf)
├── /docs                       # Documentation
└── /tools                      # CLI tools
```

---

## References

- [Substrate Documentation](https://docs.substrate.io/)
- [Frontier (EVM Compatibility)](https://github.com/polkadot-evm/frontier)
- [solana-rbpf](https://github.com/solana-labs/rbpf)
