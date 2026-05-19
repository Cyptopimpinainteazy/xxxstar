import React, { useState } from "react";
import { Network, TrendingUp, Zap, Eye, Download, Filter } from "lucide-react";
import clsx from "clsx";

interface TransactionFlow {
  id: string;
  fromToken: string;
  toToken: string;
  volume24h: number;
  volume7d: number;
  fees: number;
  status: "active" | "inactive";
}

interface TokenMetrics {
  name: string;
  symbol: string;
  price: number;
  marketCap: number;
  volume24h: number;
  holders: number;
  transfers24h: number;
  topHolder: string;
  topHolderBalance: number;
}

interface SmartContractInteraction {
  id: string;
  contract: string;
  function: string;
  calls: number;
  gasSpent: number;
  errors: number;
  lastCall: string;
}

const MOCK_FLOWS: TransactionFlow[] = [
  { id: "1", fromToken: "X3", toToken: "ETH", volume24h: 850000, volume7d: 5200000, fees: 2125, status: "active" },
  { id: "2", fromToken: "X3", toToken: "USDC", volume24h: 1200000, volume7d: 7500000, fees: 3000, status: "active" },
  { id: "3", fromToken: "ETH", toToken: "BTC", volume24h: 420000, volume7d: 2800000, fees: 1050, status: "active" },
];

const MOCK_TOKENS: TokenMetrics[] = [
  {
    name: "X3 Token",
    symbol: "X3",
    price: 12.5,
    marketCap: 2500000000,
    volume24h: 125000000,
    holders: 2847,
    transfers24h: 58234,
    topHolder: "0x742d...f595",
    topHolderBalance: 8450000,
  },
  {
    name: "Ether",
    symbol: "ETH",
    price: 1850,
    marketCap: 222300000000,
    volume24h: 8200000000,
    holders: 189234,
    transfers24h: 982341,
    topHolder: "0xc02a...48cc",
    topHolderBalance: 3500000,
  },
];

const MOCK_CALLS: SmartContractInteraction[] = [
  {
    id: "1",
    contract: "0xDEX...7a3",
    function: "swap",
    calls: 12543,
    gasSpent: 452890000,
    errors: 23,
    lastCall: "2 mins ago",
  },
  {
    id: "2",
    contract: "0xLPM...2b1",
    function: "mint",
    calls: 3456,
    gasSpent: 125670000,
    errors: 0,
    lastCall: "15 mins ago",
  },
];

export default function OnChainAnalyticsPanel() {
  const [flows] = useState<TransactionFlow[]>(MOCK_FLOWS);
  const [tokens] = useState<TokenMetrics[]>(MOCK_TOKENS);
  const [calls] = useState<SmartContractInteraction[]>(MOCK_CALLS);
  const [activeTab, setActiveTab] = useState<"flows" | "tokens" | "contracts">("flows");

  const totalVolume24h = flows.reduce((sum, f) => sum + f.volume24h, 0);
  const totalHolders = tokens.reduce((sum, t) => sum + t.holders, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Network size={20} className="text-teal-400" /> OnChain Analytics
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">24h Volume</div>
            <div className="text-lg font-bold text-cyan-400">${(totalVolume24h / 1000000).toFixed(1)}M</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Holders</div>
            <div className="text-lg font-bold text-purple-400">{totalHolders.toLocaleString()}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Routes</div>
            <div className="text-lg font-bold text-green-400">{flows.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Contract Calls</div>
            <div className="text-lg font-bold text-orange-400">{calls.reduce((s, c) => s + c.calls, 0).toLocaleString()}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["flows", "tokens", "contracts"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-teal-600 text-teal-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "contracts" ? "Contracts" : tab}
            </button>
          ))}
        </div>

        {activeTab === "flows" && (
          <div className="space-y-2">
            {flows.map((flow) => (
              <div key={flow.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-center mb-2">
                  <div className="flex items-center gap-2">
                    <div className="font-semibold">{flow.fromToken}</div>
                    <TrendingUp size={14} className="text-gray-400" />
                    <div className="font-semibold">{flow.toToken}</div>
                  </div>
                  <span className="text-xs px-2 py-1 bg-green-600/20 text-green-400 rounded font-bold">Active</span>
                </div>

                <div className="grid grid-cols-3 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">24h Volume</div>
                    <div className="font-bold text-cyan-400">${(flow.volume24h / 1000000).toFixed(2)}M</div>
                  </div>
                  <div>
                    <div className="text-gray-400">7d Volume</div>
                    <div className="font-bold text-cyan-400">${(flow.volume7d / 1000000).toFixed(2)}M</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Fees Earned</div>
                    <div className="font-bold text-yellow-400">${flow.fees.toLocaleString()}</div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="flex-1 bg-[#2a2a35] rounded-full h-2">
                    <div className="h-full bg-gradient-to-r from-teal-600 to-cyan-600" style={{ width: `${(flow.volume24h / (totalVolume24h || 1)) * 100}%` }} />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "tokens" && (
          <div className="space-y-2">
            {tokens.map((token) => (
              <div key={token.symbol} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold">{token.name}</div>
                    <div className="text-xs text-gray-400">{token.symbol}</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-cyan-400">${token.price.toLocaleString()}</div>
                    <div className="text-xs text-gray-400">Market Cap</div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">24h Volume</div>
                    <div className="font-bold text-cyan-400">${(token.volume24h / 1000000000).toFixed(2)}B</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Holders</div>
                    <div className="font-bold text-purple-400">{token.holders.toLocaleString()}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">24h Transfers</div>
                    <div className="font-bold text-orange-400">{token.transfers24h.toLocaleString()}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Top Holder</div>
                    <div className="font-mono text-xs text-gray-400">{token.topHolder}</div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-xs text-gray-400 mb-1">Top Holder Balance</div>
                  <div className="text-xs font-bold text-cyan-400">{(token.topHolderBalance / 1000000).toFixed(2)}M tokens</div>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "contracts" && (
          <div className="space-y-2">
            {calls.map((call) => (
              <div key={call.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold text-sm">{call.contract}</div>
                    <div className="text-xs text-gray-400">{call.function}</div>
                  </div>
                  <div className="text-xs text-gray-500">{call.lastCall}</div>
                </div>

                <div className="grid grid-cols-3 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">Total Calls</div>
                    <div className="font-bold text-cyan-400">{call.calls.toLocaleString()}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Gas Spent</div>
                    <div className="font-bold text-orange-400">{(call.gasSpent / 1000000).toFixed(1)}M</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Errors</div>
                    <div className={clsx("font-bold", call.errors > 0 ? "text-red-400" : "text-green-400")}>{call.errors}</div>
                  </div>
                </div>

                <div className="text-xs text-gray-500">
                  Success Rate: <span className="text-green-400 font-semibold">{(((call.calls - call.errors) / call.calls) * 100).toFixed(2)}%</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Token flow analysis, smart contract call tracking, holder distribution, and on-chain metrics.
      </div>
    </div>
  );
}
