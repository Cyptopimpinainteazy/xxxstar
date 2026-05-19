import React, { useState } from "react";
import { Gavel, Clock, TrendingUp, Target, X } from "lucide-react";
import clsx from "clsx";

interface Auction {
  id: string;
  name: string;
  image: string;
  auctionType: "dutch" | "sealed";
  startPrice: number;
  currentPrice: number;
  floorPrice: number;
  bids: number;
  timeRemaining: string;
  highestBidder: string;
  status: "active" | "ending_soon" | "closed";
  creator: string;
}

interface Bid {
  id: string;
  auctionId: string;
  bidder: string;
  amount: number;
  timestamp: string;
}

const MOCK_AUCTIONS: Auction[] = [
  {
    id: "auc-001",
    name: "X3 Genesis NFT",
    image: "🎨",
    auctionType: "dutch",
    startPrice: 50,
    currentPrice: 28.5,
    floorPrice: 20,
    bids: 45,
    timeRemaining: "2h 15m",
    highestBidder: "0x7a2c...8f4d",
    status: "active",
    creator: "0x9b1e...2c7a",
  },
  {
    id: "auc-002",
    name: "Rare Validator Badge",
    image: "⚡",
    auctionType: "sealed",
    startPrice: 10,
    currentPrice: 12.3,
    floorPrice: 8,
    bids: 18,
    timeRemaining: "5m 30s",
    highestBidder: "0x4c5d...9e2f",
    status: "ending_soon",
    creator: "0xd3e2...1a5b",
  },
  {
    id: "auc-003",
    name: "DeFi Card Collection",
    image: "🏆",
    auctionType: "dutch",
    startPrice: 25,
    currentPrice: 15.8,
    floorPrice: 12,
    bids: 32,
    timeRemaining: "1h 45m",
    highestBidder: "0xf1a2...c8e9",
    status: "active",
    creator: "0x6b3c...d4e1",
  },
  {
    id: "auc-004",
    name: "Exclusive Avatar NFT",
    image: "😎",
    auctionType: "sealed",
    startPrice: 8,
    currentPrice: 9.5,
    floorPrice: 6,
    bids: 12,
    timeRemaining: "3d 2h 15m",
    highestBidder: "0x8e2d...7f3a",
    status: "active",
    creator: "0x2f4c...5e8a",
  },
];

const MOCK_BIDS: Bid[] = [
  {
    id: "bid-001",
    auctionId: "auc-001",
    bidder: "0x7a2c...8f4d",
    amount: 28.5,
    timestamp: "2m ago",
  },
  {
    id: "bid-002",
    auctionId: "auc-001",
    bidder: "0x9b1e...2c7a",
    amount: 27.2,
    timestamp: "4m ago",
  },
  {
    id: "bid-003",
    auctionId: "auc-002",
    bidder: "0x4c5d...9e2f",
    amount: 12.3,
    timestamp: "1m ago",
  },
];

type TabType = "active" | "create" | "bids";

