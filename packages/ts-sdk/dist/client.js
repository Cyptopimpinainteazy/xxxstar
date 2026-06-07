"use strict";
/**
 * AtlasSphereClient - Main client for interacting with X3 Chain blockchain
 *
 * Provides connection management, transaction submission, and query capabilities.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.AtlasSphereClient = void 0;
exports.createClient = createClient;
exports.createLocalClient = createLocalClient;
exports.createTestnetClient = createTestnetClient;
const api_1 = require("@polkadot/api");
const errors_1 = require("./errors");
const constants_1 = require("./constants");
const utils_1 = require("./utils");
// =============================================================================
// AtlasSphereClient
// =============================================================================
/**
 * Main client for X3 Chain blockchain interaction
 *
 * @example
 * ```typescript
 * const client = new AtlasSphereClient({ endpoint: 'ws://localhost:9944' });
 * await client.connect();
 *
 * // Query balance
 * const balance = await client.getBalance(accountId);
 *
 * // Submit a Comit
 * const result = await client.submitComit(comitInput, signer);
 *
 * await client.disconnect();
 * ```
 */
class AtlasSphereClient {
    api = null;
    provider = null;
    config;
    _status = 'disconnected';
    subscriptions = new Map();
    constructor(config = {}) {
        this.config = {
            endpoint: config.endpoint ?? constants_1.DEFAULT_WS_ENDPOINT,
            useWebSocket: config.useWebSocket ?? true,
            rpcTimeoutMs: config.rpcTimeoutMs ?? constants_1.DEFAULT_RPC_TIMEOUT_MS,
            finalizationTimeoutMs: config.finalizationTimeoutMs ?? constants_1.DEFAULT_FINALIZATION_TIMEOUT_MS,
            autoReconnect: config.autoReconnect ?? true,
            signer: config.signer ?? undefined,
        };
    }
    // ===========================================================================
    // Connection Management
    // ===========================================================================
    /**
     * Get current connection status
     */
    get status() {
        return this._status;
    }
    /**
     * Check if client is connected
     */
    get isConnected() {
        return this._status === 'connected' && this.api !== null;
    }
    /**
     * Get the underlying Polkadot API instance
     */
    get polkadotApi() {
        if (!this.api) {
            throw new errors_1.ConnectionError(this.config.endpoint, new Error('Not connected'));
        }
        return this.api;
    }
    /**
     * Connect to the X3 Chain node
     */
    async connect() {
        if (this.isConnected) {
            return;
        }
        this._status = 'connecting';
        try {
            // Create provider
            if (this.config.useWebSocket) {
                this.provider = new api_1.WsProvider(this.config.endpoint, this.config.autoReconnect ? 1000 : false);
            }
            else {
                this.provider = new api_1.HttpProvider(this.config.endpoint);
            }
            // Create API instance
            this.api = await api_1.ApiPromise.create({
                provider: this.provider,
                throwOnConnect: true,
            });
            // Set up event handlers
            this.api.on('connected', () => {
                this._status = 'connected';
            });
            this.api.on('disconnected', () => {
                this._status = 'disconnected';
            });
            this.api.on('error', () => {
                this._status = 'error';
            });
            this._status = 'connected';
        }
        catch (error) {
            this._status = 'error';
            throw new errors_1.ConnectionError(this.config.endpoint, error instanceof Error ? error : new Error(String(error)));
        }
    }
    /**
     * Disconnect from the node
     */
    async disconnect() {
        // Unsubscribe from all subscriptions
        for (const [id, unsubscribe] of this.subscriptions) {
            try {
                unsubscribe();
            }
            catch {
                // Ignore unsubscribe errors
            }
            this.subscriptions.delete(id);
        }
        if (this.api) {
            await this.api.disconnect();
            this.api = null;
        }
        if (this.provider && this.provider instanceof api_1.WsProvider) {
            await this.provider.disconnect();
        }
        this.provider = null;
        this._status = 'disconnected';
    }
    /**
     * Get chain information
     */
    async getChainInfo() {
        this.ensureConnected();
        const [chain, version, properties] = await Promise.all([
            this.api.rpc.system.chain(),
            this.api.rpc.system.version(),
            this.api.rpc.system.properties(),
        ]);
        return {
            name: chain.toString(),
            version: version.toString(),
            properties: {
                tokenSymbol: properties.tokenSymbol.unwrapOr(['X3'])[0].toString(),
                tokenDecimals: Number(properties.tokenDecimals.unwrapOr([18])[0]),
                ss58Format: Number(properties.ss58Format.unwrapOr(42)),
            },
        };
    }
    // ===========================================================================
    // Query Methods
    // ===========================================================================
    /**
     * Get native balance for an account
     */
    async getBalance(account) {
        this.ensureConnected();
        const accountInfo = await this.api.query.system.account(account);
        return BigInt(accountInfo.data.free.toString());
    }
    /**
     * Get canonical ledger balance for an account and asset
     */
    async getCanonicalBalance(account, assetId = constants_1.NATIVE_ASSET_ID) {
        this.ensureConnected();
        try {
            const result = await this.api.rpc.state.call('AtlasKernelApi_get_canonical_balance', this.api.createType('(AccountId, u32)', [account, assetId]).toHex());
            return BigInt(this.api.createType('u128', result).toString());
        }
        catch (error) {
            throw new errors_1.RpcError(constants_1.RPC_METHODS.getCanonicalBalance, error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Get asset metadata
     */
    async getAssetMetadata(assetId) {
        this.ensureConnected();
        try {
            const metadata = await this.api.query.atlasKernel.assetMetadata(assetId);
            if (metadata.isNone) {
                return null;
            }
            const data = metadata.unwrap();
            return {
                symbol: data.symbol.toUtf8(),
                decimals: data.decimals.toNumber(),
            };
        }
        catch (error) {
            throw new errors_1.RpcError(constants_1.RPC_METHODS.getAssetMetadata, error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Check if an account is authorized to submit Comits
     */
    async isAuthorized(account) {
        this.ensureConnected();
        try {
            const result = await this.api.query.atlasKernel.authorizedAccounts(account);
            return result.isSome;
        }
        catch (error) {
            throw new errors_1.RpcError(constants_1.RPC_METHODS.isAuthorized, error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Get all authorized accounts
     */
    async getAuthorizedAccounts() {
        this.ensureConnected();
        try {
            const entries = await this.api.query.atlasKernel.authorizedAccounts.entries();
            return entries.map(([key]) => key.args[0].toString());
        }
        catch (error) {
            throw new errors_1.RpcError(constants_1.RPC_METHODS.getAuthorizedAccounts, error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Get current nonce for an account
     */
    async getNonce(account) {
        this.ensureConnected();
        try {
            const apiAny = this.api;
            const kernelQuery = apiAny.query?.atlasKernel?.comitNonces;
            if (typeof kernelQuery === 'function') {
                const nonce = await kernelQuery(account);
                return BigInt(nonce.toString());
            }
            const systemAccount = await this.api.query.system.account(account);
            const accountNonce = systemAccount.nonce;
            if (accountNonce !== undefined && accountNonce !== null) {
                return BigInt(accountNonce.toString());
            }
            const nextIndex = await this.api.rpc.system.accountNextIndex(account);
            return BigInt(nextIndex.toString());
        }
        catch (error) {
            throw new errors_1.RpcError('getNonce', error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Get current block number
     */
    async getBlockNumber() {
        this.ensureConnected();
        const header = await this.api.rpc.chain.getHeader();
        return header.number.toNumber();
    }
    /**
     * Get finalized block number
     */
    async getFinalizedBlockNumber() {
        this.ensureConnected();
        const hash = await this.api.rpc.chain.getFinalizedHead();
        const header = await this.api.rpc.chain.getHeader(hash);
        return header.number.toNumber();
    }
    // ===========================================================================
    // Transaction Methods
    // ===========================================================================
    /**
     * Submit a Comit transaction
     *
     * @param input - Comit input parameters
     * @param signerAccount - Account to sign with (must have signer configured)
     * @returns Promise resolving to ComitResult when finalized
     */
    async submitComit(input, signerAccount) {
        this.ensureConnected();
        // Validate authorization
        const authorized = await this.isAuthorized(signerAccount);
        if (!authorized) {
            throw new errors_1.UnauthorizedError(signerAccount);
        }
        // Prepare payloads
        const evmPayload = input.evmPayload ? (0, utils_1.toBytes)(input.evmPayload) : new Uint8Array(0);
        const svmPayload = input.svmPayload ? (0, utils_1.toBytes)(input.svmPayload) : new Uint8Array(0);
        // Validate sizes
        (0, utils_1.validatePayloadSizes)(evmPayload, svmPayload);
        // Validate fee
        (0, utils_1.validateBalance)(input.fee, 'fee');
        // Get current nonce
        const nonce = await this.getNonce(signerAccount);
        // Compute prepare_root
        const prepareRoot = input.prepareRoot ?? (0, utils_1.computePrepareRoot)(signerAccount, evmPayload, svmPayload, nonce, input.fee);
        // Compute comit_id
        const comitId = (0, utils_1.computeComitId)(prepareRoot);
        // Create extrinsic
        const extrinsic = this.api.tx.atlasKernel.submitComit(evmPayload, svmPayload, input.fee.toString(), prepareRoot);
        // Submit and wait for finalization
        return this.submitAndWaitForFinalization(extrinsic, signerAccount, {
            comitId,
            origin: signerAccount,
            evmPayload,
            svmPayload,
            nonce,
            fee: input.fee,
            prepareRoot,
        });
    }
    /**
     * Create an unsigned Comit extrinsic (for offline signing)
     */
    createComitExtrinsic(evmPayload, svmPayload, fee, prepareRoot) {
        this.ensureConnected();
        return this.api.tx.atlasKernel.submitComit(evmPayload, svmPayload, fee.toString(), prepareRoot);
    }
    // ===========================================================================
    // Subscription Methods
    // ===========================================================================
    /**
     * Subscribe to new blocks
     */
    async subscribeNewBlocks(callback) {
        this.ensureConnected();
        const subscriptionId = `block_${Date.now()}`;
        try {
            const unsub = await this.api.rpc.chain.subscribeNewHeads((header) => {
                callback(header.number.toNumber(), header.hash.toHex());
            });
            this.subscriptions.set(subscriptionId, unsub);
            return subscriptionId;
        }
        catch (error) {
            throw new errors_1.SubscriptionError('newBlocks', error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Subscribe to finalized blocks
     */
    async subscribeFinalizedBlocks(callback) {
        this.ensureConnected();
        const subscriptionId = `finalized_${Date.now()}`;
        try {
            const unsub = await this.api.rpc.chain.subscribeFinalizedHeads((header) => {
                callback(header.number.toNumber(), header.hash.toHex());
            });
            this.subscriptions.set(subscriptionId, unsub);
            return subscriptionId;
        }
        catch (error) {
            throw new errors_1.SubscriptionError('finalizedBlocks', error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Subscribe to Comit events for a specific account
     */
    async subscribeComitEvents(account, callback) {
        this.ensureConnected();
        const subscriptionId = `comit_${account}_${Date.now()}`;
        try {
            const unsub = await this.api.query.system.events((events) => {
                events.forEach((record) => {
                    const { event } = record;
                    // Check for x3 kernel events
                    if (event.section !== 'atlasKernel')
                        return;
                    // Parse and emit appropriate event type
                    const comitEvent = this.parseComitEvent(event, account);
                    if (comitEvent) {
                        callback(comitEvent);
                    }
                });
            });
            this.subscriptions.set(subscriptionId, unsub);
            return subscriptionId;
        }
        catch (error) {
            throw new errors_1.SubscriptionError('comitEvents', error instanceof Error ? error.message : String(error));
        }
    }
    /**
     * Unsubscribe from a subscription
     */
    async unsubscribe(subscriptionId) {
        const unsub = this.subscriptions.get(subscriptionId);
        if (unsub) {
            unsub();
            this.subscriptions.delete(subscriptionId);
            return true;
        }
        return false;
    }
    // ===========================================================================
    // Private Methods
    // ===========================================================================
    ensureConnected() {
        if (!this.isConnected) {
            throw new errors_1.ConnectionError(this.config.endpoint, new Error('Not connected'));
        }
    }
    async submitAndWaitForFinalization(extrinsic, account, comit) {
        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new errors_1.TimeoutError('comit finalization', this.config.finalizationTimeoutMs));
            }, this.config.finalizationTimeoutMs);
            extrinsic.signAndSend(account, async (result) => {
                if (result.status.isFinalized) {
                    clearTimeout(timeout);
                    const blockHash = result.status.asFinalized;
                    // Find the extrinsic index
                    let extrinsicIndex = 0;
                    // Look for events to get execution results
                    let evmReceipt = undefined;
                    let svmReceipt = undefined;
                    for (const { event, phase } of result.events) {
                        if (phase.isApplyExtrinsic) {
                            extrinsicIndex = phase.asApplyExtrinsic.toNumber();
                        }
                        // Check for execution completion events
                        if (event.section === 'atlasKernel') {
                            if (event.method === 'ComitExecutionCompleted') {
                                // Parse receipt data from event
                            }
                        }
                    }
                    // Query block header to get block number
                    const header = await this.api.rpc.chain.getHeader(blockHash);
                    const blockNumber = header.number.toNumber();
                    resolve({
                        comit,
                        evmReceipt,
                        svmReceipt,
                        sphereState: {
                            stateRoot: blockHash.toHex(),
                            blockNumber,
                            timestamp: Date.now(),
                        },
                        blockNumber,
                        blockHash: blockHash.toHex(),
                        extrinsicIndex,
                    });
                }
                if (result.status.isDropped || result.status.isInvalid) {
                    clearTimeout(timeout);
                    reject(new Error(`Transaction ${result.status.isDropped ? 'dropped' : 'invalid'}`));
                }
            }).catch((error) => {
                clearTimeout(timeout);
                reject(error);
            });
        });
    }
    parseComitEvent(event, filterAccount) {
        const method = event.method;
        switch (method) {
            case 'ComitSubmitted': {
                const [comitId, origin, nonce, fee] = event.data;
                if (filterAccount && origin.toString() !== filterAccount)
                    return null;
                return {
                    type: 'submitted',
                    data: {
                        comitId: comitId.toHex(),
                        origin: origin.toString(),
                        nonce: BigInt(nonce.toString()),
                        fee: BigInt(fee.toString()),
                    },
                };
            }
            case 'ComitExecutionStarted': {
                const [comitId, timestamp] = event.data;
                return {
                    type: 'executionStarted',
                    data: {
                        comitId: comitId.toHex(),
                        timestamp: timestamp.toNumber(),
                    },
                };
            }
            case 'ComitExecutionCompleted': {
                const [comitId, success, gasUsed] = event.data;
                return {
                    type: 'executionCompleted',
                    data: {
                        comitId: comitId.toHex(),
                        success: success.isTrue,
                        gasUsed: BigInt(gasUsed.toString()),
                    },
                };
            }
            case 'ComitFinalized': {
                const [comitId] = event.data;
                return {
                    type: 'finalized',
                    data: {
                        comitId: comitId.toHex(),
                    },
                };
            }
            case 'ComitFailed': {
                const [comitId, reason] = event.data;
                return {
                    type: 'failed',
                    data: {
                        comitId: comitId.toHex(),
                        reason: this.parseFailureReason(reason),
                    },
                };
            }
            default:
                return null;
        }
    }
    parseFailureReason(reason) {
        // Parse the codec enum to our type
        if (reason.isInvalidNonce) {
            const [expected, provided] = reason.asInvalidNonce;
            return {
                type: 'InvalidNonce',
                expected: BigInt(expected.toString()),
                provided: BigInt(provided.toString()),
            };
        }
        if (reason.isInsufficientBalance) {
            const [required, available] = reason.asInsufficientBalance;
            return {
                type: 'InsufficientBalance',
                required: BigInt(required.toString()),
                available: BigInt(available.toString()),
            };
        }
        if (reason.isUnauthorized) {
            return { type: 'Unauthorized' };
        }
        if (reason.isRateLimitExceeded) {
            return { type: 'RateLimitExceeded' };
        }
        if (reason.isDuplicateComitId) {
            return { type: 'DuplicateComitId' };
        }
        // Default for unknown reasons
        return { type: 'VerificationFailed', reason: reason.toString() };
    }
}
exports.AtlasSphereClient = AtlasSphereClient;
// =============================================================================
// Factory Functions
// =============================================================================
/**
 * Create and connect a client in one call
 */
async function createClient(config = {}) {
    const client = new AtlasSphereClient(config);
    await client.connect();
    return client;
}
/**
 * Create a client for local development
 */
async function createLocalClient() {
    return createClient({ endpoint: 'ws://127.0.0.1:9944' });
}
/**
 * Create a client for testnet
 */
async function createTestnetClient() {
    const endpoint = process.env.X3_RPC_ENDPOINT ?? constants_1.TESTNET_WS_ENDPOINT;
    return createClient({ endpoint });
}
//# sourceMappingURL=client.js.map