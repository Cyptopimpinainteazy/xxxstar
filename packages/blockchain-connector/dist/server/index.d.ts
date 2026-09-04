import http from 'http';
import type { HealthMonitor } from '../connector/health-monitor';
import { ConnectorManager } from '../connector/manager';
import { BillingRegistry } from './billing';
export declare function startServer({ monitor, port, billingRegistry, connectorManager, }: {
    monitor?: HealthMonitor;
    port?: number;
    billingRegistry?: BillingRegistry;
    connectorManager?: ConnectorManager;
}): http.Server<typeof http.IncomingMessage, typeof http.ServerResponse>;
//# sourceMappingURL=index.d.ts.map