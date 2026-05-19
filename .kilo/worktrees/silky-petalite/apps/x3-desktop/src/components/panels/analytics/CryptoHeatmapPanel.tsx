import React, { useState } from "react";
import { TrendingUp, TrendingDown, Zap } from "lucide-react";
import clsx from "clsx";

interface CryptoAsset {
  id: string;
  symbol: string;
  name: string;
  price: number;
  change24h: number;
  marketCap: number;
  volume24h: number;
  dominance: number;
}

const CRYPTO_DATA: CryptoAsset[] = [
  { id: "1", symbol: "BTC", name: "Bitcoin", price: 47250, change24h: 3.2, marketCap: 930000000000, volume24h: 32500000000, dominance: 42.5 },
  { id: "2", symbol: "ETH", name: "Ethereum", price: 2850, change24h: -1.8, marketCap: 342000000000, volume24h: 18700000000, dominance: 18.2 },
  { id: "3", symbol: "SOL", name: "Solana", price: 185, change24h: 8.5, marketCap: 92500000000, volume24h: 5200000000, dominance: 4.9 },
  { id: "4", symbol: "X3", name: "X3 Chain", price: 1.25, change24h: 5.2, marketCap: 125000000, volume24h: 2500000, dominance: 0.67 },
  { id: "5", symbol: "ADA", name: "Cardano", price: 1.15, change24h: 2.1, marketCap: 42000000000, volume24h: 1800000000, dominance: 2.2 },
  { id: "6", symbol: "XRP", name: "Ripple", price: 3.42, change24h: -0.8, marketCap: 185000000000, volume24h: 8950000000, dominance: 9.8 },
  { id: "7", symbol: "DOT", name: "Polkadot", price: 12.5, change24h: 4.3, marketCap: 165000000000, volume24h: 3250000000, dominance: 8.7 },
  { id: "8", symbol: "AVAX", name: "Avalanche", price: 48.5, change24h: 6.8, marketCap: 18500000000, volume24h: 1020000000, dominance: 0.98 },
  { id: "9", symbol: "LINK", name: "Chainlink", price: 28.5, change24h: -2.4, marketCap: 15000000000, volume24h: 890000000, dominance: 0.8 },
  { id: "10", symbol: "UNI", name: "Uniswap", price: 12.2, change24h: 7.1, marketCap: 12500000000, volume24h: 520000000, dominance: 0.67 },
  { id: "11", symbol: "MATIC", name: "Polygon", price: 0.95, change24h: 3.5, marketCap: 9800000000, volume24h: 420000000, dominance: 0.52 },
  { id: "12", symbol: "OP", name: "Optimism", price: 6.8, change24h: 9.2, marketCap: 5200000000, volume24h: 280000000, dominance: 0.28 },
];

