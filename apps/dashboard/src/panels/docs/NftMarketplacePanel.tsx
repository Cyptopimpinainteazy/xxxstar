import React, { useState } from 'react';
import { Search, TrendingUp, TrendingDown, Zap, BarChart3, Filter, ShoppingCart } from 'lucide-react';

interface NftCollection {
  id: string;
  name: string;
  image: string;
  floorPrice: number;
  volume24h: number;
  volumeChange: number;
  holders: number;
  items: number;
  verified: boolean;
  trending: 'up' | 'down' | 'stable';
  rarityScore?: number;
  royalties: number;
}

interface NftItem {
  id: string;
  name: string;
  collectionId: string;
  image: string;
  rarity: number; // 1-100
  rarityRank: number;
  traits: string[];
  price?: number;
  lastSale?: number;
  lastSaleTime?: string;
}

export const NftMarketplacePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'collections' | 'items' | 'activity'>('collections');
  const [sortBy, setSortBy] = useState<'floor' | 'volume' | 'trending'>('floor');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCollection, setSelectedCollection] = useState<string | null>(null);

  const collections: NftCollection[] = [
    {
      id: 'pudgy-penguins',
      name: 'Pudgy Penguins',
      image: '🐧',
      floorPrice: 4.32,
      volume24h: 268500,
      volumeChange: 12.5,
      holders: 8420,
      items: 8888,
      verified: true,
      trending: 'up',
      rarityScore: 7.2,
      royalties: 7.5,
    },
    {
      id: 'cool-cats',
      name: 'Cool Cats',
      image: '🐱',
      floorPrice: 2.15,
      volume24h: 125300,
      volumeChange: 8.3,
      holders: 5230,
      items: 9999,
      verified: true,
      trending: 'up',
      rarityScore: 6.8,
      royalties: 5.0,
    },
    {
      id: 'azuki',
      name: 'Azuki',
      image: '👹',
      floorPrice: 8.75,
      volume24h: 542100,
      volumeChange: -3.2,
      holders: 6890,
      items: 10000,
      verified: true,
      trending: 'down',
      rarityScore: 8.4,
      royalties: 5.0,
    },
    {
      id: 'doodles',
      name: 'Doodles',
      image: '🎨',
      floorPrice: 1.48,
      volume24h: 89200,
      volumeChange: 5.7,
      holders: 9120,
      items: 10000,
      verified: true,
      trending: 'stable',
      rarityScore: 6.2,
      royalties: 10.0,
    },
    {
      id: 'chromie-squiggles',
      name: 'Chromie Squiggles',
      image: '✨',
      floorPrice: 0.92,
      volume24h: 45600,
      volumeChange: 2.1,
      holders: 4230,
      items: 8888,
      verified: true,
      trending: 'stable',
      rarityScore: 5.9,
      royalties: 10.0,
    },
    {
      id: 'pixelated-heros',
      name: 'Pixelated Heroes',
      image: '🎮',
      floorPrice: 0.34,
      volume24h: 12400,
      volumeChange: 18.5,
      holders: 2150,
      items: 5000,
      verified: false,
      trending: 'up',
      rarityScore: 4.8,
      royalties: 2.5,
    },
  ];

  const filteredCollections = collections
    .filter(c => c.name.toLowerCase().includes(searchQuery.toLowerCase()))
    .sort((a, b) => {
      if (sortBy === 'floor') return b.floorPrice - a.floorPrice;
      if (sortBy === 'volume') return b.volume24h - a.volume24h;
      return b.volumeChange - a.volumeChange;
    });

  const items: NftItem[] = [
    {
      id: 'pudgy-1',
      name: 'Pudgy Penguin #2345',
      collectionId: 'pudgy-penguins',
      image: '🐧',
      rarity: 89,
      rarityRank: 234,
      traits: ['Glow', 'Halo', 'Goggles', 'Blue'],
      price: 4.85,
      lastSale: 4.32,
    },
    {
      id: 'pudgy-2',
      name: 'Pudgy Penguin #5678',
      collectionId: 'pudgy-penguins',
      image: '🐧',
      rarity: 72,
      rarityRank: 1203,
      traits: ['Happy', 'Orange', 'Crown'],
      lastSale: 3.95,
      lastSaleTime: '2 hours ago',
    },
    {
      id: 'cool-1',
      name: 'Cool Cat #1001',
      collectionId: 'cool-cats',
      image: '🐱',
      rarity: 85,
      rarityRank: 467,
      traits: ['Space Suit', 'Purple', 'Lasers'],
      price: 2.45,
    },
    {
      id: 'azuki-1',
      name: 'Azuki #3456',
      collectionId: 'azuki',
      image: '👹',
      rarity: 94,
      rarityRank: 89,
      traits: ['Oni', 'Golden Horn', 'Red Eyes', 'Smile'],
      price: 12.5,
      lastSale: 11.8,
    },
  ];

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-purple-500 to-pink-500 rounded-lg">
            <ShoppingCart className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">NFT Marketplace</h1>
            <p className="text-xs text-gray-400">Collection discovery, rarity ranking, floor prices</p>
          </div>
        </div>

        {/* Search Bar */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            placeholder="Search collections..."
            className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg pl-10 pr-4 py-2 text-white placeholder-gray-600 focus:border-purple-500 outline-none"
          />
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['collections', 'items', 'activity'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-purple-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'collections' && 'Collections'}
            {tab === 'items' && 'Items'}
            {tab === 'activity' && 'Activity'}
          </button>
        ))}
      </div>

      {/* Sort Controls */}
      {activeTab === 'collections' && (
        <div className="flex gap-2 px-4 py-3 border-b border-[#2a2a35]">
          {(['floor', 'volume', 'trending'] as const).map(sort => (
            <button
              key={sort}
              onClick={() => setSortBy(sort)}
              className={`px-3 py-1 rounded-lg text-xs font-medium transition ${
                sortBy === sort
                  ? 'bg-purple-600 text-white'
                  : 'bg-[#1a1a2e] text-gray-400 hover:text-gray-200'
              }`}
            >
              {sort === 'floor' && '📊 Floor Price'}
              {sort === 'volume' && '📈 Volume'}
              {sort === 'trending' && '⚡ Trending'}
            </button>
          ))}
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'collections' && (
          <div className="p-4 space-y-3">
            {filteredCollections.length === 0 ? (
              <div className="text-center py-8 text-gray-400">
                <p>No collections found</p>
              </div>
            ) : (
              filteredCollections.map(collection => (
                <div
                  key={collection.id}
                  onClick={() => setSelectedCollection(collection.id)}
                  className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-purple-500/50 hover:bg-[#1a1a2e]/80 transition cursor-pointer"
                >
                  <div className="flex items-start gap-4">
                    {/* Collection Image */}
                    <div className="text-4xl">{collection.image}</div>

                    {/* Collection Info */}
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-2">
                        <h3 className="font-semibold text-white">{collection.name}</h3>
                        {collection.verified && (
                          <span className="text-xs px-2 py-1 bg-blue-500/20 text-blue-400 rounded">Verified</span>
                        )}
                        {collection.trending === 'up' && (
                          <TrendingUp className="w-4 h-4 text-green-400" />
                        )}
                        {collection.trending === 'down' && (
                          <TrendingDown className="w-4 h-4 text-red-400" />
                        )}
                      </div>

                      <div className="grid grid-cols-4 gap-3 mb-2">
                        <div>
                          <p className="text-xs text-gray-500">Floor Price</p>
                          <p className="font-semibold text-cyan-400">{collection.floorPrice.toFixed(2)} ETH</p>
                        </div>
                        <div>
                          <p className="text-xs text-gray-500">Volume 24h</p>
                          <p className="font-semibold text-white">${(collection.volume24h / 1000).toFixed(1)}K</p>
                        </div>
                        <div>
                          <p className="text-xs text-gray-500">Holders</p>
                          <p className="font-semibold text-white">{(collection.holders / 1000).toFixed(1)}K</p>
                        </div>
                        <div>
                          <p className="text-xs text-gray-500">Items</p>
                          <p className="font-semibold text-white">{(collection.items / 1000).toFixed(1)}K</p>
                        </div>
                      </div>

                      {/* Stats Bar */}
                      <div className="flex items-center gap-2">
                        <div className="flex-1 bg-[#0a0a0f] rounded-full h-1.5">
                          <div
                            className="h-1.5 rounded-full bg-gradient-to-r from-purple-500 to-pink-500"
                            style={{ width: `${(collection.rarityScore! / 10) * 100}%` }}
                          />
                        </div>
                        <span className={`text-xs font-semibold ${
                          collection.volumeChange > 0 ? 'text-green-400' : 'text-red-400'
                        }`}>
                          {collection.volumeChange > 0 ? '+' : ''}{collection.volumeChange.toFixed(1)}%
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === 'items' && (
          <div className="p-4 space-y-3">
            {items.map(item => (
              <div key={item.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-purple-500/50 transition">
                <div className="flex items-start gap-4">
                  <div className="text-3xl">{item.image}</div>

                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <h3 className="font-semibold text-white">{item.name}</h3>
                      <span className="text-xs px-2 py-1 bg-purple-500/20 text-purple-400 rounded">
                        #{item.rarityRank}
                      </span>
                    </div>

                    <div className="flex items-center gap-4 mb-2">
                      <div>
                        <p className="text-xs text-gray-500">Rarity Score</p>
                        <div className="flex items-center gap-2">
                          <div className="w-24 bg-[#0a0a0f] rounded-full h-2">
                            <div
                              className="h-2 rounded-full bg-gradient-to-r from-yellow-500 to-orange-500"
                              style={{ width: `${item.rarity}%` }}
                            />
                          </div>
                          <span className="text-sm font-semibold text-orange-400">{item.rarity}%</span>
                        </div>
                      </div>

                      {item.price && (
                        <div>
                          <p className="text-xs text-gray-500">Price</p>
                          <p className="text-lg font-semibold text-cyan-400">{item.price.toFixed(2)} ETH</p>
                        </div>
                      )}

                      {item.lastSale && !item.price && (
                        <div>
                          <p className="text-xs text-gray-500">Last Sale</p>
                          <div>
                            <p className="text-sm font-semibold text-gray-300">{item.lastSale.toFixed(2)} ETH</p>
                            <p className="text-xs text-gray-500">{item.lastSaleTime}</p>
                          </div>
                        </div>
                      )}
                    </div>

                    <div className="flex flex-wrap gap-2">
                      {item.traits.map(trait => (
                        <span key={trait} className="text-xs px-2 py-1 bg-[#0a0a0f] border border-[#2a2a35] rounded text-gray-300">
                          {trait}
                        </span>
                      ))}
                    </div>
                  </div>

                  <button className="px-4 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 text-white rounded font-semibold transition">
                    Make Offer
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'activity' && (
          <div className="p-4">
            <div className="space-y-3">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="text-2xl">🐧</div>
                      <div>
                        <p className="font-semibold text-white">Pudgy Penguin #{5678 + i}</p>
                        <p className="text-xs text-gray-500">Sold by @user{i}</p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="font-semibold text-cyan-400">{(4.2 + i * 0.1).toFixed(2)} ETH</p>
                      <p className="text-xs text-gray-500">{i} hour ago</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default NftMarketplacePanel;
