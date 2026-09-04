'use strict';

// src/core/tx-helper.ts
async function signAndSend(tx, account, statusCallback) {
  return new Promise((resolve, reject) => {
    const unsubPromise = tx.signAndSend(account, (result) => {
      const status = { status: "pending" };
      if (result.status.isInBlock) {
        status.status = "inBlock";
        status.blockHash = result.status.asInBlock.toHex();
        status.txHash = result.txHash.toHex();
        statusCallback?.(status);
      }
      if (result.status.isFinalized) {
        const blockHash = result.status.asFinalized.toHex();
        const events = result.events.map((record) => ({
          type: `${record.event.section}.${record.event.method}`,
          data: record.event.data.toJSON()
        }));
        const dispatchError = result.events.find(
          ({ event }) => event.section === "system" && event.method === "ExtrinsicFailed"
        );
        if (dispatchError) {
          const errorStatus = {
            status: "error",
            blockHash,
            txHash: result.txHash.toHex(),
            error: "ExtrinsicFailed",
            events
          };
          statusCallback?.(errorStatus);
          reject(new Error(`Extrinsic failed in block ${blockHash}`));
          return;
        }
        const finalStatus = {
          status: "finalized",
          blockHash,
          txHash: result.txHash.toHex(),
          events
        };
        statusCallback?.(finalStatus);
        resolve({
          blockHash,
          blockNumber: 0,
          // populated by caller if needed
          txHash: result.txHash.toHex(),
          events
        });
      }
      if (result.isError) {
        const errorStatus = {
          status: "error",
          error: "Transaction error"
        };
        statusCallback?.(errorStatus);
        reject(new Error("Transaction error"));
      }
    });
    unsubPromise.catch((err) => {
      statusCallback?.({ status: "error", error: err.message });
      reject(err);
    });
  });
}

