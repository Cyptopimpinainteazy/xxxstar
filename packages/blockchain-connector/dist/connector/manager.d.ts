/**
 * Connector Manager — creates, manages, and monitors chain connectors.
 *
 * Central orchestrator that the SDK and UI both call into.
 */
import type { ConnectorOptions, ConnectorInstance, ConnectorMetrics, Block, Transaction } from "../types";
import { type IChainAdapter } from "../adapters";
interface ConnectorQuotaProvider {
    acquireConnectorSlot(apiKey: string): Promise<{
        remaining: number;
    }>;
    releaseConnectorSlot(apiKey: string): Promise<void>;
}
export declare class ConnectorManager {
    private connectors;
    private monitor?;
    private endpointToConnectors;
    private connectorQuotaOwners;
    private connectorQuotaProvider?;
    constructor(opts?: {
        enableHealthMonitor?: boolean;
        intervalMs?: number;
        concurrency?: number;
        timeoutMs?: number;
        connectorQuotaProvider?: ConnectorQuotaProvider;
    });
    enableHealthMonitor(opts?: {
        intervalMs?: number;
        concurrency?: number;
        timeoutMs?: number;
    }): void;
    private chooseHealthyEndpoint;
    private attemptFailover;
    /**
     * Create a new connector to a blockchain.
     */
    createConnector(options: ConnectorOptions): Promise<ConnectorInstance>;
    /**
     * Get a connector by ID.
     */
    getConnector(id: string): ConnectorInstance | undefined;
    /**
     * List all connectors.
     */
    listConnectors(): ConnectorInstance[];
    /**
     * Refresh metrics for a connector.
     */
    refreshMetrics(id: string): Promise<ConnectorMetrics>;
    /**
     * Get latest block via a connector.
     */
    getLatestBlock(id: string): Promise<Block>;
    /**
     * Get a specific block.
     */
    getBlock(id: string, numberOrHash: string | number): Promise<Block>;
    /**
     * Get a transaction.
     */
    getTransaction(id: string, hash: string): Promise<Transaction>;
    /**
     * Disconnect and remove a connector.
     */
    removeConnector(id: string): Promise<void>;
    /**
     * Get the underlying adapter for advanced operations.
     */
    getAdapter(id: string): IChainAdapter | undefined;
    private emptyMetrics;
}
export {};
//# sourceMappingURL=manager.d.ts.map