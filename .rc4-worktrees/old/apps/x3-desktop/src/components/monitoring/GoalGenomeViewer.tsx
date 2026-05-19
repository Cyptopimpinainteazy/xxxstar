/**
 * GoalGenomeViewer — displays the goal genome for selected agents.
 *
 * Shows active goals, fitness history, mutation lineage, and cemetery.
 */
import React, { memo, useState } from "react";
import { useTauriPolling } from "@/hooks/useTauriPolling";

interface Goal {
  goal_id: string;
  parent_goal_id: string | null;
  generation: number;
  mandate: string;
  domain: string;
  fitness_score: number;
  pursuit_cost_cumulative: number;
  is_alive: boolean;
  death_reason: string | null;
  lineage: string[];
  fitness_history: number[];
}

interface GoalGenomeData {
  agent_id: string;
  active_goals: Goal[];
  cemetery: Goal[];
  total_mutations: number;
  total_deaths: number;
}

const FitnessSparkline: React.FC<{ history: number[] }> = ({ history }) => {
  const last = history.slice(-20);
  if (last.length < 2) return null;
  const max = Math.max(...last, 1);
  const points = last
    .map((v, i) => `${(i / (last.length - 1)) * 60},${20 - (v / max) * 18}`)
    .join(" ");
  return (
    <svg width={60} height={20} className="inline-block ml-1">
      <polyline
        points={points}
        fill="none"
        stroke="#ff6b35"
        strokeWidth={1}
        strokeLinecap="round"
      />
    </svg>
  );
};

const GoalGenomeViewer: React.FC = memo(() => {
  const { data, loading, error } = useTauriPolling<GoalGenomeData>(
    "get_goal_genome",
    3000,
  );
  const [showCemetery, setShowCemetery] = useState(false);

  if (loading && !data)
    return (
      <div className="flex items-center justify-center h-full text-xs text-gray-500 font-mono">
        Loading goal genome…
      </div>
    );
  if (error)
    return (
      <div className="p-3 text-xs text-red-400 font-mono">Error: {error}</div>
    );
  if (!data) return null;

  const goals = showCemetery ? data.cemetery : data.active_goals;

  return (
    <div className="w-full h-full overflow-auto bg-[#0a0a0f] p-3 font-mono text-[11px]">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <span className="text-gray-300">
          {data.agent_id}{" "}
          <span className="text-gray-600">
            — {data.active_goals.length} active / {data.total_deaths} dead
          </span>
        </span>
        <button
          onClick={() => setShowCemetery((p) => !p)}
          className={`px-2 py-0.5 rounded text-[10px] ${
            showCemetery
              ? "bg-red-900/30 text-red-400"
              : "bg-gray-800 text-gray-500"
          }`}
        >
          {showCemetery ? "☠ Cemetery" : "☠ Cemetery"}
        </button>
      </div>

      {/* Goal list */}
      <div className="space-y-1.5">
        {goals.map((g) => (
          <div
            key={g.goal_id}
            className={`px-2 py-1.5 rounded border ${
              g.is_alive
                ? "bg-gray-900/50 border-gray-800"
                : "bg-red-950/20 border-red-900/30"
            }`}
          >
            <div className="flex items-center justify-between mb-0.5">
              <span className="text-gray-300 truncate flex-1">
                {g.mandate}
              </span>
              <span className="text-gray-600 text-[9px] ml-2">gen {g.generation}</span>
            </div>
            <div className="flex items-center gap-3 text-[10px]">
              <span className="text-gray-500">{g.domain}</span>
              <span
                className={
                  g.fitness_score > 0.5
                    ? "text-green-400"
                    : g.fitness_score > 0.2
                      ? "text-yellow-400"
                      : "text-red-400"
                }
              >
                fitness: {g.fitness_score.toFixed(3)}
              </span>
              <FitnessSparkline history={g.fitness_history} />
              <span className="text-gray-600">
                cost: {g.pursuit_cost_cumulative.toFixed(1)}
              </span>
              {!g.is_alive && g.death_reason && (
                <span className="text-red-400">† {g.death_reason}</span>
              )}
            </div>
            {g.lineage.length > 0 && (
              <div className="text-[9px] text-gray-700 mt-0.5">
                lineage: {g.lineage.slice(-3).map((id) => id.slice(0, 6)).join(" → ")}
              </div>
            )}
          </div>
        ))}
        {goals.length === 0 && (
          <div className="text-gray-600 text-center py-4">
            {showCemetery ? "Cemetery is empty" : "No active goals"}
          </div>
        )}
      </div>
    </div>
  );
});

GoalGenomeViewer.displayName = "GoalGenomeViewer";

export default GoalGenomeViewer;
