/**
 * Solana Chain Adapter — Mainnet-beta, Devnet, Testnet.
 *
 * Uses Solana JSON-RPC over HTTP.
 */
import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, ChainDescriptor } from "../types";
export declare class SolanaAdapter extends BaseChainAdapter {
    readonly chain: ChainDescriptor;
    constructor(chain: ChainDescriptor);
    getLatestBlock(): Promise<Block>;
    getBlock(numberOrHash: string | number): Promise<Block>;
    getTransaction(hash: string): Promise<Transaction>;
    getValidators(): Promise<ValidatorInfo[]>;
    getMetrics(): Promise<ConnectorMetrics>;
}
//# sourceMappingURL=solana.d.ts.map