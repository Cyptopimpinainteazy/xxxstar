# @x3-chain/ts-sdk

TypeScript SDK for X3 Chain - the dual-VM (EVM + SVM) Layer-1 blockchain enabling native interoperability between Ethereum and Solana execution environments.

## Features

- **Full Type Safety**: Comprehensive TypeScript types mirroring runtime structures
- **Dual-VM Support**: Native utilities for both EVM and SVM payload construction
- **Fluent Builder API**: Intuitive `ComitBuilder` for transaction construction
- **Efficient Queries**: Cached query client with batch operations
- **Event Subscriptions**: Real-time block and Comit event subscriptions
- **Comprehensive Errors**: Typed error hierarchy for precise error handling

## Installation

```bash
npm install @x3-chain/ts-sdk
# or
yarn add @x3-chain/ts-sdk
# or
pnpm add @x3-chain/ts-sdk
```

## Quick Start

```typescript
import {
  AtlasSphereClient,
  ComitBuilder,
  encodeTransfer,
  encodeSystemTransfer,
} from '@x3-chain/ts-sdk';

// Connect to X3 Chain node
const client = new AtlasSphereClient({
  endpoint: 'ws://localhost:9944',
});
await client.connect();

// Check chain info
const info = await client.getChainInfo();
console.log(`Connected to ${info.name} v${info.version}`);

// Build a dual-VM Comit
const comit = new ComitBuilder()
  // EVM: Transfer ERC20 tokens
  .withEvmPayload({
    to: '0x1234567890abcdef1234567890abcdef12345678',
    data: encodeTransfer('0xrecipient...', 1000000000000000000n),
    value: 0n,
  })
  // SVM: Transfer SOL
  .withSvmPayload({
    programId: '0x11111111111111111111111111111111',
    data: encodeSystemTransfer(1000000000n),
  })
  // Auto-calculate fee based on gas/compute estimates
  .withFee('auto')
  .build();

// Submit and wait for finalization
const result = await client.submitComit(comit, signerAccount);
console.log(`Comit ${result.comit.comitId} finalized at block ${result.blockNumber}`);

// Disconnect
await client.disconnect();
```

## Core Concepts

### Comits

A **Comit** is the atomic unit of cross-VM execution in X3 Chain. It bundles EVM and SVM payloads for simultaneous execution with deterministic ordering.

```typescript
interface Comit {
  comitId: Hash;           // Unique identifier
  origin: AccountId;       // Submitting account
  evmPayload: Uint8Array;  // EVM transaction data
  svmPayload: Uint8Array;  // SVM instruction data
  nonce: bigint;           // Account nonce
  fee: bigint;             // Execution fee
  prepareRoot: Hash;       // Cryptographic commitment
}
```

### Building Comits

Use `ComitBuilder` for type-safe Comit construction:

```typescript
// EVM-only Comit
const evmComit = new ComitBuilder()
  .withEvmPayload({
    to: contractAddress,
    data: calldata,
    value: 0n,
  })
  .withEvmGasLimit(500000n)
  .withFee('auto')
  .build();

// SVM-only Comit
const svmComit = new ComitBuilder()
  .withSvmPayload({
    programId: programId,
    data: instructionData,
  })
  .withSvmComputeUnits(200000n)
  .withFee('auto')
  .build();

// Dual-VM Comit (both EVM and SVM)
const dualComit = new ComitBuilder()
  .withEvmPayload(evmData)
  .withSvmPayload(svmData)
  .withFee('auto')
  .build();

// Shorthand factories
import { evmComit, svmComit, dualComit } from '@x3-chain/ts-sdk';

const evm = evmComit({ to: '0x...', data: '0x...' }).withFee('auto').build();
const svm = svmComit({ programId: '0x...', data: '0x...' }).withFee('auto').build();
const dual = dualComit(evmPayload, svmPayload).withFee('auto').build();
```

### Querying State

