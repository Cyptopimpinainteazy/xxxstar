/**
 * WorldSimViewer — displays the canonical world state, prediction market,
 * accuracy scoreboard, and epoch history.
 */
import React, { memo, useState } from "react";
import { useTauriPolling } from "@/hooks/useTauriPolling";

interface DomainSummary {
  domain: string;
  entity_count: number;
  avg_confidence: number;
}

interface AgentRanking {
  agent_id: string;
  avg_accuracy: number;
  total_predictions: number;
}

interface WorldSimData {
  epoch: number;
  integrity_hash: string;
  domains: DomainSummary[];
  rankings: AgentRanking[];
  pending_predictions: number;
  global_metrics: Record<string, number>;
}

const WorldSimViewer: React.FC = memo(() => {
  const { data, loading, error } = useTauriPolling<WorldSimData>(
    "get_world_sim",
    3000,
  );
  const [tab, setTab] = useState<"state" | "rankings" | "metrics">("state");

  if (loading && !data)
    return (
      <div className="flex items-center justify-center h-full text-xs text-gray-500 font-mono">
        Loading world state…
      </div>
    );
  if (error)
    return (
      <div className="p-3 text-xs text-red-400 font-mono">Error: {error}</div>
    );
  if (!data) return null;

  return (
    <div className="w-full h-full overflow-auto bg-[#0a0a0f] p-3 font-mono text-[11px]">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="text-gray-300">
          Epoch <span className="text-[#ff6b35]">{data.epoch}</span>
          <span className="text-gray-600 ml-2 text-[9px]">
            {data.integrity_hash.slice(0, 12)}…
          </span>
        </div>
        <span className="text-gray-600 text-[9px]">
          {data.pending_predictions} pending predictions
        </span>
      </div>

      {/* Tab bar */}
      <div className="flex gap-1 mb-3">
        {(["state", "rankings", "metrics"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`px-2 py-0.5 rounded text-[10px] uppercase tracking-wider ${
              tab === t
                ? "bg-[#ff6b35]/20 text-[#ff6b35]"
                : "text-gray-600 hover:text-gray-400"
            }`}
          >
            {t}
          </button>
        ))}
      </div>

      {/* Domains */}
      {tab === "state" && (
        <div className="space-y-1">
          {data.domains.map((d) => (
            <div
              key={d.domain}
              className="flex items-center justify-between px-2 py-1.5 bg-gray-900/50 rounded"
            >
              <span className="text-gray-300">{d.domain}</span>
              <span className="text-gray-500">
                {d.entity_count} entities
              </span>
              <div className="w-16 h-1.5 bg-gray-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-cyan-500 rounded-full"
                  style={{ width: `${d.avg_confidence * 100}%` }}
                />
              </div>
            </div>
          ))}
          {data.domains.length === 0 && (
            <div className="text-gray-600 text-center py-4">No domains populated</div>
          )}
        </div>
      )}

      {/* Agent rankings */}
      {tab === "rankings" && (
        <div className="space-y-1">
          {data.rankings.map((r, i) => (
            <div
              key={r.agent_id}
              className="flex items-center justify-between px-2 py-1 bg-gray-900/50 rounded"
            >
              <span className="text-gray-500 w-5">#{i + 1}</span>
              <span className="text-gray-300 flex-1 truncate">
                {r.agent_id}
              </span>
              <span
                className={
                  r.avg_accuracy > 0.5
                    ? "text-green-400"
                    : r.avg_accuracy > 0.3
                      ? "text-yellow-400"
                      : "text-red-400"
                }
              >
                {(r.avg_accuracy * 100).toFixed(1)}%
              </span>
              <span className="text-gray-600 text-[9px] ml-2">
                ({r.total_predictions})
              </span>
            </div>
          ))}
          {data.rankings.length === 0 && (
            <div className="text-gray-600 text-center py-4">No predictions resolved yet</div>
          )}
        </div>
      )}

      {/* Global metrics */}
      {tab === "metrics" && (
        <div className="space-y-1">
          {Object.entries(data.global_metrics).map(([key, val]) => (
            <div
              key={key}
              className="flex justify-between px-2 py-1 bg-gray-900/50 rounded"
            >
              <span className="text-gray-400">{key}</span>
              <span className="text-gray-300">{val.toFixed(4)}</span>
            </div>
          ))}
          {Object.keys(data.global_metrics).length === 0 && (
            <div className="text-gray-600 text-center py-4">No global metrics</div>
          )}
        </div>
      )}
    </div>
  );
});

WorldSimViewer.displayName = "WorldSimViewer";

export default WorldSimViewer;
