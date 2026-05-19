import React, { useState } from "react";
import { TrendingUp, BarChart3, PieChart, ArrowUp, ArrowDown, AlertCircle, Download } from "lucide-react";
import clsx from "clsx";

interface PortfolioMetrics {
  totalValue: number;
  dayPnl: number;
  dayPnlPercent: number;
  weekPnl: number;
  monthPnl: number;
  maxDrawdown: number;
  sharpeRatio: number;
  winRate: number;
}

interface HoldingAnalysis {
  symbol: string;
  value: number;
  allocation: number;
  unrealizedPnl: number;
  pnlPercent: number;
  beta: number;
  correlation: number;
}

interface CorrelationMatrix {
  symbol1: string;
  symbol2: string;
  correlation: number;
}

const MOCK_METRICS: PortfolioMetrics = {
  totalValue: 125430.5,
  dayPnl: 2150,
  dayPnlPercent: 1.74,
  weekPnl: 8730,
  monthPnl: 24680,
  maxDrawdown: 12.5,
  sharpeRatio: 1.82,
  winRate: 65,
};

const MOCK_HOLDINGS: HoldingAnalysis[] = [
  {
    symbol: "X3",
    value: 75430,
    allocation: 60.1,
    unrealizedPnl: 18750,
    pnlPercent: 33.2,
    beta: 1.2,
    correlation: 0.95,
  },
  {
    symbol: "USDC",
    value: 30000,
    allocation: 23.9,
    unrealizedPnl: 0,
    pnlPercent: 0,
    beta: 0.0,
    correlation: 0.0,
  },
  {
    symbol: "ETH",
    value: 15000,
    allocation: 11.9,
    unrealizedPnl: 2850,
    pnlPercent: 23.5,
    beta: 0.8,
    correlation: 0.72,
  },
  {
    symbol: "LINK",
    value: 5000,
    allocation: 4.0,
    unrealizedPnl: 680,
    pnlPercent: 15.8,
    beta: 1.1,
    correlation: 0.68,
  },
];

const MOCK_CORRELATIONS: CorrelationMatrix[] = [
  { symbol1: "X3", symbol2: "ETH", correlation: 0.72 },
  { symbol1: "X3", symbol2: "LINK", correlation: 0.68 },
  { symbol1: "ETH", symbol2: "LINK", correlation: 0.81 },
];

export default function AdvancedAnalyticsPanel() {
  const [metrics] = useState<PortfolioMetrics>(MOCK_METRICS);
  const [holdings] = useState<HoldingAnalysis[]>(MOCK_HOLDINGS);
  const [activeTab, setActiveTab] = useState<"overview" | "holdings" | "correlation">("overview");

  const totalAllocation = holdings.reduce((sum, h) => sum + h.allocation, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <BarChart3 size={20} className="text-green-400" /> Advanced Analytics
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Key Metrics */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
          <div className="grid grid-cols-4 gap-2">
            <div>
              <div className="text-xs text-gray-400 mb-0.5">Portfolio Value</div>
              <div className="text-lg font-bold text-cyan-400">${metrics.totalValue.toLocaleString()}</div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-0.5">Sharpe Ratio</div>
              <div className="text-lg font-bold text-green-400">{metrics.sharpeRatio.toFixed(2)}</div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-0.5">Max Drawdown</div>
              <div className="text-lg font-bold text-red-400">-{metrics.maxDrawdown.toFixed(1)}%</div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-0.5">Win Rate</div>
              <div className="text-lg font-bold text-yellow-400">{metrics.winRate}%</div>
            </div>
          </div>
        </div>

        {/* PnL Cards */}
        <div className="grid grid-cols-3 gap-2">
          {[
            { label: "Today", value: metrics.dayPnl, percent: metrics.dayPnlPercent },
            { label: "This Week", value: metrics.weekPnl, percent: (metrics.weekPnl / metrics.totalValue) * 100 },
            {
              label: "This Month",
              value: metrics.monthPnl,
              percent: (metrics.monthPnl / metrics.totalValue) * 100,
            },
          ].map((period) => (
            <div key={period.label} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-xs text-gray-400 mb-1">{period.label}</div>
              <div className={clsx("text-lg font-bold flex items-center gap-1", period.value >= 0 ? "text-green-400" : "text-red-400")}>
                {period.value >= 0 ? <ArrowUp size={14} /> : <ArrowDown size={14} />}
                ${Math.abs(period.value).toLocaleString()}
              </div>
              <div className="text-xs text-gray-500 mt-1">{period.percent >= 0 ? "+" : ""}{period.percent.toFixed(2)}%</div>
            </div>
          ))}
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["overview", "holdings", "correlation"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab
                  ? "border-green-600 text-green-400"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {activeTab === "overview" && (
          <div className="space-y-2">
            {/* Allocation Chart */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-sm font-bold mb-3">Portfolio Allocation</div>
              <div className="space-y-2">
                {holdings.map((holding) => (
                  <div key={holding.symbol}>
                    <div className="flex justify-between items-center mb-1 text-xs">
                      <span className="font-semibold">{holding.symbol}</span>
                      <span>{holding.allocation.toFixed(1)}%</span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="h-full bg-gradient-to-r from-cyan-600 to-green-600 rounded-full"
                        style={{ width: `${holding.allocation}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {activeTab === "holdings" && (
          <div className="space-y-2">
            {holdings.map((holding) => (
              <div key={holding.symbol} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold flex items-center gap-2">
                      <span>{holding.symbol}</span>
                      <span className="text-xs text-gray-400">β={holding.beta.toFixed(2)}</span>
                    </div>
                    <div className="text-xs text-gray-400 mt-1">${holding.value.toLocaleString()}</div>
                  </div>
                  <div className="text-right">
                    <div className={clsx("font-bold", holding.pnlPercent >= 0 ? "text-green-400" : "text-red-400")}>
                      {holding.pnlPercent >= 0 ? "+" : ""}{holding.pnlPercent.toFixed(1)}%
                    </div>
                    <div className="text-xs text-gray-400">${holding.unrealizedPnl.toLocaleString()}</div>
                  </div>
                </div>
                <div className="text-xs text-gray-400">{holding.allocation.toFixed(1)}% of portfolio</div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "correlation" && (
          <div className="space-y-2">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-sm font-bold mb-3">Asset Correlations</div>
              <div className="space-y-2">
                {MOCK_CORRELATIONS.map((corr) => (
                  <div key={`${corr.symbol1}-${corr.symbol2}`} className="flex items-center justify-between text-xs">
                    <span className="text-gray-400">
                      {corr.symbol1} ↔ {corr.symbol2}
                    </span>
                    <div className="flex items-center gap-2">
                      <div className="w-20 bg-[#2a2a35] rounded-full h-1.5">
                        <div
                          className={clsx("h-full rounded-full", corr.correlation > 0.7 ? "bg-red-600" : "bg-green-600")}
                          style={{ width: `${corr.correlation * 100}%` }}
                        />
                      </div>
                      <span className="font-mono">{corr.correlation.toFixed(2)}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Diversification Score */}
            <div className="bg-green-600/10 border border-green-600/30 rounded-lg p-3">
              <div className="text-xs font-semibold text-green-400 mb-1">✓ Diversification Score</div>
              <div className="text-xs text-gray-300">Low correlation detected between assets. Portfolio is well-diversified.</div>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Portfolio P&L tracking, correlation analysis, and risk metrics dashboard.
      </div>
    </div>
  );
}
