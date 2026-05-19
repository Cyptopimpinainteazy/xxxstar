import {
  Server,
  Shield,
  BarChart3,
  Activity,
  Globe,
  ExternalLink,
} from "lucide-react";
import { useNetworkStats, useAuthorities } from "@/hooks/useSubstrate";

const sectionCards = [
  {
    title: "Become a Validator",
    desc: "Secure the network and earn ~8% APY by running a validator node.",
    icon: Shield,
    gradient: "from-blue-600 to-cyan-600",
    badges: ["~8% APY", "256 Slots", "Low Hardware"],
  },
  {
    title: "RPC Providers",
    desc: "Connect your dApp to X3 Chain with reliable RPC endpoints.",
    icon: Server,
    gradient: "from-purple-600 to-blue-600",
    badges: ["WebSocket", "HTTPS", "Archive"],
  },
  {
    title: "Network Status",
    desc: "Real-time network health, performance metrics, and uptime.",
    icon: Activity,
    gradient: "from-green-600 to-emerald-600",
    badges: ["99.99% Uptime", "6s Blocks", "Live"],
  },
  {
    title: "On & Off Ramps",
    desc: "Buy and sell tokens with fiat currency through trusted partners.",
    icon: Globe,
    gradient: "from-orange-600 to-amber-600",
    badges: ["Fiat", "Global", "KYC"],
  },
];

const blockExplorers = [
  {
    name: "X3scan",
    desc: "Full-featured block explorer with analytics",
    features: ["Blocks", "Extrinsics", "Accounts", "Staking"],
  },
  {
    name: "x3FM",
    desc: "Developer-focused explorer with API access",
    features: ["API", "Verified Contracts", "Tokens"],
  },
  {
    name: "X3 Explorer",
    desc: "Community-built lightweight explorer",
    features: ["Fast Search", "Charts", "Validators"],
  },
  {
    name: "Orb",
    desc: "AI-powered explorer with smart analytics",
    features: ["AI Search", "Predictions", "Alerts"],
  },
];

const dataIndexers = [
  {
    name: "X3 Indexer",
    desc: "Native indexing service for X3 Chain data.",
    status: "active",
  },
  {
    name: "SubQuery",
    desc: "Flexible, fast & open indexing for Substrate chains.",
    status: "active",
  },
  {
    name: "The Graph",
    desc: "Decentralized protocol for indexing and querying.",
    status: "coming-soon",
  },
];

export default function NetworkPanel2() {
  const { data: networkStats, error: statsError } = useNetworkStats();
  const { data: authorities, error: authError } = useAuthorities();

  const liveStats = [
    {
      label: "Block Height",
      value: networkStats?.blockNumber
        ? `#${networkStats.blockNumber.toLocaleString()}`
        : "—",
    },
    {
      label: "Active Validators",
      value: authorities?.length?.toString() ?? "—",
    },
    {
      label: "Peers",
      value: networkStats?.peerCount?.toString() ?? "—",
    },
    {
      label: "Sync Status",
      value: networkStats?.isSyncing ? "Syncing…" : "Synced",
    },
  ];

  return (
    <div className="overflow-y-auto h-full bg-slate-900 text-white p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold bg-gradient-to-r from-blue-400 to-cyan-400 bg-clip-text text-transparent">
          Network Hub
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          X3 Chain network resources and infrastructure
        </p>
      </div>

      {/* Live Stats */}
      <div className="grid grid-cols-4 gap-3">
        {liveStats.map((s) => (
          <div
            key={s.label}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 text-center"
          >
            <span className="text-xs text-slate-400">{s.label}</span>
            <div className="text-lg font-bold mt-1 text-blue-400">{s.value}</div>
          </div>
        ))}
      </div>

      {(statsError || authError) && (
        <div className="bg-yellow-900/20 border border-yellow-700/30 rounded-lg p-3 text-sm text-yellow-400">
          Unable to connect to node. Showing cached data.
        </div>
      )}

      {/* Section Cards */}
      <div className="grid grid-cols-2 gap-4">
        {sectionCards.map((card) => (
          <div
            key={card.title}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-5 hover:border-blue-500/30 transition-colors"
          >
            <div className="flex items-start gap-3 mb-3">
              <div
                className={`w-10 h-10 rounded-lg bg-gradient-to-br ${card.gradient} flex items-center justify-center`}
              >
                <card.icon className="w-5 h-5 text-white" />
              </div>
              <div className="flex-1">
                <h3 className="font-semibold">{card.title}</h3>
                <p className="text-xs text-slate-400 mt-0.5">{card.desc}</p>
              </div>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {card.badges.map((badge) => (
                <span
                  key={badge}
                  className="px-2 py-0.5 text-xs rounded-full bg-slate-700/50 text-slate-300"
                >
                  {badge}
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>

      {/* Block Explorers */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <BarChart3 className="w-5 h-5 text-blue-400" />
          <h2 className="text-lg font-semibold">Block Explorers</h2>
        </div>
        <div className="grid grid-cols-2 gap-3">
          {blockExplorers.map((ex) => (
            <div
              key={ex.name}
              className="bg-slate-900/50 rounded-lg p-4 hover:bg-slate-800/50 transition-colors"
            >
              <div className="flex items-center justify-between mb-2">
                <span className="font-semibold text-sm">{ex.name}</span>
                <ExternalLink className="w-3.5 h-3.5 text-slate-500" />
              </div>
              <p className="text-xs text-slate-400 mb-2">{ex.desc}</p>
              <div className="flex flex-wrap gap-1">
                {ex.features.map((f) => (
                  <span
                    key={f}
                    className="px-2 py-0.5 text-xs rounded bg-blue-500/10 text-blue-400"
                  >
                    {f}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Data Indexers */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <Server className="w-5 h-5 text-cyan-400" />
          <h2 className="text-lg font-semibold">Data Indexers</h2>
        </div>
        <div className="space-y-3">
          {dataIndexers.map((idx) => (
            <div
              key={idx.name}
              className="flex items-center justify-between bg-slate-900/50 rounded-lg p-4"
            >
              <div>
                <div className="text-sm font-semibold">{idx.name}</div>
                <div className="text-xs text-slate-400 mt-0.5">{idx.desc}</div>
              </div>
              <span
                className={`px-2 py-0.5 text-xs rounded-full ${
                  idx.status === "active"
                    ? "bg-green-500/10 text-green-400"
                    : "bg-yellow-500/10 text-yellow-400"
                }`}
              >
                {idx.status === "active" ? "Active" : "Coming Soon"}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
