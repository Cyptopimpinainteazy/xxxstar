import React, { useState } from "react";
import { Lock, Users, Clock, Zap, CheckCircle } from "lucide-react";
import clsx from "clsx";

interface WhitelistTier {
  id: string;
  name: string;
  spots: number;
  claimed: number;
  allocation: number;
  endTime: string;
  active: boolean;
}

export default function WhitelistPresalePanel() {
  const [tiers, setTiers] = useState<WhitelistTier[]>([
    {
      id: "1",
      name: "VIP (Founders & Early Supporters)",
      spots: 100,
      claimed: 87,
      allocation: 1000,
      endTime: "3 days",
      active: true,
    },
    {
      id: "2",
      name: "Tier 1 (NFT Holders)",
      spots: 500,
      claimed: 342,
      allocation: 500,
      endTime: "5 days",
      active: true,
    },
    {
      id: "3",
      name: "Tier 2 (Community)",
      spots: 2000,
      claimed: 1156,
      allocation: 250,
      endTime: "10 days",
      active: true,
    },
  ]);

  const [userStatus, setUserStatus] = useState<"not-whitelisted" | "confirmed" | "claimed">("confirmed");
  const [userAllocation, setUserAllocation] = useState(500);
  const [claimAmount, setClaimAmount] = useState(userAllocation);

  const handleClaim = () => {
    setUserStatus("claimed");
  };

  const totalSpots = tiers.reduce((sum, t) => sum + t.spots, 0);
  const totalClaimed = tiers.reduce((sum, t) => sum + t.claimed, 0);
  const fillPercentage = (totalClaimed / totalSpots) * 100;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Lock size={20} /> Whitelist Presale
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* User Status */}
        {userStatus === "claimed" ? (
          <div className="bg-green-600/10 border border-green-600 rounded-lg p-4 flex items-start gap-3">
            <CheckCircle size={16} className="text-green-400 flex-shrink-0 mt-0.5" />
            <div>
              <div className="font-semibold text-green-400">✓ Tokens Claimed</div>
              <div className="text-xs text-gray-400 mt-1">
                {claimAmount.toLocaleString()} X3 transferred to wallet
              </div>
            </div>
          </div>
        ) : userStatus === "confirmed" ? (
          <div className="bg-blue-600/10 border border-blue-600 rounded-lg p-4 flex items-start gap-3">
            <Users size={16} className="text-blue-400 flex-shrink-0 mt-0.5" />
            <div>
              <div className="font-semibold text-blue-400">✓ Whitelist Confirmed</div>
              <div className="text-xs text-gray-400 mt-1">
                Your allocation: {userAllocation.toLocaleString()} X3
              </div>
            </div>
          </div>
        ) : (
          <div className="bg-yellow-600/10 border border-yellow-600 rounded-lg p-4 flex items-start gap-3">
            <Lock size={16} className="text-yellow-400 flex-shrink-0 mt-0.5" />
            <div>
              <div className="font-semibold text-yellow-400">Not Whitelisted</div>
              <div className="text-xs text-gray-400 mt-1">
                Check tiers below to see requirements
              </div>
            </div>
          </div>
        )}

        {/* Overall Progress */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Presale Progress</h3>
          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Total Spots Filled</span>
              <span className="font-semibold">
                {totalClaimed.toLocaleString()} / {totalSpots.toLocaleString()}
              </span>
            </div>
            <div className="bg-[#2a2a35] rounded-full h-2">
              <div
                className="bg-blue-600 h-2 rounded-full transition-all"
                style={{ width: `${Math.min(fillPercentage, 100)}%` }}
              />
            </div>
            <div className="text-xs text-gray-400">{fillPercentage.toFixed(1)}% full</div>
          </div>
        </div>

        {/* Tiers */}
        <div>
          <h3 className="font-semibold mb-3 flex items-center gap-2">
            <Zap size={16} /> Whitelist Tiers
          </h3>
          <div className="space-y-3">
            {tiers.map((tier) => {
              const tierFill = (tier.claimed / tier.spots) * 100;
              return (
                <div
                  key={tier.id}
                  className={clsx(
                    "p-4 rounded-lg border",
                    tier.active ? "border-[#2a2a35] bg-[#15151b]" : "border-red-600/30 bg-red-600/5 opacity-50"
                  )}
                >
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <h4 className="font-semibold">{tier.name}</h4>
                      <p className="text-xs text-gray-400 mt-1">Per wallet limit: {tier.allocation.toLocaleString()} X3</p>
                    </div>
                    <div className="text-right">
                      <div className="text-xs bg-blue-600 text-white px-2 py-1 rounded font-semibold">
                        {tier.claimed}/{tier.spots}
                      </div>
                    </div>
                  </div>

                  <div className="bg-[#2a2a35] rounded-full h-2 mb-2">
                    <div
                      className="bg-green-600 h-2 rounded-full"
                      style={{ width: `${tierFill}%` }}
                    />
                  </div>

                  <div className="flex items-center justify-between text-xs text-gray-400">
                    <span>{tierFill.toFixed(0)}% full</span>
                    <span className="flex items-center gap-1">
                      <Clock size={12} /> Closes in {tier.endTime}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Claim Section */}
        {userStatus !== "not-whitelisted" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
            <h3 className="font-semibold">Claim Your Allocation</h3>

            <div>
              <label className="text-sm font-medium text-gray-300 block mb-2">
                Amount to Claim (max {userAllocation.toLocaleString()})
              </label>
              <div className="flex gap-2">
                <input
                  type="number"
                  max={userAllocation}
                  value={claimAmount}
                  onChange={(e) => setClaimAmount(Math.min(parseInt(e.target.value) || 0, userAllocation))}
                  className="flex-1 bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white"
                />
                <button
                  onClick={() => setClaimAmount(userAllocation)}
                  className="bg-[#2a2a35] hover:bg-[#3a3a45] px-3 py-2 rounded text-sm font-semibold transition"
                >
                  Max
                </button>
              </div>
            </div>

            {userStatus === "confirmed" && (
              <button
                onClick={handleClaim}
                className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
              >
                <CheckCircle size={14} /> Claim Now
              </button>
            )}
          </div>
        )}

        {/* Requirements */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Whitelist Requirements</h3>
          <div className="space-y-2 text-xs text-gray-400">
            <div className="flex items-start gap-2">
              <span className="text-blue-400 font-bold mt-0.5">•</span>
              <span>VIP: Original X3 token holder with minimum 1000 X3</span>
            </div>
            <div className="flex items-start gap-2">
              <span className="text-blue-400 font-bold mt-0.5">•</span>
              <span>Tier 1: Hold any X3 Ecosystem NFT (verified)</span>
            </div>
            <div className="flex items-start gap-2">
              <span className="text-blue-400 font-bold mt-0.5">•</span>
              <span>Tier 2: Join community Discord + follow social media</span>
            </div>
          </div>
        </div>
      </div>

      <button className="w-full bg-[#15151b] hover:bg-[#1a1a20] py-2 rounded-lg font-semibold text-sm transition border border-[#2a2a35]">
        Learn About This Presale
      </button>
    </div>
  );
}
