"use strict";
/**
 * Swap Monitor — watches swap progress across chains.
 *
 * Provides real-time status updates for active atomic swaps
 * by polling HTLC state on source and destination chains.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SwapMonitor = void 0;
const eventemitter3_1 = require("eventemitter3");
const htlc_1 = require("../htlc");
class SwapMonitor extends eventemitter3_1.EventEmitter {
    config;
    adapters = new Map();
    intervals = new Map();
    watchedSwaps = new Map();
    /** Warn when less than 10 minutes remain on HTLC timelock */
    warningThresholdMs = 10 * 60 * 1000;
    constructor(config) {
        super();
        this.config = config;
    }
    /**
     * Start monitoring a swap.
     */
    watch(swap) {
        if (this.intervals.has(swap.id))
            return;
        this.watchedSwaps.set(swap.id, swap);
        const interval = setInterval(async () => {
            await this.poll(swap.id);
        }, this.config.pollInterval || 10000);
        this.intervals.set(swap.id, interval);
        // Initial poll
        this.poll(swap.id).catch(() => { });
    }
    /**
     * Stop monitoring a swap.
     */
    unwatch(swapId) {
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
    async getHealthReport(swapId) {
        const swap = this.watchedSwaps.get(swapId);
        if (!swap)
            throw new Error(`Swap ${swapId} is not being monitored`);
        const now = Date.now();
        const report = {
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
            }
            catch {
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
            }
            catch {
                report.destHtlcStatus = "unknown";
            }
        }
        // Determine health
        if (report.sourceTimeRemaining === 0 && report.sourceHtlcStatus !== "claimed") {
            report.health = "expired";
        }
        else if (report.sourceTimeRemaining < this.warningThresholdMs ||
            report.destTimeRemaining < this.warningThresholdMs) {
            report.health = report.sourceTimeRemaining < 5 * 60 * 1000 ? "critical" : "warning";
        }
        else {
            report.health = "healthy";
        }
        return report;
    }
    /**
     * Stop all monitoring.
     */
    destroy() {
        for (const [id] of this.intervals) {
            this.unwatch(id);
        }
        this.removeAllListeners();
    }
    // ─── Internal ─────────────────────────────────────────────
    async poll(swapId) {
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
                const minRemaining = Math.min(report.sourceTimeRemaining || Infinity, report.destTimeRemaining || Infinity);
                this.emit("expiry-warning", swapId, minRemaining);
            }
        }
        catch (err) {
            this.emit("monitor-error", swapId, err.message || "Poll failed");
        }
    }
    getAdapter(chainId) {
        let adapter = this.adapters.get(chainId);
        if (!adapter) {
            adapter = (0, htlc_1.createHTLCAdapter)({
                chainId,
                rpcEndpoint: this.config.endpoints[chainId] || "",
                htlcContractAddress: this.config.htlcContracts[chainId],
            });
            this.adapters.set(chainId, adapter);
        }
        return adapter;
    }
}
exports.SwapMonitor = SwapMonitor;
//# sourceMappingURL=monitor.js.map