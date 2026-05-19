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
export class HttpRpcClient implements RpcClient {
  private nextId = 1;

  constructor(private readonly endpoint: string) {}

  async call<T>(method: string, params: unknown[] = []): Promise<T> {
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
      throw new Error(
        `RPC HTTP error: ${response.status} ${response.statusText}`,
      );
    }

    const body = (await response.json()) as {
      result?: T;
      error?: { code: number; message: string; data?: unknown };
    };

    if (body.error) {
      throw new Error(
        `RPC error ${body.error.code}: ${body.error.message}`,
      );
    }

    return body.result as T;
  }
}
