/**
 * QueryClient - Specialized client for querying X3 Chain state
 *
 * Provides efficient read-only access to blockchain state with caching.
 */

import type { ApiPromise } from '@polkadot/api';
import type { HexString } from '@polkadot/util/types';

import type {
  AccountId,
  AssetId,
  Balance,
  Hash,
  BlockNumber,
  AssetMetadata,
  LedgerEntry,
} from './types';

import { RpcError } from './errors';
import { NATIVE_ASSET_ID } from './constants';

// =============================================================================
// Types
// =============================================================================

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
 * Cached query result
 */
interface CacheEntry<T> {
  value: T;
  blockNumber: BlockNumber;
  timestamp: number;
}

// =============================================================================
// QueryClient
// =============================================================================

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
export class QueryClient {
  private api: ApiPromise;
  private cache: Map<string, CacheEntry<any>> = new Map();
  private cacheMaxAge: number;
  private currentBlockNumber: BlockNumber = 0;

  /**
   * Create a new QueryClient
   *
   * @param api - Polkadot API instance
   * @param cacheMaxAgeMs - Maximum age for cached entries (default: 6000ms = 1 block)
   */
  constructor(api: ApiPromise, cacheMaxAgeMs: number = 6000) {
    this.api = api;
    this.cacheMaxAge = cacheMaxAgeMs;
  }

  // ===========================================================================
  // Balance Queries
  // ===========================================================================

