import React, { useState } from "react";
import { TrendingUp, BarChart3, AlertCircle, Zap, Eye, Download } from "lucide-react";
import clsx from "clsx";

interface Portfolio {
  symbol: string;
  amount: number;
  value: number;
  allocation: number;
  change24h: number;
  change7d: number;
}

interface RiskMetrics {
  sharpeRatio: number;
  maxDrawdown: number;
  volatility: number;
  correlation: number;
  betaVsMarket: number;
  valueAtRisk: number;
}

interface PortfolioMetadata {
  totalValue: number;
  portfolioReturn: number;
  principalInvested: number;
  unrealizedGain: number;
  concentrationRatio: number;
}

const MOCK_PORTFOLIO: Portfolio[] = [
  { symbol: "X3", amount: 10000, value: 125000, allocation: 48, change24h: 2.5, change7d: 8.3 },
  { symbol: "ETH", amount: 5.2, value: 85000, allocation: 32, change24h: -1.2, change7d: 5.1 },
  { symbol: "USDC", amount: 60000, value: 60000, allocation: 18, change24h: 0, change7d: 0 },
  { symbol: "BTC", amount: 0.15, value: 6250, allocation: 2, change24h: 3.8, change7d: 12.1 },
];

const MOCK_RISK: RiskMetrics = {
  sharpeRatio: 2.34,
  maxDrawdown: -18.5,
  volatility: 45.2,
  correlation: 0.72,
  betaVsMarket: 1.18,
  valueAtRisk: 12500,
};

const MOCK_META: PortfolioMetadata = {
  totalValue: 276250,
  portfolioReturn: 125000,
  principalInvested: 151250,
  unrealizedGain: 82500,
  concentrationRatio: 0.48,
};

