/**
 * Health Monitor — lightweight endpoint probing and status tracking.
 */

import { EventEmitter } from "events";
import client from "prom-client";

export type EndpointStatus = {
  endpoint: string;
  healthy: boolean;
  lastChecked: number | null;
  lastError?: string;
};

export class HealthMonitor extends EventEmitter {
  private statuses = new Map<string, EndpointStatus>();
  private intervalId: NodeJS.Timeout | null = null;
  private concurrency: number;
  private timeoutMs: number;
  private intervalMs: number;

  // Prometheus metrics
  private gaugeHealthy?: client.Gauge<string>;
  private counterStateChanges?: client.Counter<string>;

  constructor({ concurrency = 50, timeoutMs = 10000, intervalMs = 60_000 } = {}) {
    super();
    this.concurrency = concurrency;
    this.timeoutMs = timeoutMs;
    this.intervalMs = intervalMs;

    // setup default metrics
    try {
      this.gaugeHealthy = new client.Gauge({ name: "endpoint_healthy_total", help: "Number of healthy endpoints currently known" });
      this.counterStateChanges = new client.Counter({ name: "endpoint_state_changes_total", help: "Total endpoint state changes (healthy<->unhealthy)" });
    } catch (e) {
      // Prom-client may already have metrics registered in tests; ignore registration errors
    }
  }

  getStatus(endpoint: string): EndpointStatus | undefined {
    return this.statuses.get(endpoint);
  }

  getHealthyEndpoint(endpoints: string[]): string | null {
    // Prefer endpoints that were recently checked and are healthy; fall back to any healthy otherwise
    const candidates = endpoints
      .map((e) => this.statuses.get(e))
      .filter((s): s is EndpointStatus => !!s && s.lastChecked !== null)
      .sort((a, b) => Number(b.healthy) - Number(a.healthy) || Number((b.lastChecked || 0) - (a.lastChecked || 0)));

    const healthy = candidates.find((c) => c.healthy);
    if (healthy) return healthy.endpoint;

    // If none were previously checked or healthy, try a quick probe for each and return the first healthy
    return null;
  }

  private recordStatusChange(prev: EndpointStatus | undefined, next: EndpointStatus) {
    if (!prev) return;
    if (prev.healthy !== next.healthy) {
      this.counterStateChanges?.inc();
      this.emit("status-change", { endpoint: next.endpoint, healthy: next.healthy, previous: prev.healthy });
    }
  }

  async probeEndpoint(endpoint: string): Promise<EndpointStatus> {
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
        } else {
          // Non-JSON response but HTTP 200
          healthy = true;
        }
      } else {
        lastError = `HTTP ${res.status}`;
      }
    } catch (err: any) {
      lastError = err?.message || String(err);
    } finally {
      clearTimeout(timeout);
      const prev = this.statuses.get(endpoint);
      const status: EndpointStatus = { endpoint, healthy, lastChecked: Date.now(), lastError };
      this.statuses.set(endpoint, status);
      // update gauge
      const healthyCount = Array.from(this.statuses.values()).filter((s) => s.healthy).length;
      try { this.gaugeHealthy?.set(healthyCount); } catch (e) {}
      this.recordStatusChange(prev, status);
      return status;
    }
  }

  async probeEndpoints(endpoints: string[], concurrency = this.concurrency): Promise<EndpointStatus[]> {
    const results: EndpointStatus[] = [];
    const pool: Promise<void>[] = [];
    let i = 0;

    const worker = async () => {
      while (i < endpoints.length) {
        const idx = i++;
        const ep = endpoints[idx];
        try {
          const st = await this.probeEndpoint(ep);
          results[idx] = st;
        } catch (err: any) {
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
    try { this.gaugeHealthy?.set(healthyCount); } catch (e) {}
    return results;
  }

  startPeriodic(endpoints: string[]) {
    if (this.intervalId) return;
    // Seed statuses
    this.probeEndpoints(endpoints).catch(() => {});
    this.intervalId = setInterval(async () => {
      await this.probeEndpoints(endpoints);
    }, this.intervalMs);
  }

  stop() {
    if (this.intervalId) clearInterval(this.intervalId as any);
    this.intervalId = null;
  }
}
