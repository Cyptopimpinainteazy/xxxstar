import React, { useState } from "react";
import { Heart, Zap, TrendingUp, Users, Calendar, Share2 } from "lucide-react";
import clsx from "clsx";

interface Supporter {
  id: string;
  name: string;
  avatar: string;
  totalTips: number;
  subscriptionTier: "none" | "bronze" | "silver" | "gold";
  subscriptionMonths: number;
  lastTip: string;
  message: string;
}

const MOCK_SUPPORTERS: Supporter[] = [
  {
    id: "1",
    name: "CryptoMax",
    avatar: "🦸",
    totalTips: 250,
    subscriptionTier: "gold",
    subscriptionMonths: 6,
    lastTip: "2 hours ago",
    message: "Love your content! Keep it up 🚀",
  },
  {
    id: "2",
    name: "DefiDaisy",
    avatar: "🌼",
    totalTips: 180,
    subscriptionTier: "silver",
    subscriptionMonths: 3,
    lastTip: "5 hours ago",
    message: "This tutorial changed my life",
  },
  {
    id: "3",
    name: "NftNinja",
    avatar: "🥷",
    totalTips: 420,
    subscriptionTier: "gold",
    subscriptionMonths: 12,
    lastTip: "yesterday",
    message: "Your analysis is unmatched. Thanks!",
  },
];

const TIER_COLORS = {
  none: "bg-gray-500/20 text-gray-300",
  bronze: "bg-orange-500/20 text-orange-300",
  silver: "bg-slate-500/20 text-slate-300",
  gold: "bg-yellow-500/20 text-yellow-300",
};

const TIER_BENEFITS = {
  bronze: { price: 5, perks: ["Early access", "Badge"] },
  silver: { price: 15, perks: ["All bronze +", "Priority support", "Emote access"] },
  gold: { price: 50, perks: ["All silver +", "Custom perks", "Direct DM access"] },
};

