/**
 * SelfImprovementViewer — displays improvement proposals, costs, scars,
 * and resource budget status.
 */
import React, { memo, useState } from "react";
import { useTauriPolling } from "@/hooks/useTauriPolling";

interface Proposal {
  proposal_id: string;
  improvement_type: string;
  target_capability: string;
  target_domain: string;
  estimated_cost: number;
  status: string;
  created_at: string;
}

interface Scar {
  scar_id: string;
  target_domain: string;
  target_capability: string;
  cost_paid: number;
  failure_reason: string;
  recorded_at: string;
}

interface SelfImprovementData {
  agent_id: string;
  resource_budget: number;
  proposals: Proposal[];
  scars: Scar[];
  total_cost_spent: number;
}

const statusColor = (status: string): string => {
  switch (status) {
    case "SUCCEEDED":
      return "text-green-400";
    case "FAILED":
      return "text-red-400";
    case "REJECTED_BUDGET":
    case "REJECTED_COOLDOWN":
      return "text-yellow-400";
    case "EXECUTING":
      return "text-blue-400";
    default:
      return "text-gray-400";
  }
};

const SelfImprovementViewer: React.FC = memo(() => {
  const { data, loading, error } = useTauriPolling<SelfImprovementData>(
    "get_self_improvement",
    3000,
  );
  const [showScars, setShowScars] = useState(false);

  if (loading && !data)
    return (
      <div className="flex items-center justify-center h-full text-xs text-gray-500 font-mono">
        Loading self-improvement…
      </div>
    );
  if (error)
    return (
      <div className="p-3 text-xs text-red-400 font-mono">Error: {error}</div>
    );
  if (!data) return null;

  const budgetPct = Math.max(0, Math.min(100, (data.resource_budget / (data.resource_budget + data.total_cost_spent)) * 100));

  return (
    <div className="w-full h-full overflow-auto bg-[#0a0a0f] p-3 font-mono text-[11px]">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <span className="text-gray-300">{data.agent_id}</span>
        <div className="flex items-center gap-2">
          <span className="text-gray-500">Budget:</span>
          <div className="w-24 h-1.5 bg-gray-800 rounded-full overflow-hidden">
            <div
              className="h-full rounded-full transition-all"
              style={{
                width: `${budgetPct}%`,
                backgroundColor:
                  budgetPct > 50 ? "#22c55e" : budgetPct > 20 ? "#eab308" : "#ef4444",
              }}
            />
          </div>
          <span className="text-gray-400">{data.resource_budget.toFixed(1)}</span>
        </div>
      </div>

      {/* Toggle */}
      <div className="flex gap-1 mb-3">
        <button
          onClick={() => setShowScars(false)}
          className={`px-2 py-0.5 rounded text-[10px] ${
            !showScars ? "bg-[#ff6b35]/20 text-[#ff6b35]" : "text-gray-600"
          }`}
        >
          PROPOSALS ({data.proposals.length})
        </button>
        <button
          onClick={() => setShowScars(true)}
          className={`px-2 py-0.5 rounded text-[10px] ${
            showScars ? "bg-red-900/30 text-red-400" : "text-gray-600"
          }`}
        >
          SCARS ({data.scars.length})
        </button>
      </div>

      {/* Proposals */}
      {!showScars && (
        <div className="space-y-1">
          {data.proposals.map((p) => (
            <div
              key={p.proposal_id}
              className="flex items-center justify-between px-2 py-1.5 bg-gray-900/50 rounded"
            >
              <div className="flex-1">
                <span className="text-gray-300">{p.target_capability}</span>
                <span className="text-gray-600 text-[9px] ml-1">
                  ({p.improvement_type})
                </span>
              </div>
              <span className="text-gray-500 mx-2">
                {p.estimated_cost.toFixed(1)}
              </span>
              <span className={statusColor(p.status)}>{p.status}</span>
            </div>
          ))}
          {data.proposals.length === 0 && (
            <div className="text-gray-600 text-center py-4">No proposals</div>
          )}
        </div>
      )}

      {/* Scars — PERMANENT, never deleted */}
      {showScars && (
        <div className="space-y-1">
          {data.scars.map((s) => (
            <div
              key={s.scar_id}
              className="px-2 py-1.5 bg-red-950/20 border border-red-900/30 rounded"
            >
              <div className="flex justify-between">
                <span className="text-red-300">{s.target_capability}</span>
                <span className="text-red-400">{s.cost_paid.toFixed(1)}</span>
              </div>
              <div className="text-[9px] text-red-400/60 mt-0.5">
                {s.target_domain} — {s.failure_reason}
              </div>
            </div>
          ))}
          {data.scars.length === 0 && (
            <div className="text-gray-600 text-center py-4">No scars (yet)</div>
          )}
        </div>
      )}
    </div>
  );
});

SelfImprovementViewer.displayName = "SelfImprovementViewer";

export default SelfImprovementViewer;
