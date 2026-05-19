import React, { useState, useMemo } from "react";
import { useNavigate } from "react-router-dom";

interface AppEntry {
  id: string;
  title: string;
  description: string;
  icon: string;
  category: string;
  panelId: string;
}

const APPS: AppEntry[] = [
  // Infrastructure
  {
    id: "inferstructor",
    title: "Inferstructor Dashboard",
    description: "GPU validator admin with TPS leaderboard and orchestra controls",
    icon: "⚡",
    category: "Infrastructure",
    panelId: "inferstructor-dashboard",
  },
  {
    id: "infra-dashboard",
    title: "Infra Dashboard",
    description: "Infrastructure monitoring and blockchain explorer",
    icon: "🔧",
    category: "Infrastructure",
    panelId: "infra-dashboard",
  },
  {
    id: "tps-monitor",
    title: "TPS Monitor",
    description: "Blockchain transactions-per-second monitoring and leaderboard",
    icon: "📈",
    category: "Infrastructure",
    panelId: "tps-monitor",
  },
  {
    id: "swarm-autonomic",
    title: "Swarm Autonomic",
    description: "Swarm infrastructure autonomic control plane",
    icon: "🤖",
    category: "Infrastructure",
    panelId: "swarm-autonomic",
  },
  {
    id: "jury-anchoring",
    title: "Jury Anchoring UI",
    description: "Blockchain jury anchoring visualization component",
    icon: "⚖️",
    category: "Infrastructure",
    panelId: "jury-anchoring",
  },
  // Trading & DeFi
  {
    id: "intelligence",
    title: "X3 Intelligence",
    description: "Arbitrage control surface with floor analytics and intent matching",
    icon: "🧠",
    category: "Trading & DeFi",
    panelId: "x3-intelligence-full",
  },
  {
    id: "wallet-app",
    title: "X3 Wallet",
    description: "Multi-chain cryptocurrency wallet with Polkadex launchpad",
    icon: "💼",
    category: "Trading & DeFi",
    panelId: "wallet-app",
  },
  {
    id: "dex-app",
    title: "X3 DEX",
    description: "Decentralized exchange interface",
    icon: "🔄",
    category: "Trading & DeFi",
    panelId: "dex-app",
  },
  // Validators
  {
    id: "validators",
    title: "Validator Globe",
    description: "3D globe visualization of validator nodes with live RPC polling",
    icon: "🌐",
    category: "Validators",
    panelId: "validators-globe-embed",
  },
  // Analytics
  {
    id: "dashboard",
    title: "Modular Dashboard",
    description: "Extensible 50+ panel modular dashboard system",
    icon: "📊",
    category: "Analytics",
    panelId: "modular-dashboard",
  },
  // Site Pages
  {
    id: "x3fronend",
    title: "X3 Landing & Pages",
    description: "Marketing landing page and 40+ specialized blockchain dashboard pages",
    icon: "🚀",
    category: "Site Pages",
    panelId: "x3-frontend",
  },
  {
    id: "mainnet-progress",
    title: "Mainnet Progress",
    description: "Network mainnet launch progress tracker",
    icon: "🏁",
    category: "Site Pages",
    panelId: "mainnet-progress",
  },
  // Tools
  {
    id: "x3-extension",
    title: "X3 Browser Extension",
    description: "Chrome/browser extension for X3 Chain",
    icon: "🧩",
    category: "Tools",
    panelId: "x3-extension",
  },
];

const CATEGORIES = ["All", "Infrastructure", "Trading & DeFi", "Validators", "Analytics", "Site Pages", "Tools"];

const CATEGORY_COLORS: Record<string, string> = {
  Infrastructure: "text-[#00bfff] border-[#00bfff]/30 bg-[#00bfff]/10",
  "Trading & DeFi": "text-[#00e5c3] border-[#00e5c3]/30 bg-[#00e5c3]/10",
  Validators: "text-[#a78bfa] border-[#a78bfa]/30 bg-[#a78bfa]/10",
  Analytics: "text-[#fbbf24] border-[#fbbf24]/30 bg-[#fbbf24]/10",
  "Site Pages": "text-[#ff6b35] border-[#ff6b35]/30 bg-[#ff6b35]/10",
  Tools: "text-[#34d399] border-[#34d399]/30 bg-[#34d399]/10",
};

