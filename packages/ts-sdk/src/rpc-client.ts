/**
 * Shared JSON-RPC 2.0 client utilities for X3 Chain frontends.
 *
 * Provides lightweight HTTP+WS RPC helpers that all Tauri frontends,
 * standalone apps, and static pages can use to call node RPC methods
 * without bolting into the full AtlasSphereClient SDK.
 *
 * @module @x3-chain/ts-sdk/rpc-client
 */

// ── Types ──────────────────────────────────────────────────────────────────

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: number | string;
  method: string;
  params: unknown[];
}

export interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0';
  id: number | string;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export interface JsonRpcClientOptions {
  /** Base URL of the JSON-RPC HTTP endpoint (default: http://127.0.0.1:9933). */
  url?: string;
  /** Request timeout in milliseconds (default: 8000). */
  timeoutMs?: number;
}

export interface WsClientOptions {
  /** WebSocket URL (default: ws://127.0.0.1:9944). */
  url?: string;
  /** Auto-reconnect on close (default: true). */
  autoReconnect?: boolean;
  /** Max reconnect attempts (0 = infinite). */
  maxReconnectAttempts?: number;
  /** Reconnect delay in ms (default: 2000). */
  reconnectDelayMs?: number;
}

// ── Default endpoints (overridable via env) ─────────────────────────────────

function getDefaultHttpUrl(): string {
  if (typeof globalThis !== 'undefined' && (globalThis as any).import?.meta?.env?.VITE_X3_RPC_HTTP) {
    return (globalThis as any).import.meta.env.VITE_X3_RPC_HTTP;
  }
  if (typeof process !== 'undefined' && process.env?.VITE_X3_RPC_HTTP) {
    return process.env.VITE_X3_RPC_HTTP;
  }
  return 'http://127.0.0.1:9933';
}

function getDefaultWsUrl(): string {
  if (typeof globalThis !== 'undefined' && (globalThis as any).import?.meta?.env?.VITE_RPC_WS) {
    return (globalThis as any).import.meta.env.VITE_RPC_WS;
  }
  if (typeof process !== 'undefined' && process.env?.VITE_RPC_WS) {
    return process.env.VITE_RPC_WS;
  }
  return 'ws://127.0.0.1:9944';
}

// ── HTTP JSON-RPC Client ────────────────────────────────────────────────────

let idCounter = 0;

/**
 * Create a typed JSON-RPC 2.0 HTTP client.
 *
 * @example
 * ```typescript
 * const rpc = createJsonRpcClient({ url: 'http://127.0.0.1:9933' });
 * const validators = await rpc.call<ValidatorInfo[]>('validator_getValidators', []);
 * ```
 */
export function createJsonRpcClient(options: JsonRpcClientOptions = {}) {
  const url = options.url ?? getDefaultHttpUrl();
  const timeoutMs = options.timeoutMs ?? 8000;

  async function call<T = unknown>(method: string, params: unknown[] = []): Promise<T> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: ++idCounter,
          method,
          params,
        } satisfies JsonRpcRequest),
        signal: controller.signal,
      });

      const json = (await response.json()) as JsonRpcResponse<T>;

      if (json.error) {
        throw new RpcClientError(
          `RPC error ${json.error.code}: ${json.error.message}`,
          json.error.code,
          json.error.data,
        );
      }

      return json.result as T;
    } catch (err) {
      if (err instanceof RpcClientError) throw err;
      if (err instanceof DOMException && err.name === 'AbortError') {
        throw new RpcClientError(`RPC call to ${method} timed out after ${timeoutMs}ms`, -32000);
      }
      throw new RpcClientError(
        `RPC call to ${method} failed: ${err instanceof Error ? err.message : String(err)}`,
        -32000,
      );
    } finally {
      clearTimeout(timeout);
    }
  }

  return { call, url };
}

// ── WebSocket Client ────────────────────────────────────────────────────────

type MessageHandler<T = unknown> = (data: T) => void;

/**
 * Create a WebSocket client for subscriptions and real-time data.
 *
 * @example
 * ```typescript
 * const ws = createWsClient({ url: 'ws://127.0.0.1:9944' });
 * ws.on('network_subscribeMetrics', (metrics) => {
 *   console.log('Live metrics:', metrics);
 * });`
 * await ws.connect();
 * ```
 */