```typescript
import { QueryClient, createQueryClient } from '@x3-chain/ts-sdk';

const query = createQueryClient(client.polkadotApi);

// Get canonical ledger balance
const balance = await query.getCanonicalBalance(account, assetId);

// Batch query multiple accounts
const balances = await query.getCanonicalBalances([account1, account2]);

// Get all balances for an account
const allBalances = await query.getAllBalances(account);

// Check authorization
const isAuth = await query.isAuthorized(account);

// Query at specific block
const oldBalance = await query.getCanonicalBalance(account, assetId, {
  at: blockHash,
});
```

### Subscriptions

```typescript
// Subscribe to new blocks
const blockSubId = await client.subscribeNewBlocks((blockNumber, blockHash) => {
  console.log(`New block: ${blockNumber}`);
});

// Subscribe to finalized blocks
const finalSubId = await client.subscribeFinalizedBlocks((blockNumber, blockHash) => {
  console.log(`Finalized: ${blockNumber}`);
});

// Subscribe to Comit events for an account
const comitSubId = await client.subscribeComitEvents(account, (event) => {
  switch (event.type) {
    case 'submitted':
      console.log(`Comit submitted: ${event.data.comitId}`);
      break;
    case 'finalized':
      console.log(`Comit finalized: ${event.data.comitId}`);
      break;
    case 'failed':
      console.log(`Comit failed: ${event.data.reason.type}`);
      break;
  }
});

// Unsubscribe
await client.unsubscribe(blockSubId);
```

## EVM Utilities

```typescript
import {
  // Address utilities
  isValidAddress,
  normalizeAddress,
  checksumAddress,
  accountIdToAddress,
  addressToAccountId,
  
  // ABI encoding
  encodeUint256,
  encodeAddress,
  encodeFunctionCall,
  
  // Common function encoders
  encodeTransfer,
  encodeApprove,
  encodeBalanceOf,
  
  // Error decoding
  decodeErrorMessage,
  getPanicMessage,
} from '@x3-chain/ts-sdk';

// Encode ERC20 transfer
const transferData = encodeTransfer(recipient, amount);

// Encode custom function call
const customCall = encodeFunctionCall(
  'myFunction(uint256,address)',
  [encodeUint256(value), encodeAddress(addr)]
);

// Decode revert reason
const revertMsg = decodeErrorMessage(revertData);
```

## SVM Utilities

```typescript
import {
  // Pubkey utilities
  isValidPubkey,
  pubkeyToBytes,
  findProgramAddress,
  
  // Instruction encoding
  encodeInstruction,
  encodeInstructionData,
  
  // Data type encoding
  encodeU64,
  encodeString,
  encodeVec,
  
  // Common programs
  SYSTEM_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  encodeSystemTransfer,
  encodeTokenTransfer,
  
  // Anchor support
  anchorDiscriminator,
} from '@x3-chain/ts-sdk';

// Encode system transfer
const transferIx = encodeSystemTransfer(1000000000n);

// Find PDA
const { address, bump } = findProgramAddress(
  [Buffer.from('seed'), userPubkey],
  programId
);

// Anchor discriminator
const disc = anchorDiscriminator('initialize');
```

## Error Handling

```typescript
import {
  AtlasSphereError,
  ConnectionError,
  ComitSubmissionError,
  InvalidNonceError,
  InsufficientBalanceError,
  UnauthorizedError,
  EvmExecutionError,
  PayloadSizeError,
  TimeoutError,
} from '@x3-chain/ts-sdk';

try {
  const result = await client.submitComit(comit, account);
} catch (error) {
  if (error instanceof ConnectionError) {
    console.error(`Connection failed to ${error.endpoint}`);
  } else if (error instanceof InvalidNonceError) {
    console.error(`Invalid nonce: expected ${error.expected}, got ${error.provided}`);
  } else if (error instanceof InsufficientBalanceError) {
    console.error(`Insufficient balance: need ${error.required}, have ${error.available}`);
  } else if (error instanceof UnauthorizedError) {
    console.error(`Account ${error.account} not authorized`);
  } else if (error instanceof EvmExecutionError) {
    const reason = error.getRevertReason();
    console.error(`EVM reverted: ${reason || 'unknown'} (gas: ${error.gasUsed})`);
  } else if (error instanceof PayloadSizeError) {
    console.error(`${error.payloadType} payload too large: ${error.size}/${error.maxSize}`);
  } else if (error instanceof TimeoutError) {
    console.error(`${error.operation} timed out after ${error.timeoutMs}ms`);
  }
}
```

