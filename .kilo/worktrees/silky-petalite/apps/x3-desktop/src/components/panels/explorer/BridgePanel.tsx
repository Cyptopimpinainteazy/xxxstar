import { useState } from "react";
import {
  ArrowDown,
  ChevronDown,
  Settings,
  RefreshCw,
  CheckCircle,
  Clock,
  Zap,
  Shield,
  Wallet,
  Gift,
  Star,
} from "lucide-react";

const chains = [
  { name: "Ethereum", symbol: "ETH", color: "bg-blue-500" },
  { name: "Polygon", symbol: "MATIC", color: "bg-purple-500" },
  { name: "Arbitrum", symbol: "ARB", color: "bg-blue-400" },
  { name: "Optimism", symbol: "OP", color: "bg-red-500" },
  { name: "Base", symbol: "BASE", color: "bg-blue-600" },
  { name: "BNB Chain", symbol: "BNB", color: "bg-yellow-500" },
  { name: "Avalanche", symbol: "AVAX", color: "bg-red-600" },
  { name: "Fantom", symbol: "FTM", color: "bg-blue-300" },
  { name: "X3 X3 Chain", symbol: "X3", color: "bg-orange-500" },
];

const tokens = [
  { symbol: "ETH", name: "Ethereum", balance: "2.4521" },
  { symbol: "USDC", name: "USD Coin", balance: "5,420.00" },
  { symbol: "USDT", name: "Tether", balance: "3,120.50" },
  { symbol: "WBTC", name: "Wrapped BTC", balance: "0.1842" },
  { symbol: "DAI", name: "Dai", balance: "1,847.25" },
];

const recentTxs = [
  {
    from: "Ethereum",
    to: "X3 X3 Chain",
    token: "ETH",
    amount: "1.5",
    status: "completed",
    time: "2 min ago",
    points: 150,
  },
  {
    from: "Polygon",
    to: "X3 X3 Chain",
    token: "USDC",
    amount: "2,500",
    status: "pending",
    time: "5 min ago",
    points: 250,
  },
  {
    from: "Arbitrum",
    to: "X3 X3 Chain",
    token: "WBTC",
    amount: "0.05",
    status: "completed",
    time: "12 min ago",
    points: 500,
  },
];

