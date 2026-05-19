import React, { useState } from "react";
import { TrendingUp, Calendar, DollarSign, BarChart3, ChevronDown } from "lucide-react";
import clsx from "clsx";

interface BacktestResult {
  totalReturn: number;
  winRate: number;
  trades: number;
  avgWin: number;
  avgLoss: number;
  profitFactor: number;
  sharpeRatio: number;
  maxDD: number;
}

const MOCK_RESULTS = {
  "RSI Mean Reversion": {
    totalReturn: 24.5,
    winRate: 62,
    trades: 48,
    avgWin: 125.5,
    avgLoss: 82.3,
    profitFactor: 2.1,
    sharpeRatio: 1.85,
    maxDD: 8.2,
  },
  "MA Crossover": {
    totalReturn: 18.3,
    winRate: 58,
    trades: 61,
    avgWin: 98.2,
    avgLoss: 75.1,
    profitFactor: 1.8,
    sharpeRatio: 1.45,
    maxDD: 12.1,
  },
};

export default function BacktestingPanel() {
  const [selectedStrategy, setSelectedStrategy] = useState("RSI Mean Reversion");
  const [dateRange, setDateRange] = useState("90d");
  const [isRunning, setIsRunning] = useState(false);
  const [showEquityChart, setShowEquityChart] = useState(true);

  const result = MOCK_RESULTS[selectedStrategy as keyof typeof MOCK_RESULTS];

  const handleRunBacktest = () => {
    setIsRunning(true);
    setTimeout(() => setIsRunning(false), 2000);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">Backtesting Engine</h2>

      {/* Control Panel */}
      <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 mb-4">
        <div className="grid grid-cols-3 gap-4 mb-4">
          {/* Strategy Selection */}
          <div>
            <label className="text-xs text-gray-400 uppercase mb-1 block">Strategy</label>
            <select
              value={selectedStrategy}
              onChange={(e) => setSelectedStrategy(e.target.value)}
              className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
            >
              {Object.keys(MOCK_RESULTS).map((strat) => (
                <option key={strat} value={strat}>
                  {strat}
                </option>
              ))}
            </select>
          </div>

          {/* Date Range */}
          <div>
            <label className="text-xs text-gray-400 uppercase mb-1 block">Period</label>
            <select
              value={dateRange}
              onChange={(e) => setDateRange(e.target.value)}
              className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
            >
              <option value="30d">30 Days</option>
              <option value="90d">90 Days</option>
              <option value="6m">6 Months</option>
              <option value="1y">1 Year</option>
            </select>
          </div>

          {/* Run Button */}
          <div className="flex items-end">
            <button
              onClick={handleRunBacktest}
              disabled={isRunning}
              className={clsx(
                "w-full py-2 rounded-lg font-semibold text-sm transition",
                isRunning
                  ? "bg-gray-600 cursor-not-allowed"
                  : "bg-blue-600 hover:bg-blue-700"
              )}
            >
              {isRunning ? "Running..." : "Run Backtest"}
            </button>
          </div>
        </div>
      </div>

      {/* Results Grid */}
      {result && (
        <div className="grid grid-cols-2 gap-3 mb-4">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400">Total Return</div>
            <div className={clsx("text-2xl font-bold", result.totalReturn > 0 ? "text-green-400" : "text-red-400")}>
              +{result.totalReturn}%
            </div>
          </div>

          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400">Win Rate</div>
            <div className="text-2xl font-bold text-blue-400">{result.winRate}%</div>
          </div>

          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400">Profit Factor</div>
            <div className="text-2xl font-bold text-yellow-400">{result.profitFactor}x</div>
          </div>

          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400">Sharpe Ratio</div>
            <div className="text-2xl font-bold text-purple-400">{result.sharpeRatio}</div>
          </div>

          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400">Max Drawdown</div>
            <div className="text-2xl font-bold text-red-400">-{result.maxDD}%</div>
          </div>

          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400">Total Trades</div>
            <div className="text-2xl font-bold text-white">{result.trades}</div>
          </div>
        </div>
      )}

      {/* Equity Curve Toggle */}
      <button
        onClick={() => setShowEquityChart(!showEquityChart)}
        className="flex items-center gap-2 text-sm text-blue-400 hover:text-blue-300 mb-3"
      >
        <TrendingUp size={16} /> {showEquityChart ? "Hide" : "Show"} Equity Curve
      </button>

      {/* Equity Curve Chart Placeholder */}
      {showEquityChart && (
        <div className="flex-1 bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 mb-4 flex flex-col">
          <h3 className="text-sm font-semibold mb-3">Equity Curve</h3>
          {/* Simplified chart visualization */}
          <div className="flex-1 relative flex items-end gap-1">
            {Array.from({ length: 48 }).map((_, i) => (
              <div
                key={i}
                className="flex-1 bg-gradient-to-t from-blue-600 to-blue-400 rounded-t opacity-70"
                style={{ height: `${20 + Math.random() * 70}%` }}
              />
            ))}
          </div>
          <div className="text-xs text-gray-500 mt-2">Period: {dateRange}</div>
        </div>
      )}

      {/* Trade Summary */}
      <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
        <h3 className="text-sm font-semibold mb-3">Trade Summary</h3>
        <div className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-gray-400">Winning Trades</span>
            <span className="text-green-400 font-semibold">{Math.floor(result.trades * (result.winRate / 100))}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-400">Losing Trades</span>
            <span className="text-red-400 font-semibold">{result.trades - Math.floor(result.trades * (result.winRate / 100))}</span>
          </div>
          <div className="flex justify-between border-t border-[#2a2a35] pt-2 mt-2">
            <span className="text-gray-400">Avg Win / Loss</span>
            <span className="text-white">${result.avgWin} / ${result.avgLoss}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
