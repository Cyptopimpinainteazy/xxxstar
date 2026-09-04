import fetch from 'node-fetch';
import type { HealthMonitor, EndpointStatus } from './health-monitor';

export interface AlerterOptions {
  slackWebhook?: string;
  webhookUrl?: string; // generic webhook
  wideFailureThresholdPercent?: number; // 0-100
  wideFailureWindowSec?: number;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class Alerter {
  private monitor: HealthMonitor;
  private opts: AlerterOptions;
  private lastWideAlertAt?: number;

  constructor(monitor: HealthMonitor, opts: AlerterOptions = {}) {
    this.monitor = monitor;
    this.opts = Object.assign({ wideFailureThresholdPercent: 50, wideFailureWindowSec: 300 }, opts);

    this.monitor.on('status-change', (ev: any) => this.onStatusChange(ev));

    // periodic check for wide failures
    setInterval(() => this.checkWideFailures(), (this.opts.wideFailureWindowSec || 300) * 1000);
  }

  private async onStatusChange(ev: { endpoint: string; healthy: boolean; previous: boolean }) {
    const msg = `Endpoint ${ev.endpoint} changed: healthy=${ev.healthy} (was=${ev.previous})`;
    console.warn('ALERT:', msg);
    if (this.opts.slackWebhook) await this.postSlack(msg);
    if (this.opts.webhookUrl) await this.postWebhook({ type: 'status-change', endpoint: ev.endpoint, healthy: ev.healthy });
  }

  private async postSlack(text: string) {
    if (!this.opts.slackWebhook) return;
    try {
      await fetch(this.opts.slackWebhook, { method: 'POST', body: JSON.stringify({ text }), headers: { 'Content-Type': 'application/json' } });
    } catch (error) {
      console.warn('Alerter: slack post failed', getErrorMessage(error));
    }
  }

  private async postWebhook(payload: any) {
    if (!this.opts.webhookUrl) return;
    try {
      await fetch(this.opts.webhookUrl, { method: 'POST', body: JSON.stringify(payload), headers: { 'Content-Type': 'application/json' } });
    } catch (error) {
      console.warn('Alerter: webhook post failed', getErrorMessage(error));
    }
  }

  async checkWideFailures() {
    // compute percent healthy across all monitored endpoints
    const statuses: EndpointStatus[] = [];
    // HealthMonitor doesn't currently expose a full list; use internal map via any
    const map = (this.monitor as any).statuses as Map<string, EndpointStatus> | undefined;
    if (!map) return;
    for (const v of map.values()) statuses.push(v);
    if (statuses.length === 0) return;

    const healthyCount = statuses.filter(s => s.healthy).length;
    const percent = (healthyCount / statuses.length) * 100;
    if (percent < (this.opts.wideFailureThresholdPercent || 50)) {
      const now = Date.now();
      if (!this.lastWideAlertAt || (now - this.lastWideAlertAt) > (this.opts.wideFailureWindowSec || 300) * 1000) {
        const msg = `Wide failure: only ${healthyCount}/${statuses.length} endpoints healthy (${percent.toFixed(1)}%)`;
        console.warn('ALERT:', msg);
        await this.postSlack(msg);
        await this.postWebhook({ type: 'wide-failure', healthy: healthyCount, total: statuses.length, percent });
        this.lastWideAlertAt = now;
      }
    }
  }
}
