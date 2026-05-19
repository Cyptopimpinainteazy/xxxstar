/**
 * x3ChainServiceIntegration - Tauri Desktop Core x3ChainService Integration
 *
 * This module provides the service layer for X3 chain operations with:
 * - Chain operation methods (queries, transactions, subscriptions)
 * - Error handling with detailed error types
 * - Caching mechanism for performance
 * - Configuration support
 */

import { invoke } from '@tauri-apps/api/core';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface ChainOperationResult {
  success: boolean;
  operation: string;
  data: any;
  error: string | null;
  executionTimeMs: number;
}

export interface ConnectionStatus {
  connected: boolean;
  lastConnected: number | null;
  lastDisconnected: number | null;
  blockNumber: number | null;
  chainName: string | null;
}

export interface X3ChainServiceConfig {
  rpcUrl: string;
  wsUrl: string;
  timeoutMs: number;
  cacheTtlMs: number;
  maxRetries: number;
  retryDelayMs: number;
}

// ── x3ChainService Class ──────────────────────────────────────────────────────

export class X3ChainService {
  private config: X3ChainServiceConfig;
  private cache: Map<string, { value: any; timestamp: number; ttl: number }>;
  private connectionStatus: ConnectionStatus;
  private operationStats: Map<string, { count: number; totalDuration: number }>;

  constructor(config: Partial<X3ChainServiceConfig> = {}) {
    this.config = {
      rpcUrl: config.rpcUrl || 'http://127.0.0.1:9933',
      wsUrl: config.wsUrl || 'ws://127.0.0.1:9944',
      timeoutMs: config.timeoutMs || 30000,
      cacheTtlMs: config.cacheTtlMs || 60000,
      maxRetries: config.maxRetries || 3,
      retryDelayMs: config.retryDelayMs || 1000,
    };

    this.cache = new Map();
    this.connectionStatus = {
      connected: false,
      lastConnected: null,
      lastDisconnected: null,
      blockNumber: null,
      chainName: null,
    };
    this.operationStats = new Map();
  }

  // ── Cache Management ──────────────────────────────────────────────────────

