/**
 * RpcClient — minimal JSON-RPC client interface for blockchain communication.
 *
 * Implementations can target Substrate (WebSocket) or Solana (HTTP/WS).
 */
export interface RpcClient {
    /**
     * Execute a typed JSON-RPC call.
     *
     * @param method - RPC method name (e.g. "jury_decisionStatus")
     * @param params - Positional parameters for the call
     * @returns Parsed result of the expected type
     * @throws Error if the RPC call fails or returns an error response
     */
    call<T>(method: string, params?: unknown[]): Promise<T>;
}
/**
 * Simple HTTP-based RPC client for development and testing.
 */
export declare class HttpRpcClient implements RpcClient {
    private readonly endpoint;
    private nextId;
    constructor(endpoint: string);
    call<T>(method: string, params?: unknown[]): Promise<T>;
}
