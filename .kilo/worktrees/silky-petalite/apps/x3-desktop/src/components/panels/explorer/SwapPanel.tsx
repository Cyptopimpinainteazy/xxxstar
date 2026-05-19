import { useState } from "react";
import {
  Shield,
  Globe,
  ArrowDown,
  ChevronDown,
  Clock,
  Layers,
  RefreshCw,
} from "lucide-react";

const featureCards = [
  {
    title: "6 Second Finality",
    desc: "Near-instant cross-chain settlement powered by the X3 Kernel.",
    icon: Clock,
    gradient: "from-cyan-600 to-blue-600",
  },
  {
    title: "Zero Risk",
    desc: "Atomic execution guarantees — either all legs succeed or none do.",
    icon: Shield,
    gradient: "from-emerald-600 to-green-600",
  },
  {
    title: "103 Chains",
    desc: "Swap between 103 supported chains with unified liquidity routing.",
    icon: Globe,
    gradient: "from-purple-600 to-pink-600",
  },
];

const howItWorks = [
  { step: 1, title: "Select", desc: "Choose source and destination tokens across any chain." },
  { step: 2, title: "Route", desc: "AI finds the optimal path across all available liquidity." },
  { step: 3, title: "Comit Bundle", desc: "Transactions are bundled into an atomic commitment." },
  { step: 4, title: "Atomic Execution", desc: "All-or-nothing execution with guaranteed settlement." },
];

const supportedChains = [
  { name: "Ethereum", emoji: "🔷" },
  { name: "Arbitrum", emoji: "🔵" },
  { name: "Polygon", emoji: "🟣" },
  { name: "Optimism", emoji: "🔴" },
  { name: "Base", emoji: "🔹" },
  { name: "BNB Chain", emoji: "🟡" },
  { name: "Avalanche", emoji: "🔺" },
  { name: "Fantom", emoji: "👻" },
  { name: "Solana", emoji: "☀️" },
  { name: "Cosmos", emoji: "⚛️" },
  { name: "Sui", emoji: "💧" },
  { name: "Aptos", emoji: "🅰️" },
  { name: "Near", emoji: "🌈" },
  { name: "zkSync", emoji: "🔐" },
  { name: "Scroll", emoji: "📜" },
  { name: "X3 X3", emoji: "🌐" },
];

const tokens = [
  { symbol: "ETH", name: "Ethereum" },
  { symbol: "USDC", name: "USD Coin" },
  { symbol: "USDT", name: "Tether" },
  { symbol: "WBTC", name: "Wrapped BTC" },
  { symbol: "DAI", name: "Dai" },
  { symbol: "X3", name: "X3 Token" },
];

