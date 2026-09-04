/**
 * HTLC Base — Abstract interface for Hash Time-Locked Contracts across all chains.
 */
import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
export interface IHTLCAdapter {
    /** Chain this adapter handles */
    readonly chainId: ChainId;
    /**
     * Deploy / create an HTLC with the given parameters.
     * Returns an HTLC descriptor with funding tx hash.
     */
    createHTLC(params: HTLCCreateParams, signerKey: string): Promise<HTLC>;
    /**
     * Claim an HTLC by revealing the secret preimage.
     * Returns updated HTLC with "claimed" status.
     */
    claimHTLC(params: HTLCClaimParams, signerKey: string): Promise<HTLC>;
    /**
     * Refund an expired HTLC back to sender.
     * Returns updated HTLC with "refunded" status.
     */
    refundHTLC(params: HTLCRefundParams, signerKey: string): Promise<HTLC>;
    /**
     * Query on-chain state of an HTLC.
     */
    getHTLC(htlcId: string): Promise<HTLC | null>;
    /**
     * Check if an HTLC has been funded on-chain.
     */
    isHTLCFunded(htlcId: string): Promise<boolean>;
    /**
     * Check if an HTLC has been claimed (secret revealed).
     */
    isHTLCClaimed(htlcId: string): Promise<{
        claimed: boolean;
        secret?: string;
    }>;
    /**
     * Check if an HTLC is expired / refundable.
     */
    isHTLCExpired(htlcId: string): Promise<boolean>;
}
/**
 * Generate a cryptographically secure random secret for HTLC.
 * Returns { secret, hashLock } where hashLock = SHA-256(secret).
 */
export declare function generateSecret(): {
    secret: string;
    hashLock: string;
};
/**
 * Compute SHA-256 hash of hex-encoded data.
 */
export declare function sha256Hex(data: Uint8Array): string;
/**
 * Compute SHA-256 from hex string.
 */
export declare function sha256FromHex(hexStr: string): string;
export declare function bytesToHex(bytes: Uint8Array): string;
export declare function hexToBytes(hex: string): Uint8Array;
/**
 * Calculate a safe time lock:
 * - Initiator gets a longer timelock (e.g., 2x)
 * - Counterparty gets a shorter timelock
 * This ensures the initiator can always claim before refunding.
 */
export declare function calculateTimeLocks(baseDurationSeconds: number): {
    initiatorTimeLock: number;
    counterpartyTimeLock: number;
};
//# sourceMappingURL=base.d.ts.map