/**
 * X3 Chain Integration for X3 Chain TS SDK
 *
 * Extends the core SDK with x3-specific functionality:
 * - X3 Settlement Engine client
 * - Atomic Trade Engine client
 * - X3 Domain Registry client
 * - X3 Verifier client
 * - X3VM contract interaction
 */

import type { ApiPromise } from '@polkadot/api';
import { hexToU8a } from '@polkadot/util';

// =============================================================================
// X3 Settlement Client
// =============================================================================

export interface X3SettlementOptions {
  api: ApiPromise;
}

export class X3SettlementClient {
  constructor(private api: ApiPromise) {}

  /** Create a cross-chain settlement intent */
  async createIntent(params: {
    taker: string;
    assetA: { chain: string; assetId: string; amount: bigint };
    assetB: { chain: string; assetId: string; amount: bigint };
    secretHash: string;
    timeoutSeconds?: number;
  }) {
    return this.api.tx.x3SettlementEngine.createIntent(
      params.taker,
      { chain: params.assetA.chain, asset_id: params.assetA.assetId, amount: params.assetA.amount },
      { chain: params.assetB.chain, asset_id: params.assetB.assetId, amount: params.assetB.amount },
      params.secretHash,
      params.timeoutSeconds ?? null,
    );
  }

  /** Lock escrow for a leg of the settlement */
  async lockEscrow(params: {
    intentId: string;
    legIndex: number;
    chain: string;
    amount: bigint;
    escrowData: Uint8Array | string;
  }) {
    return this.api.tx.x3SettlementEngine.lockEscrow(
      params.intentId,
      params.legIndex,
      params.chain,
      params.amount,
      typeof params.escrowData === 'string' ? hexToU8a(params.escrowData) : params.escrowData,
    );
  }

  /** Legacy wrapper retained for test compatibility */
  lockEscrowLegacy(intentId: string, amount: number) {
    return this.api.tx.x3SettlementEngine.lockEscrow(intentId, 0, 'Legacy', BigInt(amount), []);
  }

  /** Claim a settlement by revealing the HTLC secret */
  async claimSettlement(intentId: string, secret: string) {
    return this.api.tx.x3SettlementEngine.claimSettlement(intentId, secret);
  }

  /** Refund an expired settlement */
  async refundSettlement(intentId: string) {
    return this.api.tx.x3SettlementEngine.refundSettlement(intentId);
  }

  /** Get settlement intent info */
  async getIntent(intentId: string) {
    const result = await this.api.query.x3SettlementEngine.settlementIntents(intentId);
    return (result as any).toJSON?.() ?? null;
  }

  /** Get intent state */
  async getIntentState(intentId: string): Promise<string> {
    const result = await this.api.query.x3SettlementEngine.intentStates(intentId);
    return (result as any).toString();
  }

  /** Deposit a bond */
  async depositBond(asset: string, amount: bigint, bondType: number) {
    return this.api.tx.x3SettlementEngine.depositBond(asset, amount, bondType);
  }

  /** Get total settlement stats */
  async getStats() {
    const [total, volume, violations] = await Promise.all([
      this.api.query.x3SettlementEngine.totalIntents(),
      this.api.query.x3SettlementEngine.totalSettledVolume(),
      this.api.query.x3SettlementEngine.invariantViolations(),
    ]);
    return {
      totalIntents: (total as any).toNumber?.() ?? 0,
      totalVolume: (volume as any).toBigInt?.() ?? 0n,
      violations: (violations as any).toNumber?.() ?? 0,
    };
  }
}

// =============================================================================
// X3 Atomic Trade Client
// =============================================================================

export const X3VmType = {
  Evm: 'Evm',
  Svm: 'Svm',
  X3: 'X3',
  CrossVm: 'CrossVm',
} as const;
export type X3VmType = (typeof X3VmType)[keyof typeof X3VmType];

