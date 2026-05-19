import React, { useState } from "react";
import { Lock, Calendar, Zap, AlertCircle, ChevronDown } from "lucide-react";
import clsx from "clsx";

interface LiquidityLock {
  id: string;
  tokenName: string;
  amount: number;
  locked: number;
  unlocked: number;
  lockUntil: string;
  provider: string;
  contractAddress: string;
  percentageLocked: number;
}

const MOCK_LOCKS: LiquidityLock[] = [
  {
    id: "1",
    tokenName: "X3-ETH LP",
    amount: 50000,
    locked: 50000,
    unlocked: 0,
    lockUntil: "Jun 28, 2026",
    provider: "Uniswap V3",
    contractAddress: "0x1234...5678",
    percentageLocked: 100,
  },
  {
    id: "2",
    tokenName: "X3-USDC LP",
    amount: 25000,
    locked: 22500,
    unlocked: 2500,
    lockUntil: "Apr 15, 2026",
    provider: "Uniswap V3",
    contractAddress: "0x8765...4321",
    percentageLocked: 90,
  },
];

export default function LiquidityLockPanel() {
  const [selectedLock, setSelectedLock] = useState<LiquidityLock | null>(MOCK_LOCKS[0]);
  const [lockDuration, setLockDuration] = useState(365);
  const [newLockAmount, setNewLockAmount] = useState(10000);
  const [showExtend, setShowExtend] = useState(false);

  const totalLocked = MOCK_LOCKS.reduce((sum, l) => sum + l.locked, 0);
  const daysRemaining = selectedLock
    ? Math.ceil(
        (new Date(selectedLock.lockUntil).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24)
      )
    : 0;

  const handleExtendLock = () => {
    setShowExtend(false);
  };

  const handleAddLock = () => {
    alert(`Adding ${newLockAmount.toLocaleString()} LP tokens to lock...`);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Lock size={20} className="text-green-400" /> Liquidity Locks
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Summary Cards */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Locked</div>
            <div className="text-lg font-bold text-green-400">${totalLocked.toLocaleString()}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Avg. Lock Period</div>
            <div className="text-lg font-bold">365 days</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">User Confidence</div>
            <div className="text-lg font-bold text-blue-400">98%</div>
          </div>
        </div>

        {/* Lock List */}
        <div>
          <h3 className="font-semibold mb-3 text-sm">Active Locks</h3>
          <div className="space-y-2">
            {MOCK_LOCKS.map((lock) => (
              <button
                key={lock.id}
                onClick={() => setSelectedLock(lock)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedLock?.id === lock.id
                    ? "border-green-600 bg-green-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm font-semibold">{lock.tokenName}</div>
                  <span className="text-xs bg-green-600/30 text-green-400 px-2 py-1 rounded border border-green-600">
                    100% Locked
                  </span>
                </div>
                <div className="flex items-center gap-2 mb-2">
                  <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-green-600 to-blue-600"
                      style={{ width: `${lock.percentageLocked}%` }}
                    />
                  </div>
                  <span className="text-xs text-gray-400">{lock.percentageLocked}%</span>
                </div>
                <div className="text-xs text-gray-400">
                  {lock.amount.toLocaleString()} LP • Unlock: {lock.lockUntil}
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Selected Lock Details */}
        {selectedLock && (
          <>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 text-sm">Lock Details</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Lock Amount</span>
                  <span className="font-semibold">{selectedLock.locked.toLocaleString()} LP</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Total Value</span>
                  <span className="font-semibold text-green-400">
                    ${(selectedLock.locked * (250000 / 100000)).toLocaleString()}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Provider</span>
                  <span className="font-semibold">{selectedLock.provider}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Contract</span>
                  <span className="font-mono text-xs text-green-400">{selectedLock.contractAddress}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Days Remaining</span>
                  <span className={clsx("font-semibold", daysRemaining < 30 ? "text-orange-400" : "text-green-400")}>
                    {daysRemaining} days
                  </span>
                </div>
              </div>

              {daysRemaining < 30 && (
                <div className="mt-3 flex gap-2 p-3 bg-orange-600/20 border border-orange-600 rounded-lg items-start">
                  <AlertCircle size={14} className="text-orange-400 flex-shrink-0 mt-0.5" />
                  <div className="text-xs text-orange-300">
                    This lock expires in {daysRemaining} days. Consider extending to maintain user confidence.
                  </div>
                </div>
              )}
            </div>

            {/* Extend Lock */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <button
                onClick={() => setShowExtend(!showExtend)}
                className="w-full flex items-center justify-between font-semibold hover:text-blue-400 transition"
              >
                <span className="flex items-center gap-2">
                  <Calendar size={16} /> Extend Lock Duration
                </span>
                <ChevronDown size={16} className={clsx("transition", showExtend && "rotate-180")} />
              </button>

              {showExtend && (
                <div className="mt-4 space-y-4 pt-4 border-t border-[#2a2a35]">
                  <div>
                    <label className="text-xs text-gray-400 block mb-2">Duration (days)</label>
                    <input
                      type="range"
                      min="30"
                      max="1095"
                      value={lockDuration}
                      onChange={(e) => setLockDuration(Number(e.target.value))}
                      className="w-full"
                    />
                    <div className="text-center font-semibold text-sm text-green-400 mt-2">{lockDuration} days</div>
                  </div>

                  <div className="bg-[#2a2a35] p-3 rounded text-xs space-y-1">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Current Unlock</span>
                      <span>{selectedLock.lockUntil}</span>
                    </div>
                    <div className="border-t border-[#3a3a45] my-2" />
                    <div className="flex justify-between font-semibold text-green-400">
                      <span>New Unlock</span>
                      <span>
                        {new Date(Date.now() + lockDuration * 24 * 60 * 60 * 1000).toLocaleDateString("en-US", {
                          month: "short",
                          day: "numeric",
                          year: "numeric",
                        })}
                      </span>
                    </div>
                  </div>

                  <button
                    onClick={handleExtendLock}
                    className="w-full bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition"
                  >
                    Extend Lock
                  </button>
                </div>
              )}
            </div>

            {/* Add More Lock */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 text-sm flex items-center gap-2">
                <Zap size={16} /> Add More Liquidity Lock
              </h3>

              <div className="space-y-3">
                <div>
                  <label className="text-xs text-gray-400 block mb-2">LP Amount</label>
                  <input
                    type="number"
                    value={newLockAmount}
                    onChange={(e) => setNewLockAmount(Number(e.target.value))}
                    className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-600"
                  />
                  <div className="text-xs text-gray-400 mt-1">
                    ≈ ${(newLockAmount * (250000 / 100000)).toLocaleString()}
                  </div>
                </div>

                <div>
                  <label className="text-xs text-gray-400 block mb-2">Lock Until</label>
                  <input
                    type="date"
                    className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-600"
                    defaultValue={new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString().split("T")[0]}
                  />
                </div>

                <button
                  onClick={handleAddLock}
                  className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition"
                >
                  Lock LP Tokens
                </button>
              </div>
            </div>
          </>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Liquidity locks build user trust and reduce rug pull risks.
      </div>
    </div>
  );
}
