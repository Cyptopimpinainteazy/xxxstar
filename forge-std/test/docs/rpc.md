# X3 Chain RPC API Reference

X3 Chain provides comprehensive RPC interfaces for both EVM and SVM operations. This guide covers the JSON-RPC endpoints, WebSocket subscriptions, and cross-VM functionality.

## Quick Reference

| Interface | Port | Protocol | Purpose |
|-----------|------|----------|---------|
| **EVM RPC** | 9933 | HTTP/JSON-RPC | Ethereum-compatible API |
| **SVM RPC** | 9934 | HTTP/JSON-RPC | Solana-compatible API |
| **WebSocket** | 9944 | WS | Real-time subscriptions |
| **Cross-VM** | 9933 | HTTP/JSON-RPC | Unified dual-VM API |

**Why this matters**: X3 Chain maintains compatibility with existing Ethereum and Solana tooling while providing enhanced cross-VM capabilities.

## EVM RPC API

The EVM RPC interface follows the Ethereum JSON-RPC specification with X3 Chain extensions.

### Connection Examples

```bash
# HTTP endpoint
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# WebSocket endpoint
wscat -c ws://localhost:9944
```

### Standard Ethereum Methods

#### `eth_blockNumber`
Get the latest block number.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "eth_blockNumber",
  "params": [],
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "0x1234",
  "id": 1
}
```

#### `eth_getBalance`
Get account balance for a specific address.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "eth_getBalance",
  "params": [
    "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
    "latest"
  ],
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "0x2386f26fc10000", // 1000000000000000 wei = 1 X3
  "id": 1
}
```

#### `eth_call`
Execute a message call without creating a transaction.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "eth_call",
  "params": [
    {
      "to": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
      "data": "0x70a082310000000000000000000000000000000000000000000000000000000000000001"
    },
    "latest"
  ],
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "0x0000000000000000000000000000000000000000000000000000000000000123",
  "id": 1
}
```

#### `eth_sendTransaction`
Send a transaction to the network.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "eth_sendTransaction",
  "params": [
    {
      "from": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
      "to": "0x892f5fA0F8B7E8B9c0D1E2F3A4B5C6D7E8F9A0B",
      "value": "0x2386f26fc10000",
      "data": "0x70a082310000000000000000000000000000000000000000000000000000000000000001",
      "gas": "0x5208",
      "gasPrice": "0x3b9aca00"
    }
  ],
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
  "id": 1
}
```

### X3 Chain Extensions

#### `x3_getCanonicalBalance`
Get unified balance across EVM and SVM.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "x3_getCanonicalBalance",
  "params": [
    "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
    "0x0", // Asset ID for X3
    "latest"
  ],
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "evm": "0x2386f26fc10000",
    "svm": "0x1199999999999",
    "total": "0x347acf2ec9999",
    "asset_id": "0x0",
    "last_updated": 1672531200
  },
  "id": 1
}
```

#### `x3_submitCrossVmTransaction`
Submit atomic cross-VM transaction.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "x3_submitCrossVmTransaction",
  "params": [
    {
      "evm_payload": {
        "to": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
        "data": "0x70a082310000000000000000000000000000000000000000000000000000000000000001"
      },
      "svm_payload": {
        "program_id": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
        "accounts": [
          "user_pubkey_here",
          "token_account_pubkey_here"
        ],
        "instruction_data": "base64_encoded_instruction_data"
      },
      "atomic": true,
      "max_gas": "0x1e8480",
      "max_compute_units": "200000"
    }
  ],
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "transaction_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "block_number": "0x12345",
    "status": "success",
    "evm_receipt": {
      "status": "0x1",
      "gas_used": "0x5208",
      "logs": [
        {
          "address": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
          "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
          "data": "0x000000000000000000000000742d35cc6bf4e8b5e2c1c7d1a3e9c4f8d5a2b1c300000000000000000000000000000000000000000000000000000000000000064"
        }
      ]
    },
    "svm_receipt": {
      "err": null,
      "logs": ["Program log: Trade executed successfully"],
      "accounts": [],
      "units_consumed": 75000
    }
  },
  "id": 1
}
```

## SVM RPC API

The SVM RPC interface provides Solana-compatible functionality with X3 Chain enhancements.

### Connection Examples

```bash
# HTTP endpoint
curl -X POST http://localhost:9934 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# WebSocket endpoint
wscat -c ws://localhost:9944/svm
```

### Standard Solana Methods

