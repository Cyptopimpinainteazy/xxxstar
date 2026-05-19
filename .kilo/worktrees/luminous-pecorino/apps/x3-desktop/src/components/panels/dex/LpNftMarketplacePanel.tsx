import React, { useState } from "react";
import { ShoppingCart, Filter, TrendingUp, Zap, Heart } from "lucide-react";
import clsx from "clsx";

interface LpNft {
  id: string;
  pair: string;
  liquidity: number;
  floorPrice: number;
  volume24h: number;
  owner: string;
  poolFee: string;
  range: string;
  liked: boolean;
  trending: boolean;
}

const MOCK_LNFTS: LpNft[] = [
  {
    id: "1",
    pair: "X3/USDC",
    liquidity: 125000,
    floorPrice: 3500,
    volume24h: 45000,
    owner: "0x123...456",
    poolFee: "0.05%",
    range: "-10% to +10%",
    liked: false,
    trending: true,
  },
  {
    id: "2",
    pair: "ETH/USDC",
    liquidity: 250000,
    floorPrice: 5200,
    volume24h: 125000,
    owner: "0x456...789",
    poolFee: "0.30%",
    range: "-5% to +5%",
    liked: false,
    trending: true,
  },
  {
    id: "3",
    pair: "SOL/USDC",
    liquidity: 85000,
    floorPrice: 2100,
    volume24h: 32000,
    owner: "0x789...abc",
    poolFee: "0.30%",
    range: "-20% to +20%",
    liked: false,
    trending: false,
  },
];

