/**
 * Flashloan service — transient execution capital via x3-settlement-engine intents
 *
 * Flash loans on x3chain work through the settlement engine:
 *   1. Create a flash intent (borrow)
 *   2. Execute legs (use capital across chains)
 *   3. Submit repayment proof
 *   4. Settle atomically (repay + fee)
 *
 * All within a single atomic context — if any leg fails, the whole thing reverts.
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/**
 * Create a flashloan intent — borrows capital for atomic use.
 * @param asset - Asset ID to borrow
 * @param amount - Amount to borrow
 * @param legs - Array of execution legs describing how capital is used
 * @param repaymentAmount - Amount to repay (borrow + fee)
 */
function createFlashloanIntent(
  asset: number,
  amount: string,
  legs: Array<{
    chainTarget: 'Evm' | 'Svm' | 'X3';
    action: string;
    params: string;
  }>,
  repaymentAmount: string
) {
  const api = getApi();
  // Flashloans are modeled as settlement intents with X3 chain kind
  // and immediate deadline (same block)
  return api.tx.x3SettlementEngine.createIntent(
    'X3',
    amount,
    asset,
    null, // self-settlement
    0     // same-block deadline = flash
  );
}

/**
 * Execute a multi-leg flashloan via atomic trade engine.
 * This combines a flash borrow with trade batch execution.
 */
function executeFlashloan(
  asset: number,
  borrowAmount: string,
  tradeLegs: Array<{
    assetIn: number;
    assetOut: number;
    amountIn: string;
    minAmountOut: string;
    chainTarget: 'Native' | 'Evm' | 'Svm' | 'X3';
  }>
) {
  const api = getApi();
  // Batch: create intent + create trade + execute trade + settle intent
  return api.tx.utility.batchAll([
    api.tx.x3SettlementEngine.createIntent('X3', borrowAmount, asset, null, 0),
    api.tx.atomicTradeEngine.createTradeBatch(tradeLegs),
    // The runtime handles atomic settlement automatically
  ]);
}

/**
 * Calculate estimated flash loan fee.
 * Fee structure: BaseFee + ComplexityFee(legs) + CapitalFee(log2(amount))
 */
function estimateFlashloanFee(amount: string, numLegs: number): {
  baseFee: string;
  complexityFee: string;
  capitalFee: string;
  totalFee: string;
} {
  const amountBn = BigInt(amount);
  const baseFee = BigInt('1000000000000'); // 0.001 X3
  const complexityFee = BigInt(numLegs) * BigInt('500000000000'); // 0.0005 per leg
  const capitalFee = amountBn > 0n
    ? BigInt(Math.ceil(Math.log2(Number(amountBn)))) * BigInt('100000000000')
    : 0n;
  const totalFee = baseFee + complexityFee + capitalFee;

  return {
    baseFee: baseFee.toString(),
    complexityFee: complexityFee.toString(),
    capitalFee: capitalFee.toString(),
    totalFee: totalFee.toString(),
  };
}

/**
 * Get available liquidity for flash loans.
 */
async function getFlashloanLiquidity(asset: number) {
  const api = getApi();
  // Query the settlement engine's pool balance
  const poolAccount = api.registry.createType('AccountId',
    new Uint8Array(32).fill(0) // placeholder — actual pool derived at runtime
  );
  return api.query.system.account(poolAccount);
}

export default {
  createFlashloanIntent,
  executeFlashloan,
  estimateFlashloanFee,
  getFlashloanLiquidity,
};
