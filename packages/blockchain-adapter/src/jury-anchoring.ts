/**
 * Jury Blockchain Anchoring — TypeScript Adapter
 *
 * Bridges off-chain jury decisions to on-chain anchoring.
 * Pure TypeScript — no React, no JSX. UI components live in
 * jury-anchoring-ui.tsx.
 */

import type { RpcClient } from "./rpc-client";

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

export interface JuryDecisionMetadata {
  member_count: number;
  quorum_threshold: number;
  result: boolean;
  session_duration_secs: number;
}

export interface OnChainRecord {
  block_number: number;
  block_hash: string;
  decision_hash: string;
  timestamp: number;
  jury_authority: string;
  metadata: JuryDecisionMetadata;
}

export interface OffChainRecord {
  decision_hash: string;
  audit_entry_count: number;
}

export interface JuryDecisionStatus {
  session_id: string;
  on_chain?: OnChainRecord;
  off_chain?: OffChainRecord;
  status: "anchored" | "pending" | "not_found";
}

export interface FormattedStatus {
  text: string;
  color: "success" | "pending" | "error";
  block?: number;
}

/* ------------------------------------------------------------------ */
/*  Constants                                                          */
/* ------------------------------------------------------------------ */

const DEFAULT_MAX_WAIT_MS = 30_000;
const DEFAULT_POLL_MS = 2_000;
const MAX_RETRIES = 3;
const RETRY_BASE_MS = 1_000;

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function withRetry<T>(
  fn: () => Promise<T>,
  retries: number = MAX_RETRIES,
): Promise<T> {
  let lastError: Error | undefined;
  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      return await fn();
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err));
      if (attempt < retries) {
        const delay = RETRY_BASE_MS * Math.pow(2, attempt);
        await sleep(delay);
      }
    }
  }
  throw lastError;
}

/* ------------------------------------------------------------------ */
/*  JuryAnchoring                                                      */
/* ------------------------------------------------------------------ */

export class JuryAnchoring {
  constructor(private readonly rpc: RpcClient) {}

  /**
   * Fetch on-chain decision status.
   */
  async getDecisionStatus(sessionId: string): Promise<JuryDecisionStatus> {
    try {
      return await withRetry(() =>
        this.rpc.call<JuryDecisionStatus>("jury_decisionStatus", [sessionId]),
      );
    } catch (error) {
      console.error(
        `Failed to get decision status for ${sessionId}:`,
        error,
      );
      return { session_id: sessionId, status: "not_found" };
    }
  }

  /**
   * Poll until the decision is anchored or the timeout expires.
   */
  async waitForAnchor(
    sessionId: string,
    maxWaitMs: number = DEFAULT_MAX_WAIT_MS,
    pollIntervalMs: number = DEFAULT_POLL_MS,
  ): Promise<JuryDecisionStatus | null> {
    const deadline = Date.now() + maxWaitMs;

    while (Date.now() < deadline) {
      const status = await this.getDecisionStatus(sessionId);
      if (status.status === "anchored") {
        return status;
      }
      await sleep(pollIntervalMs);
    }

    console.warn(
      `Decision ${sessionId} not anchored after ${maxWaitMs}ms`,
    );
    return null;
  }

  /**
   * Verify that the on-chain hash matches an expected hash.
   */
  async verifyDecision(
    sessionId: string,
    expectedHash: string,
  ): Promise<boolean> {
    const status = await this.getDecisionStatus(sessionId);

    if (status.status !== "anchored" || !status.on_chain) {
      return false;
    }

    const normalise = (h: string): string =>
      h.toLowerCase().replace(/^0x/, "");

    return (
      normalise(status.on_chain.decision_hash) === normalise(expectedHash)
    );
  }

  /**
   * Retrieve decisions by jury authority address.
   */
  async getDecisionsByAuthority(
    authority: string,
    limit: number = 100,
  ): Promise<JuryDecisionStatus[]> {
    try {
      const results: JuryDecisionStatus[] = await withRetry(() =>
        this.rpc.call<JuryDecisionStatus[]>("jury_decisionsByAuthority", [
          authority,
          limit,
        ]),
      );
      return results ?? [];
    } catch (error) {
      console.error("Failed to fetch decisions by authority:", error);
      return [];
    }
  }

  /**
   * Format a decision status for display.
   */
  formatStatus(status: JuryDecisionStatus): FormattedStatus {
    switch (status.status) {
      case "anchored": {
        const block = status.on_chain?.block_number ?? 0;
        return {
          text: `Verified on chain (Block #${block})`,
          color: "success",
          block,
        };
      }
      case "pending":
        return { text: "Waiting for blockchain anchor…", color: "pending" };
      case "not_found":
      default:
        return { text: "Not found on blockchain", color: "error" };
    }
  }
}
