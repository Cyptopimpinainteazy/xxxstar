"use strict";
/**
 * Constants for X3 Chain SDK
 *
 * Defines network parameters, limits, and default values.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.EVM_SELECTORS = exports.STORAGE_PREFIXES = exports.EVENTS = exports.RPC_METHODS = exports.ZERO_HASH = exports.H256_LENGTH = exports.SOLANA_PUBKEY_LENGTH = exports.EVM_ADDRESS_LENGTH = exports.ACCOUNT_ID_LENGTH = exports.ONE_MICRO_ATLAS = exports.ONE_MILLI_ATLAS = exports.ONE_ATLAS = exports.NATIVE_ASSET_DECIMALS = exports.NATIVE_ASSET_SYMBOL = exports.NATIVE_ASSET_ID = exports.FINALIZATION_BLOCKS = exports.DEFAULT_FINALIZATION_TIMEOUT_MS = exports.DEFAULT_RPC_TIMEOUT_MS = exports.BLOCK_TIME_MS = exports.COMPUTE_FEE_DIVISOR = exports.GAS_FEE_DIVISOR = exports.BASE_COMIT_FEE = exports.COMPUTE_UNIT_PRICE = exports.GAS_PRICE = exports.MAX_SVM_COMPUTE_UNITS = exports.DEFAULT_SVM_COMPUTE_UNITS = exports.MAX_EVM_GAS_LIMIT = exports.DEFAULT_EVM_GAS_LIMIT = exports.MAX_COMBINED_PAYLOAD_SIZE = exports.MAX_SVM_PAYLOAD_SIZE = exports.MAX_EVM_PAYLOAD_SIZE = exports.TESTNET_WS_ENDPOINT = exports.MAINNET_WS_ENDPOINT = exports.DEFAULT_HTTP_ENDPOINT = exports.DEFAULT_WS_ENDPOINT = void 0;
// =============================================================================
// Network Constants
// =============================================================================
/** Default WebSocket RPC endpoint for local development */
exports.DEFAULT_WS_ENDPOINT = 'ws://127.0.0.1:9944';
/** Default HTTP RPC endpoint for local development */
exports.DEFAULT_HTTP_ENDPOINT = 'http://127.0.0.1:9933';
/** X3 Chain mainnet WebSocket endpoint (when launched) */
exports.MAINNET_WS_ENDPOINT = 'wss://rpc.atlassphere.io';
/** X3 Chain testnet WebSocket endpoint */
exports.TESTNET_WS_ENDPOINT = 'wss://testnet.atlassphere.io';
// =============================================================================
// Payload Limits
// =============================================================================
/** Maximum size of EVM payload in bytes */
exports.MAX_EVM_PAYLOAD_SIZE = 16 * 1024; // 16 KB
/** Maximum size of SVM payload in bytes */
exports.MAX_SVM_PAYLOAD_SIZE = 16 * 1024; // 16 KB
/** Maximum combined payload size in bytes */
exports.MAX_COMBINED_PAYLOAD_SIZE = 32 * 1024; // 32 KB
// =============================================================================
// Gas and Compute Constants
// =============================================================================
/** Default gas limit for EVM execution */
exports.DEFAULT_EVM_GAS_LIMIT = 3000000n;
/** Maximum gas limit for EVM execution */
exports.MAX_EVM_GAS_LIMIT = 15000000n;
/** Default compute units for SVM execution */
exports.DEFAULT_SVM_COMPUTE_UNITS = 200000n;
/** Maximum compute units for SVM execution */
exports.MAX_SVM_COMPUTE_UNITS = 1400000n;
/** Gas price in smallest unit (1 micro-X3 per gas unit) */
exports.GAS_PRICE = 1000000000000n; // 10^12
/** SVM compute unit price in smallest unit */
exports.COMPUTE_UNIT_PRICE = 1000000000000n; // 10^12
// =============================================================================
// Fee Constants
// =============================================================================
/** Base fee for Comit submission */
exports.BASE_COMIT_FEE = 1000000000000n; // 1 milli-X3
/** Gas divisor for fee calculation (EVM gas / 1000) */
exports.GAS_FEE_DIVISOR = 1000n;
/** Compute unit divisor for fee calculation (SVM CU / 1000) */
exports.COMPUTE_FEE_DIVISOR = 1000n;
// =============================================================================
// Timing Constants
// =============================================================================
/** Block time in milliseconds (Aura 6-second slots) */
exports.BLOCK_TIME_MS = 6000;
/** Default timeout for RPC calls in milliseconds */
exports.DEFAULT_RPC_TIMEOUT_MS = 30000;
/** Default timeout for Comit finalization in milliseconds */
exports.DEFAULT_FINALIZATION_TIMEOUT_MS = 60000;
/** Number of blocks to wait for finalization */
exports.FINALIZATION_BLOCKS = 3;
// =============================================================================
// Asset Constants
// =============================================================================
/** Native asset ID (X3 token) */
exports.NATIVE_ASSET_ID = 0;
/** Native asset symbol */
exports.NATIVE_ASSET_SYMBOL = 'X3';
/** Native asset decimals */
exports.NATIVE_ASSET_DECIMALS = 18;
/** One X3 in smallest unit (10^18) */
exports.ONE_ATLAS = 1000000000000000000n;
/** One milli-X3 in smallest unit (10^15) */
exports.ONE_MILLI_ATLAS = 1000000000000000n;
/** One micro-X3 in smallest unit (10^12) */
exports.ONE_MICRO_ATLAS = 1000000000000n;
// =============================================================================
// Address Constants
// =============================================================================
/** Length of Substrate AccountId in bytes */
exports.ACCOUNT_ID_LENGTH = 32;
/** Length of EVM address in bytes */
exports.EVM_ADDRESS_LENGTH = 20;
/** Length of Solana pubkey in bytes */
exports.SOLANA_PUBKEY_LENGTH = 32;
// =============================================================================
// Hash Constants
// =============================================================================
/** Length of H256 hash in bytes */
exports.H256_LENGTH = 32;
/** Zero hash (32 zero bytes) */
exports.ZERO_HASH = '0x0000000000000000000000000000000000000000000000000000000000000000';
// =============================================================================
// RPC Method Names
// =============================================================================
exports.RPC_METHODS = {
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
};
// =============================================================================
// Event Names
// =============================================================================
exports.EVENTS = {
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
};
// =============================================================================
// Storage Keys
// =============================================================================
exports.STORAGE_PREFIXES = {
    authorizedAccounts: 'AtlasKernel.AuthorizedAccounts',
    canonicalLedger: 'AtlasKernel.CanonicalLedger',
    assetMetadata: 'AtlasKernel.AssetMetadata',
    comitNonces: 'AtlasKernel.ComitNonces',
    authorities: 'AtlasKernel.Authorities',
};
// =============================================================================
// EVM Selectors
// =============================================================================
exports.EVM_SELECTORS = {
    /** Error(string) - standard revert with message */
    error: '0x08c379a0',
    /** Panic(uint256) - panic code */
    panic: '0x4e487b71',
};
//# sourceMappingURL=constants.js.map