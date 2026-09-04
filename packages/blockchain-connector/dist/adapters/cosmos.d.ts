/**
 * Cosmos Chain Adapter — Cosmos Hub, Osmosis, and other Tendermint chains.
 *
 * Uses CometBFT/Tendermint REST + RPC endpoints.
 */
import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, ChainDescriptor } from "../types";
export declare class CosmosAdapter extends BaseChainAdapter {
    readonly chain: ChainDescriptor;
    constructor(chain: ChainDescriptor);
    getLatestBlock(): Promise<Block>;
    getBlock(numberOrHash: string | number): Promise<Block>;
    getTransaction(hash: string): Promise<Transaction>;
    getValidators(): Promise<ValidatorInfo[]>;
    getMetrics(): Promise<ConnectorMetrics>;
    private parseBlock;
}
//# sourceMappingURL=cosmos.d.ts.map