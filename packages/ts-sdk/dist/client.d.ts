/**
 * AtlasSphereClient - Main client for interacting with X3 Chain blockchain
 *
 * Provides connection management, transaction submission, and query capabilities.
 */
import { ApiPromise } from '@polkadot/api';
import type { SubmittableExtrinsic } from '@polkadot/api/types';
import type { Signer } from '@polkadot/types/types';
import type { AccountId, AssetId, Balance, Hash, BlockNumber, Nonce, ComitResult, ComitInput, AssetMetadata, BlockSubscriptionCallback, ComitEventCallback } from './types';
/**
 * Client configuration options
 */
export interface AtlasSphereClientConfig {
    /** WebSocket or HTTP endpoint URL */
    endpoint?: string;
    /** Use WebSocket (true) or HTTP (false) */
    useWebSocket?: boolean;
    /** Timeout for RPC calls in milliseconds */
    rpcTimeoutMs?: number;
    /** Timeout for Comit finalization in milliseconds */
    finalizationTimeoutMs?: number;
    /** Auto-reconnect on WebSocket disconnect */
    autoReconnect?: boolean;
    /** Custom signer for transaction signing */
    signer?: Signer;
}
/**
 * Connection status
 */
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';
/**
 * Chain information
 */
export interface ChainInfo {
    name: string;
    version: string;
    properties: {
        tokenSymbol: string;
        tokenDecimals: number;
        ss58Format: number;
    };
}
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
export declare class AtlasSphereClient {
    private api;
    private provider;
    private config;
    private _status;
    private subscriptions;
    constructor(config?: AtlasSphereClientConfig);
    /**
     * Get current connection status
     */
    get status(): ConnectionStatus;
    /**
     * Check if client is connected
     */
    get isConnected(): boolean;
    /**
     * Get the underlying Polkadot API instance
     */
    get polkadotApi(): ApiPromise;
    /**
     * Connect to the X3 Chain node
     */
    connect(): Promise<void>;
    /**
     * Disconnect from the node
     */
    disconnect(): Promise<void>;
    /**
     * Get chain information
     */
    getChainInfo(): Promise<ChainInfo>;
    /**
     * Get native balance for an account
     */
    getBalance(account: AccountId): Promise<Balance>;
    /**
     * Get canonical ledger balance for an account and asset
     */
    getCanonicalBalance(account: AccountId, assetId?: AssetId): Promise<Balance>;
    /**
     * Get asset metadata
     */
    getAssetMetadata(assetId: AssetId): Promise<AssetMetadata | null>;
    /**
     * Check if an account is authorized to submit Comits
     */
    isAuthorized(account: AccountId): Promise<boolean>;
    /**
     * Get all authorized accounts
     */
    getAuthorizedAccounts(): Promise<AccountId[]>;
    /**
     * Get current nonce for an account
     */
    getNonce(account: AccountId): Promise<Nonce>;
    /**
     * Get current block number
     */
    getBlockNumber(): Promise<BlockNumber>;
    /**
     * Get finalized block number
     */
    getFinalizedBlockNumber(): Promise<BlockNumber>;
    /**
     * Submit a Comit transaction
     *
     * @param input - Comit input parameters
     * @param signerAccount - Account to sign with (must have signer configured)
     * @returns Promise resolving to ComitResult when finalized
     */
    submitComit(input: ComitInput, signerAccount: AccountId): Promise<ComitResult>;
    /**
     * Create an unsigned Comit extrinsic (for offline signing)
     */
    createComitExtrinsic(evmPayload: Uint8Array, svmPayload: Uint8Array, fee: Balance, prepareRoot: Hash): SubmittableExtrinsic<'promise'>;
    /**
     * Subscribe to new blocks
     */
    subscribeNewBlocks(callback: BlockSubscriptionCallback): Promise<string>;
    /**
     * Subscribe to finalized blocks
     */
    subscribeFinalizedBlocks(callback: BlockSubscriptionCallback): Promise<string>;
    /**
     * Subscribe to Comit events for a specific account
     */
    subscribeComitEvents(account: AccountId, callback: ComitEventCallback): Promise<string>;
    /**
     * Unsubscribe from a subscription
     */
    unsubscribe(subscriptionId: string): Promise<boolean>;
    private ensureConnected;
    private submitAndWaitForFinalization;
    private parseComitEvent;
    private parseFailureReason;
}
/**
 * Create and connect a client in one call
 */
export declare function createClient(config?: AtlasSphereClientConfig): Promise<AtlasSphereClient>;
/**
 * Create a client for local development
 */
export declare function createLocalClient(): Promise<AtlasSphereClient>;
/**
 * Create a client for testnet
 */
export declare function createTestnetClient(): Promise<AtlasSphereClient>;
//# sourceMappingURL=client.d.ts.map