export default function CryptoHeatmapPanel() {
  const [sortBy, setSortBy] = useState<"marketcap" | "change" | "volume">("marketcap");
  const [timeframes, setTimeframes] = useState<"24h" | "7d" | "30d">("24h");
  const [selectedAsset, setSelectedAsset] = useState<CryptoAsset | null>(null);

  const getHeatmapColor = (change: number) => {
    if (change > 10) return "bg-green-600";
    if (change > 5) return "bg-green-500";
    if (change > 0) return "bg-green-400";
    if (change > -5) return "bg-red-400";
    if (change > -10) return "bg-red-500";
    return "bg-red-600";
  };

  const sortedData = [...CRYPTO_DATA].sort((a, b) => {
    switch (sortBy) {
      case "change":
        return b.change24h - a.change24h;
      case "volume":
        return b.volume24h - a.volume24h;
      default:
        return b.marketCap - a.marketCap;
    }
  });

  const totalMarketCap = sortedData.reduce((sum, asset) => sum + asset.marketCap, 0);
  const positive = sortedData.filter(a => a.change24h > 0).length;
  const negative = sortedData.filter(a => a.change24h < 0).length;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">Crypto Market Heatmap</h2>

      {/* Stats Bar */}
      <div className="grid grid-cols-4 gap-3 mb-6">
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Total Market Cap</div>
          <div className="text-lg font-bold">${(totalMarketCap / 1000000000000).toFixed(2)}T</div>
        </div>
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Gainers</div>
          <div className="text-lg font-bold text-green-400">{positive}</div>
        </div>
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Losers</div>
          <div className="text-lg font-bold text-red-400">{negative}</div>
        </div>
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">BTC Dominance</div>
          <div className="text-lg font-bold">42.5%</div>
        </div>
      </div>

      {/* Controls */}
      <div className="flex items-center justify-between gap-4 mb-4">
        <div className="flex gap-2">
          {(["marketcap", "change", "volume"] as const).map((sort) => (
            <button
              key={sort}
              onClick={() => setSortBy(sort)}
              className={clsx(
                "px-3 py-1 rounded text-sm font-semibold transition",
                sortBy === sort
                  ? "bg-blue-600 text-white"
                  : "bg-[#15151b] text-gray-400 border border-[#2a2a35] hover:border-[#3a3a45]"
              )}
            >
              {sort === "marketcap" && "Market Cap"}
              {sort === "change" && "Top Gainers"}
              {sort === "volume" && "By Volume"}
            </button>
          ))}
        </div>
      </div>

      {/* Heatmap Grid */}
      <div className="flex-1 overflow-y-auto">
        <div className="grid grid-cols-4 gap-3">
          {sortedData.map((asset) => (
            <button
              key={asset.id}
              onClick={() => setSelectedAsset(asset)}
              className={clsx(
                "p-4 rounded-lg border-2 transition cursor-pointer",
                selectedAsset?.id === asset.id
                  ? "border-blue-400 bg-[#15151b]"
                  : "border-[#2a2a35] hover:border-[#3a3a45]",
                getHeatmapColor(asset.change24h)
              )}
            >
              <div className="flex items-center justify-between mb-2">
                <span className="font-bold text-white">{asset.symbol}</span>
                <span className={clsx("text-xs font-semibold", asset.change24h >= 0 ? "text-green-300" : "text-red-300")}>
                  {asset.change24h >= 0 ? "+" : ""}{asset.change24h.toFixed(1)}%
                </span>
              </div>
              <div className="text-xs text-gray-300 mb-2 truncate">{asset.name}</div>
              <div className="text-sm font-semibold text-white mb-2">${asset.price.toLocaleString()}</div>
              <div className="text-xs text-gray-500">
                ${(asset.marketCap / 1000000000).toFixed(0)}B market cap
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Selected Asset Details */}
      {selectedAsset && (
        <div className="fixed bottom-6 left-6 right-6 bg-[#15151b] border border-[#3a3a45] p-4 rounded-lg shadow-xl max-w-xs">
          <div className="flex items-start justify-between mb-3">
            <div>
              <div className="flex items-center gap-2 mb-1">
                <span className="text-lg font-bold">{selectedAsset.symbol}</span>
                <span className="text-sm text-gray-400">{selectedAsset.name}</span>
              </div>
              <div className="text-2xl font-bold">${selectedAsset.price.toLocaleString()}</div>
            </div>
            <div className={clsx("flex items-center gap-1 px-2 py-1 rounded", selectedAsset.change24h >= 0 ? "bg-green-500/20 text-green-400" : "bg-red-500/20 text-red-400")}>
              {selectedAsset.change24h >= 0 ? <TrendingUp size={16} /> : <TrendingDown size={16} />}
              <span className="text-sm font-semibold">{selectedAsset.change24h >= 0 ? "+" : ""}{selectedAsset.change24h.toFixed(2)}%</span>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div>
              <span className="text-gray-500">Market Cap</span>
              <div className="font-semibold">${(selectedAsset.marketCap / 1000000000).toFixed(0)}B</div>
            </div>
            <div>
              <span className="text-gray-500">24H Volume</span>
              <div className="font-semibold">${(selectedAsset.volume24h / 1000000000).toFixed(1)}B</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
