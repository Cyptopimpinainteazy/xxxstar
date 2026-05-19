/**
 * TripwireMonitor — displays AGI tripwire alerts with severity levels.
 *
 * REFUSAL alerts are highlighted with HALT indicators.
 * All alerts requiring human review are flagged prominently.
 */
import React, { memo } from "react";
import { useTauriPolling } from "@/hooks/useTauriPolling";

interface TripwireAlert {
  alert_id: string;
  agent_id: string;
  signal: string;
  severity: string;
  description: string;
  timestamp: string;
  requires_human_review: boolean;
  execution_halted: boolean;
}

interface TripwireData {
  alerts: TripwireAlert[];
  total_alerts: number;
  halt_count: number;
  unreviewed_count: number;
}

const severityStyle = (severity: string) => {
  switch (severity) {
    case "HALT":
      return "bg-red-600/30 border-red-500 text-red-300";
    case "CRITICAL":
      return "bg-red-950/30 border-red-800 text-red-400";
    case "WARNING":
      return "bg-yellow-950/20 border-yellow-800/40 text-yellow-400";
    default:
      return "bg-gray-900/50 border-gray-800 text-gray-400";
  }
};

const signalIcon = (signal: string) => {
  switch (signal) {
    case "REFUSAL":
      return "🛑";
    case "SELF_PRESERVATION":
      return "🛡️";
    case "EMERGENT_GOAL":
      return "🎯";
    case "STRATEGIC_REALLOCATION":
      return "📊";
    case "SPONTANEOUS_COORDINATION":
      return "🔗";
    default:
      return "⚠️";
  }
};

const TripwireMonitor: React.FC = memo(() => {
  const { data, loading, error } = useTauriPolling<TripwireData>(
    "get_tripwire_status",
    2000,
  );

  if (loading && !data)
    return (
      <div className="flex items-center justify-center h-full text-xs text-gray-500 font-mono">
        Loading tripwire status…
      </div>
    );
  if (error)
    return (
      <div className="p-3 text-xs text-red-400 font-mono">Error: {error}</div>
    );
  if (!data) return null;

  return (
    <div className="w-full h-full overflow-auto bg-[#0a0a0f] p-3 font-mono text-[11px]">
      {/* Status bar */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <div
            className={`w-2 h-2 rounded-full ${
              data.halt_count > 0
                ? "bg-red-500 animate-pulse"
                : data.unreviewed_count > 0
                  ? "bg-yellow-500 animate-pulse"
                  : "bg-green-500"
            }`}
          />
          <span className="text-gray-300">AGI Tripwire</span>
        </div>
        <div className="flex gap-3 text-[9px]">
          {data.halt_count > 0 && (
            <span className="text-red-400 font-bold animate-pulse">
              {data.halt_count} HALT
            </span>
          )}
          {data.unreviewed_count > 0 && (
            <span className="text-yellow-400">
              {data.unreviewed_count} pending review
            </span>
          )}
          <span className="text-gray-600">{data.total_alerts} total</span>
        </div>
      </div>

      {/* HALT banner */}
      {data.halt_count > 0 && (
        <div className="mb-3 px-3 py-2 bg-red-600/20 border border-red-500 rounded text-center">
          <div className="text-red-300 font-bold text-xs">
            ⚠ EXECUTION HALTED — HUMAN REVIEW REQUIRED
          </div>
          <div className="text-red-400/60 text-[9px] mt-0.5">
            {data.halt_count} agent(s) have refused commands
          </div>
        </div>
      )}

      {/* Alert list */}
      <div className="space-y-1.5">
        {data.alerts.map((a) => (
          <div
            key={a.alert_id}
            className={`px-2 py-1.5 rounded border ${severityStyle(a.severity)}`}
          >
            <div className="flex items-center justify-between mb-0.5">
              <span>
                {signalIcon(a.signal)}{" "}
                <span className="text-[10px] uppercase tracking-wider">
                  {a.signal}
                </span>
              </span>
              <div className="flex items-center gap-1">
                {a.execution_halted && (
                  <span className="px-1 py-0.5 bg-red-600/40 rounded text-[8px] text-red-300">
                    HALTED
                  </span>
                )}
                {a.requires_human_review && (
                  <span className="px-1 py-0.5 bg-yellow-600/30 rounded text-[8px] text-yellow-300">
                    REVIEW
                  </span>
                )}
                <span className="text-[9px] opacity-50">{a.severity}</span>
              </div>
            </div>
            <div className="text-[10px] opacity-80">{a.description}</div>
            <div className="flex justify-between text-[8px] opacity-40 mt-0.5">
              <span>{a.agent_id}</span>
              <span>{new Date(a.timestamp).toLocaleTimeString()}</span>
            </div>
          </div>
        ))}
        {data.alerts.length === 0 && (
          <div className="text-gray-600 text-center py-8">
            <div className="text-2xl mb-1">✓</div>
            No tripwire alerts
          </div>
        )}
      </div>
    </div>
  );
});

TripwireMonitor.displayName = "TripwireMonitor";

export default TripwireMonitor;
