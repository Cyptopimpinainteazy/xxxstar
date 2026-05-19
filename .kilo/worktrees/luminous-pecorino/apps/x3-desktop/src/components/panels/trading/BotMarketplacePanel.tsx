import React, { useState } from "react";
import { Star, Download, TrendingUp, Users, DollarSign } from "lucide-react";
import clsx from "clsx";

interface BotStrategy {
  id: string;
  name: string;
  desc: string;
  author: string;
  rating: number;
  reviews: number;
  subscriptions: number;
  monthlyReturn: number;
  price: number;
  subscribers: string[];
  trending: boolean;
}

const MOCK_BOTS: BotStrategy[] = [
  {
    id: "1",
    name: "RSI Mean Reversion Pro",
    desc: "Advanced RSI-based reversal strategy with dynamic levels",
    author: "TradingMaster",
    rating: 4.8,
    reviews: 342,
    subscriptions: 1243,
    monthlyReturn: 24.5,
    price: 29.99,
    subscribers: ["You"],
    trending: true,
  },
  {
    id: "2",
    name: "MA Crossover Elite",
    desc: "Moving average crossover with volatility filters",
    author: "ChartGuy",
    rating: 4.6,
    reviews: 218,
    subscriptions: 876,
    monthlyReturn: 18.3,
    price: 19.99,
    subscribers: [],
    trending: false,
  },
  {
    id: "3",
    name: "Bollinger Band Bounce",
    desc: "High-probability bounce trades with tight stops",
    author: "VolatilityPro",
    rating: 4.7,
    reviews: 156,
    subscriptions: 642,
    monthlyReturn: 21.2,
    price: 24.99,
    subscribers: [],
    trending: true,
  },
];

export default function BotMarketplacePanel() {
  const [bots, setBots] = useState<BotStrategy[]>(MOCK_BOTS);
  const [selectedBot, setSelectedBot] = useState<BotStrategy | null>(null);
  const [sortBy, setSortBy] = useState<"rating" | "trending" | "return">("rating");

  const sortedBots = [...bots].sort((a, b) => {
    if (sortBy === "rating") return b.rating - a.rating;
    if (sortBy === "trending") return b.subscriptions - a.subscriptions;
    return b.monthlyReturn - a.monthlyReturn;
  });

  const handleSubscribe = (botId: string) => {
    setBots(
      bots.map((bot) =>
        bot.id === botId && !bot.subscribers.includes("You")
          ? { ...bot, subscriptions: bot.subscriptions + 1, subscribers: [...bot.subscribers, "You"] }
          : bot
      )
    );
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold">Bot Marketplace</h2>
        <div className="flex gap-2">
          {(["rating", "trending", "return"] as const).map((sort) => (
            <button
              key={sort}
              onClick={() => setSortBy(sort)}
              className={clsx(
                "px-3 py-1 rounded text-sm font-medium transition",
                sortBy === sort
                  ? "bg-blue-600 text-white"
                  : "bg-[#15151b] text-gray-400 hover:bg-[#1a1a20]"
              )}
            >
              {sort === "rating" && "Top Rated"}
              {sort === "trending" && "Trending"}
              {sort === "return" && "Best Return"}
            </button>
          ))}
        </div>
      </div>

      {/* Bot List */}
      <div className="flex-1 overflow-y-auto mb-4">
        <div className="space-y-3">
          {sortedBots.map((bot) => (
            <button
              key={bot.id}
              onClick={() => setSelectedBot(bot)}
              className={clsx(
                "w-full text-left p-4 rounded-lg border-2 transition",
                selectedBot?.id === bot.id
                  ? "border-blue-400 bg-blue-600/10"
                  : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
              )}
            >
              <div className="flex items-start justify-between mb-2">
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="font-semibold">{bot.name}</h3>
                    {bot.trending && (
                      <span className="bg-orange-600 text-white text-xs px-2 py-0.5 rounded">Trending</span>
                    )}
                  </div>
                  <p className="text-xs text-gray-400 mt-1">{bot.desc}</p>
                </div>
                <div className="text-right">
                  <div className="flex items-center gap-1 justify-end mb-1">
                    <Star size={14} className="fill-yellow-400 text-yellow-400" />
                    <span className="font-semibold text-sm">{bot.rating}</span>
                  </div>
                  <div className="text-xs text-gray-500">{bot.reviews} reviews</div>
                </div>
              </div>

              <div className="flex items-center justify-between text-xs">
                <div className="space-y-1">
                  <div className="text-gray-400">
                    by <span className="text-blue-400 font-medium">{bot.author}</span>
                  </div>
                  <div className="flex items-center gap-4">
                    <span className="flex items-center gap-1 text-gray-400">
                      <Users size={12} /> {bot.subscriptions} subs
                    </span>
                    <span className="text-green-400 font-semibold">+{bot.monthlyReturn}% avg</span>
                  </div>
                </div>
                <div className="text-right">
                  <div className="font-bold text-white">${bot.price}/mo</div>
                  {bot.subscribers.includes("You") && (
                    <div className="text-green-400 text-xs mt-1">✓ Subscribed</div>
                  )}
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Bot Detail Panel */}
      {selectedBot && (
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
          <div className="flex items-start justify-between">
            <div>
              <h3 className="font-bold text-lg">{selectedBot.name}</h3>
              <p className="text-sm text-gray-400">{selectedBot.desc}</p>
              <p className="text-xs text-blue-400 mt-1">by {selectedBot.author}</p>
            </div>
            <div className="text-right">
              <div className="text-2xl font-bold text-green-400">+{selectedBot.monthlyReturn}%</div>
              <div className="text-xs text-gray-500">avg monthly</div>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div>
              <div className="text-xs text-gray-400 mb-1">Rating</div>
              <div className="font-bold flex items-center gap-1">
                <Star size={14} className="fill-yellow-400 text-yellow-400" />
                {selectedBot.rating}
              </div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-1">Reviews</div>
              <div className="font-bold">{selectedBot.reviews}</div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-1">Subscriptions</div>
              <div className="font-bold">{selectedBot.subscriptions}</div>
            </div>
          </div>

          {selectedBot.subscribers.includes("You") ? (
            <div className="bg-green-600/10 border border-green-600 rounded-lg p-3 text-center">
              <div className="text-sm font-semibold text-green-400">✓ You are subscribed</div>
              <div className="text-xs text-gray-400 mt-1">Active trades will execute daily</div>
            </div>
          ) : (
            <button
              onClick={() => handleSubscribe(selectedBot.id)}
              className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
            >
              <Download size={14} /> Subscribe for ${selectedBot.price}/mo
            </button>
          )}
        </div>
      )}
    </div>
  );
}
