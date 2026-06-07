/**
 * @module @x3-chain/ts-sdk
 *
 * TypeScript SDK for X3 Chain - Dual-VM (EVM + SVM) Layer-1 Blockchain
 *
 * This SDK provides a comprehensive interface for interacting with X3 Chain nodes,
 * submitting Comit transactions, querying the canonical ledger, and working with
 * both EVM and SVM payloads.
 *
 * @example
 * ```typescript
 * import { AtlasSphereClient, ComitBuilder } from '@x3-chain/ts-sdk';
 *
 * const client = new AtlasSphereClient({ endpoint: 'ws://localhost:9944' });
 * await client.connect();
 *
 * const comit = new ComitBuilder()
 *   .withEvmPayload({ to: '0x...', data: '0x...', value: 0n })
 *   .withSvmPayload({ programId: '0x...', data: '0x...' })
 *   .withFee('auto')
 *   .build();
 *
 * const result = await client.submitComit(comit, signerAccount);
 * ```
 */

// =============================================================================
// Core Types
// =============================================================================

export type {
  // Primitive types
  Hash,
  AccountId,
  AssetId,
  Balance,
  BlockNumber,
  Timestamp,
  Nonce,
  // Comit types
  Comit,
  ComitInput,
  ExecutionLog,
  StateChange,
  ExecutionReceipt,
  ComitResult,
  // State types
  SphereState,
  AssetMetadata,
  LedgerEntry,
  // Event types
  ComitSubmittedEvent,
  ComitExecutionStartedEvent,
  ComitExecutionCompletedEvent,
  ComitFinalizedEvent,
  ComitFailedEvent,
  ComitFailureReason,
  ComitEvent,
  // Authority types
  Authority,
  // RPC response types
  GetCanonicalBalanceResponse,
  GetAssetMetadataResponse,
  IsAuthorizedResponse,
  GetAuthorizedAccountsResponse,
  GetAuthoritiesResponse,
  // Subscription types
  BlockSubscriptionCallback,
  ComitEventCallback,
} from './types';

// =============================================================================
// Main Client
// =============================================================================

export {
  AtlasSphereClient,
  createClient,
  createLocalClient,
  createTestnetClient,
} from './client';

export type {
  AtlasSphereClientConfig,
  ConnectionStatus,
  ChainInfo,
} from './client';

// =============================================================================
// Comit Builder
// =============================================================================

export {
  ComitBuilder,
  comit,
  evmComit,
  svmComit,
  dualComit,
} from './comit';

export type {
  EvmPayloadOptions,
  SvmPayloadOptions,
} from './comit';

// =============================================================================
// Query Client
// =============================================================================

export {
  QueryClient,
  createQueryClient,
} from './query';

// Collateral exports
export {
  CollateralManagerClient,
} from './collateral';

export type {
  BondId,
  BondState,
  DepositReceipt,
  WithdrawRequest,
} from './collateral';

export type {
  QueryOptions,
  PaginationOptions,
} from './query';

// =============================================================================
// EVM Utilities
// =============================================================================

export {
  // Address utilities
  isValidAddress,
  normalizeAddress,
  checksumAddress,
  accountIdToAddress,
  addressToAccountId,
  publicKeyToAddress,
  // ABI encoding
  functionSelector,
  encodeUint256,
  decodeUint256,
  encodeAddress,
  decodeAddress,
  encodeBytes,
  encodeString as encodeEvmString,
  encodeBool,
  decodeBool,
  // Function call encoding
  encodeFunctionCall,
  decodeFunctionCall,
  // Common function encoders
  encodeTransfer,
  encodeApprove,
  encodeTransferFrom,
  encodeBalanceOf,
  // Error decoding
  isErrorRevert,
  isPanicRevert,
  decodeErrorMessage,
  decodePanicCode,
  getPanicMessage,
} from './evm';

export type {
  EvmTxParams,
  FunctionSignature,
  DecodedCall,
} from './evm';

// =============================================================================
// SVM Utilities
// =============================================================================

