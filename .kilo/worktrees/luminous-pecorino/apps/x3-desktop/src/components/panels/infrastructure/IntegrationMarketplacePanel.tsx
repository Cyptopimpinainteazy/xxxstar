import React, { useState } from "react";
import { Puzzle, TrendingUp, Zap, Eye, Download, Star, Badge } from "lucide-react";
import clsx from "clsx";

interface Integration {
  id: string;
  name: string;
  category: string;
  description: string;
  users: number;
  rating: number;
  status: "active" | "beta" | "coming-soon";
  developer: string;
  image: string;
}

interface IntegrationCategory {
  name: string;
  count: number;
  icon: string;
}

interface InstallationStats {
  id: string;
  integrationName: string;
  installations: number;
  activeInstances: number;
  daysSinceAdded: number;
}

const MOCK_INTEGRATIONS: Integration[] = [
  {
    id: "1",
    name: "ChainLink Oracle",
    category: "Oracles",
    description: "Real-time price feeds and external data integration",
    users: 32456,
    rating: 4.8,
    status: "active",
    developer: "ChainLink Team",
    image: "🔗",
  },
  {
    id: "2",
    name: "Uniswap V3 Integration",
    category: "DEX",
    description: "Access to liquidity and swap functionality",
    users: 28932,
    rating: 4.9,
    status: "active",
    developer: "Uniswap Labs",
    image: "🔄",
  },
  {
    id: "3",
    name: "AAVE Lending Protocol",
    category: "Lending",
    description: "Lending and borrowing market integration",
    users: 21543,
    rating: 4.7,
    status: "active",
    developer: "AAVE DAO",
    image: "💰",
  },
  {
    id: "4",
    name: "The Graph Subgraph",
    category: "Indexing",
    description: "Indexed blockchain data queries and analytics",
    users: 15234,
    rating: 4.6,
    status: "beta",
    developer: "The Graph",
    image: "📊",
  },
];

const CATEGORIES: IntegrationCategory[] = [
  { name: "Oracles", count: 8, icon: "🔗" },
  { name: "DEX", count: 12, icon: "🔄" },
  { name: "Lending", count: 5, icon: "💰" },
  { name: "Indexing", count: 4, icon: "📊" },
];

const MOCK_STATS: InstallationStats[] = [
  { id: "1", integrationName: "ChainLink Oracle", installations: 45234, activeInstances: 38234, daysSinceAdded: 156 },
  { id: "2", integrationName: "Uniswap V3", installations: 41234, activeInstances: 35123, daysSinceAdded: 142 },
  { id: "3", integrationName: "AAVE Lending", installations: 28932, activeInstances: 24521, daysSinceAdded: 98 },
];

