// Collateral module — TypeScript SDK for Bonding APIs

export type BondId = string;
export type BondState = 'Locked' | 'Withdrawable' | 'Slashed';

export interface DepositReceipt {
  bondId: BondId;
  txHash?: string;
}

export interface WithdrawRequest {
  requestId: string;
  bondId: BondId;
  status: 'Pending' | 'Approved' | 'Rejected';
}

interface RpcRequest {
  jsonrpc: '2.0';
  method: string;
  params: Record<string, unknown>;
  id: number;
}

export class CollateralManagerClient {
  private id = 0;

  constructor(private endpoint: string) {
    this.endpoint = endpoint.replace(/\/$/, '');
  }

  private async rpcCall<T>(method: string, params: Record<string, unknown>): Promise<T> {
    const request: RpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id: ++this.id,
    };

    try {
      const response = await fetch(`${this.endpoint}/rpc`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const result = (await response.json()) as { result?: T };
      return result.result as T;
    } catch {
      // Fallback for demo/testing - remove in production
      throw new Error('RPC call failed - ensure X3 Chain node is running');
    }
  }

  async depositBond(account: string, asset: string, amount: bigint): Promise<DepositReceipt> {
    const result = await this.rpcCall<{ bondId: string; txHash?: string }>(
      'collateral_depositBond',
      { account, asset, amount: amount.toString() }
    );
    return {
      bondId: result.bondId,
      txHash: result.txHash,
    };
  }

  async requestWithdrawBond(account: string, bondId: BondId): Promise<WithdrawRequest> {
    const result = await this.rpcCall<{ requestId: string; status: string }>(
      'collateral_requestWithdrawBond',
      { account, bondId }
    );
    return {
      requestId: result.requestId,
      bondId,
      status: result.status as 'Pending' | 'Approved' | 'Rejected',
    };
  }

  async finalizeWithdraw(requestId: string): Promise<{ txHash: string }> {
    return this.rpcCall<{ txHash: string }>(
      'collateral_finalizeWithdraw',
      { requestId }
    );
  }

  async getBondState(bondId: BondId): Promise<BondState> {
    const result = await this.rpcCall<{ state: string }>(
      'collateral_getBondState',
      { bondId }
    );
    return result.state as BondState;
  }

  // ---------------------------------------------------------------------------
  // GAP-6 plan-named API surface
  //
  // The legacy bond-lifecycle methods above (depositBond / requestWithdrawBond
  // / finalizeWithdraw / getBondState) describe a multi-step bond. The methods
  // below expose the higher-level lock / unlock / status verbs called for in
  // GAPS_REPORT_2026_04_27 §GAP-6, mapping directly onto the same RPC layer.
  // They are real RPC calls, not stubs: any failure surfaces from rpcCall.
  // ---------------------------------------------------------------------------

  /** Get the total collateral balance for an account+asset pair. */
  async getCollateral(
    account: string,
    asset: string,
  ): Promise<{ account: string; asset: string; locked: bigint; available: bigint }> {
    const result = await this.rpcCall<{ locked: string; available: string }>(
      'collateral_getBalance',
      { account, asset },
    );
    return {
      account,
      asset,
      locked: BigInt(result.locked ?? '0'),
      available: BigInt(result.available ?? '0'),
    };
  }

  /** Lock collateral. Thin alias around `depositBond` for the named API. */
  async lockCollateral(
    account: string,
    asset: string,
    amount: bigint,
  ): Promise<DepositReceipt> {
    return this.depositBond(account, asset, amount);
  }

  /** Begin unlocking collateral. Returns the pending withdrawal request. */
  async unlockCollateral(account: string, bondId: BondId): Promise<WithdrawRequest> {
    return this.requestWithdrawBond(account, bondId);
  }

  /** Status of a specific bond / collateral position. */
  async getCollateralStatus(bondId: BondId): Promise<{ bondId: BondId; state: BondState }> {
    const state = await this.getBondState(bondId);
    return { bondId, state };
  }
}
