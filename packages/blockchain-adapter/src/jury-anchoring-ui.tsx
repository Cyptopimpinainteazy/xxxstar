/**
 * Jury Decision UI Components — React hooks and display components
 * for jury decision monitoring in X3 Desktop.
 */

import React, { useEffect, useRef, useState } from "react";
import type {
  JuryAnchoring,
  JuryDecisionStatus,
  FormattedStatus,
} from "./jury-anchoring";

/* ------------------------------------------------------------------ */
/*  Hook                                                               */
/* ------------------------------------------------------------------ */

export function useJuryDecisionStatus(
  sessionId: string,
  juryAnchor: JuryAnchoring,
): {
  status: JuryDecisionStatus | null;
  isLoading: boolean;
  error: string | null;
} {
  const [status, setStatus] = useState<JuryDecisionStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    const poll = async (): Promise<void> => {
      try {
        const result = await juryAnchor.getDecisionStatus(sessionId);
        if (!mounted.current) return;
        setStatus(result);
        setError(null);

        if (result.status === "anchored" && pollInterval) {
          clearInterval(pollInterval);
          pollInterval = null;
        }
      } catch (err) {
        if (!mounted.current) return;
        setError(
          err instanceof Error ? err.message : "Unknown error",
        );
      }
    };

    (async () => {
      setIsLoading(true);
      await poll();
      if (mounted.current) setIsLoading(false);
    })();

    pollInterval = setInterval(poll, 2_000);

    return () => {
      mounted.current = false;
      if (pollInterval) clearInterval(pollInterval);
    };
  }, [sessionId, juryAnchor]);

  return { status, isLoading, error };
}

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */

interface JuryDecisionCardProps {
  sessionId: string;
  decisionHash: string;
  juryAnchor: JuryAnchoring;
}

export const JuryDecisionCard: React.FC<JuryDecisionCardProps> = React.memo(
  function JuryDecisionCard({ sessionId, decisionHash, juryAnchor }) {
    const { status, isLoading, error } = useJuryDecisionStatus(
      sessionId,
      juryAnchor,
    );

    const isVerified =
      status?.status === "anchored" &&
      status.on_chain?.decision_hash === decisionHash;

    const statusDisplay: FormattedStatus | null = status
      ? juryAnchor.formatStatus(status)
      : null;

    return (
      <div
        className="border border-gray-700 rounded-lg p-4 bg-[#0a0a0f] font-mono text-xs"
        role="article"
        aria-label={`Jury decision ${sessionId.slice(0, 8)}`}
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-3 pb-2 border-b border-gray-800">
          <span className="text-[#ff6b35] font-bold">
            Decision #{sessionId.slice(0, 8)}
          </span>
          <code className="text-gray-600 text-[9px]">
            {decisionHash.slice(0, 16)}…
          </code>
        </div>

        {/* Body */}
        <div className="space-y-2">
          {isLoading && (
            <div className="flex items-center gap-2 text-gray-500">
              <div className="w-3 h-3 border-2 border-gray-600 border-t-[#ff6b35] rounded-full animate-spin" />
              Loading…
            </div>
          )}

          {error && (
            <div className="text-red-400" role="alert">
              Error: {error}
            </div>
          )}

          {statusDisplay && !isLoading && (
            <div
              className={
                statusDisplay.color === "success"
                  ? "text-green-400"
                  : statusDisplay.color === "pending"
                    ? "text-yellow-400"
                    : "text-red-400"
              }
            >
              {statusDisplay.text}
              {statusDisplay.block != null && (
                <span className="text-gray-500 ml-2">
                  Block #{statusDisplay.block}
                </span>
              )}
              {isVerified && (
                <span className="text-green-500 ml-2 font-bold">
                  ✓ Hash verified
                </span>
              )}
            </div>
          )}
        </div>

        {/* Details */}
        {status?.on_chain && (
          <div className="mt-3 pt-2 border-t border-gray-800 space-y-1 text-gray-500">
            <div className="flex justify-between">
              <span>Block Hash</span>
              <code className="text-gray-400 text-[9px]">
                {status.on_chain.block_hash}
              </code>
            </div>
            <div className="flex justify-between">
              <span>Members</span>
              <span className="text-gray-300">
                {status.on_chain.metadata.member_count}
              </span>
            </div>
            <div className="flex justify-between">
              <span>Quorum</span>
              <span className="text-gray-300">
                {status.on_chain.metadata.quorum_threshold}%
              </span>
            </div>
            <div className="flex justify-between">
              <span>Result</span>
              <span
                className={
                  status.on_chain.metadata.result
                    ? "text-green-400"
                    : "text-red-400"
                }
              >
                {status.on_chain.metadata.result ? "PASS" : "FAIL"}
              </span>
            </div>
          </div>
        )}
      </div>
    );
  },
);
