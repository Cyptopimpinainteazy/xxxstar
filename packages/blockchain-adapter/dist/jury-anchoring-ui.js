import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
/**
 * Jury Decision UI Components — React hooks and display components
 * for jury decision monitoring in X3 Desktop.
 */
import React, { useEffect, useRef, useState } from "react";
/* ------------------------------------------------------------------ */
/*  Hook                                                               */
/* ------------------------------------------------------------------ */
export function useJuryDecisionStatus(sessionId, juryAnchor) {
    const [status, setStatus] = useState(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState(null);
    const mounted = useRef(true);
    useEffect(() => {
        mounted.current = true;
        let pollInterval = null;
        const poll = async () => {
            try {
                const result = await juryAnchor.getDecisionStatus(sessionId);
                if (!mounted.current)
                    return;
                setStatus(result);
                setError(null);
                if (result.status === "anchored" && pollInterval) {
                    clearInterval(pollInterval);
                    pollInterval = null;
                }
            }
            catch (err) {
                if (!mounted.current)
                    return;
                setError(err instanceof Error ? err.message : "Unknown error");
            }
        };
        (async () => {
            setIsLoading(true);
            await poll();
            if (mounted.current)
                setIsLoading(false);
        })();
        pollInterval = setInterval(poll, 2000);
        return () => {
            mounted.current = false;
            if (pollInterval)
                clearInterval(pollInterval);
        };
    }, [sessionId, juryAnchor]);
    return { status, isLoading, error };
}
export const JuryDecisionCard = React.memo(function JuryDecisionCard({ sessionId, decisionHash, juryAnchor }) {
    const { status, isLoading, error } = useJuryDecisionStatus(sessionId, juryAnchor);
    const isVerified = status?.status === "anchored" &&
        status.on_chain?.decision_hash === decisionHash;
    const statusDisplay = status
        ? juryAnchor.formatStatus(status)
        : null;
    return (_jsxs("div", { className: "border border-gray-700 rounded-lg p-4 bg-[#0a0a0f] font-mono text-xs", role: "article", "aria-label": `Jury decision ${sessionId.slice(0, 8)}`, children: [_jsxs("div", { className: "flex items-center justify-between mb-3 pb-2 border-b border-gray-800", children: [_jsxs("span", { className: "text-[#ff6b35] font-bold", children: ["Decision #", sessionId.slice(0, 8)] }), _jsxs("code", { className: "text-gray-600 text-[9px]", children: [decisionHash.slice(0, 16), "\u2026"] })] }), _jsxs("div", { className: "space-y-2", children: [isLoading && (_jsxs("div", { className: "flex items-center gap-2 text-gray-500", children: [_jsx("div", { className: "w-3 h-3 border-2 border-gray-600 border-t-[#ff6b35] rounded-full animate-spin" }), "Loading\u2026"] })), error && (_jsxs("div", { className: "text-red-400", role: "alert", children: ["Error: ", error] })), statusDisplay && !isLoading && (_jsxs("div", { className: statusDisplay.color === "success"
                            ? "text-green-400"
                            : statusDisplay.color === "pending"
                                ? "text-yellow-400"
                                : "text-red-400", children: [statusDisplay.text, statusDisplay.block != null && (_jsxs("span", { className: "text-gray-500 ml-2", children: ["Block #", statusDisplay.block] })), isVerified && (_jsx("span", { className: "text-green-500 ml-2 font-bold", children: "\u2713 Hash verified" }))] }))] }), status?.on_chain && (_jsxs("div", { className: "mt-3 pt-2 border-t border-gray-800 space-y-1 text-gray-500", children: [_jsxs("div", { className: "flex justify-between", children: [_jsx("span", { children: "Block Hash" }), _jsx("code", { className: "text-gray-400 text-[9px]", children: status.on_chain.block_hash })] }), _jsxs("div", { className: "flex justify-between", children: [_jsx("span", { children: "Members" }), _jsx("span", { className: "text-gray-300", children: status.on_chain.metadata.member_count })] }), _jsxs("div", { className: "flex justify-between", children: [_jsx("span", { children: "Quorum" }), _jsxs("span", { className: "text-gray-300", children: [status.on_chain.metadata.quorum_threshold, "%"] })] }), _jsxs("div", { className: "flex justify-between", children: [_jsx("span", { children: "Result" }), _jsx("span", { className: status.on_chain.metadata.result
                                    ? "text-green-400"
                                    : "text-red-400", children: status.on_chain.metadata.result ? "PASS" : "FAIL" })] })] }))] }));
});
