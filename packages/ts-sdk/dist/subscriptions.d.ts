/**
 * WebSocket Subscription Module for X3 Chain
 *
 * Provides real-time event subscriptions via the x3_subscribe* RPC methods.
 * Supports new block notifications, finalized blocks, and comit event streams.
 */
/** Block notification from x3_subscribeNewBlocks / x3_subscribeFinalizedBlocks */
export interface BlockNotification {
    /** Block number */
    number: number;
    /** Block hash (hex) */
    hash: string;
    /** Parent block hash (hex) */
    parentHash: string;
    /** State root (hex) */
    stateRoot: string;
    /** Extrinsics root (hex) */
    extrinsicsRoot: string;
}
/** Comit notification from x3_subscribeComits */
export interface ComitNotification {
    /** Block number that included this comit */
    blockNumber: number;
    /** Block hash (hex) */
    blockHash: string;
    /** Index within the block */
    extrinsicIndex: number;
    /** Extrinsic hash (hex) */
    hash: string;
}
/** EVM log notification */
export interface EvmLogNotification {
    /** Block number */
    blockNumber: number;
    /** Contract address (hex) */
    address: string;
    /** Log topics (hex array) */
    topics: string[];
    /** Log data (hex) */
    data: string;
}
/** Subscription event handlers */
export interface SubscriptionHandlers {
    /** Called when a new block is imported */
    onNewBlock?: (block: BlockNotification) => void;
    /** Called when a block is finalized */
    onFinalizedBlock?: (block: BlockNotification) => void;
    /** Called when a new comit is observed */
    onNewComit?: (comit: ComitNotification) => void;
    /** Called on subscription errors */
    onError?: (error: Error) => void;
    /** Called when connected */
    onConnected?: () => void;
    /** Called when disconnected */
    onDisconnected?: () => void;
}
/**
 * Manages WebSocket subscriptions to X3 Chain RPC events.
 *
 * @example
 * ```typescript
 * const sub = new X3SubscriptionManager('ws://localhost:9944');
 * await sub.connect();
 *
 * sub.subscribeNewBlocks((block) => {
 *   console.log('New block:', block.number, block.hash);
 * });
 *
 * sub.subscribeComits((comit) => {
 *   console.log('New comit:', comit.hash, 'in block', comit.blockNumber);
 * });
 *
 * // Cleanup
 * await sub.disconnect();
 * ```
 */
export declare class X3SubscriptionManager {
    private endpoint;
    private provider;
    private subscriptionIds;
    private handlers;
    private _connected;
    constructor(endpoint?: string);
    /** Whether the manager is currently connected */
    get connected(): boolean;
    /**
     * Connect to the X3 Chain WebSocket endpoint
     */
    connect(): Promise<void>;
    /**
     * Disconnect and clean up all subscriptions
     */
    disconnect(): Promise<void>;
    /**
     * Set event handlers for all subscription types
     */
    setHandlers(handlers: SubscriptionHandlers): void;
    /**
     * Subscribe to new block headers as they are imported
     */
    subscribeNewBlocks(callback: (block: BlockNotification) => void): Promise<string>;
    /**
     * Subscribe to finalized block headers
     */
    subscribeFinalizedBlocks(callback: (block: BlockNotification) => void): Promise<string>;
    /**
     * Subscribe to new comit (transaction) events
     */
    subscribeComits(callback: (comit: ComitNotification) => void): Promise<string>;
    /**
     * Unsubscribe from a specific subscription
     */
    unsubscribe(subscriptionId: string): Promise<boolean>;
    /**
     * Get the count of active subscriptions
     */
    get activeSubscriptionCount(): number;
    private ensureConnected;
    private getUnsubMethod;
}
//# sourceMappingURL=subscriptions.d.ts.map