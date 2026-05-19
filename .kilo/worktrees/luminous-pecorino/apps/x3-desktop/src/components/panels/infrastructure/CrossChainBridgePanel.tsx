import React, { useState } from "react";
import { Send, TrendingUp, Clock, CheckCircle, AlertTriangle } from "lucide-react";
import clsx from "clsx";

interface CrossChainTransfer {
  id: string;
  from: string;
  to: string;
  asset: string;
  amount: number;
  status: "pending" | "confirmed" | "completed" | "failed";
  timestamp: string;
  fee: number;
  txHash: string;
}

interface BridgePool {
  id: string;
  chain: string;
  asset: string;
  liquidity: number;
  apy: number;
  utilization: number;
}

const MOCK_TRANSFERS: CrossChainTransfer[] = [
  {
    id: "1",
    from: "Ethereum",
    to: "X3",
    asset: "USDC",
    amount: 5000,
    status: "completed",
    timestamp: "2 hours ago",
    fee: 12.5,
    txHash: "0x1234...5678",
  },
  {
    id: "2",
    from: "Solana",
    to: "X3",
    asset: "SOL",
    amount: 50,
    status: "confirmed",
    timestamp: "15 mins ago",
    fee: 0.5,
    txHash: "5Nx...9Jk",
  },
  {
    id: "3",
    from: "X3",
    to: "Ethereum",
    asset: "X3",
    amount: 10000,
    status: "pending",
    timestamp: "5 mins ago",
    fee: 2.0,
    txHash: "0xabcd...efgh",
  },
];

const MOCK_POOLS: BridgePool[] = [
  { id: "1", chain: "Ethereum", asset: "USDC", liquidity: 15000000, apy: 4.2, utilization: 62 },
  { id: "2", chain: "Solana", asset: "SOL", liquidity: 5000, apy: 5.8, utilization: 45 },
  { id: "3", chain: "X3", asset: "X3", liquidity: 25000000, apy: 6.5, utilization: 38 },
];

export default function CrossChainBridgePanel() {
  const [transfers, setTransfers] = useState<CrossChainTransfer[]>(MOCK_TRANSFERS);
  const [pools, setPools] = useState<BridgePool[]>(MOCK_POOLS);
  const [selectedTransfer, setSelectedTransfer] = useState<CrossChainTransfer | null>(MOCK_TRANSFERS[0]);
  const [selectedPool, setSelectedPool] = useState<BridgePool | null>(MOCK_POOLS[0]);

  const completedCount = transfers.filter((t) => t.status === "completed").length;
  const totalVolume = transfers.reduce((sum, t) => sum + t.amount, 0);
  const totalFees = transfers.reduce((sum, t) => sum + t.fee, 0);

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle size={14} className="text-green-400" />;
      case "confirmed":
        return <Clock size={14} className="text-blue-400" />;
      case "pending":
        return <Clock size={14} className="text-yellow-400 animate-spin" />;
      case "failed":
        return <AlertTriangle size={14} className="text-red-400" />;
      default:
        return null;
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Send size={20} className="text-cyan-400" /> Cross-Chain Bridge
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Volume</div>
            <div className="text-lg font-bold text-cyan-400">${(totalVolume / 1000).toFixed(0)}K</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Completed</div>
            <div className="text-lg font-bold text-green-400">{completedCount}/{transfers.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Fees</div>
            <div className="text-lg font-bold text-purple-400">${totalFees.toFixed(2)}</div>
          </div>
        </div>

        {/* New Transfer Button */}
        <button className="w-full bg-cyan-600 hover:bg-cyan-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
          <Send size={14} /> New Transfer
        </button>

        {/* Transfer History */}
        <div>
          <h3 className="font-semibold mb-2 text-sm">Recent Transfers</h3>
          <div className="space-y-2">
            {transfers.map((transfer) => (
              <button
                key={transfer.id}
                onClick={() => setSelectedTransfer(transfer)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedTransfer?.id === transfer.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm font-semibold flex items-center gap-2">
                    {getStatusIcon(transfer.status)}
                    {transfer.from} → {transfer.to}
                  </div>
                  <span className="text-Green-400 font-bold">{transfer.amount} {transfer.asset}</span>
                </div>

                <div className="flex justify-between text-xs text-gray-400">
                  <span>{transfer.timestamp}</span>
                  <span className={clsx("font-semibold", transfer.status === "completed" ? "text-green-400" : "")}>
                    {transfer.status}
                  </span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Transfer Details */}
        {selectedTransfer && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-sm space-y-3">
            <h3 className="font-semibold">Transfer Details</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">From</span>
                <span className="font-semibold">{selectedTransfer.from}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">To</span>
                <span className="font-semibold">{selectedTransfer.to}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Amount</span>
                <span className="font-bold text-cyan-400">
                  {selectedTransfer.amount} {selectedTransfer.asset}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Fee</span>
                <span className="font-semibold">${selectedTransfer.fee.toFixed(2)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">TX Hash</span>
                <span className="font-mono text-xs">{selectedTransfer.txHash}</span>
              </div>
            </div>

            {selectedTransfer.status === "completed" && (
              <div className="bg-green-600/20 border border-green-600 rounded p-2 text-xs text-green-300">
                ✓ Transfer successful and confirmed on both chains.
              </div>
            )}

            {selectedTransfer.status === "pending" && (
              <div className="bg-yellow-600/20 border border-yellow-600 rounded p-2 text-xs text-yellow-300">
                ⏱ Waiting for validator signatures. Usually 5-10 minutes.
              </div>
            )}
          </div>
        )}

        {/* Bridge Liquidity Pools */}
        <div>
          <h3 className="font-semibold mb-2 text-sm flex items-center gap-2">
            <TrendingUp size={16} /> Bridge Liquidity
          </h3>
          <div className="space-y-2">
            {pools.map((pool) => (
              <button
                key={pool.id}
                onClick={() => setSelectedPool(pool)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedPool?.id === pool.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="text-sm font-semibold">{pool.chain}</div>
                    <div className="text-xs text-gray-400">{pool.asset}</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-cyan-400">${(pool.liquidity / 1000000).toFixed(1)}M</div>
                    <div className="text-xs text-green-400">{pool.apy}% APY</div>
                  </div>
                </div>

                <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                  <div className="h-full bg-gradient-to-r from-cyan-600 to-blue-600" style={{ width: `${pool.utilization}%` }} />
                </div>
                <div className="text-xs text-gray-400 mt-1">Utilization: {pool.utilization}%</div>
              </button>
            ))}
          </div>
        </div>

        {/* Pool Details */}
        {selectedPool && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-sm space-y-3">
            <h3 className="font-semibold">{selectedPool.chain} Pool</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Asset</span>
                <span className="font-semibold">{selectedPool.asset}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Liquidity</span>
                <span className="font-bold text-cyan-400">${(selectedPool.liquidity / 1000000).toFixed(1)}M</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">APY</span>
                <span className="font-bold text-green-400">{selectedPool.apy}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Utilization</span>
                <span className="font-semibold">{selectedPool.utilization}%</span>
              </div>
            </div>

            <button className="w-full bg-cyan-600 hover:bg-cyan-700 py-2 rounded-lg font-semibold text-sm transition">
              Provide Liquidity
            </button>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Trustless cross-chain transfers with atomic settlement.
      </div>
    </div>
  );
}
