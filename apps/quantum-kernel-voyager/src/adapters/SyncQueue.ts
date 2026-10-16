/**
 * @module adapters/SyncQueue
 * Offline-first transaction queue with IndexedDB persistence and retry logic.
 *
 * When the chain adapter is disconnected, transactions are queued locally.
 * On reconnection, queued transactions are replayed in order with configurable
 * retry policy (exponential backoff, max 5 attempts).
 */
import type { ChainAdapter, SigningIntent } from "../types/chain";

export type SyncStatus = "pending" | "submitting" | "confirmed" | "failed";

export interface QueuedTransaction {
  id: string;
  intent: SigningIntent;
  status: SyncStatus;
  attempts: number;
  maxAttempts: number;
  createdAt: number;
  lastAttemptAt: number;
  txHash: string | null;
  error: string | null;
}

export type SyncQueueEventType = "queued" | "submitting" | "confirmed" | "failed" | "drained";
export type SyncQueueCallback = (type: SyncQueueEventType, item: QueuedTransaction | null) => void;

const DB_NAME = "quantum-voyager-sync";
const STORE_NAME = "tx-queue";
const MAX_ATTEMPTS = 5;
const BASE_DELAY_MS = 1000;

export class SyncQueue {
  private adapter: ChainAdapter | null = null;
  private queue: QueuedTransaction[] = [];
  private processing = false;
  private listeners: SyncQueueCallback[] = [];
  private db: IDBDatabase | null = null;

  /** Attach a chain adapter for submission. */
  setAdapter(adapter: ChainAdapter): void {
    this.adapter = adapter;
  }

  /** Subscribe to queue events. */
  on(callback: SyncQueueCallback): void {
    this.listeners.push(callback);
  }

  /** Initialize IndexedDB store and load persisted queue. */
  async init(): Promise<void> {
    this.db = await this.openDb();
    this.queue = await this.loadQueue();
    console.info(`[SyncQueue] Loaded ${this.queue.length} persisted items`);
  }

  /** Enqueue a transaction for submission. */
  async enqueue(intent: SigningIntent): Promise<string> {
    const item: QueuedTransaction = {
      id: `sq-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      intent,
      status: "pending",
      attempts: 0,
      maxAttempts: MAX_ATTEMPTS,
      createdAt: Date.now(),
      lastAttemptAt: 0,
      txHash: null,
      error: null,
    };

    this.queue.push(item);
    await this.persistItem(item);
    this.emit("queued", item);

    // Auto-process if idle
    if (!this.processing) {
      this.processQueue();
    }

    return item.id;
  }

  /** Get all queued items. */
  getAll(): QueuedTransaction[] {
    return [...this.queue];
  }

  /** Get pending count. */
  pendingCount(): number {
    return this.queue.filter((i) => i.status === "pending" || i.status === "submitting").length;
  }

  /** Force retry all failed items. */
  async retryFailed(): Promise<void> {
    for (const item of this.queue) {
      if (item.status === "failed") {
        item.status = "pending";
        item.attempts = 0;
        item.error = null;
        await this.persistItem(item);
      }
    }
    if (!this.processing) {
      this.processQueue();
    }
  }

  /** Remove confirmed and permanently failed items. */
  async prune(): Promise<void> {
    const toRemove = this.queue.filter(
      (i) => i.status === "confirmed" || (i.status === "failed" && i.attempts >= i.maxAttempts),
    );
    for (const item of toRemove) {
      await this.deleteItem(item.id);
    }
    this.queue = this.queue.filter(
      (i) => i.status !== "confirmed" && !(i.status === "failed" && i.attempts >= i.maxAttempts),
    );
  }

  /** Teardown. */
  dispose(): void {
    this.processing = false;
    this.listeners.length = 0;
    this.db?.close();
    this.db = null;
  }

  // -----------------------------------------------------------------------
  // Private — processing
  // -----------------------------------------------------------------------

  private async processQueue(): Promise<void> {
    if (this.processing || !this.adapter) return;
    this.processing = true;

    while (true) {
      const item = this.queue.find((i) => i.status === "pending");
      if (!item) break;

      item.status = "submitting";
      item.attempts++;
      item.lastAttemptAt = Date.now();
      this.emit("submitting", item);

      try {
        const hash = await this.adapter.submitTransaction(item.intent);
        item.txHash = hash;
        item.status = "confirmed";
        item.error = null;
        await this.persistItem(item);
        this.emit("confirmed", item);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        item.error = msg;

        if (item.attempts >= item.maxAttempts) {
          item.status = "failed";
          this.emit("failed", item);
        } else {
          item.status = "pending";
          // Exponential backoff before next attempt
          const delay = BASE_DELAY_MS * Math.pow(2, item.attempts - 1);
          await sleep(delay);
        }
        await this.persistItem(item);
      }
    }

    this.processing = false;
    this.emit("drained", null);
  }

  // -----------------------------------------------------------------------
  // Private — IndexedDB
  // -----------------------------------------------------------------------

  private openDb(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, 1);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          db.createObjectStore(STORE_NAME, { keyPath: "id" });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }

  private loadQueue(): Promise<QueuedTransaction[]> {
    return new Promise((resolve, reject) => {
      if (!this.db) { resolve([]); return; }
      const tx = this.db.transaction(STORE_NAME, "readonly");
      const store = tx.objectStore(STORE_NAME);
      const req = store.getAll();
      req.onsuccess = () => resolve(req.result as QueuedTransaction[]);
      req.onerror = () => reject(req.error);
    });
  }

  private persistItem(item: QueuedTransaction): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) { resolve(); return; }
      const tx = this.db.transaction(STORE_NAME, "readwrite");
      const store = tx.objectStore(STORE_NAME);
      const req = store.put(item);
      req.onsuccess = () => resolve();
      req.onerror = () => reject(req.error);
    });
  }

  private deleteItem(id: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.db) { resolve(); return; }
      const tx = this.db.transaction(STORE_NAME, "readwrite");
      const store = tx.objectStore(STORE_NAME);
      const req = store.delete(id);
      req.onsuccess = () => resolve();
      req.onerror = () => reject(req.error);
    });
  }

  // -----------------------------------------------------------------------
  // Private — events
  // -----------------------------------------------------------------------

  private emit(type: SyncQueueEventType, item: QueuedTransaction | null): void {
    for (const cb of this.listeners) {
      try {
        cb(type, item);
      } catch (err) {
        console.error("[SyncQueue] Listener error:", err);
      }
    }
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