export default function LpNftMarketplacePanel() {
  const [nfts, setNfts] = useState<LpNft[]>(MOCK_LNFTS);
  const [selectedNft, setSelectedNft] = useState<LpNft | null>(null);
  const [filterPair, setFilterPair] = useState("all");
  const [sortBy, setSortBy] = useState<"price" | "volume" | "liquidity">("price");
  const [showBuyModal, setShowBuyModal] = useState(false);

  const filteredNfts = filterPair === "all" ? nfts : nfts.filter((n) => n.pair.includes(filterPair));

  const sortedNfts = [...filteredNfts].sort((a, b) => {
    if (sortBy === "price") return a.floorPrice - b.floorPrice;
    if (sortBy === "volume") return b.volume24h - a.volume24h;
    return b.liquidity - a.liquidity;
  });

  const toggleLike = (nftId: string) => {
    setNfts(nfts.map((n) => (n.id === nftId ? { ...n, liked: !n.liked } : n)));
  };

  const handleBuy = () => {
    setNfts(nfts.filter((n) => n.id !== selectedNft?.id));
    setSelectedNft(null);
    setShowBuyModal(false);
  };

  const uniquePairs = Array.from(new Set(nfts.map((n) => n.pair)));

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <ShoppingCart size={20} /> LP Position NFTs
      </h2>

      {/* Filters & Sort */}
      <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 mb-4">
        <div className="grid grid-cols-3 gap-3">
          {/* Pair Filter */}
          <div>
            <label className="text-xs text-gray-400 uppercase mb-1 block">Pair</label>
            <select
              value={filterPair}
              onChange={(e) => setFilterPair(e.target.value)}
              className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
            >
              <option value="all">All Pairs</option>
              {uniquePairs.map((pair) => (
                <option key={pair} value={pair}>
                  {pair}
                </option>
              ))}
            </select>
          </div>

          {/* Sort By */}
          <div>
            <label className="text-xs text-gray-400 uppercase mb-1 block">Sort</label>
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
              className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
            >
              <option value="price">Floor Price</option>
              <option value="volume">24h Volume</option>
              <option value="liquidity">Liquidity</option>
            </select>
          </div>

          {/* Results Count */}
          <div className="flex items-end">
            <div className="text-sm text-gray-400">
              {sortedNfts.length} position{sortedNfts.length !== 1 ? "s" : ""} found
            </div>
          </div>
        </div>
      </div>

      {/* NFT Grid */}
      <div className="flex-1 overflow-y-auto mb-4">
        <div className="grid grid-cols-1 gap-3">
          {sortedNfts.map((nft) => (
            <button
              key={nft.id}
              onClick={() => setSelectedNft(nft)}
              className={clsx(
                "text-left p-4 rounded-lg border-2 transition",
                selectedNft?.id === nft.id
                  ? "border-blue-400 bg-blue-600/10"
                  : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
              )}
            >
              <div className="flex items-start justify-between mb-2">
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="font-semibold">{nft.pair}</h3>
                    {nft.trending && (
                      <span className="flex items-center gap-1 text-xs bg-orange-600 text-white px-2 py-0.5 rounded">
                        <TrendingUp size={12} /> Trending
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-gray-400">Range: {nft.range}</p>
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleLike(nft.id);
                  }}
                  className={clsx("transition", nft.liked ? "text-red-500" : "text-gray-400 hover:text-red-500")}
                >
                  <Heart size={16} className={nft.liked ? "fill-red-500" : ""} />
                </button>
              </div>

              <div className="grid grid-cols-4 gap-2 text-sm">
                <div>
                  <div className="text-xs text-gray-400">Floor Price</div>
                  <div className="font-semibold text-green-400">${nft.floorPrice.toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-xs text-gray-400">Liquidity</div>
                  <div className="font-semibold">${nft.liquidity.toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-xs text-gray-400">24h Volume</div>
                  <div className="font-semibold text-blue-400">${(nft.volume24h / 1000).toFixed(1)}K</div>
                </div>
                <div>
                  <div className="text-xs text-gray-400">Fee Tier</div>
                  <div className="font-semibold">{nft.poolFee}</div>
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Detail Panel */}
      {selectedNft && !showBuyModal && (
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
          <div>
            <h3 className="font-bold text-lg mb-1">{selectedNft.pair}</h3>
            <p className="text-xs text-gray-400">Owner: {selectedNft.owner}</p>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="bg-[#2a2a35] p-3 rounded-lg">
              <div className="text-xs text-gray-400 mb-1">Floor Price</div>
              <div className="text-2xl font-bold text-green-400">${selectedNft.floorPrice.toLocaleString()}</div>
            </div>
            <div className="bg-[#2a2a35] p-3 rounded-lg">
              <div className="text-xs text-gray-400 mb-1">Liquidity</div>
              <div className="text-2xl font-bold text-blue-400">${selectedNft.liquidity.toLocaleString()}</div>
            </div>
          </div>

          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Pool Fee</span>
              <span className="font-semibold">{selectedNft.poolFee}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Price Range</span>
              <span className="font-semibold">{selectedNft.range}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">24h Volume</span>
              <span className="font-semibold text-blue-400">${selectedNft.volume24h.toLocaleString()}</span>
            </div>
          </div>

          <button
            onClick={() => setShowBuyModal(true)}
            className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
          >
            <ShoppingCart size={14} /> Buy Now
          </button>
        </div>
      )}

      {/* Buy Modal */}
      {showBuyModal && selectedNft && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-6 w-96">
            <h3 className="text-lg font-bold mb-4">Purchase LP NFT</h3>

            <div className="bg-[#15151b] border border-[#2a2a35] p-4 rounded-lg mb-4">
              <div className="flex items-center justify-between mb-2">
                <span className="text-gray-400">{selectedNft.pair}</span>
                <span className="font-bold text-green-400">${selectedNft.floorPrice}</span>
              </div>
              <div className="text-xs text-gray-400">
                {selectedNft.pair} LP Position | Range: {selectedNft.range}
              </div>
            </div>

            <div className="space-y-2 mb-4 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Price</span>
                <span className="font-semibold">${selectedNft.floorPrice}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Platform Fee (2%)</span>
                <span className="font-semibold">${Math.floor(selectedNft.floorPrice * 0.02)}</span>
              </div>
              <div className="border-t border-[#2a2a35] pt-2 flex justify-between font-bold">
                <span>Total</span>
                <span className="text-green-400">${Math.floor(selectedNft.floorPrice * 1.02)}</span>
              </div>
            </div>

            <div className="flex gap-2">
              <button
                onClick={() => setShowBuyModal(false)}
                className="flex-1 bg-[#15151b] border border-[#2a2a35] py-2 rounded-lg text-sm font-semibold hover:bg-[#1a1a20]"
              >
                Cancel
              </button>
              <button
                onClick={handleBuy}
                className="flex-1 bg-green-600 hover:bg-green-700 py-2 rounded-lg text-sm font-semibold transition flex items-center justify-center gap-2"
              >
                <Zap size={14} /> Confirm Purchase
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
