/**
 * Generic Chain Adapter — fallback for chains without dedicated adapters.
 *
 * Returns mock/simulated data so every chain in the registry can
 * show a connector status and basic metrics in the UI.
 */
import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ConnectorMetrics, ChainDescriptor } from "../types";
export declare class GenericAdapter extends BaseChainAdapter {
    readonly chain: ChainDescriptor;
    constructor(chain: ChainDescriptor);
    getLatestBlock(): Promise<Block>;
    getBlock(numberOrHash: string | number): Promise<Block>;
    getTransaction(hash: string): Promise<Transaction>;
    getMetrics(): Promise<ConnectorMetrics>;
}
//# sourceMappingURL=generic.d.ts.map