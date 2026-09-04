/**
 * IChainAdapter — abstract adapter interface for blockchain connectors.
 *
 * Each chain family (EVM, Bitcoin, Solana, etc.) implements this interface.
 */
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, EventEnvelope, SubscriptionFilter, ChainDescriptor } from "../types";
export interface IChainAdapter {
    /** Chain descriptor */
    readonly chain: ChainDescriptor;
    /** Connect to the chain endpoint */
    connect(endpoint: string): Promise<void>;
    /** Disconnect cleanly */
    disconnect(): Promise<void>;
    /** Check if connected */
    isConnected(): boolean;
    /** Get the latest block */
    getLatestBlock(): Promise<Block>;
    /** Get a specific block by number or hash */
    getBlock(numberOrHash: string | number): Promise<Block>;
    /** Get a transaction by hash */
    getTransaction(hash: string): Promise<Transaction>;
    /** Get current validator set (if applicable) */
    getValidators?(): Promise<ValidatorInfo[]>;
    /** Get live connector metrics */
    getMetrics(): Promise<ConnectorMetrics>;
    /** Submit a signed transaction */
    submitRawTx?(signedTx: string): Promise<{
        txHash: string;
    }>;
    /** Subscribe to chain events via callback */
    subscribe?(events: string[], filter: SubscriptionFilter | undefined, handler: (event: EventEnvelope) => void): Promise<{
        unsubscribe: () => void;
    }>;
}
/**
 * Base adapter with shared logic for latency tracking and error counting.
 */
export declare abstract class BaseChainAdapter implements IChainAdapter {
    abstract readonly chain: ChainDescriptor;
    protected endpoint: string;
    protected connected: boolean;
    protected requestCount: number;
    protected errorCount: number;
    protected startTime: number;
    protected latencySamples: number[];
    connect(endpoint: string): Promise<void>;
    disconnect(): Promise<void>;
    isConnected(): boolean;
    abstract getLatestBlock(): Promise<Block>;
    abstract getBlock(numberOrHash: string | number): Promise<Block>;
    abstract getTransaction(hash: string): Promise<Transaction>;
    getMetrics(): Promise<ConnectorMetrics>;
    /** Track a request's latency */
    protected trackRequest(startMs: number): void;
    protected trackError(): void;
    /**
     * Make an RPC call with latency tracking.
     */
    protected rpcCall<T>(method: string, params?: unknown[]): Promise<T>;
    /**
     * Make an HTTP GET call with latency tracking.
     */
    protected httpGet<T>(path: string): Promise<T>;
}
//# sourceMappingURL=base.d.ts.map