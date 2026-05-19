import React, { useState } from "react";
import { ShoppingBag, Heart, MessageCircle, TrendingUp, Tag, Package } from "lucide-react";
import clsx from "clsx";

interface Listing {
  id: string;
  title: string;
  seller: string;
  price: number;
  image: string;
  category: "nft" | "token" | "collectible";
  likes: number;
  offers: number;
  status: "active" | "sold" | "pending";
  createdAt: string;
}

interface Offer {
  id: string;
  listingId: string;
  buyer: string;
  amount: number;
  expiresAt: string;
  status: "pending" | "accepted" | "rejected";
}

const MOCK_LISTINGS: Listing[] = [
  {
    id: "lst-001",
    title: "X3 Genesis NFT #1",
    seller: "0x7a2c...8f4d",
    price: 15.5,
    image: "🎨",
    category: "nft",
    likes: 247,
    offers: 12,
    status: "active",
    createdAt: "2024-01-15",
  },
  {
    id: "lst-002",
    title: "1000 X3 Token Bundle",
    seller: "0x9b1e...2c7a",
    price: 8.25,
    image: "💰",
    category: "token",
    likes: 89,
    offers: 5,
    status: "active",
    createdAt: "2024-01-14",
  },
  {
    id: "lst-003",
    title: "Rare DeFi Card Collection",
    seller: "0x4c5d...9e2f",
    price: 3.75,
    image: "🏆",
    category: "collectible",
    likes: 156,
    offers: 8,
    status: "pending",
    createdAt: "2024-01-13",
  },
  {
    id: "lst-004",
    title: "Limited Edition Validator Badge",
    seller: "0xd3e2...1a5b",
    price: 2.1,
    image: "⚡",
    category: "nft",
    likes: 45,
    offers: 2,
    status: "active",
    createdAt: "2024-01-13",
  },
];

const MOCK_OFFERS: Offer[] = [
  {
    id: "off-001",
    listingId: "lst-001",
    buyer: "0xf1a2...c8e9",
    amount: 12.8,
    expiresAt: "2024-01-25",
    status: "pending",
  },
  {
    id: "off-002",
    listingId: "lst-001",
    buyer: "0x6b3c...d4e1",
    amount: 14.2,
    expiresAt: "2024-01-26",
    status: "pending",
  },
  {
    id: "off-003",
    listingId: "lst-002",
    buyer: "0x8e2d...7f3a",
    amount: 8.5,
    expiresAt: "2024-01-24",
    status: "accepted",
  },
];

type TabType = "browse" | "create" | "offers" | "stats";

