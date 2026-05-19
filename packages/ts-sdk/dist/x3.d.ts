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
export interface X3SettlementOptions {
    api: ApiPromise;
}
export declare class X3SettlementClient {
    private api;
    constructor(api: ApiPromise);
    /** Create a cross-chain settlement intent */
    createIntent(params: {
        taker: string;
        assetA: {
            chain: string;
            assetId: string;
            amount: bigint;
        };
        assetB: {
            chain: string;
            assetId: string;
            amount: bigint;
        };
        secretHash: string;
        timeoutSeconds?: number;
    }): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Lock escrow for a leg of the settlement */
    lockEscrow(params: {
        intentId: string;
        legIndex: number;
        chain: string;
        amount: bigint;
        escrowData: Uint8Array | string;
    }): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Legacy wrapper retained for test compatibility */
    lockEscrowLegacy(intentId: string, amount: number): import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>;
    /** Claim a settlement by revealing the HTLC secret */
    claimSettlement(intentId: string, secret: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Refund an expired settlement */
    refundSettlement(intentId: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Get settlement intent info */
    getIntent(intentId: string): Promise<any>;
    /** Get intent state */
    getIntentState(intentId: string): Promise<string>;
    /** Deposit a bond */
    depositBond(asset: string, amount: bigint, bondType: number): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Get total settlement stats */
    getStats(): Promise<{
        totalIntents: any;
        totalVolume: any;
        violations: any;
    }>;
}
export declare const X3VmType: {
    readonly Evm: "Evm";
    readonly Svm: "Svm";
    readonly X3: "X3";
    readonly CrossVm: "CrossVm";
};
export type X3VmType = (typeof X3VmType)[keyof typeof X3VmType];
export declare const X3AmmProtocol: {
    readonly UniswapV2: "UniswapV2";
    readonly UniswapV3: "UniswapV3";
    readonly Raydium: "Raydium";
    readonly Orca: "Orca";
    readonly Jupiter: "Jupiter";
    readonly SushiSwap: "SushiSwap";
    readonly PancakeSwap: "PancakeSwap";
    readonly Curve: "Curve";
    readonly Balancer: "Balancer";
    readonly AtlasNative: "AtlasNative";
};
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
export declare class X3AtomicTradeClient {
    private api;
    constructor(api: ApiPromise);
    /** Create a multi-leg trade batch */
    createTradeBatch(params: {
        legs: X3TradeLeg[];
        slippageBps: number;
        deadline: number;
        nonce?: bigint;
    }): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Legacy wrappers retained for test compatibility */
    createBatch(legs: Array<{
        fromVm: X3VmType;
        toVm: X3VmType;
        fromAsset: string;
        toAsset: string;
        amount: number;
        minOut: number;
    }>): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    executeBatch(batchId: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    cancelBatch(batchId: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Execute a pending trade batch */
    executeTradeBatch(batchId: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Execute via Kernel ComitV2 (tri-VM) */
    executeTradeBatchViaKernel(batchId: string, comitId: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Cancel a pending batch */
    cancelTradeBatch(batchId: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Get batch info */
    getBatch(batchId: string): Promise<any>;
    /** Get TWAP price */
    getTwap(tokenA: string, tokenB: string): Promise<any>;
    /** Get protocol stats */
    getStats(): Promise<{
        completed: any;
        failed: any;
        totalVolume: any;
    }>;
}
export declare class X3DomainClient {
    private api;
    constructor(api: ApiPromise);
    /** Register a .x3 domain */
    registerDomain(domain: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    register(domain: string): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Set DNS records */
    setRecords(domain: string, records: Array<{
        ttl: number;
        data: unknown;
    }>): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    setRecordsLegacy(domain: string, records: Array<{
        key: string;
        value: unknown;
    }>): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    /** Get domain info */
    getDomain(domain: string): Promise<any>;
    lookup(domain: string): Promise<any>;
    /** Check availability */
    isAvailable(domain: string): Promise<boolean>;
}
export declare class X3VerifierClient {
    private api;
    constructor(api: ApiPromise);
    /** Register as executor */
    registerExecutor(stake: bigint): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    registerExecutorLegacy(_executorId: string, stake: number): import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>;
    /** Submit a verification job */
    submitJob(bytecodeHash: string, inputHash: string, gasLimit: bigint, reward: bigint): Promise<import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>>;
    submitJobLegacy(bytecodeHash: string, params: {
        gasLimit: number;
    }): import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>;
    /** Get job info */
    getJob(jobId: string): Promise<any>;
    getExecutor(executorId: string): Promise<any>;
    /** Get verified state root */
    getVerifiedStateRoot(jobId: string): Promise<string | null>;
    /** Get stats */
    getStats(): Promise<{
        submitted: any;
        verified: any;
    }>;
}
export declare function createX3SettlementClient(api: ApiPromise): X3SettlementClient;
export declare function createX3TradeClient(api: ApiPromise): X3AtomicTradeClient;
export declare function createX3DomainClient(api: ApiPromise): X3DomainClient;
export declare function createX3VerifierClient(api: ApiPromise): X3VerifierClient;
//# sourceMappingURL=x3.d.ts.map