import {
  Brain,
  Zap,
  Cpu,
  TrendingUp,
  Activity,
  Target,
  Network,
  DollarSign,
  BarChart3,
  Settings,
  PlayCircle,
  RefreshCw,
} from "lucide-react";

const nodes = [
  {
    id: "alpha-001",
    name: "Alpha Node",
    gpu: "RTX 4090",
    status: "active",
    utilization: 94,
    currentTask: "LLM Inference Batch",
    completed: 12847,
    earnings: "$1,247.50",
    location: "US-East",
  },
  {
    id: "beta-002",
    name: "Beta Node",
    gpu: "A100 80GB",
    status: "active",
    utilization: 87,
    currentTask: "Stable Diffusion XL",
    completed: 9432,
    earnings: "$2,891.20",
    location: "EU-West",
  },
  {
    id: "gamma-003",
    name: "Gamma Node",
    gpu: "RTX 4080",
    status: "idle",
    utilization: 12,
    currentTask: "Awaiting Assignment",
    completed: 7621,
    earnings: "$891.40",
    location: "US-West",
  },
  {
    id: "delta-004",
    name: "Delta Node",
    gpu: "H100 SXM",
    status: "active",
    utilization: 98,
    currentTask: "Model Fine-Tuning",
    completed: 15203,
    earnings: "$4,102.80",
    location: "AP-Tokyo",
  },
];

const stats = [
  { label: "Total Nodes", value: "247", icon: Network, color: "text-purple-400" },
  { label: "Active Nodes", value: "198", icon: Activity, color: "text-green-400" },
  { label: "Compute Power", value: "847 TFLOPS", icon: Cpu, color: "text-blue-400" },
  { label: "Tasks / Sec", value: "12,847", icon: Zap, color: "text-yellow-400" },
  { label: "Total Earnings", value: "$2.4M", icon: DollarSign, color: "text-emerald-400" },
  { label: "Avg Response", value: "12ms", icon: Target, color: "text-orange-400" },
];

const arbitrageOpportunities = [
  { pair: "ETH/USDC", spread: "0.12%", profit: "$847", exchange: "Uniswap → SushiSwap", time: "2s ago" },
  { pair: "WBTC/ETH", spread: "0.08%", profit: "$1,203", exchange: "Curve → Balancer", time: "5s ago" },
  { pair: "DAI/USDT", spread: "0.03%", profit: "$124", exchange: "Aave → Compound", time: "8s ago" },
];

const strategies = [
  { name: "Flash Loan Arb", roi: "+42.7%", trades: 1247, winRate: "94.2%", status: "active" },
  { name: "MEV Sandwich", roi: "+28.3%", trades: 892, winRate: "87.6%", status: "active" },
  { name: "Cross-DEX Spread", roi: "+15.9%", trades: 2104, winRate: "91.1%", status: "paused" },
];

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    active: "bg-green-500/20 text-green-400 border-green-500/30",
    idle: "bg-yellow-500/20 text-yellow-400 border-yellow-500/30",
    offline: "bg-red-500/20 text-red-400 border-red-500/30",
  };
  return (
    <span className={`px-2 py-0.5 rounded-full text-xs font-medium border ${colors[status] ?? colors.idle}`}>
      {status}
    </span>
  );
}

