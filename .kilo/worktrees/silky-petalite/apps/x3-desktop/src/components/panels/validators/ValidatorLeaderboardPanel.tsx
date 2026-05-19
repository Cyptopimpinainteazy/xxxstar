import React, { useState } from "react";
import { Trophy, TrendingUp, Zap, Shield, GitBranch, Cpu } from "lucide-react";
import clsx from "clsx";

interface ValidatorStats {
  id: string;
  name: string;
  rank: number;
  uptime: number;
  blocksProduced: number;
  gpuScore: number;
  reputation: number;
  stakes: number;
  rewards24h: number;
  status: "online" | "offline" | "at-risk";
}

const MOCK_VALIDATORS: ValidatorStats[] = [
  {
    id: "1",
    name: "ValidatorKing",
    rank: 1,
    uptime: 99.97,
    blocksProduced: 12847,
    gpuScore: 9850,
    reputation: 98,
    stakes: 2500,
    rewards24h: 45.2,
    status: "online",
  },
  {
    id: "2",
    name: "x3-supernova",
    rank: 2,
    uptime: 99.95,
    blocksProduced: 12654,
    gpuScore: 9720,
    reputation: 95,
    stakes: 1800,
    rewards24h: 38.6,
    status: "online",
  },
  {
    id: "3",
    name: "lightning-node",
    rank: 3,
    uptime: 99.88,
    blocksProduced: 12421,
    gpuScore: 9540,
    reputation: 92,
    stakes: 1500,
    rewards24h: 32.1,
    status: "online",
  },
  {
    id: "4",
    name: "slow-turtle",
    rank: 4,
    uptime: 98.5,
    blocksProduced: 10250,
    gpuScore: 7200,
    reputation: 78,
    stakes: 800,
    rewards24h: 12.8,
    status: "at-risk",
  },
  {
    id: "5",
    name: "offline-ghost",
    rank: 5,
    uptime: 45.2,
    blocksProduced: 3100,
    gpuScore: 2100,
    reputation: 22,
    stakes: 200,
    rewards24h: 0,
    status: "offline",
  },
];

