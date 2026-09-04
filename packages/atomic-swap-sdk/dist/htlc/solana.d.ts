/**
 * Solana HTLC Adapter — Creates and manages HTLCs on Solana via a deployed Anchor program.
 *
 * The HTLC program stores state in a PDA derived from [b"htlc", hashLock].
 * Instructions: initialize, claim, refund, get_htlc.
 */
import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
import { type IHTLCAdapter } from "./base";
export declare class SolanaHTLCAdapter implements IHTLCAdapter {
    readonly chainId: ChainId;
    private rpcEndpoint;
    private programId;
    constructor(chainId: ChainId, rpcEndpoint: string, programId: string);
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
    private deriveHTLCPda;
    private encodeLittleEndianU64;
    private decodeLittleEndianU64;
    private bs58Encode;
    private getAccountInfo;
    private sendSolanaTransaction;
}
/**
 * Factory function to create a Solana HTLC adapter with env var configuration.
 * Reads X3_SOLANA_HTLC_PROGRAM_ID from environment.
 */
export declare function createSolanaHTLCAdapter(chainId: ChainId, rpcEndpoint: string): SolanaHTLCAdapter;
//# sourceMappingURL=solana.d.ts.map