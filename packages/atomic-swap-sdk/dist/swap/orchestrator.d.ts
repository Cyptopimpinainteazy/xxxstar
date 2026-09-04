/**
 * Atomic Swap Orchestrator
 *
 * Coordinates the full lifecycle of a cross-chain atomic swap:
 *
 * 1. Initiator generates secret + hashLock
 * 2. Initiator creates HTLC on source chain (longer timelock)
 * 3. Counterparty verifies source HTLC, creates HTLC on dest chain (shorter timelock)
 * 4. Initiator claims dest HTLC (reveals secret)
 * 5. Counterparty extracts secret from dest chain, claims source HTLC
 *
 * Supports: EVM ↔ EVM, EVM ↔ Solana, EVM ↔ Bitcoin, EVM ↔ Substrate,
 *           Solana ↔ Bitcoin, Substrate ↔ any
 */
import { EventEmitter } from "eventemitter3";
import type { AtomicSwap, SwapInitParams, SwapStatus, DexConfig } from "../types";
type SwapEvents = {
    "swap-initiated": (swap: AtomicSwap) => void;
    "swap-source-funded": (swap: AtomicSwap) => void;
    "swap-dest-funded": (swap: AtomicSwap) => void;
    "swap-claimed": (swap: AtomicSwap) => void;
    "swap-refunded": (swap: AtomicSwap) => void;
    "swap-expired": (swap: AtomicSwap) => void;
    "swap-failed": (swap: AtomicSwap, error: string) => void;
    "swap-status-change": (swap: AtomicSwap, oldStatus: SwapStatus, newStatus: SwapStatus) => void;
};
export declare class SwapOrchestrator extends EventEmitter<SwapEvents> {
    private config;
    private adapters;
    private swaps;
    private monitorIntervals;
    /** Monitor polling interval in ms */
    private pollIntervalMs;
    constructor(config: DexConfig, pollIntervalMs?: number);
    /**
     * Initialize a new atomic swap as the initiator.
     *
     * Steps:
     * 1. Generate secret + hashLock
     * 2. Create HTLC on source chain
     * 3. Return swap object for counterparty to verify
     */
    initiateSwap(params: SwapInitParams, signerKey: string): Promise<AtomicSwap>;
    /**
     * As counterparty, respond to a swap by creating HTLC on destination chain.
     *
     * The counterparty must verify the source HTLC first, then lock their tokens
     * on the destination chain with the same hashLock but a shorter timelock.
     */
    respondToSwap(swapId: string, destAmount: string, signerKey: string): Promise<AtomicSwap>;
    /**
     * As initiator, claim the destination HTLC (reveals secret).
     * After this, the counterparty can extract the secret and claim the source HTLC.
     */
    claimSwap(swapId: string, signerKey: string): Promise<AtomicSwap>;
    /**
     * As counterparty, extract the revealed secret from the destination chain
     * and claim the source HTLC.
     */
    counterpartyClaim(swapId: string, signerKey: string): Promise<AtomicSwap>;
    /**
     * Refund an expired swap (source HTLC).
     */
    refundSwap(swapId: string, signerKey: string): Promise<AtomicSwap>;
    /**
     * Get swap by ID.
     */
    getSwap(swapId: string): AtomicSwap | undefined;
    /**
     * List all swaps.
     */
    listSwaps(filter?: {
        status?: SwapStatus;
    }): AtomicSwap[];
    /**
     * Stop all monitoring and clean up.
     */
    destroy(): void;
    private getOrCreateAdapter;
    private startMonitoring;
    private stopMonitoring;
    private checkSwapProgress;
    private updateSwapStatus;
}
export {};
//# sourceMappingURL=orchestrator.d.ts.map