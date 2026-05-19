/**
 * QueryClient - Specialized client for querying X3 Chain state
 *
 * Provides efficient read-only access to blockchain state with caching.
 */
import type { ApiPromise } from '@polkadot/api';
import type { HexString } from '@polkadot/util/types';
import type { AccountId, AssetId, Balance, Hash, BlockNumber, AssetMetadata, LedgerEntry } from './types';
/**
 * Query options
 */
export interface QueryOptions {
    /** Block hash to query at (default: latest) */
    at?: Hash;
    /** Whether to use cache (default: true) */
    useCache?: boolean;
}
/**
 * Pagination options for list queries
 */
export interface PaginationOptions {
    /** Maximum number of results */
    limit?: number;
    /** Offset for pagination */
    offset?: number;
    /** Starting key for cursor-based pagination */
    startKey?: HexString;
}
/**
 * Specialized client for querying X3 Chain state
 *
 * @example
 * ```typescript
 * const query = new QueryClient(api);
 *
 * // Get balance with caching
 * const balance = await query.getCanonicalBalance(account);
 *
 * // Get multiple balances efficiently
 * const balances = await query.getCanonicalBalances([account1, account2]);
 *
 * // Query at specific block
 * const oldBalance = await query.getCanonicalBalance(account, {
 *   at: blockHash,
 * });
 * ```
 */
export declare class QueryClient {
    private api;
    private cache;
    private cacheMaxAge;
    private currentBlockNumber;
    /**
     * Create a new QueryClient
     *
     * @param api - Polkadot API instance
     * @param cacheMaxAgeMs - Maximum age for cached entries (default: 6000ms = 1 block)
     */
    constructor(api: ApiPromise, cacheMaxAgeMs?: number);
    /**
     * Get canonical ledger balance for an account
     */
    getCanonicalBalance(account: AccountId, assetId?: AssetId, options?: QueryOptions): Promise<Balance>;
    /**
     * Get canonical balances for multiple accounts
     */
    getCanonicalBalances(accounts: AccountId[], assetId?: AssetId, options?: QueryOptions): Promise<Map<AccountId, Balance>>;
    /**
     * Get all balances for an account across all assets
     */
    getAllBalances(account: AccountId, options?: QueryOptions): Promise<LedgerEntry[]>;
    /**
     * Get metadata for an asset
     */
    getAssetMetadata(assetId: AssetId, options?: QueryOptions): Promise<AssetMetadata | null>;
    /**
     * Get all registered assets
     */
    getAllAssets(options?: QueryOptions): Promise<Array<{
        assetId: AssetId;
    } & AssetMetadata>>;
    /**
     * Check if an account is authorized
     */
    isAuthorized(account: AccountId, options?: QueryOptions): Promise<boolean>;
    /**
     * Get all authorized accounts
     */
    getAuthorizedAccounts(pagination?: PaginationOptions, options?: QueryOptions): Promise<AccountId[]>;
    /**
     * Get current nonce for an account
     */
    getNonce(account: AccountId, options?: QueryOptions): Promise<bigint>;
    /**
     * Get current authorities
     */
    getAuthorities(options?: QueryOptions): Promise<AccountId[]>;
    /**
     * Get current block number
     */
    getBlockNumber(): Promise<BlockNumber>;
    /**
     * Get block hash for a given number
     */
    getBlockHash(blockNumber: BlockNumber): Promise<Hash>;
    /**
     * Get finalized block number
     */
    getFinalizedBlockNumber(): Promise<BlockNumber>;
    /**
     * Clear all cached data
     */
    clearCache(): void;
    /**
     * Clear cache for specific account
     */
    clearAccountCache(account: AccountId): void;
    /**
     * Update the current block number (for cache validation)
     */
    updateBlockNumber(blockNumber: BlockNumber): void;
    private getFromCache;
    private setCache;
    private pruneCache;
}
/**
 * Create a QueryClient from an AtlasSphereClient
 */
export declare function createQueryClient(api: ApiPromise, cacheMaxAgeMs?: number): QueryClient;
//# sourceMappingURL=query.d.ts.map