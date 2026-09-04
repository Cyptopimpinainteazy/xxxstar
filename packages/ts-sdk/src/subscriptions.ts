/**
 * WebSocket Subscription Module for X3 Chain
 *
 * Provides real-time event subscriptions via the x3_subscribe* RPC methods.
 * Supports new block notifications, finalized blocks, and comit event streams.
 */

import { WsProvider } from '@polkadot/api';

// =============================================================================
// Types
// =============================================================================

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
export class X3SubscriptionManager {
  private endpoint: string;
  private provider: WsProvider | null = null;
  private subscriptionIds: Map<string, string> = new Map();
  private handlers: SubscriptionHandlers = {};
  private _connected = false;

  constructor(endpoint: string = 'ws://127.0.0.1:9944') {
    this.endpoint = endpoint;
  }

  /** Whether the manager is currently connected */
  get connected(): boolean {
    return this._connected;
  }

  /**
   * Connect to the X3 Chain WebSocket endpoint
   */
  async connect(): Promise<void> {
    if (this._connected) return;

    this.provider = new WsProvider(this.endpoint, 1000);

    // Wait for connection
    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`Connection timeout to ${this.endpoint}`));
      }, 10_000);

      this.provider!.on('connected', () => {
        clearTimeout(timeout);
        this._connected = true;
        this.handlers.onConnected?.();
        resolve();
      });

      this.provider!.on('disconnected', () => {
        this._connected = false;
        this.handlers.onDisconnected?.();
      });

      this.provider!.on('error', (error: Error) => {
        this.handlers.onError?.(error);
      });
    });
  }

  /**
   * Disconnect and clean up all subscriptions
   */
  async disconnect(): Promise<void> {
    // Unsubscribe from all active subscriptions
    for (const [name, subId] of this.subscriptionIds) {
      try {
        const unsubMethod = this.getUnsubMethod(name);
        await this.provider?.send(unsubMethod, [subId]);
      } catch {
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
  setHandlers(handlers: SubscriptionHandlers): void {
    this.handlers = { ...this.handlers, ...handlers };
  }

  /**
   * Subscribe to new block headers as they are imported
   */
  async subscribeNewBlocks(
    callback: (block: BlockNotification) => void
  ): Promise<string> {
    this.ensureConnected();

    const subId = await this.provider!.subscribe(
      'x3_newBlock',
      'x3_subscribeNewBlocks',
      [],
      (error: Error | null, result: any) => {
        if (error) {
          this.handlers.onError?.(error);
          return;
        }
        callback(result as BlockNotification);
      }
    );

    const id = `newBlocks_${Date.now()}`;
    this.subscriptionIds.set(id, String(subId));
    return id;
  }

  /**
   * Subscribe to finalized block headers
   */
  async subscribeFinalizedBlocks(
    callback: (block: BlockNotification) => void
  ): Promise<string> {
    this.ensureConnected();

    const subId = await this.provider!.subscribe(
      'x3_finalizedBlock',
      'x3_subscribeFinalizedBlocks',
      [],
      (error: Error | null, result: any) => {
        if (error) {
          this.handlers.onError?.(error);
          return;
        }
        callback(result as BlockNotification);
      }
    );

    const id = `finalizedBlocks_${Date.now()}`;
    this.subscriptionIds.set(id, String(subId));
    return id;
  }

  /**
   * Subscribe to new comit (transaction) events
   */
  async subscribeComits(
    callback: (comit: ComitNotification) => void
  ): Promise<string> {
    this.ensureConnected();

    const subId = await this.provider!.subscribe(
      'x3_newComit',
      'x3_subscribeComits',
      [],
      (error: Error | null, result: any) => {
        if (error) {
          this.handlers.onError?.(error);
          return;
        }
        callback(result as ComitNotification);
      }
    );

    const id = `comits_${Date.now()}`;
    this.subscriptionIds.set(id, String(subId));
    return id;
  }

  /**
   * Unsubscribe from a specific subscription
   */
  async unsubscribe(subscriptionId: string): Promise<boolean> {
    const subId = this.subscriptionIds.get(subscriptionId);
    if (!subId) return false;

    try {
      const unsubMethod = this.getUnsubMethod(subscriptionId);
      await this.provider?.send(unsubMethod, [subId]);
    } catch {
      // Best-effort unsubscribe
    }

    this.subscriptionIds.delete(subscriptionId);
    return true;
  }

  /**
   * Get the count of active subscriptions
   */
  get activeSubscriptionCount(): number {
    return this.subscriptionIds.size;
  }

  // ===========================================================================
  // Private
  // ===========================================================================

  private ensureConnected(): void {
    if (!this._connected || !this.provider) {
      throw new Error('Not connected. Call connect() first.');
    }
  }

  private getUnsubMethod(subscriptionId: string): string {
    if (subscriptionId.startsWith('newBlocks')) return 'x3_unsubscribeNewBlocks';
    if (subscriptionId.startsWith('finalizedBlocks')) return 'x3_unsubscribeFinalizedBlocks';
    if (subscriptionId.startsWith('comits')) return 'x3_unsubscribeComits';
    return 'x3_unsubscribeNewBlocks'; // fallback
  }
}
