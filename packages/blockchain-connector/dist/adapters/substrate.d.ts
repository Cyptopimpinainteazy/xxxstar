/**
 * Substrate Adapter — minimal implementation using @polkadot/api
 */
import type { Block, Transaction, ChainDescriptor } from "../types";
import { BaseChainAdapter } from "./base";
export declare class SubstrateAdapter extends BaseChainAdapter {
    readonly chain: ChainDescriptor;
    private api?;
    constructor(chain: ChainDescriptor);
    connect(endpoint: string): Promise<void>;
    disconnect(): Promise<void>;
    isConnected(): boolean;
    getLatestBlock(): Promise<Block>;
    getBlock(numberOrHash: string | number): Promise<Block>;
    getTransaction(hash: string): Promise<Transaction>;
    getMetrics(): Promise<any>;
    getSystemHealth(): Promise<any>;
    /** Subscribe to new blocks and call handler with a canonical Block payload */
    subscribe(events: string[], filter: any, handler: (event: any) => void): Promise<{
        unsubscribe: () => void;
    }>;
}
//# sourceMappingURL=substrate.d.ts.map