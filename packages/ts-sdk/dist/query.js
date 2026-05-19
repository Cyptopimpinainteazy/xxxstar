"use strict";
/**
 * QueryClient - Specialized client for querying X3 Chain state
 *
 * Provides efficient read-only access to blockchain state with caching.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.QueryClient = void 0;
exports.createQueryClient = createQueryClient;
const errors_1 = require("./errors");
const constants_1 = require("./constants");
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
class QueryClient {
    api;
    cache = new Map();
    cacheMaxAge;
    currentBlockNumber = 0;
    /**
     * Create a new QueryClient
     *
     * @param api - Polkadot API instance
     * @param cacheMaxAgeMs - Maximum age for cached entries (default: 6000ms = 1 block)
     */
    constructor(api, cacheMaxAgeMs = 6000) {
        this.api = api;
        this.cacheMaxAge = cacheMaxAgeMs;
    }
    // ===========================================================================
    // Balance Queries
    // ===========================================================================
    /**
     * Get canonical ledger balance for an account
     */
    async getCanonicalBalance(account, assetId = constants_1.NATIVE_ASSET_ID, options = {}) {
        const cacheKey = `balance:${account}:${assetId}`;
        // Check cache
        if (options.useCache !== false) {
            const cached = this.getFromCache(cacheKey);
            if (cached !== undefined) {
                return cached;
            }
        }
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            const balance = await queryAt.query.atlasKernel.canonicalLedger(account, assetId);
            const value = BigInt(balance.toString());
            // Cache result
            this.setCache(cacheKey, value);
            return value;
        }
        catch (error) {
            throw new errors_1.RpcError('getCanonicalBalance', error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Get canonical balances for multiple accounts
     */
    async getCanonicalBalances(accounts, assetId = constants_1.NATIVE_ASSET_ID, options = {}) {
        const results = new Map();
        // Split into cached and uncached
        const uncached = [];
        if (options.useCache !== false) {
            for (const account of accounts) {
                const cached = this.getFromCache(`balance:${account}:${assetId}`);
                if (cached !== undefined) {
                    results.set(account, cached);
                }
                else {
                    uncached.push(account);
                }
            }
        }
        else {
            uncached.push(...accounts);
        }
        // Batch query uncached
        if (uncached.length > 0) {
            try {
                const queryAt = options.at
                    ? await this.api.at(options.at)
                    : this.api;
                const keys = uncached.map((account) => [account, assetId]);
                const balances = await queryAt.query.atlasKernel.canonicalLedger.multi(keys);
                for (let i = 0; i < uncached.length; i++) {
                    const value = BigInt(balances[i].toString());
                    results.set(uncached[i], value);
                    this.setCache(`balance:${uncached[i]}:${assetId}`, value);
                }
            }
            catch (error) {
                throw new errors_1.RpcError('getCanonicalBalances', error instanceof Error ? error.message : String(error));
            }
        }
        return results;
    }
    /**
     * Get all balances for an account across all assets
     */
    async getAllBalances(account, options = {}) {
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            // Query all entries with this account prefix
            const entries = await queryAt.query.atlasKernel.canonicalLedger.entries(account);
            return entries.map(([key, value]) => ({
                account,
                assetId: key.args[1].toNumber(),
                balance: BigInt(value.toString()),
            }));
        }
        catch (error) {
            throw new errors_1.RpcError('getAllBalances', error instanceof Error ? error.message : String(error));
        }
    }
    // ===========================================================================
    // Asset Queries
    // ===========================================================================
    /**
     * Get metadata for an asset
     */
    async getAssetMetadata(assetId, options = {}) {
        const cacheKey = `asset:${assetId}`;
        // Check cache (asset metadata rarely changes)
        if (options.useCache !== false) {
            const cached = this.getFromCache(cacheKey);
            if (cached !== undefined) {
                return cached;
            }
        }
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            const metadata = await queryAt.query.atlasKernel.assetMetadata(assetId);
            if (metadata.isNone) {
                this.setCache(cacheKey, null);
                return null;
            }
            const data = metadata.unwrap();
            const result = {
                symbol: data.symbol.toUtf8(),
                decimals: data.decimals.toNumber(),
            };
            this.setCache(cacheKey, result);
            return result;
        }
        catch (error) {
            throw new errors_1.RpcError('getAssetMetadata', error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Get all registered assets
     */
    async getAllAssets(options = {}) {
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            const entries = await queryAt.query.atlasKernel.assetMetadata.entries();
            return entries.map(([key, value]) => {
                const data = value.unwrap();
                return {
                    assetId: key.args[0].toNumber(),
                    symbol: data.symbol.toUtf8(),
                    decimals: data.decimals.toNumber(),
                };
            });
        }
        catch (error) {
            throw new errors_1.RpcError('getAllAssets', error instanceof Error ? error.message : String(error));
        }
    }
    // ===========================================================================
    // Authorization Queries
    // ===========================================================================
    /**
     * Check if an account is authorized
     */
    async isAuthorized(account, options = {}) {
        const cacheKey = `auth:${account}`;
        if (options.useCache !== false) {
            const cached = this.getFromCache(cacheKey);
            if (cached !== undefined) {
                return cached;
            }
        }
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            const result = await queryAt.query.atlasKernel.authorizedAccounts(account);
            const authorized = result.isSome;
            this.setCache(cacheKey, authorized);
            return authorized;
        }
        catch (error) {
            throw new errors_1.RpcError('isAuthorized', error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Get all authorized accounts
     */
    async getAuthorizedAccounts(pagination, options = {}) {
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            let entries = await queryAt.query.atlasKernel.authorizedAccounts.entries();
            // Apply pagination
            if (pagination?.offset) {
                entries = entries.slice(pagination.offset);
            }
            if (pagination?.limit) {
                entries = entries.slice(0, pagination.limit);
            }
            return entries.map(([key]) => key.args[0].toString());
        }
        catch (error) {
            throw new errors_1.RpcError('getAuthorizedAccounts', error instanceof Error ? error.message : String(error));
        }
    }
    // ===========================================================================
    // Nonce Queries
    // ===========================================================================
    /**
     * Get current nonce for an account
     */
    async getNonce(account, options = {}) {
        // Don't cache nonces - they change frequently
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            const nonce = await queryAt.query.atlasKernel.comitNonces(account);
            return BigInt(nonce.toString());
        }
        catch (error) {
            throw new errors_1.RpcError('getNonce', error instanceof Error ? error.message : String(error));
        }
    }
    // ===========================================================================
    // Authority Queries
    // ===========================================================================
    /**
     * Get current authorities
     */
    async getAuthorities(options = {}) {
        const cacheKey = 'authorities';
        if (options.useCache !== false) {
            const cached = this.getFromCache(cacheKey);
            if (cached !== undefined) {
                return cached;
            }
        }
        try {
            const queryAt = options.at
                ? await this.api.at(options.at)
                : this.api;
            // Authorities are typically stored in session or aura pallet
            const authorities = await queryAt.query.aura.authorities();
            const result = authorities.map((a) => a.toString());
            this.setCache(cacheKey, result);
            return result;
        }
        catch (error) {
            throw new errors_1.RpcError('getAuthorities', error instanceof Error ? error.message : String(error));
        }
    }
    // ===========================================================================
    // Block Queries
    // ===========================================================================
    /**
     * Get current block number
     */
    async getBlockNumber() {
        const header = await this.api.rpc.chain.getHeader();
        this.currentBlockNumber = header.number.toNumber();
        return this.currentBlockNumber;
    }
    /**
     * Get block hash for a given number
     */
    async getBlockHash(blockNumber) {
        const hash = await this.api.rpc.chain.getBlockHash(blockNumber);
        return hash.toHex();
    }
    /**
     * Get finalized block number
     */
    async getFinalizedBlockNumber() {
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
    clearCache() {
        this.cache.clear();
    }
    /**
     * Clear cache for specific account
     */
    clearAccountCache(account) {
        for (const key of this.cache.keys()) {
            if (key.includes(account)) {
                this.cache.delete(key);
            }
        }
    }
    /**
     * Update the current block number (for cache validation)
     */
    updateBlockNumber(blockNumber) {
        if (blockNumber > this.currentBlockNumber) {
            this.currentBlockNumber = blockNumber;
            // Invalidate old entries
            this.pruneCache();
        }
    }
    // ===========================================================================
    // Private Methods
    // ===========================================================================
    getFromCache(key) {
        const entry = this.cache.get(key);
        if (!entry)
            return undefined;
        const age = Date.now() - entry.timestamp;
        if (age > this.cacheMaxAge) {
            this.cache.delete(key);
            return undefined;
        }
        return entry.value;
    }
    setCache(key, value) {
        this.cache.set(key, {
            value,
            blockNumber: this.currentBlockNumber,
            timestamp: Date.now(),
        });
    }
    pruneCache() {
        const now = Date.now();
        for (const [key, entry] of this.cache.entries()) {
            if (now - entry.timestamp > this.cacheMaxAge) {
                this.cache.delete(key);
            }
        }
    }
}
exports.QueryClient = QueryClient;
// =============================================================================
// Factory Functions
// =============================================================================
/**
 * Create a QueryClient from an AtlasSphereClient
 */
function createQueryClient(api, cacheMaxAgeMs) {
    return new QueryClient(api, cacheMaxAgeMs);
}
//# sourceMappingURL=query.js.map