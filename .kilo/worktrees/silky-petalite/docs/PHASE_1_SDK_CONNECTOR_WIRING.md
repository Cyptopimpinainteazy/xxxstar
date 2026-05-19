# Phase 1: SDK & Connector Layer Wiring

This document describes the implementation of Phase 1 of the Off-Chain Assets Audit & Blockchain Integration epic, focusing on wiring the TypeScript SDK, Python SDK, and blockchain connector to live node endpoints.

## Overview

Phase 1 implements the following:

1. **TypeScript SDK Integration** - Wire `ts-sdk` to live node infrastructure
2. **Python SDK Integration** - Wire `py-sdk` to live node infrastructure
3. **Blockchain Connector Wiring** - Connect `blockchain-connector` to live node endpoints

## Implementation Details

### 1. TypeScript SDK Integration

#### Files Modified/Created

| File | Description |
|------|-------------|
| `packages/polkawallet-plugin/src/config/env.ts` | **NEW** - Environment configuration with network endpoints |
| `packages/polkawallet-plugin/src/core/api.ts` | **MODIFIED** - Enhanced with retry logic and reconnection |
| `packages/polkawallet-plugin/src/index.ts` | **MODIFIED** - Export new API functions |

#### Key Features

- **Environment Variable Support**: Configure SDK via environment variables
- **Automatic Reconnection**: Exponential backoff reconnection logic
- **Retry Logic**: Built-in retry for failed requests
- **Network Configuration**: Support for mainnet, testnet, and local networks

#### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `X3_RPC_ENDPOINT` | Custom WebSocket endpoint | Network-specific |
| `X3_NETWORK` | Network: mainnet/testnet/local | `local` |
| `X3_AUTO_RECONNECT` | Enable auto-reconnect | `true` |
| `X3_RECONNECT_MAX` | Maximum reconnect attempts | `5` |
| `X3_RECONNECT_DELAY` | Reconnect delay in ms | `1000` |
| `X3_TIMEOUT` | Request timeout in ms | `30000` |
| `X3_DEBUG` | Enable debug logging | `false` |

#### Usage Examples

```typescript
// Connect to mainnet using environment configuration
import { createX3ApiFromEnv } from '@x3/polkawallet-plugin';

const api = await createX3ApiFromEnv();
await api.connect();

// Connect to specific network
import { createX3Api } from '@x3/polkawallet-plugin';

const api = await createX3Api({
  network: 'mainnet',
  autoReconnect: true,
  reconnectMaxAttempts: 5,
  reconnectDelay: 1000,
});

// Execute with retry
const result = await api.executeWithRetry(
  () => api.api.query.system.account(accountId),
  3, // max retries
  1000 // initial delay
);
```

### 2. Python SDK Integration

#### Files Modified/Created

| File | Description |
|------|-------------|
| `packages/py-sdk/src/atlas_sphere_sdk/config.py` | **NEW** - Environment configuration |
| `packages/py-sdk/src/atlas_sphere_sdk/client.py` | **MODIFIED** - Enhanced with retry logic |
| `packages/py-sdk/src/atlas_sphere_sdk/__init__.py` | **MODIFIED** - Export new modules |

#### Key Features

- **Environment Variable Support**: Configure SDK via environment variables
- **Automatic Reconnection**: Async reconnection with exponential backoff
- **Retry Logic**: Built-in retry for both sync and async operations
- **Connection State Management**: Track connection status and handle disconnections

#### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `X3_RPC_ENDPOINT` | Custom WebSocket endpoint | Network-specific |
| `X3_NETWORK` | Network: mainnet/testnet/local | `local` |
| `X3_AUTO_RECONNECT` | Enable auto-reconnect | `true` |
| `X3_RECONNECT_MAX` | Maximum reconnect attempts | `5` |
| `X3_RECONNECT_DELAY` | Reconnect delay in ms | `1000` |
| `X3_TIMEOUT` | Request timeout in ms | `30000` |
| `X3_DEBUG` | Enable debug logging | `false` |

#### Usage Examples

```python
from atlas_sphere_sdk import AtlasClient, sdk_config

# Connect using environment configuration
client = AtlasClient()
client.connect()

# Connect to specific network
client = AtlasClient(url="wss://rpc.x3chain.io:9944")
client.connect()

# Execute with retry
result = client.execute_with_retry(
    lambda: client.get_chain_info(),
    max_retries=3,
    delay=1000
)

# Async execution with retry
async def async_example():
    result = await client.execute_with_retry_async(
        lambda: client.get_account_info(address),
        max_retries=3,
        delay=1000
    )
```

### 3. Blockchain Connector Wiring

#### Files Modified/Created

| File | Description |
|------|-------------|
| `crates/cross-vm-bridge/src/connector.rs` | **NEW** - Live node RPC connector |

#### Key Features

- **Live Node Dispatcher**: Connects to X3 Chain RPC endpoints
- **Network Configuration**: Support for mainnet, testnet, and local
- **Reconnection Logic**: Automatic reconnection with exponential backoff
- **Cross-VM Execution**: Execute EVM, SVM, and X3VM operations

#### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `X3_RPC_ENDPOINT` | WebSocket endpoint | `wss://rpc.x3chain.io:9944` |
| `X3_NETWORK` | Network: mainnet/testnet/local | `local` |
| `X3_TIMEOUT` | Request timeout in ms | `30000` |
| `X3_RECONNECT_MAX` | Maximum reconnect attempts | `5` |
| `X3_RECONNECT_DELAY` | Reconnect delay in ms | `1000` |

#### Usage Examples

```rust
use cross_vm_bridge::connector::{
    LiveNodeDispatcher, LiveNodeConfig, create_dispatcher_for_network
};

// Create dispatcher with default configuration
let dispatcher = LiveNodeDispatcher::default();

// Create dispatcher for specific network
let dispatcher = LiveNodeDispatcher::for_network("mainnet");

// Create dispatcher with custom configuration
let config = LiveNodeConfig {
    endpoint: "wss://rpc.x3chain.io:9944".to_string(),
    timeout_ms: 30000,
    reconnect_max_attempts: 5,
    reconnect_delay_ms: 1000,
};
let mut dispatcher = LiveNodeDispatcher::new(config);

// Connect to the live node
dispatcher.connect()?;

// Execute cross-VM operations
let result = dispatcher.execute_evm_tx(
    &caller,
    &target,
    &input,
    value,
)?;
```

## Network Endpoints

### Default Endpoints

| Network | WebSocket Endpoint |
|---------|-------------------|
| Mainnet | `wss://rpc.x3chain.io:9944` |
| Testnet | `wss://testnet.x3chain.io:9944` |
| Local | `ws://127.0.0.1:9944` |

### Configuration via Environment

```bash
# Mainnet
export X3_NETWORK=mainnet
export X3_RPC_ENDPOINT=wss://rpc.x3chain.io:9944

# Testnet
export X3_NETWORK=testnet
export X3_RPC_ENDPOINT=wss://testnet.x3chain.io:9944

# Local
export X3_NETWORK=local
export X3_RPC_ENDPOINT=ws://127.0.0.1:9944
```

## Error Handling

### TypeScript SDK

- **Connection Errors**: Automatically handled with reconnection attempts
- **Request Failures**: Retry logic with exponential backoff
- **Timeout Errors**: Configurable via `X3_TIMEOUT` environment variable

### Python SDK

- **Connection Errors**: Async reconnection with exponential backoff
- **Request Failures**: Retry logic with configurable attempts
- **Timeout Errors**: Handled by SubstrateInterface

### Blockchain Connector

- **Connection Errors**: Reconnection attempts with exponential backoff
- **Execution Errors**: Proper error propagation via `DispatchError`
- **Timeout Errors**: Configurable via `X3_TIMEOUT` environment variable

## Testing

### TypeScript SDK Testing

```bash
# Test with local node
npm test

# Test with testnet
X3_NETWORK=testnet npm test

# Test with mainnet
X3_NETWORK=mainnet npm test
```

### Python SDK Testing

```bash
# Test with local node
pytest

# Test with testnet
X3_NETWORK=testnet pytest

# Test with mainnet
X3_NETWORK=mainnet pytest
```

### Blockchain Connector Testing

```bash
# Test with default configuration
cargo test

# Test with custom endpoint
X3_RPC_ENDPOINT=ws://127.0.0.1:9944 cargo test
```

## Deployment

### TypeScript SDK

1. Build the SDK:
   ```bash
   npm run build
   ```

2. Deploy to production:
   ```bash
   npm publish
   ```

### Python SDK

1. Build the SDK:
   ```bash
   python setup.py sdist bdist_wheel
   ```

2. Deploy to production:
   ```bash
   twine upload dist/*
   ```

### Blockchain Connector

1. Build the crate:
   ```bash
   cargo build --release
   ```

2. Deploy to production:
   ```bash
   # Include in the node build
   ```

## Monitoring

### Connection Health

Monitor the following metrics:

- **Connection Status**: Track connection state (connected/disconnected)
- **Reconnection Attempts**: Count of reconnection attempts
- **Request Latency**: Measure request processing time
- **Error Rates**: Track error rates for different operations

### Logging

Enable debug logging via environment variable:

```bash
# TypeScript
export X3_DEBUG=true

# Python
export X3_DEBUG=true

# Rust
export RUST_LOG=debug
```

## Troubleshooting

### Common Issues

1. **Connection Refused**
   - Verify the RPC endpoint is correct
   - Check network connectivity
   - Ensure the node is running

2. **Authentication Errors**
   - Verify the account has proper permissions
   - Check the keypair configuration

3. **Timeout Errors**
   - Increase the timeout value via `X3_TIMEOUT`
   - Check network latency
   - Verify the node is not overloaded

## Future Enhancements

1. **Load Balancing**: Support multiple endpoints with load balancing
2. **Circuit Breaker**: Implement circuit breaker pattern for failed connections
3. **Metrics Export**: Export connection metrics to Prometheus
4. **Health Checks**: Implement health check endpoints
5. **Fallback Endpoints**: Support multiple fallback endpoints

## References

- [TypeScript SDK Documentation](../README.md)
- [Python SDK Documentation](../README.md)
- [Blockchain Connector Documentation](../README.md)
- [X3 Chain RPC API](https://docs.x3chain.io)
