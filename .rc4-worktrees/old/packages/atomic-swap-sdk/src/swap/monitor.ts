/**
 * Swap Monitor — watches swap progress across chains.
 *
 * Provides real-time status updates for active atomic swaps
 * by polling HTLC state on source and destination chains.
 */

import { EventEmitter } from "eventemitter3";
import type { AtomicSwap, SwapStatus, ChainId } from "../types";
import { type IHTLCAdapter, createHTLCAdapter } from "../htlc";

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

export class SwapMonitor extends EventEmitter<MonitorEvents> {
  private config: SwapMonitorConfig;
  private adapters: Map<string, IHTLCAdapter> = new Map();
  private intervals: Map<string, ReturnType<typeof setInterval>> = new Map();
  private watchedSwaps: Map<string, AtomicSwap> = new Map();

  /** Warn when less than 10 minutes remain on HTLC timelock */
  private warningThresholdMs = 10 * 60 * 1000;

  constructor(config: SwapMonitorConfig) {
    super();
    this.config = config;
  }

  /**
   * Start monitoring a swap.
   */
  watch(swap: AtomicSwap): void {
    if (this.intervals.has(swap.id)) return;
    this.watchedSwaps.set(swap.id, swap);

    const interval = setInterval(async () => {
      await this.poll(swap.id);
    }, this.config.pollInterval || 10000);

    this.intervals.set(swap.id, interval);

    // Initial poll
    this.poll(swap.id).catch(() => {});
  }

  /**
   * Stop monitoring a swap.
   */
  unwatch(swapId: string): void {
    const interval = this.intervals.get(swapId);
    if (interval) {
      clearInterval(interval);
      this.intervals.delete(swapId);
    }
    this.watchedSwaps.delete(swapId);
  }

  /**
   * Get health report for a swap.
   */
  async getHealthReport(swapId: string): Promise<SwapHealthReport> {
    const swap = this.watchedSwaps.get(swapId);
    if (!swap) throw new Error(`Swap ${swapId} is not being monitored`);

    const now = Date.now();
    const report: SwapHealthReport = {
      swapId,
      sourceHtlcStatus: "unknown",
      destHtlcStatus: "none",
      sourceTimeRemaining: 0,
      destTimeRemaining: 0,
      secretRevealed: false,
      health: "healthy",
    };

    // Check source HTLC
    if (swap.sourceHtlc) {
      const adapter = this.getAdapter(swap.sourceChain);
      try {
        const htlc = await adapter.getHTLC(swap.sourceHtlc.id);
        if (htlc) {
          report.sourceHtlcStatus = htlc.status;
          report.sourceTimeRemaining = Math.max(0, htlc.timeLock * 1000 - now);
        }
      } catch {
        report.sourceHtlcStatus = "unknown";
      }
    }

    // Check dest HTLC
    if (swap.destHtlc) {
      const adapter = this.getAdapter(swap.destChain);
      try {
        const htlc = await adapter.getHTLC(swap.destHtlc.id);
        if (htlc) {
          report.destHtlcStatus = htlc.status;
          report.destTimeRemaining = Math.max(0, htlc.timeLock * 1000 - now);
        }

        const { claimed, secret } = await adapter.isHTLCClaimed(swap.destHtlc.id);
        if (claimed && secret) {
          report.secretRevealed = true;
        }
      } catch {
        report.destHtlcStatus = "unknown";
      }
    }

    // Determine health
    if (report.sourceTimeRemaining === 0 && report.sourceHtlcStatus !== "claimed") {
      report.health = "expired";
    } else if (
      report.sourceTimeRemaining < this.warningThresholdMs ||
      report.destTimeRemaining < this.warningThresholdMs
    ) {
      report.health = report.sourceTimeRemaining < 5 * 60 * 1000 ? "critical" : "warning";
    } else {
      report.health = "healthy";
    }

    return report;
  }

  /**
   * Stop all monitoring.
   */
  destroy(): void {
    for (const [id] of this.intervals) {
      this.unwatch(id);
    }
    this.removeAllListeners();
  }

  // ─── Internal ─────────────────────────────────────────────

  private async poll(swapId: string): Promise<void> {
    try {
      const report = await this.getHealthReport(swapId);
      this.emit("health-update", report);

      if (report.secretRevealed) {
        const swap = this.watchedSwaps.get(swapId);
        if (swap?.secret) {
          this.emit("secret-revealed", swapId, swap.secret);
        }
      }

      if (report.health === "warning" || report.health === "critical") {
        const minRemaining = Math.min(
          report.sourceTimeRemaining || Infinity,
          report.destTimeRemaining || Infinity,
        );
        this.emit("expiry-warning", swapId, minRemaining);
      }
    } catch (err: any) {
      this.emit("monitor-error", swapId, err.message || "Poll failed");
    }
  }

  private getAdapter(chainId: ChainId): IHTLCAdapter {
    let adapter = this.adapters.get(chainId);
    if (!adapter) {
      adapter = createHTLCAdapter({
        chainId,
        rpcEndpoint: this.config.endpoints[chainId] || "",
        htlcContractAddress: this.config.htlcContracts[chainId],
      });
      this.adapters.set(chainId, adapter);
    }
    return adapter;
  }
}
