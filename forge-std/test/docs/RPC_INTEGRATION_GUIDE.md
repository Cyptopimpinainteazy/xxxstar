# Frontier RPC Integration Guide

## Status

⚠️ **Developer Preview** - Frontier dependency resolution blocked  
RPC wiring is architecturally complete and ready to integrate once Frontier dependencies are resolved.

## Current Implementation

### Already Implemented ✅

The X3 Kernel pallet provides these runtime APIs via `AtlasKernelRuntimeApi`:

- `get_canonical_balance(account, asset_id)` - Query canonical ledger balances
- `get_asset_metadata(asset_id)` - Query asset symbol and decimals
- `is_authorized(account)` - Check if account is authorized for Comits
- `get_authorized_accounts()` - List all authorized accounts
- `get_authorities()` - Get current authority set

### Future Enhancements 🔮

The sections below describe future EVM-specific RPC endpoints that will be added
when Frontier integration is complete. These are design documents, not current implementations.

## Overview

This guide documents how to wire the X3 Chain canonical ledger to Frontier's EVM RPC endpoints, enabling MetaMask and Hardhat compatibility.

## Canonical Ledger Status

- ✅ Canonical ledger schema supports EVM address queries
- ✅ DualVmDispatcher trait includes `canonical_ledger_update()` for state persistence
- ✅ Cross-VM bridge validates and merges EVM state changes
- ⏳ Frontier RPC layers not yet wired to kernel queries

## Architecture

### Query Flow: eth_call → Canonical Ledger

```
Frontier JSON-RPC
    ↓
fc-rpc (Frontier RPC)
    ↓
eth_call handler
    ↓
x3-kernel query layer (to implement)
    ↓
CanonicalLedger StorageMap
    ↓
EVM Account Balance / Storage
```

### Key Endpoints to Wire

1. **eth_call** - Read-only contract calls
   - Maps to: `CanonicalLedger` query with EVM address (20 bytes)
   - Should return: Account balance + storage state

2. **eth_getBalance** - Account balance queries
   - Maps to: `CanonicalLedger<(account_id, asset_id)>`
   - Should return: u128 balance in wei

3. **eth_getCode** - Contract bytecode queries
   - Maps to: StateChange.value field for bytecode storage keys
   - Should return: Bytecode from canonical storage

4. **eth_getStorageAt** - Contract storage queries
   - Maps to: StateChange.value for specific keys
   - Should return: 32-byte storage values

5. **eth_sendTransaction** - Transaction submission
   - Maps to: `submit_comit` extrinsic via Frontier RPC
   - Should trigger: Dual-VM execution and state commit

## Implementation Steps

### 1. Create RPC Runtime API

**File: `pallets/x3-kernel/src/runtime_api.rs`** (new)

```rust
sp_api::decl_runtime_apis! {
    pub trait AtlasKernelApi {
        /// Query canonical ledger balance for an EVM address
        fn get_evm_balance(
            account: Vec<u8>,
            asset_id: u32,
        ) -> Option<u128>;

        /// Query contract bytecode
        fn get_evm_code(address: Vec<u8>) -> Vec<u8>;

        /// Query contract storage
        fn get_evm_storage(
            address: Vec<u8>,
            storage_key: H256,
        ) -> Option<H256>;
    }
}
```

### 2. Implement Runtime API in Runtime

**File: `runtime/src/lib.rs`** (add to RuntimeApi)

```rust
impl sp_api::impl_runtime_apis! {
    impl pallet_x3_kernel::runtime_api::AtlasKernelApi<Block> for Runtime {
        fn get_evm_balance(account: Vec<u8>, asset_id: u32) -> Option<u128> {
            // Convert EVM address (20 bytes) to AccountId
            let account_id = AccountId32::new([0u8; 32]);
            
            // Query canonical ledger
            pallet_x3_kernel::CanonicalLedger::<Runtime>::get(
                &account_id,
                &AssetId::from(asset_id),
            )
        }

        fn get_evm_code(address: Vec<u8>) -> Vec<u8> {
            // Query ContractCode storage keyed by EVM address
            // Return bytecode or empty vec if not found
            Vec::new() // Placeholder
        }

        fn get_evm_storage(
            address: Vec<u8>,
            storage_key: H256,
        ) -> Option<H256> {
            // Query StateChange storage for this address + key
            // Return storage value or None
            None // Placeholder
        }
    }
}
```

### 3. Wire RPC Handlers

**File: `node/src/rpc.rs`** (new RPC module)