const AllAppsLauncher: React.FC = () => {
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [activeCategory, setActiveCategory] = useState("All");

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return APPS.filter((app) => {
      const matchesCategory = activeCategory === "All" || app.category === activeCategory;
      const matchesSearch =
        !q ||
        app.title.toLowerCase().includes(q) ||
        app.description.toLowerCase().includes(q) ||
        app.category.toLowerCase().includes(q);
      return matchesCategory && matchesSearch;
    });
  }, [search, activeCategory]);

  const grouped = useMemo(() => {
    const map: Record<string, AppEntry[]> = {};
    for (const app of filtered) {
      if (!map[app.category]) map[app.category] = [];
      map[app.category].push(app);
    }
    return map;
  }, [filtered]);

  const handleAppClick = (app: AppEntry) => {
    // Navigate to main desktop with the panel open via hash/query
    navigate(`/?panel=${app.panelId}`);
  };

  return (
    <div className="min-h-screen bg-[#07070d] text-white flex flex-col">
      {/* Header */}
      <div className="border-b border-[#1a1a2e] px-8 py-6">
        <div className="flex items-center justify-between mb-5">
          <div>
            <h1 className="text-2xl font-bold text-white tracking-tight">All Apps</h1>
            <p className="text-sm text-[#666] mt-0.5">No frontend left behind — {APPS.length} surfaces unified</p>
          </div>
          <button
            onClick={() => navigate(-1)}
            className="text-xs text-[#666] hover:text-white transition-colors flex items-center gap-1.5"
          >
            ← Back
          </button>
        </div>

        {/* Search */}
        <input
          type="text"
          placeholder="Search apps…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full max-w-md bg-[#111] border border-[#222] rounded-lg px-4 py-2 text-sm
            text-white placeholder-[#555] focus:outline-none focus:border-[#ff6b35]/50"
        />

        {/* Category filter */}
        <div className="flex gap-2 mt-4 flex-wrap">
          {CATEGORIES.map((cat) => (
            <button
              key={cat}
              onClick={() => setActiveCategory(cat)}
              className={`text-xs px-3 py-1 rounded-full border transition-colors ${
                activeCategory === cat
                  ? "bg-[#ff6b35] border-[#ff6b35] text-white"
                  : "border-[#2a2a3e] text-[#888] hover:text-white hover:border-[#444]"
              }`}
            >
              {cat}
            </button>
          ))}
        </div>
      </div>

      {/* Grid */}
      <div className="flex-1 overflow-y-auto px-8 py-6 space-y-8">
        {Object.keys(grouped).length === 0 ? (
          <div className="text-center text-[#555] py-20 text-sm">No apps match your search.</div>
        ) : (
          Object.entries(grouped).map(([category, apps]) => (
            <section key={category}>
              <h2 className="text-xs font-semibold uppercase tracking-widest text-[#555] mb-3">
                {category}
              </h2>
              <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
                {apps.map((app) => (
                  <button
                    key={app.id}
                    onClick={() => handleAppClick(app)}
                    className="text-left bg-[#0d0d1a] border border-[#1a1a2e] rounded-xl p-4
                      hover:border-[#ff6b35]/40 hover:bg-[#0f0f20] transition-all group"
                  >
                    <div className="text-3xl mb-3">{app.icon}</div>
                    <div className="text-sm font-semibold text-white group-hover:text-[#ff6b35] transition-colors leading-tight mb-1">
                      {app.title}
                    </div>
                    <div className="text-[10px] text-[#666] leading-relaxed mb-3 line-clamp-2">
                      {app.description}
                    </div>
                    <span
                      className={`text-[9px] px-2 py-0.5 rounded-full border font-medium ${
                        CATEGORY_COLORS[app.category] ?? "text-[#888] border-[#333] bg-[#333]/20"
                      }`}
                    >
                      {app.category}
                    </span>
                  </button>
                ))}
              </div>
            </section>
          ))
        )}
      </div>
    </div>
  );
};

export default AllAppsLauncher;
