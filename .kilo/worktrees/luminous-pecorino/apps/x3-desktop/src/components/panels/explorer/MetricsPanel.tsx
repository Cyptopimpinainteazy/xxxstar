import { useState, useEffect } from "react";
import {
  BarChart3,
  TrendingUp,
  Activity,
  Zap,
  DollarSign,
  Globe,
  Target,
  ArrowUp,
  Shield,
  CheckCircle,
  AlertTriangle,
  Play,
  Pause,
  RefreshCw,
} from "lucide-react";

const liveMetrics = [
  { label: "Total Volume", value: "$847.3M", change: "+23.7%", icon: DollarSign, color: "text-green-400" },
  { label: "Active Protocols", value: "47", change: "+3", icon: Activity, color: "text-blue-400" },
  { label: "Cross-chain Swaps", value: "12,847", change: "+18.2%", icon: Globe, color: "text-purple-400" },
  { label: "AI Optimizations", value: "1,247", change: "+31.5%", icon: Zap, color: "text-yellow-400" },
  { label: "MEV Protected", value: "$5.7B", change: "+8.9%", icon: Shield, color: "text-cyan-400" },
  { label: "Success Rate", value: "99.97%", change: "+0.02%", icon: CheckCircle, color: "text-emerald-400" },
];

const protocols = [
  { name: "X3 Swap", volume: "$234.5M", apy: "12.4%", users: "14.2K", growth: "+15.3%", status: "active" },
  { name: "X3 Lend", volume: "$187.2M", apy: "8.7%", users: "9.8K", growth: "+22.1%", status: "active" },
  { name: "X3 Bridge", volume: "$156.8M", apy: "—", users: "21.3K", growth: "+31.2%", status: "active" },
  { name: "X3 Yield", volume: "$98.4M", apy: "18.2%", users: "6.4K", growth: "+8.7%", status: "active" },
  { name: "X3 Options", volume: "$72.1M", apy: "24.5%", users: "3.1K", growth: "+45.8%", status: "beta" },
  { name: "X3 Perps", volume: "$98.3M", apy: "—", users: "5.7K", growth: "+12.6%", status: "active" },
];

const marketTrends = [
  { label: "24h Volume", value: "$34.2M", change: "+8.3%" },
  { label: "7d Volume", value: "$214.7M", change: "+15.2%" },
  { label: "TVL", value: "$1.24B", change: "+5.1%" },
  { label: "Unique Users", value: "84.3K", change: "+12.7%" },
  { label: "Avg Tx Size", value: "$2,847", change: "-3.2%" },
  { label: "Gas Savings", value: "$4.1M", change: "+28.4%" },
];

const chainDistribution = [
  { name: "Ethereum", pct: 27.6, color: "bg-blue-500" },
  { name: "Arbitrum", pct: 22.3, color: "bg-blue-400" },
  { name: "Polygon", pct: 18.4, color: "bg-purple-500" },
  { name: "Optimism", pct: 15.8, color: "bg-red-500" },
  { name: "Base", pct: 10.5, color: "bg-blue-600" },
  { name: "Others", pct: 5.4, color: "bg-slate-500" },
];

const timeframes = ["1h", "24h", "7d", "30d"] as const;