export default function AISwarmPanel() {
  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-slate-900 via-[#0c0a1a] to-black text-white p-6 space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Brain className="w-8 h-8 text-purple-400" />
          <div>
            <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent">
              GPU AI Swarm Dashboard
            </h1>
            <p className="text-slate-400 text-sm">Real-time distributed compute network</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 transition">
            <RefreshCw className="w-4 h-4 text-slate-400" />
          </button>
          <button className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 transition">
            <Settings className="w-4 h-4 text-slate-400" />
          </button>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        {stats.map((s) => (
          <div
            key={s.label}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 flex flex-col items-center gap-2"
          >
            <s.icon className={`w-5 h-5 ${s.color}`} />
            <span className="text-lg font-bold">{s.value}</span>
            <span className="text-xs text-slate-400">{s.label}</span>
          </div>
        ))}
      </div>

      {/* Node Table */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl overflow-hidden">
        <div className="px-5 py-4 border-b border-slate-700/50 flex items-center justify-between">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <Cpu className="w-5 h-5 text-purple-400" />
            Active Nodes
          </h2>
          <button className="flex items-center gap-1 text-sm text-purple-400 hover:text-purple-300 transition">
            <PlayCircle className="w-4 h-4" /> Deploy New Node
          </button>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-slate-400 border-b border-slate-700/50">
                <th className="px-5 py-3 font-medium">Node</th>
                <th className="px-5 py-3 font-medium">GPU</th>
                <th className="px-5 py-3 font-medium">Status</th>
                <th className="px-5 py-3 font-medium">Utilization</th>
                <th className="px-5 py-3 font-medium">Current Task</th>
                <th className="px-5 py-3 font-medium">Completed</th>
                <th className="px-5 py-3 font-medium">Earnings</th>
                <th className="px-5 py-3 font-medium">Location</th>
              </tr>
            </thead>
            <tbody>
              {nodes.map((n) => (
                <tr key={n.id} className="border-b border-slate-700/30 hover:bg-slate-700/20 transition">
                  <td className="px-5 py-3 font-medium">{n.name}</td>
                  <td className="px-5 py-3 text-slate-300">{n.gpu}</td>
                  <td className="px-5 py-3">
                    <StatusBadge status={n.status} />
                  </td>
                  <td className="px-5 py-3">
                    <div className="flex items-center gap-2">
                      <div className="w-24 h-2 bg-slate-700 rounded-full overflow-hidden">
                        <div
                          className={`h-full rounded-full ${
                            n.utilization > 90
                              ? "bg-red-500"
                              : n.utilization > 60
                              ? "bg-yellow-500"
                              : "bg-green-500"
                          }`}
                          style={{ width: `${n.utilization}%` }}
                        />
                      </div>
                      <span className="text-xs text-slate-400">{n.utilization}%</span>
                    </div>
                  </td>
                  <td className="px-5 py-3 text-slate-300">{n.currentTask}</td>
                  <td className="px-5 py-3 text-slate-300">{n.completed.toLocaleString()}</td>
                  <td className="px-5 py-3 text-emerald-400 font-medium">{n.earnings}</td>
                  <td className="px-5 py-3 text-slate-400">{n.location}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Bottom Sections */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Recent Arbitrage Opportunities */}
        <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
          <h2 className="text-lg font-semibold flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-green-400" />
            Recent Arbitrage Opportunities
          </h2>
          <div className="space-y-3">
            {arbitrageOpportunities.map((opp, i) => (
              <div key={i} className="flex items-center justify-between bg-slate-700/30 rounded-lg p-3">
                <div>
                  <div className="font-medium">{opp.pair}</div>
                  <div className="text-xs text-slate-400">
                    {opp.exchange} &middot; {opp.time}
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-green-400 font-medium">{opp.profit}</div>
                  <div className="text-xs text-slate-400">Spread: {opp.spread}</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Strategy Performance */}
        <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
          <h2 className="text-lg font-semibold flex items-center gap-2 mb-4">
            <BarChart3 className="w-5 h-5 text-purple-400" />
            Strategy Performance
          </h2>
          <div className="space-y-3">
            {strategies.map((s, i) => (
              <div key={i} className="flex items-center justify-between bg-slate-700/30 rounded-lg p-3">
                <div>
                  <div className="font-medium flex items-center gap-2">
                    {s.name}
                    <StatusBadge status={s.status === "active" ? "active" : "idle"} />
                  </div>
                  <div className="text-xs text-slate-400">
                    {s.trades.toLocaleString()} trades &middot; {s.winRate} win rate
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-emerald-400 font-bold">{s.roi}</div>
                  <div className="text-xs text-slate-400">ROI</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
