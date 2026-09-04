/**
 * @x3-chain/blockchain-connector — Public API
 *
 * Enterprise-grade multi-chain connector SDK.
 */
export type { ChainFamily, NetworkType, ChainDescriptor, ConnectorType, ConnectorStatus, ConnectorAuth, ConnectorOptions, ConnectorInstance, ConnectorMetrics, Block, Transaction, ValidatorInfo, EventType, EventEnvelope, ReorgEvent, LogEvent, ValidatorUpdate, ErrorPayload, SubscriptionFilter, SubscriptionRequest, Subscription, TestProfileId, TestStatus, TestProfile, TestCase, TestRun, TestResult, TestMetrics, TestSummary, BillingTier, BillingPlan, BillingAccount, ApiResponse, HealthCheck, } from "./types";
export { CHAIN_REGISTRY, getChain, getChains, getChainFamilies, chainCountByFamily } from "./chains/registry";
export { createAdapter, type IChainAdapter, BaseChainAdapter, EvmAdapter, SolanaAdapter, BitcoinAdapter, CosmosAdapter, NearAdapter, GenericAdapter, } from "./adapters";
export { ConnectorManager } from "./connector/manager";
export { TestRunner, TEST_PROFILES } from "./testing/harness";
import type { ConnectorOptions, ConnectorInstance } from "./types";
import { ConnectorManager } from "./connector/manager";
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
export declare function connect(options: ConnectorOptions): Promise<ConnectorInstance>;
export declare function getDefaultManager(): ConnectorManager;
//# sourceMappingURL=index.d.ts.map