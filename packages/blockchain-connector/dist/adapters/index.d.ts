/**
 * Adapter Factory — creates the correct chain adapter from a ChainDescriptor.
 */
import type { ChainDescriptor } from "../types";
import type { IChainAdapter } from "./base";
export declare function createAdapter(chain: ChainDescriptor): IChainAdapter;
export type { IChainAdapter } from "./base";
export { BaseChainAdapter } from "./base";
export { EvmAdapter } from "./evm";
export { SolanaAdapter } from "./solana";
export { BitcoinAdapter } from "./bitcoin";
export { CosmosAdapter } from "./cosmos";
export { NearAdapter } from "./near";
export { GenericAdapter } from "./generic";
export { SubstrateAdapter } from "./substrate";
//# sourceMappingURL=index.d.ts.map