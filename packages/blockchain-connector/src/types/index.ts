/**
 * @x3-chain/blockchain-connector — Core Types
 *
 * Unified data models for blocks, transactions, validators, connectors,
 * test harness, and billing across all supported chains.
 */

// ─── Chain & Network Identifiers ────────────────────────────────

export type ChainFamily = "evm" | "bitcoin" | "solana" | "cosmos" | "substrate" | "near" | "other";

export type NetworkType = "mainnet" | "testnet" | "devnet" | "regtest" | "local";

export interface ChainDescriptor {
  /** Internal chain identifier (e.g. "ethereum", "bitcoin", "solana") */
  id: string;
  /** Human name (e.g. "Ethereum Mainnet") */
  name: string;
  /** Chain family for adapter dispatch */
  family: ChainFamily;
  /** Which network variant */
  network: NetworkType;
  /** Native currency symbol */
  nativeCurrency: { name: string; symbol: string; decimals: number };
  /** Numeric chain ID (EVM) or string identifier */
  chainId: number | string;
  /** Default public RPC endpoints */
  defaultRpcUrls: string[];
  /** Default WebSocket endpoints */
  defaultWsUrls: string[];
  /** Block explorer URL pattern */
  explorerUrl?: string;
  /** Whether the chain is available for connector creation */
  available: boolean;
  /** Average block time in seconds */
  avgBlockTimeSeconds: number;
  /** Logo/icon URL or key */
  icon?: string;
  /** Consensus mechanism label */
  consensus: string;
  /** Crypto primitives used */
  signatureAlgorithm: "secp256k1" | "ed25519" | "bls" | "sr25519";
  hashAlgorithm: "keccak256" | "sha256" | "blake2b" | "sha3";
  /** GPU kernel profile if available */
  gpuAccelerated: boolean;
}

// ─── Connector ──────────────────────────────────────────────────

export type ConnectorType = "rpc" | "ws" | "archive";

export type ConnectorStatus =
  | "connecting"
  | "connected"
  | "syncing"
  | "degraded"
  | "disconnected"
  | "error";

export interface ConnectorAuth {
  apiKey?: string;
  jwt?: string;
  basic?: { username: string; password: string };
}

export interface ConnectorOptions {
  /** Which chain to connect to */
  chain: string;
  /** Which network variant */
  network: string;
  /** Connection type */
  type: ConnectorType;
  /** Override RPC endpoint */
  endpoint?: string;
  /** Auth credentials */
  auth?: ConnectorAuth;
  /** Number of confirmations before emitting block events */
  minConfirmations?: number;
  /** Custom label for this connector */
  label?: string;
}

export interface ConnectorInstance {
  /** Unique connector ID */
  id: string;
  /** Creation options */
  options: ConnectorOptions;
  /** Resolved chain descriptor */
  chain: ChainDescriptor;
  /** Current status */
  status: ConnectorStatus;
  /** Live metrics */
  metrics: ConnectorMetrics;
  /** When the connector was created */
  createdAt: string;
  /** Last status change */
  updatedAt: string;
  /** Error message if status is 'error' */
  error?: string;
}

export interface ConnectorMetrics {
  /** Current block height */
  blockHeight: number;
  /** Estimated TPS over last minute */
  tps: number;
  /** Hash rate (PoW chains only) */
  hashRate?: string;
  /** Number of known peers */
  peerCount: number;
  /** Median request latency (ms) */
  latencyMs: number;
  /** Requests served since connector creation */
  totalRequests: number;
  /** Errors since creation */
  totalErrors: number;
  /** Time connected (seconds) */
  uptimeSeconds: number;
  /** Gas price / fee estimate (chain-specific) */
  gasPrice?: string;
  /** Finality lag (blocks behind tip) */
  finalityLag: number;
}

// ─── Canonical Data Models ──────────────────────────────────────

export interface Block {
  hash: string;
  number: number;
  parentHash: string;
  timestamp: string;
  txCount: number;
  size: number;
  miner?: string;
  gasUsed?: string;
  gasLimit?: string;
  baseFeePerGas?: string;
  difficulty?: string;
  nonce?: string;
  stateRoot?: string;
  /** Chain-specific raw payload */
  raw?: unknown;
}

export interface Transaction {
  hash: string;
  blockHash?: string;
  blockNumber?: number;
  from: string;
  to?: string;
  value: string;
  data?: string;
  nonce: number;
  gasPrice?: string;
  gasLimit?: string;
  status?: "success" | "reverted" | "pending";
  timestamp?: string;
  confirmations?: number;
  raw?: unknown;
}

export interface ValidatorInfo {
  address: string;
  stake?: string;
  commission?: number;
  active: boolean;
  uptime?: number;
  blocksProduced?: number;
  lastVotedSlot?: number;
  /** Chain-specific identity */
  identity?: string;
}

