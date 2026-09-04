/**
 * Bitcoin Chain Adapter — Uses Blockstream/Esplora REST API.
 */
import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ConnectorMetrics, ChainDescriptor } from "../types";
export declare class BitcoinAdapter extends BaseChainAdapter {
    readonly chain: ChainDescriptor;
    constructor(chain: ChainDescriptor);
    getLatestBlock(): Promise<Block>;
    getBlock(numberOrHash: string | number): Promise<Block>;
    getTransaction(hash: string): Promise<Transaction>;
    getMetrics(): Promise<ConnectorMetrics>;
    private httpGetText;
}
//# sourceMappingURL=bitcoin.d.ts.map