```rust
use jsonrpc_core::{BoxFuture, Result as RpcResult};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;

pub struct EthApi<C, P> {
    client: Arc<C>,
    _marker: PhantomData<P>,
}

#[rpc]
pub trait EthApiServer {
    #[rpc(name = "eth_getBalance")]
    fn eth_get_balance(
        &self,
        address: String,
        block: String,
    ) -> BoxFuture<RpcResult<String>>;

    #[rpc(name = "eth_call")]
    fn eth_call(
        &self,
        tx: EthCallRequest,
        block: String,
    ) -> BoxFuture<RpcResult<String>>;
}

impl<C, P> EthApiServer for EthApi<C, P>
where
    C: ProvideRuntimeApi<P> + Send + Sync + 'static,
    C::Api: AtlasKernelApi<P>,
    P: sp_api::BlockT,
{
    fn eth_get_balance(
        &self,
        address: String,
        _block: String,
    ) -> BoxFuture<RpcResult<String>> {
        let api = self.client.runtime_api();
        
        // Parse EVM address from hex string (0x + 40 chars)
        let addr_bytes = hex::decode(&address[2..])
            .map_err(|e| RpcError::InvalidParams(e.to_string()))?;

        // Default asset is the native token (asset_id=0)
        let balance = api.get_evm_balance(addr_bytes, 0)
            .map_err(|_| RpcError::ServerError(-1, "Runtime error".into()))?;

        // Return balance in wei as hex string
        Ok(format!("0x{:x}", balance))
    }

    fn eth_call(
        &self,
        tx: EthCallRequest,
        _block: String,
    ) -> BoxFuture<RpcResult<String>> {
        // Parse to address, data, etc.
        let api = self.client.runtime_api();
        
        let addr_bytes = hex::decode(&tx.to[2..])
            .map_err(|e| RpcError::InvalidParams(e.to_string()))?;

        let code = api.get_evm_code(addr_bytes)
            .map_err(|_| RpcError::ServerError(-1, "Runtime error".into()))?;

        // Execute call on canonical state
        // Return encoded result
        Ok(format!("0x{}", hex::encode(&code)))
    }
}
```

### 4. Register in Node Service

**File: `node/src/service.rs`** (extend with EthApi)

```rust
pub fn create_full<C>(
    config: ServiceConfiguration,
    rpc_builder: impl Fn(Arc<FullClient>) -> jsonrpc_core::IoHandler<sp_io::io_handler::Metadata>,
) -> Result<TaskManager> {
    // ... existing code ...
    
    // Register X3 Kernel RPC handler
    let eth_api = EthApi::new(Arc::clone(&client));
    let io = rpc_builder(Arc::clone(&client));
    io.extend_with(EthApiServer::to_delegate(eth_api));
    
    Ok(task_manager)
}
```

### 5. Wire to Frontier (if dependencies resolved)

Once Frontier is available, wire through Frontier's JSON-RPC layer:

Note: An optional node feature `frontier` is provided to gate Frontier RPC wiring in `node/Cargo.toml`. When enabled, node RPC merges Frontier endpoints; use caution to pin Frontier versions compatible with the Substrate `rev` in the workspace (see root `Cargo.toml`).


```rust
// In frontier/src/rpc.rs integration
impl EthApiT for EthApi {
    fn eth_balance(
        &self,
        address: H160,
        number: Option<BlockNumber>,
    ) -> RpcFuture<U256> {
        // Delegate to x3-kernel RPC API
        let x3_addr = address.to_fixed_bytes().to_vec();
        self.x3_client.get_evm_balance(x3_addr, 0)
            .map(|balance| U256::from(balance))
            .into()
    }
}
```

## Integration Checklist

- [ ] Create `pallets/x3-kernel/src/runtime_api.rs`
- [ ] Add `AtlasKernelApi` trait to runtime
- [ ] Implement runtime_api in `runtime/src/lib.rs`
- [ ] Create `node/src/rpc.rs` with EthApi implementation
- [ ] Register EthApi in `node/src/service.rs`
- [ ] Resolve Frontier dependencies (version compatibility)
- [ ] Wire Frontier JSON-RPC handlers
- [ ] Test with MetaMask or Hardhat
- [ ] Add integration tests for RPC queries

## Testing

### Manual Testing

```bash
# Query balance via cURL
curl -X POST http://localhost:9945 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["0x1234567890123456789012345678901234567890","latest"],"id":1}'

# Expected response
{"jsonrpc":"2.0","result":"0x0","id":1}
```

### Integration Test

```rust
#[test]
fn test_eth_get_balance() {
    new_test_ext().execute_with(|| {
        // Set up canonical ledger balance
        CanonicalLedger::<Test>::insert(
            (ALICE_EVM_ADDR, NATIVE_ASSET_ID),
            1_000_000_000_000u128,
        );

        // Query via RPC
        let balance = EthApi::eth_get_balance(
            ALICE_EVM_ADDR.to_vec(),
            "latest".to_string(),
        );

        assert_eq!(balance, Some(1_000_000_000_000u128));
    });
}
```

## Dependencies to Install

```toml
# node/Cargo.toml
[dependencies]
jsonrpc-core = "18"
jsonrpc-derive = "18"
hex = "0.4"

# When Frontier is resolved:
frontier = { version = "1.0.0", features = ["rpc"] }
fc-rpc = "1.0.0"
fc-api = "1.0.0"
```

## Known Issues

1. **Frontier Dependencies**: Currently blocked on version compatibility with Polkadot v1.0.0
   - **Workaround**: Use stable v0.9.x or wait for Frontier v1.0 release

2. **EVM Address Mapping**: Converting 20-byte EVM addresses to 32-byte Substrate AccountId
   - **Solution**: Use h160_to_account32() utility function

3. **Storage Format**: ExecutionReceipt.state_changes stores minimal format
   - **Solution**: Deserialize and expand in RPC handler

## Future Work

- [ ] Add RPC caching for performance
- [ ] Implement `eth_sendRawTransaction` for transaction submission
- [ ] Add ERC-20 contract ABI support
- [ ] Implement full JSON-RPC v2.0 spec
- [ ] Add MetaMask provider for testnet

## References

- Frontier: https://github.com/paritytech/frontier
- JSON-RPC 2.0: https://www.jsonrpc.org/specification
- Ethereum JSON-RPC: https://ethereum.org/en/developers/docs/apis/json-rpc/
- Substrate Runtime APIs: https://docs.substrate.io/main-docs/build/runtime-storage/#runtime-storage-api