export function createWsClient(options: WsClientOptions = {}) {
  const url = options.url ?? getDefaultWsUrl();
  const autoReconnect = options.autoReconnect ?? true;
  const maxReconnectAttempts = options.maxReconnectAttempts ?? 10;
  const reconnectDelayMs = options.reconnectDelayMs ?? 2000;

  let ws: WebSocket | null = null;
  let reconnectAttempts = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let subHandlers = new Map<string, Set<MessageHandler>>();

  function connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      ws = new WebSocket(url);

      ws.onopen = () => {
        reconnectAttempts = 0;
        resolve();
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data as string);
          // Route to subscription handler if params.subscription exists
          const subId = msg?.params?.subscription as string | undefined;
          if (subId && subHandlers.has(subId)) {
            subHandlers.get(subId)?.forEach((handler) => handler(msg.params.result));
          }
          // Also route by method name for response callbacks
          if (msg?.method && subHandlers.has(msg.method)) {
            subHandlers.get(msg.method)?.forEach((handler) => handler(msg.params?.result));
          }
        } catch {
          // Ignore parse errors on unknown messages
        }
      };

      ws.onerror = (err) => {
        reject(new RpcClientError(`WebSocket error: ${String(err)}`));
      };

      ws.onclose = () => {
        if (autoReconnect && reconnectAttempts < maxReconnectAttempts) {
          reconnectAttempts++;
          reconnectTimer = setTimeout(() => {
            connect().catch(() => { /* retry loop */ });
          }, reconnectDelayMs * Math.min(reconnectAttempts, 5));
        }
      };
    });
  }

  function on<T = unknown>(method: string, handler: MessageHandler<T>): () => void {
    const handlers = subHandlers.get(method) ?? new Set();
    handlers.add(handler as MessageHandler);
    subHandlers.set(method, handlers);
    // Return unsubscribe function
    return () => {
      handlers.delete(handler as MessageHandler);
      if (handlers.size === 0) subHandlers.delete(method);
    };
  }

  function send(method: string, params: unknown[] = []): void {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      throw new RpcClientError('WebSocket is not connected');
    }
    ws.send(JSON.stringify({
      jsonrpc: '2.0',
      id: ++idCounter,
      method,
      params,
    }));
  }

  function disconnect(): void {
    if (reconnectTimer) clearTimeout(reconnectTimer);
    ws?.close();
    ws = null;
    subHandlers.clear();
  }

  return { connect, disconnect, on, send, getUrl: () => url };
}

// ── Error type ──────────────────────────────────────────────────────────────

export class RpcClientError extends Error {
  public readonly code: number;
  public readonly data: unknown;

  constructor(message: string, code: number = -32000, data?: unknown) {
    super(message);
    this.name = 'RpcClientError';
    this.code = code;
    this.data = data;
  }
}

// ── Typed method wrappers for every real RPC method ─────────────────────────

/**
 * Create a pre-typed RPC client with method wrappers for all known X3 RPC methods.
 */
export function createX3RpcClient(httpUrl?: string) {
  const rpc = createJsonRpcClient({ url: httpUrl });

  return {
    rpc,

    // ── Wallet ──
    wallet: {
      createWallet: (req: unknown) => rpc.call('wallet_createWallet', [req]),
      importWallet: (req: unknown) => rpc.call('wallet_importWallet', [req]),
      getBalance: (req: unknown) => rpc.call('wallet_getBalance', [req]),
      signTransaction: (req: unknown) => rpc.call('wallet_signTransaction', [req]),
      submitTransaction: (req: unknown) => rpc.call('wallet_submitTransaction', [req]),
      getTransactions: (req: unknown) => rpc.call('wallet_getTransactions', [req]),
      getWalletStatus: (req: unknown) => rpc.call('wallet_getWalletStatus', [req]),
      listWallets: (req: unknown) => rpc.call('wallet_listWallets', [req]),
      setNetwork: (req: unknown) => rpc.call('wallet_setNetwork', [req]),
      getNetworks: () => rpc.call('wallet_getNetworks', []),
    },

    // ── DEX ──
    dex: {
      estimateSwap: (req: unknown) => rpc.call('walletDex_estimateSwap', [req]),
      executeSwap: (req: unknown) => rpc.call('walletDex_executeSwap', [req]),
    },

    // ── Validator ──
    validator: {
      getValidators: () => rpc.call('validator_getValidators', []),
      getLeaderboard: () => rpc.call('validator_getLeaderboard', []),
      getMetrics: () => rpc.call('validator_getMetrics', []),
    },

    // ── Signing ──
    sign: {
      ed25519: (msg: string, secret: string) => rpc.call('x3_sign_ed25519', [msg, secret]),
      secp256k1: (msg: string, secret: string) => rpc.call('x3_sign_secp256k1', [msg, secret]),
      sr25519: (msg: string, secret: string) => rpc.call('x3_sign_sr25519', [msg, secret]),
      verify: (msg: string, sig: string, pk: string, keyType: string) =>
        rpc.call('x3_verify_signature', [msg, sig, pk, keyType]),
    },

    // ── Cross-VM (EXPERIMENTAL — backend not yet wired) ──
    crossVm: {
      /** @experimental Backend returns JSON-RPC error until wired to bridge pallet runtime data. */
      getRecentTransfers: (limit?: number) => rpc.call('crossVm_getRecentTransfers', [{ limit: limit ?? 10 }]),
    },

    // ── Swarm (EXPERIMENTAL — backend not yet wired) ──
    swarm: {
      /** @experimental Backend returns JSON-RPC error until wired to swarm API. */
      getMetrics: () => rpc.call('swarm_getMetrics', []),
      /** @experimental Backend returns JSON-RPC error until wired to swarm API. */
      getRecentTasks: (limit?: number) => rpc.call('swarm_getRecentTasks', [{ limit: limit ?? 10 }]),
    },

    // ── Token (EXPERIMENTAL — backend not yet wired) ──
    token: {
      /** @experimental Backend returns JSON-RPC error until wired to balances pallet runtime data. */
      getSupply: () => rpc.call('token_getSupply', []),
    },

    // ── Weight meter ──
    weight: {
      meter: (config: unknown) => rpc.call('x3_weight_meter', [config]),
    },
  };
}