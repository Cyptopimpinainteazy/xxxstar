import React, { useEffect, useState, useRef } from "react";
import { motion, useInView } from "framer-motion";
import {
  Globe,
  Layers,
  BarChart3,
  Brain,
  Zap,
  ShieldCheck,
  Clock,
  Link2,
} from "lucide-react";

/* ------------------------------------------------------------------ */
/*  Ticker data                                                        */
/* ------------------------------------------------------------------ */
const tickerText =
  "X3 X3 Chain  •  847K TPS  •  $2.8B TVL  •  103 Chains  •  99.99% Uptime  •  6s Finality  •  Quantum-Ready  •  ";

/* ------------------------------------------------------------------ */
/*  Live stats (mock, auto-updating)                                   */
/* ------------------------------------------------------------------ */
const baseStats = [
  { label: "TPS", base: 847000, icon: Zap, color: "text-violet-400", suffix: "" },
  { label: "Uptime", base: 99.99, icon: ShieldCheck, color: "text-green-400", suffix: "%", fixed: 2 },
  { label: "Finality", base: 6, icon: Clock, color: "text-cyan-400", suffix: "s", fixed: 1 },
  { label: "Chains", base: 103, icon: Link2, color: "text-purple-400", suffix: "" },
];

/* ------------------------------------------------------------------ */
/*  Showcase cards                                                     */
/* ------------------------------------------------------------------ */
const showcaseCards = [
  {
    title: "Validator Globe",
    desc: "Global validator distribution with real-time consensus visualization",
    icon: Globe,
    gradient: "from-violet-600 to-purple-800",
    placeholder: true,
  },
  {
    title: "Holographic Architecture",
    desc: "Multi-layer execution engine with parallel VM orchestration",
    icon: Layers,
    gradient: "from-purple-600 to-indigo-800",
  },
  {
    title: "Orderbook Depth",
    desc: "Atomic cross-chain orderbook with sub-second settlement",
    icon: BarChart3,
    gradient: "from-indigo-600 to-blue-800",
  },
  {
    title: "Neural Processing",
    desc: "AI-powered transaction routing and MEV protection layer",
    icon: Brain,
    gradient: "from-fuchsia-600 to-pink-800",
  },
];

/* ------------------------------------------------------------------ */
/*  Animated stat component                                            */
/* ------------------------------------------------------------------ */
function LiveStat({
  label,
  base,
  icon: Icon,
  color,
  suffix,
  fixed,
}: {
  label: string;
  base: number;
  icon: React.ElementType;
  color: string;
  suffix: string;
  fixed?: number;
}) {
  const [value, setValue] = useState(base);
  useEffect(() => {
    const id = setInterval(() => {
      setValue(() => {
        const jitter = base > 1000 ? Math.floor(Math.random() * 200 - 100) : +(Math.random() * 0.02 - 0.01).toFixed(fixed ?? 0);
        return +(base + jitter).toFixed(fixed ?? 0);
      });
    }, 2000);
    return () => clearInterval(id);
  }, [base, fixed]);

  const display = fixed != null ? value.toFixed(fixed) : value.toLocaleString();

  return (
    <div className="bg-slate-800/50 border border-slate-700/50 rounded-2xl p-5 text-center space-y-2">
      <Icon className={`w-6 h-6 mx-auto ${color}`} />
      <div className="text-2xl md:text-3xl font-bold text-white">
        {display}
        <span className="text-sm text-slate-400 ml-0.5">{suffix}</span>
      </div>
      <div className="text-xs text-slate-400">{label}</div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  CardInView — scroll-triggered animation wrapper                    */
/* ------------------------------------------------------------------ */
function CardInView({ children, delay = 0 }: { children: React.ReactNode; delay?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { once: true, margin: "-40px" });
  return (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, y: 30 }}
      animate={inView ? { opacity: 1, y: 0 } : {}}
      transition={{ duration: 0.5, delay }}
    >
      {children}
    </motion.div>
  );
}

/* ================================================================== */
/*  QuantumPanel                                                       */
/* ================================================================== */
export default function QuantumPanel() {
  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-slate-900 via-[#0d0820] to-black text-white">
      {/* ---- Marquee ticker ---- */}
      <div className="relative overflow-hidden bg-violet-500/5 border-b border-violet-500/10 py-2">
        <div className="flex whitespace-nowrap animate-marquee">
          <span className="text-xs text-violet-400/70 font-mono tracking-wide">
            {tickerText.repeat(4)}
          </span>
        </div>
      </div>

      <div className="p-6 space-y-10">
        {/* ---- Hero ---- */}
        <div className="text-center space-y-4 pt-4">
          <motion.h1
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
            className="text-4xl md:text-5xl font-extrabold"
          >
            <span className="bg-gradient-to-r from-violet-400 via-purple-300 to-fuchsia-400 bg-clip-text text-transparent">
              Quantum-Ready
            </span>
            <br />
            <span className="text-white">Infrastructure</span>
          </motion.h1>
          <motion.p
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.3 }}
            className="text-slate-400 max-w-lg mx-auto text-sm"
          >
            Next-generation blockchain infrastructure built for the quantum era — 
            ultra-high throughput, instant finality, and cross-chain interoperability.
          </motion.p>
        </div>

        {/* ---- Live stats ---- */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {baseStats.map((s) => (
            <LiveStat key={s.label} {...s} />
          ))}
        </div>

        {/* ---- Showcase cards ---- */}
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
          {showcaseCards.map((card, i) => (
            <CardInView key={card.title} delay={i * 0.1}>
              <div
                className={`relative rounded-2xl overflow-hidden border border-slate-700/50 bg-gradient-to-br ${card.gradient} p-[1px]`}
              >
                <div className="bg-slate-900/95 rounded-2xl p-6 space-y-3 h-full">
                  <card.icon className="w-8 h-8 text-violet-400" />
                  <h3 className="text-lg font-bold text-white">{card.title}</h3>
                  <p className="text-sm text-slate-400 leading-relaxed">{card.desc}</p>
                  {card.placeholder && (
                    <div className="mt-3 h-32 rounded-xl bg-slate-800/60 border border-slate-700/30 flex items-center justify-center">
                      <Globe className="w-10 h-10 text-slate-700 animate-pulse" />
                    </div>
                  )}
                </div>
              </div>
            </CardInView>
          ))}
        </div>
      </div>

      {/* Marquee animation keyframes injected via style tag */}
      <style>{`
        @keyframes marquee {
          0% { transform: translateX(0); }
          100% { transform: translateX(-50%); }
        }
        .animate-marquee {
          animation: marquee 30s linear infinite;
        }
      `}</style>
    </div>
  );
}
