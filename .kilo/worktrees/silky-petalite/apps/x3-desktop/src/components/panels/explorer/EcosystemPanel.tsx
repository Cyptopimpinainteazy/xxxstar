import {
  Server,
  Layers,
  Rocket,
  Brain,
  Globe,
  Shield,
  TrendingUp,
  Users,
  Zap,
} from "lucide-react";

const categories = [
  {
    name: "Core Infrastructure",
    desc: "Foundational blockchain components powering the X3 Chain ecosystem.",
    icon: Server,
    gradient: "from-emerald-600 to-green-600",
    stats: { label: "TVL", value: "$420M" },
    items: ["X3 Kernel", "Consensus Engine", "RPC Gateway", "State Indexer"],
  },
  {
    name: "DeFi Protocols",
    desc: "Decentralized finance protocols for swapping, lending, and yield farming.",
    icon: Layers,
    gradient: "from-blue-600 to-cyan-600",
    stats: { label: "Volume", value: "$847M" },
    items: ["X3 Swap", "X3 Lend", "X3 Yield", "X3 Perps"],
  },
  {
    name: "Launchpad",
    desc: "Token launches, IDOs, and community-driven project incubation.",
    icon: Rocket,
    gradient: "from-purple-600 to-pink-600",
    stats: { label: "Launched", value: "34 Projects" },
    items: ["X3 Launchpad", "Fair Launch", "Community Pools", "Vesting Manager"],
  },
  {
    name: "AI & Analytics",
    desc: "AI-powered analytics, MEV protection, and smart routing.",
    icon: Brain,
    gradient: "from-orange-600 to-amber-600",
    stats: { label: "Optimized", value: "$5.7B" },
    items: ["AI Router", "MEV Shield", "Analytics Hub", "Prediction Engine"],
  },
];

const supportedChains = [
  { name: "Ethereum", color: "bg-blue-500" },
  { name: "Arbitrum", color: "bg-blue-400" },
  { name: "Polygon", color: "bg-purple-500" },
  { name: "Optimism", color: "bg-red-500" },
  { name: "Base", color: "bg-blue-600" },
  { name: "BNB Chain", color: "bg-yellow-500" },
  { name: "Avalanche", color: "bg-red-600" },
  { name: "Fantom", color: "bg-blue-300" },
  { name: "X3 X3 Chain", color: "bg-orange-500" },
];

const liveStats = [
  { label: "Total TVL", value: "$1.24B", icon: Shield, color: "text-emerald-400" },
  { label: "Protocols", value: "47", icon: Layers, color: "text-blue-400" },
  { label: "Unique Users", value: "84.3K", icon: Users, color: "text-purple-400" },
  { label: "Transactions", value: "2.4M", icon: Zap, color: "text-cyan-400" },
  { label: "24h Volume", value: "$34.2M", icon: TrendingUp, color: "text-green-400" },
  { label: "Chains", value: "9", icon: Globe, color: "text-orange-400" },
];

export default function EcosystemPanel() {
  return (
    <div className="overflow-y-auto h-full bg-slate-900 text-white p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold bg-gradient-to-r from-emerald-400 to-green-400 bg-clip-text text-transparent">
          Ecosystem
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Explore the X3 Chain ecosystem and its components
        </p>
      </div>

      {/* Live Stats */}
      <div className="grid grid-cols-6 gap-3">
        {liveStats.map((s) => (
          <div
            key={s.label}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-3 text-center"
          >
            <s.icon className={`w-4 h-4 mx-auto mb-1 ${s.color}`} />
            <div className="text-sm font-bold">{s.value}</div>
            <span className="text-xs text-slate-400">{s.label}</span>
          </div>
        ))}
      </div>

      {/* Category Grid */}
      <div className="grid grid-cols-2 gap-4">
        {categories.map((cat) => (
          <div
            key={cat.name}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-5 hover:border-emerald-500/30 transition-colors"
          >
            <div className="flex items-start gap-3 mb-3">
              <div
                className={`w-10 h-10 rounded-lg bg-gradient-to-br ${cat.gradient} flex items-center justify-center shrink-0`}
              >
                <cat.icon className="w-5 h-5 text-white" />
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between">
                  <h3 className="font-semibold">{cat.name}</h3>
                  <div className="text-right">
                    <span className="text-sm font-bold text-emerald-400">{cat.stats.value}</span>
                    <span className="text-xs text-slate-500 ml-1">{cat.stats.label}</span>
                  </div>
                </div>
                <p className="text-xs text-slate-400 mt-0.5">{cat.desc}</p>
              </div>
            </div>
            <div className="flex flex-wrap gap-1.5 mt-2">
              {cat.items.map((item) => (
                <span
                  key={item}
                  className="px-2 py-1 text-xs rounded-lg bg-slate-700/50 text-slate-300"
                >
                  {item}
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>

      {/* Supported Chains */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <Globe className="w-5 h-5 text-emerald-400" />
          <h2 className="text-lg font-semibold">Supported Chains</h2>
        </div>
        <div className="flex flex-wrap gap-2">
          {supportedChains.map((chain) => (
            <div
              key={chain.name}
              className="flex items-center gap-2 px-3 py-2 rounded-lg bg-slate-900/50 hover:bg-slate-800/50 transition-colors"
            >
              <div className={`w-2.5 h-2.5 rounded-full ${chain.color}`} />
              <span className="text-sm">{chain.name}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
