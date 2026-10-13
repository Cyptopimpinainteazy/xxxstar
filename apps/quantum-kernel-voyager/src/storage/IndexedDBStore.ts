/**
 * @module storage/IndexedDBStore
 * Persistent key-value store backed by IndexedDB.
 *
 * Used for: settings, cached chain data, discovery journal,
 * and anything that should survive app restarts but doesn't
 * need the Rust keystore's encryption.
 */

const DB_NAME = "quantum-voyager-store";
const DB_VERSION = 1;

export type StoreName = "settings" | "cache" | "journal";

export class IndexedDBStore {
  private db: IDBDatabase | null = null;

  async open(): Promise<void> {
    if (this.db) return;

    this.db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, DB_VERSION);

      req.onupgradeneeded = () => {
        const db = req.result;
        const stores: StoreName[] = ["settings", "cache", "journal"];
        for (const name of stores) {
          if (!db.objectStoreNames.contains(name)) {
            db.createObjectStore(name);
          }
        }
      };

      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }

  /** Get a value by key from a store. */
  async get<T>(store: StoreName, key: string): Promise<T | undefined> {
    this.ensureOpen();
    return new Promise((resolve, reject) => {
      const tx = this.db!.transaction(store, "readonly");
      const req = tx.objectStore(store).get(key);
      req.onsuccess = () => resolve(req.result as T | undefined);
      req.onerror = () => reject(req.error);
    });
  }

  /** Set a value by key in a store. */
  async set<T>(store: StoreName, key: string, value: T): Promise<void> {
    this.ensureOpen();
    return new Promise((resolve, reject) => {
      const tx = this.db!.transaction(store, "readwrite");
      const req = tx.objectStore(store).put(value, key);
      req.onsuccess = () => resolve();
      req.onerror = () => reject(req.error);
    });
  }

  /** Delete a key from a store. */
  async delete(store: StoreName, key: string): Promise<void> {
    this.ensureOpen();
    return new Promise((resolve, reject) => {
      const tx = this.db!.transaction(store, "readwrite");
      const req = tx.objectStore(store).delete(key);
      req.onsuccess = () => resolve();
      req.onerror = () => reject(req.error);
    });
  }

  /** Get all keys in a store. */
  async keys(store: StoreName): Promise<string[]> {
    this.ensureOpen();
    return new Promise((resolve, reject) => {
      const tx = this.db!.transaction(store, "readonly");
      const req = tx.objectStore(store).getAllKeys();
      req.onsuccess = () => resolve(req.result as string[]);
      req.onerror = () => reject(req.error);
    });
  }

  /** Get all entries in a store. */
  async getAll<T>(store: StoreName): Promise<T[]> {
    this.ensureOpen();
    return new Promise((resolve, reject) => {
      const tx = this.db!.transaction(store, "readonly");
      const req = tx.objectStore(store).getAll();
      req.onsuccess = () => resolve(req.result as T[]);
      req.onerror = () => reject(req.error);
    });
  }

  /** Clear all entries in a store. */
  async clear(store: StoreName): Promise<void> {
    this.ensureOpen();
    return new Promise((resolve, reject) => {
      const tx = this.db!.transaction(store, "readwrite");
      const req = tx.objectStore(store).clear();
      req.onsuccess = () => resolve();
      req.onerror = () => reject(req.error);
    });
  }

  /** Close the database connection. */
  close(): void {
    this.db?.close();
    this.db = null;
  }

  private ensureOpen(): void {
    if (!this.db) {
      throw new Error("[IndexedDBStore] Database not open. Call open() first.");
    }
  }
}

/** Singleton store instance for the application. */
export const store = new IndexedDBStore();
