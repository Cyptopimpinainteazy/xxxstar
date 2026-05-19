/**
 * IChainAdapter — abstract adapter interface for blockchain connectors.
 *
 * Each chain family (EVM, Bitcoin, Solana, etc.) implements this interface.
 */

import type {
  Block,
  Transaction,
  ValidatorInfo,
  ConnectorMetrics,
  EventEnvelope,
  SubscriptionFilter,
  ChainDescriptor,
} from "../types";

export interface IChainAdapter {
  /** Chain descriptor */
  readonly chain: ChainDescriptor;

  /** Connect to the chain endpoint */
  connect(endpoint: string): Promise<void>;

  /** Disconnect cleanly */
  disconnect(): Promise<void>;

  /** Check if connected */
  isConnected(): boolean;

  /** Get the latest block */
  getLatestBlock(): Promise<Block>;

  /** Get a specific block by number or hash */
  getBlock(numberOrHash: string | number): Promise<Block>;

  /** Get a transaction by hash */
  getTransaction(hash: string): Promise<Transaction>;

  /** Get current validator set (if applicable) */
  getValidators?(): Promise<ValidatorInfo[]>;

  /** Get live connector metrics */
  getMetrics(): Promise<ConnectorMetrics>;

  /** Submit a signed transaction */
  submitRawTx?(signedTx: string): Promise<{ txHash: string }>;

  /** Subscribe to chain events via callback */
  subscribe?(
    events: string[],
    filter: SubscriptionFilter | undefined,
    handler: (event: EventEnvelope) => void,
  ): Promise<{ unsubscribe: () => void }>;
}

/**
 * Base adapter with shared logic for latency tracking and error counting.
 */
export abstract class BaseChainAdapter implements IChainAdapter {
  abstract readonly chain: ChainDescriptor;

  protected endpoint = "";
  protected connected = false;
  protected requestCount = 0;
  protected errorCount = 0;
  protected startTime = 0;
  protected latencySamples: number[] = [];

  async connect(endpoint: string): Promise<void> {
    this.endpoint = endpoint;
    this.startTime = Date.now();
    this.connected = true;
  }

  async disconnect(): Promise<void> {
    this.connected = false;
  }

  isConnected(): boolean {
    return this.connected;
  }

  abstract getLatestBlock(): Promise<Block>;
  abstract getBlock(numberOrHash: string | number): Promise<Block>;
  abstract getTransaction(hash: string): Promise<Transaction>;

  async getMetrics(): Promise<ConnectorMetrics> {
    const sorted = [...this.latencySamples].sort((a, b) => a - b);
    const p50 = sorted[Math.floor(sorted.length * 0.5)] || 0;

    return {
      blockHeight: 0,
      tps: 0,
      peerCount: 0,
      latencyMs: p50,
      totalRequests: this.requestCount,
      totalErrors: this.errorCount,
      uptimeSeconds: Math.floor((Date.now() - this.startTime) / 1000),
      finalityLag: 0,
    };
  }

  /** Track a request's latency */
  protected trackRequest(startMs: number): void {
    this.requestCount++;
    this.latencySamples.push(Date.now() - startMs);
    if (this.latencySamples.length > 1000) this.latencySamples.shift();
  }

  protected trackError(): void {
    this.errorCount++;
  }

  /**
   * Make an RPC call with latency tracking.
   */
  protected async rpcCall<T>(method: string, params: unknown[] = []): Promise<T> {
    const start = Date.now();
    try {
      const res = await fetch(this.endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: this.requestCount + 1, method, params }),
      });
      const json = await res.json();
      this.trackRequest(start);
      if (json.error) {
        this.trackError();
        throw new Error(`RPC error ${json.error.code}: ${json.error.message}`);
      }
      return json.result as T;
    } catch (err) {
      this.trackError();
      this.trackRequest(start);
      throw err;
    }
  }

  /**
   * Make an HTTP GET call with latency tracking.
   */
  protected async httpGet<T>(path: string): Promise<T> {
    const start = Date.now();
    try {
      const url = this.endpoint.endsWith("/") ? this.endpoint + path : `${this.endpoint}/${path}`;
      const res = await fetch(url);
      const json = await res.json();
      this.trackRequest(start);
      return json as T;
    } catch (err) {
      this.trackError();
      this.trackRequest(start);
      throw err;
    }
  }
}
