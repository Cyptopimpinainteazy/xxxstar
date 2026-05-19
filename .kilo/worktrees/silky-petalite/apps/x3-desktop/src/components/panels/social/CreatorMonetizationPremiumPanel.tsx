import React, { useState } from "react";
import { Star, DollarSign, Users, TrendingUp, Gift, Lock, Eye, Settings } from "lucide-react";
import clsx from "clsx";

interface SubscriptionTier {
  id: string;
  name: string;
  price: number;
  currency: string;
  benefits: string[];
  subscribers: number;
  revenue: number;
}

interface TipPool {
  id: string;
  from: string;
  amount: number;
  currency: string;
  message: string;
  timestamp: string;
}

interface RevenueSplit {
  id: string;
  recipient: string;
  percentage: number;
  address: string;
}

interface CreatorStats {
  totalRevenue: number;
  monthlyRevenue: number;
  totalSubscribers: number;
  totalTips: number;
  avgTipSize: number;
}

const MOCK_TIERS: SubscriptionTier[] = [
  {
    id: "1",
    name: "Basic",
    price: 5,
    currency: "USD",
    benefits: ["Early access to posts", "Community badge", "Monthly newsletter"],
    subscribers: 234,
    revenue: 1170,
  },
  {
    id: "2",
    name: "Pro",
    price: 15,
    currency: "USD",
    benefits: ["All Basic benefits", "1-on-1 consultation", "Private Discord", "Custom NFT"],
    subscribers: 89,
    revenue: 1335,
  },
  {
    id: "3",
    name: "Legend",
    price: 50,
    currency: "USD",
    benefits: ["All Pro benefits", "Monthly live streams", "Business consultation", "Merchandise"],
    subscribers: 12,
    revenue: 600,
  },
];

const MOCK_TIPS: TipPool[] = [
  {
    id: "1",
    from: "Alice",
    amount: 50,
    currency: "X3",
    message: "Love your content! Keep it up!",
    timestamp: "2 hours ago",
  },
  {
    id: "2",
    from: "Bob",
    amount: 100,
    currency: "X3",
    message: "This tutorial saved me hours!",
    timestamp: "4 hours ago",
  },
  {
    id: "3",
    from: "Carol",
    amount: 25,
    currency: "X3",
    message: "Great insights on tokenomics",
    timestamp: "6 hours ago",
  },
];

const MOCK_SPLITS: RevenueSplit[] = [
  {
    id: "1",
    recipient: "Creator (Me)",
    percentage: 85,
    address: "x3c7b...2f4a",
  },
  {
    id: "2",
    recipient: "Platform Fee",
    percentage: 10,
    address: "x3plat...fee1",
  },
  {
    id: "3",
    recipient: "Moderator",
    percentage: 5,
    address: "x3mod...9c8e",
  },
];

const MOCK_STATS: CreatorStats = {
  totalRevenue: 3105,
  monthlyRevenue: 892,
  totalSubscribers: 335,
  totalTips: 5750,
  avgTipSize: 47.5,
};

