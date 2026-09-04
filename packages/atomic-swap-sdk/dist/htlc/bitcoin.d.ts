/**
 * Bitcoin HTLC Adapter — Creates and manages HTLCs using Bitcoin Script.
 *
 * Uses P2SH or P2WSH scripts with:
 *   OP_IF
 *     OP_SHA256 <hashLock> OP_EQUALVERIFY <recipientPubKey> OP_CHECKSIG
 *   OP_ELSE
 *     <timeLock> OP_CHECKLOCKTIMEVERIFY OP_DROP <senderPubKey> OP_CHECKSIG
 *   OP_ENDIF
 *
 * Uses Blockstream/Esplora REST API for querying.
 */
import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
import { type IHTLCAdapter } from "./base";
export declare class BitcoinHTLCAdapter implements IHTLCAdapter {
    readonly chainId: ChainId;
    private apiEndpoint;
    private network;
    private htlcCache;
    constructor(chainId: ChainId, apiEndpoint: string, network?: "mainnet" | "testnet" | "signet");
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
    /**
     * Build an HTLC redeem script.
     *
     * OP_IF
     *   OP_SHA256 <hashLock> OP_EQUALVERIFY <recipientPubKey> OP_CHECKSIG
     * OP_ELSE
     *   <timeLock> OP_CHECKLOCKTIMEVERIFY OP_DROP <senderPubKey> OP_CHECKSIG
     * OP_ENDIF
     */
    private buildRedeemScript;
    private encodeScriptNumber;
    private scriptHashToAddress;
    private fetchJson;
    private fetchText;
    private getBitcoinNetwork;
    private addressFromPrivateKey;
    private fundHtlc;
    private spendHtlc;
    private witnessStackToScriptWitness;
    private broadcastRawTransaction;
}
//# sourceMappingURL=bitcoin.d.ts.map