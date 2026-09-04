/**
 * HTLC Module — Re-exports all HTLC adapters and utilities.
 */
export { type IHTLCAdapter, generateSecret, sha256Hex, sha256FromHex, bytesToHex, hexToBytes, calculateTimeLocks } from "./base";
export { EvmHTLCAdapter } from "./evm";
export { SolanaHTLCAdapter } from "./solana";
export { BitcoinHTLCAdapter } from "./bitcoin";
export { SubstrateHTLCAdapter } from "./substrate";
import type { ChainId } from "../types";
import type { IHTLCAdapter } from "./base";
export interface HTLCAdapterConfig {
    chainId: ChainId;
    rpcEndpoint: string;
    wsEndpoint?: string;
    htlcContractAddress?: string;
}
/**
 * Factory: create the right HTLC adapter for a given chain.
 */
export declare function createHTLCAdapter(config: HTLCAdapterConfig): IHTLCAdapter;
//# sourceMappingURL=index.d.ts.map