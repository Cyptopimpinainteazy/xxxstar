import React, { useState, useEffect } from "react";
import { Search, Zap, Wallet, DollarSign, User, TrendingUp, Clock, X } from "lucide-react";
import clsx from "clsx";

interface SearchResult {
  id: string;
  title: string;
  category: "wallet" | "token" | "validator" | "transaction" | "dex" | "user";
  description: string;
  icon: string;
  url: string;
  relevance: number;
}

const ALL_RESULTS: SearchResult[] = [
  { id: "1", title: "Ethereum Token", category: "token", description: "ETH - Layer 2 Ethereum", icon: "🔗", url: "/tokens/eth", relevance: 95 },
  { id: "2", title: "My Wallet", category: "wallet", description: "0x1234...5678 - 5 assets", icon: "👛", url: "/wallet", relevance: 98 },
  { id: "3", title: "ValidatorKing", category: "validator", description: "Top validator - 99.97% uptime", icon: "⭐", url: "/validators/1", relevance: 87 },
  { id: "4", title: "tx: Swap ETH to USDC", category: "transaction", description: "10 hours ago - $8,500", icon: "↔️", url: "/tx/abc123", relevance: 85 },
  { id: "5", title: "X3/USDC Pool", category: "dex", description: "TVL: $45M - APY: 18.5%", icon: "💧", url: "/pools/x3-usdc", relevance: 82 },
  { id: "6", title: "CryptoMax", category: "user", description: "Top creator - 12.4K followers", icon: "👨‍💻", url: "/users/cryptomax", relevance: 75 },
  { id: "7", title: "Bitcoin Token", category: "token", description: "BTC - Layer 1", icon: "₿", url: "/tokens/btc", relevance: 92 },
  { id: "8", title: "Uniswap Pool", category: "dex", description: "ETH/USDC - TVL: $120M", icon: "💧", url: "/pools/eth-usdc", relevance: 88 },
];

export default function GlobalSearchPanel() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [recentSearches, setRecentSearches] = useState(["ETH", "X3/USDC", "ValidatorKing"]);
  const [selectedIndex, setSelectedIndex] = useState(-1);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    const filtered = ALL_RESULTS
      .filter((r) =>
        r.title.toLowerCase().includes(query.toLowerCase()) ||
        r.description.toLowerCase().includes(query.toLowerCase())
      )
      .sort((a, b) => b.relevance - a.relevance)
      .slice(0, 8);

    setResults(filtered);
    setSelectedIndex(-1);
  }, [query]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex(Math.min(selectedIndex + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex(Math.max(selectedIndex - 1, -1));
    } else if (e.key === "Enter" && selectedIndex >= 0) {
      handleSearch(results[selectedIndex]);
    }
  };

  const handleSearch = (result: SearchResult) => {
    addToRecentSearches(result.title);
    setQuery("");
    setResults([]);
  };

  const addToRecentSearches = (search: string) => {
    setRecentSearches([
      search,
      ...recentSearches.filter((s) => s !== search).slice(0, 4),
    ]);
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "wallet":
        return <Wallet size={16} className="text-blue-400" />;
      case "token":
        return <TrendingUp size={16} className="text-yellow-400" />;
      case "validator":
        return <Zap size={16} className="text-purple-400" />;
      case "transaction":
        return <Clock size={16} className="text-green-400" />;
      case "dex":
        return <DollarSign size={16} className="text-pink-400" />;
      case "user":
        return <User size={16} className="text-cyan-400" />;
      default:
        return null;
    }
  };

  const getCategoryColor = (category: string) => {
    switch (category) {
      case "wallet":
        return "bg-blue-500/10 text-blue-400";
      case "token":
        return "bg-yellow-500/10 text-yellow-400";
      case "validator":
        return "bg-purple-500/10 text-purple-400";
      case "transaction":
        return "bg-green-500/10 text-green-400";
      case "dex":
        return "bg-pink-500/10 text-pink-400";
      case "user":
        return "bg-cyan-500/10 text-cyan-400";
      default:
        return "bg-gray-500/10 text-gray-400";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-6">Global Search</h2>

      {/* Search Box */}
      <div className="relative mb-6">
        <Search size={20} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
        <input
          type="text"
          placeholder="Search wallets, tokens, validators, transactions... ⌘K"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          className="w-full bg-[#15151b] border border-[#2a2a35] rounded-lg pl-10 pr-4 py-3 text-white placeholder-gray-600 focus:border-blue-500 focus:outline-none text-lg"
          autoFocus
        />
      </div>

      <div className="flex-1 overflow-y-auto">
        {!query ? (
          // Recent Searches
          <div>
            <h3 className="text-sm font-semibold text-gray-400 mb-3">Recent Searches</h3>
            <div className="space-y-2">
              {recentSearches.map((search, i) => (
                <button
                  key={i}
                  onClick={() => {
                    setQuery(search);
                    addToRecentSearches(search);
                  }}
                  className="w-full flex items-center gap-3 p-3 rounded-lg bg-[#15151b] border border-[#2a2a35] hover:border-[#3a3a45] transition text-left"
                >
                  <Clock size={16} className="text-gray-500 flex-shrink-0" />
                  <span className="flex-1">{search}</span>
                  <Zap size={14} className="text-gray-600" />
                </button>
              ))}
            </div>

            {/* Quick Links */}
            <h3 className="text-sm font-semibold text-gray-400 mt-6 mb-3">Quick Links</h3>
            <div className="grid grid-cols-2 gap-2">
              {[
                { icon: "👛", label: "My Wallet", url: "/wallet" },
                { icon: "⭐", label: "Top Validators", url: "/validators" },
                { icon: "💧", label: "Liquidity Pools", url: "/pools" },
                { icon: "📊", label: "Portfolio", url: "/portfolio" },
              ].map((link, i) => (
                <button
                  key={i}
                  className="p-3 rounded-lg bg-[#15151b] border border-[#2a2a35] hover:border-[#3a3a45] transition text-left text-sm font-semibold"
                >
                  <span className="text-lg">{link.icon}</span>
                  <div className="mt-1">{link.label}</div>
                </button>
              ))}
            </div>
          </div>
        ) : results.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-gray-500">
            <Search size={48} className="opacity-30 mb-3" />
            <div>No results found for "{query}"</div>
          </div>
        ) : (
          // Search Results
          <div className="space-y-2">
            {results.map((result, index) => (
              <button
                key={result.id}
                onClick={() => handleSearch(result)}
                className={clsx(
                  "w-full text-left p-4 rounded-lg border-2 transition",
                  selectedIndex === index
                    ? "border-blue-400 bg-blue-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start gap-3">
                  <div className="text-2xl mt-0.5">{result.icon}</div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-semibold truncate">{result.title}</span>
                      <span className={clsx("text-xs px-2 py-0.5 rounded-full font-semibold flex-shrink-0", getCategoryColor(result.category))}>
                        {result.category}
                      </span>
                    </div>
                    <p className="text-sm text-gray-400 truncate">{result.description}</p>
                  </div>
                  <div className="text-right flex-shrink-0">
                    <div className="text-xs text-gray-500">{result.relevance}% match</div>
                    {getCategoryIcon(result.category)}
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Footer Help */}
      {!query && (
        <div className="mt-6 pt-6 border-t border-[#2a2a35] text-xs text-gray-500 flex items-center justify-between">
          <span>Press <kbd className="bg-[#2a2a35] px-2 py-1 rounded">⌘K</kbd> anywhere to search</span>
          <span>↑↓ to navigate • ↵ to select</span>
        </div>
      )}
    </div>
  );
}
