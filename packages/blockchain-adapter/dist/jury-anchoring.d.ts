/**
 * Jury Blockchain Anchoring — TypeScript Adapter
 *
 * Bridges off-chain jury decisions to on-chain anchoring.
 * Pure TypeScript — no React, no JSX. UI components live in
 * jury-anchoring-ui.tsx.
 */
import type { RpcClient } from "./rpc-client";
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
export declare class JuryAnchoring {
    private readonly rpc;
    constructor(rpc: RpcClient);
    /**
     * Fetch on-chain decision status.
     */
    getDecisionStatus(sessionId: string): Promise<JuryDecisionStatus>;
    /**
     * Poll until the decision is anchored or the timeout expires.
     */
    waitForAnchor(sessionId: string, maxWaitMs?: number, pollIntervalMs?: number): Promise<JuryDecisionStatus | null>;
    /**
     * Verify that the on-chain hash matches an expected hash.
     */
    verifyDecision(sessionId: string, expectedHash: string): Promise<boolean>;
    /**
     * Retrieve decisions by jury authority address.
     */
    getDecisionsByAuthority(authority: string, limit?: number): Promise<JuryDecisionStatus[]>;
    /**
     * Format a decision status for display.
     */
    formatStatus(status: JuryDecisionStatus): FormattedStatus;
}
