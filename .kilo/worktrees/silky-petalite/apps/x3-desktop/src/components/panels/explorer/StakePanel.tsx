import { useState } from "react";
import {
  Coins,
  Lock,
  Star,
  Trophy,
  Zap,
  TrendingUp,
  Users,
  Shield,
  Clock,
} from "lucide-react";

interface Tier {
  name: string;
  apy: string;
  lock: string;
  multiplier: string;
  icon: React.ElementType;
  color: string;
  border: string;
  bg: string;
}

const tiers: Tier[] = [
  {
    name: "Flexible",
    apy: "5%",
    lock: "No lock",
    multiplier: "1x",
    icon: Coins,
    color: "text-slate-300",
    border: "border-slate-600",
    bg: "bg-slate-800/60",
  },
  {
    name: "Silver",
    apy: "10%",
    lock: "30 days",
    multiplier: "2x",
    icon: Star,
    color: "text-slate-200",
    border: "border-slate-500",
    bg: "bg-slate-800/70",
  },
  {
    name: "Gold",
    apy: "15%",
    lock: "90 days",
    multiplier: "3x",
    icon: Trophy,
    color: "text-amber-400",
    border: "border-amber-600/40",
    bg: "bg-amber-950/20",
  },
  {
    name: "Diamond",
    apy: "25%",
    lock: "180 days",
    multiplier: "5x",
    icon: Zap,
    color: "text-orange-400",
    border: "border-orange-500/40",
    bg: "bg-orange-950/20",
  },
];

const globalStats = [
  { label: "Total Staked", value: "$124.5M", icon: Lock },
  { label: "Total Stakers", value: "45K", icon: Users },
  { label: "Avg APY", value: "12.3%", icon: TrendingUp },
];

export default function StakePanel() {
  const [selectedTier, setSelectedTier] = useState<string>("Gold");
  const [amount, setAmount] = useState("");

  const currentTier = tiers.find((t) => t.name === selectedTier)!;

  return (
    <div className="overflow-y-auto h-full bg-slate-900 text-white p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold bg-gradient-to-r from-orange-400 to-amber-400 bg-clip-text text-transparent">
          Stake X3
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Lock tokens to earn rewards and boost your multiplier
        </p>
      </div>

      {/* Global Stats */}
      <div className="grid grid-cols-3 gap-3">
        {globalStats.map((s) => (
          <div
            key={s.label}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 text-center"
          >
            <s.icon className="w-5 h-5 mx-auto mb-1 text-orange-400" />
            <div className="text-lg font-bold">{s.value}</div>
            <span className="text-xs text-slate-400">{s.label}</span>
          </div>
        ))}
      </div>

      {/* Tier Grid */}
      <div>
        <h2 className="text-lg font-semibold mb-3">Select Tier</h2>
        <div className="grid grid-cols-4 gap-3">
          {tiers.map((tier) => {
            const selected = selectedTier === tier.name;
            return (
              <button
                key={tier.name}
                onClick={() => setSelectedTier(tier.name)}
                className={`relative rounded-xl p-4 border text-left transition-all ${
                  selected
                    ? `${tier.bg} ${tier.border} border-2 ring-1 ring-orange-500/30`
                    : "bg-slate-800/40 border-slate-700/50 hover:border-slate-600"
                }`}
              >
                {selected && (
                  <div className="absolute top-2 right-2">
                    <Shield className="w-4 h-4 text-orange-400" />
                  </div>
                )}
                <tier.icon className={`w-6 h-6 mb-2 ${tier.color}`} />
                <div className="font-semibold text-sm">{tier.name}</div>
                <div className="text-xl font-bold mt-1 text-orange-400">{tier.apy} APY</div>
                <div className="mt-2 space-y-1 text-xs text-slate-400">
                  <div className="flex items-center gap-1">
                    <Clock className="w-3 h-3" />
                    <span>{tier.lock}</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <Zap className="w-3 h-3" />
                    <span>{tier.multiplier} multiplier</span>
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Stake Input */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <h2 className="text-lg font-semibold mb-3">Stake Amount</h2>
        <div className="flex items-center gap-3">
          <div className="flex-1 relative">
            <input
              type="text"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.00"
              className="w-full bg-slate-900/80 border border-slate-700 rounded-lg px-4 py-3 text-lg font-mono focus:outline-none focus:border-orange-500/50 transition-colors"
            />
            <span className="absolute right-4 top-1/2 -translate-y-1/2 text-sm text-slate-500">
              X3
            </span>
          </div>
          <button
            onClick={() => setAmount("10000")}
            className="px-4 py-3 rounded-lg bg-orange-600/20 text-orange-400 text-sm font-semibold hover:bg-orange-600/30 transition-colors border border-orange-600/30"
          >
            MAX
          </button>
        </div>
        <div className="mt-3 flex items-center justify-between text-xs text-slate-400">
          <span>Available: 10,000 X3</span>
          <span>
            Est. reward: {amount ? `${(parseFloat(amount.replace(/,/g, "")) * parseFloat(currentTier.apy) / 100).toFixed(2)} X3/yr` : "—"}
          </span>
        </div>
        <button className="w-full mt-4 py-3 rounded-lg bg-gradient-to-r from-orange-600 to-amber-600 text-white font-semibold hover:from-orange-500 hover:to-amber-500 transition-all">
          Stake {selectedTier} — {currentTier.apy} APY
        </button>
      </div>

      {/* Info */}
      <div className="bg-slate-800/20 border border-slate-700/30 rounded-xl p-4 text-xs text-slate-400 space-y-1">
        <p>• Rewards are distributed every epoch (~6 hours).</p>
        <p>• Early unstake incurs a 10% penalty on earned rewards.</p>
        <p>• Higher tiers unlock governance voting power and exclusive features.</p>
      </div>
    </div>
  );
}
