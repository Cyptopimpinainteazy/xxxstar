"use strict";
/**
 * Shared JSON-RPC 2.0 client utilities for X3 Chain frontends.
 *
 * Provides lightweight HTTP+WS RPC helpers that all Tauri frontends,
 * standalone apps, and static pages can use to call node RPC methods
 * without bolting into the full AtlasSphereClient SDK.
 *
 * @module @x3-chain/ts-sdk/rpc-client
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.RpcClientError = void 0;
exports.createJsonRpcClient = createJsonRpcClient;
exports.createWsClient = createWsClient;
exports.createX3RpcClient = createX3RpcClient;
// ── Default endpoints (overridable via env) ─────────────────────────────────
function getDefaultHttpUrl() {
    if (typeof globalThis !== 'undefined' && globalThis.import?.meta?.env?.VITE_X3_RPC_HTTP) {
        return globalThis.import.meta.env.VITE_X3_RPC_HTTP;
    }
    if (typeof process !== 'undefined' && process.env?.VITE_X3_RPC_HTTP) {
        return process.env.VITE_X3_RPC_HTTP;
    }
    return 'http://rpc.testnet.x3-chain.io:9944';
}
function getDefaultWsUrl() {
    if (typeof globalThis !== 'undefined' && globalThis.import?.meta?.env?.VITE_RPC_WS) {
        return globalThis.import.meta.env.VITE_RPC_WS;
    }
    if (typeof process !== 'undefined' && process.env?.VITE_RPC_WS) {
        return process.env.VITE_RPC_WS;
    }
    return 'ws://rpc.testnet.x3-chain.io:9944';
}
// ── HTTP JSON-RPC Client ────────────────────────────────────────────────────
let idCounter = 0;
/**
 * Create a typed JSON-RPC 2.0 HTTP client.
 *
 * @example
 * ```typescript
 * const rpc = createJsonRpcClient({ url: 'http://rpc.testnet.x3-chain.io:9944' });
 * const validators = await rpc.call<ValidatorInfo[]>('validator_getValidators', []);
 * ```
 */
function createJsonRpcClient(options = {}) {
    const url = options.url ?? getDefaultHttpUrl();
    const timeoutMs = options.timeoutMs ?? 8000;
    async function call(method, params = []) {
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
                }),
                signal: controller.signal,
            });
            const json = (await response.json());
            if (json.error) {
                throw new RpcClientError(`RPC error ${json.error.code}: ${json.error.message}`, json.error.code, json.error.data);
            }
            return json.result;
        }
        catch (err) {
            if (err instanceof RpcClientError)
                throw err;
            if (err instanceof DOMException && err.name === 'AbortError') {
                throw new RpcClientError(`RPC call to ${method} timed out after ${timeoutMs}ms`, -32000);
            }
            throw new RpcClientError(`RPC call to ${method} failed: ${err instanceof Error ? err.message : String(err)}`, -32000);
        }
        finally {
            clearTimeout(timeout);
        }
    }
    return { call, url };
}
/**
 * Create a WebSocket client for subscriptions and real-time data.
 *
 * @example
 * ```typescript
 * const ws = createWsClient({ url: 'ws://rpc.testnet.x3-chain.io:9944' });
 * ws.on('network_subscribeMetrics', (metrics) => {
 *   console.log('Live metrics:', metrics);
 * });`
 * await ws.connect();
 * ```
 */