export default function BridgePanel() {
  const [sourceChainIdx, setSourceChainIdx] = useState(0);
  const [tokenIdx, setTokenIdx] = useState(0);
  const [amount, setAmount] = useState("");
  const [showChainDropdown, setShowChainDropdown] = useState(false);
  const [showTokenDropdown, setShowTokenDropdown] = useState(false);

  const sourceChain = chains[sourceChainIdx];
  const selectedToken = tokens[tokenIdx];

  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-[#0a0a0f] via-slate-900 to-black text-white p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Cross-Chain Bridge</h1>
          <p className="text-slate-400 text-sm">Bridge assets to X3 X3 Chain</p>
        </div>
        <div className="flex gap-2">
          <button className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 transition">
            <RefreshCw className="w-4 h-4 text-slate-400" />
          </button>
          <button className="p-2 rounded-lg bg-slate-800 hover:bg-slate-700 transition">
            <Settings className="w-4 h-4 text-slate-400" />
          </button>
        </div>
      </div>

      {/* Points Banner */}
      <div className="bg-gradient-to-r from-orange-500/20 to-yellow-500/20 border border-orange-500/30 rounded-xl p-4 flex items-center gap-3">
        <Gift className="w-6 h-6 text-orange-400 flex-shrink-0" />
        <div>
          <span className="font-semibold text-orange-300">Earn 2x Points on all bridges!</span>
          <span className="text-slate-300 text-sm ml-2">Limited time promotion</span>
        </div>
        <Star className="w-5 h-5 text-yellow-400 ml-auto flex-shrink-0" />
      </div>

      {/* Bridge Card */}
      <div className="max-w-lg mx-auto space-y-4">
        {/* Source */}
        <div className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-5 space-y-4">
          <div className="text-sm text-slate-400 font-medium">From</div>

          {/* Chain Selector */}
          <div className="relative">
            <button
              onClick={() => setShowChainDropdown(!showChainDropdown)}
              className="w-full flex items-center justify-between bg-slate-700/50 rounded-lg px-4 py-3 hover:bg-slate-700 transition"
            >
              <div className="flex items-center gap-2">
                <div className={`w-6 h-6 rounded-full ${sourceChain.color}`} />
                <span className="font-medium">{sourceChain.name}</span>
              </div>
              <ChevronDown className="w-4 h-4 text-slate-400" />
            </button>
            {showChainDropdown && (
              <div className="absolute z-20 top-full mt-1 left-0 right-0 bg-slate-800 border border-slate-700 rounded-lg shadow-xl max-h-60 overflow-y-auto">
                {chains.filter((_, i) => i !== chains.length - 1).map((c, i) => (
                  <button
                    key={c.name}
                    className="w-full flex items-center gap-2 px-4 py-2 hover:bg-slate-700 transition text-left"
                    onClick={() => { setSourceChainIdx(i); setShowChainDropdown(false); }}
                  >
                    <div className={`w-5 h-5 rounded-full ${c.color}`} />
                    <span>{c.name}</span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Token Selector + Amount */}
          <div className="flex gap-3">
            <div className="relative">
              <button
                onClick={() => setShowTokenDropdown(!showTokenDropdown)}
                className="flex items-center gap-2 bg-slate-700/50 rounded-lg px-4 py-3 hover:bg-slate-700 transition"
              >
                <span className="font-medium">{selectedToken.symbol}</span>
                <ChevronDown className="w-4 h-4 text-slate-400" />
              </button>
              {showTokenDropdown && (
                <div className="absolute z-20 top-full mt-1 left-0 bg-slate-800 border border-slate-700 rounded-lg shadow-xl w-48">
                  {tokens.map((t, i) => (
                    <button
                      key={t.symbol}
                      className="w-full flex items-center justify-between px-4 py-2 hover:bg-slate-700 transition text-left"
                      onClick={() => { setTokenIdx(i); setShowTokenDropdown(false); }}
                    >
                      <span>{t.symbol}</span>
                      <span className="text-xs text-slate-400">{t.balance}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
            <input
              type="text"
              placeholder="0.0"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              className="flex-1 bg-slate-700/50 rounded-lg px-4 py-3 text-right text-lg font-medium outline-none focus:ring-2 focus:ring-orange-500/50 transition"
            />
          </div>
          <div className="text-xs text-slate-500 text-right">
            Balance: {selectedToken.balance} {selectedToken.symbol}
          </div>
        </div>

        {/* Arrow */}
        <div className="flex justify-center -my-2 relative z-10">
          <div className="bg-slate-700 border border-slate-600 rounded-full p-2">
            <ArrowDown className="w-5 h-5 text-orange-400" />
          </div>
        </div>

        {/* Destination */}
        <div className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-5 space-y-3">
          <div className="text-sm text-slate-400 font-medium">To</div>
          <div className="flex items-center justify-between bg-slate-700/30 rounded-lg px-4 py-3">
            <div className="flex items-center gap-2">
              <div className="w-6 h-6 rounded-full bg-orange-500" />
              <span className="font-medium">X3 X3 Chain</span>
            </div>
            <span className="text-xs bg-green-500/20 text-green-400 border border-green-500/30 px-2 py-0.5 rounded-full font-medium">
              LIVE
            </span>
          </div>
        </div>

        {/* Estimates */}
        <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-4 space-y-2 text-sm">
          <div className="flex justify-between text-slate-400">
            <span>Estimated Time</span>
            <span className="text-white">~2 minutes</span>
          </div>
          <div className="flex justify-between text-slate-400">
            <span>Gas Fee</span>
            <span className="text-white">~$1.24</span>
          </div>
          <div className="flex justify-between text-slate-400">
            <span>Points Earned</span>
            <span className="text-orange-400 font-medium">+{amount ? Math.floor(Number(amount.replace(/,/g, "")) * 100) || 0 : 0} pts (2x)</span>
          </div>
        </div>

        {/* Bridge Button */}
        <button className="w-full bg-gradient-to-r from-orange-500 to-orange-600 hover:from-orange-600 hover:to-orange-700 text-white font-bold py-4 rounded-xl transition flex items-center justify-center gap-2">
          <Wallet className="w-5 h-5" />
          Connect Wallet &amp; Bridge
        </button>

        {/* Features */}
        <div className="flex justify-center gap-6 text-xs text-slate-400">
          <div className="flex items-center gap-1">
            <Shield className="w-4 h-4 text-green-400" />
            Secure
          </div>
          <div className="flex items-center gap-1">
            <Zap className="w-4 h-4 text-yellow-400" />
            Fast
          </div>
          <div className="flex items-center gap-1">
            <Gift className="w-4 h-4 text-orange-400" />
            Earn Points
          </div>
        </div>
      </div>

      {/* Recent Transactions */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <h2 className="text-lg font-semibold mb-4">Recent Transactions</h2>
        <div className="space-y-3">
          {recentTxs.map((tx, i) => (
            <div key={i} className="flex items-center justify-between bg-slate-700/30 rounded-lg p-3">
              <div className="flex items-center gap-3">
                {tx.status === "completed" ? (
                  <CheckCircle className="w-5 h-5 text-green-400" />
                ) : (
                  <Clock className="w-5 h-5 text-yellow-400" />
                )}
                <div>
                  <div className="font-medium text-sm">
                    {tx.amount} {tx.token}
                  </div>
                  <div className="text-xs text-slate-400">
                    {tx.from} → {tx.to}
                  </div>
                </div>
              </div>
              <div className="text-right">
                <div className="text-xs text-orange-400">+{tx.points} pts</div>
                <div className="text-xs text-slate-500">{tx.time}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