#### `getHealth`
Check if the RPC node is healthy.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getHealth"
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "ok",
  "id": 1
}
```

#### `getAccountInfo`
Get account information.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getAccountInfo",
  "params": [
    "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
    {
      "encoding": "jsonParsed"
    }
  ]
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "value": {
      "lamports": 1000000000000,
      "data": {
        "parsed": {
          "info": {
            "name": "Token Program",
            "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
          },
          "type": "program"
        },
        "program": "spl-token",
        "space": 0
      },
      "owner": "11111111111111111111111111111111",
      "executable": true,
      "rentEpoch": 18446744073709551615
    }
  },
  "id": 1
}
```

#### `getProgramAccounts`
Get program accounts.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getProgramAccounts",
  "params": [
    "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
    {
      "encoding": "jsonParsed",
      "filters": [
        {
          "memcmp": {
            "offset": 0,
            "bytes": "base64_encoded_filter_data"
          }
        }
      ]
    }
  ]
}
```

#### `sendTransaction`
Send a transaction.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "sendTransaction",
  "params": [
    "base64_encoded_transaction",
    {
      "encoding": "base64",
      "skipPreflight": false,
      "preflightCommitment": "processed"
    }
  ]
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "5abc123def456",
  "id": 1
}
```

### X3 Chain SVM Extensions

#### `x3_getCanonicalAccount`
Get unified account information across EVM and SVM.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "x3_getCanonicalAccount",
  "params": [
    "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
  ]
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "evm_address": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
    "svm_pubkey": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
    "balances": {
      "x3": {
        "evm": "1000000000000",
        "svm": "500000000000",
        "total": "1500000000000"
      }
    },
    "nonce": 42,
    "last_updated": 1672531200
  },
  "id": 1
}
```

## WebSocket Subscriptions

WebSocket subscriptions provide real-time updates for both EVM and SVM events.

### Connection

```bash
# Connect to WebSocket
wscat -c ws://localhost:9944

# Subscribe to EVM logs
wscat -c ws://localhost:9944/evm
```

### EVM Subscriptions

#### `eth_subscribe` - New Headed Blocks

**Request:**
```json
{
  "id": 1,
  "method": "eth_subscribe",
  "params": ["newHeads"]
}
```

**Response:**
```json
{
  "id": 1,
  "result": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**Updates:**
```json
{
  "jsonrpc": "2.0",
  "method": "eth_subscription",
  "params": {
    "subscription": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "result": {
      "parentHash": "0x...",
      "sha3Uncles": "0x...",
      "miner": "0x...",
      "stateRoot": "0x...",
      "receiptsRoot": "0x...",
      "number": "0x12345",
      "gasUsed": "0x...",
      "timestamp": "0x1672531200"
    }
  }
}
```

#### `eth_subscribe` - Transaction Receipts

**Request:**
```json
{
  "id": 2,
  "method": "eth_subscribe",
  "params": [
    "alchemy_newFullPendingTransactions",
    {
      "address": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3"
    }
  ]
}
```

**Updates:**
```json
{
  "jsonrpc": "2.0",
  "method": "eth_subscription",
  "params": {
    "subscription": "0x9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba",
    "result": {
      "hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "from": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
      "to": "0x892f5fA0F8B7E8B9c0D1E2F3A4B5C6D7E8F9A0B",
      "gasUsed": "0x5208",
      "status": "0x1"
    }
  }
}
```

#### X3 Chain Cross-VM Events

**Request:**
```json
{
  "id": 3,
  "method": "x3_subscribeCrossVmEvents",
  "params": [
    {
      "address": "0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3",
      "program_id": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
    }
  ]
}
```

**Updates:**
```json
{
  "jsonrpc": "2.0",
  "method": "x3_crossVmEvent",
  "params": {
    "subscription": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "result": {
      "transaction_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "block_number": 46647,
      "timestamp": 1672531200,
      "evm_receipt": {
        "status": "success",
        "events": ["CrossVmCallExecuted", "AssetTransferred"]
      },
      "svm_receipt": {
        "status": "success",
        "logs": ["Trade executed", "Fees collected"]
      },
      "canonical_state": {
        "total_atlas": "1500000000000",
        "gas_used": 150000,
        "compute_units": 75000
      }
    }
  }
}
```

### SVM Subscriptions

#### Account Subscriptions

**Request:**
```json
{
  "id": 4,
  "method": "x3_svm_subscribeAccount",
  "params": [
    "7xKXtg2CW87d97TXJSDpbD5
