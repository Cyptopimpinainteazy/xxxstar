/**
 * Constants for X3 Chain SDK
 *
 * Defines network parameters, limits, and default values.
 */

// =============================================================================
// Network Constants
// =============================================================================

/** Default WebSocket RPC endpoint for local development */
export const DEFAULT_WS_ENDPOINT = 'ws://127.0.0.1:9944';

/** Default HTTP RPC endpoint for local development */
export const DEFAULT_HTTP_ENDPOINT = 'http://127.0.0.1:9933';

/** X3 Chain mainnet WebSocket endpoint (when launched) */
export const MAINNET_WS_ENDPOINT = 'wss://rpc.atlassphere.io';

/** X3 Chain testnet WebSocket endpoint */
export const TESTNET_WS_ENDPOINT = 'wss://testnet.atlassphere.io';

// =============================================================================
// Payload Limits
// =============================================================================

/** Maximum size of EVM payload in bytes */
export const MAX_EVM_PAYLOAD_SIZE = 16 * 1024; // 16 KB

/** Maximum size of SVM payload in bytes */
export const MAX_SVM_PAYLOAD_SIZE = 16 * 1024; // 16 KB

/** Maximum combined payload size in bytes */
export const MAX_COMBINED_PAYLOAD_SIZE = 32 * 1024; // 32 KB

// =============================================================================
// Gas and Compute Constants
// =============================================================================

/** Default gas limit for EVM execution */
export const DEFAULT_EVM_GAS_LIMIT = 3_000_000n;

/** Maximum gas limit for EVM execution */
export const MAX_EVM_GAS_LIMIT = 15_000_000n;

/** Default compute units for SVM execution */
export const DEFAULT_SVM_COMPUTE_UNITS = 200_000n;

/** Maximum compute units for SVM execution */
export const MAX_SVM_COMPUTE_UNITS = 1_400_000n;

/** Gas price in smallest unit (1 micro-X3 per gas unit) */
export const GAS_PRICE = 1_000_000_000_000n; // 10^12

/** SVM compute unit price in smallest unit */
export const COMPUTE_UNIT_PRICE = 1_000_000_000_000n; // 10^12

// =============================================================================
// Fee Constants
// =============================================================================

/** Base fee for Comit submission */
export const BASE_COMIT_FEE = 1_000_000_000_000n; // 1 milli-X3

/** Gas divisor for fee calculation (EVM gas / 1000) */
export const GAS_FEE_DIVISOR = 1000n;

/** Compute unit divisor for fee calculation (SVM CU / 1000) */
export const COMPUTE_FEE_DIVISOR = 1000n;

// =============================================================================
// Timing Constants
// =============================================================================

/** Block time in milliseconds (Aura 6-second slots) */
export const BLOCK_TIME_MS = 6000;

/** Default timeout for RPC calls in milliseconds */
export const DEFAULT_RPC_TIMEOUT_MS = 30000;

/** Default timeout for Comit finalization in milliseconds */
export const DEFAULT_FINALIZATION_TIMEOUT_MS = 60000;

/** Number of blocks to wait for finalization */
export const FINALIZATION_BLOCKS = 3;

// =============================================================================
// Asset Constants
// =============================================================================

/** Native asset ID (X3 token) */
export const NATIVE_ASSET_ID = 0;

/** Native asset symbol */
export const NATIVE_ASSET_SYMBOL = 'X3';

/** Native asset decimals */
export const NATIVE_ASSET_DECIMALS = 18;

/** One X3 in smallest unit (10^18) */
export const ONE_ATLAS = 1_000_000_000_000_000_000n;

/** One milli-X3 in smallest unit (10^15) */
export const ONE_MILLI_ATLAS = 1_000_000_000_000_000n;

/** One micro-X3 in smallest unit (10^12) */
export const ONE_MICRO_ATLAS = 1_000_000_000_000n;

// =============================================================================
// Address Constants
// =============================================================================

/** Length of Substrate AccountId in bytes */
export const ACCOUNT_ID_LENGTH = 32;

/** Length of EVM address in bytes */
export const EVM_ADDRESS_LENGTH = 20;

/** Length of Solana pubkey in bytes */
export const SOLANA_PUBKEY_LENGTH = 32;

// =============================================================================
// Hash Constants
// =============================================================================

/** Length of H256 hash in bytes */
export const H256_LENGTH = 32;

/** Zero hash (32 zero bytes) */
export const ZERO_HASH = '0x0000000000000000000000000000000000000000000000000000000000000000';

// =============================================================================
// RPC Method Names
// =============================================================================

export const RPC_METHODS = {
  // X3 Kernel methods
  getCanonicalBalance: 'atlasKernel_getCanonicalBalance',
  getAssetMetadata: 'atlasKernel_getAssetMetadata',
  isAuthorized: 'atlasKernel_isAuthorized',
  getAuthorizedAccounts: 'atlasKernel_getAuthorizedAccounts',
  getAuthorities: 'atlasKernel_getAuthorities',

  // Standard Substrate methods
  chainGetHeader: 'chain_getHeader',
  chainGetBlock: 'chain_getBlock',
  chainGetBlockHash: 'chain_getBlockHash',
  chainGetFinalizedHead: 'chain_getFinalizedHead',
  stateGetStorage: 'state_getStorage',
  stateGetStorageAt: 'state_getStorageAt',
  stateCall: 'state_call',
  systemHealth: 'system_health',
  systemChain: 'system_chain',
  systemName: 'system_name',
  systemVersion: 'system_version',
  systemProperties: 'system_properties',
  authorSubmitExtrinsic: 'author_submitExtrinsic',
  authorSubmitAndWatchExtrinsic: 'author_submitAndWatchExtrinsic',
} as const;

// =============================================================================
// Event Names
// =============================================================================

export const EVENTS = {
  // Comit lifecycle events
  comitSubmitted: 'AtlasKernel.ComitSubmitted',
  comitExecutionStarted: 'AtlasKernel.ComitExecutionStarted',
  comitExecutionCompleted: 'AtlasKernel.ComitExecutionCompleted',
  comitFinalized: 'AtlasKernel.ComitFinalized',
  comitFailed: 'AtlasKernel.ComitFailed',

  // Authority events
  accountAuthorized: 'AtlasKernel.AccountAuthorized',
  accountDeauthorized: 'AtlasKernel.AccountDeauthorized',

  // Asset events
  assetRegistered: 'AtlasKernel.AssetRegistered',
  canonicalBalanceUpdated: 'AtlasKernel.CanonicalBalanceUpdated',
} as const;

// =============================================================================
// Storage Keys
// =============================================================================

export const STORAGE_PREFIXES = {
  authorizedAccounts: 'AtlasKernel.AuthorizedAccounts',
  canonicalLedger: 'AtlasKernel.CanonicalLedger',
  assetMetadata: 'AtlasKernel.AssetMetadata',
  comitNonces: 'AtlasKernel.ComitNonces',
  authorities: 'AtlasKernel.Authorities',
} as const;

// =============================================================================
// EVM Selectors
// =============================================================================

export const EVM_SELECTORS = {
  /** Error(string) - standard revert with message */
  error: '0x08c379a0',
  /** Panic(uint256) - panic code */
  panic: '0x4e487b71',
} as const;
