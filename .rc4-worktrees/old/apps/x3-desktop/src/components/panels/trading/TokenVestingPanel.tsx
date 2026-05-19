import React, { useState } from "react";
import { Unlock, Calendar, TrendingUp, Clock, DollarSign, CheckCircle } from "lucide-react";
import clsx from "clsx";

interface VestingSchedule {
  id: string;
  beneficiary: string;
  totalTokens: number;
  vestedTokens: number;
  cliffDate: string;
  startDate: string;
  endDate: string;
  vestingPeriod: "monthly" | "quarterly" | "linear";
  status: "pending" | "vesting" | "completed";
}

interface VestingMilestone {
  date: string;
  amount: number;
  percentage: number;
  status: "completed" | "upcoming";
}

const MOCK_SCHEDULES: VestingSchedule[] = [
  {
    id: "1",
    beneficiary: "Founder A",
    totalTokens: 1000000,
    vestedTokens: 750000,
    cliffDate: "2024-01-01",
    startDate: "2024-01-01",
    endDate: "2026-12-31",
    vestingPeriod: "linear",
    status: "vesting",
  },
  {
    id: "2",
    beneficiary: "Team Lead B",
    totalTokens: 500000,
    vestedTokens: 250000,
    cliffDate: "2024-06-01",
    startDate: "2024-06-01",
    endDate: "2028-06-01",
    vestingPeriod: "monthly",
    status: "vesting",
  },
  {
    id: "3",
    beneficiary: "Advisor C",
    totalTokens: 100000,
    vestedTokens: 100000,
    cliffDate: "2023-01-01",
    startDate: "2023-01-01",
    endDate: "2025-01-01",
    vestingPeriod: "quarterly",
    status: "completed",
  },
];

const generateMilestones = (schedule: VestingSchedule): VestingMilestone[] => {
  const milestones: VestingMilestone[] = [];
  const start = new Date(schedule.startDate);
  const end = new Date(schedule.endDate);
  const totalTime = end.getTime() - start.getTime();
  const now = new Date();

  if (schedule.vestingPeriod === "linear") {
    for (let i = 0; i <= 4; i++) {
      const date = new Date(start.getTime() + (totalTime / 4) * i);
      milestones.push({
        date: date.toISOString().split("T")[0],
        amount: (schedule.totalTokens / 4) * i,
        percentage: i * 25,
        status: date <= now ? "completed" : "upcoming",
      });
    }
  } else if (schedule.vestingPeriod === "monthly") {
    for (let i = 0; i <= 12; i++) {
      const date = new Date(start.getFullYear(), start.getMonth() + i, start.getDate());
      if (date <= end) {
        milestones.push({
          date: date.toISOString().split("T")[0],
          amount: (schedule.totalTokens / 12) * (i + 1),
          percentage: Math.min(100, ((i + 1) / 12) * 100),
          status: date <= now ? "completed" : "upcoming",
        });
      }
    }
  } else {
    for (let i = 0; i <= 4; i++) {
      const date = new Date(start.getFullYear(), start.getMonth() + i * 3, start.getDate());
      if (date <= end) {
        milestones.push({
          date: date.toISOString().split("T")[0],
          amount: (schedule.totalTokens / 4) * (i + 1),
          percentage: Math.min(100, ((i + 1) / 4) * 100),
          status: date <= now ? "completed" : "upcoming",
        });
      }
    }
  }

  return milestones;
};

