/**
 * Substrate HTLC Adapter — Manages HTLCs via the atomic-trade-engine pallet on X3 Chain.
 *
 * Uses Substrate RPC to interact with the pallet extrinsics:
 * - atomicTradeEngine.createTradeBatch
 * - atomicTradeEngine.executeTradeBatch
 * - atomicTradeEngine.cancelTradeBatch
 *
 * For direct HTLC-like behavior, we use single-leg trade batches
 * with the x3-amm protocol.
 */
import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
import { type IHTLCAdapter } from "./base";
export declare class SubstrateHTLCAdapter implements IHTLCAdapter {
    readonly chainId: ChainId;
    private wsEndpoint;
    private rpcEndpoint;
    constructor(chainId: ChainId, rpcEndpoint: string, wsEndpoint?: string);
    createHTLC(params: HTLCCreateParams, signerKey: string): Promise<HTLC>;
    claimHTLC(params: HTLCClaimParams, signerKey: string): Promise<HTLC>;
    refundHTLC(params: HTLCRefundParams, signerKey: string): Promise<HTLC>;
    getHTLC(htlcId: string): Promise<HTLC | null>;
    isHTLCFunded(htlcId: string): Promise<boolean>;
    isHTLCClaimed(htlcId: string): Promise<{
        claimed: boolean;
        secret?: string;
    }>;
    isHTLCExpired(htlcId: string): Promise<boolean>;
    private rpcCall;
    private queryStorage;
    private buildExtrinsic;
    private submitExtrinsic;
    private getTradeNonce;
    private getCurrentBlockNumber;
    private encodeHTLCRouteData;
}
/**
 * Factory function to create a Substrate HTLC adapter with env var configuration.
 * Reads X3_RPC_ENDPOINT for HTTP endpoint and X3_WS_ENDPOINT for WebSocket endpoint.
 */
export declare function createSubstrateHTLCAdapter(chainId: ChainId): SubstrateHTLCAdapter;
//# sourceMappingURL=substrate.d.ts.map