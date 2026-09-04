import React, { useState } from 'react';
import { Search, Download, Star, TrendingUp, Users, BarChart3, Zap, Award } from 'lucide-react';

interface Plugin {
  id: string;
  name: string;
  category: string;
  description: string;
  author: string;
  rating: number;
  reviews: number;
  installs: number;
  active: number;
  versionLatest: string;
  updated: string;
  fee?: number;
}

export const IntegrationMarketplacePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'featured' | 'category' | 'trending' | 'installed'>('featured');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);

  const plugins: Plugin[] = [
    {
      id: 'chainlink-oracle',
      name: 'Chainlink Price Oracle',
      category: 'Data Feeds',
      description: 'Real-time price feeds for 1000+ assets with 99.9% uptime guarantee',
      author: 'Chainlink Labs',
      rating: 4.9,
      reviews: 2345,
      installs: 12450,
      active: 8920,
      versionLatest: '3.2.1',
      updated: '5 days ago',
    },
    {
      id: 'uniswap-router',
      name: 'Uniswap V3 Router',
      category: 'DEX Aggregation',
      description: 'Cross-pool routing with MEV protection and dynamic fee optimization',
      author: 'Uniswap Labs',
      rating: 4.8,
      reviews: 1876,
      installs: 9240,
      active: 7340,
      versionLatest: '2.1.5',
      updated: '2 weeks ago',
    },
    {
      id: 'aave-lending',
      name: 'Aave Lending Protocol',
      category: 'Lending',
      description: 'Multi-collateral lending with risk management and liquidation engine',
      author: 'Aave DAO',
      rating: 4.7,
      reviews: 1654,
      installs: 5670,
      active: 4210,
      versionLatest: '5.0.2',
      updated: '10 days ago',
    },
    {
      id: 'graph-indexer',
      name: 'The Graph Subgraph Indexer',
      category: 'Indexing',
      description: 'Queryable subgraph for real-time on-chain data without running a node',
      author: 'The Graph',
      rating: 4.6,
      reviews: 987,
      installs: 6340,
      active: 5120,
      versionLatest: '1.4.2',
      updated: '1 week ago',
      fee: 10,
    },
    {
      id: 'openai-agents',
      name: 'OpenAI Agent Framework',
      category: 'AI/Agents',
      description: 'GPT-4 powered agent framework with tool use and memory management',
      author: 'OpenAI',
      rating: 4.5,
      reviews: 654,
      installs: 3240,
      active: 1890,
      versionLatest: '2.0.1',
      updated: '3 days ago',
      fee: 25,
    },
  ];

  const categories = ['Data Feeds', 'DEX Aggregation', 'Lending', 'Indexing', 'AI/Agents', 'Analytics', 'Security'];

  const filteredPlugins = plugins.filter(p => {
    const matchesSearch = p.name.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = !selectedCategory || p.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-indigo-500 to-purple-500 rounded-lg">
            <Zap className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Integration Marketplace</h1>
            <p className="text-xs text-gray-400">Plugin discovery, ratings, adoption tracking</p>
          </div>
        </div>

        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            placeholder="Search plugins..."
            className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg pl-10 pr-4 py-2 text-white placeholder-gray-600 focus:border-indigo-500 outline-none"
          />
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['featured', 'category', 'trending', 'installed'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-indigo-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'featured' && '⭐ Featured'}
            {tab === 'category' && '📂 Category'}
            {tab === 'trending' && '🔥 Trending'}
            {tab === 'installed' && '✓ Installed'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'category' && (
          <div className="p-4 border-b border-[#2a2a35]">
            <div className="flex gap-2 flex-wrap">
              <button
                onClick={() => setSelectedCategory(null)}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium transition ${
                  selectedCategory === null
                    ? 'bg-indigo-600 text-white'
                    : 'bg-[#1a1a2e] text-gray-400 hover:text-gray-200'
                }`}
              >
                All Categories
              </button>
              {categories.map(cat => (
                <button
                  key={cat}
                  onClick={() => setSelectedCategory(cat)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-medium transition ${
                    selectedCategory === cat
                      ? 'bg-indigo-600 text-white'
                      : 'bg-[#1a1a2e] text-gray-400 hover:text-gray-200'
                  }`}
                >
                  {cat}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="p-4 space-y-3">
          {filteredPlugins.map(plugin => (
            <div key={plugin.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-indigo-500/50 transition">
              <div className="flex items-start justify-between mb-3">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <h3 className="font-semibold text-white">{plugin.name}</h3>
                    <span className="text-xs px-2 py-0.5 bg-purple-500/20 text-purple-400 rounded">
                      {plugin.category}
                    </span>
                    {plugin.fee && (
                      <span className="text-xs px-2 py-0.5 bg-yellow-500/20 text-yellow-400 rounded">
                        ${plugin.fee}/mo
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-gray-400">{plugin.description}</p>
                  <p className="text-xs text-gray-600 mt-1">by {plugin.author}</p>
                </div>
              </div>

              <div className="grid grid-cols-4 gap-2 mb-3">
                <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                  <p className="text-xs text-gray-500">Rating</p>
                  <div className="flex items-center gap-1">
                    <Star size={12} className="text-yellow-400 fill-yellow-400" />
                    <span className="text-sm font-semibold text-white">{plugin.rating}</span>
                  </div>
                  <p className="text-xs text-gray-600">{plugin.reviews} reviews</p>
                </div>
                <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                  <p className="text-xs text-gray-500">Installs</p>
                  <p className="text-sm font-semibold text-white">{(plugin.installs / 1000).toFixed(1)}K</p>
                  <p className="text-xs text-gray-600">{(plugin.active / 1000).toFixed(1)}K active</p>
                </div>
                <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                  <p className="text-xs text-gray-500">Version</p>
                  <p className="text-sm font-semibold text-cyan-400">{plugin.versionLatest}</p>
                  <p className="text-xs text-gray-600">{plugin.updated}</p>
                </div>
                <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                  <p className="text-xs text-gray-500">Adoption</p>
                  <div className="flex items-center gap-1 mb-1">
                    <TrendingUp size={12} className="text-green-400" />
                    <span className="text-xs text-green-400">+12.3%</span>
                  </div>
                  <p className="text-xs text-gray-600">This month</p>
                </div>
              </div>

              <button className="w-full px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded font-medium transition text-sm flex items-center justify-center gap-2">
                <Download size={14} />
                Install Plugin
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default IntegrationMarketplacePanel;
