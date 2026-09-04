/**
 * Connector Manager — creates, manages, and monitors chain connectors.
 *
 * Central orchestrator that the SDK and UI both call into.
 */
import { CHAIN_REGISTRY, getChain } from "../chains/registry";
import { createAdapter } from "../adapters";
import { HealthMonitor } from "./health-monitor";
import { chainDB } from "../chains/db";
export class ConnectorManager {
    connectors = new Map();
    monitor;
    endpointToConnectors = new Map();
    connectorQuotaOwners = new Map();
    connectorQuotaProvider;
    constructor(opts) {
        this.connectorQuotaProvider = opts?.connectorQuotaProvider;
        if (opts?.enableHealthMonitor) {
            this.monitor = new HealthMonitor({ concurrency: opts.concurrency || 50, timeoutMs: opts.timeoutMs || 10000, intervalMs: opts.intervalMs || 60000 });
        }
    }
    enableHealthMonitor(opts) {
        if (!this.monitor) {
            this.monitor = new HealthMonitor({ concurrency: opts?.concurrency || 50, timeoutMs: opts?.timeoutMs || 10000, intervalMs: opts?.intervalMs || 60000 });
            // Log status changes and optionally trigger alerts (hook for external alerting)
            this.monitor.on('status-change', (ev) => {
                // ev: { endpoint, healthy, previous }
                console.warn(`HealthMonitor: ${ev.endpoint} -> healthy=${ev.healthy} (previous=${ev.previous})`);
                if (ev.healthy === false) {
                    const set = this.endpointToConnectors.get(ev.endpoint);
                    if (set) {
                        for (const id of set) {
                            const managed = this.connectors.get(id);
                            if (managed) {
                                // try to failover asynchronously
                                this.attemptFailover(managed).catch((e) => console.warn(`Failover for ${id} failed: ${e?.message || e}`));
                            }
                        }
                    }
                }
            });
        }
    }
    async chooseHealthyEndpoint(chain, options) {
        const endpoints = options.endpoint ? [options.endpoint] : chain.defaultRpcUrls;
        if (!endpoints || endpoints.length === 0)
            return null;
        if (this.monitor) {
            // ask monitor for a healthy endpoint (requires prior probes)
            const healthy = this.monitor.getHealthyEndpoint(endpoints);
            if (healthy)
                return healthy;
            // if none known healthy, do a targeted probe of endpoints in parallel and return first healthy
            const results = await this.monitor.probeEndpoints(endpoints, 10);
            const first = results.find((r) => r.healthy);
            if (first)
                return first.endpoint;
        }
        // Fallback: use rotation from DB
        return chainDB.getNextRpc(chain.id);
    }
    async attemptFailover(managed) {
        const chain = managed.instance.chain;
        const endpoints = managed.instance.options.endpoint ? [managed.instance.options.endpoint] : chain.defaultRpcUrls;
        if (!endpoints || endpoints.length === 0)
            return false;
        if (!this.monitor)
            return false;
        // Find another healthy endpoint (not equal to current)
        const healthy = this.monitor.getHealthyEndpoint(endpoints.filter((e) => e !== managed.instance.options.endpoint));
        if (!healthy)
            return false;
        try {
            await managed.adapter.disconnect().catch(() => { });
            await managed.adapter.connect(healthy);
            // quick health check
            await managed.adapter.getLatestBlock();
            managed.instance.options.endpoint = healthy;
            managed.instance.status = "connected";
            managed.instance.updatedAt = new Date().toISOString();
            // refresh metrics
            managed.instance.metrics = await managed.adapter.getMetrics().catch(() => this.emptyMetrics());
            return true;
        }
        catch (err) {
            // failed to failover
            return false;
        }
    }
    /**
     * Create a new connector to a blockchain.
     */
    async createConnector(options) {
        const chain = getChain(options.chain) ?? chainDB.searchChains(options.chain)[0];
        if (!chain) {
            throw new Error(`Unknown chain: ${options.chain}. Available: ${CHAIN_REGISTRY.map((chain) => chain.id).join(", ")}`);
        }
        const id = `conn_${crypto.randomUUID().split("-")[0]}`;
        const adapter = createAdapter(chain);
        const endpoints = options.endpoint ? [options.endpoint] : chain.defaultRpcUrls;
        if (!endpoints || endpoints.length === 0) {
            throw new Error(`No RPC endpoint for ${chain.name}. Provide one in options.endpoint or add to chain.defaultRpcUrls.`);
        }
        const apiKey = options.auth?.apiKey;
        let connectorSlotAcquired = false;
        if (this.connectorQuotaProvider) {
            if (!apiKey) {
                throw new Error('API key required for connector quota enforcement');
            }
            try {
                await this.connectorQuotaProvider.acquireConnectorSlot(apiKey);
                connectorSlotAcquired = true;
            }
            catch (error) {
                if (error?.message === 'CONNECTOR_QUOTA_EXCEEDED') {
                    throw new Error('Connector quota exceeded for API key tier');
                }
                if (error?.message === 'INVALID_API_KEY') {
                    throw new Error('Invalid API key');
                }
                throw error;
            }
        }
        const instance = {
            id,
            options,
            chain,
            status: "connecting",
            metrics: this.emptyMetrics(),
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
        };
        this.connectors.set(id, { instance, adapter });
        try {
            // Choose a preferred endpoint (monitor-suggested) and fall back to sequential attempts
            let connectedEndpoint = null;
            const errors = [];
            const preferred = await this.chooseHealthyEndpoint(chain, options);
            const tryOrder = preferred ? [preferred, ...endpoints.filter((e) => e !== preferred)] : endpoints;
            for (const ep of tryOrder) {
                try {
                    await adapter.connect(ep);
                    try {
                        await adapter.getLatestBlock(); // quick health check
                        connectedEndpoint = ep;
                        break;
                    }
                    catch (err) {
                        errors.push(`Health check failed for ${ep}: ${err.message}`);
                        await adapter.disconnect().catch(() => { });
                    }
                }
                catch (err) {
                    errors.push(`Connect failed for ${ep}: ${err.message}`);
                }
            }
            if (connectedEndpoint) {
                instance.status = "connected";
                instance.updatedAt = new Date().toISOString();
                instance.options.endpoint = connectedEndpoint;
                // If a health monitor is enabled, probe and register endpoints for background checks
                if (this.monitor) {
                    // probe chosen endpoint now and also schedule all chain endpoints for periodic checks
                    await this.monitor.probeEndpoint(connectedEndpoint).catch(() => { });
                    if (chain.defaultRpcUrls && chain.defaultRpcUrls.length > 0) {
                        this.monitor.startPeriodic(chain.defaultRpcUrls);
                        // register endpoint->connector mapping for failover handling
                        for (const ep of chain.defaultRpcUrls) {
                            const s = this.endpointToConnectors.get(ep) ?? new Set();
                            s.add(id);
                            this.endpointToConnectors.set(ep, s);
                        }
                    }
                }
                // Fetch initial metrics
                const metrics = await adapter.getMetrics().catch(() => this.emptyMetrics());
                instance.metrics = metrics;
                if (apiKey) {
                    this.connectorQuotaOwners.set(id, apiKey);
                }
            }
            else {
                instance.status = "error";
                instance.error = `All endpoints failed: ${errors.join("; ")}`;
                instance.updatedAt = new Date().toISOString();
                if (this.connectorQuotaProvider && apiKey && connectorSlotAcquired) {
                    await this.connectorQuotaProvider.releaseConnectorSlot(apiKey);
                    connectorSlotAcquired = false;
                }
            }
            return instance;
        }
        catch (error) {
            if (this.connectorQuotaProvider && apiKey && connectorSlotAcquired) {
                await this.connectorQuotaProvider.releaseConnectorSlot(apiKey);
            }
            throw error;
        }
    }
    /**
     * Get a connector by ID.
     */
    getConnector(id) {
        return this.connectors.get(id)?.instance;
    }
    /**
     * List all connectors.
     */
    listConnectors() {
        return Array.from(this.connectors.values()).map((c) => c.instance);
    }
    /**
     * Refresh metrics for a connector.
     */
    async refreshMetrics(id) {
        const managed = this.connectors.get(id);
        if (!managed)
            throw new Error(`Connector ${id} not found`);
        try {
            const metrics = await managed.adapter.getMetrics();
            managed.instance.metrics = metrics;
            managed.instance.updatedAt = new Date().toISOString();
            return metrics;
        }
        catch (err) {
            managed.instance.status = "degraded";
            managed.instance.error = err.message;
            // Try to failover automatically
            const failedOver = await this.attemptFailover(managed).catch(() => false);
            if (failedOver) {
                // re-run metrics after failover
                const metrics = await managed.adapter.getMetrics().catch(() => this.emptyMetrics());
                managed.instance.metrics = metrics;
                managed.instance.status = "connected";
                managed.instance.updatedAt = new Date().toISOString();
                return metrics;
            }
            throw err;
        }
    }
    /**
     * Get latest block via a connector.
     */
    async getLatestBlock(id) {
        const managed = this.connectors.get(id);
        if (!managed)
            throw new Error(`Connector ${id} not found`);
        try {
            return await managed.adapter.getLatestBlock();
        }
        catch (err) {
            // Try automatic failover and retry once
            const failedOver = await this.attemptFailover(managed).catch(() => false);
            if (failedOver)
                return managed.adapter.getLatestBlock();
            throw err;
        }
    }
    /**
     * Get a specific block.
     */
    async getBlock(id, numberOrHash) {
        const managed = this.connectors.get(id);
        if (!managed)
            throw new Error(`Connector ${id} not found`);
        try {
            return await managed.adapter.getBlock(numberOrHash);
        }
        catch (err) {
            const failedOver = await this.attemptFailover(managed).catch(() => false);
            if (failedOver)
                return managed.adapter.getBlock(numberOrHash);
            throw err;
        }
    }
    /**
     * Get a transaction.
     */
    async getTransaction(id, hash) {
        const managed = this.connectors.get(id);
        if (!managed)
            throw new Error(`Connector ${id} not found`);
        try {
            return await managed.adapter.getTransaction(hash);
        }
        catch (err) {
            const failedOver = await this.attemptFailover(managed).catch(() => false);
            if (failedOver)
                return managed.adapter.getTransaction(hash);
            throw err;
        }
    }
    /**
     * Disconnect and remove a connector.
     */
    async removeConnector(id) {
        const managed = this.connectors.get(id);
        if (managed) {
            const apiKey = this.connectorQuotaOwners.get(id);
            try {
                await managed.adapter.disconnect();
            }
            finally {
                this.connectors.delete(id);
                if (this.connectorQuotaProvider && apiKey) {
                    await this.connectorQuotaProvider.releaseConnectorSlot(apiKey);
                }
                this.connectorQuotaOwners.delete(id);
            }
        }
    }
    /**
     * Get the underlying adapter for advanced operations.
     */
    getAdapter(id) {
        return this.connectors.get(id)?.adapter;
    }
    emptyMetrics() {
        return {
            blockHeight: 0,
            tps: 0,
            peerCount: 0,
            latencyMs: 0,
            totalRequests: 0,
            totalErrors: 0,
            uptimeSeconds: 0,
            finalityLag: 0,
        };
    }
}
//# sourceMappingURL=manager.js.map