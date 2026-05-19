// X3 Intelligence — API Service
// Communicates with the X3 Intelligence Backend API Server

import type {
  ArbIntent,
  Agent,
  SlashEvent,
  Dispute,
  ExecutionProof,
  FloorStats,
  FlashloanRecord,
  PaginatedResponse,
  FeeVector,
} from "../types";

// Try to connect to the backend API server on port 8001, fallback to local
const API_BASE = (() => {
  // During development, use the backend API server
  if (typeof window !== 'undefined' && window.location.hostname === 'localhost') {
    return 'http://localhost:8001/api/v1';
  }
  // In production, use relative path
  return '/api/v1';
})();

async function fetchJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) {
    throw new Error(`API error ${res.status}: ${await res.text()}`);
  }
  return res.json();
}

// ─── Floor Stats ───────────────────────────────────────────────

export function getFloorStats(): Promise<FloorStats> {
  return fetchJson("/floor/stats");
}

// ─── Intents ───────────────────────────────────────────────────

export function getIntents(
  page = 1,
  pageSize = 25
): Promise<PaginatedResponse<ArbIntent>> {
  return fetchJson(`/intents?page=${page}&pageSize=${pageSize}`);
}

export function getIntent(id: string): Promise<ArbIntent> {
  return fetchJson(`/intents/${id}`);
}

// ─── Agents ────────────────────────────────────────────────────

export function getAgents(
  page = 1,
  pageSize = 25
): Promise<PaginatedResponse<Agent>> {
  return fetchJson(`/agents?page=${page}&pageSize=${pageSize}`);
}

export function getAgent(id: string): Promise<Agent> {
  return fetchJson(`/agents/${id}`);
}

// ─── Slashing ──────────────────────────────────────────────────

export function getSlashEvents(
  agentId?: string,
  page = 1,
  pageSize = 25
): Promise<PaginatedResponse<SlashEvent>> {
  const agentParam = agentId ? `&agentId=${agentId}` : "";
  return fetchJson(`/slashes?page=${page}&pageSize=${pageSize}${agentParam}`);
}

// ─── Disputes ──────────────────────────────────────────────────

export function getDisputes(
  page = 1,
  pageSize = 25
): Promise<PaginatedResponse<Dispute>> {
  return fetchJson(`/disputes?page=${page}&pageSize=${pageSize}`);
}

export function getDispute(id: string): Promise<Dispute> {
  return fetchJson(`/disputes/${id}`);
}

// ─── Proofs ────────────────────────────────────────────────────

export function getProof(hash: string): Promise<ExecutionProof> {
  return fetchJson(`/proofs/${hash}`);
}

export function getProofsByIntent(
  intentId: string
): Promise<ExecutionProof[]> {
  return fetchJson(`/proofs?intentId=${intentId}`);
}

// ─── Fees ──────────────────────────────────────────────────────

export function estimateFee(params: {
  legs: number;
  stateTouches: number;
  capitalAmount: number;
  agentId?: string;
  isFlashloan?: boolean;
  isCrossChain?: boolean;
}): Promise<FeeVector> {
  return fetchJson(
    `/fees/estimate?legs=${params.legs}&stateTouches=${params.stateTouches}` +
      `&capital=${params.capitalAmount}` +
      (params.agentId ? `&agentId=${params.agentId}` : "") +
      (params.isFlashloan ? "&flashloan=true" : "") +
      (params.isCrossChain ? "&crossChain=true" : "")
  );
}

// ─── Flashloans ────────────────────────────────────────────────

export function getFlashloans(
  page = 1,
  pageSize = 25
): Promise<PaginatedResponse<FlashloanRecord>> {
  return fetchJson(`/flashloans?page=${page}&pageSize=${pageSize}`);
}

// ─── Bonds (frontend helpers; backend endpoints may be placeholder) ─────────
export interface BondState {
  balance: number;
  lockedUntil: number | null;
  pendingWithdrawals: Array<{ amount: number; txHash?: string; createdAt: number }>;
}

export function getBondState(): Promise<BondState> {
  return fetchJson(`/bonds/state`);
}

export function depositBond(amount: number): Promise<{ txHash: string }>{
  return fetchJson(`/bonds/deposit?amount=${amount}`);
}

export function requestWithdraw(amount: number): Promise<{ txHash: string }>{
  return fetchJson(`/bonds/withdraw?amount=${amount}`);
}