export default function CreatorMonetizationPanel() {
  const [supporters, setSupporters] = useState<Supporter[]>(MOCK_SUPPORTERS);
  const [showTipModal, setShowTipModal] = useState(false);
  const [activeTab, setActiveTab] = useState<"supporters" | "subscriptions" | "earnings">("supporters");
  const [tipAmount, setTipAmount] = useState(10);

  const totalTips = supporters.reduce((sum, s) => sum + s.totalTips, 0);
  const totalSubscriptions = supporters.filter(s => s.subscriptionTier !== "none").length;
  const monthlySubscriptionRevenue = supporters.reduce((sum, s) => {
    if (s.subscriptionTier === "bronze") return sum + 5;
    if (s.subscriptionTier === "silver") return sum + 15;
    if (s.subscriptionTier === "gold") return sum + 50;
    return sum;
  }, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">Creator Monetization</h2>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-3 mb-6">
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Total Tips</div>
          <div className="text-2xl font-bold">${totalTips}</div>
          <div className="text-xs text-gray-500 mt-1">{supporters.length} supporters</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Monthly Subs</div>
          <div className="text-2xl font-bold">${monthlySubscriptionRevenue}</div>
          <div className="text-xs text-gray-500 mt-1">{totalSubscriptions} active</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-2">Total Revenue</div>
          <div className="text-2xl font-bold">${totalTips + monthlySubscriptionRevenue}</div>
          <div className="text-xs text-green-400 mt-1">↑ 12% this month</div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 mb-4 border-b border-[#2a2a35]">
        {(["supporters", "subscriptions", "earnings"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={clsx(
              "px-4 py-2 text-sm font-semibold border-b-2 transition",
              activeTab === tab
                ? "border-blue-500 text-blue-400"
                : "border-transparent text-gray-400 hover:text-white"
            )}
          >
            {tab === "supporters" && "Supporters"}
            {tab === "subscriptions" && "Subscriptions"}
            {tab === "earnings" && "Earnings"}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === "supporters" && (
          <div className="space-y-3">
            {supporters.map((supporter) => (
              <div key={supporter.id} className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-3 flex-1">
                    <span className="text-2xl">{supporter.avatar}</span>
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-semibold">{supporter.name}</span>
                        {supporter.subscriptionTier !== "none" && (
                          <span className={clsx("text-xs px-2 py-0.5 rounded-full font-semibold", TIER_COLORS[supporter.subscriptionTier])}>
                            {supporter.subscriptionTier.toUpperCase()}
                          </span>
                        )}
                      </div>
                      <div className="text-xs text-gray-500 mt-0.5">{supporter.lastTip}</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="flex items-center gap-1 justify-end mb-1">
                      <Heart size={14} className="text-red-400" />
                      <span className="font-semibold">${supporter.totalTips}</span>
                    </div>
                    {supporter.subscriptionTier !== "none" && (
                      <div className="text-xs text-gray-500">{supporter.subscriptionMonths}m sub</div>
                    )}
                  </div>
                </div>
                <p className="text-sm text-gray-400 italic">"{supporter.message}"</p>
              </div>
            ))}
          </div>
        )}

        {activeTab === "subscriptions" && (
          <div className="space-y-4">
            {(["bronze", "silver", "gold"] as const).map((tier) => (
              <div key={tier} className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <span className="text-lg font-bold capitalize">{tier}</span>
                    <span className={clsx("text-xs px-2 py-0.5 rounded-full font-semibold", TIER_COLORS[tier])}>
                      ${TIER_BENEFITS[tier].price}/mo
                    </span>
                  </div>
                  <span className="text-sm text-gray-400">
                    {supporters.filter(s => s.subscriptionTier === tier).length} subscribers
                  </span>
                </div>
                <div className="flex flex-wrap gap-2">
                  {TIER_BENEFITS[tier].perks.map((perk, i) => (
                    <span key={i} className="text-xs bg-[#2a2a35] px-2 py-1 rounded text-gray-300">
                      ✓ {perk}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "earnings" && (
          <div className="space-y-4">
            <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
              <div className="flex items-center gap-2 mb-3">
                <TrendingUp size={18} className="text-green-400" />
                <span className="font-semibold">Revenue Breakdown</span>
              </div>
              <div className="space-y-3">
                <div>
                  <div className="flex justify-between mb-1 text-sm">
                    <span className="text-gray-400">Tips (One-time)</span>
                    <span className="font-semibold">${totalTips}</span>
                  </div>
                  <div className="w-full bg-[#2a2a35] rounded-full h-2">
                    <div className="bg-red-500 h-2 rounded-full" style={{width: "40%"}} />
                  </div>
                </div>
                <div>
                  <div className="flex justify-between mb-1 text-sm">
                    <span className="text-gray-400">Subscriptions (Recurring)</span>
                    <span className="font-semibold">${monthlySubscriptionRevenue}</span>
                  </div>
                  <div className="w-full bg-[#2a2a35] rounded-full h-2">
                    <div className="bg-green-500 h-2 rounded-full" style={{width: "60%"}} />
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
              <div className="flex items-center gap-2 mb-3">
                <Calendar size={18} className="text-blue-400" />
                <span className="font-semibold">30-Day Performance</span>
              </div>
              <div className="grid grid-cols-2 gap-3 text-sm">
                <div>
                  <span className="text-gray-400">Avg Daily Tips</span>
                  <div className="font-semibold">${(totalTips / 30).toFixed(0)}</div>
                </div>
                <div>
                  <span className="text-gray-400">New Supporters</span>
                  <div className="font-semibold text-green-400">+{Math.floor(supporters.length * 0.4)}</div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Send Tip Button */}
      <button
        onClick={() => setShowTipModal(true)}
        className="w-full mt-4 bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold flex items-center justify-center gap-2 transition"
      >
        <Heart size={16} /> Send Tip
      </button>

      {/* Tip Modal */}
      {showTipModal && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-6 w-80">
            <h3 className="text-lg font-bold mb-4">Send a Tip</h3>

            <div className="space-y-4">
              <div>
                <label className="text-sm text-gray-400 block mb-2">Amount (USDC)</label>
                <input
                  type="number"
                  value={tipAmount}
                  onChange={(e) => setTipAmount(Number(e.target.value))}
                  className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white"
                />
              </div>

              <div className="flex gap-2">
                {[5, 10, 25, 50].map((amt) => (
                  <button
                    key={amt}
                    onClick={() => setTipAmount(amt)}
                    className={clsx(
                      "flex-1 py-1 rounded text-sm font-semibold transition",
                      tipAmount === amt ? "bg-blue-600" : "bg-[#15151b] border border-[#2a2a35]"
                    )}
                  >
                    ${amt}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex gap-2 mt-6">
              <button
                onClick={() => setShowTipModal(false)}
                className="flex-1 bg-[#15151b] border border-[#2a2a35] py-2 rounded font-semibold hover:bg-[#1a1a20] transition"
              >
                Cancel
              </button>
              <button
                onClick={() => { setSupporters([...supporters, {
                  id: String(supporters.length + 1),
                  name: "You",
                  avatar: "❤️",
                  totalTips: tipAmount,
                  subscriptionTier: "none",
                  subscriptionMonths: 0,
                  lastTip: "now",
                  message: "Thanks for your content!",
                }]); setShowTipModal(false); }}
                className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded font-semibold transition"
              >
                Send ${tipAmount}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