export default function SwapPanel() {
  const [fromToken, setFromToken] = useState("ETH");
  const [toToken, setToToken] = useState("USDC");
  const [amount, setAmount] = useState("");
  const [showFromDropdown, setShowFromDropdown] = useState(false);
  const [showToDropdown, setShowToDropdown] = useState(false);

  const switchTokens = () => {
    setFromToken(toToken);
    setToToken(fromToken);
  };

  return (
    <div className="overflow-y-auto h-full bg-slate-900 text-white p-6 space-y-6">
      {/* Hero */}
      <div className="text-center">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-cyan-400 to-blue-400 bg-clip-text text-transparent">
          Atomic Cross-Chain Swaps
        </h1>
        <p className="text-slate-400 mt-2 max-w-lg mx-auto">
          Swap any token across 103 chains with 6-second finality and zero counterparty risk.
        </p>
        <div className="flex items-center justify-center gap-4 mt-3">
          <span className="flex items-center gap-1 text-sm text-cyan-400">
            <Clock className="w-4 h-4" /> 6s finality
          </span>
          <span className="flex items-center gap-1 text-sm text-emerald-400">
            <Shield className="w-4 h-4" /> Zero risk
          </span>
          <span className="flex items-center gap-1 text-sm text-purple-400">
            <Globe className="w-4 h-4" /> 103 chains
          </span>
        </div>
      </div>

      {/* Feature Cards */}
      <div className="grid grid-cols-3 gap-4">
        {featureCards.map((f) => (
          <div
            key={f.title}
            className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 hover:border-cyan-500/30 transition-colors"
          >
            <div
              className={`w-10 h-10 rounded-lg bg-gradient-to-br ${f.gradient} flex items-center justify-center mb-3`}
            >
              <f.icon className="w-5 h-5 text-white" />
            </div>
            <h3 className="font-semibold text-sm">{f.title}</h3>
            <p className="text-xs text-slate-400 mt-1">{f.desc}</p>
          </div>
        ))}
      </div>

      {/* Swap Interface */}
      <div className="max-w-md mx-auto bg-slate-800/60 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center justify-between mb-4">
          <h2 className="font-semibold">Swap</h2>
          <button className="p-1.5 rounded-lg hover:bg-slate-700/50 text-slate-400 hover:text-white transition-colors">
            <RefreshCw className="w-4 h-4" />
          </button>
        </div>

        {/* From */}
        <div className="bg-slate-900/80 rounded-lg p-3 mb-1">
          <div className="text-xs text-slate-400 mb-2">From</div>
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.0"
              className="flex-1 bg-transparent text-lg font-mono focus:outline-none"
            />
            <div className="relative">
              <button
                onClick={() => { setShowFromDropdown(!showFromDropdown); setShowToDropdown(false); }}
                className="flex items-center gap-1 px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 transition-colors text-sm font-semibold"
              >
                {fromToken}
                <ChevronDown className="w-3.5 h-3.5 text-slate-400" />
              </button>
              {showFromDropdown && (
                <div className="absolute right-0 top-full mt-1 w-36 bg-slate-800 border border-slate-700 rounded-lg shadow-xl z-10 py-1">
                  {tokens.map((t) => (
                    <button
                      key={t.symbol}
                      onClick={() => { setFromToken(t.symbol); setShowFromDropdown(false); }}
                      className="w-full text-left px-3 py-2 text-sm hover:bg-slate-700 transition-colors"
                    >
                      {t.symbol} <span className="text-slate-500 text-xs">{t.name}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Switch button */}
        <div className="flex justify-center -my-2 relative z-10">
          <button
            onClick={switchTokens}
            className="w-8 h-8 rounded-full bg-slate-700 border-2 border-slate-900 flex items-center justify-center hover:bg-cyan-600 transition-colors"
          >
            <ArrowDown className="w-4 h-4" />
          </button>
        </div>

        {/* To */}
        <div className="bg-slate-900/80 rounded-lg p-3 mt-1">
          <div className="text-xs text-slate-400 mb-2">To</div>
          <div className="flex items-center gap-2">
            <div className="flex-1 text-lg font-mono text-slate-500">
              {amount ? "≈ calculating…" : "0.0"}
            </div>
            <div className="relative">
              <button
                onClick={() => { setShowToDropdown(!showToDropdown); setShowFromDropdown(false); }}
                className="flex items-center gap-1 px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 transition-colors text-sm font-semibold"
              >
                {toToken}
                <ChevronDown className="w-3.5 h-3.5 text-slate-400" />
              </button>
              {showToDropdown && (
                <div className="absolute right-0 top-full mt-1 w-36 bg-slate-800 border border-slate-700 rounded-lg shadow-xl z-10 py-1">
                  {tokens.map((t) => (
                    <button
                      key={t.symbol}
                      onClick={() => { setToToken(t.symbol); setShowToDropdown(false); }}
                      className="w-full text-left px-3 py-2 text-sm hover:bg-slate-700 transition-colors"
                    >
                      {t.symbol} <span className="text-slate-500 text-xs">{t.name}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        <button className="w-full mt-4 py-3 rounded-lg bg-gradient-to-r from-cyan-600 to-blue-600 text-white font-semibold hover:from-cyan-500 hover:to-blue-500 transition-all">
          Swap
        </button>
      </div>

      {/* How It Works */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <Layers className="w-5 h-5 text-cyan-400" />
          <h2 className="text-lg font-semibold">How It Works</h2>
        </div>
        <div className="flex items-start gap-4">
          {howItWorks.map((step, i) => (
            <div key={step.step} className="flex-1 relative">
              <div className="flex items-center gap-2 mb-2">
                <div className="w-7 h-7 rounded-full bg-cyan-600/20 text-cyan-400 flex items-center justify-center text-xs font-bold">
                  {step.step}
                </div>
                <span className="font-semibold text-sm">{step.title}</span>
              </div>
              <p className="text-xs text-slate-400 pl-9">{step.desc}</p>
              {i < howItWorks.length - 1 && (
                <div className="absolute top-3.5 left-full w-full h-px bg-slate-700/50 -translate-x-1/2" />
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Supported Chains Grid */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <Globe className="w-5 h-5 text-purple-400" />
          <h2 className="text-lg font-semibold">Supported Chains</h2>
          <span className="ml-auto text-xs text-slate-400">{supportedChains.length} of 103</span>
        </div>
        <div className="grid grid-cols-4 gap-2">
          {supportedChains.map((chain) => (
            <div
              key={chain.name}
              className="flex items-center gap-2 px-3 py-2 rounded-lg bg-slate-900/50 hover:bg-slate-800/50 transition-colors text-sm"
            >
              <span>{chain.emoji}</span>
              <span>{chain.name}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
