/**
 * X3 Chain DEX RPC Client
 * 
 * Connects to node/src/rpc.rs walletDex_* RPC methods via WebSocket
 */

export interface SwapRequest {
  token_in: string;  // Hex address (64 chars)
  token_out: string; // Hex address (64 chars)
  amount_in: string; // u128 as string
  min_amount_out: string; // u128 as string
  wallet_id: string; // Hex (64 chars)
  require_approval: boolean;
  approval_threshold: string; // u128 as string
}

export interface SwapResponse {
  swap_id: string;
  amount_out: string;
  approval_required: boolean;
  approval_request_id: string | null;
  estimated_gas: string;
}

export interface BalanceRequest {
  wallet_id: string;
  token: string;
}

export interface BalanceResponse {
  free: string;
  reserved: string;
  frozen: string;
}

/**
 * X3 DEX RPC Client
 * Wraps walletDex_* RPC methods
 */
export class X3DexRpcClient {
  private ws: WebSocket | null = null;
  private requestId = 0;
  private pendingRequests = new Map<number, {
    resolve: (value: any) => void;
    reject: (error: any) => void;
  }>();

  constructor(private endpoint: string = 'ws://localhost:9944') {}

  /**
   * Connect to X3 node WebSocket RPC endpoint
   */
  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.endpoint);

      this.ws.onopen = () => {
        console.log(`Connected to X3 RPC at ${this.endpoint}`);
        resolve();
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        reject(error);
      };

      this.ws.onmessage = (event) => {
        try {
          const response = JSON.parse(event.data);
          
          if (response.id !== undefined) {
            const pending = this.pendingRequests.get(response.id);
            
            if (pending) {
              this.pendingRequests.delete(response.id);
              
              if (response.error) {
                pending.reject(new Error(response.error.message || 'RPC error'));
              } else {
                pending.resolve(response.result);
              }
            }
          }
        } catch (error) {
          console.error('Failed to parse RPC response:', error);
        }
      };

      this.ws.onclose = () => {
        console.log('WebSocket connection closed');
        // Reject all pending requests
        for (const [id, pending] of this.pendingRequests.entries()) {
          pending.reject(new Error('Connection closed'));
          this.pendingRequests.delete(id);
        }
      };
    });
  }

  /**
   * Close WebSocket connection
   */
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  /**
   * Send JSON-RPC request
   */
  private async request(method: string, params: any[]): Promise<any> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket not connected');
    }

    const id = ++this.requestId;
    const request = {
      jsonrpc: '2.0',
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject });
      this.ws!.send(JSON.stringify(request));

      // Timeout after 30 seconds
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error('Request timeout'));
        }
      }, 30000);
    });
  }

  /**
   * Estimate swap output amount
   */
  async estimateSwap(request: SwapRequest): Promise<SwapResponse> {
    return await this.request('walletDex_estimateSwap', [
      request.token_in,
      request.token_out,
      request.amount_in,
      request.min_amount_out,
      request.wallet_id,
      request.require_approval,
      request.approval_threshold,
    ]);
  }

  /**
   * Execute swap
   */
  async executeSwap(request: SwapRequest): Promise<SwapResponse> {
    return await this.request('walletDex_executeSwap', [
      request.token_in,
      request.token_out,
      request.amount_in,
      request.min_amount_out,
      request.wallet_id,
      request.require_approval,
      request.approval_threshold,
    ]);
  }

  /**
   * Get token balance
   */
  async getBalance(walletId: string, token: string): Promise<BalanceResponse> {
    return await this.request('walletDex_getBalance', [walletId, token]);
  }

  /**
   * Get approval status
   */
  async getApprovalStatus(approvalId: string): Promise<{
    approved: boolean;
    threshold_met: boolean;
    approvers: string[];
  }> {
    return await this.request('walletDex_getApprovalStatus', [approvalId]);
  }
}

/**
 * Create singleton RPC client instance
 */
let rpcClient: X3DexRpcClient | null = null;

export function getRpcClient(endpoint?: string): X3DexRpcClient {
  if (!rpcClient) {
    rpcClient = new X3DexRpcClient(endpoint);
  }
  return rpcClient;
}

/**
 * Hook for React components to use RPC client
 */
export function useX3RpcClient(endpoint?: string) {
  const client = getRpcClient(endpoint);

  // Auto-connect on mount if not connected
  if (!client['ws'] || client['ws'].readyState !== WebSocket.OPEN) {
    client.connect().catch(console.error);
  }

  return client;
}
