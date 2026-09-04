/**
 * @x3-chain/blockchain-connector — Public API
 *
 * Enterprise-grade multi-chain connector SDK.
 */

// Types
export type {
  ChainFamily,
  NetworkType,
  ChainDescriptor,
  ConnectorType,
  ConnectorStatus,
  ConnectorAuth,
  ConnectorOptions,
  ConnectorInstance,
  ConnectorMetrics,
  Block,
  Transaction,
  ValidatorInfo,
  EventType,
  EventEnvelope,
  ReorgEvent,
  LogEvent,
  ValidatorUpdate,
  ErrorPayload,
  SubscriptionFilter,
  SubscriptionRequest,
  Subscription,
  TestProfileId,
  TestStatus,
  TestProfile,
  TestCase,
  TestRun,
  TestResult,
  TestMetrics,
  TestSummary,
  BillingTier,
  BillingPlan,
  BillingAccount,
  ApiResponse,
  HealthCheck,
} from "./types";

// Chain Registry
export { CHAIN_REGISTRY, getChain, getChains, getChainFamilies, chainCountByFamily } from "./chains/registry";

// Adapters
export {
  createAdapter,
  type IChainAdapter,
  BaseChainAdapter,
  EvmAdapter,
  SolanaAdapter,
  BitcoinAdapter,
  CosmosAdapter,
  NearAdapter,
  GenericAdapter,
} from "./adapters";

// Connector Manager
export { ConnectorManager } from "./connector/manager";

// Test Harness
export { TestRunner, TEST_PROFILES } from "./testing/harness";

// ─── Convenience: connect() function ────────────────────────────

import type { ConnectorOptions, ConnectorInstance } from "./types";
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
export async function connect(options: ConnectorOptions): Promise<ConnectorInstance> {
  return defaultManager.createConnector(options);
}

export function getDefaultManager(): ConnectorManager {
  return defaultManager;
}
