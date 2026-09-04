/**
 * RpcClient — minimal JSON-RPC client interface for blockchain communication.
 *
 * Implementations can target Substrate (WebSocket) or Solana (HTTP/WS).
 */
/**
 * Simple HTTP-based RPC client for development and testing.
 */
export class HttpRpcClient {
    constructor(endpoint) {
        this.endpoint = endpoint;
        this.nextId = 1;
    }
    async call(method, params = []) {
        const id = this.nextId++;
        const response = await fetch(this.endpoint, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
                jsonrpc: "2.0",
                id,
                method,
                params,
            }),
        });
        if (!response.ok) {
            throw new Error(`RPC HTTP error: ${response.status} ${response.statusText}`);
        }
        const body = (await response.json());
        if (body.error) {
            throw new Error(`RPC error ${body.error.code}: ${body.error.message}`);
        }
        return body.result;
    }
}
