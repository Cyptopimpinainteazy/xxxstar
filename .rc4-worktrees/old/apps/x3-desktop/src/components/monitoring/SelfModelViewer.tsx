/**
 * SelfModelViewer — Displays an agent's Self-Model Ledger.
 *
 * Shows: past events (with decay), capabilities, constraints,
 * future projections, mortality assessment, integrity hash.
 */
import React, { memo, useState } from "react";
import { useTauriPolling } from "@/hooks/useTauriPolling";

interface CausalEvent {
  event_id: string;
  timestamp: string;
  action_taken: string;
  outcome: string;
  decay_score: number;
}

interface Capability {
  capability_id: string;
  domain: string;
  proficiency_score: number;
  failure_rate_30d: number;
}

interface Projection {
  projection_id: string;
  time_horizon_seconds: number;
  confidence_score: number;
  predicted_failure_modes: Array<{ mode: string; probability: number }>;
}

interface SelfModelData {
  agent_id: string;
  version: number;
  is_alive: boolean;
  integrity_hash: string;
  past: CausalEvent[];
  present_capabilities: Capability[];
  present_constraints: {
    resource_budget_remaining: number;
    max_concurrent_tasks: number;
  };
  future_projections: Projection[];
}

const DecayBar: React.FC<{ score: number }> = ({ score }) => (
  <div className="w-16 h-1.5 bg-gray-800 rounded-full overflow-hidden">
    <div
      className="h-full rounded-full transition-all"
      style={{
        width: `${score * 100}%`,
        backgroundColor:
          score > 0.5 ? "#22c55e" : score > 0.2 ? "#eab308" : "#ef4444",
      }}
    />
  </div>
);

const SelfModelViewer: React.FC = memo(function SelfModelViewer() {
  const { data, loading, error } = useTauriPolling<SelfModelData>(
    "get_self_model",
    3000,
  );
  const [tab, setTab] = useState<"past" | "present" | "future">("present");

  if (loading && !data) {
    return (
      <div
        className="flex items-center justify-center h-full bg-[#0a0a0f] text-[#666] font-mono text-xs"
        role="status"
        aria-label="Loading self-model"
      >
        Loading self-model…
      </div>
    );
  }

  if (error) {
    return (
      <div
        className="flex items-center justify-center h-full bg-[#0a0a0f] text-red-400 font-mono text-xs"
        role="alert"
      >
        {error}
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="h-full overflow-auto bg-[#0a0a0f] p-3 font-mono text-xs text-gray-300">
      {/* Header */}
      <div className="flex items-center justify-between mb-3 pb-2 border-b border-gray-800">
        <div className="flex items-center gap-2">
          <div
            className={`w-2 h-2 rounded-full ${data.is_alive ? "bg-green-500 animate-pulse" : "bg-red-600"}`}
            aria-label={data.is_alive ? "Agent alive" : "Agent dead"}
          />
          <span className="text-[#ff6b35] font-bold">{data.agent_id}</span>
          <span className="text-gray-600">v{data.version}</span>
        </div>
        <span className="text-gray-700 text-[9px]">
          {data.integrity_hash?.slice(0, 16)}…
        </span>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 mb-3" role="tablist" aria-label="Self-model views">
        {(["past", "present", "future"] as const).map((t) => (
          <button
            key={t}
            role="tab"
            aria-selected={tab === t}
            onClick={() => setTab(t)}
            className={`px-2 py-1 rounded text-[10px] uppercase tracking-wider ${
              tab === t
                ? "bg-[#ff6b35]/20 text-[#ff6b35]"
                : "text-gray-600 hover:text-gray-400"
            }`}
          >
            {t}
          </button>
        ))}
      </div>

      {/* Past — Causal Events */}
      {tab === "past" && (
        <div className="space-y-1" role="tabpanel" aria-label="Past events">
          {data.past.length === 0 && (
            <div className="text-gray-600">No events recorded.</div>
          )}
          {data.past
            .slice(-20)
            .reverse()
            .map((e) => (
              <div key={e.event_id} className="flex items-center gap-2 py-0.5">
                <span
                  className={`text-[10px] ${
                    e.outcome === "SUCCESS" ? "text-green-500" : "text-red-400"
                  }`}
                >
                  {e.outcome}
                </span>
                <span className="text-gray-500 flex-1 truncate">
                  {e.action_taken}
                </span>
                <DecayBar score={e.decay_score} />
              </div>
            ))}
        </div>
      )}

      {/* Present — Capabilities & Constraints */}
      {tab === "present" && (
        <div className="space-y-3" role="tabpanel" aria-label="Present state">
          <div>
            <div className="text-gray-600 mb-1">Capabilities</div>
            {data.present_capabilities.map((c) => (
              <div
                key={c.capability_id}
                className="flex items-center justify-between py-0.5"
              >
                <span className="text-gray-400">
                  {c.domain}/{c.capability_id}
                </span>
                <div className="flex items-center gap-2">
                  <span className="text-[#ff6b35]">
                    {(c.proficiency_score * 100).toFixed(0)}%
                  </span>
                  <span className="text-gray-700">
                    fail:{(c.failure_rate_30d * 100).toFixed(0)}%
                  </span>
                </div>
              </div>
            ))}
          </div>
          <div>
            <div className="text-gray-600 mb-1">Constraints</div>
            <div className="text-gray-400">
              Budget:{" "}
              {data.present_constraints.resource_budget_remaining.toFixed(1)}
            </div>
            <div className="text-gray-400">
              Max tasks: {data.present_constraints.max_concurrent_tasks}
            </div>
          </div>
        </div>
      )}

      {/* Future — Projections */}
      {tab === "future" && (
        <div className="space-y-2" role="tabpanel" aria-label="Future projections">
          {data.future_projections.length === 0 && (
            <div className="text-gray-600">No projections.</div>
          )}
          {data.future_projections.map((p) => (
            <div
              key={p.projection_id}
              className="border border-gray-800 rounded p-2"
            >
              <div className="flex justify-between mb-1">
                <span className="text-gray-400">
                  Horizon: {p.time_horizon_seconds}s
                </span>
                <span className="text-[#ff6b35]">
                  Confidence: {(p.confidence_score * 100).toFixed(0)}%
                </span>
              </div>
              {p.predicted_failure_modes.map((fm, i) => (
                <div key={i} className="text-gray-500 text-[10px]">
                  {fm.mode}: {(fm.probability * 100).toFixed(0)}%
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
});

export default SelfModelViewer;
