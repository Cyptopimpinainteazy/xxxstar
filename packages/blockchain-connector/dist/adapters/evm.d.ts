/**
 * EVM Chain Adapter — Ethereum, Polygon, BSC, Arbitrum, Optimism, Base, etc.
 *
 * Uses JSON-RPC over HTTP. Compatible with any eth_* endpoint.
 */
import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ConnectorMetrics, ChainDescriptor } from "../types";
export declare class EvmAdapter extends BaseChainAdapter {
    readonly chain: ChainDescriptor;
    constructor(chain: ChainDescriptor);
    getLatestBlock(): Promise<Block>;
    getBlock(numberOrHash: string | number): Promise<Block>;
    getTransaction(hash: string): Promise<Transaction>;
    getMetrics(): Promise<ConnectorMetrics>;
    submitRawTx(signedTx: string): Promise<{
        txHash: string;
    }>;
    getBalance(address: string): Promise<string>;
    private parseBlock;
}
//# sourceMappingURL=evm.d.ts.map