// src/services/trades.ts
var AtomicTradeService = class {
  constructor(api) {
    this.api = api;
  }
  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------
  /** Create a multi-leg trade batch across EVM/SVM/X3 */
  async createTradeBatch(account, params, statusCb) {
    const legs = params.legs.map((l) => this._encodeTradeLeg(l));
    const nonce = params.nonce ?? 0n;
    const tx = this.api.tx.atomicTradeEngine.createTradeBatch(
      legs,
      params.slippageToleranceBps,
      params.deadline,
      nonce
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Execute a pending trade batch */
  async executeTradeBatch(account, batchId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.executeTradeBatch(batchId);
    const result = await signAndSend(tx, account, statusCb);
    return this._parseTradeResult(batchId, result);
  }
  /** Execute trade batch through the Kernel ComitV2 path (tri-VM) */
  async executeTradeBatchViaKernel(account, batchId, comitId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.executeTradeBatchViaKernelComitV2(
      batchId,
      comitId
    );
    const result = await signAndSend(tx, account, statusCb);
    return this._parseTradeResult(batchId, result);
  }
  /** Cancel a pending trade batch */
  async cancelTradeBatch(account, batchId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.cancelTradeBatch(batchId);
    return signAndSend(tx, account, statusCb);
  }
  /** Create a manual checkpoint for a batch */
  async createCheckpoint(account, batchId, statusCb) {
    const tx = this.api.tx.atomicTradeEngine.createManualCheckpoint(batchId);
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // High-level convenience: create & execute in one shot
  // ---------------------------------------------------------------------------
  /** Create and immediately execute a trade batch */
  async trade(account, params, statusCb) {
    const createResult = await this.createTradeBatch(account, params, statusCb);
    const createdEvent = createResult.events.find(
      (e) => e.type === "atomicTradeEngine.TradeBatchCreated"
    );
    const batchId = createdEvent?.data?.batch_id ?? "";
    if (!batchId) {
      throw new Error("Failed to extract batch_id from TradeBatchCreated event");
    }
    return this.executeTradeBatch(account, batchId, statusCb);
  }
  /**
   * Convenience: single-leg swap (the most common case)
   */
  async swap(account, opts, statusCb) {
    const currentBlock = (await this.api.rpc.chain.getHeader()).number.toNumber();
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
            routeData: opts.routeData
          }
        ],
        slippageToleranceBps: opts.slippageBps ?? 50,
        deadline: opts.deadline ?? currentBlock + 100
      },
      statusCb
    );
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get trade batch info by ID */
  async getBatch(batchId) {
    const batch = await this.api.query.atomicTradeEngine.tradeBatches(batchId);
    const json = batch.toJSON?.();
    if (!json) return null;
    return {
      batchId,
      creator: json.creator,
      legs: (json.legs ?? []).map(this._decodeTradeLeg),
      slippageToleranceBps: json.slippage_tolerance_bps,
      deadline: json.deadline,
      status: json.status,
      createdAt: json.created_at
    };
  }
  /** Get all pending batch IDs for an account */
  async getPendingBatches(account) {
    const batches = await this.api.query.atomicTradeEngine.pendingBatches(account);
    return batches.toJSON?.() ?? [];
  }
  /** Get TWAP price for a token pair */
  async getTwap(tokenA, tokenB) {
    const twap = await this.api.query.atomicTradeEngine.twapData([tokenA, tokenB]);
    const json = twap.toJSON?.();
    if (!json) return null;
    return {
      cumulativePrice: BigInt(json.cumulative_price ?? "0"),
      lastUpdate: json.last_update ?? 0
    };
  }
  /** Get AMM adapter configuration */
  async getAmmAdapter(protocol) {
    const adapter = await this.api.query.atomicTradeEngine.ammAdapters(protocol);
    return adapter.toJSON?.() ?? null;
  }
  /** Get protocol stats */
  async getStats() {
    const [completed, failed, totalVolume] = await Promise.all([
      this.api.query.atomicTradeEngine.completedBatchCount(),
      this.api.query.atomicTradeEngine.failedBatchCount(),
      this.api.query.atomicTradeEngine.totalVolume()
    ]);
    return {
      completedBatches: completed.toNumber?.() ?? 0,
      failedBatches: failed.toNumber?.() ?? 0,
      totalVolume: totalVolume.toBigInt?.() ?? 0n
    };
  }
  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------
  _encodeTradeLeg(leg) {
    return {
      amm_protocol: leg.ammProtocol,
      vm_type: leg.vmType,
      asset_in: leg.assetIn,
      asset_out: leg.assetOut,
      amount_in: leg.amountIn,
      min_amount_out: leg.minAmountOut,
      route_data: leg.routeData ? typeof leg.routeData === "string" ? leg.routeData : Array.from(leg.routeData) : []
    };
  }
  _decodeTradeLeg(raw) {
    return {
      ammProtocol: raw.amm_protocol,
      vmType: raw.vm_type,
      assetIn: raw.asset_in,
      assetOut: raw.asset_out,
      amountIn: BigInt(raw.amount_in ?? "0"),
      minAmountOut: BigInt(raw.min_amount_out ?? "0"),
      routeData: raw.route_data
    };
  }
  _parseTradeResult(batchId, result) {
    const completedEvent = result.events.find(
      (e) => e.type === "atomicTradeEngine.TradeBatchCompleted"
    );
    const failedEvent = result.events.find(
      (e) => e.type === "atomicTradeEngine.TradeBatchFailed"
    );
    const legResults = result.events.filter(
      (e) => e.type === "atomicTradeEngine.TradeLegCompleted" || e.type === "atomicTradeEngine.TradeLegFailed"
    ).map((e) => ({
      legIndex: e.data.leg_index ?? 0,
      success: e.type.includes("Completed"),
      amountOut: BigInt(e.data.amount_out ?? "0"),
      error: e.data.reason
    }));
    return {
      batchId,
      success: !!completedEvent && !failedEvent,
      totalInput: BigInt(completedEvent?.data?.total_input ?? "0"),
      totalOutput: BigInt(completedEvent?.data?.total_output ?? "0"),
      gasUsed: BigInt(completedEvent?.data?.gas_used ?? "0"),
      legResults
    };
  }
};

exports.AtomicTradeService = AtomicTradeService;
//# sourceMappingURL=index.js.map
//# sourceMappingURL=index.js.map