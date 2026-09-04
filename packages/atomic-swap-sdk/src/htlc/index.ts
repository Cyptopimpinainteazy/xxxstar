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
import { EvmHTLCAdapter } from "./evm";
import { SolanaHTLCAdapter } from "./solana";
import { BitcoinHTLCAdapter } from "./bitcoin";
import { SubstrateHTLCAdapter } from "./substrate";

/** EVM chain IDs that use the EvmHTLCAdapter */
const EVM_CHAINS = new Set([
  "ethereum", "ethereum-sepolia", "ethereum-holesky",
  "polygon", "polygon-amoy",
  "bsc", "bsc-testnet",
  "arbitrum", "arbitrum-sepolia",
  "optimism", "optimism-sepolia",
  "base", "base-sepolia",
  "avalanche", "avalanche-fuji",
  "fantom", "zksync", "linea", "scroll", "celo", "gnosis", "moonbeam",
]);

const SOLANA_CHAINS = new Set(["solana", "solana-devnet", "solana-testnet"]);
const BITCOIN_CHAINS = new Set(["bitcoin", "bitcoin-testnet", "bitcoin-signet"]);
const SUBSTRATE_CHAINS = new Set(["x3-substrate", "polkadot", "kusama"]);

export interface HTLCAdapterConfig {
  chainId: ChainId;
  rpcEndpoint: string;
  wsEndpoint?: string;
  htlcContractAddress?: string; // EVM: deployed AtlasHTLC contract; Solana: program ID
}

/**
 * Factory: create the right HTLC adapter for a given chain.
 */
export function createHTLCAdapter(config: HTLCAdapterConfig): IHTLCAdapter {
  if (EVM_CHAINS.has(config.chainId)) {
    if (!config.htlcContractAddress) {
      throw new Error(`HTLC contract address required for EVM chain ${config.chainId}`);
    }
    return new EvmHTLCAdapter(config.chainId, config.rpcEndpoint, config.htlcContractAddress);
  }

  if (SOLANA_CHAINS.has(config.chainId)) {
    if (!config.htlcContractAddress) {
      throw new Error(`HTLC program ID required for Solana chain ${config.chainId}`);
    }
    return new SolanaHTLCAdapter(config.chainId, config.rpcEndpoint, config.htlcContractAddress);
  }

  if (BITCOIN_CHAINS.has(config.chainId)) {
    const network = config.chainId === "bitcoin" ? "mainnet" as const :
                    config.chainId === "bitcoin-signet" ? "signet" as const : "testnet" as const;
    return new BitcoinHTLCAdapter(config.chainId, config.rpcEndpoint, network);
  }

  if (SUBSTRATE_CHAINS.has(config.chainId)) {
    return new SubstrateHTLCAdapter(config.chainId, config.rpcEndpoint, config.wsEndpoint);
  }

  // Default: try EVM adapter if contract address provided
  if (config.htlcContractAddress) {
    return new EvmHTLCAdapter(config.chainId, config.rpcEndpoint, config.htlcContractAddress);
  }

  throw new Error(`No HTLC adapter available for chain ${config.chainId}`);
}
