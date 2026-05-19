/**
 * X3Chain API Connection Manager with Retry Logic
 *
 * Handles WebSocket lifecycle, type registration, and API initialization
 * for the X3 Chain x3chain runtime. Includes automatic reconnection with
 * exponential backoff and environment variable configuration.
 */

import { ApiPromise, WsProvider } from '@polkadot/api';
import { EventEmitter } from 'eventemitter3';
import { X3ChainCustomTypes, X3ChainRpc } from '../types/runtime-types';
import type { X3ChainConfig, ConnectionState, X3Network } from '../types/interfaces';
import { getSdkConfig, getCurrentEndpoint, getCurrentNetwork } from '../config/env';

// Re-export network types for convenience
export { X3Network };

export interface ApiEvents {
  connected: (state: ConnectionState) => void;
  disconnected: () => void;
  error: (error: Error) => void;
  ready: (api: ApiPromise) => void;
  reconnecting: (attempt: number, delay: number) => void;
  reconnected: (state: ConnectionState) => void;
}

/**
 * Enhanced X3Chain API with automatic reconnection and retry logic
 */
export class X3ChainApi extends EventEmitter<ApiEvents> {
  private _api: ApiPromise | null = null;
  private _provider: WsProvider | null = null;
  private _config: X3ChainConfig;
  private _connectionState: ConnectionState | null = null;
  private _reconnectAttempts: number = 0;
  private _reconnectTimer: NodeJS.Timeout | null = null;
  private _isDisconnecting: boolean = false;

  constructor(config: X3ChainConfig = {}) {
    super();
    
    // Load environment configuration
    const envConfig = getSdkConfig();
    
    this._config = {
      autoConnect: true,
      timeout: 30_000,
      network: envConfig.network,
      endpoint: envConfig.endpoint,
      autoReconnect: envConfig.autoReconnect,
      reconnectMaxAttempts: envConfig.reconnectMaxAttempts,
      reconnectDelay: envConfig.reconnectDelay,
      ...config,
    };
  }

  /** Get the underlying Polkadot API instance */
  get api(): ApiPromise {
    if (!this._api) {
      throw new Error('API not connected. Call connect() first.');
    }
    return this._api;
  }

  /** Current connection state */
  get state(): ConnectionState | null {
    return this._connectionState;
  }

  /** Whether the API is connected */
  get isConnected(): boolean {
    return this._api?.isConnected ?? false;
  }

  /** Get current network */
  get network(): X3Network {
    return this._config.network || 'local';
  }

  /**
   * Connect to the x3chain node
   */
  async connect(): Promise<ApiPromise> {
    const endpoint = this._config.endpoint || getCurrentEndpoint();
    
    this._isDisconnecting = false;
    this._reconnectAttempts = 0;

    this._provider = new WsProvider(endpoint, this._config.autoReconnect ? 1000 : false);

    this._provider.on('disconnected', () => {
      if (!this._isDisconnecting) {
        this._handleDisconnect();
      } else {
        this._connectionState = null;
        this.emit('disconnected');
      }
    });

    this._provider.on('error', (err: Error) => {
      this.emit('error', err);
    });

    try {
      this._api = await ApiPromise.create({
        provider: this._provider,
        types: X3ChainCustomTypes,
        rpc: X3ChainRpc,
        signer: this._config.signer,
      });

      await this._api.isReady;

      const [chain, header] = await Promise.all([
        this._api.rpc.system.chain(),
        this._api.rpc.chain.getHeader(),
      ]);

      this._connectionState = {
        connected: true,
        endpoint,
        chainName: chain.toString(),
        genesisHash: this._api.genesisHash.toHex(),
        runtimeVersion: this._api.runtimeVersion.specVersion.toNumber(),
        latestBlock: header.number.toNumber(),
      };

      this.emit('connected', this._connectionState);
      this.emit('ready', this._api);

      return this._api;
    } catch (err) {
      this.emit('error', err as Error);
      throw err;
    }
  }

