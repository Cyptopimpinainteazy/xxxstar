import React, { useState } from "react";
import { Zap, TrendingUp, Shield, AlertCircle, Activity } from "lucide-react";
import clsx from "clsx";

interface MevStats {
  todayCapture: number;
  weekCapture: number;
  avgPerTx: number;
  sandwichesBlocked: number;
  successRate: number;
}

const MOCK_STATS: MevStats = {
  todayCapture: 2450.75,
  weekCapture: 12340.50,
  avgPerTx: 45.20,
  sandwichesBlocked: 18,
  successRate: 89,
};

export default function MevBotPanel() {
  const [mevEnabled, setMevEnabled] = useState(true);
  const [sandwichProtection, setSandwichProtection] = useState(true);
  const [aggressiveness, setAggressiveness] = useState(60);
  const [selectedStrategy, setSelectedStrategy] = useState("balanced");
  const [autoMode, setAutoMode] = useState(true);
  const [earnings, setEarnings] = useState<{ id: string; amount: number; type: string; time: string }[]>([
    { id: "1", amount: 125.50, type: "Front-run", time: "2 mins ago" },
    { id: "2", amount: 87.30, type: "Sandwich", time: "5 mins ago" },
    { id: "3", amount: 234.75, type: "Arbitrage", time: "12 mins ago" },
  ]);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold flex items-center gap-2">
          <Zap size={20} /> MEV Bot
        </h2>
        <div className={clsx(
          "px-3 py-1 rounded text-sm font-semibold",
          mevEnabled ? "bg-green-600" : "bg-gray-600"
        )}>
          {mevEnabled ? "Active" : "Inactive"}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto mb-4 space-y-4">
        {/* Status Cards */}
        <div className="grid grid-cols-2 gap-3">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Today's Capture</div>
            <div className="text-2xl font-bold text-green-400">${MOCK_STATS.todayCapture}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">This Week</div>
            <div className="text-2xl font-bold text-blue-400">${MOCK_STATS.weekCapture}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Avg Per Tx</div>
            <div className="text-2xl font-bold text-yellow-400">${MOCK_STATS.avgPerTx}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Success Rate</div>
            <div className="text-2xl font-bold text-purple-400">{MOCK_STATS.successRate}%</div>
          </div>
        </div>

        {/* Main Controls */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
          <h3 className="font-semibold flex items-center gap-2">
            <Activity size={16} /> Bot Controls
          </h3>

          {/* MEV Enabled Toggle */}
          <div className="flex items-center justify-between">
            <div>
              <h4 className="text-sm font-medium">Enable MEV Bot</h4>
              <p className="text-xs text-gray-400">Capture MEV opportunities automatically</p>
            </div>
            <button
              onClick={() => setMevEnabled(!mevEnabled)}
              className={clsx(
                "w-12 h-6 rounded-full transition relative",
                mevEnabled ? "bg-green-600" : "bg-[#2a2a35]"
              )}
            >
              <div
                className={clsx(
                  "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                  mevEnabled ? "right-0.5" : "left-0.5"
                )}
              />
            </button>
          </div>

          {mevEnabled && (
            <>
              {/* Sandwich Protection */}
              <div className="flex items-center justify-between border-t border-[#2a2a35] pt-3">
                <div>
                  <h4 className="text-sm font-medium">Sandwich Protection</h4>
                  <p className="text-xs text-gray-400">Block sandwich attacks on your TXs</p>
                </div>
                <button
                  onClick={() => setSandwichProtection(!sandwichProtection)}
                  className={clsx(
                    "w-12 h-6 rounded-full transition relative",
                    sandwichProtection ? "bg-blue-600" : "bg-[#2a2a35]"
                  )}
                >
                  <div
                    className={clsx(
                      "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                      sandwichProtection ? "right-0.5" : "left-0.5"
                    )}
                  />
                </button>
              </div>

              {/* Auto Mode */}
              <div className="flex items-center justify-between border-t border-[#2a2a35] pt-3">
                <div>
                  <h4 className="text-sm font-medium">Auto Mode</h4>
                  <p className="text-xs text-gray-400">Auto-execute all MEV opportunities</p>
                </div>
                <button
                  onClick={() => setAutoMode(!autoMode)}
                  className={clsx(
                    "w-12 h-6 rounded-full transition relative",
                    autoMode ? "bg-green-600" : "bg-[#2a2a35]"
                  )}
                >
                  <div
                    className={clsx(
                      "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                      autoMode ? "right-0.5" : "left-0.5"
                    )}
                  />
                </button>
              </div>

              {/* Aggressiveness Slider */}
              <div className="border-t border-[#2a2a35] pt-3">
                <label className="text-sm font-medium block mb-2">
                  Bid Aggressiveness: {aggressiveness}%
                </label>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={aggressiveness}
                  onChange={(e) => setAggressiveness(parseInt(e.target.value))}
                  className="w-full"
                />
                <p className="text-xs text-gray-400 mt-1">
                  {aggressiveness < 33 && "Conservative - lower success, higher profit"}
                  {aggressiveness >= 33 && aggressiveness < 66 && "Balanced - moderate risk/reward"}
                  {aggressiveness >= 66 && "Aggressive - higher success, lower profit"}
                </p>
              </div>

              {/* Strategy Selection */}
              <div className="border-t border-[#2a2a35] pt-3">
                <label className="text-sm font-medium block mb-2">Strategy</label>
                <select
                  value={selectedStrategy}
                  onChange={(e) => setSelectedStrategy(e.target.value)}
                  className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
                >
                  <option value="balanced">Balanced (All MEV)</option>
                  <option value="frontrun">Front-Run Only</option>
                  <option value="sandwich">Sandwich Only</option>
                  <option value="arbitrage">Arbitrage Only</option>
                </select>
              </div>
            </>
          )}
        </div>

        {/* Protection Status */}
        {sandwichProtection && (
          <div className="bg-blue-600/10 border border-blue-600 rounded-lg p-3 flex items-start gap-2">
            <Shield size={16} className="text-blue-400 flex-shrink-0 mt-0.5" />
            <div>
              <div className="text-sm font-semibold text-blue-400">Sandwich Protection Active</div>
              <div className="text-xs text-blue-300">Blocked {MOCK_STATS.sandwichesBlocked} sandwich attempts this week</div>
            </div>
          </div>
        )}

        {/* Live Earnings */}
        <div>
          <h3 className="font-semibold mb-3 flex items-center gap-2">
            <TrendingUp size={16} /> Recent Earnings
          </h3>
          <div className="space-y-2">
            {earnings.map((earning) => (
              <div
                key={earning.id}
                className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 flex items-center justify-between"
              >
                <div>
                  <div className="text-sm font-semibold">{earning.type}</div>
                  <div className="text-xs text-gray-400">{earning.time}</div>
                </div>
                <div className="text-right">
                  <div className="font-bold text-green-400">${earning.amount.toFixed(2)}</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Risk Warning */}
        <div className="bg-orange-600/10 border border-orange-600 rounded-lg p-3 flex items-start gap-2">
          <AlertCircle size={16} className="text-orange-400 flex-shrink-0 mt-0.5" />
          <div>
            <div className="text-sm font-semibold text-orange-400">Risk Disclosure</div>
            <div className="text-xs text-orange-300 mt-1">
              MEV operations may be subject to regulatory scrutiny. Use at your own risk and comply with local laws.
            </div>
          </div>
        </div>
      </div>

      <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
        Save Settings
      </button>
    </div>
  );
}
