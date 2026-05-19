import React, { useState } from "react";
import { TrendingUp, TrendingDown, BarChart3, Eye, EyeOff } from "lucide-react";
import clsx from "clsx";

interface TokenPrice {
  symbol: string;
  name: string;
  price: number;
  change24h: number;
  change7d: number;
  change30d: number;
  high24h: number;
  low24h: number;
  volume24h: number;
  marketCap: number;
}

const MOCK_TOKENS: TokenPrice[] = [
  {
    symbol: "X3",
    name: "X3 Chain",
    price: 1.25,
    change24h: 5.2,
    change7d: 12.8,
    change30d: 28.5,
    high24h: 1.28,
    low24h: 1.15,
    volume24h: 2500000,
    marketCap: 125000000,
  },
  {
    symbol: "ETH",
    name: "Ethereum",
    price: 2850,
    change24h: -2.1,
    change7d: 8.3,
    change30d: 15.2,
    high24h: 2900,
    low24h: 2750,
    volume24h: 45000000,
    marketCap: 342000000,
  },
  {
    symbol: "USDC",
    name: "USD Coin",
    price: 1.0,
    change24h: 0.02,
    change7d: 0.05,
    change30d: 0.1,
    high24h: 1.01,
    low24h: 0.99,
    volume24h: 5000000,
    marketCap: 50000000,
  },
];

export default function TokenChartsPanel() {
  const [selectedToken, setSelectedToken] = useState<TokenPrice>(MOCK_TOKENS[0]);
  const [timeframe, setTimeframe] = useState<"24h" | "7d" | "30d">("24h");
  const [showVolume, setShowVolume] = useState(true);

  const getChangeColor = (value: number) => value >= 0 ? "text-green-400" : "text-red-400";

  const renderChart = () => {
    // Simple bar chart visualization
    const bars = Array.from({ length: 12 }, (_, i) => ({
      height: 20 + Math.random() * 60,
      isPositive: Math.random() > 0.4,
    }));

    return (
      <div className="flex items-end justify-around h-40 gap-1 p-4 bg-[#15151b] rounded-lg border border-[#2a2a35]">
        {bars.map((bar, i) => (
          <div
            key={i}
            className={clsx(
              "flex-1 rounded-t",
              bar.isPositive ? "bg-green-500" : "bg-red-500",
              "opacity-70 hover:opacity-100 transition"
            )}
            style={{ height: `${bar.height}%`, minHeight: "2px" }}
          />
        ))}
      </div>
    );
  };

  const changeValue = timeframe === "24h" ? selectedToken.change24h : timeframe === "7d" ? selectedToken.change7d : selectedToken.change30d;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-6">Token Charts</h2>

      {/* Token Selector */}
      <div className="flex gap-2 mb-6 overflow-x-auto">
        {MOCK_TOKENS.map((token) => (
          <button
            key={token.symbol}
            onClick={() => setSelectedToken(token)}
            className={clsx(
              "flex-shrink-0 px-4 py-2 rounded-lg font-semibold transition",
              selectedToken.symbol === token.symbol
                ? "bg-blue-600 text-white"
                : "bg-[#15151b] border border-[#2a2a35] text-gray-400 hover:border-[#3a3a45]"
            )}
          >
            {token.symbol}
          </button>
        ))}
      </div>

      {/* Price Section */}
      <div className="mb-6">
        <div className="flex items-baseline gap-3 mb-4">
          <div className="text-4xl font-bold">${selectedToken.price.toFixed(selectedToken.symbol === "USDC" ? 2 : 2)}</div>
          <div className={clsx("text-lg font-semibold flex items-center gap-1", getChangeColor(changeValue))}>
            {changeValue >= 0 ? <TrendingUp size={20} /> : <TrendingDown size={20} />}
            {Math.abs(changeValue).toFixed(2)}% ({timeframe})
          </div>
        </div>

        {/* 24h High/Low */}
        <div className="flex gap-6 text-sm text-gray-400">
          <div>
            <span className="text-xs">24H High</span>
            <div className="font-semibold text-white">${selectedToken.high24h.toFixed(2)}</div>
          </div>
          <div>
            <span className="text-xs">24H Low</span>
            <div className="font-semibold text-white">${selectedToken.low24h.toFixed(2)}</div>
          </div>
          <div>
            <span className="text-xs">24H Vol</span>
            <div className="font-semibold text-white">${(selectedToken.volume24h / 1000000).toFixed(1)}M</div>
          </div>
        </div>
      </div>

      {/* Chart Controls */}
      <div className="flex items-center justify-between gap-4 mb-4">
        <div className="flex gap-2">
          {(["24h", "7d", "30d"] as const).map((tf) => (
            <button
              key={tf}
              onClick={() => setTimeframe(tf)}
              className={clsx(
                "px-3 py-1 rounded text-sm font-semibold transition",
                timeframe === tf
                  ? "bg-blue-600 text-white"
                  : "bg-[#15151b] text-gray-400 border border-[#2a2a35] hover:border-[#3a3a45]"
              )}
            >
              {tf}
            </button>
          ))}
        </div>
        <button
          onClick={() => setShowVolume(!showVolume)}
          className="p-2 hover:bg-[#15151b] rounded-lg transition text-gray-400 hover:text-white"
        >
          {showVolume ? <Eye size={18} /> : <EyeOff size={18} />}
        </button>
      </div>

      {/* Chart */}
      {renderChart()}

      {/* Stats Grid */}
      <div className="grid grid-cols-2 gap-4 mt-6">
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Market Cap</div>
          <div className="text-lg font-bold">${(selectedToken.marketCap / 1000000).toFixed(0)}M</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">24H Volume</div>
          <div className="text-lg font-bold">${(selectedToken.volume24h / 1000000).toFixed(1)}M</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">7D Change</div>
          <div className={clsx("text-lg font-bold", getChangeColor(selectedToken.change7d))}>
            {selectedToken.change7d >= 0 ? "+" : ""}{selectedToken.change7d.toFixed(2)}%
          </div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">30D Change</div>
          <div className={clsx("text-lg font-bold", getChangeColor(selectedToken.change30d))}>
            {selectedToken.change30d >= 0 ? "+" : ""}{selectedToken.change30d.toFixed(2)}%
          </div>
        </div>
      </div>

      {/* Info Card */}
      <div className="mt-6 p-4 bg-[#15151b] rounded-lg border border-[#2a2a35] text-sm">
        <div className="flex items-start gap-2 text-gray-400">
          <BarChart3 size={16} className="mt-0.5 flex-shrink-0" />
          <span>Add {selectedToken.symbol} to watchlist to track alerts and price milestones</span>
        </div>
      </div>
    </div>
  );
}
