import { motion } from "framer-motion";
import {
  Wallet,
  TrendingUp,
  Layers,
  Link2,
  Activity,
  ArrowUp,
} from "lucide-react";

const aggregateStats = [
  { label: "Total Value", value: "$127,432.58", icon: Wallet, color: "text-green-400" },
  { label: "Total Yield", value: "$12,847", icon: TrendingUp, color: "text-purple-400" },
  { label: "Positions", value: "12", icon: Layers, color: "text-blue-400" },
  { label: "Chains", value: "5", icon: Link2, color: "text-cyan-400" },
  { label: "Health Score", value: "92.3", icon: Activity, color: "text-emerald-400" },
];

interface Position {
  chain: string;
  chainColor: string;
  protocol: string;
  token: string;
  balance: string;
  healthFactor: number;
  yield: string;
  risk: "Low" | "Medium";
}

const positions: Position[] = [
  { chain: "Ethereum", chainColor: "bg-blue-500", protocol: "Aave V3", token: "ETH", balance: "$42,150.00", healthFactor: 2.45, yield: "$3,200", risk: "Low" },
  { chain: "Ethereum", chainColor: "bg-blue-500", protocol: "Lido", token: "stETH", balance: "$28,740.00", healthFactor: 0, yield: "$2,100", risk: "Low" },
  { chain: "Arbitrum", chainColor: "bg-blue-400", protocol: "GMX", token: "GLP", balance: "$18,420.00", healthFactor: 0, yield: "$2,840", risk: "Medium" },
  { chain: "Arbitrum", chainColor: "bg-blue-400", protocol: "Radiant", token: "USDC", balance: "$12,000.00", healthFactor: 1.87, yield: "$1,080", risk: "Low" },
  { chain: "Polygon", chainColor: "bg-purple-500", protocol: "QuickSwap", token: "MATIC/USDC", balance: "$8,540.00", healthFactor: 0, yield: "$1,240", risk: "Medium" },
  { chain: "Polygon", chainColor: "bg-purple-500", protocol: "Aave V3", token: "WBTC", balance: "$7,200.00", healthFactor: 3.12, yield: "$580", risk: "Low" },
  { chain: "Base", chainColor: "bg-blue-600", protocol: "Aerodrome", token: "ETH/USDC", balance: "$5,832.58", healthFactor: 0, yield: "$920", risk: "Low" },
  { chain: "BSC", chainColor: "bg-yellow-500", protocol: "PancakeSwap", token: "CAKE", balance: "$4,550.00", healthFactor: 0, yield: "$887", risk: "Medium" },
];

function healthColor(hf: number): string {
  if (hf === 0) return "text-slate-500";
  if (hf >= 2) return "text-green-400";
  if (hf >= 1.5) return "text-yellow-400";
  return "text-red-400";
}

function riskBadge(risk: "Low" | "Medium") {
  return risk === "Low" ? (
    <span className="px-2 py-0.5 text-xs rounded-full bg-green-500/10 text-green-400">
      Low
    </span>
  ) : (
    <span className="px-2 py-0.5 text-xs rounded-full bg-yellow-500/10 text-yellow-400">
      Medium
    </span>
  );
}

export default function PortfolioPanel() {
  return (
    <div className="overflow-y-auto h-full bg-slate-900 text-white p-6 space-y-6">
      {/* Header */}
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
      >
        <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-blue-400 bg-clip-text text-transparent">
          Cross-Chain Portfolio
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Aggregated view across all connected chains
        </p>
      </motion.div>

      {/* Aggregate Stats */}
      <div className="grid grid-cols-5 gap-3">
        {aggregateStats.map((s, i) => (
          <motion.div
            key={s.label}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.3, delay: i * 0.05 }}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 text-center"
          >
            <s.icon className={`w-5 h-5 mx-auto mb-1 ${s.color}`} />
            <div className="text-lg font-bold">{s.value}</div>
            <span className="text-xs text-slate-400">{s.label}</span>
          </motion.div>
        ))}
      </div>

      {/* Position Table */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.2 }}
        className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5"
      >
        <div className="flex items-center gap-2 mb-4">
          <Layers className="w-5 h-5 text-purple-400" />
          <h2 className="text-lg font-semibold">Positions</h2>
          <span className="ml-auto text-xs text-slate-400">{positions.length} active</span>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-slate-400 border-b border-slate-700/50">
                <th className="text-left pb-3 font-medium">Chain</th>
                <th className="text-left pb-3 font-medium">Protocol</th>
                <th className="text-left pb-3 font-medium">Token</th>
                <th className="text-right pb-3 font-medium">Balance</th>
                <th className="text-right pb-3 font-medium">Health</th>
                <th className="text-right pb-3 font-medium">Yield</th>
                <th className="text-right pb-3 font-medium">Risk</th>
              </tr>
            </thead>
            <tbody>
              {positions.map((p, i) => (
                <motion.tr
                  key={i}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.2, delay: 0.3 + i * 0.04 }}
                  className="border-b border-slate-700/20 hover:bg-slate-700/20"
                >
                  <td className="py-3">
                    <div className="flex items-center gap-2">
                      <div className={`w-2 h-2 rounded-full ${p.chainColor}`} />
                      <span>{p.chain}</span>
                    </div>
                  </td>
                  <td className="py-3 text-slate-300">{p.protocol}</td>
                  <td className="py-3 font-medium">{p.token}</td>
                  <td className="py-3 text-right">{p.balance}</td>
                  <td className={`py-3 text-right font-mono ${healthColor(p.healthFactor)}`}>
                    {p.healthFactor > 0 ? p.healthFactor.toFixed(2) : "—"}
                  </td>
                  <td className="py-3 text-right text-green-400">
                    <div className="flex items-center justify-end gap-1">
                      <ArrowUp className="w-3 h-3" />
                      {p.yield}
                    </div>
                  </td>
                  <td className="py-3 text-right">{riskBadge(p.risk)}</td>
                </motion.tr>
              ))}
            </tbody>
          </table>
        </div>
      </motion.div>
    </div>
  );
}
