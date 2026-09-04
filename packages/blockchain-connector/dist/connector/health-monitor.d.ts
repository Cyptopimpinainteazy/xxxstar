/**
 * Health Monitor — lightweight endpoint probing and status tracking.
 */
import { EventEmitter } from "events";
export type EndpointStatus = {
    endpoint: string;
    healthy: boolean;
    lastChecked: number | null;
    lastError?: string;
};
export declare class HealthMonitor extends EventEmitter {
    private statuses;
    private intervalId;
    private concurrency;
    private timeoutMs;
    private intervalMs;
    private gaugeHealthy?;
    private counterStateChanges?;
    constructor({ concurrency, timeoutMs, intervalMs }?: {
        concurrency?: number | undefined;
        timeoutMs?: number | undefined;
        intervalMs?: number | undefined;
    });
    getStatus(endpoint: string): EndpointStatus | undefined;
    getHealthyEndpoint(endpoints: string[]): string | null;
    private recordStatusChange;
    probeEndpoint(endpoint: string): Promise<EndpointStatus>;
    probeEndpoints(endpoints: string[], concurrency?: number): Promise<EndpointStatus[]>;
    startPeriodic(endpoints: string[]): void;
    stop(): void;
}
//# sourceMappingURL=health-monitor.d.ts.map