export default function CreatorMonetizationPremiumPanel() {
  const [tiers, setTiers] = useState<SubscriptionTier[]>(MOCK_TIERS);
  const [tips, setTips] = useState<TipPool[]>(MOCK_TIPS);
  const [splits, setSplits] = useState<RevenueSplit[]>(MOCK_SPLITS);
  const [stats] = useState<CreatorStats>(MOCK_STATS);
  const [activeTab, setActiveTab] = useState<"subscriptions" | "tipping" | "splits" | "analytics">("subscriptions");
  const [editingTier, setEditingTier] = useState<string | null>(null);
  const [newTierPrice, setNewTierPrice] = useState("");

  const handleUpdateTierPrice = (tierId: string) => {
    if (newTierPrice) {
      setTiers(
        tiers.map((t) =>
          t.id === tierId ? { ...t, price: parseFloat(newTierPrice) } : t
        )
      );
      setEditingTier(null);
      setNewTierPrice("");
    }
  };

  const handleRemoveTip = (tipId: string) => {
    setTips(tips.filter((t) => t.id !== tipId));
  };

  const totalSubscriptionRevenue = tiers.reduce((sum, t) => sum + t.revenue, 0);
  const totalTipsRevenue = tips.reduce((sum, t) => sum + t.amount, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Star size={20} className="text-yellow-400" /> Creator Monetization Premium
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Revenue Overview */}
        <div className="grid grid-cols-2 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Revenue (This Month)</div>
            <div className="text-lg font-bold text-green-400">${stats.monthlyRevenue.toLocaleString()}</div>
            <div className="text-xs text-gray-500 mt-1">
              <TrendingUp size={10} className="inline mr-1" />
              +12% vs last month
            </div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Subscribers</div>
            <div className="text-lg font-bold text-cyan-400">{stats.totalSubscribers}</div>
            <div className="text-xs text-gray-500 mt-1">Across all tiers</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["subscriptions", "tipping", "splits", "analytics"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab
                  ? "border-yellow-600 text-yellow-400"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {activeTab === "subscriptions" && (
          <div className="space-y-3">
            {/* Tier Cards */}
            {tiers.map((tier) => (
              <div key={tier.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
                <div className="flex justify-between items-start">
                  <div>
                    <div className="text-sm font-bold">{tier.name}</div>
                    <div className="text-xs text-gray-400 mt-1">{tier.subscribers} active subscribers</div>
                  </div>
                  <div className="text-right">
                    {editingTier === tier.id ? (
                      <div className="flex gap-1">
                        <input
                          type="number"
                          value={newTierPrice}
                          onChange={(e) => setNewTierPrice(e.target.value)}
                          placeholder={tier.price.toString()}
                          className="w-16 bg-[#0a0a0f] border border-[#2a2a35] rounded px-2 py-1 text-xs focus:border-yellow-600 focus:outline-none"
                        />
                        <button
                          onClick={() => handleUpdateTierPrice(tier.id)}
                          className="bg-yellow-600 hover:bg-yellow-700 px-2 py-1 rounded text-xs font-semibold transition"
                        >
                          Save
                        </button>
                      </div>
                    ) : (
                      <div>
                        <div className="text-lg font-bold text-yellow-400">${tier.price}/{tier.currency}</div>
                        <button
                          onClick={() => setEditingTier(tier.id)}
                          className="text-xs text-gray-400 hover:text-yellow-400 transition mt-1"
                        >
                          Edit Price
                        </button>
                      </div>
                    )}
                  </div>
                </div>

                {/* Benefits */}
                <div className="space-y-1">
                  {tier.benefits.map((benefit, idx) => (
                    <div key={idx} className="text-xs text-gray-300 flex items-center gap-2">
                      <span className="text-green-400">✓</span> {benefit}
                    </div>
                  ))}
                </div>

                {/* Revenue */}
                <div className="bg-[#0a0a0f] rounded-lg p-2 flex justify-between items-center">
                  <span className="text-xs text-gray-400">Monthly Revenue</span>
                  <span className="font-bold text-green-400">${tier.revenue.toLocaleString()}</span>
                </div>
              </div>
            ))}

            {/* Add Tier */}
            <button className="w-full border border-dashed border-[#2a2a35] hover:border-yellow-600 rounded-lg p-4 text-center text-sm font-semibold text-gray-400 hover:text-yellow-400 transition">
              + Create New Tier
            </button>

            {/* Total */}
            <div className="bg-yellow-600/10 border border-yellow-600/30 rounded-lg p-3">
              <div className="flex justify-between items-center">
                <span className="font-semibold">Total Subscription Revenue</span>
                <span className="text-lg font-bold text-yellow-400">${totalSubscriptionRevenue.toLocaleString()}</span>
              </div>
            </div>
          </div>
        )}

        {activeTab === "tipping" && (
          <div className="space-y-3">
            {/* Tip Stats */}
            <div className="grid grid-cols-2 gap-2">
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="text-xs text-gray-400 mb-1">Total Tips</div>
                <div className="text-lg font-bold text-pink-400">{totalTipsRevenue.toLocaleString()} X3</div>
              </div>
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="text-xs text-gray-400 mb-1">Avg Tip Size</div>
                <div className="text-lg font-bold text-cyan-400">{stats.avgTipSize.toFixed(1)} X3</div>
              </div>
            </div>

            {/* Recent Tips */}
            <div className="space-y-2">
              {tips.length === 0 ? (
                <div className="text-center text-gray-500 py-8">No tips yet</div>
              ) : (
                tips.map((tip) => (
                  <div key={tip.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                    <div className="flex justify-between items-start mb-2">
                      <div>
                        <div className="font-semibold text-sm">{tip.from}</div>
                        <div className="text-xs text-gray-400">{tip.timestamp}</div>
                      </div>
                      <div className="text-lg font-bold text-pink-400">{tip.amount} {tip.currency}</div>
                    </div>
                    <div className="text-xs text-gray-300 italic">"{tip.message}"</div>
                    <button
                      onClick={() => handleRemoveTip(tip.id)}
                      className="text-xs text-gray-400 hover:text-red-400 transition mt-2"
                    >
                      Dismiss
                    </button>
                  </div>
                ))
              )}
            </div>

            {/* Enable Tipping */}
            <div className="bg-pink-600/10 border border-pink-600/30 rounded-lg p-4">
              <div className="flex items-center justify-between">
                <div>
                  <div className="font-semibold text-sm flex items-center gap-2">
                    <Gift size={14} className="text-pink-400" /> Enable Tipping
                  </div>
                  <div className="text-xs text-gray-400 mt-1">Let supporters send you tips</div>
                </div>
                <input type="checkbox" defaultChecked className="w-5 h-5 accent-pink-600" />
              </div>
            </div>
          </div>
        )}

        {activeTab === "splits" && (
          <div className="space-y-3">
            {/* Revenue Split Visualization */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-sm font-bold mb-3">Revenue Split</div>
              <div className="flex-1 bg-[#2a2a35] rounded-full h-3 overflow-hidden flex mb-3">
                {splits.map((split) => (
                  <div
                    key={split.recipient}
                    className={clsx("h-full", split.recipient.includes("Creator") && "bg-green-600", split.recipient.includes("Platform") && "bg-blue-600", split.recipient.includes("Moderator") && "bg-purple-600")}
                    style={{ width: `${split.percentage}%` }}
                  />
                ))}
              </div>
            </div>

            {/* Split Details */}
            {splits.map((split) => (
              <div key={split.recipient} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start">
                  <div>
                    <div className="font-semibold text-xs">{split.recipient}</div>
                    <div className="text-xs text-gray-400 font-mono mt-0.5">{split.address}</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-sm text-cyan-400">{split.percentage}%</div>
                    <div className="text-xs text-gray-400 mt-0.5">of all revenue</div>
                  </div>
                </div>
              </div>
            ))}

            {/* Update Splits */}
            <button className="w-full bg-[#15151b] border border-[#2a2a35] hover:border-yellow-600 rounded-lg p-3 text-sm font-semibold text-gray-400 hover:text-yellow-400 transition">
              Edit Revenue Splits
            </button>
          </div>
        )}

        {activeTab === "analytics" && (
          <div className="space-y-3">
            <div className="grid grid-cols-2 gap-2">
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="text-xs text-gray-400 mb-1">All-Time Revenue</div>
                <div className="text-lg font-bold text-green-400">${stats.totalRevenue.toLocaleString()}</div>
              </div>
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="text-xs text-gray-400 mb-1">Growth</div>
                <div className="text-lg font-bold text-yellow-400">+47% YoY</div>
              </div>
            </div>

            {/* Revenue Breakdown */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <div className="text-sm font-bold">Revenue Breakdown</div>

              <div className="space-y-2">
                <div className="flex justify-between items-center text-xs">
                  <span className="text-gray-400">Subscriptions</span>
                  <span className="font-bold text-cyan-400">${totalSubscriptionRevenue.toLocaleString()}</span>
                </div>
                <div className="flex-1 bg-[#2a2a35] rounded-full h-2">
                  <div
                    className="h-full bg-cyan-600 rounded-full"
                    style={{ width: `${(totalSubscriptionRevenue / stats.totalRevenue) * 100}%` }}
                  />
                </div>
              </div>

              <div className="space-y-2">
                <div className="flex justify-between items-center text-xs">
                  <span className="text-gray-400">Tips</span>
                  <span className="font-bold text-pink-400">{totalTipsRevenue.toLocaleString()} X3</span>
                </div>
                <div className="flex-1 bg-[#2a2a35] rounded-full h-2">
                  <div
                    className="h-full bg-pink-600 rounded-full"
                    style={{ width: `${(totalTipsRevenue / 5750) * 100}%` }}
                  />
                </div>
              </div>
            </div>

            {/* Top Supporters */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-2">
              <div className="text-sm font-bold mb-3">Top Supporters</div>
              {[
                { name: "Alice", amount: 234 },
                { name: "Bob", amount: 189 },
                { name: "Carol", amount: 145 },
              ].map((supporter, idx) => (
                <div key={idx} className="flex justify-between items-center text-xs">
                  <span>{supporter.name}</span>
                  <span className="font-semibold text-yellow-400">${supporter.amount}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Subscription tiers, tipping pools, and revenue split configuration for creator monetization.
      </div>
    </div>
  );
}
