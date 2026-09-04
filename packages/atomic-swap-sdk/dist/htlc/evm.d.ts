/**
 * EVM HTLC Adapter — Creates and manages Hash Time-Locked Contracts on EVM chains.
 *
 * Interacts with the AtlasHTLC.sol smart contract deployed on Ethereum, Polygon,
 * BSC, Arbitrum, Optimism, Base, etc.
 *
 * ABI: createHTLC(bytes32 hashLock, address recipient, address token, uint256 amount, uint256 timelock)
 *      claimHTLC(bytes32 htlcId, bytes32 secret)
 *      refundHTLC(bytes32 htlcId)
 */
import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
import { type IHTLCAdapter } from "./base";
export declare class EvmHTLCAdapter implements IHTLCAdapter {
    readonly chainId: ChainId;
    private rpcEndpoint;
    private htlcContractAddress;
    constructor(chainId: ChainId, rpcEndpoint: string, htlcContractAddress: string);
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
    private encodeCreateHTLC;
    private encodeClaimHTLC;
    private encodeRefundHTLC;
    private computeHTLCId;
    private padBytes32;
    private padAddress;
    private padUint256;
    private isNativeToken;
    private addressFromKey;
    private ethCall;
    private sendTransaction;
    private getTransactionCount;
    private getGasPrice;
}
/**
 * Factory function to create an EVM HTLC adapter with env var configuration.
 * Reads X3_EVM_HTLC_CONTRACT from environment.
 */
export declare function createEvmHTLCAdapter(chainId: ChainId, rpcEndpoint: string): EvmHTLCAdapter;
//# sourceMappingURL=evm.d.ts.map