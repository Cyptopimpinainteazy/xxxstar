import React, { useState } from "react";
import { Download, Star, Users, TrendingUp, Shield } from "lucide-react";
import clsx from "clsx";

interface Plugin {
  id: string;
  name: string;
  author: string;
  desc: string;
  rating: number;
  downloads: number;
  installed: boolean;
  featured: boolean;
  category: "trading" | "wallet" | "defi" | "analytics" | "social";
}

const MOCK_PLUGINS: Plugin[] = [
  {
    id: "1",
    name: "Advanced TA Indicators",
    author: "TradingMasters",
    desc: "20+ technical indicators: Ichimoku, MACD, Bollinger Bands, Volume Profile",
    rating: 4.9,
    downloads: 5200,
    installed: false,
    featured: true,
    category: "analytics",
  },
  {
    id: "2",
    name: "Wallet Guard Pro",
    author: "SecurityTeam",
    desc: "Real-time transaction simulation + multi-sig proposal UI",
    rating: 4.8,
    downloads: 3400,
    installed: true,
    featured: true,
    category: "wallet",
  },
  {
    id: "3",
    name: "Yield Tracker",
    author: "DeFiAnalytics",
    desc: "Track LP yields across all pools. Alert on APY changes.",
    rating: 4.7,
    downloads: 2100,
    installed: false,
    featured: false,
    category: "defi",
  },
];

export default function AppStorePanel() {
  const [plugins, setPlugins] = useState<Plugin[]>(MOCK_PLUGINS);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);

  const filteredPlugins = plugins.filter((p) => {
    const matchesCategory = !selectedCategory || p.category === selectedCategory;
    const matchesSearch =
      !searchQuery ||
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.desc.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const handleInstall = (pluginId: string) => {
    setPlugins(
      plugins.map((p) =>
        p.id === pluginId ? { ...p, installed: true, downloads: p.downloads + 1 } : p
      )
    );
    if (selectedPlugin?.id === pluginId) {
      setSelectedPlugin({ ...selectedPlugin, installed: true, downloads: selectedPlugin.downloads + 1 });
    }
  };

  const handleUninstall = (pluginId: string) => {
    setPlugins(
      plugins.map((p) =>
        p.id === pluginId ? { ...p, installed: false } : p
      )
    );
    if (selectedPlugin?.id === pluginId) {
      setSelectedPlugin({ ...selectedPlugin, installed: false });
    }
  };

  const categories = [
    { id: "trading", label: "Trading" },
    { id: "wallet", label: "Wallet" },
    { id: "defi", label: "DeFi" },
    { id: "analytics", label: "Analytics" },
    { id: "social", label: "Social" },
  ];

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">Plugin Store</h2>

      {/* Search & Filters */}
      <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 mb-4">
        <input
          type="text"
          placeholder="Search plugins..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm placeholder-gray-500 mb-3"
        />

        <div className="flex gap-2 overflow-x-auto">
          <button
            onClick={() => setSelectedCategory(null)}
            className={clsx(
              "px-3 py-1 rounded text-sm font-medium whitespace-nowrap transition",
              !selectedCategory ? "bg-blue-600" : "bg-[#2a2a35] hover:bg-[#3a3a45]"
            )}
          >
            All
          </button>
          {categories.map((cat) => (
            <button
              key={cat.id}
              onClick={() => setSelectedCategory(cat.id)}
              className={clsx(
                "px-3 py-1 rounded text-sm font-medium whitespace-nowrap transition",
                selectedCategory === cat.id
                  ? "bg-blue-600"
                  : "bg-[#2a2a35] hover:bg-[#3a3a45]"
              )}
            >
              {cat.label}
            </button>
          ))}
        </div>
      </div>

      {/* Featured Banner */}
      <div className="bg-gradient-to-r from-purple-600 to-blue-600 rounded-lg p-4 mb-4">
        <h3 className="font-bold mb-1">⭐ Featured Plugins</h3>
        <p className="text-sm opacity-90">Curated by the X3 team for quality & performance</p>
      </div>

      <div className="flex-1 overflow-y-auto mb-4">
        <div className="space-y-3">
          {filteredPlugins.length === 0 ? (
            <div className="text-center py-8 text-gray-400">
              No plugins found for your query
            </div>
          ) : (
            filteredPlugins.map((plugin) => (
              <button
                key={plugin.id}
                onClick={() => setSelectedPlugin(plugin)}
                className={clsx(
                  "w-full text-left p-4 rounded-lg border-2 transition",
                  selectedPlugin?.id === plugin.id
                    ? "border-blue-400 bg-blue-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="flex items-center gap-2">
                      <h3 className="font-semibold">{plugin.name}</h3>
                      {plugin.featured && (
                        <span className="text-xs bg-purple-600 text-white px-2 py-0.5 rounded">
                          Featured
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-gray-400 mt-1">{plugin.desc}</p>
                  </div>
                  {plugin.installed ? (
                    <span className="text-xs bg-green-600 text-white px-2 py-1 rounded">
                      ✓ Installed
                    </span>
                  ) : (
                    <span className="text-xs bg-[#2a2a35] text-gray-400 px-2 py-1 rounded">
                      Not Installed
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-4 text-xs">
                  <span className="text-gray-400">
                    by <span className="text-blue-400 font-medium">{plugin.author}</span>
                  </span>
                  <div className="flex items-center gap-1 text-yellow-400">
                    <Star size={12} className="fill-yellow-400" />
                    {plugin.rating}
                  </div>
                  <div className="flex items-center gap-1 text-gray-400">
                    <Download size={12} /> {plugin.downloads.toLocaleString()}
                  </div>
                </div>
              </button>
            ))
          )}
        </div>
      </div>

      {/* Plugin Details */}
      {selectedPlugin && (
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
          <div>
            <h3 className="font-bold text-lg">{selectedPlugin.name}</h3>
            <p className="text-sm text-gray-400 mt-1">{selectedPlugin.desc}</p>
            <p className="text-xs text-blue-400 mt-2">by {selectedPlugin.author}</p>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div>
              <div className="text-xs text-gray-400 mb-1">Rating</div>
              <div className="font-bold flex items-center gap-1">
                <Star size={14} className="fill-yellow-400 text-yellow-400" />
                {selectedPlugin.rating}
              </div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-1">Downloads</div>
              <div className="font-bold">{(selectedPlugin.downloads / 1000).toFixed(1)}K</div>
            </div>
            <div>
              <div className="text-xs text-gray-400 mb-1">Status</div>
              <div className={clsx("font-bold text-sm", selectedPlugin.installed ? "text-green-400" : "text-gray-400")}>
                {selectedPlugin.installed ? "Active" : "Available"}
              </div>
            </div>
          </div>

          <div className="flex gap-2">
            {selectedPlugin.installed ? (
              <button
                onClick={() => handleUninstall(selectedPlugin.id)}
                className="flex-1 bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition"
              >
                Uninstall
              </button>
            ) : (
              <button
                onClick={() => handleInstall(selectedPlugin.id)}
                className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
              >
                <Download size={14} /> Install Plugin
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
