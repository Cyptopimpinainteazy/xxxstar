import React, { useState } from "react";
import { Zap, TrendingUp, BarChart3, Grid3x3, AlertCircle } from "lucide-react";
import clsx from "clsx";

interface HeatmapData {
  symbol: string;
  sector: string;
  change24h: number;
  volume: number;
  marketCap: number;
  volatility: number;
}

interface SectorCorrelation {
  sector1: string;
  sector2: string;
  correlation: number;
}

const MOCK_HEATMAP: HeatmapData[] = [
  {
    symbol: "X3",
    sector: "L1 Blockchain",
    change24h: 12.45,
    volume: 2450000,
    marketCap: 5200000,
    volatility: 2.1,
  },
  {
    symbol: "ETH",
    sector: "L1 Blockchain",
    change24h: 3.21,
    volume: 18500000,
    marketCap: 235000000,
    volatility: 1.8,
  },
  {
    symbol: "BTC",
    sector: "L1 Blockchain",
    change24h: 2.15,
    volume: 32000000,
    marketCap: 1200000000,
    volatility: 1.5,
  },
  {
    symbol: "LINK",
    sector: "Oracle",
    change24h: 5.32,
    volume: 520000,
    marketCap: 45000000,
    volatility: 2.3,
  },
  {
    symbol: "UNI",
    sector: "DEX",
    change24h: 8.21,
    volume: 850000,
    marketCap: 78000000,
    volatility: 2.5,
  },
  {
    symbol: "AAVE",
    sector: "Lending",
    change24h: 6.15,
    volume: 450000,
    marketCap: 62000000,
    volatility: 2.2,
  },
];

const MOCK_SECTOR_CORRELATIONS: SectorCorrelation[] = [
  { sector1: "L1 Blockchain", sector2: "Oracle", correlation: 0.68 },
  { sector1: "L1 Blockchain", sector2: "DEX", correlation: 0.72 },
  { sector1: "DEX", sector2: "Lending", correlation: 0.81 },
  { sector1: "Oracle", sector2: "Lending", correlation: 0.65 },
];

export default function MarketHeatmapPanel() {
  const [heatmapData] = useState<HeatmapData[]>(MOCK_HEATMAP);
  const [selectedSector, setSelectedSector] = useState("all");
  const [sortBy, setSortBy] = useState<"change" | "volume" | "volatility">("change");

  const sectors = Array.from(new Set(heatmapData.map((h) => h.sector)));
  const filteredData =
    selectedSector === "all" ? heatmapData : heatmapData.filter((h) => h.sector === selectedSector);

  const sortedData = [...filteredData].sort((a, b) => {
    if (sortBy === "change") return b.change24h - a.change24h;
    if (sortBy === "volume") return b.volume - a.volume;
    return b.volatility - a.volatility;
  });

  const getColorForChange = (change: number): string => {
    if (change > 10) return "bg-green-600";
    if (change > 5) return "bg-green-500";
    if (change > 0) return "bg-cyan-500";
    if (change > -5) return "bg-yellow-600";
    return "bg-red-600";
  };

  const getTextColorForChange = (change: number): string => {
    if (change > 5) return "text-green-400";
    if (change > 0) return "text-cyan-400";
    if (change > -5) return "text-yellow-400";
    return "text-red-400";
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Grid3x3 size={20} className="text-yellow-400" /> Market Heatmap
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Sector Filter */}
        <div className="flex gap-2 overflow-x-auto pb-2">
          <button
            onClick={() => setSelectedSector("all")}
            className={clsx(
              "px-3 py-1.5 rounded-lg text-xs font-semibold whitespace-nowrap transition",
              selectedSector === "all"
                ? "bg-yellow-600/20 border border-yellow-600 text-yellow-400"
                : "bg-[#15151b] border border-[#2a2a35] hover:border-[#3a3a45]"
            )}
          >
            All Assets
          </button>
          {sectors.map((sector) => (
            <button
              key={sector}
              onClick={() => setSelectedSector(sector)}
              className={clsx(
                "px-3 py-1.5 rounded-lg text-xs font-semibold whitespace-nowrap transition",
                selectedSector === sector
                  ? "bg-yellow-600/20 border border-yellow-600 text-yellow-400"
                  : "bg-[#15151b] border border-[#2a2a35] hover:border-[#3a3a45]"
              )}
            >
              {sector}
            </button>
          ))}
        </div>

        {/* Sort Controls */}
        <div className="flex gap-2">
          {(["change", "volume", "volatility"] as const).map((sort) => (
            <button
              key={sort}
              onClick={() => setSortBy(sort)}
              className={clsx(
                "px-3 py-1.5 rounded text-xs font-semibold transition",
                sortBy === sort
                  ? "bg-yellow-600/20 border border-yellow-600 text-yellow-400"
                  : "bg-[#15151b] border border-[#2a2a35] hover:border-[#3a3a45]"
              )}
            >
              {sort === "change" && "24h %"}
              {sort === "volume" && "Volume"}
              {sort === "volatility" && "Volatility"}
            </button>
          ))}
        </div>

        {/* Heatmap Grid */}
        <div className="grid grid-cols-1 gap-2">
          {sortedData.map((asset) => (
            <div
              key={asset.symbol}
              className={clsx("rounded-lg p-3 border border-[#2a2a35] transition hover:border-[#3a3a45]", getColorForChange(asset.change24h))}
            >
              <div className="flex justify-between items-start">
                <div>
                  <div className="font-semibold text-sm">{asset.symbol}</div>
                  <div className="text-xs text-gray-400 mt-1">{asset.sector}</div>
                </div>
                <div className="text-right">
                  <div className={clsx("text-lg font-bold", getTextColorForChange(asset.change24h))}>
                    {asset.change24h >= 0 ? "+" : ""}{asset.change24h.toFixed(2)}%
                  </div>
                  <div className="text-xs text-gray-400 mt-1">Vol: ${(asset.volume / 1000000).toFixed(1)}M</div>
                </div>
              </div>
              <div className="mt-2 flex gap-4 text-xs">
                <div>
                  <span className="text-gray-500">Cap: </span>
                  <span className="text-cyan-400">${(asset.marketCap / 1000000).toFixed(1)}M</span>
                </div>
                <div>
                  <span className="text-gray-500">Vol: </span>
                  <span className="text-yellow-400">{asset.volatility.toFixed(1)}%</span>
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Sector Correlations */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <div className="text-sm font-bold mb-3">Sector Correlations</div>
          <div className="space-y-2">
            {MOCK_SECTOR_CORRELATIONS.map((corr) => (
              <div key={`${corr.sector1}-${corr.sector2}`} className="flex items-center justify-between text-xs">
                <span className="text-gray-400">{corr.sector1} ↔ {corr.sector2}</span>
                <div className="flex items-center gap-2">
                  <div className="w-16 bg-[#2a2a35] rounded-full h-1.5">
                    <div
                      className="h-full bg-yellow-600 rounded-full"
                      style={{ width: `${corr.correlation * 100}%` }}
                    />
                  </div>
                  <span className="font-mono text-cyan-400">{corr.correlation.toFixed(2)}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Real-time market heatmap with sector correlation, volatility tracking, and volume analysis.
      </div>
    </div>
  );
}
