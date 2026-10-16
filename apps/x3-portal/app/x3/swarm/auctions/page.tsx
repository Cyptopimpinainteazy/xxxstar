'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';

interface Auction {
  id: number;
  type: 'BLOCKSPACE' | 'VALIDATOR_SLOT' | 'MEV_BUNDLE' | 'PRIORITY_LANE';
  title: string;
  description: string;
  startPrice: number;
  currentPrice: number;
  minPrice: number;
  decayRate: number;
  startTime: Date;
  endTime: Date;
  bidCount: number;
  highestBidder: string;
  status: 'PENDING' | 'ACTIVE' | 'ENDED' | 'SETTLED';
}

interface Bid {
  id: number;
  auctionId: number;
  bidder: string;
  amount: number;
  timestamp: Date;
  status: 'PENDING' | 'CONFIRMED' | 'OUTBID' | 'WON';
}

const MOCK_AUCTIONS: Auction[] = [
  {
    id: 1,
    type: 'BLOCKSPACE',
    title: 'Priority Blockspace Slot #12847',
    description: 'Guaranteed inclusion in next 10 blocks with priority ordering',
    startPrice: 10000,
    currentPrice: 7234,
    minPrice: 1000,
    decayRate: 5,
    startTime: new Date(Date.now() - 3600000),
    endTime: new Date(Date.now() + 7200000),
    bidCount: 14,
    highestBidder: '0x7a23...f891',
    status: 'ACTIVE',
  },
  {
    id: 2,
    type: 'VALIDATOR_SLOT',
    title: 'Validator Slot Epoch #3421',
    description: 'Block proposal rights for epoch 3421-3430 (10 epochs)',
    startPrice: 50000,
    currentPrice: 42100,
    minPrice: 5000,
    decayRate: 3,
    startTime: new Date(Date.now() - 7200000),
    endTime: new Date(Date.now() + 3600000),
    bidCount: 8,
    highestBidder: '0x9b45...c234',
    status: 'ACTIVE',
  },
  {
    id: 3,
    type: 'MEV_BUNDLE',
    title: 'MEV Bundle Rights Block #28947',
    description: 'Exclusive MEV extraction rights for specified block range',
    startPrice: 25000,
    currentPrice: 18500,
    minPrice: 2500,
    decayRate: 8,
    startTime: new Date(Date.now() - 1800000),
    endTime: new Date(Date.now() + 5400000),
    bidCount: 23,
    highestBidder: '0x3d89...a567',
    status: 'ACTIVE',
  },
  {
    id: 4,
    type: 'PRIORITY_LANE',
    title: 'Priority Lane Access - 24hr',
    description: '24-hour priority transaction processing lane access',
    startPrice: 15000,
    currentPrice: 11200,
    minPrice: 1500,
    decayRate: 6,
    startTime: new Date(Date.now() - 5400000),
    endTime: new Date(Date.now() + 1800000),
    bidCount: 31,
    highestBidder: '0xf234...e789',
    status: 'ACTIVE',
  },
];

const MOCK_BIDS: Bid[] = [
  { id: 1, auctionId: 1, bidder: '0x7a23...f891', amount: 7234, timestamp: new Date(Date.now() - 120000), status: 'CONFIRMED' },
  { id: 2, auctionId: 1, bidder: '0x8b34...a567', amount: 7100, timestamp: new Date(Date.now() - 300000), status: 'OUTBID' },
  { id: 3, auctionId: 2, bidder: '0x9b45...c234', amount: 42100, timestamp: new Date(Date.now() - 180000), status: 'CONFIRMED' },
  { id: 4, auctionId: 3, bidder: '0x3d89...a567', amount: 18500, timestamp: new Date(Date.now() - 60000), status: 'CONFIRMED' },
  { id: 5, auctionId: 4, bidder: '0xf234...e789', amount: 11200, timestamp: new Date(Date.now() - 240000), status: 'CONFIRMED' },
];

