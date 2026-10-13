'use client';

import { useState, useEffect } from 'react';
import { ArrowDownUp, Zap, Clock, DollarSign, ChevronDown, Loader2, CheckCircle2, AlertCircle } from 'lucide-react';

// Chain data - matches our 103 chain registry
const CHAINS = [
  { id: 1, name: 'Ethereum', symbol: 'ETH', logo: '⟠', color: '#627EEA' },
  { id: 10, name: 'Optimism', symbol: 'ETH', logo: '🔴', color: '#FF0420' },
  { id: 25, name: 'Cronos', symbol: 'CRO', logo: '🔷', color: '#002D74' },
  { id: 42, name: 'Atlas Sphere', symbol: 'ATLAS', logo: '🌐', color: '#00D4FF' },
  { id: 56, name: 'BNB Chain', symbol: 'BNB', logo: '🟡', color: '#F0B90B' },
  { id: 137, name: 'Polygon', symbol: 'MATIC', logo: '🟣', color: '#8247E5' },
  { id: 250, name: 'Fantom', symbol: 'FTM', logo: '👻', color: '#1969FF' },
  { id: 324, name: 'zkSync Era', symbol: 'ETH', logo: '⚡', color: '#8C8DFC' },
  { id: 8217, name: 'Klaytn', symbol: 'KLAY', logo: '🔶', color: '#FF3D00' },
  { id: 8453, name: 'Base', symbol: 'ETH', logo: '🔵', color: '#0052FF' },
  { id: 42161, name: 'Arbitrum', symbol: 'ETH', logo: '🔷', color: '#28A0F0' },
  { id: 43114, name: 'Avalanche', symbol: 'AVAX', logo: '🔺', color: '#E84142' },
  { id: 534352, name: 'Scroll', symbol: 'ETH', logo: '📜', color: '#FFEBCD' },
];

