import React, { useState } from 'react';
import { Search, TrendingUp, TrendingDown, Zap, BarChart3, Star, Rocket } from 'lucide-react';

interface Token {
  id: string;
  symbol: string;
  name: string;
  price: number;
  change24h: number;
  marketCap: number;
  volume24h: number;
  holders: number;
  launchDate: string;
  verified: boolean;
  riskScore: number; // 1-10
}

interface TokenPair {
  id: string;
  token0: string;
  token1: string;
  price: number;
  volume24h: number;
  liquidity: number;
  fee: number;
}

export const TokenMarketplacePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'trending' | 'launches' | 'pairs'>('trending');
  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<'market-cap' | 'volume' | 'change'>('market-cap');

  const tokens: Token[] = [
    {
      id: 'x3',
      symbol: 'X3',
      name: 'X3 Chain',
      price: 1.25,
      change24h: 5.2,
      marketCap: 125000000,
      volume24h: 4200000,
      holders: 45230,
      launchDate: '2024-01-15',
      verified: true,
      riskScore: 3,
    },
    {
      id: 'usdc',
      symbol: 'USDC',
      name: 'USD Coin',
      price: 1.0,
      change24h: 0.1,
      marketCap: 33000000000,
      volume24h: 8500000,
      holders: 2340000,
      launchDate: '2018-09-26',
      verified: true,
      riskScore: 1,
    },
    {
      id: 'aibot',
      symbol: 'AIBOT',
      name: 'X3 AI Agents',
      price: 0.82,
      change24h: 12.3,
      marketCap: 82000000,
      volume24h: 2100000,
      holders: 12450,
      launchDate: '2025-06-20',
      verified: true,
      riskScore: 6,
    },
    {
      id: 'defi-x3',
      symbol: 'DEFI',
      name: 'X3 DeFi Protocol',
      price: 2.45,
      change24h: -2.1,
      marketCap: 245000000,
      volume24h: 1850000,
      holders: 28900,
      launchDate: '2025-03-10',
      verified: true,
      riskScore: 5,
    },
  ];

  const pairs: TokenPair[] = [
    { id: '1', token0: 'X3', token1: 'USDC', price: 1.25, volume24h: 4200000, liquidity: 15000000, fee: 0.3 },
    { id: '2', token0: 'ETH', token1: 'USDC', price: 3245.8, volume24h: 3200000, liquidity: 120000000, fee: 0.05 },
    { id: '3', token0: 'SOL', token1: 'USDC', price: 178.42, volume24h: 2100000, liquidity: 85000000, fee: 0.3 },
    { id: '4', token0: 'AIBOT', token1: 'X3', price: 0.656, volume24h: 850000, liquidity: 5200000, fee: 1.0 },
  ];

  const filteredTokens = tokens
    .filter(t => t.symbol.toLowerCase().includes(searchQuery.toLowerCase()))
    .sort((a, b) => {
      if (sortBy === 'market-cap') return b.marketCap - a.marketCap;
      if (sortBy === 'volume') return b.volume24h - a.volume24h;
      return b.change24h - a.change24h;
    });

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-yellow-500 to-orange-500 rounded-lg">
            <BarChart3 className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Token Marketplace</h1>
            <p className="text-xs text-gray-400">Token listings, market cap ranking, launch tracking</p>
          </div>
        </div>

        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            placeholder="Search tokens..."
            className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg pl-10 pr-4 py-2 text-white placeholder-gray-600 focus:border-yellow-500 outline-none"
          />
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['trending', 'launches', 'pairs'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-yellow-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'trending' && '📊 Trending'}
            {tab === 'launches' && '🚀 New Launches'}
            {tab === 'pairs' && '🔗 Trading Pairs'}
          </button>
        ))}
      </div>

      {/* Sort Controls */}
      {activeTab === 'trending' && (
        <div className="flex gap-2 px-4 py-3 border-b border-[#2a2a35]">
          {(['market-cap', 'volume', 'change'] as const).map(sort => (
            <button
              key={sort}
              onClick={() => setSortBy(sort)}
              className={`px-3 py-1 rounded-lg text-xs font-medium transition ${
                sortBy === sort
                  ? 'bg-yellow-600 text-white'
                  : 'bg-[#1a1a2e] text-gray-400 hover:text-gray-200'
              }`}
            >
              {sort === 'market-cap' && 'Market Cap'}
              {sort === 'volume' && 'Volume'}
              {sort === 'change' && 'Change'}
            </button>
          ))}
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'trending' && (
          <div className="p-4 space-y-3">
            {filteredTokens.map(token => (
              <div key={token.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-yellow-500/50 transition">
                <div className="flex items-start justify-between mb-3">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h3 className="font-semibold text-white">{token.symbol}</h3>
                      {token.verified && <span className="text-xs px-2 py-0.5 bg-blue-500/20 text-blue-400 rounded">Verified</span>}
                      <span className={`text-xs px-2 py-0.5 rounded ${
                        token.riskScore <= 3 ? 'bg-green-500/20 text-green-400' :
                        token.riskScore <= 6 ? 'bg-yellow-500/20 text-yellow-400' :
                        'bg-red-500/20 text-red-400'
                      }`}>
                        Risk {token.riskScore}/10
                      </span>
                    </div>
                    <p className="text-xs text-gray-400">{token.name}</p>
                  </div>
                  <div className="text-right">
                    <p className="text-lg font-semibold text-cyan-400">${token.price.toFixed(4)}</p>
                    <p className={`text-sm font-medium ${token.change24h > 0 ? 'text-green-400' : 'text-red-400'}`}>
                      {token.change24h > 0 ? '+' : ''}{token.change24h.toFixed(1)}%
                    </p>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-3 mb-3">
                  <div>
                    <p className="text-xs text-gray-500">Market Cap</p>
                    <p className="font-semibold text-white">${(token.marketCap / 1000000).toFixed(1)}M</p>
                  </div>
                  <div>
                    <p className="text-xs text-gray-500">24h Volume</p>
                    <p className="font-semibold text-white">${(token.volume24h / 1000000).toFixed(2)}M</p>
                  </div>
                  <div>
                    <p className="text-xs text-gray-500">Holders</p>
                    <p className="font-semibold text-white">{(token.holders / 1000).toFixed(1)}K</p>
                  </div>
                </div>

                <div className="flex gap-2">
                  <button className="flex-1 px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded font-medium transition text-sm">
                    Buy {token.symbol}
                  </button>
                  <button className="px-4 py-2 bg-[#0a0a0f] border border-[#2a2a35] rounded text-gray-300 hover:border-gray-500 transition font-medium text-sm">
                    Chart
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'launches' && (
          <div className="p-4 space-y-3">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-yellow-500/50 transition">
              <div className="flex items-start gap-4 mb-3">
                <Rocket className="w-5 h-5 text-orange-400 mt-1" />
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <h3 className="font-semibold text-white">AIBOT</h3>
                    <span className="text-xs px-2 py-0.5 bg-orange-500/20 text-orange-400 rounded">New</span>
                  </div>
                  <p className="text-xs text-gray-400">Launch: June 20, 2025 • Bonding Curve → AMM</p>
                </div>
              </div>

              <div className="bg-[#0a0a0f] rounded p-3 mb-3">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs text-gray-500">Bonding Curve Progress</span>
                  <span className="text-xs font-semibold text-orange-400">$82M/$100M</span>
                </div>
                <div className="w-full h-2 bg-[#2a2a35] rounded-full overflow-hidden">
                  <div className="h-2 bg-gradient-to-r from-orange-500 to-yellow-500" style={{ width: '82%' }} />
                </div>
              </div>

              <button className="w-full px-4 py-2 bg-orange-600 hover:bg-orange-700 text-white rounded font-medium transition text-sm">
                Buy on Bonding Curve
              </button>
            </div>
          </div>
        )}

        {activeTab === 'pairs' && (
          <div className="p-4 space-y-3">
            {pairs.map(pair => (
              <div key={pair.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-yellow-500/50 transition">
                <div className="flex items-center justify-between mb-3">
                  <h3 className="font-semibold text-white">{pair.token0}/{pair.token1}</h3>
                  <span className="text-xs px-2 py-1 bg-purple-500/20 text-purple-400 rounded">{pair.fee}% Fee</span>
                </div>

                <div className="grid grid-cols-3 gap-3">
                  <div>
                    <p className="text-xs text-gray-500">Price</p>
                    <p className="font-semibold text-cyan-400">${pair.price.toFixed(4)}</p>
                  </div>
                  <div>
                    <p className="text-xs text-gray-500">24h Volume</p>
                    <p className="font-semibold text-white">${(pair.volume24h / 1000000).toFixed(2)}M</p>
                  </div>
                  <div>
                    <p className="text-xs text-gray-500">Liquidity</p>
                    <p className="font-semibold text-white">${(pair.liquidity / 1000000).toFixed(1)}M</p>
                  </div>
                </div>

                <button className="w-full mt-3 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition text-sm">
                  Trade Pair
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default TokenMarketplacePanel;
