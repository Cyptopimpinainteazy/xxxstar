/**
 * Health Monitor — lightweight endpoint probing and status tracking.
 */
import { EventEmitter } from "events";
import client from "prom-client";
export class HealthMonitor extends EventEmitter {
    statuses = new Map();
    intervalId = null;
    concurrency;
    timeoutMs;
    intervalMs;
    // Prometheus metrics
    gaugeHealthy;
    counterStateChanges;
    constructor({ concurrency = 50, timeoutMs = 10000, intervalMs = 60_000 } = {}) {
        super();
        this.concurrency = concurrency;
        this.timeoutMs = timeoutMs;
        this.intervalMs = intervalMs;
        // setup default metrics
        try {
            this.gaugeHealthy = new client.Gauge({ name: "endpoint_healthy_total", help: "Number of healthy endpoints currently known" });
            this.counterStateChanges = new client.Counter({ name: "endpoint_state_changes_total", help: "Total endpoint state changes (healthy<->unhealthy)" });
        }
        catch (e) {
            // Prom-client may already have metrics registered in tests; ignore registration errors
        }
    }
    getStatus(endpoint) {
        return this.statuses.get(endpoint);
    }
    getHealthyEndpoint(endpoints) {
        // Prefer endpoints that were recently checked and are healthy; fall back to any healthy otherwise
        const candidates = endpoints
            .map((e) => this.statuses.get(e))
            .filter((s) => !!s && s.lastChecked !== null)
            .sort((a, b) => Number(b.healthy) - Number(a.healthy) || Number((b.lastChecked || 0) - (a.lastChecked || 0)));
        const healthy = candidates.find((c) => c.healthy);
        if (healthy)
            return healthy.endpoint;
        // If none were previously checked or healthy, try a quick probe for each and return the first healthy
        return null;
    }
    recordStatusChange(prev, next) {
        if (!prev)
            return;
        if (prev.healthy !== next.healthy) {
            this.counterStateChanges?.inc();
            this.emit("status-change", { endpoint: next.endpoint, healthy: next.healthy, previous: prev.healthy });
        }
    }
    async probeEndpoint(endpoint) {
        const controller = new AbortController();
        const signal = controller.signal;
        const timeout = setTimeout(() => controller.abort(), this.timeoutMs);
        let healthy = false;
        let lastError;
        try {
            // Try JSON-RPC POST (eth_blockNumber) first
            const body = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "eth_blockNumber", params: [] });
            const res = await fetch(endpoint, { method: "POST", headers: { "Content-Type": "application/json" }, body, signal });
            const contentType = res.headers.get("content-type") || "";
            if (res.ok) {
                if (contentType.includes("application/json")) {
                    const json = await res.json();
                    if (json && (json.result || typeof json.result !== "undefined")) {
                        healthy = true;
                    }
                }
                else {
                    // Non-JSON response but HTTP 200
                    healthy = true;
                }
            }
            else {
                lastError = `HTTP ${res.status}`;
            }
        }
        catch (err) {
            lastError = err?.message || String(err);
        }
        finally {
            clearTimeout(timeout);
            const prev = this.statuses.get(endpoint);
            const status = { endpoint, healthy, lastChecked: Date.now(), lastError };
            this.statuses.set(endpoint, status);
            // update gauge
            const healthyCount = Array.from(this.statuses.values()).filter((s) => s.healthy).length;
            try {
                this.gaugeHealthy?.set(healthyCount);
            }
            catch (e) { }
            this.recordStatusChange(prev, status);
            return status;
        }
    }
    async probeEndpoints(endpoints, concurrency = this.concurrency) {
        const results = [];
        const pool = [];
        let i = 0;
        const worker = async () => {
            while (i < endpoints.length) {
                const idx = i++;
                const ep = endpoints[idx];
                try {
                    const st = await this.probeEndpoint(ep);
                    results[idx] = st;
                }
                catch (err) {
                    const st = { endpoint: ep, healthy: false, lastChecked: Date.now(), lastError: err?.message };
                    results[idx] = st;
                    const prev = this.statuses.get(ep);
                    this.statuses.set(ep, st);
                    this.recordStatusChange(prev, st);
                }
            }
        };
        for (let w = 0; w < Math.min(concurrency, endpoints.length); w++) {
            pool.push(worker());
        }
        await Promise.all(pool);
        // update gauge
        const healthyCount = Array.from(this.statuses.values()).filter((s) => s.healthy).length;
        try {
            this.gaugeHealthy?.set(healthyCount);
        }
        catch (e) { }
        return results;
    }
    startPeriodic(endpoints) {
        if (this.intervalId)
            return;
        // Seed statuses
        this.probeEndpoints(endpoints).catch(() => { });
        this.intervalId = setInterval(async () => {
            await this.probeEndpoints(endpoints);
        }, this.intervalMs);
    }
    stop() {
        if (this.intervalId)
            clearInterval(this.intervalId);
        this.intervalId = null;
    }
}
//# sourceMappingURL=health-monitor.js.map