/**
 * Shared JSON-RPC 2.0 client utilities for X3 Chain frontends.
 *
 * Provides lightweight HTTP+WS RPC helpers that all Tauri frontends,
 * standalone apps, and static pages can use to call node RPC methods
 * without bolting into the full AtlasSphereClient SDK.
 *
 * @module @x3-chain/ts-sdk/rpc-client
 */
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
    /** Base URL of the JSON-RPC HTTP endpoint (default: http://rpc.testnet.x3-chain.io:9944). */
    url?: string;
    /** Request timeout in milliseconds (default: 8000). */
    timeoutMs?: number;
}
export interface WsClientOptions {
    /** WebSocket URL (default: ws://rpc.testnet.x3-chain.io:9944). */
    url?: string;
    /** Auto-reconnect on close (default: true). */
    autoReconnect?: boolean;
    /** Max reconnect attempts (0 = infinite). */
    maxReconnectAttempts?: number;
    /** Reconnect delay in ms (default: 2000). */
    reconnectDelayMs?: number;
}
/**
 * Create a typed JSON-RPC 2.0 HTTP client.
 *
 * @example
 * ```typescript
 * const rpc = createJsonRpcClient({ url: 'http://rpc.testnet.x3-chain.io:9944' });
 * const validators = await rpc.call<ValidatorInfo[]>('validator_getValidators', []);
 * ```
 */
export declare function createJsonRpcClient(options?: JsonRpcClientOptions): {
    call: <T = unknown>(method: string, params?: unknown[]) => Promise<T>;
    url: string;
};
type MessageHandler<T = unknown> = (data: T) => void;
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
export declare function createWsClient(options?: WsClientOptions): {
    connect: () => Promise<void>;
    disconnect: () => void;
    on: <T = unknown>(method: string, handler: MessageHandler<T>) => () => void;
    send: (method: string, params?: unknown[]) => void;
    getUrl: () => string;
};
export declare class RpcClientError extends Error {
    readonly code: number;
    readonly data: unknown;
    constructor(message: string, code?: number, data?: unknown);
}
/**
 * Create a pre-typed RPC client with method wrappers for all known X3 RPC methods.
 */
export declare function createX3RpcClient(httpUrl?: string): {
    rpc: {
        call: <T = unknown>(method: string, params?: unknown[]) => Promise<T>;
        url: string;
    };
    wallet: {
        createWallet: (req: unknown) => Promise<unknown>;
        importWallet: (req: unknown) => Promise<unknown>;
        getBalance: (req: unknown) => Promise<unknown>;
        signTransaction: (req: unknown) => Promise<unknown>;
        submitTransaction: (req: unknown) => Promise<unknown>;
        getTransactions: (req: unknown) => Promise<unknown>;
        getWalletStatus: (req: unknown) => Promise<unknown>;
        listWallets: (req: unknown) => Promise<unknown>;
        setNetwork: (req: unknown) => Promise<unknown>;
        getNetworks: () => Promise<unknown>;
    };
    dex: {
        estimateSwap: (req: unknown) => Promise<unknown>;
        executeSwap: (req: unknown) => Promise<unknown>;
    };
    validator: {
        getValidators: () => Promise<unknown>;
        getLeaderboard: () => Promise<unknown>;
        getMetrics: () => Promise<unknown>;
    };
    sign: {
        ed25519: (msg: string, secret: string) => Promise<unknown>;
        secp256k1: (msg: string, secret: string) => Promise<unknown>;
        sr25519: (msg: string, secret: string) => Promise<unknown>;
        verify: (msg: string, sig: string, pk: string, keyType: string) => Promise<unknown>;
    };
    crossVm: {
        /** Query recent cross-VM asset transfers (wired to bridge pallet; degrades to empty array). */
        getRecentTransfers: (limit?: number) => Promise<unknown>;
    };
    swarm: {
        /** Proxy x3-swarm-api :8787 health + scoreboard; degrades gracefully when unreachable. */
        getMetrics: () => Promise<unknown>;
        /** Proxy x3-swarm-api :8787/tasks for recent task list. */
        getRecentTasks: (limit?: number) => Promise<unknown>;
    };
    token: {
        /** Query total token supply from balances pallet runtime API; degrades to zero. */
        getSupply: () => Promise<unknown>;
    };
    network: {
        subscribeMetrics: () => Promise<unknown>;
    };
    weight: {
        meter: (config: unknown) => Promise<unknown>;
    };
};
export {};
//# sourceMappingURL=rpc-client.d.ts.map