export default function AdvancedPortfolioAnalyticsPanel() {
  const [portfolio] = useState<Portfolio[]>(MOCK_PORTFOLIO);
  const [risk] = useState<RiskMetrics>(MOCK_RISK);
  const [metadata] = useState<PortfolioMetadata>(MOCK_META);
  const [activeTab, setActiveTab] = useState<"overview" | "risk" | "correlation">("overview");

  const gainLoss = metadata.unrealizedGain;
  const returnPercent = ((metadata.unrealizedGain / metadata.principalInvested) * 100).toFixed(1);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <TrendingUp size={20} className="text-blue-400" /> Advanced Portfolio Analytics
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Key Metrics */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Value</div>
            <div className="text-lg font-bold text-cyan-400">${metadata.totalValue.toLocaleString()}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Unrealized Gain</div>
            <div className={clsx("text-lg font-bold", gainLoss >= 0 ? "text-green-400" : "text-red-400")}>
              +${gainLoss.toLocaleString()}
            </div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Return</div>
            <div className={clsx("text-lg font-bold", parseFloat(returnPercent) >= 0 ? "text-green-400" : "text-red-400")}>
              {parseFloat(returnPercent) >= 0 ? "+" : ""}{returnPercent}%
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["overview", "risk", "correlation"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-blue-600 text-blue-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {activeTab === "overview" && (
          <div className="space-y-2">
            {portfolio.map((asset) => (
              <div key={asset.symbol} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold text-sm">{asset.symbol}</div>
                    <div className="text-xs text-gray-400">{asset.amount} tokens</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-cyan-400">${asset.value.toLocaleString()}</div>
                    <div className="text-xs text-gray-400">{asset.allocation}% of portfolio</div>
                  </div>
                </div>

                <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden mb-2">
                  <div className="h-full bg-gradient-to-r from-blue-600 to-cyan-600" style={{ width: `${asset.allocation}%` }} />
                </div>

                <div className="flex justify-between text-xs">
                  <span className={clsx("font-semibold", asset.change24h >= 0 ? "text-green-400" : "text-red-400")}>
                    24h: {asset.change24h >= 0 ? "+" : ""}{asset.change24h}%
                  </span>
                  <span className={clsx("font-semibold", asset.change7d >= 0 ? "text-green-400" : "text-red-400")}>
                    7d: {asset.change7d >= 0 ? "+" : ""}{asset.change7d}%
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "risk" && (
          <div className="space-y-2">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <AlertCircle size={14} className="text-yellow-400" /> Risk Metrics
              </h3>

              <div className="grid grid-cols-2 gap-3 text-sm">
                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-gray-400 text-xs mb-1">Sharpe Ratio</div>
                  <div className="font-bold text-cyan-400">{risk.sharpeRatio.toFixed(2)}</div>
                  <div className="text-xs text-gray-500">Higher is better</div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-gray-400 text-xs mb-1">Max Drawdown</div>
                  <div className="font-bold text-red-400">{risk.maxDrawdown.toFixed(1)}%</div>
                  <div className="text-xs text-gray-500">Worst loss</div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-gray-400 text-xs mb-1">Volatility</div>
                  <div className="font-bold text-yellow-400">{risk.volatility.toFixed(1)}%</div>
                  <div className="text-xs text-gray-500">Annualized</div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-gray-400 text-xs mb-1">Value at Risk (95%)</div>
                  <div className="font-bold text-orange-400">${risk.valueAtRisk.toLocaleString()}</div>
                  <div className="text-xs text-gray-500">Max 1-day loss</div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-gray-400 text-xs mb-1">Beta vs Market</div>
                  <div className="font-bold text-purple-400">{risk.betaVsMarket.toFixed(2)}</div>
                  <div className="text-xs text-gray-500">Market sensitivity</div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-gray-400 text-xs mb-1">Concentration</div>
                  <div className="font-bold text-cyan-400">{(risk.correlation * 100).toFixed(0)}%</div>
                  <div className="text-xs text-gray-500">Avg correlation</div>
                </div>
              </div>
            </div>

            {/* Risk Score */}
            <div className="bg-blue-600/10 border border-blue-600/30 rounded-lg p-3">
              <div className="flex justify-between items-center mb-2">
                <div className="font-semibold text-sm">Portfolio Risk Score</div>
                <div className="text-lg font-bold text-blue-400">6.8/10</div>
              </div>
              <div className="w-full bg-[#2a2a35] rounded-full h-2">
                <div className="h-full bg-gradient-to-r from-green-600 via-yellow-600 to-red-600 w-2/3 rounded-full" />
              </div>
              <div className="text-xs text-gray-400 mt-2">Medium risk - suitable for balanced investors</div>
            </div>
          </div>
        )}

        {activeTab === "correlation" && (
          <div className="space-y-2">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="text-sm font-semibold mb-3">Asset Correlation Matrix</h3>
              <div className="space-y-2 text-xs">
                <div className="grid grid-cols-5 gap-1">
                  <div></div>
                  {["X3", "ETH", "USDC", "BTC"].map((sym) => (
                    <div key={sym} className="font-semibold text-center text-gray-400">{sym}</div>
                  ))}
                </div>
                {[
                  { asset: "X3", corr: [1.0, 0.68, 0.12, 0.84] },
                  { asset: "ETH", corr: [0.68, 1.0, 0.15, 0.92] },
                  { asset: "USDC", corr: [0.12, 0.15, 1.0, 0.08] },
                  { asset: "BTC", corr: [0.84, 0.92, 0.08, 1.0] },
                ].map(({ asset, corr }) => (
                  <div key={asset} className="grid grid-cols-5 gap-1 items-center">
                    <div className="font-semibold text-gray-400">{asset}</div>
                    {corr.map((c, i) => (
                      <div
                        key={i}
                        className={clsx("text-center rounded py-1 font-bold", c === 1.0 ? "bg-[#2a2a35]" : c > 0.7 ? "bg-red-600/20 text-red-400" : c > 0.4 ? "bg-yellow-600/20 text-yellow-400" : "bg-green-600/20 text-green-400")}
                      >
                        {c.toFixed(2)}
                      </div>
                    ))}
                  </div>
                ))}
              </div>
              <div className="text-xs text-gray-500 mt-3">
                High correlation (0.7+) between major assets. USDC provides diversification.
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Sharpe ratio, drawdown analysis, volatility tracking, and asset correlation matrix for portfolio optimization.
      </div>
    </div>
  );
}
