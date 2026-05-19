import React, { useState } from "react";
import { CheckCircle, AlertCircle, Clock, ArrowRight, TrendingUp, Zap } from "lucide-react";
import clsx from "clsx";

interface BridgeTransfer {
  id: string;
  from: string;
  fromChain: string;
  to: string;
  toChain: string;
  amount: number;
  asset: string;
  status: "pending" | "confirmed" | "completed" | "failed";
  timestamp: string;
  estimatedTime: string;
  txHash: string;
}

const MOCK_TRANSFERS: BridgeTransfer[] = [
  {
    id: "1",
    from: "0x1234...5678",
    fromChain: "Ethereum",
    to: "0x8765...4321",
    toChain: "X3",
    amount: 100,
    asset: "USDC",
    status: "completed",
    timestamp: "2 hours ago",
    estimatedTime: "~2 minutes",
    txHash: "0xabc123def456",
  },
  {
    id: "2",
    from: "0x1234...5678",
    fromChain: "Polygon",
    to: "0x8765...4321",
    toChain: "X3",
    amount: 50,
    asset: "USDC",
    status: "confirmed",
    timestamp: "15 minutes ago",
    estimatedTime: "~30 seconds remaining",
    txHash: "0xdef456abc789",
  },
  {
    id: "3",
    from: "0x1234...5678",
    fromChain: "X3",
    to: "0x8765...4321",
    toChain: "Ethereum",
    amount: 1.5,
    asset: "X3",
    status: "pending",
    timestamp: "just now",
    estimatedTime: "~5 minutes",
    txHash: "0xghi789jkl012",
  },
];

const BRIDGE_STATUS = {
  liquidity: { ethereum: 2500000, polygon: 850000, x3: 1200000 },
  fees: { ethereum: 0.25, polygon: 0.15, x3: 0.05 },
  latency: { ethereum: 120, polygon: 45, x3: 8 },
};

export default function BridgeStatusPanel() {
  const [transfers, setTransfers] = useState<BridgeTransfer[]>(MOCK_TRANSFERS);
  const [showNewTransfer, setShowNewTransfer] = useState(false);
  const [selectedChain, setSelectedChain] = useState<string | null>(null);

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle size={16} className="text-green-400" />;
      case "confirmed":
        return <Clock size={16} className="text-yellow-400 animate-spin" />;
      case "pending":
        return <Clock size={16} className="text-blue-400 animate-pulse" />;
      case "failed":
        return <AlertCircle size={16} className="text-red-400" />;
      default:
        return null;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "bg-green-500/20 text-green-300";
      case "confirmed":
        return "bg-yellow-500/20 text-yellow-300";
      case "pending":
        return "bg-blue-500/20 text-blue-300";
      case "failed":
        return "bg-red-500/20 text-red-300";
      default:
        return "bg-gray-500/20 text-gray-300";
    }
  };

  const chains = ["Ethereum", "Polygon", "X3", "Solana", "Avalanche"];

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-6">Bridge Status</h2>

      {/* Chain Status Grid */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        {chains.slice(0, 3).map((chain) => (
          <div key={chain} className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
              <span className="font-semibold">{chain}</span>
            </div>
            <div className="space-y-2 text-sm text-gray-400">
              <div>
                <span className="text-xs">Liquidity</span>
                <div className="font-semibold text-white">
                  ${(BRIDGE_STATUS.liquidity[chain as keyof typeof BRIDGE_STATUS.liquidity] / 1000000).toFixed(1)}M
                </div>
              </div>
              <div>
                <span className="text-xs">Fee</span>
                <div className="font-semibold text-white">
                  {BRIDGE_STATUS.fees[chain as keyof typeof BRIDGE_STATUS.fees].toFixed(2)}%
                </div>
              </div>
              <div>
                <span className="text-xs">Latency</span>
                <div className="font-semibold text-white">
                  {BRIDGE_STATUS.latency[chain as keyof typeof BRIDGE_STATUS.latency]}s
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Recent Transfers */}
      <div className="mb-4">
        <h3 className="text-sm font-semibold text-gray-400 mb-3">Recent Transfers</h3>
        <div className="space-y-3 flex-1 overflow-y-auto max-h-64">
          {transfers.map((transfer) => (
            <div key={transfer.id} className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  {getStatusIcon(transfer.status)}
                  <span className="text-sm font-semibold">{transfer.amount} {transfer.asset}</span>
                  <span className={clsx("text-xs px-2 py-0.5 rounded-full font-semibold", getStatusColor(transfer.status))}>
                    {transfer.status.toUpperCase()}
                  </span>
                </div>
                <span className="text-xs text-gray-500">{transfer.timestamp}</span>
              </div>
              <div className="flex items-center justify-between text-xs text-gray-400 mb-2">
                <span>{transfer.fromChain}</span>
                <ArrowRight size={14} />
                <span>{transfer.toChain}</span>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span className="font-mono text-gray-500">{transfer.txHash}</span>
                <span className="text-gray-500">{transfer.estimatedTime}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Network Health */}
      <div className="grid grid-cols-2 gap-3 mt-4">
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="flex items-center gap-2 mb-2">
            <Zap size={14} className="text-yellow-400" />
            <span className="text-xs font-semibold">Bridge Throughput</span>
          </div>
          <div className="text-sm font-bold">$2.4M/hour</div>
          <div className="text-xs text-gray-500 mt-1">Peak capacity: $5M/hour</div>
        </div>
        <div className="bg-[#15151b] p-3 rounded-lg border border-[#2a2a35]">
          <div className="flex items-center gap-2 mb-2">
            <TrendingUp size={14} className="text-green-400" />
            <span className="text-xs font-semibold">Avg Processing</span>
          </div>
          <div className="text-sm font-bold">58s</div>
          <div className="text-xs text-gray-500 mt-1">±12s variance</div>
        </div>
      </div>

      {/* Action Button */}
      <button className="w-full mt-4 bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
        New Bridge Transfer
      </button>
    </div>
  );
}
