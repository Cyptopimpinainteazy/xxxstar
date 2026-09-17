/**
 * Jury Blockchain Anchoring — TypeScript Adapter
 *
 * Bridges off-chain jury decisions to on-chain anchoring.
 * Pure TypeScript — no React, no JSX. UI components live in
 * jury-anchoring-ui.tsx.
 */
/* ------------------------------------------------------------------ */
/*  Constants                                                          */
/* ------------------------------------------------------------------ */
const DEFAULT_MAX_WAIT_MS = 30000;
const DEFAULT_POLL_MS = 2000;
const MAX_RETRIES = 3;
const RETRY_BASE_MS = 1000;
/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */
function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
async function withRetry(fn, retries = MAX_RETRIES) {
    let lastError;
    for (let attempt = 0; attempt <= retries; attempt++) {
        try {
            return await fn();
        }
        catch (err) {
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
    constructor(rpc) {
        this.rpc = rpc;
    }
    /**
     * Fetch on-chain decision status.
     */
    async getDecisionStatus(sessionId) {
        try {
            return await withRetry(() => this.rpc.call("jury_decisionStatus", [sessionId]));
        }
        catch (error) {
            console.error(`Failed to get decision status for ${sessionId}:`, error);
            return { session_id: sessionId, status: "not_found" };
        }
    }
    /**
     * Poll until the decision is anchored or the timeout expires.
     */
    async waitForAnchor(sessionId, maxWaitMs = DEFAULT_MAX_WAIT_MS, pollIntervalMs = DEFAULT_POLL_MS) {
        const deadline = Date.now() + maxWaitMs;
        while (Date.now() < deadline) {
            const status = await this.getDecisionStatus(sessionId);
            if (status.status === "anchored") {
                return status;
            }
            await sleep(pollIntervalMs);
        }
        console.warn(`Decision ${sessionId} not anchored after ${maxWaitMs}ms`);
        return null;
    }
    /**
     * Verify that the on-chain hash matches an expected hash.
     */
    async verifyDecision(sessionId, expectedHash) {
        const status = await this.getDecisionStatus(sessionId);
        if (status.status !== "anchored" || !status.on_chain) {
            return false;
        }
        const normalise = (h) => h.toLowerCase().replace(/^0x/, "");
        return (normalise(status.on_chain.decision_hash) === normalise(expectedHash));
    }
    /**
     * Retrieve decisions by jury authority address.
     */
    async getDecisionsByAuthority(authority, limit = 100) {
        try {
            const results = await withRetry(() => this.rpc.call("jury_decisionsByAuthority", [
                authority,
                limit,
            ]));
            return results ?? [];
        }
        catch (error) {
            console.error("Failed to fetch decisions by authority:", error);
            return [];
        }
    }
    /**
     * Format a decision status for display.
     */
    formatStatus(status) {
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
