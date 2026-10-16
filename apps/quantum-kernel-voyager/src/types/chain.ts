/**
 * @module types/chain
 * Blockchain-related TypeScript types mirroring x3_operator Python enums and
 * the @atlas-sphere/ts-sdk type surface. These are the canonical types used
 * by all chain adapters, the IPC layer, and the UI.
 */

// ---------------------------------------------------------------------------
// Enums — mirrors of x3_operator.config
// ---------------------------------------------------------------------------

export const OperatorRole = {
  VALIDATOR: "validator",
  GPU: "gpu",
  STORAGE: "storage",
  RELAYER: "relayer",
} as const;
export type OperatorRole = (typeof OperatorRole)[keyof typeof OperatorRole];

export const NetworkPhase = {
  DEVNET: "devnet",
  TESTNET: "testnet",
  MAINNET: "mainnet",
} as const;
export type NetworkPhase = (typeof NetworkPhase)[keyof typeof NetworkPhase];

export const BondStatus = {
  UNBONDED: "unbonded",
  PENDING: "pending",
  BONDED: "bonded",
  UNBONDING: "unbonding",
  SLASHED: "slashed",
} as const;
export type BondStatus = (typeof BondStatus)[keyof typeof BondStatus];

export const FaultType = {
  DOWNTIME: "downtime",
  EQUIVOCATION: "equivocation",
  INVALID_PROOF: "invalid_proof",
  MISSED_HEARTBEAT: "missed_heartbeat",
  SLA_VIOLATION: "sla_violation",
  DATA_CORRUPTION: "data_corruption",
  GOVERNANCE_ABUSE: "governance_abuse",
  AGENT_VIOLATION: "agent_violation",
} as const;
export type FaultType = (typeof FaultType)[keyof typeof FaultType];

export const DealStatus = {
  PROPOSED: "proposed",
  ACCEPTED: "accepted",
  ACTIVE: "active",
  EXPIRED: "expired",
  FAULTED: "faulted",
} as const;
export type DealStatus = (typeof DealStatus)[keyof typeof DealStatus];

export const AgentState = {
  REGISTERED: "registered",
  RUNNING: "running",
  PAUSED: "paused",
  KILLED: "killed",
} as const;
export type AgentState = (typeof AgentState)[keyof typeof AgentState];

// ---------------------------------------------------------------------------
// Chain adapter types
// ---------------------------------------------------------------------------

export interface ChainConfig {
  readonly rpcUrl: string;
  readonly chainId: string;
  readonly networkPhase: NetworkPhase | string;
  readonly label?: string;
}

export type SyncState = "synced" | "syncing" | "stalled" | "offline";

export interface ChainStatus {
  readonly connected: boolean;
  readonly blockHeight: number;
  readonly peerCount: number;
  readonly syncState: SyncState;
  readonly latencyMs: number;
}

export interface Block {
  readonly hash: string;
  readonly parentHash: string;
  readonly height: number;
  readonly timestamp: number;
  readonly extrinsicCount: number;
  transactions: Transaction[];
}

export interface Transaction {
  readonly hash: string;
  readonly from: string;
  readonly to: string;
  readonly method: string;
  readonly args: unknown;
  status: "pending" | "included" | "finalized" | "failed";
  blockHeight: number;
  readonly timestamp: number;
}

export interface ChainEvent {
  readonly id: string;
  readonly chainId: string;
  readonly blockHeight: number;
  readonly type: string;
  readonly data: Record<string, unknown> | Block;
  readonly timestamp: number;
}

export interface EventFilter {
  readonly module?: string;
  readonly method?: string;
}

export interface SigningIntent {
  readonly chainId: string;
  readonly accountId?: string;
  readonly from?: string;
  readonly to?: string;
  readonly module: string;
  readonly method: string;
  readonly args: unknown;
  readonly tip?: string;
}

export interface FeeEstimate {
  readonly estimatedFee: string;
  readonly currency: string;
  readonly confidence: number;
}

export type TransactionHash = string;
export type Unsubscribe = () => void;

/**
 * Chain adapter interface — multi-chain abstraction.
 * Every concrete adapter (AtlasSphere, Ethereum, LocalDev) implements this.
 */
export interface ChainAdapter {
  connect(config?: ChainConfig): Promise<void>;
  disconnect(): Promise<void>;
  getStatus(): Promise<ChainStatus>;
  getBlock(height: number): Promise<Block>;
  getLatestBlocks(count: number): Promise<Block[]>;
  submitTransaction(intent: SigningIntent): Promise<string>;
  estimateFee(intent: SigningIntent): Promise<FeeEstimate>;
  getAccount(address: string): Promise<AccountInfo>;
  subscribe(filter: EventFilter, callback: (event: ChainEvent) => void): string;
  unsubscribe(subscriptionId: string): void;
}

// ---------------------------------------------------------------------------
// Account / keystore types
// ---------------------------------------------------------------------------

export const ChainType = {
  SUBSTRATE: "substrate",
  EVM: "evm",
} as const;
export type ChainType = (typeof ChainType)[keyof typeof ChainType];

export interface AccountInfo {
  readonly address: string;
  readonly balance: string;
  readonly nonce: number;
  readonly staked: string;
}

// ---------------------------------------------------------------------------
// Operator types — mirrors x3_operator Python models
// ---------------------------------------------------------------------------

export interface OperatorIdentity {
  readonly operatorId: string;
  readonly pubkey: string;
  readonly hardwareFingerprint: string;
  readonly role: OperatorRole;
  readonly createdAt: number;
}

export interface BondRecord {
  readonly operatorId: string;
  readonly role: OperatorRole;
  readonly amount: number;
  readonly status: BondStatus;
  readonly bondedAt: number;
  readonly slashTotal: number;
  readonly txHash: string;
  readonly effectiveStake: number;
}

export interface SlashEvidence {
  readonly faultType: FaultType;
  readonly operatorId: string;
  readonly blockNumber: number;
  readonly timestamp: number;
  readonly description: string;
  readonly reporterId: string;
}

export interface SlashVerdict {
  readonly slashAmount: number;
  readonly faultType: FaultType;
  readonly operatorId: string;
  readonly verdictHash: string;
}

// ---------------------------------------------------------------------------
// IPC error type
// ---------------------------------------------------------------------------

export const AppErrorKind = {
  CHAIN: "ChainError",
  KEYSTORE: "KeystoreError",
  IO: "IoError",
  SERIALIZATION: "SerializationError",
  VALIDATION: "ValidationError",
  RATE_LIMIT: "RateLimitError",
  NOT_FOUND: "NotFound",
  UNAUTHORIZED: "Unauthorized",
} as const;
export type AppErrorKind = (typeof AppErrorKind)[keyof typeof AppErrorKind];

export interface AppError {
  readonly kind: AppErrorKind;
  readonly message: string;
}