export default function BlockspaceAuctions() {
  const [auctions, setAuctions] = useState(MOCK_AUCTIONS);
  const [bids, setBids] = useState(MOCK_BIDS);
  const [selectedAuction, setSelectedAuction] = useState<number | null>(null);
  const [bidAmount, setBidAmount] = useState('');
  const [filter, setFilter] = useState<string>('ALL');

  // Simulate Dutch auction price decay
  useEffect(() => {
    const interval = setInterval(() => {
      setAuctions(prev => prev.map(a => {
        if (a.status !== 'ACTIVE') return a;
        const elapsed = (Date.now() - a.startTime.getTime()) / 1000 / 60; // minutes
        const decay = a.startPrice * (a.decayRate / 100) * (elapsed / 60);
        const newPrice = Math.max(a.minPrice, a.startPrice - decay);
        return { ...a, currentPrice: Math.round(newPrice) };
      }));
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  const typeColors: Record<string, string> = {
    BLOCKSPACE: 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30',
    VALIDATOR_SLOT: 'text-purple-400 bg-purple-500/20 border-purple-500/30',
    MEV_BUNDLE: 'text-red-400 bg-red-500/20 border-red-500/30',
    PRIORITY_LANE: 'text-cyan-400 bg-cyan-500/20 border-cyan-500/30',
  };

  const typeIcons: Record<string, string> = {
    BLOCKSPACE: '📦',
    VALIDATOR_SLOT: '🎫',
    MEV_BUNDLE: '⚡',
    PRIORITY_LANE: '🚀',
  };

  const formatTimeRemaining = (endTime: Date) => {
    const diff = endTime.getTime() - Date.now();
    if (diff <= 0) return 'Ended';
    const hours = Math.floor(diff / 3600000);
    const minutes = Math.floor((diff % 3600000) / 60000);
    const seconds = Math.floor((diff % 60000) / 1000);
    return `${hours}h ${minutes}m ${seconds}s`;
  };

  const filteredAuctions = filter === 'ALL' 
    ? auctions 
    : auctions.filter(a => a.type === filter);

  const totalValue = auctions.reduce((acc, a) => acc + a.currentPrice, 0);
  const activeBids = bids.filter(b => b.status === 'CONFIRMED').length;

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-purple-950/10 to-slate-950">
      {/* Header */}
      <div className="border-b border-purple-500/20 bg-black/40 backdrop-blur-xl sticky top-0 z-50">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Link href="/x3/swarm" className="text-gray-400 hover:text-white transition">
                ← Back to Swarm
              </Link>
              <span className="text-2xl">🔨</span>
              <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent">
                Blockspace Auctions
              </h1>
            </div>
            <button className="px-4 py-2 bg-gradient-to-r from-purple-600 to-pink-600 text-white rounded-lg font-mono text-sm hover:opacity-90 transition">
              Create Auction
            </button>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="container mx-auto px-6 py-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-black/60 border border-purple-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Active Auctions</div>
            <div className="text-2xl font-bold text-purple-400 font-mono">{auctions.filter(a => a.status === 'ACTIVE').length}</div>
          </div>
          <div className="bg-black/60 border border-purple-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Total Value</div>
            <div className="text-2xl font-bold text-white font-mono">{totalValue.toLocaleString()} ATLAS</div>
          </div>
          <div className="bg-black/60 border border-purple-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Active Bids</div>
            <div className="text-2xl font-bold text-cyan-400 font-mono">{activeBids}</div>
          </div>
          <div className="bg-black/60 border border-purple-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Total Participants</div>
            <div className="text-2xl font-bold text-yellow-400 font-mono">{new Set(bids.map(b => b.bidder)).size}</div>
          </div>
        </div>
      </div>

      {/* Filter */}
      <div className="container mx-auto px-6 mb-6">
        <div className="flex items-center gap-2 overflow-x-auto">
          {['ALL', 'BLOCKSPACE', 'VALIDATOR_SLOT', 'MEV_BUNDLE', 'PRIORITY_LANE'].map(type => (
            <button
              key={type}
              onClick={() => setFilter(type)}
              className={`px-4 py-2 rounded-lg font-mono text-sm whitespace-nowrap transition ${
                filter === type
                  ? 'bg-purple-500/30 text-purple-400 border border-purple-500/50'
                  : 'bg-black/40 text-gray-500 border border-gray-700 hover:border-purple-500/30'
              }`}
            >
              {type === 'ALL' ? '🎯 All' : `${typeIcons[type]} ${type.replace('_', ' ')}`}
            </button>
          ))}
        </div>
      </div>

      <div className="container mx-auto px-6 pb-12">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Auctions Grid */}
          <div className="lg:col-span-2">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {filteredAuctions.map(auction => (
                <div 
                  key={auction.id}
                  className={`p-6 bg-black/60 border rounded-xl cursor-pointer transition-all ${
                    selectedAuction === auction.id 
                      ? 'border-purple-500/50 ring-2 ring-purple-500/20' 
                      : 'border-gray-800/50 hover:border-purple-500/30'
                  }`}
                  onClick={() => setSelectedAuction(auction.id)}
                >
                  {/* Header */}
                  <div className="flex items-start justify-between mb-4">
                    <span className={`px-3 py-1 rounded-full text-xs font-mono border ${typeColors[auction.type]}`}>
                      {typeIcons[auction.type]} {auction.type.replace('_', ' ')}
                    </span>
                    <div className="text-right">
                      <div className={`text-xs font-mono ${auction.status === 'ACTIVE' ? 'text-green-400' : 'text-gray-500'}`}>
                        {auction.status}
                      </div>
                    </div>
                  </div>

                  <h3 className="text-lg font-semibold text-white mb-2">{auction.title}</h3>
                  <p className="text-sm text-gray-500 mb-4 line-clamp-2">{auction.description}</p>

                  {/* Price Display */}
                  <div className="bg-black/40 rounded-lg p-4 mb-4">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-gray-500 text-xs">Current Price</span>
                      <span className="text-purple-400 text-xs font-mono">Dutch Auction</span>
                    </div>
                    <div className="text-3xl font-bold text-white font-mono">
                      {auction.currentPrice.toLocaleString()} <span className="text-lg text-gray-500">ATLAS</span>
                    </div>
                    <div className="flex items-center justify-between mt-2 text-xs text-gray-500">
                      <span>Start: {auction.startPrice.toLocaleString()}</span>
                      <span>Min: {auction.minPrice.toLocaleString()}</span>
                    </div>
                    <div className="mt-2 h-2 bg-gray-800 rounded-full overflow-hidden">
                      <div 
                        className="h-full bg-gradient-to-r from-purple-600 to-pink-500 rounded-full transition-all"
                        style={{ width: `${((auction.currentPrice - auction.minPrice) / (auction.startPrice - auction.minPrice)) * 100}%` }}
                      />
                    </div>
                  </div>

                  {/* Timer & Bids */}
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="text-xs text-gray-500">Time Remaining</div>
                      <div className="text-lg font-mono text-cyan-400">{formatTimeRemaining(auction.endTime)}</div>
                    </div>
                    <div className="text-right">
                      <div className="text-xs text-gray-500">Bids</div>
                      <div className="text-lg font-mono text-yellow-400">{auction.bidCount}</div>
                    </div>
                  </div>

                  {/* Bid Panel */}
                  {selectedAuction === auction.id && (
                    <div className="mt-6 pt-6 border-t border-gray-800">
                      <div className="flex items-center gap-2 mb-4">
                        <input
                          type="number"
                          value={bidAmount}
                          onChange={(e) => setBidAmount(e.target.value)}
                          placeholder={`Min: ${auction.currentPrice} ATLAS`}
                          className="flex-1 bg-black/40 border border-gray-700 rounded-lg px-4 py-3 text-white font-mono focus:border-purple-500/50 outline-none"
                          onClick={(e) => e.stopPropagation()}
                        />
                        <button 
                          className="px-6 py-3 bg-gradient-to-r from-purple-600 to-pink-600 text-white rounded-lg font-mono hover:opacity-90 transition"
                          onClick={(e) => e.stopPropagation()}
                        >
                          Place Bid
                        </button>
                      </div>
                      <div className="text-xs text-gray-500">
                        Leading bidder: <span className="text-purple-400 font-mono">{auction.highestBidder}</span>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Activity Feed */}
          <div className="space-y-4">
            <h2 className="text-xl font-bold text-white font-mono">📜 Recent Activity</h2>
            <div className="bg-black/60 border border-purple-500/20 rounded-xl p-4">
              <div className="space-y-4 max-h-[600px] overflow-y-auto">
                {bids.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime()).map(bid => {
                  const auction = auctions.find(a => a.id === bid.auctionId);
                  return (
                    <div key={bid.id} className="p-4 bg-black/40 rounded-lg border border-gray-800">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-xs text-gray-500 font-mono">{bid.bidder}</span>
                        <span className={`px-2 py-1 rounded text-xs font-mono ${
                          bid.status === 'CONFIRMED' ? 'bg-green-500/20 text-green-400' :
                          bid.status === 'OUTBID' ? 'bg-red-500/20 text-red-400' :
                          'bg-yellow-500/20 text-yellow-400'
                        }`}>
                          {bid.status}
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-white">{bid.amount.toLocaleString()} ATLAS</span>
                        <span className="text-gray-500 text-xs">{auction?.title.slice(0, 20)}...</span>
                      </div>
                      <div className="text-xs text-gray-600 mt-1">
                        {Math.floor((Date.now() - bid.timestamp.getTime()) / 60000)}m ago
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* How It Works */}
            <div className="bg-black/60 border border-cyan-500/20 rounded-xl p-4">
              <h3 className="text-sm font-bold text-cyan-400 font-mono mb-4">ℹ️ Dutch Auction Mechanics</h3>
              <div className="space-y-3 text-xs text-gray-400">
                <div className="flex items-start gap-2">
                  <span className="text-cyan-400">1.</span>
                  <span>Price starts high and decreases over time</span>
                </div>
                <div className="flex items-start gap-2">
                  <span className="text-cyan-400">2.</span>
                  <span>First valid bid wins at current price</span>
                </div>
                <div className="flex items-start gap-2">
                  <span className="text-cyan-400">3.</span>
                  <span>Decay rate determines price drop speed</span>
                </div>
                <div className="flex items-start gap-2">
                  <span className="text-cyan-400">4.</span>
                  <span>Minimum price acts as reserve floor</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
