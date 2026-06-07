/**
 * Adapter Factory — creates the correct chain adapter from a ChainDescriptor.
 */

import type { ChainDescriptor } from "../types";
import type { IChainAdapter } from "./base";
import { EvmAdapter } from "./evm";
import { SolanaAdapter } from "./solana";
import { BitcoinAdapter } from "./bitcoin";
import { CosmosAdapter } from "./cosmos";
import { NearAdapter } from "./near";
import { SubstrateAdapter } from "./substrate";
import { GenericAdapter } from "./generic";

export function createAdapter(chain: ChainDescriptor): IChainAdapter {
  switch (chain.family) {
    case "evm":
      return new EvmAdapter(chain);
    case "solana":
      return new SolanaAdapter(chain);
    case "bitcoin":
      return new BitcoinAdapter(chain);
    case "cosmos":
      return new CosmosAdapter(chain);
    case "near":
      return new NearAdapter(chain);
    case "substrate":
      return new SubstrateAdapter(chain);
    default:
      return new GenericAdapter(chain);
  }
}

export type { IChainAdapter } from "./base";
export { BaseChainAdapter } from "./base";
export { EvmAdapter } from "./evm";
export { SolanaAdapter } from "./solana";
export { BitcoinAdapter } from "./bitcoin";
export { CosmosAdapter } from "./cosmos";
export { NearAdapter } from "./near";
export { GenericAdapter } from "./generic";
export { SubstrateAdapter } from "./substrate";