export const X3AmmProtocol = {
  UniswapV2: 'UniswapV2',
  UniswapV3: 'UniswapV3',
  Raydium: 'Raydium',
  Orca: 'Orca',
  Jupiter: 'Jupiter',
  SushiSwap: 'SushiSwap',
  PancakeSwap: 'PancakeSwap',
  Curve: 'Curve',
  Balancer: 'Balancer',
  AtlasNative: 'AtlasNative',
} as const;
export type X3AmmProtocol = (typeof X3AmmProtocol)[keyof typeof X3AmmProtocol];

export interface X3TradeLeg {
  ammProtocol: X3AmmProtocol;
  vmType: X3VmType;
  assetIn: string;
  assetOut: string;
  amountIn: bigint;
  minAmountOut: bigint;
  routeData?: Uint8Array | string;
}

export class X3AtomicTradeClient {
  constructor(private api: ApiPromise) {}

  /** Create a multi-leg trade batch */
  async createTradeBatch(params: {
    legs: X3TradeLeg[];
    slippageBps: number;
    deadline: number;
    nonce?: bigint;
  }) {
    const legs = params.legs.map((l) => ({
      amm_protocol: l.ammProtocol,
      vm_type: l.vmType,
      asset_in: l.assetIn,
      asset_out: l.assetOut,
      amount_in: l.amountIn,
      min_amount_out: l.minAmountOut,
      route_data: l.routeData
        ? typeof l.routeData === 'string' ? l.routeData : Array.from(l.routeData)
        : [],
    }));
    return this.api.tx.atomicTradeEngine.createTradeBatch(
      legs, params.slippageBps, params.deadline, params.nonce ?? 0n,
    );
  }

  /** Legacy wrappers retained for test compatibility */
  createBatch(legs: Array<{
    fromVm: X3VmType;
    toVm: X3VmType;
    fromAsset: string;
    toAsset: string;
    amount: number;
    minOut: number;
  }>) {
    const mapped: X3TradeLeg[] = legs.map((leg) => ({
      ammProtocol: X3AmmProtocol.AtlasNative,
      vmType: leg.fromVm,
      assetIn: leg.fromAsset,
      assetOut: leg.toAsset,
      amountIn: BigInt(leg.amount),
      minAmountOut: BigInt(leg.minOut),
    }));
    return this.createTradeBatch({
      legs: mapped,
      slippageBps: 0,
      deadline: Math.floor(Date.now() / 1000) + 600,
    });
  }

  executeBatch(batchId: string) {
    return this.executeTradeBatch(batchId);
  }

  cancelBatch(batchId: string) {
    return this.cancelTradeBatch(batchId);
  }

  /** Execute a pending trade batch */
  async executeTradeBatch(batchId: string) {
    return this.api.tx.atomicTradeEngine.executeTradeBatch(batchId);
  }

  /** Execute via Kernel ComitV2 (tri-VM) */
  async executeTradeBatchViaKernel(batchId: string, comitId: string) {
    return this.api.tx.atomicTradeEngine.executeTradeBatchViaKernelComitV2(batchId, comitId);
  }

  /** Cancel a pending batch */
  async cancelTradeBatch(batchId: string) {
    return this.api.tx.atomicTradeEngine.cancelTradeBatch(batchId);
  }

  /** Get batch info */
  async getBatch(batchId: string) {
    const result = await this.api.query.atomicTradeEngine.tradeBatches(batchId);
    return (result as any).toJSON?.() ?? null;
  }

  /** Get TWAP price */
  async getTwap(tokenA: string, tokenB: string) {
    const result = await this.api.query.atomicTradeEngine.twapData([tokenA, tokenB]);
    return (result as any).toJSON?.() ?? null;
  }

  /** Get protocol stats */
  async getStats() {
    const [completed, failed, volume] = await Promise.all([
      this.api.query.atomicTradeEngine.completedBatchCount(),
      this.api.query.atomicTradeEngine.failedBatchCount(),
      this.api.query.atomicTradeEngine.totalVolume(),
    ]);
    return {
      completed: (completed as any).toNumber?.() ?? 0,
      failed: (failed as any).toNumber?.() ?? 0,
      totalVolume: (volume as any).toBigInt?.() ?? 0n,
    };
  }
}

