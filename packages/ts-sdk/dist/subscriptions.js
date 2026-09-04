"use strict";
/**
 * WebSocket Subscription Module for X3 Chain
 *
 * Provides real-time event subscriptions via the x3_subscribe* RPC methods.
 * Supports new block notifications, finalized blocks, and comit event streams.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.X3SubscriptionManager = void 0;
const api_1 = require("@polkadot/api");
// =============================================================================
// X3SubscriptionManager
// =============================================================================
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
class X3SubscriptionManager {
    endpoint;
    provider = null;
    subscriptionIds = new Map();
    handlers = {};
    _connected = false;
    constructor(endpoint = 'ws://127.0.0.1:9944') {
        this.endpoint = endpoint;
    }
    /** Whether the manager is currently connected */
    get connected() {
        return this._connected;
    }
    /**
     * Connect to the X3 Chain WebSocket endpoint
     */
    async connect() {
        if (this._connected)
            return;
        this.provider = new api_1.WsProvider(this.endpoint, 1000);
        // Wait for connection
        await new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new Error(`Connection timeout to ${this.endpoint}`));
            }, 10_000);
            this.provider.on('connected', () => {
                clearTimeout(timeout);
                this._connected = true;
                this.handlers.onConnected?.();
                resolve();
            });
            this.provider.on('disconnected', () => {
                this._connected = false;
                this.handlers.onDisconnected?.();
            });
            this.provider.on('error', (error) => {
                this.handlers.onError?.(error);
            });
        });
    }
    /**
     * Disconnect and clean up all subscriptions
     */
    async disconnect() {
        // Unsubscribe from all active subscriptions
        for (const [name, subId] of this.subscriptionIds) {
            try {
                const unsubMethod = this.getUnsubMethod(name);
                await this.provider?.send(unsubMethod, [subId]);
            }
            catch {
                // Ignore unsubscribe errors during cleanup
            }
        }
        this.subscriptionIds.clear();
        if (this.provider) {
            await this.provider.disconnect();
            this.provider = null;
        }
        this._connected = false;
    }
    /**
     * Set event handlers for all subscription types
     */
    setHandlers(handlers) {
        this.handlers = { ...this.handlers, ...handlers };
    }
    /**
     * Subscribe to new block headers as they are imported
     */
    async subscribeNewBlocks(callback) {
        this.ensureConnected();
        const subId = await this.provider.subscribe('x3_newBlock', 'x3_subscribeNewBlocks', [], (error, result) => {
            if (error) {
                this.handlers.onError?.(error);
                return;
            }
            callback(result);
        });
        const id = `newBlocks_${Date.now()}`;
        this.subscriptionIds.set(id, String(subId));
        return id;
    }
    /**
     * Subscribe to finalized block headers
     */
    async subscribeFinalizedBlocks(callback) {
        this.ensureConnected();
        const subId = await this.provider.subscribe('x3_finalizedBlock', 'x3_subscribeFinalizedBlocks', [], (error, result) => {
            if (error) {
                this.handlers.onError?.(error);
                return;
            }
            callback(result);
        });
        const id = `finalizedBlocks_${Date.now()}`;
        this.subscriptionIds.set(id, String(subId));
        return id;
    }
    /**
     * Subscribe to new comit (transaction) events
     */
    async subscribeComits(callback) {
        this.ensureConnected();
        const subId = await this.provider.subscribe('x3_newComit', 'x3_subscribeComits', [], (error, result) => {
            if (error) {
                this.handlers.onError?.(error);
                return;
            }
            callback(result);
        });
        const id = `comits_${Date.now()}`;
        this.subscriptionIds.set(id, String(subId));
        return id;
    }
    /**
     * Unsubscribe from a specific subscription
     */
    async unsubscribe(subscriptionId) {
        const subId = this.subscriptionIds.get(subscriptionId);
        if (!subId)
            return false;
        try {
            const unsubMethod = this.getUnsubMethod(subscriptionId);
            await this.provider?.send(unsubMethod, [subId]);
        }
        catch {
            // Best-effort unsubscribe
        }
        this.subscriptionIds.delete(subscriptionId);
        return true;
    }
    /**
     * Get the count of active subscriptions
     */
    get activeSubscriptionCount() {
        return this.subscriptionIds.size;
    }
    // ===========================================================================
    // Private
    // ===========================================================================
    ensureConnected() {
        if (!this._connected || !this.provider) {
            throw new Error('Not connected. Call connect() first.');
        }
    }
    getUnsubMethod(subscriptionId) {
        if (subscriptionId.startsWith('newBlocks'))
            return 'x3_unsubscribeNewBlocks';
        if (subscriptionId.startsWith('finalizedBlocks'))
            return 'x3_unsubscribeFinalizedBlocks';
        if (subscriptionId.startsWith('comits'))
            return 'x3_unsubscribeComits';
        return 'x3_unsubscribeNewBlocks'; // fallback
    }
}
exports.X3SubscriptionManager = X3SubscriptionManager;
//# sourceMappingURL=subscriptions.js.map