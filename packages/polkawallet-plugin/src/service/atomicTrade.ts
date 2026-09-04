/**
 * Atomic Trade Engine service
 *
 * Maps to pallet: atomic-trade-engine
 * Extrinsics: create_trade_batch, execute_trade_batch, cancel_trade_batch,
 *             register_amm_adapter, submit_price_observation,
 *             execute_trade_batch_via_kernel_comit_v2
 *
 * This enables:
 *   - Multi-leg atomic swaps across EVM/SVM/X3
 *   - AMM adapter registration
 *   - Price oracle submissions
 *   - Comit v2-integrated atomic execution
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/* ─── Queries ─── */

async function getBatchStatus(batchId: number) {
  const api = getApi();
  return (api.rpc as any).atomicTradeEngine.getBatchStatus(batchId);
}

async function simulateTrade(batch: any) {
  const api = getApi();
  return (api.rpc as any).atomicTradeEngine.simulateTrade(batch);
}

async function getPrice(assetId: number) {
  const api = getApi();
  return (api.rpc as any).atomicTradeEngine.getPrice(assetId);
}

async function getAllTradeBatches(account: string) {
  const api = getApi();
  const entries = await api.query.atomicTradeEngine.tradeBatches.entries();
  return entries
    .map(([key, val]: [any, any]) => ({
      id: key.args[0].toString(),
      ...val.toJSON(),
    }))
    .filter((b: any) => b.creator === account);
}

/* ─── Extrinsics ─── */

/**
 * Create an atomic trade batch.
 * @param legs - Array of {assetIn, assetOut, amountIn, minAmountOut, chainTarget}
 */
function createTradeBatch(
  legs: Array<{
    assetIn: number;
    assetOut: number;
    amountIn: string;
    minAmountOut: string;
    chainTarget: 'Native' | 'Evm' | 'Svm' | 'X3';
  }>
) {
  const api = getApi();
  return api.tx.atomicTradeEngine.createTradeBatch(legs);
}

/**
 * Execute a pending trade batch.
 */
function executeTradeBatch(batchId: number) {
  const api = getApi();
  return api.tx.atomicTradeEngine.executeTradeBatch(batchId);
}

/**
 * Cancel a pending trade batch.
 */
function cancelTradeBatch(batchId: number) {
  const api = getApi();
  return api.tx.atomicTradeEngine.cancelTradeBatch(batchId);
}

/**
 * Register an AMM adapter for routing.
 */
function registerAmmAdapter(adapterAddress: string, chainTarget: string) {
  const api = getApi();
  return api.tx.atomicTradeEngine.registerAmmAdapter(adapterAddress, chainTarget);
}

/**
 * Submit a price observation (oracle feed).
 */
function submitPriceObservation(assetId: number, price: string, source: string) {
  const api = getApi();
  return api.tx.atomicTradeEngine.submitPriceObservation(assetId, price, source);
}

/**
 * Execute trade batch via Kernel Comit v2 (full tri-VM atomic execution).
 */
function executeTradeBatchViaComitV2(batchId: number) {
  const api = getApi();
  return api.tx.atomicTradeEngine.executeTradeBatchViaKernelComitV2(batchId);
}

/**
 * Subscribe to trade batch status changes.
 */
async function subscribeTradeBatch(batchId: number, msgChannel: string) {
  const api = getApi();
  return api.query.atomicTradeEngine.tradeBatches(batchId, (batch: any) => {
    (window as any).send(msgChannel, {
      batchId,
      ...batch.toJSON(),
    });
  });
}

export default {
  // queries
  getBatchStatus,
  simulateTrade,
  getPrice,
  getAllTradeBatches,
  // extrinsics
  createTradeBatch,
  executeTradeBatch,
  cancelTradeBatch,
  registerAmmAdapter,
  submitPriceObservation,
  executeTradeBatchViaComitV2,
  // subscriptions
  subscribeTradeBatch,
};