export default function NftAuctionPanel() {
  const [activeTab, setActiveTab] = useState<TabType>("active");
  const [auctions, setAuctions] = useState<Auction[]>(MOCK_AUCTIONS);
  const [newBid, setNewBid] = useState({ auctionId: "", amount: "" });
  const [selectedAuction, setSelectedAuction] = useState<string | null>(null);

  const activeAuctions = auctions.filter((a) => a.status !== "closed");

  const handlePlaceBid = () => {
    if (newBid.auctionId && newBid.amount) {
      const auction = auctions.find((a) => a.id === newBid.auctionId);
      if (auction && parseFloat(newBid.amount) > auction.currentPrice) {
        setAuctions(
          auctions.map((a) =>
            a.id === newBid.auctionId
              ? {
                  ...a,
                  currentPrice: parseFloat(newBid.amount),
                  bids: a.bids + 1,
                  highestBidder: "0xYour...Address",
                }
              : a
          )
        );
        setNewBid({ auctionId: "", amount: "" });
      }
    }
  };

  const getStatusColor = (status: string) => {
    if (status === "active") return "bg-green-600/20 text-green-400";
    if (status === "ending_soon") return "bg-red-600/20 text-red-400";
    return "bg-gray-600/20 text-gray-400";
  };

  const getAuctionTypeColor = (type: string) => {
    return type === "dutch" ? "bg-cyan-600/20 text-cyan-400" : "bg-purple-600/20 text-purple-400";
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Gavel size={20} className="text-yellow-400" /> NFT Auctions
      </h2>

      {/* Tab Navigation */}
      <div className="flex gap-2 mb-4 border-b border-[#2a2a35]">
        {(["active", "create", "bids"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={clsx(
              "px-4 py-2 text-sm font-semibold border-b-2 transition",
              activeTab === tab
                ? "border-yellow-400 text-yellow-400"
                : "border-transparent text-gray-400 hover:text-white"
            )}
          >
            {tab === "active" && `Active (${activeAuctions.length})`}
            {tab === "create" && "Create Auction"}
            {tab === "bids" && `My Bids (${MOCK_BIDS.length})`}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto space-y-4">
        {/* Active Auctions */}
        {activeTab === "active" && (
          <div className="space-y-3">
            {activeAuctions.map((auction) => (
              <div
                key={auction.id}
                className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 hover:border-[#3a3a45] transition cursor-pointer"
                onClick={() => setSelectedAuction(selectedAuction === auction.id ? null : auction.id)}
              >
                {/* Auction Header */}
                <div className="flex justify-between items-start">
                  <div className="flex gap-3">
                    <div className="text-3xl">{auction.image}</div>
                    <div>
                      <div className="font-semibold text-sm">{auction.name}</div>
                      <div className="flex gap-2 mt-1">
                        <span className={clsx("text-xs px-2 py-0.5 rounded", getStatusColor(auction.status))}>
                          {auction.status === "ending_soon" ? "⏰ Ending Soon" : auction.status.charAt(0).toUpperCase() + auction.status.slice(1)}
                        </span>
                        <span className={clsx("text-xs px-2 py-0.5 rounded", getAuctionTypeColor(auction.auctionType))}>
                          {auction.auctionType === "dutch" ? "Dutch" : "Sealed"}
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-lg font-bold text-cyan-400">{auction.currentPrice.toFixed(2)} X3</div>
                    <div className="text-xs text-yellow-400 mt-1 flex items-center justify-end gap-1">
                      <Clock size={12} /> {auction.timeRemaining}
                    </div>
                  </div>
                </div>

                {/* Auction Details */}
                <div className="mt-3 grid grid-cols-3 gap-2 text-xs">
                  <div className="bg-[#0a0a0f] rounded p-2">
                    <div className="text-gray-500">Start</div>
                    <div className="text-cyan-400 font-mono">{auction.startPrice.toFixed(2)}</div>
                  </div>
                  <div className="bg-[#0a0a0f] rounded p-2">
                    <div className="text-gray-500">Floor</div>
                    <div className="text-yellow-400 font-mono">{auction.floorPrice.toFixed(2)}</div>
                  </div>
                  <div className="bg-[#0a0a0f] rounded p-2">
                    <div className="text-gray-500">Bids</div>
                    <div className="text-green-400 font-mono">{auction.bids}</div>
                  </div>
                </div>

                {/* Price Trend */}
                <div className="mt-2 text-xs">
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-500">Price Trend</span>
                    <span className="text-gray-500">{((auction.currentPrice / auction.startPrice) * 100).toFixed(0)}% of start</span>
                  </div>
                  <div className="bg-[#0a0a0f] rounded-full h-1.5">
                    <div
                      className="h-full bg-gradient-to-r from-yellow-600 to-red-600 rounded-full"
                      style={{ width: `${(auction.currentPrice / auction.startPrice) * 100}%` }}
                    />
                  </div>
                </div>

                {/* Expanded Details */}
                {selectedAuction === auction.id && (
                  <div className="mt-3 pt-3 border-t border-[#2a2a35] space-y-2">
                    <div className="flex justify-between text-xs">
                      <span className="text-gray-500">Highest Bidder:</span>
                      <span className="text-cyan-400 font-mono">{auction.highestBidder}</span>
                    </div>
                    <div className="flex justify-between text-xs">
                      <span className="text-gray-500">Creator:</span>
                      <span className="text-purple-400 font-mono">{auction.creator}</span>
                    </div>
                    <button className="w-full mt-2 bg-yellow-600/20 border border-yellow-600 text-yellow-400 py-1.5 rounded text-xs font-semibold hover:bg-yellow-600/30 transition">
                      Place Bid
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Create Auction */}
        {activeTab === "create" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4 max-w-md">
            <h3 className="font-semibold text-sm">Create New Auction</h3>
            <div>
              <label className="text-xs text-gray-400">Item Name</label>
              <input
                type="text"
                placeholder="NFT Name"
                className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white placeholder-gray-600"
              />
            </div>
            <div>
              <label className="text-xs text-gray-400">Starting Price (X3)</label>
              <input
                type="number"
                placeholder="0.00"
                className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white placeholder-gray-600"
              />
            </div>
            <div>
              <label className="text-xs text-gray-400">Auction Type</label>
              <select className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white">
                <option value="dutch">Dutch (Price Decreases)</option>
                <option value="sealed">Sealed Bid</option>
              </select>
            </div>
            <div>
              <label className="text-xs text-gray-400">Duration</label>
              <select className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white">
                <option value="1h">1 Hour</option>
                <option value="1d">1 Day</option>
                <option value="7d">7 Days</option>
                <option value="30d">30 Days</option>
              </select>
            </div>
            <button className="w-full bg-yellow-600/20 border border-yellow-600 text-yellow-400 py-2 rounded font-semibold text-sm hover:bg-yellow-600/30 transition">
              Create Auction
            </button>
          </div>
        )}

        {/* Bids */}
        {activeTab === "bids" && (
          <div className="space-y-3">
            {MOCK_BIDS.map((bid) => {
              const auction = auctions.find((a) => a.id === bid.auctionId);
              return (
                <div
                  key={bid.id}
                  className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3"
                >
                  <div className="flex justify-between items-start">
                    <div>
                      <div className="text-sm font-semibold">{auction?.name}</div>
                      <div className="text-xs text-gray-500 mt-1">Bid: {bid.amount.toFixed(2)} X3</div>
                      <div className="text-xs text-gray-600 mt-1">{bid.timestamp}</div>
                    </div>
                    <div className="text-right">
                      <div className="text-xs px-2 py-1 rounded bg-cyan-600/20 text-cyan-400">
                        Highest
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Dutch & sealed-bid auctions with real-time bidding
      </div>
    </div>
  );
}
