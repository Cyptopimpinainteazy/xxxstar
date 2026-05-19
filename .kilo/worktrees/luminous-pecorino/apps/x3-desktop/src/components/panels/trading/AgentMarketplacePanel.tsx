import React, { useState } from "react";
import { Bot, TrendingUp, Download, Star, Users, Zap } from "lucide-react";
import clsx from "clsx";

interface Agent {
  id: string;
  name: string;
  creator: string;
  description: string;
  strategy: string;
  winRate: number;
  totalTrades: number;
  monthlyROI: number;
  subscribers: number;
  price: number;
  rating: number;
  featured: boolean;
  subscriptionFee: number;
}

const MOCK_AGENTS: Agent[] = [
  {
    id: "1",
    name: "Grid Trading Pro",
    creator: "0x1234...5678",
    description: "Automated grid trading bot for high volatility markets",
    strategy: "Grid + momentum",
    winRate: 72,
    totalTrades: 1250,
    monthlyROI: 8.5,
    subscribers: 342,
    price: 5.0,
    rating: 4.8,
    featured: true,
    subscriptionFee: 0.5,
  },
  {
    id: "2",
    name: "Yield Maximizer",
    creator: "0x8765...4321",
    description: "Auto-harvest and compound farming rewards across pools",
    strategy: "Liquidity farming",
    winRate: 95,
    totalTrades: 4890,
    monthlyROI: 12.3,
    subscribers: 721,
    price: 8.0,
    rating: 4.9,
    featured: true,
    subscriptionFee: 1.0,
  },
  {
    id: "3",
    name: "DCA Dollar-Cost Averaging",
    creator: "0xabcd...efgh",
    description: "Smart dollar-cost averaging over configurable time periods",
    strategy: "DCA + rebalance",
    winRate: 68,
    totalTrades: 567,
    monthlyROI: 6.2,
    subscribers: 189,
    price: 3.0,
    rating: 4.5,
    featured: false,
    subscriptionFee: 0.25,
  },
];