// =============================================================================
// X3 Domain Client
// =============================================================================

export class X3DomainClient {
  constructor(private api: ApiPromise) {}

  /** Register a .x3 domain */
  async registerDomain(domain: string) {
    const bytes = new TextEncoder().encode(domain.endsWith('.x3') ? domain : `${domain}.x3`);
    return this.api.tx.x3DomainRegistry.registerDomain(bytes);
  }

  register(domain: string) {
    return this.registerDomain(domain);
  }

  /** Set DNS records */
  async setRecords(domain: string, records: Array<{ ttl: number; data: unknown }>) {
    const bytes = new TextEncoder().encode(domain.endsWith('.x3') ? domain : `${domain}.x3`);
    return this.api.tx.x3DomainRegistry.setRecords(bytes, records);
  }

  setRecordsLegacy(domain: string, records: Array<{ key: string; value: unknown }>) {
    const mapped = records.map((record) => ({
      ttl: 0,
      data: { key: record.key, value: record.value },
    }));
    return this.setRecords(domain, mapped);
  }

  /** Get domain info */
  async getDomain(domain: string) {
    const bytes = new TextEncoder().encode(domain.endsWith('.x3') ? domain : `${domain}.x3`);
    const result = await this.api.query.x3DomainRegistry.domains(bytes);
    return (result as any).toJSON?.() ?? null;
  }

  lookup(domain: string) {
    return this.getDomain(domain);
  }

  /** Check availability */
  async isAvailable(domain: string): Promise<boolean> {
    return (await this.getDomain(domain)) === null;
  }
}

// =============================================================================
// X3 Verifier Client
// =============================================================================

export class X3VerifierClient {
  constructor(private api: ApiPromise) {}

  /** Register as executor */
  async registerExecutor(stake: bigint) {
    return this.api.tx.x3Verifier.registerExecutor(stake);
  }

  registerExecutorLegacy(_executorId: string, stake: number) {
    return this.api.tx.x3Verifier.registerExecutor(BigInt(stake));
  }

  /** Submit a verification job */
  async submitJob(bytecodeHash: string, inputHash: string, gasLimit: bigint, reward: bigint) {
    return this.api.tx.x3Verifier.submitJob(bytecodeHash, inputHash, gasLimit, reward);
  }

  submitJobLegacy(bytecodeHash: string, params: { gasLimit: number }) {
    return this.api.tx.x3Verifier.submitJob(bytecodeHash, '0x', BigInt(params.gasLimit), 0n);
  }

  /** Get job info */
  async getJob(jobId: string) {
    const result = await this.api.query.x3Verifier.jobs(jobId);
    return (result as any).toJSON?.() ?? null;
  }

  getExecutor(executorId: string) {
    return this.api.query.x3Verifier.executors(executorId).then((result: any) => result.toJSON?.() ?? null);
  }

  /** Get verified state root */
  async getVerifiedStateRoot(jobId: string): Promise<string | null> {
    const result = await this.api.query.x3Verifier.verifiedStateRoots(jobId);
    const hex = (result as any).toHex?.();
    return hex && hex !== '0x' + '00'.repeat(32) ? hex : null;
  }

  /** Get stats */
  async getStats() {
    const [submitted, verified] = await Promise.all([
      this.api.query.x3Verifier.totalJobsSubmitted(),
      this.api.query.x3Verifier.totalJobsVerified(),
    ]);
    return {
      submitted: (submitted as any).toNumber?.() ?? 0,
      verified: (verified as any).toNumber?.() ?? 0,
    };
  }
}

// =============================================================================
// Exports
// =============================================================================

export function createX3SettlementClient(api: ApiPromise): X3SettlementClient {
  return new X3SettlementClient(api);
}

export function createX3TradeClient(api: ApiPromise): X3AtomicTradeClient {
  return new X3AtomicTradeClient(api);
}

export function createX3DomainClient(api: ApiPromise): X3DomainClient {
  return new X3DomainClient(api);
}

export function createX3VerifierClient(api: ApiPromise): X3VerifierClient {
  return new X3VerifierClient(api);
}
