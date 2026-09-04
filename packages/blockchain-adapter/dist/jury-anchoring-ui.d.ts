/**
 * Jury Decision UI Components — React hooks and display components
 * for jury decision monitoring in X3 Desktop.
 */
import React from "react";
import type { JuryAnchoring, JuryDecisionStatus } from "./jury-anchoring";
export declare function useJuryDecisionStatus(sessionId: string, juryAnchor: JuryAnchoring): {
    status: JuryDecisionStatus | null;
    isLoading: boolean;
    error: string | null;
};
interface JuryDecisionCardProps {
    sessionId: string;
    decisionHash: string;
    juryAnchor: JuryAnchoring;
}
export declare const JuryDecisionCard: React.FC<JuryDecisionCardProps>;
export {};
