import React, { useState } from "react";
import { Zap, TrendingUp, Plus, Wallet, CheckCircle, AlertCircle } from "lucide-react";
import clsx from "clsx";

interface LMReward {
  poolId: string;
  poolName: string;
  apy: number;
  tvl: number;
  userShare: number;
  unclaimedRewards: number;
  lockPeriod: string;
}

const MOCK_REWARDS: LMReward[] = [
  { poolId: "1", poolName: "X3/USDC", apy: 45.2, tvl: 45000000, userShare: 2500, unclaimedRewards: 125.5, lockPeriod: "7 days" },
  { poolId: "2", poolName: "ETH/USDC", apy: 28.5, tvl: 120000000, userShare: 1200, unclaimedRewards: 42.8, lockPeriod: "14 days" },
  { poolId: "3", poolName: "SOL/USDC", apy: 52.1, tvl: 35000000, userShare: 800, unclaimedRewards: 78.3, lockPeriod: "3 days" },
];

export default function LiquidityMiningPanel() {
  const [rewards, setRewards] = useState<LMReward[]>(MOCK_REWARDS);
  const [selectedPool, setSelectedPool] = useState<LMReward | null>(MOCK_REWARDS[0]);
  const [claimingId, setClaimingId] = useState<string | null>(null);

  const totalUnclaimed = rewards.reduce((sum, r) => sum + r.unclaimedRewards, 0);
  const totalStaked = rewards.reduce((sum, r) => sum + r.userShare, 0);

  const handleClaim = (poolId: string) => {
    setClaimingId(poolId);
    setTimeout(() => {
      setRewards(rewards.map(r => r.poolId === poolId ? { ...r, unclaimedRewards: 0 } : r));
      setClaimingId(null);
    }, 1500);
  };

  const handleClaimAll = () => {
    setClaimingId("all");
    setTimeout(() => {
      setRewards(rewards.map(r => ({ ...r, unclaimedRewards: 0 })));
      setClaimingId(null);
    }, 1500);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-6">Liquidity Mining Rewards</h2>

      {/* Summary */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Total Unclaimed</div>
          <div className="text-2xl font-bold">${totalUnclaimed.toFixed(2)}</div>
          <div className="text-xs text-green-400 mt-2">Claimable now</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Total Staked</div>
          <div className="text-2xl font-bold">${totalStaked.toLocaleString()}</div>
          <div className="text-xs text-gray-500 mt-2">Across {rewards.length} pools</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Avg APY</div>
          <div className="text-2xl font-bold">{(rewards.reduce((sum, r) => sum + r.apy, 0) / rewards.length).toFixed(1)}%</div>
          <div className="text-xs text-green-400 mt-2">↑ Variable by pool</div>
        </div>
      </div>

      {/* Pools */}
      <div className="flex-1 overflow-y-auto mb-4">
        <div className="space-y-3">
          {rewards.map((reward) => (
            <button
              key={reward.poolId}
              onClick={() => setSelectedPool(reward)}
              className={clsx(
                "w-full text-left p-4 rounded-lg border-2 transition",
                selectedPool?.poolId === reward.poolId
                  ? "border-blue-400 bg-blue-600/10"
                  : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
              )}
            >
              <div className="flex items-center justify-between mb-2">
                <div>
                  <div className="font-semibold">{reward.poolName}</div>
                  <div className="text-xs text-gray-500">{reward.lockPeriod} lock period</div>
                </div>
                <div className="text-right">
                  <div className="text-2xl font-bold text-green-400">{reward.apy.toFixed(1)}%</div>
                  <div className="text-xs text-gray-500">APY</div>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-3 text-sm mb-3">
                <div>
                  <span className="text-xs text-gray-400">Your Stake</span>
                  <div className="font-semibold">${reward.userShare}</div>
                </div>
                <div>
                  <span className="text-xs text-gray-400">TVL</span>
                  <div className="font-semibold">${(reward.tvl / 1000000).toFixed(0)}M</div>
                </div>
                <div>
                  <span className="text-xs text-gray-400">Unclaimed</span>
                  <div className="font-semibold text-yellow-400">${reward.unclaimedRewards.toFixed(2)}</div>
                </div>
              </div>

              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleClaim(reward.poolId);
                }}
                disabled={reward.unclaimedRewards === 0 || claimingId === reward.poolId}
                className={clsx(
                  "w-full py-2 rounded-lg font-semibold text-sm transition",
                  reward.unclaimedRewards === 0
                    ? "bg-gray-500/20 text-gray-400 cursor-not-allowed"
                    : claimingId === reward.poolId
                    ? "bg-yellow-600 text-white"
                    : "bg-green-600 hover:bg-green-700 text-white"
                )}
              >
                {claimingId === reward.poolId ? "Claiming..." : "Claim Rewards"}
              </button>
            </button>
          ))}
        </div>
      </div>

      {/* Claim All Button */}
      {totalUnclaimed > 0 && (
        <button
          onClick={handleClaimAll}
          disabled={claimingId !== null}
          className={clsx(
            "w-full py-3 rounded-lg font-bold flex items-center justify-center gap-2 transition",
            claimingId ? "bg-yellow-600" : "bg-gradient-to-r from-green-600 to-blue-600 hover:from-green-700 hover:to-blue-700"
          )}
        >
          <Zap size={18} />
          {claimingId ? "Claiming All..." : `Claim All ${totalUnclaimed.toFixed(2)} X3`}
        </button>
      )}

      {/* Detail Card */}
      {selectedPool && (
        <div className="bg-[#15151b] border border-[#2a2a35] p-4 rounded-lg mt-4 text-sm">
          <h4 className="font-bold mb-3 flex items-center gap-2">
            <TrendingUp size={16} /> {selectedPool.poolName} Details
          </h4>
          <div className="space-y-2 text-xs">
            <div className="flex justify-between">
              <span className="text-gray-400">Pool APY</span>
              <span className="font-semibold text-green-400">{selectedPool.apy}%</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Your Share</span>
              <span className="font-semibold">{(selectedPool.userShare / selectedPool.tvl * 100).toFixed(3)}%</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Est. Daily Reward</span>
              <span className="font-semibold">${(selectedPool.userShare * selectedPool.apy / 365 / 100).toFixed(2)}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