export default function ValidatorLeaderboardPanel() {
  const [validators, setValidators] = useState<ValidatorStats[]>(MOCK_VALIDATORS);
  const [sortBy, setSortBy] = useState<"rank" | "uptime" | "blocks" | "gpu" | "reputation">("rank");
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const sortedValidators = [...validators].sort((a, b) => {
    switch (sortBy) {
      case "uptime":
        return b.uptime - a.uptime;
      case "blocks":
        return b.blocksProduced - a.blocksProduced;
      case "gpu":
        return b.gpuScore - a.gpuScore;
      case "reputation":
        return b.reputation - a.reputation;
      default:
        return a.rank - b.rank;
    }
  });

  const getStatusColor = (status: string) => {
    switch (status) {
      case "online":
        return "text-green-400 bg-green-500/20";
      case "at-risk":
        return "text-yellow-400 bg-yellow-500/20";
      case "offline":
        return "text-red-400 bg-red-500/20";
      default:
        return "text-gray-400 bg-gray-500/20";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">Validator Leaderboard</h2>

      {/* Stats Summary */}
      <div className="grid grid-cols-4 gap-3 mb-6">
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Active Validators</div>
          <div className="text-xl font-bold">{validators.filter(v => v.status === "online").length}</div>
        </div>
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Avg Uptime</div>
          <div className="text-xl font-bold">{(validators.reduce((sum, v) => sum + v.uptime, 0) / validators.length).toFixed(2)}%</div>
        </div>
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Total Blocks</div>
          <div className="text-xl font-bold">{(validators.reduce((sum, v) => sum + v.blocksProduced, 0) / 1000).toFixed(1)}K</div>
        </div>
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">24H Rewards</div>
          <div className="text-xl font-bold">${validators.reduce((sum, v) => sum + v.rewards24h, 0).toFixed(0)}</div>
        </div>
      </div>

      {/* Sort Buttons */}
      <div className="flex gap-2 mb-4 overflow-x-auto pb-2">
        {(["rank", "uptime", "blocks", "gpu", "reputation"] as const).map((sort) => (
          <button
            key={sort}
            onClick={() => setSortBy(sort)}
            className={clsx(
              "flex-shrink-0 px-3 py-1 rounded text-sm font-semibold transition",
              sortBy === sort
                ? "bg-blue-600 text-white"
                : "bg-[#15151b] text-gray-400 border border-[#2a2a35] hover:border-[#3a3a45]"
            )}
          >
            {sort === "rank" && "Rank"}
            {sort === "uptime" && "Uptime"}
            {sort === "blocks" && "Blocks"}
            {sort === "gpu" && "GPU Score"}
            {sort === "reputation" && "Reputation"}
          </button>
        ))}
      </div>

      {/* Leaderboard */}
      <div className="flex-1 overflow-y-auto">
        <div className="space-y-2">
          {sortedValidators.map((validator) => (
            <div key={validator.id}>
              <button
                onClick={() => setExpandedId(expandedId === validator.id ? null : validator.id)}
                className="w-full bg-[#15151b] p-4 rounded-lg border border-[#2a2a35] hover:border-[#3a3a45] transition text-left"
              >
                <div className="flex items-center justify-between gap-4">
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    <div className={clsx("flex items-center justify-center w-8 h-8 rounded-lg font-bold", validator.rank <= 3 ? "bg-yellow-500/20 text-yellow-400" : "bg-[#2a2a35] text-gray-400")}>
                      {validator.rank}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-semibold truncate">{validator.name}</span>
                        <span className={clsx("text-xs px-2 py-0.5 rounded-full font-semibold flex-shrink-0", getStatusColor(validator.status))}>
                          {validator.status === "online" && "●"}
                          {validator.status === "at-risk" && "◐"}
                          {validator.status === "offline" && "○"}
                        </span>
                      </div>
                      <div className="text-xs text-gray-500 mt-0.5">Reputation: {validator.reputation}%</div>
                    </div>
                  </div>

                  {/* Stats */}
                  <div className="flex items-center gap-3 text-sm text-gray-400">
                    <div className="text-right">
                      <div className="font-semibold text-white">{validator.uptime.toFixed(2)}%</div>
                      <div className="text-xs">uptime</div>
                    </div>
                    <div className="w-px h-8 bg-[#2a2a35]" />
                    <div className="text-right">
                      <div className="font-semibold text-white">{validator.blocksProduced}K</div>
                      <div className="text-xs">blocks</div>
                    </div>
                    <div className="w-px h-8 bg-[#2a2a35]" />
                    <div className="text-right">
                      <div className="font-semibold text-green-400">${validator.rewards24h.toFixed(1)}</div>
                      <div className="text-xs">24h</div>
                    </div>
                  </div>
                </div>
              </button>

              {/* Expanded Details */}
              {expandedId === validator.id && (
                <div className="bg-[#15151b] border border-[#2a2a35] border-t-0 rounded-b-lg p-4 space-y-3 text-sm">
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <div className="text-xs text-gray-400 mb-1">GPU Score</div>
                      <div className="flex items-center gap-2">
                        <Cpu size={14} className="text-blue-400" />
                        <span className="font-semibold">{validator.gpuScore}</span>
                      </div>
                    </div>
                    <div>
                      <div className="text-xs text-gray-400 mb-1">Total Stakes</div>
                      <div className="font-semibold">{validator.stakes} X3</div>
                    </div>
                    <div>
                      <div className="text-xs text-gray-400 mb-1">Blocks This Week</div>
                      <div className="font-semibold text-green-400">+{Math.floor(validator.blocksProduced * 0.15)}</div>
                    </div>
                    <div>
                      <div className="text-xs text-gray-400 mb-1">Avg Response Time</div>
                      <div className="font-semibold">{Math.floor(200 - validator.gpuScore / 50)}ms</div>
                    </div>
                  </div>

                  {/* Progress Bars */}
                  <div>
                    <div className="text-xs text-gray-400 mb-1">Uptime Trend</div>
                    <div className="w-full bg-[#2a2a35] rounded h-2">
                      <div className="bg-green-500 h-2 rounded" style={{width: `${validator.uptime}%`}} />
                    </div>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