  /**
   * Handle disconnection with automatic reconnection
   */
  private _handleDisconnect(): void {
    if (this._isDisconnecting) return;

    const maxAttempts = this._config.reconnectMaxAttempts || 5;
    const baseDelay = this._config.reconnectDelay || 1000;

    if (this._reconnectAttempts < maxAttempts) {
      this._reconnectAttempts++;
      const delay = baseDelay * Math.pow(2, this._reconnectAttempts - 1); // Exponential backoff

      this.emit('reconnecting', this._reconnectAttempts, delay);

      this._reconnectTimer = setTimeout(() => {
        this._reconnect();
      }, delay);
    } else {
      this._connectionState = null;
      this.emit('disconnected');
    }
  }

  /**
   * Attempt to reconnect to the node
   */
  private async _reconnect(): Promise<void> {
    if (this._api) {
      await this._api.disconnect();
      this._api = null;
    }
    if (this._provider) {
      await this._provider.disconnect();
      this._provider = null;
    }

    try {
      await this.connect();
      if (this._connectionState) {
        this.emit('reconnected', this._connectionState);
      }
    } catch (err) {
      this.emit('error', err as Error);
      this._handleDisconnect();
    }
  }

  /**
   * Disconnect from the node
   */
  async disconnect(): Promise<void> {
    this._isDisconnecting = true;
    
    if (this._reconnectTimer) {
      clearTimeout(this._reconnectTimer);
      this._reconnectTimer = null;
    }

    if (this._api) {
      await this._api.disconnect();
      this._api = null;
    }
    if (this._provider) {
      await this._provider.disconnect();
      this._provider = null;
    }
    this._connectionState = null;
    this.emit('disconnected');
  }

  /**
   * Set a signer (for Polkawallet mobile extension bridge)
   */
  setSigner(signer: import('@polkadot/types/types').Signer): void {
    if (this._api) {
      this._api.setSigner(signer);
    }
    this._config.signer = signer;
  }

  /**
   * Get available account addresses from the connected signer/extension
   */
  async getAccounts(): Promise<string[]> {
    if (!this._api) throw new Error('Not connected');

    try {
      const { web3Accounts, web3Enable } = await import('@polkadot/extension-dapp');
      await web3Enable('X3 Chain x3chain');
      const accounts = await web3Accounts();
      return accounts.map((a: { address: string }) => a.address);
    } catch {
      return [];
    }
  }

  /**
   * Execute a query with retry logic
   */
  async executeWithRetry<T>(
    fn: () => Promise<T>,
    maxRetries: number = 3,
    delay: number = 1000
  ): Promise<T> {
    let lastError: Error | undefined;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        return await fn();
      } catch (err) {
        lastError = err as Error;
        
        if (attempt < maxRetries) {
          this.emit('error', new Error(`Attempt ${attempt}/${maxRetries} failed: ${lastError.message}`));
          await new Promise(resolve => setTimeout(resolve, delay * attempt));
        }
      }
    }

    throw new Error(`All ${maxRetries} attempts failed: ${lastError?.message}`);
  }

  /**
   * Check if the API is connected and ready
   */
  async ensureConnected(): Promise<void> {
    if (!this._api || !this.isConnected) {
      await this.connect();
    }
  }
}

/**
 * Convenience factory to create and connect an API instance
 */
export async function createX3Api(config: X3ChainConfig = {}): Promise<X3ChainApi> {
  const x3 = new X3ChainApi(config);
  await x3.connect();
  return x3;
}

/**
 * Create API instance from environment configuration
 */
export async function createX3ApiFromEnv(): Promise<X3ChainApi> {
  const config = getSdkConfig();
  return createX3Api({
    network: config.network,
    endpoint: config.endpoint,
    autoReconnect: config.autoReconnect,
    reconnectMaxAttempts: config.reconnectMaxAttempts,
    reconnectDelay: config.reconnectDelay,
  });
}
