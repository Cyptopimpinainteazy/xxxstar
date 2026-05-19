// X3 Intelligence — Domain Types
// Mirror of the Rust jurisdiction layer types for frontend consumption.

// ─── Identifiers ───────────────────────────────────────────────

export type IntentId = string;
export type AgentId = string;
export type DisputeId = string;
export type ProofHash = string;
export type FlashloanId = string;

// ─── Enums ─────────────────────────────────────────────────────

export enum IntentState {
  Submitted = "Submitted",
  RouteBound = "RouteBound",
  Executing = "Executing",
  Executed = "Executed",
  Finalized = "Finalized",
  Slashed = "Slashed",
  Cancelled = "Cancelled",
  Expired = "Expired",
}

export enum AgentStatus {
  Active = "Active",
  Suspended = "Suspended",
  Deregistered = "Deregistered",
  Deactivated = "Deactivated",
}

export enum SlashSeverity {
  Minor = "Minor",
  Moderate = "Moderate",
  Major = "Major",
  Critical = "Critical",
}

export enum DisputeState {
  Filed = "Filed",
  Replaying = "Replaying",
  Resolved = "Resolved",
  Dismissed = "Dismissed",
}

export enum VerdictOutcome {
  Guilty = "Guilty",
  NotGuilty = "NotGuilty",
  InvalidDispute = "InvalidDispute",
}

export enum ChainKind {
  Evm = "Evm",
  Svm = "Svm",
}

// ─── Domain Objects ────────────────────────────────────────────

export interface ArbIntent {
  id: IntentId;
  agentId: AgentId;
  state: IntentState;
  legs: RouteLeg[];
  feeCap: number;
  feeActual: number | null;
  createdAt: number;
  executedAt: number | null;
  proofHash: ProofHash | null;
}

export interface RouteLeg {
  chain: string;
  protocol: string;
  tokenIn: string;
  tokenOut: string;
  amountIn: string;
  expectedOut: string;
}

export interface Agent {
  id: AgentId;
  status: AgentStatus;
  bondAmount: number;
  reputation: number;
  successRate: number;
  totalExecutions: number;
  totalSlashes: number;
  registeredAt: number;
}

export interface SlashEvent {
  id: string;
  agentId: AgentId;
  severity: SlashSeverity;
  reason: string;
  amountSlashed: number;
  proofHash: ProofHash;
  timestamp: number;
}

export interface Dispute {
  id: DisputeId;
  state: DisputeState;
  filedBy: AgentId;
  against: AgentId;
  intentId: IntentId;
  verdict: VerdictOutcome | null;
  filedAt: number;
  resolvedAt: number | null;
}

export interface ExecutionProof {
  hash: ProofHash;
  intentId: IntentId;
  agentId: AgentId;
  blockNumber: number;
  stateDiffCount: number;
  timestamp: number;
  verified: boolean;
}

export interface FeeVector {
  baseFee: number;
  complexityFee: number;
  capitalFee: number;
  reputationDiscount: number;
  totalFee: number;
}

export interface FlashloanRecord {
  id: FlashloanId;
  chain: string;
  asset: string;
  principal: string;
  premium: string;
  status: "Settled" | "Reverted" | "Defaulted";
  proofHash: ProofHash;
  timestamp: number;
}

// ─── API Responses ─────────────────────────────────────────────

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
}

export interface FloorStats {
  activeAgents: number;
  totalIntents: number;
  totalVolume: string;
  totalSlashes: number;
  totalDisputes: number;
  avgSuccessRate: number;
  activeFlashloans: number;
}
