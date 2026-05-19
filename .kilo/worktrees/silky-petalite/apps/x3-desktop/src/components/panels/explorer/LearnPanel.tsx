import {
  BookOpen,
  Terminal,
  Wallet,
  Rocket,
  Clock,
  ExternalLink,
  FileText,
  Coins,
  ChefHat,
  GraduationCap,
  Layers,
  Atom,
  Database,
  Cpu,
  ArrowRight,
  CheckCircle2,
} from "lucide-react";

/* ------------------------------------------------------------------ */
/*  Onboarding steps                                                   */
/* ------------------------------------------------------------------ */
const steps = [
  {
    number: 1,
    title: "Set Up Environment",
    time: "10 min",
    icon: Terminal,
    color: "text-violet-400",
    bg: "bg-violet-500/10",
    border: "border-violet-500/30",
    description:
      "Install Rust, Node.js, and the X3 CLI. Clone the repository and configure your local development workspace.",
  },
  {
    number: 2,
    title: "Run a Node",
    time: "15 min",
    icon: Rocket,
    color: "text-purple-400",
    bg: "bg-purple-500/10",
    border: "border-purple-500/30",
    description:
      "Start a local X3 Chain node in development mode. Connect to the testnet and verify block production is working.",
  },
  {
    number: 3,
    title: "Connect Your Wallet",
    time: "5 min",
    icon: Wallet,
    color: "text-indigo-400",
    bg: "bg-indigo-500/10",
    border: "border-indigo-500/30",
    description:
      "Configure a Substrate-compatible wallet (e.g., Polkadot.js Extension) and fund it with testnet tokens from the faucet.",
  },
  {
    number: 4,
    title: "Deploy a Contract",
    time: "20 min",
    icon: FileText,
    color: "text-blue-400",
    bg: "bg-blue-500/10",
    border: "border-blue-500/30",
    description:
      "Write and deploy your first smart contract. Choose between EVM (Solidity), SVM (Rust), or native WASM targets.",
  },
];

/* ------------------------------------------------------------------ */
/*  Quick links                                                        */
/* ------------------------------------------------------------------ */
const quickLinks = [
  { label: "Documentation", icon: BookOpen, href: "#" },
  { label: "Tokenomics", icon: Coins, href: "#" },
  { label: "Cookbook", icon: ChefHat, href: "#" },
  { label: "Tutorials", icon: GraduationCap, href: "#" },
];

/* ------------------------------------------------------------------ */
/*  Learn cards                                                        */
/* ------------------------------------------------------------------ */
const learnCards = [
  {
    title: "Cross-VM Development",
    desc: "Build applications that span EVM, SVM, and native WASM execution environments seamlessly.",
    icon: Layers,
  },
  {
    title: "Atomic Transactions",
    desc: "Execute all-or-nothing operations across multiple virtual machines in a single block.",
    icon: Atom,
  },
  {
    title: "Canonical Ledger",
    desc: "Understand the unified state root model and deterministic finality guarantees.",
    icon: Database,
  },
  {
    title: "GPU Swarm Integration",
    desc: "Offload heavy computation to the distributed GPU swarm and receive verified results on-chain.",
    icon: Cpu,
  },
];

/* ================================================================== */
/*  LearnPanel                                                         */
/* ================================================================== */
export default function LearnPanel() {
  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-slate-900 via-[#0c0818] to-black text-white p-6 space-y-10">
      {/* ---- Header ---- */}
      <div className="space-y-2">
        <div className="flex items-center gap-3">
          <BookOpen className="w-8 h-8 text-purple-400" />
          <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-indigo-400 bg-clip-text text-transparent">
            Getting Started
          </h1>
        </div>
        <p className="text-slate-400 text-sm max-w-xl">
          Follow these four steps to set up your environment through deploying your first contract on X3 X3 Chain.
        </p>
      </div>

      {/* ---- Onboarding steps ---- */}
      <div className="space-y-4">
        {steps.map((step) => (
          <div
            key={step.number}
            className={`${step.bg} border ${step.border} rounded-2xl p-5 flex gap-5 items-start`}
          >
            {/* Step number */}
            <div className={`shrink-0 w-10 h-10 rounded-xl flex items-center justify-center font-bold text-lg ${step.color} bg-slate-800/80`}>
              {step.number}
            </div>
            {/* Content */}
            <div className="flex-1 space-y-1.5">
              <div className="flex items-center gap-3">
                <h3 className={`font-semibold text-lg ${step.color}`}>{step.title}</h3>
                <span className="inline-flex items-center gap-1 text-[10px] text-slate-400 bg-slate-800 px-2 py-0.5 rounded-full">
                  <Clock className="w-3 h-3" />
                  {step.time}
                </span>
              </div>
              <p className="text-sm text-slate-400 leading-relaxed">{step.description}</p>
            </div>
            {/* Icon */}
            <step.icon className={`w-6 h-6 ${step.color} shrink-0 mt-1 hidden sm:block`} />
          </div>
        ))}
      </div>

      {/* ---- Quick links ---- */}
      <div className="space-y-3">
        <h2 className="text-lg font-bold text-slate-200">Quick Links</h2>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
          {quickLinks.map((link) => (
            <a
              key={link.label}
              href={link.href}
              className="flex items-center gap-2.5 bg-slate-800/50 border border-slate-700/50 rounded-xl px-4 py-3 hover:border-purple-500/30 hover:bg-slate-800/80 transition group"
            >
              <link.icon className="w-4 h-4 text-purple-400" />
              <span className="text-sm text-slate-300 group-hover:text-white transition">{link.label}</span>
              <ExternalLink className="w-3 h-3 text-slate-600 ml-auto group-hover:text-purple-400 transition" />
            </a>
          ))}
        </div>
      </div>

      {/* ---- What you'll learn ---- */}
      <div className="space-y-4">
        <h2 className="text-lg font-bold text-slate-200 flex items-center gap-2">
          <CheckCircle2 className="w-5 h-5 text-indigo-400" />
          What You'll Learn
        </h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {learnCards.map((card) => (
            <div
              key={card.title}
              className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5 space-y-2 group hover:border-indigo-500/30 transition"
            >
              <card.icon className="w-6 h-6 text-indigo-400" />
              <h3 className="font-semibold text-slate-100">
                {card.title}
                <ArrowRight className="w-3.5 h-3.5 inline-block ml-1.5 text-slate-600 group-hover:text-indigo-400 transition" />
              </h3>
              <p className="text-sm text-slate-400 leading-relaxed">{card.desc}</p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
