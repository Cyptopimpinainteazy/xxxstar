# Phase 2: Tauri Desktop Core Integration

## Overview

This document describes the Phase 2 implementation for the Tauri Desktop Core, which wires together IPC, Substrate hooks, wallet store, and x3ChainService components.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Tauri Desktop Core (Frontend)                    │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────┐ │
│  │ useSubstrate │  │ useWalletStore│ │ useX3Chain  │  │  IPC    │ │
│  │   Hook       │  │              │ │  Service    │  │Service  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ Tauri Commands
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Tauri Backend (Rust)                             │
├─────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    wallet_core Module                         │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐ │  │
│  │  │ substrate_hook │  │ wallet_store │  │   x3_chain_service│ │  │
│  │  │              │  │              │  │                     │ │  │
│  │  │ - Event      │  │ - Encrypted  │  │ - Chain queries   │ │  │
│  │  │   subscriptions│ │   storage    │  │ - Transaction exec│ │  │
│  │  │ - Hook reg   │  │ - Recovery   │  │ - Caching         │ │  │
│  │  │   & exec     │  │ - Multi-chain│  │ - Config support  │ │  │
│  │  └──────────────┘  └──────────────┘  └─────────────────────┘ │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                              │                                       │
│                              │ Tauri Commands                        │
│                              ▼                                       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    wallet.rs (Commands)                       │  │
│  │  - Substrate hooks (subscribe, state)                         │  │
│  │  - Wallet store (store, retrieve, delete, backup)             │  │
│  │  - Chain service (query, submit, status)                      │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ JSON-RPC / WebSocket
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    X3 Chain Node                                    │
├─────────────────────────────────────────────────────────────────────┤
│  - Block queries                                                    │
│  - Account state                                                    │
│  - Balance queries                                                  │
│  - Extrinsic submission                                             │
│  - Event subscriptions                                              │
└─────────────────────────────────────────────────────────────────────┘
```

## Components

### 1. IPC Communication

**Location:** `apps/x3-desktop/src-tauri/src/wallet_core/ipc.rs`

The IPC module defines the message types for communication between frontend and backend:

- `IntentDraft` - Cross-chain intent structure
- `AssetRequirement` - Asset requirements for intents
- `FeeCap` - Fee limits for transactions
- `Attestation` - Verifier attestation for intents

**Frontend Integration:**
```typescript
import { invoke } from '@tauri-apps/api/core';

// Submit an intent
const result = await invoke<string>('run_cross_chain_intent', {
  draft: intentDraft,
});
```

### 2. Substrate Hooks

**Location:** `apps/x3-desktop/src-tauri/src/wallet_core/substrate_hook.rs`

The substrate hook module provides:

- Event subscriptions (new blocks, extrinsics, chain reorgs)
- Hook registration and execution
- Error handling and retry logic

**Backend Commands:**
```rust
#[command]
pub async fn subscribe_substrate_events() -> Result<String, String> {
    // Subscribe to Substrate chain events
}

#[command]
pub async fn get_substrate_hook_state() -> Result<String, String> {
    // Get the current state of substrate hooks
}
```

**Frontend Hook:**
```typescript
import { useSubstrateHook } from '@/hooks/useSubstrateHook';

function MyComponent() {
  const { state, events, registerHook } = useSubstrateHook();
  
  // Register a hook for new blocks
  useEffect(() => {
    registerHook('newBlocks', (event) => {
      console.log('New block:', event);
    });
  }, [registerHook]);
  
  return (
    <div>
      <p>Connected: {state.connected ? 'Yes' : 'No'}</p>
      <p>Last Block: {state.lastBlockNumber}</p>
    </div>
  );
}
```

### 3. Wallet Store

**Location:** `apps/x3-desktop/src-tauri/src/wallet_core/wallet_store.rs`

The wallet store module provides:

- Encrypted wallet storage
- Wallet recovery functionality
- Multi-chain wallet support

**Backend Commands:**
```rust
#[command]
pub async fn store_wallet_encrypted(
    wallet_id: String,
    mnemonic: String,
    seed: String,
    derivation_path: String,
) -> Result<(), String> {
    // Store wallet with encryption
}

#[command]
pub async fn retrieve_wallet_encrypted(wallet_id: String) -> Result<String, String> {
    // Retrieve and decrypt wallet data
}

#[command]
pub async fn delete_wallet(wallet_id: String) -> Result<(), String> {
    // Delete a wallet from storage
}

#[command]
pub async fn export_wallet_backup(wallet_id: String) -> Result<String, String> {
    // Export wallet for recovery
}

#[command]
pub async fn import_wallet_backup(backup: String) -> Result<String, String> {
    // Import wallet from backup
}
```

**Frontend Hook:**
```typescript
import { useWalletStore } from '@/hooks/useWalletStore';