  /**
   * Get canonical ledger balance for an account
   */
  async getCanonicalBalance(
    account: AccountId,
    assetId: AssetId = NATIVE_ASSET_ID,
    options: QueryOptions = {}
  ): Promise<Balance> {
    const cacheKey = `balance:${account}:${assetId}`;

    // Check cache
    if (options.useCache !== false) {
      const cached = this.getFromCache<Balance>(cacheKey);
      if (cached !== undefined) {
        return cached;
      }
    }

    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      const balance = await (queryAt as any).query.atlasKernel.canonicalLedger(account, assetId);
      const value = BigInt(balance.toString());

      // Cache result
      this.setCache(cacheKey, value);

      return value;
    } catch (error) {
      throw new RpcError(
        'getCanonicalBalance',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Get canonical balances for multiple accounts
   */
  async getCanonicalBalances(
    accounts: AccountId[],
    assetId: AssetId = NATIVE_ASSET_ID,
    options: QueryOptions = {}
  ): Promise<Map<AccountId, Balance>> {
    const results = new Map<AccountId, Balance>();

    // Split into cached and uncached
    const uncached: AccountId[] = [];

    if (options.useCache !== false) {
      for (const account of accounts) {
        const cached = this.getFromCache<Balance>(`balance:${account}:${assetId}`);
        if (cached !== undefined) {
          results.set(account, cached);
        } else {
          uncached.push(account);
        }
      }
    } else {
      uncached.push(...accounts);
    }

    // Batch query uncached
    if (uncached.length > 0) {
      try {
        const queryAt = options.at
          ? await this.api.at(options.at)
          : this.api;

        const keys = uncached.map((account) => [account, assetId]);
        const balances = await (queryAt as any).query.atlasKernel.canonicalLedger.multi(keys);

        for (let i = 0; i < uncached.length; i++) {
          const value = BigInt(balances[i].toString());
          results.set(uncached[i], value);
          this.setCache(`balance:${uncached[i]}:${assetId}`, value);
        }
      } catch (error) {
        throw new RpcError(
          'getCanonicalBalances',
          error instanceof Error ? error.message : String(error)
        );
      }
    }

    return results;
  }

  /**
   * Get all balances for an account across all assets
   */
  async getAllBalances(
    account: AccountId,
    options: QueryOptions = {}
  ): Promise<LedgerEntry[]> {
    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      // Query all entries with this account prefix
      const entries = await (queryAt as any).query.atlasKernel.canonicalLedger.entries(account);

      return entries.map(([key, value]: [any, any]) => ({
        account,
        assetId: key.args[1].toNumber(),
        balance: BigInt(value.toString()),
      }));
    } catch (error) {
      throw new RpcError(
        'getAllBalances',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  // ===========================================================================
  // Asset Queries
  // ===========================================================================

  /**
   * Get metadata for an asset
   */
  async getAssetMetadata(
    assetId: AssetId,
    options: QueryOptions = {}
  ): Promise<AssetMetadata | null> {
    const cacheKey = `asset:${assetId}`;

    // Check cache (asset metadata rarely changes)
    if (options.useCache !== false) {
      const cached = this.getFromCache<AssetMetadata | null>(cacheKey);
      if (cached !== undefined) {
        return cached;
      }
    }

    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      const metadata = await (queryAt as any).query.atlasKernel.assetMetadata(assetId);

      if (metadata.isNone) {
        this.setCache(cacheKey, null);
        return null;
      }

      const data = metadata.unwrap();
      const result: AssetMetadata = {
        symbol: data.symbol.toUtf8(),
        decimals: data.decimals.toNumber(),
      };

      this.setCache(cacheKey, result);
      return result;
    } catch (error) {
      throw new RpcError(
        'getAssetMetadata',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Get all registered assets
   */
  async getAllAssets(options: QueryOptions = {}): Promise<Array<{ assetId: AssetId } & AssetMetadata>> {
    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      const entries = await (queryAt as any).query.atlasKernel.assetMetadata.entries();

      return entries.map(([key, value]: [any, any]) => {
        const data = value.unwrap();
        return {
          assetId: key.args[0].toNumber(),
          symbol: data.symbol.toUtf8(),
          decimals: data.decimals.toNumber(),
        };
      });
    } catch (error) {
      throw new RpcError(
        'getAllAssets',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  // ===========================================================================
  // Authorization Queries
  // ===========================================================================

  /**
   * Check if an account is authorized
   */
  async isAuthorized(
    account: AccountId,
    options: QueryOptions = {}
  ): Promise<boolean> {
    const cacheKey = `auth:${account}`;

    if (options.useCache !== false) {
      const cached = this.getFromCache<boolean>(cacheKey);
      if (cached !== undefined) {
        return cached;
      }
    }

    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      const result = await (queryAt as any).query.atlasKernel.authorizedAccounts(account);
      const authorized = result.isSome;

      this.setCache(cacheKey, authorized);
      return authorized;
    } catch (error) {
      throw new RpcError(
        'isAuthorized',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Get all authorized accounts
   */
  async getAuthorizedAccounts(
    pagination?: PaginationOptions,
    options: QueryOptions = {}
  ): Promise<AccountId[]> {
    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      let entries = await (queryAt as any).query.atlasKernel.authorizedAccounts.entries();

      // Apply pagination
      if (pagination?.offset) {
        entries = entries.slice(pagination.offset);
      }
      if (pagination?.limit) {
        entries = entries.slice(0, pagination.limit);
      }

      return entries.map(([key]: [any]) => key.args[0].toString());
    } catch (error) {
      throw new RpcError(
        'getAuthorizedAccounts',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  // ===========================================================================
  // Nonce Queries
  // ===========================================================================

  /**
   * Get current nonce for an account
   */
  async getNonce(
    account: AccountId,
    options: QueryOptions = {}
  ): Promise<bigint> {
    // Don't cache nonces - they change frequently
    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      const nonce = await (queryAt as any).query.atlasKernel.comitNonces(account);
      return BigInt(nonce.toString());
    } catch (error) {
      throw new RpcError(
        'getNonce',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  // ===========================================================================
  // Authority Queries
  // ===========================================================================

  /**
   * Get current authorities
   */
  async getAuthorities(options: QueryOptions = {}): Promise<AccountId[]> {
    const cacheKey = 'authorities';

    if (options.useCache !== false) {
      const cached = this.getFromCache<AccountId[]>(cacheKey);
      if (cached !== undefined) {
        return cached;
      }
    }

    try {
      const queryAt = options.at
        ? await this.api.at(options.at)
        : this.api;

      // Authorities are typically stored in session or aura pallet
      const authorities = await (queryAt as any).query.aura.authorities();
      const result = authorities.map((a: any) => a.toString());

      this.setCache(cacheKey, result);
      return result;
    } catch (error) {
      throw new RpcError(
        'getAuthorities',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  // ===========================================================================
  // Block Queries
  // ===========================================================================

  /**
   * Get current block number
   */
  async getBlockNumber(): Promise<BlockNumber> {
    const header = await this.api.rpc.chain.getHeader();
    this.currentBlockNumber = header.number.toNumber();
    return this.currentBlockNumber;
  }

  /**
   * Get block hash for a given number
   */
  async getBlockHash(blockNumber: BlockNumber): Promise<Hash> {
    const hash = await this.api.rpc.chain.getBlockHash(blockNumber);
    return hash.toHex() as Hash;
  }

  /**
   * Get finalized block number
   */
  async getFinalizedBlockNumber(): Promise<BlockNumber> {
    const hash = await this.api.rpc.chain.getFinalizedHead();
    const header = await this.api.rpc.chain.getHeader(hash);
    return header.number.toNumber();
  }

  // ===========================================================================
  // Cache Management
  // ===========================================================================

  /**
   * Clear all cached data
   */
  clearCache(): void {
    this.cache.clear();
  }

  /**
   * Clear cache for specific account
   */
  clearAccountCache(account: AccountId): void {
    for (const key of this.cache.keys()) {
      if (key.includes(account)) {
        this.cache.delete(key);
      }
    }
  }

  /**
   * Update the current block number (for cache validation)
   */
  updateBlockNumber(blockNumber: BlockNumber): void {
    if (blockNumber > this.currentBlockNumber) {
      this.currentBlockNumber = blockNumber;
      // Invalidate old entries
      this.pruneCache();
    }
  }

  // ===========================================================================
  // Private Methods
  // ===========================================================================

  private getFromCache<T>(key: string): T | undefined {
    const entry = this.cache.get(key);
    if (!entry) return undefined;

    const age = Date.now() - entry.timestamp;
    if (age > this.cacheMaxAge) {
      this.cache.delete(key);
      return undefined;
    }

    return entry.value as T;
  }

  private setCache<T>(key: string, value: T): void {
    this.cache.set(key, {
      value,
      blockNumber: this.currentBlockNumber,
      timestamp: Date.now(),
    });
  }

  private pruneCache(): void {
    const now = Date.now();
    for (const [key, entry] of this.cache.entries()) {
      if (now - entry.timestamp > this.cacheMaxAge) {
        this.cache.delete(key);
      }
    }
  }
}

// =============================================================================
// Factory Functions
// =============================================================================

/**
 * Create a QueryClient from an AtlasSphereClient
 */
export function createQueryClient(
  api: ApiPromise,
  cacheMaxAgeMs?: number
): QueryClient {
  return new QueryClient(api, cacheMaxAgeMs);
}
