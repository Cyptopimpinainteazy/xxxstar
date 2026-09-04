/**
 * Atomic Trade Engine Service — cross-VM trade batches,
 * AMM routing, TWAP oracle, and kernel comit v2 integration
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  CreateTradeBatchParams,
  TradeBatchInfo,
  TradeResult,
  TradeLegInput,
  AmmProtocol,
  TxStatusCallback,
} from '../types/interfaces';

export class AtomicTradeService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------

  /** Create a multi-leg trade batch across EVM/SVM/X3 */
  async createTradeBatch(
    account: SignerAccount,
    params: CreateTradeBatchParams,
    statusCb?: TxStatusCallback,
  ) {
    const legs = params.legs.map((l) => this._encodeTradeLeg(l));
    const nonce = params.nonce ?? 0n;

    const tx = this.api.tx.atomicTradeEngine.createTradeBatch(
      legs,
      params.slippageToleranceBps,
      params.deadline,
      nonce,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Execute a pending trade batch */
  async executeTradeBatch(
    account: SignerAccount,
    batchId: string,
    statusCb?: TxStatusCallback,
  ): Promise<TradeResult> {
    const tx = this.api.tx.atomicTradeEngine.executeTradeBatch(batchId);
    const result = await signAndSend(tx, account, statusCb);
    return this._parseTradeResult(batchId, result);
  }

  /** Execute trade batch through the Kernel ComitV2 path (tri-VM) */
  async executeTradeBatchViaKernel(
    account: SignerAccount,
    batchId: string,
    comitId: string,
    statusCb?: TxStatusCallback,
  ): Promise<TradeResult> {
    const tx = this.api.tx.atomicTradeEngine.executeTradeBatchViaKernelComitV2(
      batchId,
      comitId,
    );
    const result = await signAndSend(tx, account, statusCb);
    return this._parseTradeResult(batchId, result);
  }

  /** Cancel a pending trade batch */
  async cancelTradeBatch(
    account: SignerAccount,
    batchId: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.atomicTradeEngine.cancelTradeBatch(batchId);
    return signAndSend(tx, account, statusCb);
  }

  /** Create a manual checkpoint for a batch */
  async createCheckpoint(
    account: SignerAccount,
    batchId: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.atomicTradeEngine.createManualCheckpoint(batchId);
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // High-level convenience: create & execute in one shot
  // ---------------------------------------------------------------------------

  /** Create and immediately execute a trade batch */
  async trade(
    account: SignerAccount,
    params: CreateTradeBatchParams,
    statusCb?: TxStatusCallback,
  ): Promise<TradeResult> {
    // Create the batch
    const createResult = await this.createTradeBatch(account, params, statusCb);
    // Extract batch_id from events
    const createdEvent = createResult.events.find(
      (e) => e.type === 'atomicTradeEngine.TradeBatchCreated',
    );
    const batchId = (createdEvent?.data?.batch_id as string) ?? '';

    if (!batchId) {
      throw new Error('Failed to extract batch_id from TradeBatchCreated event');
    }

    // Execute it
    return this.executeTradeBatch(account, batchId, statusCb);
  }

  /**
   * Convenience: single-leg swap (the most common case)
   */
  async swap(
    account: SignerAccount,
    opts: {
      ammProtocol: AmmProtocol;
      vmType: 'Evm' | 'Svm' | 'X3' | 'CrossVm';
      assetIn: string;
      assetOut: string;
      amountIn: bigint;
      minAmountOut: bigint;
      slippageBps?: number;
      deadline?: number;
      routeData?: Uint8Array | string;
    },
    statusCb?: TxStatusCallback,
  ): Promise<TradeResult> {
    const currentBlock = (
      await this.api.rpc.chain.getHeader()
    ).number.toNumber();

    return this.trade(
      account,
      {
        legs: [
          {
            ammProtocol: opts.ammProtocol,
            vmType: opts.vmType,
            assetIn: opts.assetIn,
            assetOut: opts.assetOut,
            amountIn: opts.amountIn,
            minAmountOut: opts.minAmountOut,
            routeData: opts.routeData,
          },
        ],
        slippageToleranceBps: opts.slippageBps ?? 50,
        deadline: opts.deadline ?? currentBlock + 100,
      },
      statusCb,
    );
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get trade batch info by ID */
  async getBatch(batchId: string): Promise<TradeBatchInfo | null> {
    const batch = await this.api.query.atomicTradeEngine.tradeBatches(batchId);
    const json = (batch as any).toJSON?.();
    if (!json) return null;

    return {
      batchId,
      creator: json.creator,
      legs: (json.legs ?? []).map(this._decodeTradeLeg),
      slippageToleranceBps: json.slippage_tolerance_bps,
      deadline: json.deadline,
      status: json.status,
      createdAt: json.created_at,
    };
  }

  /** Get all pending batch IDs for an account */
  async getPendingBatches(account: string): Promise<string[]> {
    const batches = await this.api.query.atomicTradeEngine.pendingBatches(account);
    return (batches as any).toJSON?.() ?? [];
  }

  /** Get TWAP price for a token pair */
  async getTwap(tokenA: string, tokenB: string): Promise<{ cumulativePrice: bigint; lastUpdate: number } | null> {
    const twap = await this.api.query.atomicTradeEngine.twapData([tokenA, tokenB]);
    const json = (twap as any).toJSON?.();
    if (!json) return null;
    return {
      cumulativePrice: BigInt(json.cumulative_price ?? '0'),
      lastUpdate: json.last_update ?? 0,
    };
  }

  /** Get AMM adapter configuration */
  async getAmmAdapter(protocol: AmmProtocol) {
    const adapter = await this.api.query.atomicTradeEngine.ammAdapters(protocol);
    return (adapter as any).toJSON?.() ?? null;
  }

  /** Get protocol stats */
  async getStats() {
    const [completed, failed, totalVolume] = await Promise.all([
      this.api.query.atomicTradeEngine.completedBatchCount(),
      this.api.query.atomicTradeEngine.failedBatchCount(),
      this.api.query.atomicTradeEngine.totalVolume(),
    ]);

    return {
      completedBatches: (completed as any).toNumber?.() ?? 0,
      failedBatches: (failed as any).toNumber?.() ?? 0,
      totalVolume: (totalVolume as any).toBigInt?.() ?? 0n,
    };
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private _encodeTradeLeg(leg: TradeLegInput) {
    return {
      amm_protocol: leg.ammProtocol,
      vm_type: leg.vmType,
      asset_in: leg.assetIn,
      asset_out: leg.assetOut,
      amount_in: leg.amountIn,
      min_amount_out: leg.minAmountOut,
      route_data: leg.routeData
        ? typeof leg.routeData === 'string'
          ? leg.routeData
          : Array.from(leg.routeData)
        : [],
    };
  }

  private _decodeTradeLeg(raw: any): TradeLegInput {
    return {
      ammProtocol: raw.amm_protocol,
      vmType: raw.vm_type,
      assetIn: raw.asset_in,
      assetOut: raw.asset_out,
      amountIn: BigInt(raw.amount_in ?? '0'),
      minAmountOut: BigInt(raw.min_amount_out ?? '0'),
      routeData: raw.route_data,
    };
  }

  private _parseTradeResult(
    batchId: string,
    result: Awaited<ReturnType<typeof signAndSend>>,
  ): TradeResult {
    const completedEvent = result.events.find(
      (e) => e.type === 'atomicTradeEngine.TradeBatchCompleted',
    );
    const failedEvent = result.events.find(
      (e) => e.type === 'atomicTradeEngine.TradeBatchFailed',
    );

    const legResults = result.events
      .filter(
        (e) =>
          e.type === 'atomicTradeEngine.TradeLegCompleted' ||
          e.type === 'atomicTradeEngine.TradeLegFailed',
      )
      .map((e) => ({
        legIndex: (e.data.leg_index as number) ?? 0,
        success: e.type.includes('Completed'),
        amountOut: BigInt((e.data.amount_out as string) ?? '0'),
        error: e.data.reason as string | undefined,
      }));

    return {
      batchId,
      success: !!completedEvent && !failedEvent,
      totalInput: BigInt((completedEvent?.data?.total_input as string) ?? '0'),
      totalOutput: BigInt((completedEvent?.data?.total_output as string) ?? '0'),
      gasUsed: BigInt((completedEvent?.data?.gas_used as string) ?? '0'),
      legResults,
    };
  }
}