export default function MetricsPanel() {
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [timeframe, setTimeframe] = useState<(typeof timeframes)[number]>("24h");
  const [lastUpdated, setLastUpdated] = useState(new Date());

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(() => setLastUpdated(new Date()), 30000);
    return () => clearInterval(interval);
  }, [autoRefresh]);

  return (
    <div className="overflow-y-auto h-full bg-slate-900 text-white p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-blue-400 bg-clip-text text-transparent">
            DeFi Metrics Dashboard
          </h1>
          <p className="text-sm text-slate-400 mt-1">
            Last updated: {lastUpdated.toLocaleTimeString()}
          </p>
        </div>
        <div className="flex items-center gap-3">
          {/* Timeframe selector */}
          <div className="flex bg-slate-800 rounded-lg p-1">
            {timeframes.map((tf) => (
              <button
                key={tf}
                onClick={() => setTimeframe(tf)}
                className={`px-3 py-1 text-xs rounded-md transition-colors ${
                  timeframe === tf
                    ? "bg-purple-600 text-white"
                    : "text-slate-400 hover:text-white"
                }`}
              >
                {tf}
              </button>
            ))}
          </div>
          {/* Auto-refresh toggle */}
          <button
            onClick={() => setAutoRefresh(!autoRefresh)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs transition-colors ${
              autoRefresh
                ? "bg-green-600/20 text-green-400 border border-green-600/30"
                : "bg-slate-800 text-slate-400 border border-slate-700"
            }`}
          >
            {autoRefresh ? <Play className="w-3 h-3" /> : <Pause className="w-3 h-3" />}
            {autoRefresh ? "Live" : "Paused"}
          </button>
          <button
            onClick={() => setLastUpdated(new Date())}
            className="p-1.5 rounded-lg bg-slate-800 text-slate-400 hover:text-white transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Live Metrics Cards */}
      <div className="grid grid-cols-3 gap-4">
        {liveMetrics.map((m) => (
          <div
            key={m.label}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 hover:border-purple-500/30 transition-colors"
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs text-slate-400">{m.label}</span>
              <m.icon className={`w-4 h-4 ${m.color}`} />
            </div>
            <div className="text-xl font-bold">{m.value}</div>
            <div className="flex items-center gap-1 mt-1">
              <ArrowUp className="w-3 h-3 text-green-400" />
              <span className="text-xs text-green-400">{m.change}</span>
              <span className="text-xs text-slate-500 ml-1">{timeframe}</span>
            </div>
          </div>
        ))}
      </div>

      {/* Protocol Performance */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <BarChart3 className="w-5 h-5 text-purple-400" />
          <h2 className="text-lg font-semibold">Protocol Performance</h2>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-slate-400 border-b border-slate-700/50">
                <th className="text-left pb-3 font-medium">Protocol</th>
                <th className="text-right pb-3 font-medium">Volume</th>
                <th className="text-right pb-3 font-medium">APY</th>
                <th className="text-right pb-3 font-medium">Users</th>
                <th className="text-right pb-3 font-medium">Growth</th>
                <th className="text-right pb-3 font-medium">Status</th>
              </tr>
            </thead>
            <tbody>
              {protocols.map((p) => (
                <tr key={p.name} className="border-b border-slate-700/20 hover:bg-slate-700/20">
                  <td className="py-3 font-medium">{p.name}</td>
                  <td className="py-3 text-right">{p.volume}</td>
                  <td className="py-3 text-right text-purple-400">{p.apy}</td>
                  <td className="py-3 text-right text-slate-300">{p.users}</td>
                  <td className="py-3 text-right text-green-400">{p.growth}</td>
                  <td className="py-3 text-right">
                    {p.status === "active" ? (
                      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-green-500/10 text-green-400 text-xs">
                        <CheckCircle className="w-3 h-3" /> Active
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-yellow-500/10 text-yellow-400 text-xs">
                        <AlertTriangle className="w-3 h-3" /> Beta
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Market Trends */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <TrendingUp className="w-5 h-5 text-blue-400" />
          <h2 className="text-lg font-semibold">Market Trends</h2>
        </div>
        <div className="grid grid-cols-3 gap-3">
          {marketTrends.map((t) => (
            <div key={t.label} className="bg-slate-900/50 rounded-lg p-3">
              <span className="text-xs text-slate-400">{t.label}</span>
              <div className="text-lg font-semibold mt-1">{t.value}</div>
              <span
                className={`text-xs ${
                  t.change.startsWith("+") ? "text-green-400" : "text-red-400"
                }`}
              >
                {t.change}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Chain Distribution */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <Target className="w-5 h-5 text-purple-400" />
          <h2 className="text-lg font-semibold">Chain Distribution</h2>
        </div>
        <div className="space-y-3">
          {chainDistribution.map((c) => (
            <div key={c.name} className="flex items-center gap-3">
              <span className="text-sm text-slate-300 w-24">{c.name}</span>
              <div className="flex-1 bg-slate-700/50 rounded-full h-2.5 overflow-hidden">
                <div
                  className={`${c.color} h-full rounded-full transition-all`}
                  style={{ width: `${c.pct}%` }}
                />
              </div>
              <span className="text-sm text-slate-400 w-14 text-right">{c.pct}%</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