function createWsClient(options = {}) {
    const url = options.url ?? getDefaultWsUrl();
    const autoReconnect = options.autoReconnect ?? true;
    const maxReconnectAttempts = options.maxReconnectAttempts ?? 10;
    const reconnectDelayMs = options.reconnectDelayMs ?? 2000;
    let ws = null;
    let reconnectAttempts = 0;
    let reconnectTimer = null;
    let subHandlers = new Map();
    function connect() {
        return new Promise((resolve, reject) => {
            ws = new WebSocket(url);
            ws.onopen = () => {
                reconnectAttempts = 0;
                resolve();
            };
            ws.onmessage = (event) => {
                try {
                    const msg = JSON.parse(event.data);
                    // Route to subscription handler if params.subscription exists
                    const subId = msg?.params?.subscription;
                    if (subId && subHandlers.has(subId)) {
                        subHandlers.get(subId)?.forEach((handler) => handler(msg.params.result));
                    }
                    // Also route by method name for response callbacks
                    if (msg?.method && subHandlers.has(msg.method)) {
                        subHandlers.get(msg.method)?.forEach((handler) => handler(msg.params?.result));
                    }
                }
                catch {
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
                        connect().catch(() => { });
                    }, reconnectDelayMs * Math.min(reconnectAttempts, 5));
                }
            };
        });
    }
    function on(method, handler) {
        const handlers = subHandlers.get(method) ?? new Set();
        handlers.add(handler);
        subHandlers.set(method, handlers);
        // Return unsubscribe function
        return () => {
            handlers.delete(handler);
            if (handlers.size === 0)
                subHandlers.delete(method);
        };
    }
    function send(method, params = []) {
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
    function disconnect() {
        if (reconnectTimer)
            clearTimeout(reconnectTimer);
        ws?.close();
        ws = null;
        subHandlers.clear();
    }
    return { connect, disconnect, on, send, getUrl: () => url };
}
// ── Error type ──────────────────────────────────────────────────────────────
class RpcClientError extends Error {
    code;
    data;
    constructor(message, code = -32000, data) {
        super(message);
        this.name = 'RpcClientError';
        this.code = code;
        this.data = data;
    }
}
exports.RpcClientError = RpcClientError;
// ── Typed method wrappers for every real RPC method ─────────────────────────
/**
 * Create a pre-typed RPC client with method wrappers for all known X3 RPC methods.
 */
function createX3RpcClient(httpUrl) {
    const rpc = createJsonRpcClient({ url: httpUrl });
    return {
        rpc,
        // ── Wallet ──
        wallet: {
            createWallet: (req) => rpc.call('wallet_createWallet', [req]),
            importWallet: (req) => rpc.call('wallet_importWallet', [req]),
            getBalance: (req) => rpc.call('wallet_getBalance', [req]),
            signTransaction: (req) => rpc.call('wallet_signTransaction', [req]),
            submitTransaction: (req) => rpc.call('wallet_submitTransaction', [req]),
            getTransactions: (req) => rpc.call('wallet_getTransactions', [req]),
            getWalletStatus: (req) => rpc.call('wallet_getWalletStatus', [req]),
            listWallets: (req) => rpc.call('wallet_listWallets', [req]),
            setNetwork: (req) => rpc.call('wallet_setNetwork', [req]),
            getNetworks: () => rpc.call('wallet_getNetworks', []),
        },
        // ── DEX ──
        dex: {
            estimateSwap: (req) => rpc.call('walletDex_estimateSwap', [req]),
            executeSwap: (req) => rpc.call('walletDex_executeSwap', [req]),
        },
        // ── Validator ──
        validator: {
            getValidators: () => rpc.call('validator_getValidators', []),
            getLeaderboard: () => rpc.call('validator_getLeaderboard', []),
            getMetrics: () => rpc.call('validator_getMetrics', []),
        },
        // ── Signing ──
        sign: {
            ed25519: (msg, secret) => rpc.call('x3_sign_ed25519', [msg, secret]),
            secp256k1: (msg, secret) => rpc.call('x3_sign_secp256k1', [msg, secret]),
            sr25519: (msg, secret) => rpc.call('x3_sign_sr25519', [msg, secret]),
            verify: (msg, sig, pk, keyType) => rpc.call('x3_verify_signature', [msg, sig, pk, keyType]),
        },
        // ── Cross-VM ──
        crossVm: {
            /** Query recent cross-VM asset transfers (wired to bridge pallet; degrades to empty array). */
            getRecentTransfers: (limit) => rpc.call('crossVm_getRecentTransfers', [{ limit: limit ?? 10 }]),
        },
        // ── Swarm ──
        swarm: {
            /** Proxy x3-swarm-api :8787 health + scoreboard; degrades gracefully when unreachable. */
            getMetrics: () => rpc.call('swarm_getMetrics', []),
            /** Proxy x3-swarm-api :8787/tasks for recent task list. */
            getRecentTasks: (limit) => rpc.call('swarm_getRecentTasks', [{ limit: limit ?? 10 }]),
        },
        // ── Token ──
        token: {
            /** Query total token supply from balances pallet runtime API; degrades to zero. */
            getSupply: () => rpc.call('token_getSupply', []),
        },
        // ── Network (subscription) ──
        network: {
            subscribeMetrics: () => rpc.call('network_subscribeMetrics', []),
        },
        // ── Weight meter ──
        weight: {
            meter: (config) => rpc.call('x3_weight_meter', [config]),
        },
    };
}
//# sourceMappingURL=rpc-client.js.map