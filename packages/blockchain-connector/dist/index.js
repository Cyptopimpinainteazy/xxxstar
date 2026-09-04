/**
 * @x3-chain/blockchain-connector — Public API
 *
 * Enterprise-grade multi-chain connector SDK.
 */
// Chain Registry
export { CHAIN_REGISTRY, getChain, getChains, getChainFamilies, chainCountByFamily } from "./chains/registry";
// Adapters
export { createAdapter, BaseChainAdapter, EvmAdapter, SolanaAdapter, BitcoinAdapter, CosmosAdapter, NearAdapter, GenericAdapter, } from "./adapters";
// Connector Manager
export { ConnectorManager } from "./connector/manager";
// Test Harness
export { TestRunner, TEST_PROFILES } from "./testing/harness";
import { ConnectorManager } from "./connector/manager";
const defaultManager = new ConnectorManager();
/**
 * Connect to a blockchain — single-call convenience API.
 *
 * @example
 * ```ts
 * import { connect } from "@x3-chain/blockchain-connector";
 *
 * const conn = await connect({ chain: "ethereum", network: "mainnet", type: "rpc" });
 * const block = await defaultManager.getLatestBlock(conn.id);
 * ```
 */
export async function connect(options) {
    return defaultManager.createConnector(options);
}
export function getDefaultManager() {
    return defaultManager;
}
//# sourceMappingURL=index.js.map