export default function AgentMarketplacePanel() {
  const [agents, setAgents] = useState<Agent[]>(MOCK_AGENTS);
  const [selectedAgent, setSelectedAgent] = useState<Agent | null>(MOCK_AGENTS[0]);
  const [userSubscriptions, setUserSubscriptions] = useState<string[]>([]);
  const [sortBy, setSortBy] = useState<"roi" | "subscribers" | "rating" | "price">("roi");

  const sortedAgents = [...agents].sort((a, b) => {
    switch (sortBy) {
      case "roi":
        return b.monthlyROI - a.monthlyROI;
      case "subscribers":
        return b.subscribers - a.subscribers;
      case "rating":
        return b.rating - a.rating;
      case "price":
        return a.price - b.price;
      default:
        return 0;
    }
  });

  const handleSubscribe = (agentId: string) => {
    if (!userSubscriptions.includes(agentId)) {
      setUserSubscriptions([...userSubscriptions, agentId]);
    } else {
      setUserSubscriptions(userSubscriptions.filter((id) => id !== agentId));
    }
  };

  const handlePublishAgent = () => {
    alert("Publishing your strategy as an NFT agent...");
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Bot size={20} className="text-blue-400" /> AI Agent Marketplace
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Action & Filter */}
        <div className="flex gap-2">
          <button
            onClick={handlePublishAgent}
            className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
          >
            <Zap size={14} /> Publish Your Agent
          </button>

          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as any)}
            className="bg-[#15151b] border border-[#2a2a35] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-600"
          >
            <option value="roi">Sort by ROI</option>
            <option value="subscribers">Sort by Subscribers</option>
            <option value="rating">Sort by Rating</option>
            <option value="price">Sort by Price</option>
          </select>
        </div>

        {/* Your Subscriptions */}
        {userSubscriptions.length > 0 && (
          <div className="bg-green-600/20 border border-green-600 rounded-lg p-4">
            <h3 className="font-semibold mb-2 text-sm">Your Active Subscriptions</h3>
            <div className="flex gap-2 flex-wrap">
              {userSubscriptions.map((subId) => {
                const agent = agents.find((a) => a.id === subId);
                return agent ? (
                  <div
                    key={subId}
                    className="bg-[#15151b] border border-green-600 px-3 py-1 rounded text-xs font-semibold flex items-center gap-2"
                  >
                    ✓ {agent.name}
                    <button
                      onClick={() => handleSubscribe(subId)}
                      className="text-red-400 hover:text-red-300 ml-1"
                    >
                      ✕
                    </button>
                  </div>
                ) : null;
              })}
            </div>
          </div>
        )}

        {/* Agent List */}
        <div className="space-y-2">
          {sortedAgents.map((agent) => (
            <button
              key={agent.id}
              onClick={() => setSelectedAgent(agent)}
              className={clsx(
                "w-full text-left p-3 rounded-lg border-2 transition",
                selectedAgent?.id === agent.id
                  ? "border-blue-600 bg-blue-600/10"
                  : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
              )}
            >
              <div className="flex items-start justify-between mb-2">
                <div className="flex-1">
                  <div className="text-sm font-semibold flex items-center gap-2">
                    {agent.name}
                    {agent.featured && <span className="text-xs bg-yellow-600 text-yellow-100 px-2 py-0.5 rounded">featured</span>}
                  </div>
                  <div className="text-xs text-gray-400">by {agent.creator}</div>
                </div>
                <div className="text-right">
                  <div className="text-lg font-bold text-green-400">{agent.monthlyROI}%</div>
                  <div className="text-xs text-gray-400">monthly ROI</div>
                </div>
              </div>

              <div className="flex items-center justify-between mb-2">
                <div className="flex gap-2 text-xs">
                  <span className="flex items-center gap-1">
                    <Star size={12} className="text-yellow-400" /> {agent.rating}
                  </span>
                  <span className="flex items-center gap-1">
                    <Users size={12} /> {agent.subscribers}
                  </span>
                  <span className={clsx("font-semibold", agent.winRate >= 70 ? "text-green-400" : "text-yellow-400")}>
                    {agent.winRate}% win rate
                  </span>
                </div>
                <span className="text-sm font-bold text-blue-400">${agent.subscriptionFee.toFixed(2)}/mo</span>
              </div>

              <div className="text-xs text-gray-400">{agent.description.substring(0, 60)}...</div>
            </button>
          ))}
        </div>

        {/* Selected Agent Details */}
        {selectedAgent && (
          <>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 text-sm">Agent Details</h3>

              <div className="space-y-3 text-sm">
                <div>
                  <div className="text-gray-400 mb-1">Description</div>
                  <div className="text-white">{selectedAgent.description}</div>
                </div>

                <div className="grid grid-cols-2 gap-2">
                  <div className="bg-[#2a2a35] p-2 rounded">
                    <div className="text-xs text-gray-400">Monthly ROI</div>
                    <div className="font-bold text-green-400">{selectedAgent.monthlyROI}%</div>
                  </div>
                  <div className="bg-[#2a2a35] p-2 rounded">
                    <div className="text-xs text-gray-400">Win Rate</div>
                    <div className="font-bold text-blue-400">{selectedAgent.winRate}%</div>
                  </div>
                  <div className="bg-[#2a2a35] p-2 rounded">
                    <div className="text-xs text-gray-400">Total Trades</div>
                    <div className="font-bold">{selectedAgent.totalTrades.toLocaleString()}</div>
                  </div>
                  <div className="bg-[#2a2a35] p-2 rounded">
                    <div className="text-xs text-gray-400">Subscribers</div>
                    <div className="font-bold">{selectedAgent.subscribers}</div>
                  </div>
                </div>

                <div className="border-t border-[#2a2a35] pt-3">
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-400">Strategy</span>
                    <span className="font-semibold">{selectedAgent.strategy}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Rating</span>
                    <span className="flex items-center gap-1">
                      <Star size={12} className="text-yellow-400" />
                      <span className="font-semibold">{selectedAgent.rating}/5.0</span>
                    </span>
                  </div>
                </div>
              </div>
            </div>

            {/* Subscribe Button */}
            <button
              onClick={() => handleSubscribe(selectedAgent.id)}
              className={clsx(
                "w-full py-2 rounded-lg font-semibold text-sm transition",
                userSubscriptions.includes(selectedAgent.id)
                  ? "bg-red-600 hover:bg-red-700"
                  : "bg-green-600 hover:bg-green-700"
              )}
            >
              {userSubscriptions.includes(selectedAgent.id)
                ? `✓ Subscribed (${selectedAgent.subscriptionFee.toFixed(2)} X3/mo)`
                : `Subscribe for ${selectedAgent.subscriptionFee.toFixed(2)} X3/mo`}
            </button>

            {/* Copy Trading Info */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-xs text-gray-400 space-y-2">
              <p>
                <strong>How it works:</strong> Subscribe to copy this agent's trades to your wallet automatically.
              </p>
              <p>
                <strong>Performance verified:</strong> All trades audited on-chain. Cannot be falsified.
              </p>
              <p>
                <strong>Risk:</strong> Past performance ≠ future results. Set your own max drawdown limit.
              </p>
            </div>
          </>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        AI agents earn revenue. Users copy winners. Crypto-native copy trading.
      </div>
    </div>
  );
}
