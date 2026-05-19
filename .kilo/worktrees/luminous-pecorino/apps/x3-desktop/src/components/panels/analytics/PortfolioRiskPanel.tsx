import React, { useState } from "react";
import { TrendingUp, AlertTriangle, BarChart3, DollarSign, Target, Activity } from "lucide-react";
import clsx from "clsx";

interface Position {
  symbol: string;
  amount: number;
  value: number;
  percentage: number;
  volatility: number;
  beta: number;
  shortDelta: number;
  correlation: number;
}

const MOCK_POSITIONS: Position[] = [
  { symbol: "X3", amount: 1000, value: 1250, percentage: 35.2, volatility: 45.2, beta: 2.1, shortDelta: 0.95, correlation: 0.5 },
  { symbol: "ETH", amount: 2.5, value: 7125, percentage: 20.1, volatility: 32.5, beta: 1.8, shortDelta: 0.85, correlation: 0.7 },
  { symbol: "USDC", amount: 8500, value: 8500, percentage: 24.0, volatility: 0.5, beta: 0.0, shortDelta: 0.0, correlation: -0.1 },
  { symbol: "SOL", amount: 50, value: 8500, percentage: 20.7, volatility: 55.3, beta: 2.4, shortDelta: 0.92, correlation: 0.6 },
];

