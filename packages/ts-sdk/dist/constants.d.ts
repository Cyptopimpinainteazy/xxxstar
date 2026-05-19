/**
 * Constants for X3 Chain SDK
 *
 * Defines network parameters, limits, and default values.
 */
/** Default WebSocket RPC endpoint for local development */
export declare const DEFAULT_WS_ENDPOINT = "ws://127.0.0.1:9944";
/** Default HTTP RPC endpoint for local development */
export declare const DEFAULT_HTTP_ENDPOINT = "http://127.0.0.1:9933";
/** X3 Chain mainnet WebSocket endpoint (when launched) */
export declare const MAINNET_WS_ENDPOINT = "wss://rpc.atlassphere.io";
/** X3 Chain testnet WebSocket endpoint */
export declare const TESTNET_WS_ENDPOINT = "wss://testnet.atlassphere.io";
/** Maximum size of EVM payload in bytes */
export declare const MAX_EVM_PAYLOAD_SIZE: number;
/** Maximum size of SVM payload in bytes */
export declare const MAX_SVM_PAYLOAD_SIZE: number;
/** Maximum combined payload size in bytes */
export declare const MAX_COMBINED_PAYLOAD_SIZE: number;
/** Default gas limit for EVM execution */
export declare const DEFAULT_EVM_GAS_LIMIT = 3000000n;
/** Maximum gas limit for EVM execution */
export declare const MAX_EVM_GAS_LIMIT = 15000000n;
/** Default compute units for SVM execution */
export declare const DEFAULT_SVM_COMPUTE_UNITS = 200000n;
/** Maximum compute units for SVM execution */
export declare const MAX_SVM_COMPUTE_UNITS = 1400000n;
/** Gas price in smallest unit (1 micro-X3 per gas unit) */
export declare const GAS_PRICE = 1000000000000n;
/** SVM compute unit price in smallest unit */
export declare const COMPUTE_UNIT_PRICE = 1000000000000n;
/** Base fee for Comit submission */
export declare const BASE_COMIT_FEE = 1000000000000n;
/** Gas divisor for fee calculation (EVM gas / 1000) */
export declare const GAS_FEE_DIVISOR = 1000n;
/** Compute unit divisor for fee calculation (SVM CU / 1000) */
export declare const COMPUTE_FEE_DIVISOR = 1000n;
/** Block time in milliseconds (Aura 6-second slots) */
export declare const BLOCK_TIME_MS = 6000;
/** Default timeout for RPC calls in milliseconds */
export declare const DEFAULT_RPC_TIMEOUT_MS = 30000;
/** Default timeout for Comit finalization in milliseconds */
export declare const DEFAULT_FINALIZATION_TIMEOUT_MS = 60000;
/** Number of blocks to wait for finalization */
export declare const FINALIZATION_BLOCKS = 3;
/** Native asset ID (X3 token) */
export declare const NATIVE_ASSET_ID = 0;
/** Native asset symbol */
export declare const NATIVE_ASSET_SYMBOL = "X3";
/** Native asset decimals */
export declare const NATIVE_ASSET_DECIMALS = 18;
/** One X3 in smallest unit (10^18) */
export declare const ONE_ATLAS = 1000000000000000000n;
/** One milli-X3 in smallest unit (10^15) */
export declare const ONE_MILLI_ATLAS = 1000000000000000n;
/** One micro-X3 in smallest unit (10^12) */
export declare const ONE_MICRO_ATLAS = 1000000000000n;
/** Length of Substrate AccountId in bytes */
export declare const ACCOUNT_ID_LENGTH = 32;
/** Length of EVM address in bytes */
export declare const EVM_ADDRESS_LENGTH = 20;
/** Length of Solana pubkey in bytes */
export declare const SOLANA_PUBKEY_LENGTH = 32;
/** Length of H256 hash in bytes */
export declare const H256_LENGTH = 32;
/** Zero hash (32 zero bytes) */
export declare const ZERO_HASH = "0x0000000000000000000000000000000000000000000000000000000000000000";
export declare const RPC_METHODS: {
    readonly getCanonicalBalance: "atlasKernel_getCanonicalBalance";
    readonly getAssetMetadata: "atlasKernel_getAssetMetadata";
    readonly isAuthorized: "atlasKernel_isAuthorized";
    readonly getAuthorizedAccounts: "atlasKernel_getAuthorizedAccounts";
    readonly getAuthorities: "atlasKernel_getAuthorities";
    readonly chainGetHeader: "chain_getHeader";
    readonly chainGetBlock: "chain_getBlock";
    readonly chainGetBlockHash: "chain_getBlockHash";
    readonly chainGetFinalizedHead: "chain_getFinalizedHead";
    readonly stateGetStorage: "state_getStorage";
    readonly stateGetStorageAt: "state_getStorageAt";
    readonly stateCall: "state_call";
    readonly systemHealth: "system_health";
    readonly systemChain: "system_chain";
    readonly systemName: "system_name";
    readonly systemVersion: "system_version";
    readonly systemProperties: "system_properties";
    readonly authorSubmitExtrinsic: "author_submitExtrinsic";
    readonly authorSubmitAndWatchExtrinsic: "author_submitAndWatchExtrinsic";
};
export declare const EVENTS: {
    readonly comitSubmitted: "AtlasKernel.ComitSubmitted";
    readonly comitExecutionStarted: "AtlasKernel.ComitExecutionStarted";
    readonly comitExecutionCompleted: "AtlasKernel.ComitExecutionCompleted";
    readonly comitFinalized: "AtlasKernel.ComitFinalized";
    readonly comitFailed: "AtlasKernel.ComitFailed";
    readonly accountAuthorized: "AtlasKernel.AccountAuthorized";
    readonly accountDeauthorized: "AtlasKernel.AccountDeauthorized";
    readonly assetRegistered: "AtlasKernel.AssetRegistered";
    readonly canonicalBalanceUpdated: "AtlasKernel.CanonicalBalanceUpdated";
};
export declare const STORAGE_PREFIXES: {
    readonly authorizedAccounts: "AtlasKernel.AuthorizedAccounts";
    readonly canonicalLedger: "AtlasKernel.CanonicalLedger";
    readonly assetMetadata: "AtlasKernel.AssetMetadata";
    readonly comitNonces: "AtlasKernel.ComitNonces";
    readonly authorities: "AtlasKernel.Authorities";
};
export declare const EVM_SELECTORS: {
    /** Error(string) - standard revert with message */
    readonly error: "0x08c379a0";
    /** Panic(uint256) - panic code */
    readonly panic: "0x4e487b71";
};
//# sourceMappingURL=constants.d.ts.map