  private getCacheKey(prefix: string, params: Record<string, any>): string {
    const sortedParams = Object.entries(params)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => `${k}=${v}`)
      .join('&');
    return `${prefix}:${sortedParams}`;
  }

  private get<T>(key: string): T | null {
    const entry = this.cache.get(key);
    if (!entry) return null;

    const elapsed = Date.now() - entry.timestamp;
    if (elapsed > entry.ttl) {
      this.cache.delete(key);
      return null;
    }

    return entry.value as T;
  }

  private set<T>(key: string, value: T, ttl?: number): void {
    const effectiveTtl = ttl ?? this.config.cacheTtlMs;
    this.cache.set(key, {
      value,
      timestamp: Date.now(),
      ttl: effectiveTtl,
    });
  }

  private _clearCache(): void {
    this.cache.clear();
  }

  // ── Chain Operations ────────────────────────────────────────────────────────

  /**
   * Query block data from X3 chain
   */
  async queryBlock(
    blockNumber?: number,
    blockHash?: string,
  ): Promise<ChainOperationResult> {
    const cacheKey = this.getCacheKey('block', { blockNumber, blockHash });
    const cached = this.get<any>(cacheKey);

    if (cached) {
      return {
        success: true,
        operation: 'QueryBlock',
        data: cached,
        error: null,
        executionTimeMs: 0,
      };
    }

    const startTime = Date.now();

    try {
      const result = await invoke<string>('query_block', {
        blockNumber,
        blockHash,
      });

      const data = JSON.parse(result);
      this.set(cacheKey, data);

      this.updateStats('QueryBlock', Date.now() - startTime);

      return {
        success: true,
        operation: 'QueryBlock',
        data,
        error: null,
        executionTimeMs: Date.now() - startTime,
      };
    } catch (error) {
      this.updateStats('QueryBlock', Date.now() - startTime);

      return {
        success: false,
        operation: 'QueryBlock',
        data: null,
        error: error instanceof Error ? error.message : String(error),
        executionTimeMs: Date.now() - startTime,
      };
    }
  }

  /**
   * Query account data from X3 chain
   */
  async queryAccount(address: string, atBlock?: number): Promise<ChainOperationResult> {
    const cacheKey = this.getCacheKey('account', { address, atBlock });
    const cached = this.get<any>(cacheKey);

    if (cached) {
      return {
        success: true,
        operation: 'QueryAccount',
        data: cached,
        error: null,
        executionTimeMs: 0,
      };
    }

    const startTime = Date.now();

    try {
      const result = await invoke<string>('query_account', {
        address,
        atBlock,
      });

      const data = JSON.parse(result);
      this.set(cacheKey, data);

      this.updateStats('QueryAccount', Date.now() - startTime);

      return {
        success: true,
        operation: 'QueryAccount',
        data,
        error: null,
        executionTimeMs: Date.now() - startTime,
      };
    } catch (error) {
      this.updateStats('QueryAccount', Date.now() - startTime);

      return {
        success: false,
        operation: 'QueryAccount',
        data: null,
        error: error instanceof Error ? error.message : String(error),
        executionTimeMs: Date.now() - startTime,
      };
    }
  }

  /**
   * Query account balance from X3 chain
   */
  async queryBalance(
    address: string,
    assetId?: string,
  ): Promise<ChainOperationResult> {
    const cacheKey = this.getCacheKey('balance', { address, assetId });
    const cached = this.get<any>(cacheKey);

    if (cached) {
      return {
        success: true,
        operation: 'QueryBalance',
        data: cached,
        error: null,
        executionTimeMs: 0,
      };
    }

    const startTime = Date.now();

    try {
      const result = await invoke<string>('query_balance', {
        address,
        assetId,
      });

      const data = JSON.parse(result);
      this.set(cacheKey, data);

      this.updateStats('QueryBalance', Date.now() - startTime);

      return {
        success: true,
        operation: 'QueryBalance',
        data,
        error: null,
        executionTimeMs: Date.now() - startTime,
      };
    } catch (error) {
      this.updateStats('QueryBalance', Date.now() - startTime);

      return {
        success: false,
        operation: 'QueryBalance',
        data: null,
        error: error instanceof Error ? error.message : String(error),
        executionTimeMs: Date.now() - startTime,
      };
    }
  }

  /**
   * Submit an extrinsic to X3 chain
   */
  async submitExtrinsic(
    call: string,
    signer: string,
    nonce?: number,
    tip?: number,
  ): Promise<ChainOperationResult> {
    const startTime = Date.now();

    try {
      const result = await invoke<string>('submit_extrinsic', {
        call,
        signer,
        nonce,
        tip,
      });

      const data = JSON.parse(result);

      this.updateStats('SubmitExtrinsic', Date.now() - startTime);

      return {
        success: true,
        operation: 'SubmitExtrinsic',
        data,
        error: null,
        executionTimeMs: Date.now() - startTime,
      };
    } catch (error) {
      this.updateStats('SubmitExtrinsic', Date.now() - startTime);

      return {
        success: false,
        operation: 'SubmitExtrinsic',
        data: null,
        error: error instanceof Error ? error.message : String(error),
        executionTimeMs: Date.now() - startTime,
      };
    }
  }

  /**
   * Get the connection status
   */
  async getConnectionStatus(): Promise<ConnectionStatus> {
    try {
      const result = await invoke<string>('get_connection_status');
      const data = JSON.parse(result);

      this.connectionStatus = {
        ...this.connectionStatus,
        connected: data.connected,
        blockNumber: data.blockNumber,
      };

      return this.connectionStatus;
    } catch (error) {
      console.error('[x3ChainService] Failed to get connection status:', error);
      return this.connectionStatus;
    }
  }

  /**
   * Clear the chain operation cache
   */
  async clearCache(): Promise<void> {
    this._clearCache();
    await invoke('clear_chain_cache');
  }

  // ── Statistics ──────────────────────────────────────────────────────────────

  private updateStats(operation: string, duration: number): void {
    const current = this.operationStats.get(operation) ?? { count: 0, totalDuration: 0 };
    this.operationStats.set(operation, {
      count: current.count + 1,
      totalDuration: current.totalDuration + duration,
    });
  }

  /**
   * Get operation statistics
   */
  getOperationStats(): Map<string, { count: number; totalDuration: number }> {
    return this.operationStats;
  }

  /**
   * Get the service configuration
   */
  getConfig(): X3ChainServiceConfig {
    return this.config;
  }

  /**
   * Get cache size
   */
  getCacheSize(): number {
    return this.cache.size;
  }
}

// ── Hook for React integration ────────────────────────────────────────────────

import { useState, useEffect, useCallback } from 'react';

export function useX3ChainService(config?: Partial<X3ChainServiceConfig>) {
  const [service] = useState(() => new X3ChainService(config));
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({
    connected: false,
    lastConnected: null,
    lastDisconnected: null,
    blockNumber: null,
    chainName: null,
  });

  // Update connection status periodically
  useEffect(() => {
    const updateStatus = async () => {
      const status = await service.getConnectionStatus();
      setConnectionStatus(status);
    };

    updateStatus();
    const interval = setInterval(updateStatus, 5000); // Update every 5 seconds

    return () => clearInterval(interval);
  }, [service]);

  return {
    service,
    connectionStatus,
    queryBlock: service.queryBlock.bind(service),
    queryAccount: service.queryAccount.bind(service),
    queryBalance: service.queryBalance.bind(service),
    submitExtrinsic: service.submitExtrinsic.bind(service),
    getConnectionStatus: service.getConnectionStatus.bind(service),
    clearCache: service.clearCache.bind(service),
    getOperationStats: service.getOperationStats.bind(service),
    getCacheSize: service.getCacheSize.bind(service),
  };
}