export default function PortfolioRiskPanel() {
  const [selectedPos, setSelectedPos] = useState<Position | null>(MOCK_POSITIONS[0]);
  const [riskView, setRiskView] = useState<"delta" | "correlation" | "vol">("delta");

  const totalValue = MOCK_POSITIONS.reduce((sum, p) => sum + p.value, 0);
  const portfolioVolatility = (
    MOCK_POSITIONS.reduce((sum, p) => sum + (p.volatility * p.percentage) / 100, 0)
  ).toFixed(2);
  const portfolioDelta = (MOCK_POSITIONS.reduce((sum, p) => sum + (p.shortDelta * p.percentage) / 100, 0)).toFixed(2);

  const getRiskColor = (val: number) => {
    if (val > 50) return "text-red-400 bg-red-500/20";
    if (val > 30) return "text-yellow-400 bg-yellow-500/20";
    return "text-green-400 bg-green-500/20";
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">Portfolio Risk Analysis</h2>

      {/* Risk Summary */}
      <div className="grid grid-cols-4 gap-3 mb-6">
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Portfolio Value</div>
          <div className="text-2xl font-bold">${totalValue.toLocaleString()}</div>
        </div>
        <div className={clsx("p-4 rounded-lg border border-[#2a2a35]", getRiskColor(Number(portfolioVolatility)))}>
          <div className="text-xs text-gray-400 mb-2">Portfolio Volatility</div>
          <div className="text-2xl font-bold">{portfolioVolatility}%</div>
        </div>
        <div className={clsx("p-4 rounded-lg border border-[#2a2a35]", getRiskColor(Math.abs(Number(portfolioDelta)) * 100))}>
          <div className="text-xs text-gray-400 mb-2">Portfolio Delta</div>
          <div className="text-2xl font-bold">{portfolioDelta}</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">VaR (95%)</div>
          <div className="text-2xl font-bold text-red-400">-${(totalValue * 0.12).toFixed(0)}</div>
          <div className="text-xs text-gray-500 mt-1">24h worst case</div>
        </div>
      </div>

      {/* Risk Views */}
      <div className="flex gap-2 mb-4">
        {(["delta", "correlation", "vol"] as const).map((view) => (
          <button
            key={view}
            onClick={() => setRiskView(view)}
            className={clsx(
              "px-3 py-1 rounded text-sm font-semibold transition",
              riskView === view
                ? "bg-blue-600 text-white"
                : "bg-[#15151b] text-gray-400 border border-[#2a2a35]"
            )}
          >
            {view === "delta" && "Greeks"}
            {view === "correlation" && "Correlation"}
            {view === "vol" && "Volatility"}
          </button>
        ))}
      </div>

      {/* Positions Grid */}
      <div className="flex-1 overflow-y-auto mb-4">
        <div className="space-y-2">
          {MOCK_POSITIONS.map((pos) => (
            <button
              key={pos.symbol}
              onClick={() => setSelectedPos(pos)}
              className={clsx(
                "w-full text-left p-4 rounded-lg border-2 transition",
                selectedPos?.symbol === pos.symbol
                  ? "border-blue-400 bg-[#15151b]"
                  : "border-[#2a2a35] hover:border-[#3a3a45]"
              )}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-3">
                  <span className="font-bold text-lg">{pos.symbol}</span>
                  <span className="text-xs bg-[#2a2a35] px-2 py-1 rounded text-gray-400">
                    {pos.percentage.toFixed(1)}% of portfolio
                  </span>
                </div>
                <div className="text-right">
                  <div className="font-semibold">${pos.value.toLocaleString()}</div>
                  <div className="text-xs text-gray-500">{pos.amount} {pos.symbol}</div>
                </div>
              </div>

              {/* Risk Metric Bar */}
              {riskView === "delta" && (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-gray-400 w-12">Δ:</span>
                  <div className="flex-1 bg-[#2a2a35] rounded h-1.5">
                    <div
                      className="bg-blue-500 h-1.5 rounded"
                      style={{ width: `${pos.shortDelta * 100}%` }}
                    />
                  </div>
                  <span className="text-xs font-semibold">{pos.shortDelta.toFixed(2)}</span>
                </div>
              )}

              {riskView === "vol" && (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-gray-400 w-12">Σ:</span>
                  <div className="flex-1 bg-[#2a2a35] rounded h-1.5">
                    <div
                      className={clsx("h-1.5 rounded", pos.volatility > 40 ? "bg-red-500" : "bg-yellow-500")}
                      style={{ width: `${Math.min(pos.volatility, 100)}%` }}
                    />
                  </div>
                  <span className="text-xs font-semibold">{pos.volatility.toFixed(1)}%</span>
                </div>
              )}

              {riskView === "correlation" && (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-gray-400 w-12">ρ:</span>
                  <div className={clsx("flex-1 px-2 py-1 rounded text-xs font-semibold", pos.correlation > 0.5 ? "bg-red-500/20 text-red-400" : "bg-green-500/20 text-green-400")}>
                    {pos.correlation > 0 ? "+" : ""}{pos.correlation.toFixed(2)}
                  </div>
                </div>
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Detailed Analysis */}
      {selectedPos && (
        <div className="bg-[#15151b] border border-[#2a2a35] p-4 rounded-lg">
          <h4 className="font-bold mb-3 flex items-center gap-2">
            <Target size={16} /> {selectedPos.symbol} Analysis
          </h4>
          <div className="grid grid-cols-3 gap-3 text-sm">
            <div>
              <div className="text-xs text-gray-400 mb-1">Delta (Δ)</div>
              <div className="text-lg font-bold">{selectedPos.shortDelta.toFixed(3)}</div>
              <div className="text-xs text-gray-500 mt-1">Linear sensitivity</div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-1">Volatility (Σ)</div>
              <div className="text-lg font-bold">{selectedPos.volatility.toFixed(1)}%</div>
              <div className="text-xs text-gray-500 mt-1">30-day realized</div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-1">Beta (β)</div>
              <div className="text-lg font-bold">{selectedPos.beta.toFixed(2)}</div>
              <div className="text-xs text-gray-500 mt-1">vs market</div>
            </div>
          </div>
          <div className="mt-3 p-3 bg-blue-500/10 border border-blue-500/20 rounded text-xs">
            <strong>Recommendation:</strong> Position represents {selectedPos.percentage.toFixed(1)}% of portfolio with {selectedPos.volatility.toFixed(1)}% volatility. Consider rebalancing if concentration exceeds 30%.
          </div>
        </div>
      )}
    </div>
  );
}
