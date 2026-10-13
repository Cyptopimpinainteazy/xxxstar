/**
 * @module adapters/AtlasSphereAdapter
 * Substrate/Polkadot chain adapter for the Atlas Sphere devnet.
 *
 * Connects via WebSocket to the Substrate node and provides the
 * ChainAdapter interface. Mirrors patterns from @atlas-sphere/ts-sdk.
 */
import type {
  ChainAdapter,
  ChainConfig,
  ChainStatus,
  Block,
  Transaction,
  ChainEvent,
  EventFilter,
  SigningIntent,
  FeeEstimate,
  AccountInfo,
} from "../types/chain";

/**
 * Atlas Sphere Substrate adapter.
 *
 * In production this wraps @polkadot/api. This scaffold provides the
 * complete interface with simulated responses so the app compiles and
 * runs standalone without requiring a live Substrate node.
 */
export class AtlasSphereAdapter implements ChainAdapter {
  private config: ChainConfig;
  private connected = false;
  private ws: WebSocket | null = null;
  private blockHeight = 0;
  private subscriptions = new Map<string, (event: ChainEvent) => void>();
  private pollingTimer: ReturnType<typeof setInterval> | null = null;

  constructor(config?: Partial<ChainConfig>) {
    this.config = {
      chainId: config?.chainId ?? "atlas-sphere-devnet",
      rpcUrl: config?.rpcUrl ?? "ws://127.0.0.1:9944",
      networkPhase: config?.networkPhase ?? "Devnet",
      label: config?.label ?? "Atlas Sphere",
    };
  }

  async connect(config?: ChainConfig): Promise<void> {
    if (config) this.config = config;

    // Probe the RPC endpoint before opening a WebSocket to avoid noisy
    // console errors when no local Substrate node is running.
    const reachable = await this.probeEndpoint();

    if (!reachable) {
      console.info("[AtlasSphereAdapter] No node at %s — running in simulated mode", this.config.rpcUrl);
      this.connected = true;
      this.startSimulatedBlocks();
      return;
    }

    return new Promise<void>((resolve) => {
      try {
        this.ws = new WebSocket(this.config.rpcUrl);

        const timeout = setTimeout(() => {
          this.ws?.close();
          console.warn("[AtlasSphereAdapter] WS connect timeout — falling back to simulated mode");
          this.connected = true;
          this.startSimulatedBlocks();
          resolve();
        }, 5000);

        this.ws.onopen = () => {
          clearTimeout(timeout);
          this.connected = true;
          console.info(`[AtlasSphereAdapter] Connected to ${this.config.rpcUrl}`);
          this.startBlockSubscription();
          resolve();
        };

        this.ws.onerror = () => {
          clearTimeout(timeout);
          console.warn("[AtlasSphereAdapter] WS error — falling back to simulated mode");
          this.connected = true;
          this.startSimulatedBlocks();
          resolve();
        };

        this.ws.onclose = () => {
          this.connected = false;
        };
      } catch {
        this.connected = true;
        this.startSimulatedBlocks();
        resolve();
      }
    });
  }

