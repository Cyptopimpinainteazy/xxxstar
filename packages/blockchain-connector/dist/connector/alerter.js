import fetch from 'node-fetch';
function getErrorMessage(error) {
    return error instanceof Error ? error.message : String(error);
}
export class Alerter {
    monitor;
    opts;
    lastWideAlertAt;
    constructor(monitor, opts = {}) {
        this.monitor = monitor;
        this.opts = Object.assign({ wideFailureThresholdPercent: 50, wideFailureWindowSec: 300 }, opts);
        this.monitor.on('status-change', (ev) => this.onStatusChange(ev));
        // periodic check for wide failures
        setInterval(() => this.checkWideFailures(), (this.opts.wideFailureWindowSec || 300) * 1000);
    }
    async onStatusChange(ev) {
        const msg = `Endpoint ${ev.endpoint} changed: healthy=${ev.healthy} (was=${ev.previous})`;
        console.warn('ALERT:', msg);
        if (this.opts.slackWebhook)
            await this.postSlack(msg);
        if (this.opts.webhookUrl)
            await this.postWebhook({ type: 'status-change', endpoint: ev.endpoint, healthy: ev.healthy });
    }
    async postSlack(text) {
        if (!this.opts.slackWebhook)
            return;
        try {
            await fetch(this.opts.slackWebhook, { method: 'POST', body: JSON.stringify({ text }), headers: { 'Content-Type': 'application/json' } });
        }
        catch (error) {
            console.warn('Alerter: slack post failed', getErrorMessage(error));
        }
    }
    async postWebhook(payload) {
        if (!this.opts.webhookUrl)
            return;
        try {
            await fetch(this.opts.webhookUrl, { method: 'POST', body: JSON.stringify(payload), headers: { 'Content-Type': 'application/json' } });
        }
        catch (error) {
            console.warn('Alerter: webhook post failed', getErrorMessage(error));
        }
    }
    async checkWideFailures() {
        // compute percent healthy across all monitored endpoints
        const statuses = [];
        // HealthMonitor doesn't currently expose a full list; use internal map via any
        const map = this.monitor.statuses;
        if (!map)
            return;
        for (const v of map.values())
            statuses.push(v);
        if (statuses.length === 0)
            return;
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
//# sourceMappingURL=alerter.js.map