// Popular tokens
const TOKENS = [
  { symbol: 'USDC', name: 'USD Coin', address: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48', decimals: 6, logo: '💵' },
  { symbol: 'USDT', name: 'Tether', address: '0xdAC17F958D2ee523a2206206994597C13D831ec7', decimals: 6, logo: '💴' },
  { symbol: 'WETH', name: 'Wrapped ETH', address: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2', decimals: 18, logo: '⟠' },
  { symbol: 'WBTC', name: 'Wrapped BTC', address: '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599', decimals: 8, logo: '₿' },
  { symbol: 'DAI', name: 'Dai', address: '0x6B175474E89094C44Da98b954EesdeREAL123456', decimals: 18, logo: '◈' },
];

interface Quote {
  inputAmount: string;
  outputAmount: string;
  priceImpact: number;
  estimatedGas: number;
  estimatedTimeSecs: number;
  route: {
    legs: { fromChain: number; toChain: number; action: string }[];
  };
}

type SwapStatus = 'idle' | 'quoting' | 'quoted' | 'confirming' | 'pending' | 'success' | 'error';

export default function SwapInterface() {
  const [fromChain, setFromChain] = useState(CHAINS[5]); // Polygon
  const [toChain, setToChain] = useState(CHAINS[10]); // Arbitrum
  const [fromToken, setFromToken] = useState(TOKENS[0]); // USDC
  const [toToken, setToToken] = useState(TOKENS[2]); // WETH
  const [amount, setAmount] = useState('100');
  const [quote, setQuote] = useState<Quote | null>(null);
  const [status, setStatus] = useState<SwapStatus>('idle');
  const [showFromChains, setShowFromChains] = useState(false);
  const [showToChains, setShowToChains] = useState(false);
  const [showFromTokens, setShowFromTokens] = useState(false);
  const [showToTokens, setShowToTokens] = useState(false);

  // Fetch quote when inputs change
  useEffect(() => {
    if (!amount || parseFloat(amount) <= 0) {
      setQuote(null);
      return;
    }

    const timer = setTimeout(() => {
      fetchQuote();
    }, 500);

    return () => clearTimeout(timer);
  }, [fromChain, toChain, fromToken, toToken, amount]);

  const fetchQuote = async () => {
    setStatus('quoting');
    
    // Simulate API call - in production this would call the actual router
    await new Promise(resolve => setTimeout(resolve, 800));
    
    const inputNum = parseFloat(amount);
    const outputNum = inputNum * 0.995 * (fromToken.symbol === toToken.symbol ? 1 : 0.35); // Mock conversion
    
    setQuote({
      inputAmount: amount,
      outputAmount: outputNum.toFixed(6),
      priceImpact: 0.5,
      estimatedGas: 50000,
      estimatedTimeSecs: 6,
      route: {
        legs: fromChain.id === toChain.id 
          ? [{ fromChain: fromChain.id, toChain: toChain.id, action: 'swap' }]
          : [{ fromChain: fromChain.id, toChain: toChain.id, action: 'bridge' }]
      }
    });
    setStatus('quoted');
  };

  const handleSwap = async () => {
    setStatus('confirming');
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    setStatus('pending');
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    setStatus('success');
  };

  const swapChains = () => {
    const temp = fromChain;
    setFromChain(toChain);
    setToChain(temp);
  };

  return (
    <div className="w-full max-w-md mx-auto">
      <div className="bg-gradient-to-br from-slate-900 to-slate-800 rounded-2xl p-6 shadow-2xl border border-slate-700">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-bold text-white flex items-center gap-2">
            <Zap className="w-5 h-5 text-cyan-400" />
            Atomic Swap
          </h2>
          <div className="text-xs text-slate-400 bg-slate-800 px-2 py-1 rounded">
            103 Chains
          </div>
        </div>

        {/* From Section */}
        <div className="bg-slate-800/50 rounded-xl p-4 mb-2">
          <div className="flex justify-between text-sm text-slate-400 mb-2">
            <span>From</span>
            <span>Balance: 1,234.56</span>
          </div>
          
          <div className="flex items-center gap-3">
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.0"
              className="flex-1 bg-transparent text-2xl text-white outline-none"
            />
            
            <div className="relative">
              <button
                onClick={() => setShowFromTokens(!showFromTokens)}
                className="flex items-center gap-2 bg-slate-700 hover:bg-slate-600 px-3 py-2 rounded-lg transition"
              >
                <span className="text-lg">{fromToken.logo}</span>
                <span className="text-white font-medium">{fromToken.symbol}</span>
                <ChevronDown className="w-4 h-4 text-slate-400" />
              </button>
              
              {showFromTokens && (
                <div className="absolute right-0 top-full mt-2 bg-slate-700 rounded-lg shadow-xl z-10 min-w-[160px]">
                  {TOKENS.map(token => (
                    <button
                      key={token.symbol}
                      onClick={() => { setFromToken(token); setShowFromTokens(false); }}
                      className="flex items-center gap-2 w-full px-3 py-2 hover:bg-slate-600 text-white text-left"
                    >
                      <span>{token.logo}</span>
                      <span>{token.symbol}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
          
          {/* Chain Selector */}
          <div className="mt-3 relative">
            <button
              onClick={() => setShowFromChains(!showFromChains)}
              className="flex items-center gap-2 text-sm text-slate-300 hover:text-white transition"
            >
              <span>{fromChain.logo}</span>
              <span>{fromChain.name}</span>
              <ChevronDown className="w-3 h-3" />
            </button>
            
            {showFromChains && (
              <div className="absolute left-0 top-full mt-2 bg-slate-700 rounded-lg shadow-xl z-10 max-h-60 overflow-y-auto min-w-[180px]">
                {CHAINS.map(chain => (
                  <button
                    key={chain.id}
                    onClick={() => { setFromChain(chain); setShowFromChains(false); }}
                    className="flex items-center gap-2 w-full px-3 py-2 hover:bg-slate-600 text-white text-left"
                  >
                    <span>{chain.logo}</span>
                    <span>{chain.name}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Swap Button */}
        <div className="flex justify-center -my-1 relative z-10">
          <button
            onClick={swapChains}
            className="bg-slate-700 hover:bg-slate-600 p-2 rounded-lg border-4 border-slate-900 transition"
          >
            <ArrowDownUp className="w-5 h-5 text-cyan-400" />
          </button>
        </div>

        {/* To Section */}
        <div className="bg-slate-800/50 rounded-xl p-4 mt-2">
          <div className="flex justify-between text-sm text-slate-400 mb-2">
            <span>To</span>
            <span>Balance: 0.00</span>
          </div>
          
          <div className="flex items-center gap-3">
            <div className="flex-1 text-2xl text-white">
              {status === 'quoting' ? (
                <Loader2 className="w-6 h-6 animate-spin text-slate-500" />
              ) : quote ? (
                quote.outputAmount
              ) : (
                <span className="text-slate-500">0.0</span>
              )}
            </div>
            
            <div className="relative">
              <button
                onClick={() => setShowToTokens(!showToTokens)}
                className="flex items-center gap-2 bg-slate-700 hover:bg-slate-600 px-3 py-2 rounded-lg transition"
              >
                <span className="text-lg">{toToken.logo}</span>
                <span className="text-white font-medium">{toToken.symbol}</span>
                <ChevronDown className="w-4 h-4 text-slate-400" />
              </button>
              
              {showToTokens && (
                <div className="absolute right-0 top-full mt-2 bg-slate-700 rounded-lg shadow-xl z-10 min-w-[160px]">
                  {TOKENS.map(token => (
                    <button
                      key={token.symbol}
                      onClick={() => { setToToken(token); setShowToTokens(false); }}
                      className="flex items-center gap-2 w-full px-3 py-2 hover:bg-slate-600 text-white text-left"
                    >
                      <span>{token.logo}</span>
                      <span>{token.symbol}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
          
          {/* Chain Selector */}
          <div className="mt-3 relative">
            <button
              onClick={() => setShowToChains(!showToChains)}
              className="flex items-center gap-2 text-sm text-slate-300 hover:text-white transition"
            >
              <span>{toChain.logo}</span>
              <span>{toChain.name}</span>
              <ChevronDown className="w-3 h-3" />
            </button>
            
            {showToChains && (
              <div className="absolute left-0 top-full mt-2 bg-slate-700 rounded-lg shadow-xl z-10 max-h-60 overflow-y-auto min-w-[180px]">
                {CHAINS.map(chain => (
                  <button
                    key={chain.id}
                    onClick={() => { setToChain(chain); setShowToChains(false); }}
                    className="flex items-center gap-2 w-full px-3 py-2 hover:bg-slate-600 text-white text-left"
                  >
                    <span>{chain.logo}</span>
                    <span>{chain.name}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Quote Details */}
        {quote && status === 'quoted' && (
          <div className="mt-4 bg-slate-800/30 rounded-xl p-4 space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-slate-400">Route</span>
              <span className="text-white flex items-center gap-1">
                {fromChain.logo} → {toChain.logo}
                <span className="text-cyan-400 text-xs ml-1">
                  ({quote.route.legs[0].action})
                </span>
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-slate-400">Price Impact</span>
              <span className="text-green-400">{quote.priceImpact}%</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-slate-400 flex items-center gap-1">
                <Clock className="w-3 h-3" /> Time
              </span>
              <span className="text-cyan-400 font-medium">{quote.estimatedTimeSecs}s ⚡</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-slate-400 flex items-center gap-1">
                <DollarSign className="w-3 h-3" /> Fee
              </span>
              <span className="text-white">~$0.10</span>
            </div>
            
            {/* Speed comparison */}
            <div className="mt-3 pt-3 border-t border-slate-700">
              <div className="text-xs text-slate-500 mb-2">vs Traditional Bridge</div>
              <div className="flex justify-between text-xs">
                <span className="text-slate-400">Speed</span>
                <span>
                  <span className="text-red-400 line-through mr-2">15-45 min</span>
                  <span className="text-green-400 font-bold">6 sec</span>
                </span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-slate-400">Cost</span>
                <span>
                  <span className="text-red-400 line-through mr-2">$5-25</span>
                  <span className="text-green-400 font-bold">$0.10</span>
                </span>
              </div>
            </div>
          </div>
        )}

        {/* Action Button */}
        <button
          onClick={handleSwap}
          disabled={!quote || status === 'pending' || status === 'confirming'}
          className={`w-full mt-4 py-4 rounded-xl font-bold text-lg transition ${
            status === 'success'
              ? 'bg-green-500 text-white'
              : status === 'error'
              ? 'bg-red-500 text-white'
              : quote
              ? 'bg-gradient-to-r from-cyan-500 to-blue-500 hover:from-cyan-400 hover:to-blue-400 text-white'
              : 'bg-slate-700 text-slate-400 cursor-not-allowed'
          }`}
        >
          {status === 'quoting' && (
            <span className="flex items-center justify-center gap-2">
              <Loader2 className="w-5 h-5 animate-spin" />
              Getting Quote...
            </span>
          )}
          {status === 'confirming' && (
            <span className="flex items-center justify-center gap-2">
              <Loader2 className="w-5 h-5 animate-spin" />
              Confirm in Wallet...
            </span>
          )}
          {status === 'pending' && (
            <span className="flex items-center justify-center gap-2">
              <Loader2 className="w-5 h-5 animate-spin" />
              Executing Atomic Swap...
            </span>
          )}
          {status === 'success' && (
            <span className="flex items-center justify-center gap-2">
              <CheckCircle2 className="w-5 h-5" />
              Swap Complete!
            </span>
          )}
          {status === 'error' && (
            <span className="flex items-center justify-center gap-2">
              <AlertCircle className="w-5 h-5" />
              Swap Failed
            </span>
          )}
          {(status === 'idle' || status === 'quoted') && (
            quote ? 'Swap' : 'Enter Amount'
          )}
        </button>

        {/* Powered by */}
        <div className="mt-4 text-center text-xs text-slate-500">
          Powered by <span className="text-cyan-400">Atlas Kernel</span> Comit Transactions
        </div>
      </div>
    </div>
  );
}