export default function IntegrationMarketplacePanel() {
  const [integrations] = useState<Integration[]>(MOCK_INTEGRATIONS);
  const [categories] = useState<IntegrationCategory[]>(CATEGORIES);
  const [stats] = useState<InstallationStats[]>(MOCK_STATS);
  const [activeTab, setActiveTab] = useState<"browse" | "categories" | "stats">("browse");
  const [selectedIntegration, setSelectedIntegration] = useState<Integration | null>(integrations[0]);

  const totalUsers = integrations.reduce((sum, i) => sum + i.users, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Puzzle size={20} className="text-pink-400" /> Integration Marketplace
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Integrations</div>
            <div className="text-lg font-bold text-pink-400">{integrations.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Installations</div>
            <div className="text-lg font-bold text-purple-400">{totalUsers.toLocaleString()}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Integrations</div>
            <div className="text-lg font-bold text-green-400">{integrations.filter((i) => i.status === "active").length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Avg Rating</div>
            <div className="text-lg font-bold text-orange-400">
              {((integrations.reduce((sum, i) => sum + i.rating, 0) / integrations.length).toFixed(2))}
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["browse", "categories", "stats"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-pink-600 text-pink-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Browse Tab */}
        {activeTab === "browse" && (
          <div className="space-y-2">
            {integrations.map((integration) => (
              <div
                key={integration.id}
                onClick={() => setSelectedIntegration(integration)}
                className={clsx("bg-[#15151b] border rounded-lg p-3 cursor-pointer transition", selectedIntegration?.id === integration.id ? "border-pink-600" : "border-[#2a2a35] hover:border-pink-600/50")}
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <div className="text-3xl">{integration.image}</div>
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <div className="font-semibold">{integration.name}</div>
                        <span
                          className={clsx(
                            "text-xs px-2 py-1 rounded font-bold",
                            integration.status === "active" && "bg-green-600/20 text-green-400",
                            integration.status === "beta" && "bg-yellow-600/20 text-yellow-400",
                            integration.status === "coming-soon" && "bg-blue-600/20 text-blue-400"
                          )}
                        >
                          {integration.status}
                        </span>
                      </div>
                      <div className="text-xs text-gray-400">{integration.developer}</div>
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <Star size={14} className="text-yellow-400 fill-yellow-400" />
                    <span className="text-sm font-bold text-yellow-400">{integration.rating.toFixed(1)}</span>
                  </div>
                </div>

                <div className="text-xs text-gray-400 mb-2">{integration.description}</div>

                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <div className="text-xs text-gray-500">Category</div>
                    <div className="text-xs font-semibold text-cyan-400">{integration.category}</div>
                  </div>
                  <div>
                    <div className="text-xs text-gray-500">Installations</div>
                    <div className="text-xs font-semibold text-purple-400">{(integration.users / 1000).toFixed(0)}K</div>
                  </div>
                </div>

                <div className="mt-2 flex gap-2">
                  {integration.status === "active" ? (
                    <>
                      <button className="flex-1 bg-pink-600/20 text-pink-400 text-xs font-semibold py-1 rounded hover:bg-pink-600/30">Install</button>
                      <button className="flex-1 bg-purple-600/20 text-purple-400 text-xs font-semibold py-1 rounded hover:bg-purple-600/30">View Docs</button>
                    </>
                  ) : (
                    <button className="w-full bg-gray-600/20 text-gray-400 text-xs font-semibold py-1 rounded cursor-not-allowed">{integration.status}</button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Categories Tab */}
        {activeTab === "categories" && (
          <div className="space-y-2">
            {categories.map((cat) => (
              <div key={cat.name} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 hover:border-pink-600/50 cursor-pointer transition">
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <div className="text-4xl">{cat.icon}</div>
                    <div>
                      <div className="font-semibold">{cat.name}</div>
                      <div className="text-xs text-gray-500">{cat.count} integrations</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-2xl font-bold text-pink-400">{cat.count}</div>
                  </div>
                </div>

                <button className="w-full bg-pink-600/20 text-pink-400 text-sm font-semibold py-2 rounded hover:bg-pink-600/30">Explore Category</button>
              </div>
            ))}
          </div>
        )}

        {/* Stats Tab */}
        {activeTab === "stats" && (
          <div className="space-y-2">
            {stats.map((stat) => (
              <div key={stat.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold text-sm">{stat.integrationName}</div>
                    <div className="text-xs text-gray-400">Added {stat.daysSinceAdded} days ago</div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">Total Installations</div>
                    <div className="font-bold text-cyan-400">{(stat.installations / 1000).toFixed(0)}K</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Active Instances</div>
                    <div className="font-bold text-green-400">{(stat.activeInstances / 1000).toFixed(0)}K</div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="text-xs text-gray-400 mb-1">Adoption Rate</div>
                  <div className="bg-[#2a2a35] rounded-full h-2">
                    <div className="h-full bg-gradient-to-r from-pink-600 to-purple-600 rounded-full" style={{ width: `${(stat.activeInstances / stat.installations) * 100}%` }} />
                  </div>
                  <div className="text-xs font-semibold text-pink-400 mt-1">{((stat.activeInstances / stat.installations) * 100).toFixed(1)}% active</div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Plugin marketplace, integration discovery, adoption metrics, and developer ecosystem.
      </div>
    </div>
  );
}