// ─── Event Envelope (WebSocket) ──────────────────────────────────

export type EventType = "block" | "tx" | "log" | "validator_update" | "reorg" | "error";

export interface EventEnvelope {
  id: string;
  type: EventType;
  connectorId: string;
  chain: string;
  network: string;
  timestamp: string;
  payload: Block | Transaction | ReorgEvent | LogEvent | ValidatorUpdate | ErrorPayload;
}

export interface ReorgEvent {
  depth: number;
  oldHead: { hash: string; number: number };
  newHead: { hash: string; number: number };
  droppedBlocks: string[];
  addedBlocks: string[];
}

export interface LogEvent {
  address: string;
  topics: string[];
  data: string;
  blockNumber: number;
  transactionHash: string;
  logIndex: number;
}

export interface ValidatorUpdate {
  validators: ValidatorInfo[];
  epoch?: number;
  timestamp: string;
}

export interface ErrorPayload {
  code: string;
  message: string;
  details?: unknown;
}

// ─── Subscription ────────────────────────────────────────────────

export interface SubscriptionFilter {
  addresses?: string[];
  topics?: string[][];
  minConfirmations?: number;
  includeRaw?: boolean;
}

export interface SubscriptionRequest {
  events: EventType[];
  filter?: SubscriptionFilter;
}

export interface Subscription {
  id: string;
  connectorId: string;
  events: EventType[];
  filter?: SubscriptionFilter;
  active: boolean;
  createdAt: string;
}

// ─── Test Harness ────────────────────────────────────────────────

export type TestProfileId =
  | "latency"
  | "throughput"
  | "reorg-simulation"
  | "edge-cases"
  | "full-suite"
  | "validator-health"
  | "gpu-benchmark"
  | "pool-performance"
  | "custom";

export type TestStatus = "queued" | "running" | "completed" | "failed" | "cancelled";

export interface TestProfile {
  id: TestProfileId;
  name: string;
  description: string;
  /** Tests within this profile */
  tests: TestCase[];
  /** Expected duration in seconds */
  estimatedDuration: number;
  /** Chains this profile supports ("*" = all) */
  supportedChains: string[];
  /** Category for UI grouping */
  category: "performance" | "reliability" | "security" | "functional" | "custom";
}

export interface TestCase {
  id: string;
  name: string;
  description: string;
  type: "latency" | "throughput" | "reorg" | "edge" | "validation" | "gpu" | "pool";
  params: Record<string, unknown>;
}

export interface TestRun {
  id: string;
  connectorId: string;
  profileId: TestProfileId;
  status: TestStatus;
  startedAt: string;
  completedAt?: string;
  results: TestResult[];
  summary?: TestSummary;
  error?: string;
}

export interface TestResult {
  testId: string;
  testName: string;
  passed: boolean;
  durationMs: number;
  metrics: TestMetrics;
  error?: string;
  details?: unknown;
}

export interface TestMetrics {
  /** Latency percentiles */
  p50Ms?: number;
  p90Ms?: number;
  p99Ms?: number;
  /** Throughput */
  requestsPerSecond?: number;
  successRate?: number;
  /** Reorg */
  reorgsDetected?: number;
  reorgsHandledCorrectly?: number;
  /** Errors */
  totalRequests?: number;
  totalErrors?: number;
  /** GPU-specific */
  gpuOpsPerSecond?: number;
  gpuMemoryUsedMB?: number;
  /** Custom */
  [key: string]: number | string | boolean | undefined;
}

export interface TestSummary {
  totalTests: number;
  passed: number;
  failed: number;
  skipped: number;
  totalDurationMs: number;
  overallScore: number; // 0-100
  grade: "A+" | "A" | "B" | "C" | "D" | "F";
}

// ─── Billing ─────────────────────────────────────────────────────

export type BillingTier = "free" | "bronze" | "silver" | "gold" | "enterprise";

export interface BillingPlan {
  tier: BillingTier;
  name: string;
  monthlyPrice: number;
  maxConnectors: number;
  maxRequestsPerMonth: number;
  maxConcurrentWs: number;
  slaUptime: number;
  features: string[];
}

export interface BillingAccount {
  id: string;
  tier: BillingTier;
  apiKey: string;
  usage: {
    requestsThisMonth: number;
    wsMinutesThisMonth: number;
    connectorsActive: number;
  };
  quotaRemaining: {
    requests: number;
    wsMinutes: number;
    connectors: number;
  };
  currentPeriodEnd: string;
}

// ─── API Responses ───────────────────────────────────────────────

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: { code: string; message: string; correlationId: string };
  meta?: { page?: number; pageSize?: number; total?: number };
}

export interface HealthCheck {
  status: "healthy" | "degraded" | "down";
  version: string;
  uptime: number;
  chains: { chain: string; status: ConnectorStatus; blockHeight: number }[];
}