## Constants

```typescript
import {
  // Network endpoints
  DEFAULT_WS_ENDPOINT,    // ws://127.0.0.1:9944
  TESTNET_WS_ENDPOINT,    // wss://testnet.atlassphere.io
  
  // Payload limits
  MAX_EVM_PAYLOAD_SIZE,   // 16 KB
  MAX_SVM_PAYLOAD_SIZE,   // 16 KB
  MAX_COMBINED_PAYLOAD_SIZE, // 32 KB
  
  // Gas/compute defaults
  DEFAULT_EVM_GAS_LIMIT,  // 3,000,000
  DEFAULT_SVM_COMPUTE_UNITS, // 200,000
  
  // Fees
  BASE_COMIT_FEE,         // 1 milli-X3
  
  // Assets
  NATIVE_ASSET_ID,        // 0
  ONE_ATLAS,              // 10^18
  
  // Timing
  BLOCK_TIME_MS,          // 6000 (6 seconds)
} from '@x3-chain/ts-sdk';
```

## API Reference

### AtlasSphereClient

Main client for X3 Chain blockchain interaction.

| Method                                    | Description                         |
| ----------------------------------------- | ----------------------------------- |
| `connect()`                               | Connect to the node                 |
| `disconnect()`                            | Disconnect from the node            |
| `getChainInfo()`                          | Get chain name, version, properties |
| `getBalance(account)`                     | Get native balance                  |
| `getCanonicalBalance(account, assetId)`   | Get canonical ledger balance        |
| `isAuthorized(account)`                   | Check authorization status          |
| `getNonce(account)`                       | Get Comit nonce                     |
| `submitComit(input, signer)`              | Submit and wait for finalization    |
| `subscribeNewBlocks(callback)`            | Subscribe to new blocks             |
| `subscribeFinalizedBlocks(callback)`      | Subscribe to finalized blocks       |
| `subscribeComitEvents(account, callback)` | Subscribe to Comit events           |

### ComitBuilder

Fluent builder for Comit construction.

| Method                    | Description                              |
| ------------------------- | ---------------------------------------- |
| `withEvmPayload(payload)` | Set EVM payload                          |
| `withSvmPayload(payload)` | Set SVM payload                          |
| `withFee(fee)`            | Set fee (explicit or 'auto')             |
| `withEvmGasLimit(gas)`    | Set EVM gas limit                        |
| `withSvmComputeUnits(cu)` | Set SVM compute units                    |
| `withOrigin(account)`     | Set origin for prepare_root              |
| `withNonce(nonce)`        | Set nonce for prepare_root               |
| `build()`                 | Build ComitInput                         |
| `buildComit()`            | Build full Comit (requires origin/nonce) |

### QueryClient

Cached query client for efficient state access.

| Method                                           | Description            |
| ------------------------------------------------ | ---------------------- |
| `getCanonicalBalance(account, assetId, options)` | Get balance            |
| `getCanonicalBalances(accounts, assetId)`        | Batch get balances     |
| `getAllBalances(account)`                        | Get all asset balances |
| `getAssetMetadata(assetId)`                      | Get asset info         |
| `isAuthorized(account)`                          | Check authorization    |
| `getNonce(account)`                              | Get Comit nonce        |
| `getAuthorities()`                               | Get validator set      |
| `clearCache()`                                   | Clear all cached data  |

## License

Apache-2.0