export default function TokenVestingPanel() {
  const [schedules, setSchedules] = useState<VestingSchedule[]>(MOCK_SCHEDULES);
  const [selectedSchedule, setSelectedSchedule] = useState<VestingSchedule | null>(MOCK_SCHEDULES[0]);

  const totalVested = schedules.reduce((sum, s) => sum + s.vestedTokens, 0);
  const totalTokens = schedules.reduce((sum, s) => sum + s.totalTokens, 0);
  const vestingCount = schedules.filter((s) => s.status === "vesting").length;

  const selectedMilestones = selectedSchedule ? generateMilestones(selectedSchedule) : [];
  const nextMilestone = selectedMilestones.find((m) => m.status === "upcoming");

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Unlock size={20} className="text-purple-400" /> Token Vesting
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Vested</div>
            <div className="text-lg font-bold text-cyan-400">{(totalVested / 1000000).toFixed(1)}M</div>
            <div className="text-xs text-gray-500 mt-1">{((totalVested / totalTokens) * 100).toFixed(0)}% of total</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Schedules</div>
            <div className="text-lg font-bold text-purple-400">{vestingCount}/{schedules.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Locked Tokens</div>
            <div className="text-lg font-bold text-red-400">{((totalTokens - totalVested) / 1000000).toFixed(1)}M</div>
          </div>
        </div>

        {/* Vesting Schedules */}
        <div>
          <h3 className="font-semibold mb-2 text-sm">Vesting Schedules</h3>
          <div className="space-y-2">
            {schedules.map((schedule) => (
              <button
                key={schedule.id}
                onClick={() => setSelectedSchedule(schedule)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedSchedule?.id === schedule.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="font-semibold text-sm">{schedule.beneficiary}</div>
                    <div className="text-xs text-gray-400">{(schedule.totalTokens / 1000000).toFixed(2)}M tokens</div>
                  </div>
                  <div className="text-right">
                    <div className={clsx("text-xs font-bold", schedule.status === "completed" ? "text-green-400" : schedule.status === "vesting" ? "text-blue-400" : "text-yellow-400")}>
                      {schedule.status.toUpperCase()}
                    </div>
                    <div className="text-lg font-bold text-cyan-400">
                      {((schedule.vestedTokens / schedule.totalTokens) * 100).toFixed(0)}%
                    </div>
                  </div>
                </div>

                <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-purple-600 to-pink-600"
                    style={{ width: `${(schedule.vestedTokens / schedule.totalTokens) * 100}%` }}
                  />
                </div>

                <div className="text-xs text-gray-400 mt-2">
                  {schedule.vestedTokens.toLocaleString()} / {schedule.totalTokens.toLocaleString()} tokens released
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Selected Schedule Details */}
        {selectedSchedule && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
            <h3 className="font-semibold text-sm">Schedule Details: {selectedSchedule.beneficiary}</h3>

            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Total Tokens</span>
                <span className="font-bold text-cyan-400">{(selectedSchedule.totalTokens / 1000000).toFixed(2)}M</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Vested</span>
                <span className="font-bold text-green-400">{(selectedSchedule.vestedTokens / 1000000).toFixed(2)}M</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Locked</span>
                <span className="font-bold text-red-400">
                  {((selectedSchedule.totalTokens - selectedSchedule.vestedTokens) / 1000000).toFixed(2)}M
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Vesting Period</span>
                <span className="font-semibold capitalize">{selectedSchedule.vestingPeriod}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Cliff Date</span>
                <span className="font-mono text-xs">{selectedSchedule.cliffDate}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Start → End</span>
                <span className="font-mono text-xs">
                  {selectedSchedule.startDate} → {selectedSchedule.endDate}
                </span>
              </div>
            </div>

            {nextMilestone && (
              <div className="bg-blue-600/20 border border-blue-600 rounded p-3 space-y-2">
                <div className="font-semibold text-sm flex items-center gap-2 text-blue-300">
                  <Clock size={14} /> Next Release
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-300">Date</span>
                  <span className="font-semibold">{nextMilestone.date}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-300">Amount</span>
                  <span className="font-bold text-cyan-400">{(nextMilestone.amount / 1000000).toFixed(2)}M</span>
                </div>
              </div>
            )}
          </div>
        )}

        {/* Milestone Timeline */}
        {selectedSchedule && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
            <h3 className="font-semibold text-sm">Release Timeline</h3>

            <div className="space-y-2">
              {selectedMilestones.slice(0, 5).map((milestone, idx) => (
                <div key={idx} className="space-y-1">
                  <div className="flex justify-between items-center">
                    <div className="flex items-center gap-2">
                      {milestone.status === "completed" ? (
                        <CheckCircle size={12} className="text-green-400" />
                      ) : (
                        <Calendar size={12} className="text-gray-400" />
                      )}
                      <span className="text-xs font-mono">{milestone.date}</span>
                      <span className={clsx("text-xs font-bold", milestone.status === "completed" ? "text-green-400" : "text-gray-400")}>
                        {milestone.percentage}%
                      </span>
                    </div>
                    <span className="text-xs text-gray-400">{(milestone.amount / 1000000).toFixed(2)}M</span>
                  </div>

                  <div className="flex-1 bg-[#2a2a35] rounded-full h-1 overflow-hidden">
                    <div
                      className={clsx("h-full", milestone.status === "completed" ? "bg-green-600" : "bg-gray-600")}
                      style={{ width: "100%" }}
                    />
                  </div>
                </div>
              ))}
            </div>

            {selectedMilestones.length > 5 && (
              <button className="w-full text-center text-xs text-cyan-400 hover:text-cyan-300 py-2">
                View all {selectedMilestones.length} milestones
              </button>
            )}
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Transparent token release schedules with cliff protection.
      </div>
    </div>
  );
}
