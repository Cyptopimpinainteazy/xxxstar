/**
 * Chain Registry — canonical list of all supported blockchains.
 *
 * Includes mainnets + testnets for: Ethereum family, Bitcoin, Solana,
 * Polygon, BSC, Avalanche, Near, Cosmos, Substrate, and more.
 */
import type { ChainDescriptor } from "../types";
export declare const CHAIN_REGISTRY: ChainDescriptor[];
/**
 * Get a chain descriptor by ID.
 */
export declare function getChain(id: string): ChainDescriptor | undefined;
/**
 * Get all chains, optionally filtered.
 */
export declare function getChains(filter?: {
    family?: ChainDescriptor["family"];
    network?: ChainDescriptor["network"];
    available?: boolean;
}): ChainDescriptor[];
/**
 * Get unique chain families.
 */
export declare function getChainFamilies(): ChainDescriptor["family"][];
/**
 * Count chains by family.
 */
export declare function chainCountByFamily(): Record<string, number>;
//# sourceMappingURL=registry.d.ts.map