export {
  // Pubkey utilities
  isValidPubkey,
  pubkeyToBytes,
  bytesToPubkey,
  zeroPubkey,
  accountIdToPubkey,
  pubkeyToAccountId,
  findProgramAddress,
  // Instruction encoding
  encodeCompactU16,
  decodeCompactU16,
  encodeInstruction,
  encodeInstructionData,
  // Data type encoding
  encodeU8,
  encodeU16,
  encodeU32,
  encodeU64,
  decodeU64,
  encodeString as encodeSvmString,
  encodeVec,
  encodeOption,
  // Common programs
  SYSTEM_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  encodeSystemTransfer,
  encodeTokenTransfer,
  createTransferAccounts,
  // Anchor
  anchorDiscriminator,
  anchorAccountDiscriminator,
} from './svm';

export type {
  Pubkey,
  AccountMeta,
  Instruction,
  CompactU16,
} from './svm';

// =============================================================================
// Utility Functions
// =============================================================================

export {
  // Encoding utilities
  hexToBytes,
  bytesToHex,
  stringToBytes,
  toBytes,
  toHex,
  // Hashing
  blake2_256,
  blake2_256_bytes,
  computePrepareRoot,
  computeComitId,
  // Number encoding
  encodeU128,
  decodeU128,
  // Address utilities
  decodeAccountId,
  encodeAccountId,
  accountIdToEvmAddress,
  evmAddressToAccountId,
  isValidEvmAddress,
  isValidSolanaPubkey,
  // Validation
  validatePayloadSizes,
  isValidH256,
  validateBalance,
  validateNonce,
  // Format utilities
  formatBalance,
  parseBalance,
  truncateHash,
  // Async utilities
  sleep,
  retry,
} from './utils';

// =============================================================================
// Constants
// =============================================================================

export {
  // Network endpoints
  DEFAULT_WS_ENDPOINT,
  DEFAULT_HTTP_ENDPOINT,
  MAINNET_WS_ENDPOINT,
  TESTNET_WS_ENDPOINT,
  // Payload limits
  MAX_EVM_PAYLOAD_SIZE,
  MAX_SVM_PAYLOAD_SIZE,
  MAX_COMBINED_PAYLOAD_SIZE,
  // Gas and compute
  DEFAULT_EVM_GAS_LIMIT,
  MAX_EVM_GAS_LIMIT,
  DEFAULT_SVM_COMPUTE_UNITS,
  MAX_SVM_COMPUTE_UNITS,
  GAS_PRICE,
  COMPUTE_UNIT_PRICE,
  // Fees
  BASE_COMIT_FEE,
  GAS_FEE_DIVISOR,
  COMPUTE_FEE_DIVISOR,
  // Timing
  BLOCK_TIME_MS,
  DEFAULT_RPC_TIMEOUT_MS,
  DEFAULT_FINALIZATION_TIMEOUT_MS,
  FINALIZATION_BLOCKS,
  // Assets
  NATIVE_ASSET_ID,
  NATIVE_ASSET_SYMBOL,
  NATIVE_ASSET_DECIMALS,
  ONE_ATLAS,
  ONE_MILLI_ATLAS,
  ONE_MICRO_ATLAS,
  // Addresses
  ACCOUNT_ID_LENGTH,
  EVM_ADDRESS_LENGTH,
  SOLANA_PUBKEY_LENGTH,
  H256_LENGTH,
  ZERO_HASH,
  // RPC methods
  RPC_METHODS,
  // Events
  EVENTS,
  // Storage
  STORAGE_PREFIXES,
  // EVM selectors
  EVM_SELECTORS,
} from './constants';

// =============================================================================
// X3 Chain Integration
// =============================================================================

export {
  X3SubscriptionManager,
} from './subscriptions';

export type {
  BlockNotification,
  ComitNotification,
  EvmLogNotification,
  SubscriptionHandlers,
} from './subscriptions';

export {
  X3SettlementClient,
  X3AtomicTradeClient,
  X3DomainClient,
  X3VerifierClient,
  createX3SettlementClient,
  createX3TradeClient,
  createX3DomainClient,
  createX3VerifierClient,
} from './x3';

export type {
  X3VmType,
  X3AmmProtocol,
  X3TradeLeg,
  X3SettlementOptions,
} from './x3';

// =============================================================================
// Errors
// =============================================================================

export {
  AtlasSphereError,
  ConnectionError,
  RpcError,
  ComitSubmissionError,
  InvalidNonceError,
  InsufficientBalanceError,
  UnauthorizedError,
  RateLimitError,
  EvmExecutionError,
  SvmExecutionError,
  VerificationError,
  PayloadSizeError,
  TimeoutError,
  SubscriptionError,
  ValidationError,
  reasonToError,
} from './errors';
