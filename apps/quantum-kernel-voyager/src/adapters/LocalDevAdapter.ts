/**
 * @module adapters/LocalDevAdapter
 * Mock chain adapter for local development and testing.
 *
 * Produces simulated blocks every 3 seconds with 0–50 transactions.
 * No network calls — everything runs in-memory.
 * Ideal for UI development without a running node.
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

const BLOCK_INTERVAL = 3000; // 3 second blocks
const MAX_TX_PER_BLOCK = 50;

interface MockAccount {
  address: string;
  balance: number;
  nonce: number;
}

export class LocalDevAdapter implements ChainAdapter {
  private config: ChainConfig;
  private connected = false;
  private blockHeight = 0;
  private subscriptions = new Map<string, (event: ChainEvent) => void>();
  private blockTimer: ReturnType<typeof setInterval> | null = null;
  private accounts = new Map<string, MockAccount>();
  private pendingTxQueue: Transaction[] = [];
  private blocks: Block[] = [];

  constructor(config?: Partial<ChainConfig>) {
    this.config = {
      chainId: config?.chainId ?? "local-dev",
      rpcUrl: config?.rpcUrl ?? "mock://localhost",
      networkPhase: config?.networkPhase ?? "Devnet",
      label: config?.label ?? "Local Dev",
    };

    // Seed some accounts
    for (let i = 0; i < 5; i++) {
      const addr = `5Dev${i.toString().padStart(44, "0")}`;
      this.accounts.set(addr, { address: addr, balance: 100_000, nonce: 0 });
    }
  }

  async connect(_config?: ChainConfig): Promise<void> {
    this.connected = true;
    this.blockHeight = 0;
    console.info("[LocalDevAdapter] Connected (mock mode)");
    this.startBlockProduction();
  }

  async disconnect(): Promise<void> {
    if (this.blockTimer) {
      clearInterval(this.blockTimer);
      this.blockTimer = null;
    }
    this.connected = false;
    this.subscriptions.clear();
    console.info("[LocalDevAdapter] Disconnected");
  }

  async getStatus(): Promise<ChainStatus> {
    return {
      connected: this.connected,
      blockHeight: this.blockHeight,
      peerCount: 0,
      syncState: "synced",
      latencyMs: 1,
    };
  }

  async getBlock(height: number): Promise<Block> {
    const cached = this.blocks.find((b) => b.height === height);
    if (cached) return cached;
    return this.generateBlock(height);
  }

  async getLatestBlocks(count: number): Promise<Block[]> {
    const start = Math.max(0, this.blocks.length - count);
    return this.blocks.slice(start);
  }

  async submitTransaction(intent: SigningIntent): Promise<string> {
    const tx: Transaction = {
      hash: `0x${mockHex(32)}`,
      from: intent.from ?? "",
      to: intent.to ?? "",
      method: intent.method,
      args: intent.args,
      status: "pending",
      blockHeight: 0,
      timestamp: Date.now(),
    };
    this.pendingTxQueue.push(tx);

    // Update account nonce
    const acct = this.accounts.get(intent.from ?? "");
    if (acct) acct.nonce++;

    console.info(`[LocalDevAdapter] Tx queued: ${tx.hash} (${intent.method})`);
    return tx.hash;
  }

  async estimateFee(_intent: SigningIntent): Promise<FeeEstimate> {
    return {
      estimatedFee: "0",
      currency: "DEV",
      confidence: 1.0,
    };
  }

  async getAccount(address: string): Promise<AccountInfo> {
    const acct = this.accounts.get(address);
    if (acct) {
      return {
        address: acct.address,
        balance: acct.balance.toString(),
        nonce: acct.nonce,
        staked: "0",
      };
    }
    // Auto-create account with airdrop
    const newAcct: MockAccount = { address, balance: 10_000, nonce: 0 };
    this.accounts.set(address, newAcct);
    return {
      address,
      balance: newAcct.balance.toString(),
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
  // Private
  // -----------------------------------------------------------------------

  private startBlockProduction(): void {
    if (this.blockTimer) return;
    this.blockTimer = setInterval(() => {
      this.produceBlock();
    }, BLOCK_INTERVAL);
  }

  private produceBlock(): void {
    this.blockHeight++;
    const block = this.generateBlock(this.blockHeight);

    // Include pending transactions
    const included = this.pendingTxQueue.splice(0, MAX_TX_PER_BLOCK);
    for (const tx of included) {
      tx.status = "finalized";
      tx.blockHeight = this.blockHeight;
      block.transactions.push(tx);
    }

    // Add random simulated transactions
    const simCount = Math.floor(Math.random() * (MAX_TX_PER_BLOCK - included.length));
    for (let i = 0; i < simCount; i++) {
      block.transactions.push(this.randomTransaction(this.blockHeight));
    }
    (block as { extrinsicCount: number }).extrinsicCount = block.transactions.length;

    this.blocks.push(block);
    // Keep only last 100 blocks in memory
    if (this.blocks.length > 100) {
      this.blocks.shift();
    }

    // Notify subscribers
    this.notifySubscribers({
      id: `evt-block-${this.blockHeight}`,
      chainId: this.config.chainId,
      blockHeight: this.blockHeight,
      type: "block",
      data: block,
      timestamp: Date.now(),
    });

    // Emit per-transaction events
    for (const tx of block.transactions) {
      this.notifySubscribers({
        id: `evt-tx-${tx.hash}`,
        chainId: this.config.chainId,
        blockHeight: this.blockHeight,
        type: "transaction",
        data: tx as unknown as Record<string, unknown>,
        timestamp: Date.now(),
      });
    }
  }

  private generateBlock(height: number): Block {
    return {
      height,
      hash: `0x${mockHex(32)}`,
      parentHash: height > 0 ? `0x${mockHex(32)}` : "0x" + "0".repeat(64),
      timestamp: Date.now(),
      transactions: [],
      extrinsicCount: 0,
    };
  }

  private randomTransaction(blockHeight: number): Transaction {
    const accounts = Array.from(this.accounts.keys());
    const from = accounts[Math.floor(Math.random() * accounts.length)];
    const to = accounts[Math.floor(Math.random() * accounts.length)];
    const methods = ["transfer", "bond", "register_operator", "submit_deal", "heartbeat"];

    return {
      hash: `0x${mockHex(32)}`,
      from,
      to,
      method: methods[Math.floor(Math.random() * methods.length)],
      args: { amount: Math.floor(Math.random() * 1000) },
      status: "finalized",
      blockHeight,
      timestamp: Date.now(),
    };
  }

  private notifySubscribers(event: ChainEvent): void {
    for (const cb of this.subscriptions.values()) {
      try {
        cb(event);
      } catch (err) {
        console.error("[LocalDevAdapter] Subscriber error:", err);
      }
    }
  }
}

function mockHex(bytes: number): string {
  let result = "";
  for (let i = 0; i < bytes; i++) {
    result += Math.floor(Math.random() * 256).toString(16).padStart(2, "0");
  }
  return result;
}
