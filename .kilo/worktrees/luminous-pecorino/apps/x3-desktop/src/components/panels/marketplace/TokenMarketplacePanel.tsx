import React, { useState } from "react";
import { Coins, TrendingUp, Zap, Eye, Download, Flame, Star } from "lucide-react";
import clsx from "clsx";

interface TokenListing {
  id: string;
  symbol: string;
  name: string;
  price: number;
  marketCap: number;
  volume24h: number;
  change24h: number;
  holders: number;
  launchDate: string;
  verified: boolean;
}

interface TokenChart {
  timestamp: string;
  price: number;
  volume: number;
}

interface TokenLaunch {
  id: string;
  name: string;
  symbol: string;
  launchDate: string;
  targetRaise: number;
  raisedSoFar: number;
  participants: number;
  status: "upcoming" | "live" | "completed";
}

const MOCK_TOKENS: TokenListing[] = [
  {
    id: "1",
    symbol: "X3",
    name: "X3 Token",
    price: 12.5,
    marketCap: 2500000000,
    volume24h: 125000000,
    change24h: 2.5,
    holders: 2847,
    launchDate: "2024-01-15",
    verified: true,
  },
  {
    id: "2",
    symbol: "X3DEX",
    name: "X3 DEX",
    price: 2.3,
    marketCap: 450000000,
    volume24h: 15000000,
    change24h: -1.2,
    holders: 8234,
    launchDate: "2024-02-10",
    verified: true,
  },
  {
    id: "3",
    symbol: "NEXUS",
    name: "Nexus Protocol",
    price: 0.24,
    marketCap: 24000000,
    volume24h: 2500000,
    change24h: 8.7,
    holders: 15623,
    launchDate: "2024-03-01",
    verified: false,
  },
];

const MOCK_LAUNCHES: TokenLaunch[] = [
  {
    id: "1",
    name: "ChainLoop",
    symbol: "LOOP",
    launchDate: "2024-04-15",
    targetRaise: 5000000,
    raisedSoFar: 4250000,
    participants: 8234,
    status: "live",
  },
  {
    id: "2",
    name: "Quantum Mesh",
    symbol: "QM",
    launchDate: "2024-04-22",
    targetRaise: 3000000,
    raisedSoFar: 0,
    participants: 0,
    status: "upcoming",
  },
  {
    id: "3",
    name: "Oracle Grid",
    symbol: "ORACLE",
    launchDate: "2024-03-10",
    targetRaise: 2000000,
    raisedSoFar: 2000000,
    participants: 12543,
    status: "completed",
  },
];

const CHART_DATA: TokenChart[] = [
  { timestamp: "00:00", price: 12.1, volume: 8.2 },
  { timestamp: "04:00", price: 12.3, volume: 9.5 },
  { timestamp: "08:00", price: 12.0, volume: 7.8 },
  { timestamp: "12:00", price: 12.6, volume: 10.2 },
  { timestamp: "16:00", price: 12.5, volume: 11.5 },
  { timestamp: "20:00", price: 12.8, volume: 12.3 },
];

