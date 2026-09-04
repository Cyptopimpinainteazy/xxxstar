"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.X3VerifierClient = exports.X3DomainClient = exports.X3AtomicTradeClient = exports.X3AmmProtocol = exports.X3VmType = exports.X3SettlementClient = void 0;
exports.createX3SettlementClient = createX3SettlementClient;
exports.createX3TradeClient = createX3TradeClient;
exports.createX3DomainClient = createX3DomainClient;
exports.createX3VerifierClient = createX3VerifierClient;
const util_1 = require("@polkadot/util");
class X3SettlementClient {
    api;
    constructor(api) {
        this.api = api;
    }
    /** Create a cross-chain settlement intent */
    async createIntent(params) {
        return this.api.tx.x3SettlementEngine.createIntent(params.taker, { chain: params.assetA.chain, asset_id: params.assetA.assetId, amount: params.assetA.amount }, { chain: params.assetB.chain, asset_id: params.assetB.assetId, amount: params.assetB.amount }, params.secretHash, params.timeoutSeconds ?? null);
    }
    /** Lock escrow for a leg of the settlement */
    async lockEscrow(params) {
        return this.api.tx.x3SettlementEngine.lockEscrow(params.intentId, params.legIndex, params.chain, params.amount, typeof params.escrowData === 'string' ? (0, util_1.hexToU8a)(params.escrowData) : params.escrowData);
    }
    /** Legacy wrapper retained for test compatibility */
    lockEscrowLegacy(intentId, amount) {
        return this.api.tx.x3SettlementEngine.lockEscrow(intentId, 0, 'Legacy', BigInt(amount), []);
    }
    /** Claim a settlement by revealing the HTLC secret */
    async claimSettlement(intentId, secret) {
        return this.api.tx.x3SettlementEngine.claimSettlement(intentId, secret);
    }
    /** Refund an expired settlement */
    async refundSettlement(intentId) {
        return this.api.tx.x3SettlementEngine.refundSettlement(intentId);
    }
    /** Get settlement intent info */
    async getIntent(intentId) {
        const result = await this.api.query.x3SettlementEngine.settlementIntents(intentId);
        return result.toJSON?.() ?? null;
    }
    /** Get intent state */
    async getIntentState(intentId) {
        const result = await this.api.query.x3SettlementEngine.intentStates(intentId);
        return result.toString();
    }
    /** Deposit a bond */
    async depositBond(asset, amount, bondType) {
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
            totalIntents: total.toNumber?.() ?? 0,
            totalVolume: volume.toBigInt?.() ?? 0n,
            violations: violations.toNumber?.() ?? 0,
        };
    }
}
exports.X3SettlementClient = X3SettlementClient;
// =============================================================================
// X3 Atomic Trade Client
// =============================================================================
exports.X3VmType = {
    Evm: 'Evm',
    Svm: 'Svm',
    X3: 'X3',
    CrossVm: 'CrossVm',
};
exports.X3AmmProtocol = {
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
};
class X3AtomicTradeClient {
    api;
    constructor(api) {
        this.api = api;
    }
    /** Create a multi-leg trade batch */
    async createTradeBatch(params) {
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
        return this.api.tx.atomicTradeEngine.createTradeBatch(legs, params.slippageBps, params.deadline, params.nonce ?? 0n);
    }
    /** Legacy wrappers retained for test compatibility */
    createBatch(legs) {
        const mapped = legs.map((leg) => ({
            ammProtocol: exports.X3AmmProtocol.AtlasNative,
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
    executeBatch(batchId) {
        return this.executeTradeBatch(batchId);
    }
    cancelBatch(batchId) {
        return this.cancelTradeBatch(batchId);
    }
    /** Execute a pending trade batch */
    async executeTradeBatch(batchId) {
        return this.api.tx.atomicTradeEngine.executeTradeBatch(batchId);
    }
    /** Execute via Kernel ComitV2 (tri-VM) */
    async executeTradeBatchViaKernel(batchId, comitId) {
        return this.api.tx.atomicTradeEngine.executeTradeBatchViaKernelComitV2(batchId, comitId);
    }
    /** Cancel a pending batch */
    async cancelTradeBatch(batchId) {
        return this.api.tx.atomicTradeEngine.cancelTradeBatch(batchId);
    }
    /** Get batch info */
    async getBatch(batchId) {
        const result = await this.api.query.atomicTradeEngine.tradeBatches(batchId);
        return result.toJSON?.() ?? null;
    }
    /** Get TWAP price */
    async getTwap(tokenA, tokenB) {
        const result = await this.api.query.atomicTradeEngine.twapData([tokenA, tokenB]);
        return result.toJSON?.() ?? null;
    }
    /** Get protocol stats */
    async getStats() {
        const [completed, failed, volume] = await Promise.all([
            this.api.query.atomicTradeEngine.completedBatchCount(),
            this.api.query.atomicTradeEngine.failedBatchCount(),
            this.api.query.atomicTradeEngine.totalVolume(),
        ]);
        return {
            completed: completed.toNumber?.() ?? 0,
            failed: failed.toNumber?.() ?? 0,
            totalVolume: volume.toBigInt?.() ?? 0n,
        };
    }
}
exports.X3AtomicTradeClient = X3AtomicTradeClient;
// =============================================================================
// X3 Domain Client
// =============================================================================
class X3DomainClient {
    api;
    constructor(api) {
        this.api = api;
    }
    /** Register a .x3 domain */
    async registerDomain(domain) {
        const bytes = new TextEncoder().encode(domain.endsWith('.x3') ? domain : `${domain}.x3`);
        return this.api.tx.x3DomainRegistry.registerDomain(bytes);
    }
    register(domain) {
        return this.registerDomain(domain);
    }
    /** Set DNS records */
    async setRecords(domain, records) {
        const bytes = new TextEncoder().encode(domain.endsWith('.x3') ? domain : `${domain}.x3`);
        return this.api.tx.x3DomainRegistry.setRecords(bytes, records);
    }
    setRecordsLegacy(domain, records) {
        const mapped = records.map((record) => ({
            ttl: 0,
            data: { key: record.key, value: record.value },
        }));
        return this.setRecords(domain, mapped);
    }
    /** Get domain info */
    async getDomain(domain) {
        const bytes = new TextEncoder().encode(domain.endsWith('.x3') ? domain : `${domain}.x3`);
        const result = await this.api.query.x3DomainRegistry.domains(bytes);
        return result.toJSON?.() ?? null;
    }
    lookup(domain) {
        return this.getDomain(domain);
    }
    /** Check availability */
    async isAvailable(domain) {
        return (await this.getDomain(domain)) === null;
    }
}
exports.X3DomainClient = X3DomainClient;
// =============================================================================
// X3 Verifier Client
// =============================================================================
class X3VerifierClient {
    api;
    constructor(api) {
        this.api = api;
    }
    /** Register as executor */
    async registerExecutor(stake) {
        return this.api.tx.x3Verifier.registerExecutor(stake);
    }
    registerExecutorLegacy(_executorId, stake) {
        return this.api.tx.x3Verifier.registerExecutor(BigInt(stake));
    }
    /** Submit a verification job */
    async submitJob(bytecodeHash, inputHash, gasLimit, reward) {
        return this.api.tx.x3Verifier.submitJob(bytecodeHash, inputHash, gasLimit, reward);
    }
    submitJobLegacy(bytecodeHash, params) {
        return this.api.tx.x3Verifier.submitJob(bytecodeHash, '0x', BigInt(params.gasLimit), 0n);
    }
    /** Get job info */
    async getJob(jobId) {
        const result = await this.api.query.x3Verifier.jobs(jobId);
        return result.toJSON?.() ?? null;
    }
    getExecutor(executorId) {
        return this.api.query.x3Verifier.executors(executorId).then((result) => result.toJSON?.() ?? null);
    }
    /** Get verified state root */
    async getVerifiedStateRoot(jobId) {
        const result = await this.api.query.x3Verifier.verifiedStateRoots(jobId);
        const hex = result.toHex?.();
        return hex && hex !== '0x' + '00'.repeat(32) ? hex : null;
    }
    /** Get stats */
    async getStats() {
        const [submitted, verified] = await Promise.all([
            this.api.query.x3Verifier.totalJobsSubmitted(),
            this.api.query.x3Verifier.totalJobsVerified(),
        ]);
        return {
            submitted: submitted.toNumber?.() ?? 0,
            verified: verified.toNumber?.() ?? 0,
        };
    }
}
exports.X3VerifierClient = X3VerifierClient;
// =============================================================================
// Exports
// =============================================================================
function createX3SettlementClient(api) {
    return new X3SettlementClient(api);
}
function createX3TradeClient(api) {
    return new X3AtomicTradeClient(api);
}
function createX3DomainClient(api) {
    return new X3DomainClient(api);
}
function createX3VerifierClient(api) {
    return new X3VerifierClient(api);
}
//# sourceMappingURL=x3.js.map