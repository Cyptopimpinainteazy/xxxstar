/**
 * Swap Monitor — watches swap progress across chains.
 *
 * Provides real-time status updates for active atomic swaps
 * by polling HTLC state on source and destination chains.
 */
import { EventEmitter } from "eventemitter3";
import type { AtomicSwap } from "../types";
export interface SwapMonitorConfig {
    /** Polling interval in ms (default: 10000) */
    pollInterval?: number;
    /** Chain RPC endpoints */
    endpoints: Partial<Record<string, string>>;
    /** HTLC contract addresses per chain */
    htlcContracts: Partial<Record<string, string>>;
}
export interface SwapHealthReport {
    swapId: string;
    sourceHtlcStatus: "pending" | "funded" | "claimed" | "refunded" | "expired" | "unknown";
    destHtlcStatus: "pending" | "funded" | "claimed" | "refunded" | "expired" | "unknown" | "none";
    sourceTimeRemaining: number;
    destTimeRemaining: number;
    secretRevealed: boolean;
    health: "healthy" | "warning" | "critical" | "expired";
}
type MonitorEvents = {
    "health-update": (report: SwapHealthReport) => void;
    "secret-revealed": (swapId: string, secret: string) => void;
    "expiry-warning": (swapId: string, remainingMs: number) => void;
    "monitor-error": (swapId: string, error: string) => void;
};
export declare class SwapMonitor extends EventEmitter<MonitorEvents> {
    private config;
    private adapters;
    private intervals;
    private watchedSwaps;
    /** Warn when less than 10 minutes remain on HTLC timelock */
    private warningThresholdMs;
    constructor(config: SwapMonitorConfig);
    /**
     * Start monitoring a swap.
     */
    watch(swap: AtomicSwap): void;
    /**
     * Stop monitoring a swap.
     */
    unwatch(swapId: string): void;
    /**
     * Get health report for a swap.
     */
    getHealthReport(swapId: string): Promise<SwapHealthReport>;
    /**
     * Stop all monitoring.
     */
    destroy(): void;
    private poll;
    private getAdapter;
}
export {};
//# sourceMappingURL=monitor.d.ts.map