  /**
   * Light HTTP probe to check if the RPC endpoint is reachable before
   * opening a WebSocket (avoids browser console "Connection refused" noise).
   */
  private async probeEndpoint(): Promise<boolean> {
    try {
      const httpUrl = this.config.rpcUrl
        .replace(/^ws:/, "http:")
        .replace(/^wss:/, "https:");
      const ctrl = new AbortController();
      const timer = setTimeout(() => ctrl.abort(), 2000);
      const res = await fetch(httpUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "system_health", params: [] }),
        signal: ctrl.signal,
      });
      clearTimeout(timer);
      return res.ok || res.status === 400; // 400 = node alive but bad request
    } catch {
      return false;
    }
  }

  async disconnect(): Promise<void> {
    if (this.pollingTimer) {
      clearInterval(this.pollingTimer);
      this.pollingTimer = null;
    }
    this.ws?.close();
    this.ws = null;
    this.connected = false;
    this.subscriptions.clear();
  }

  async getStatus(): Promise<ChainStatus> {
    return {
      connected: this.connected,
      blockHeight: this.blockHeight,
      peerCount: this.connected ? 3 : 0,
      syncState: "synced",
      latencyMs: this.connected ? 12 : 0,
    };
  }

  async getBlock(height: number): Promise<Block> {
    return this.createSimulatedBlock(height);
  }

  async getLatestBlocks(count: number): Promise<Block[]> {
    const blocks: Block[] = [];
    const start = Math.max(0, this.blockHeight - count);
    for (let h = start; h <= this.blockHeight; h++) {
      blocks.push(this.createSimulatedBlock(h));
    }
    return blocks;
  }

  async submitTransaction(intent: SigningIntent): Promise<string> {
    // In production, this would encode + sign + submit via @polkadot/api
    const txHash = `0x${randomHex(32)}`;
    console.info(`[AtlasSphereAdapter] Submitted tx ${txHash} (${intent.method})`);

    // Notify subscribers
    this.notifySubscribers({
      id: `evt-${Date.now()}`,
      chainId: this.config.chainId,
      blockHeight: this.blockHeight,
      type: "transaction",
      data: { hash: txHash, method: intent.method },
      timestamp: Date.now(),
    });

    return txHash;
  }

  async estimateFee(_intent: SigningIntent): Promise<FeeEstimate> {
    // Substrate fees: base + weight-based
    return {
      estimatedFee: "0.0025",
      currency: "ATLAS",
      confidence: 0.95,
    };
  }

  async getAccount(address: string): Promise<AccountInfo> {
    return {
      address,
      balance: "1000.0",
      nonce: 0,
      staked: "0",
    };
  }

  subscribe(_filter: EventFilter, callback: (event: ChainEvent) => void): string {
    const id = `sub-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.subscriptions.set(id, callback);
    return id;
  }

  unsubscribe(subscriptionId: string): void {
    this.subscriptions.delete(subscriptionId);
  }

  // -----------------------------------------------------------------------
  // Private — block production
  // -----------------------------------------------------------------------

  private startBlockSubscription(): void {
    // In production: ws.send(JSON_RPC subscribe_newHead)
    // For scaffold: poll or simulate
    this.startSimulatedBlocks();
  }

  private startSimulatedBlocks(): void {
    if (this.pollingTimer) return;
    this.pollingTimer = setInterval(() => {
      this.blockHeight++;
      const block = this.createSimulatedBlock(this.blockHeight);
      this.notifySubscribers({
        id: `evt-block-${this.blockHeight}`,
        chainId: this.config.chainId,
        blockHeight: this.blockHeight,
        type: "block",
        data: block,
        timestamp: Date.now(),
      });
    }, 6000); // ~6 second block time (Substrate default)
  }

  private createSimulatedBlock(height: number): Block {
    const txCount = Math.floor(Math.random() * 10);
    const transactions: Transaction[] = [];
    for (let i = 0; i < txCount; i++) {
      transactions.push({
        hash: `0x${randomHex(32)}`,
        from: `5${randomHex(23)}`,
        to: `5${randomHex(23)}`,
        method: ["transfer", "bond", "validate", "nominate", "sudo"][Math.floor(Math.random() * 5)],
        args: {},
        status: "finalized",
        blockHeight: height,
        timestamp: Date.now() - (this.blockHeight - height) * 6000,
      });
    }

    return {
      height,
      hash: `0x${randomHex(32)}`,
      parentHash: `0x${randomHex(32)}`,
      timestamp: Date.now() - (this.blockHeight - height) * 6000,
      transactions,
      extrinsicCount: txCount,
    };
  }

  private notifySubscribers(event: ChainEvent): void {
    for (const cb of this.subscriptions.values()) {
      try {
        cb(event);
      } catch (err) {
        console.error("[AtlasSphereAdapter] Subscriber error:", err);
      }
    }
  }
}

// -------------------------------------------------------------------------
// Utility
// -------------------------------------------------------------------------

function randomHex(bytes: number): string {
  const arr = new Uint8Array(bytes);
  crypto.getRandomValues(arr);
  return Array.from(arr)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
