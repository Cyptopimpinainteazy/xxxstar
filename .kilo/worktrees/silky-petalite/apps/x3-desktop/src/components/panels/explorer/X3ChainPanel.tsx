import { useEffect, useState } from "react";
import {
  Dna,
  Cpu,
  Shield,
  Code2,
  Zap,
  Atom,
  BookOpen,
  Network,
  ArrowRight,
  Sparkles,
} from "lucide-react";

/* ------------------------------------------------------------------ */
/*  Animated counter hook                                              */
/* ------------------------------------------------------------------ */
function useAnimatedCounter(target: number, duration = 2000) {
  const [count, setCount] = useState(0);
  useEffect(() => {
    let start = 0;
    const step = target / (duration / 16);
    const id = setInterval(() => {
      start += step;
      if (start >= target) {
        setCount(target);
        clearInterval(id);
      } else {
        setCount(Math.floor(start));
      }
    }, 16);
    return () => clearInterval(id);
  }, [target, duration]);
  return count;
}

/* ------------------------------------------------------------------ */
/*  Data                                                               */
/* ------------------------------------------------------------------ */
const heroStats = [
  { label: "Mutations", value: 1200000, suffix: "", format: true },
  { label: "Swarm Nodes", value: 247, suffix: "", format: false },
  { label: "Receipts", value: 89000, suffix: "", format: true },
  { label: "Scripts", value: 12000, suffix: "", format: true },
];

const navCards = [
  {
    title: "Evolution Engine",
    desc: "Self-optimizing runtime that mutates chain parameters in real-time",
    icon: Dna,
    color: "from-green-500 to-emerald-600",
    accent: "text-green-400",
  },
  {
    title: "GPU Swarm",
    desc: "Distributed GPU compute network for AI/ML workloads",
    icon: Cpu,
    color: "from-cyan-500 to-blue-600",
    accent: "text-cyan-400",
  },
  {
    title: "State Verifier",
    desc: "Canonical ledger verification with cryptographic proofs",
    icon: Shield,
    color: "from-amber-500 to-orange-600",
    accent: "text-amber-400",
  },
  {
    title: "Smart Scripts",
    desc: "Multi-VM smart contract development and deployment",
    icon: Code2,
    color: "from-violet-500 to-purple-600",
    accent: "text-violet-400",
  },
];

const features = [
  {
    title: "Multi-VM Architecture",
    desc: "Run EVM, SVM, and native WASM contracts side-by-side on a single chain with unified state.",
    icon: Network,
  },
  {
    title: "Atomic Execution",
    desc: "Cross-VM atomic transactions ensure all-or-nothing execution across heterogeneous virtual machines.",
    icon: Atom,
  },
  {
    title: "Canonical Ledger",
    desc: "Immutable, verifiable state root anchoring across all execution layers with deterministic finality.",
    icon: BookOpen,
  },
];

/* ================================================================== */
/*  AnimatedStat                                                       */
/* ================================================================== */
function AnimatedStat({
  label,
  target,
  shouldFormat,
}: {
  label: string;
  target: number;
  shouldFormat: boolean;
}) {
  const count = useAnimatedCounter(target, 2200);
  const display = shouldFormat ? count.toLocaleString() : String(count);
  return (
    <div className="text-center">
      <div className="text-3xl md:text-4xl font-bold bg-gradient-to-r from-green-400 to-cyan-400 bg-clip-text text-transparent">
        {display}
      </div>
      <div className="text-xs text-slate-400 mt-1">{label}</div>
    </div>
  );
}

/* ================================================================== */
/*  X3ChainPanel                                                       */
/* ================================================================== */
export default function X3ChainPanel() {
  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-slate-900 via-[#020d0a] to-black text-white p-6 space-y-10">
      {/* ---- Hero ---- */}
      <div className="text-center space-y-4 pt-6 pb-2">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-green-500/10 border border-green-500/20 text-green-400 text-xs font-medium">
          <Sparkles className="w-3 h-3" /> Live on Testnet
        </div>
        <h1 className="text-4xl md:text-5xl font-extrabold leading-tight">
          <span className="bg-gradient-to-r from-green-400 via-emerald-300 to-cyan-400 bg-clip-text text-transparent animate-gradient">
            The Self-Evolving
          </span>
          <br />
          <span className="text-white">Blockchain</span>
        </h1>
        <p className="text-slate-400 max-w-lg mx-auto text-sm">
          X3 Chain adapts, mutates, and evolves its runtime in real-time — powering
          self-optimizing decentralized infrastructure at scale.
        </p>
      </div>

      {/* ---- Animated stats ---- */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-6 max-w-2xl mx-auto">
        {heroStats.map((s) => (
          <AnimatedStat
            key={s.label}
            label={s.label}
            target={s.value}
            shouldFormat={s.format}
          />
        ))}
      </div>

      {/* ---- Navigation cards ---- */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {navCards.map((card) => (
          <div
            key={card.title}
            className="group relative bg-slate-800/50 border border-slate-700/50 rounded-2xl p-5 hover:border-green-500/30 transition-all cursor-pointer overflow-hidden"
          >
            <div
              className={`absolute inset-0 bg-gradient-to-br ${card.color} opacity-0 group-hover:opacity-5 transition-opacity`}
            />
            <div className="relative flex items-start gap-4">
              <div className="p-2.5 rounded-xl bg-slate-700/50 shrink-0">
                <card.icon className={`w-6 h-6 ${card.accent}`} />
              </div>
              <div className="flex-1">
                <h3 className="font-semibold text-slate-100 flex items-center gap-2">
                  {card.title}
                  <ArrowRight className="w-4 h-4 text-slate-600 group-hover:text-green-400 transition" />
                </h3>
                <p className="text-sm text-slate-400 mt-1">{card.desc}</p>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* ---- Feature cards ---- */}
      <div className="space-y-4">
        <h2 className="text-lg font-bold text-slate-200 flex items-center gap-2">
          <Zap className="w-5 h-5 text-cyan-400" />
          Core Technology
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {features.map((f) => (
            <div
              key={f.title}
              className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5 space-y-3"
            >
              <f.icon className="w-6 h-6 text-emerald-400" />
              <h3 className="font-semibold text-slate-100">{f.title}</h3>
              <p className="text-sm text-slate-400 leading-relaxed">{f.desc}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
