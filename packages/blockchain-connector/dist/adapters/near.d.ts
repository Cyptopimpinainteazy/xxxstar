/**
 * NEAR Chain Adapter — NEAR Mainnet and Testnet.
 */
import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, ChainDescriptor } from "../types";
export declare class NearAdapter extends BaseChainAdapter {
    readonly chain: ChainDescriptor;
    constructor(chain: ChainDescriptor);
    getLatestBlock(): Promise<Block>;
    getBlock(numberOrHash: string | number): Promise<Block>;
    getTransaction(hash: string): Promise<Transaction>;
    getValidators(): Promise<ValidatorInfo[]>;
    getMetrics(): Promise<ConnectorMetrics>;
    private parseBlock;
    private nearRpc;
}
//# sourceMappingURL=near.d.ts.map