import type { HealthMonitor } from './health-monitor';
export interface AlerterOptions {
    slackWebhook?: string;
    webhookUrl?: string;
    wideFailureThresholdPercent?: number;
    wideFailureWindowSec?: number;
}
export declare class Alerter {
    private monitor;
    private opts;
    private lastWideAlertAt?;
    constructor(monitor: HealthMonitor, opts?: AlerterOptions);
    private onStatusChange;
    private postSlack;
    private postWebhook;
    checkWideFailures(): Promise<void>;
}
//# sourceMappingURL=alerter.d.ts.map