function WalletManager() {
  const { storeWallet, retrieveWallet, deleteWallet, exportBackup } = useWalletStore();
  
  const handleStoreWallet = async () => {
    await storeWallet(
      'my_wallet',
      'mnemonic words...',
      'seed hex...',
      "m/44'/60'/0'/0/0"
    );
  };
  
  return (
    <div>
      <button onClick={handleStoreWallet}>Store Wallet</button>
    </div>
  );
}
```

### 4. x3ChainService

**Location:** `apps/x3-desktop/src-tauri/src/wallet_core/x3_chain_service.rs`

The x3ChainService module provides:

- Chain operation methods (queries, transactions, subscriptions)
- Error handling with detailed error types
- Caching mechanism for performance
- Configuration support

**Backend Commands:**
```rust
#[command]
pub async fn query_block(block_number: Option<u64>, block_hash: Option<String>) -> Result<String, String> {
    // Query block data from X3 chain
}

#[command]
pub async fn query_account(address: String, at_block: Option<u64>) -> Result<String, String> {
    // Query account data from X3 chain
}

#[command]
pub async fn query_balance(address: String, asset_id: Option<String>) -> Result<String, String> {
    // Query account balance from X3 chain
}

#[command]
pub async fn submit_extrinsic(call: String, signer: String, nonce: Option<u64>, tip: Option<u64>) -> Result<String, String> {
    // Submit an extrinsic to X3 chain
}

#[command]
pub async fn get_connection_status() -> Result<String, String> {
    // Get the connection status of x3ChainService
}

#[command]
pub async fn clear_chain_cache() -> Result<(), String> {
    // Clear the chain operation cache
}
```

**Frontend Service:**
```typescript
import { useX3ChainService } from '@/services/x3ChainServiceIntegration';

function ChainViewer() {
  const { service, connectionStatus, queryBlock, queryBalance } = useX3ChainService();
  
  const handleQueryBlock = async () => {
    const result = await queryBlock(12345);
    console.log('Block data:', result.data);
  };
  
  return (
    <div>
      <p>Connected: {connectionStatus.connected ? 'Yes' : 'No'}</p>
      <p>Block Number: {connectionStatus.blockNumber}</p>
      <button onClick={handleQueryBlock}>Query Block</button>
    </div>
  );
}
```

## Integration Flow

### 1. Intent Processing Flow

```
User creates intent → IPC sends to backend → Coordinator validates
→ Verifier quorum checks → Attestation generated → Intent executed
```

### 2. Wallet Storage Flow

```
User creates/import wallet → Wallet store encrypts → Data persisted
→ Wallet ID returned → Can be retrieved/backup/exported
```

### 3. Chain Query Flow

```
Frontend requests data → x3ChainService checks cache → RPC call if needed
→ Data cached → Response returned
```

## Configuration

### Backend Configuration

The backend configuration is defined in `X3ChainServiceConfig`:

```rust
pub struct X3ChainServiceConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub timeout_ms: u64,
    pub cache_ttl_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}
```

### Frontend Configuration

The frontend service can be configured:

```typescript
const { service, connectionStatus } = useX3ChainService({
  rpcUrl: 'http://127.0.0.1:9933',
  wsUrl: 'ws://127.0.0.1:9944',
  timeoutMs: 30000,
  cacheTtlMs: 60000,
});
```

## Testing

### Unit Tests

Run the unit tests:

```bash
cd apps/x3-desktop/src-tauri
cargo test --lib wallet_core
```

### Integration Tests

Run the integration tests:

```bash
cargo test --test phase2_integration
```

## Security Considerations

1. **Wallet Encryption**: All sensitive data (mnemonics, seeds) are encrypted before storage
2. **IPC Security**: Tauri's command system provides secure message passing
3. **Substrate Hook Isolation**: Hooks are isolated from direct RPC access
4. **Cache Security**: Chain data is cached locally but not sensitive information

## Error Handling

All components implement comprehensive error handling:

- RPC errors with retry logic
- Serialization/deserialization errors
- Network timeout errors
- Invalid operation errors

## Future Enhancements

1. Add support for hardware wallet integration
2. Implement multi-signature wallet support
3. Add transaction signing capabilities
4. Implement advanced caching strategies
5. Add support for additional blockchain networks

## Troubleshooting

### Common Issues

1. **Connection Failed**: Ensure X3 chain node is running and accessible
2. **Wallet Not Found**: Verify wallet ID and storage path
3. **Cache Stale**: Clear cache and retry query

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

Frontend logging:

```typescript
console.log('[useSubstrateHook] State:', state);
console.log('[useWalletStore] Wallets:', wallets);
console.log('[useX3ChainService] Status:', connectionStatus);
```

## References

- [Tauri Documentation](https://tauri.app/)
- [Substrate Documentation](https://substrate.dev/)
- [Polkadot JS API](https://polkadot.js.org/)