export default function TokenMarketplacePanel() {
  const [tokens] = useState<TokenListing[]>(MOCK_TOKENS);
  const [launches] = useState<TokenLaunch[]>(MOCK_LAUNCHES);
  const [activeTab, setActiveTab] = useState<"listings" | "launches" | "chart">("listings");
  const [selectedToken, setSelectedToken] = useState<TokenListing | null>(tokens[0]);

  const totalMarketCap = tokens.reduce((sum, t) => sum + t.marketCap, 0);
  const totalVolume = tokens.reduce((sum, t) => sum + t.volume24h, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Coins size={20} className="text-yellow-400" /> Token Marketplace
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Market Cap</div>
            <div className="text-lg font-bold text-yellow-400">${(totalMarketCap / 1000000000).toFixed(2)}B</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">24h Volume</div>
            <div className="text-lg font-bold text-cyan-400">${(totalVolume / 1000000).toFixed(1)}M</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Listed Tokens</div>
            <div className="text-lg font-bold text-green-400">{tokens.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Launches Active</div>
            <div className="text-lg font-bold text-orange-400">{launches.filter((l) => l.status === "live").length}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["listings", "launches", "chart"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-yellow-600 text-yellow-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Listings Tab */}
        {activeTab === "listings" && (
          <div className="space-y-2">
            {tokens.map((token) => (
              <div
                key={token.id}
                onClick={() => setSelectedToken(token)}
                className={clsx("bg-[#15151b] border rounded-lg p-3 cursor-pointer transition", selectedToken?.id === token.id ? "border-yellow-600" : "border-[#2a2a35] hover:border-yellow-600/50")}
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="flex items-center gap-2 mb-1">
                      <div className="font-semibold">{token.symbol}</div>
                      {token.verified && <Star size={14} className="text-blue-400" />}
                    </div>
                    <div className="text-xs text-gray-400">{token.name}</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-cyan-400">${token.price.toFixed(2)}</div>
                    <div className={clsx("text-xs font-semibold", token.change24h >= 0 ? "text-green-400" : "text-red-400")}>
                      {token.change24h >= 0 ? "+" : ""}{token.change24h}%
                    </div>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-2 text-xs">
                  <div>
                    <div className="text-gray-400">Market Cap</div>
                    <div className="font-bold text-cyan-400">${(token.marketCap / 1000000000).toFixed(2)}B</div>
                  </div>
                  <div>
                    <div className="text-gray-400">24h Volume</div>
                    <div className="font-bold text-cyan-400">${(token.volume24h / 1000000).toFixed(1)}M</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Holders</div>
                    <div className="font-bold text-purple-400">{token.holders.toLocaleString()}</div>
                  </div>
                </div>

                <div className="mt-2 flex gap-2">
                  <button className="flex-1 bg-yellow-600/20 text-yellow-400 text-xs font-semibold py-1 rounded hover:bg-yellow-600/30">Buy</button>
                  <button className="flex-1 bg-purple-600/20 text-purple-400 text-xs font-semibold py-1 rounded hover:bg-purple-600/30">Swap</button>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Launches Tab */}
        {activeTab === "launches" && (
          <div className="space-y-2">
            {launches.map((launch) => (
              <div key={launch.id} className={clsx("bg-[#15151b] border border-[#2a2a35] rounded-lg p-3", launch.status === "live" && "border-yellow-600/50")}>
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="flex items-center gap-2">
                      <div className="font-semibold">{launch.symbol}</div>
                      <span
                        className={clsx(
                          "text-xs px-2 py-1 rounded font-bold",
                          launch.status === "upcoming" && "bg-blue-600/20 text-blue-400",
                          launch.status === "live" && "bg-yellow-600/20 text-yellow-400",
                          launch.status === "completed" && "bg-green-600/20 text-green-400"
                        )}
                      >
                        {launch.status}
                      </span>
                    </div>
                    <div className="text-xs text-gray-400 mt-1">{launch.name}</div>
                  </div>
                  {launch.status === "live" && <Flame size={16} className="text-orange-400" />}
                </div>

                <div className="grid grid-cols-3 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">Target</div>
                    <div className="font-bold text-cyan-400">${(launch.targetRaise / 1000000).toFixed(1)}M</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Raised</div>
                    <div className="font-bold text-green-400">${(launch.raisedSoFar / 1000000).toFixed(1)}M</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Participants</div>
                    <div className="font-bold text-purple-400">{launch.participants.toLocaleString()}</div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2 mb-2">
                  <div className="flex-1 bg-[#2a2a35] rounded-full h-2">
                    <div className="h-full bg-gradient-to-r from-yellow-600 to-orange-600" style={{ width: `${(launch.raisedSoFar / launch.targetRaise) * 100}%` }} />
                  </div>
                  <div className="text-xs text-gray-400 mt-1">{((launch.raisedSoFar / launch.targetRaise) * 100).toFixed(1)}% funded</div>
                </div>

                <div className="text-xs text-gray-500">Launch: {launch.launchDate}</div>
              </div>
            ))}
          </div>
        )}

        {/* Chart Tab */}
        {activeTab === "chart" && selectedToken && (
          <div className="space-y-4">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <div className="flex justify-between items-start mb-4">
                <div>
                  <div className="text-sm text-gray-400">Price Chart</div>
                  <div className="text-2xl font-bold text-cyan-400">${selectedToken.price.toFixed(2)}</div>
                </div>
                <div className="text-right">
                  <div className={clsx("text-lg font-bold", selectedToken.change24h >= 0 ? "text-green-400" : "text-red-400")}>
                    {selectedToken.change24h >= 0 ? "+" : ""}{selectedToken.change24h}% (24h)
                  </div>
                </div>
              </div>

              <div className="bg-[#0a0a0f] rounded p-4 mb-2">
                <div className="flex items-end justify-between h-32 gap-1">
                  {CHART_DATA.map((point, idx) => (
                    <div key={idx} className="flex-1">
                      <div
                        className="w-full bg-gradient-to-t from-cyan-600 to-cyan-400 rounded-t opacity-70 hover:opacity-100 transition"
                        style={{ height: `${(point.price / Math.max(...CHART_DATA.map((d) => d.price))) * 100}%` }}
                      />
                      <div className="text-xs text-gray-500 mt-2 text-center">{point.timestamp}</div>
                    </div>
                  ))}
                </div>
              </div>

              <div className="grid grid-cols-3 gap-2 text-xs">
                <div>
                  <div className="text-gray-400">High</div>
                  <div className="font-bold text-green-400">${Math.max(...CHART_DATA.map((d) => d.price)).toFixed(2)}</div>
                </div>
                <div>
                  <div className="text-gray-400">Low</div>
                  <div className="font-bold text-red-400">${Math.min(...CHART_DATA.map((d) => d.price)).toFixed(2)}</div>
                </div>
                <div>
                  <div className="text-gray-400">Avg Volume</div>
                  <div className="font-bold text-orange-400">${(CHART_DATA.reduce((s, d) => s + d.volume, 0) / CHART_DATA.length).toFixed(2)}M</div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Token listings, price charts, launches, and trading pairs.
      </div>
    </div>
  );
}
