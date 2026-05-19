import React, { useState } from "react";
import { Image, TrendingUp, Zap, Eye, Download, Star, Badge } from "lucide-react";
import clsx from "clsx";

interface NftCollection {
  id: string;
  name: string;
  floorPrice: number;
  holders: number;
  volume24h: number;
  items: number;
  royalty: number;
  image: string;
  verified: boolean;
}

interface NftItem {
  id: string;
  tokenId: string;
  name: string;
  rarity: number;
  price: number;
  owner: string;
  lastSale: string;
  traits: number;
  image: string;
}

interface NftTrade {
  id: string;
  item: string;
  from: string;
  to: string;
  price: number;
  timestamp: string;
  gasUsed: number;
}

const MOCK_COLLECTIONS: NftCollection[] = [
  {
    id: "1",
    name: "X3 Cosmic Keys",
    floorPrice: 2.5,
    holders: 847,
    volume24h: 125000,
    items: 5000,
    royalty: 7.5,
    image: "🔑",
    verified: true,
  },
  {
    id: "2",
    name: "Chain Artifacts",
    floorPrice: 0.85,
    holders: 3421,
    volume24h: 325000,
    items: 10000,
    royalty: 5.0,
    image: "⛓️",
    verified: true,
  },
  {
    id: "3",
    name: "Pixel Realms",
    floorPrice: 0.12,
    holders: 12453,
    volume24h: 850000,
    items: 100000,
    royalty: 2.5,
    image: "🎮",
    verified: false,
  },
];

const MOCK_ITEMS: NftItem[] = [
  { id: "1", tokenId: "#4521", name: "Cosmic Key #4521", rarity: 92, price: 3.8, owner: "0xUser...f595", lastSale: "6h ago", traits: 8, image: "🔑" },
  { id: "2", tokenId: "#7823", name: "Cosmic Key #7823", rarity: 78, price: 2.3, owner: "0xUser...a234", lastSale: "2d ago", traits: 6, image: "🔑" },
  { id: "3", tokenId: "#1942", name: "Chain Artifact #1942", rarity: 88, price: 1.2, owner: "0xUser...d891", lastSale: "12h ago", traits: 7, image: "⛓️" },
];

const MOCK_TRADES: NftTrade[] = [
  { id: "1", item: "Cosmic Key #4521", from: "0xOld...f123", to: "0xNew...a456", price: 3.8, timestamp: "6 mins ago", gasUsed: 125000 },
  { id: "2", item: "Chain Artifact #1234", from: "0xOld...b789", to: "0xNew...c012", price: 0.95, timestamp: "18 mins ago", gasUsed: 98000 },
];

export default function NftMarketplacePanel() {
  const [collections] = useState<NftCollection[]>(MOCK_COLLECTIONS);
  const [items] = useState<NftItem[]>(MOCK_ITEMS);
  const [trades] = useState<NftTrade[]>(MOCK_TRADES);
  const [activeTab, setActiveTab] = useState<"collections" | "items" | "trades">("collections");

  const totalVolume = collections.reduce((sum, c) => sum + c.volume24h, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Image size={20} className="text-purple-400" /> NFT Marketplace
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Volume</div>
            <div className="text-lg font-bold text-purple-400">${(totalVolume / 1000000).toFixed(1)}M</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Collections</div>
            <div className="text-lg font-bold text-pink-400">{collections.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Avg Floor Price</div>
            <div className="text-lg font-bold text-cyan-400">${(collections.reduce((s, c) => s + c.floorPrice, 0) / collections.length).toFixed(2)}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Recent Trades</div>
            <div className="text-lg font-bold text-orange-400">{trades.length}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["collections", "items", "trades"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-purple-600 text-purple-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Collections Tab */}
        {activeTab === "collections" && (
          <div className="space-y-2">
            {collections.map((col) => (
              <div key={col.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 hover:border-purple-600/50 cursor-pointer transition">
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <div className="text-3xl">{col.image}</div>
                    <div className="flex-1">
                      <div className="flex items-center gap-1">
                        <div className="font-semibold text-sm">{col.name}</div>
                        {col.verified && <Badge size={14} className="text-blue-400" />}
                      </div>
                      <div className="text-xs text-gray-400">{col.items.toLocaleString()} items</div>
                    </div>
                  </div>
                  <Star size={16} className="text-yellow-400" />
                </div>

                <div className="grid grid-cols-3 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">Floor Price</div>
                    <div className="font-bold text-cyan-400">{col.floorPrice.toFixed(2)} X3</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Holders</div>
                    <div className="font-bold text-purple-400">{col.holders}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">24h Volume</div>
                    <div className="font-bold text-green-400">${(col.volume24h / 1000).toFixed(0)}K</div>
                  </div>
                </div>

                <div className="text-xs text-gray-500">Royalty: <span className="text-yellow-400 font-semibold">{col.royalty}%</span></div>
              </div>
            ))}
          </div>
        )}

        {/* Items Tab */}
        {activeTab === "items" && (
          <div className="space-y-2">
            {items.map((item) => (
              <div key={item.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 hover:border-purple-600/50 cursor-pointer transition">
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <div className="text-2xl">{item.image}</div>
                    <div className="flex-1">
                      <div className="font-semibold text-sm">{item.name}</div>
                      <div className="text-xs text-gray-400">{item.tokenId}</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-cyan-400 text-sm">{item.price.toFixed(2)} X3</div>
                    <div className="text-xs text-gray-400">{item.lastSale}</div>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">Rarity</div>
                    <div className="font-bold text-orange-400">{item.rarity}%</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Traits</div>
                    <div className="font-bold text-purple-400">{item.traits}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Owner</div>
                    <div className="font-mono text-xs text-gray-500">{item.owner}</div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-2 flex justify-between">
                  <button className="text-xs font-semibold text-pink-400 hover:text-pink-300">Make Offer</button>
                  <button className="text-xs font-semibold text-cyan-400 hover:text-cyan-300">View Details</button>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Trades Tab */}
        {activeTab === "trades" && (
          <div className="space-y-2">
            {trades.map((trade) => (
              <div key={trade.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold text-sm">{trade.item}</div>
                    <div className="text-xs text-gray-400">{trade.timestamp}</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-cyan-400">{trade.price.toFixed(2)} X3</div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">From</div>
                    <div className="font-mono text-gray-400 text-xs">{trade.from}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">To</div>
                    <div className="font-mono text-gray-400 text-xs">{trade.to}</div>
                  </div>
                </div>

                <div className="text-xs text-gray-500">Gas Used: <span className="text-orange-400 font-semibold">{(trade.gasUsed / 1000).toFixed(0)}K</span></div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        NFT collections, rarity ranking, floor prices, and trading activity.
      </div>
    </div>
  );
}