export default function RealMarketplacePanel() {
  const [activeTab, setActiveTab] = useState<TabType>("browse");
  const [listings, setListings] = useState<Listing[]>(MOCK_LISTINGS);
  const [selectedCategory, setSelectedCategory] = useState("all");
  const [newListing, setNewListing] = useState({
    title: "",
    price: "",
    category: "nft" as const,
  });

  const filteredListings =
    selectedCategory === "all" ? listings : listings.filter((l) => l.category === selectedCategory);

  const handleCreateListing = () => {
    if (newListing.title && newListing.price) {
      const listing: Listing = {
        id: `lst-${Date.now()}`,
        title: newListing.title,
        seller: "0xYou...rAddr",
        price: parseFloat(newListing.price),
        image: "📦",
        category: newListing.category,
        likes: 0,
        offers: 0,
        status: "active",
        createdAt: new Date().toISOString().split("T")[0],
      };
      setListings([listing, ...listings]);
      setNewListing({ title: "", price: "", category: "nft" });
    }
  };

  const handleAcceptOffer = (offerId: string) => {
    const offer = MOCK_OFFERS.find((o) => o.id === offerId);
    if (offer) {
      setListings(
        listings.map((l) =>
          l.id === offer.listingId ? { ...l, status: "sold" as const } : l
        )
      );
    }
  };

  const stats = {
    totalVolume: filteredListings.reduce((sum, l) => sum + l.price, 0),
    activeListings: filteredListings.filter((l) => l.status === "active").length,
    totalOffers: MOCK_OFFERS.length,
    floorPrice: Math.min(...filteredListings.map((l) => l.price)),
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <ShoppingBag size={20} className="text-cyan-400" /> Real Marketplace
      </h2>

      {/* Tab Navigation */}
      <div className="flex gap-2 mb-4 border-b border-[#2a2a35]">
        {(["browse", "create", "offers", "stats"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={clsx(
              "px-4 py-2 text-sm font-semibold border-b-2 transition",
              activeTab === tab
                ? "border-cyan-400 text-cyan-400"
                : "border-transparent text-gray-400 hover:text-white"
            )}
          >
            {tab === "browse" && "Browse"}
            {tab === "create" && "Create"}
            {tab === "offers" && "Offers"}
            {tab === "stats" && "Stats"}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto">
        {/* Browse Listings */}
        {activeTab === "browse" && (
          <div className="space-y-4">
            <div className="flex gap-2 overflow-x-auto pb-2">
              {["all", "nft", "token", "collectible"].map((cat) => (
                <button
                  key={cat}
                  onClick={() => setSelectedCategory(cat)}
                  className={clsx(
                    "px-3 py-1 text-xs rounded-lg whitespace-nowrap transition",
                    selectedCategory === cat
                      ? "bg-cyan-600/20 border border-cyan-600 text-cyan-400"
                      : "bg-[#15151b] border border-[#2a2a35] hover:border-[#3a3a45]"
                  )}
                >
                  {cat.charAt(0).toUpperCase() + cat.slice(1)}
                </button>
              ))}
            </div>

            <div className="grid grid-cols-1 gap-3">
              {filteredListings.map((listing) => (
                <div
                  key={listing.id}
                  className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 hover:border-[#3a3a45] transition"
                >
                  <div className="flex justify-between items-start">
                    <div className="flex gap-3">
                      <div className="text-3xl">{listing.image}</div>
                      <div>
                        <div className="font-semibold text-sm">{listing.title}</div>
                        <div className="text-xs text-gray-500 mt-1">by {listing.seller}</div>
                        <div className="flex gap-4 mt-2 text-xs">
                          <span className="flex items-center gap-1">
                            <Heart size={14} className="text-red-500" /> {listing.likes}
                          </span>
                          <span className="flex items-center gap-1">
                            <MessageCircle size={14} className="text-yellow-500" /> {listing.offers}
                          </span>
                        </div>
                      </div>
                    </div>
                    <div className="text-right">
                      <div className="text-lg font-bold text-cyan-400">{listing.price.toFixed(2)} X3</div>
                      <span
                        className={clsx(
                          "inline-block text-xs px-2 py-1 rounded mt-2",
                          listing.status === "active"
                            ? "bg-green-600/20 text-green-400"
                            : listing.status === "sold"
                            ? "bg-gray-600/20 text-gray-400"
                            : "bg-yellow-600/20 text-yellow-400"
                        )}
                      >
                        {listing.status.charAt(0).toUpperCase() + listing.status.slice(1)}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Create Listing */}
        {activeTab === "create" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4 max-w-md">
            <h3 className="font-semibold text-sm">Create New Listing</h3>
            <div>
              <label className="text-xs text-gray-400">Item Title</label>
              <input
                type="text"
                value={newListing.title}
                onChange={(e) => setNewListing({ ...newListing, title: e.target.value })}
                placeholder="e.g., X3 Rare NFT"
                className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white placeholder-gray-600"
              />
            </div>
            <div>
              <label className="text-xs text-gray-400">Price (X3)</label>
              <input
                type="number"
                value={newListing.price}
                onChange={(e) => setNewListing({ ...newListing, price: e.target.value })}
                placeholder="0.00"
                className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white placeholder-gray-600"
              />
            </div>
            <div>
              <label className="text-xs text-gray-400">Category</label>
              <select
                value={newListing.category}
                onChange={(e) =>
                  setNewListing({ ...newListing, category: e.target.value as any })
                }
                className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white"
              >
                <option value="nft">NFT</option>
                <option value="token">Token</option>
                <option value="collectible">Collectible</option>
              </select>
            </div>
            <button
              onClick={handleCreateListing}
              className="w-full bg-cyan-600/20 border border-cyan-600 text-cyan-400 py-2 rounded font-semibold text-sm hover:bg-cyan-600/30 transition"
            >
              Create Listing
            </button>
          </div>
        )}

        {/* Offers & Bids */}
        {activeTab === "offers" && (
          <div className="space-y-3">
            {MOCK_OFFERS.map((offer) => (
              <div
                key={offer.id}
                className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3"
              >
                <div className="flex justify-between items-start">
                  <div>
                    <div className="text-sm font-semibold">{offer.amount.toFixed(2)} X3</div>
                    <div className="text-xs text-gray-500 mt-1">from {offer.buyer}</div>
                    <div className="text-xs text-gray-600 mt-1">Expires: {offer.expiresAt}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded",
                      offer.status === "accepted"
                        ? "bg-green-600/20 text-green-400"
                        : offer.status === "rejected"
                        ? "bg-red-600/20 text-red-400"
                        : "bg-yellow-600/20 text-yellow-400"
                    )}
                  >
                    {offer.status.charAt(0).toUpperCase() + offer.status.slice(1)}
                  </span>
                </div>
                {offer.status === "pending" && (
                  <div className="flex gap-2 mt-3">
                    <button
                      onClick={() => handleAcceptOffer(offer.id)}
                      className="flex-1 bg-green-600/20 border border-green-600 text-green-400 py-1 rounded text-xs font-semibold hover:bg-green-600/30 transition"
                    >
                      Accept
                    </button>
                    <button className="flex-1 bg-red-600/20 border border-red-600 text-red-400 py-1 rounded text-xs font-semibold hover:bg-red-600/30 transition">
                      Reject
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Stats */}
        {activeTab === "stats" && (
          <div className="grid grid-cols-2 gap-3">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-xs text-gray-500 mb-2">Total Volume</div>
              <div className="text-xl font-bold text-cyan-400">
                {stats.totalVolume.toFixed(2)} X3
              </div>
            </div>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-xs text-gray-500 mb-2">Active Listings</div>
              <div className="text-xl font-bold text-green-400">{stats.activeListings}</div>
            </div>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-xs text-gray-500 mb-2">Total Offers</div>
              <div className="text-xl font-bold text-yellow-400">{stats.totalOffers}</div>
            </div>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-xs text-gray-500 mb-2">Floor Price</div>
              <div className="text-xl font-bold text-purple-400">
                {stats.floorPrice.toFixed(2)} X3
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Peer-to-peer marketplace for NFTs, tokens, and collectibles
      </div>
    </div>
  );
}
