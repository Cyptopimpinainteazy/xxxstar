'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';

// Mock prediction markets data
const MOCK_MARKETS = [
  {
    id: 1,
    question: 'Will ETH be above $5,000 by March 2025?',
    type: 'PRICE',
    status: 'ACTIVE',
    yesPrice: 0.67,
    noPrice: 0.33,
    totalLiquidity: '234,567 ATLAS',
    volume24h: '12,345 ATLAS',
    resolutionTime: '2025-03-31',
    aiConsensus: { yes: 8, no: 3, avgConfidence: 78 },
  },
  {
    id: 2,
    question: 'Will Uniswap V4 TVL exceed $10B in Q1 2025?',
    type: 'TVL',
    status: 'ACTIVE',
    yesPrice: 0.45,
    noPrice: 0.55,
    totalLiquidity: '156,234 ATLAS',
    volume24h: '8,901 ATLAS',
    resolutionTime: '2025-03-31',
    aiConsensus: { yes: 5, no: 6, avgConfidence: 65 },
  },
  {
    id: 3,
    question: 'Will Aave lending rates average > 8% APY?',
    type: 'YIELD',
    status: 'ACTIVE',
    yesPrice: 0.38,
    noPrice: 0.62,
    totalLiquidity: '89,012 ATLAS',
    volume24h: '5,678 ATLAS',
    resolutionTime: '2025-02-28',
    aiConsensus: { yes: 4, no: 7, avgConfidence: 72 },
  },
  {
    id: 4,
    question: 'Will Bitcoin ETF daily volume exceed $5B?',
    type: 'CUSTOM',
    status: 'ACTIVE',
    yesPrice: 0.82,
    noPrice: 0.18,
    totalLiquidity: '456,789 ATLAS',
    volume24h: '34,567 ATLAS',
    resolutionTime: '2025-01-31',
    aiConsensus: { yes: 12, no: 2, avgConfidence: 91 },
  },
  {
    id: 5,
    question: 'Will Arbitrum TPS exceed 1000 sustained?',
    type: 'CUSTOM',
    status: 'ACTIVE',
    yesPrice: 0.56,
    noPrice: 0.44,
    totalLiquidity: '123,456 ATLAS',
    volume24h: '7,890 ATLAS',
    resolutionTime: '2025-06-30',
    aiConsensus: { yes: 6, no: 5, avgConfidence: 68 },
  },
];

// Mock AI signals
const MOCK_AI_SIGNALS = [
  { agent: 'agent-pred-001', market: 1, prediction: true, confidence: 85, reasoning: 'ETH supply dynamics post-merge favor price appreciation', timestamp: '2m ago' },
  { agent: 'agent-pred-002', market: 1, prediction: true, confidence: 72, reasoning: 'Institutional inflows increasing, spot ETF approval likely', timestamp: '5m ago' },
  { agent: 'agent-pred-003', market: 4, prediction: true, confidence: 94, reasoning: 'Current volume trajectory suggests $5B easily achievable', timestamp: '8m ago' },
  { agent: 'agent-pred-004', market: 2, prediction: false, confidence: 68, reasoning: 'Competition from new DEXes may fragment liquidity', timestamp: '12m ago' },
  { agent: 'agent-pred-005', market: 3, prediction: false, confidence: 76, reasoning: 'Fed rate cuts will compress DeFi yields', timestamp: '15m ago' },
];

export default function PredictionMarkets() {
  const [markets, setMarkets] = useState(MOCK_MARKETS);
  const [signals, setSignals] = useState(MOCK_AI_SIGNALS);
  const [selectedMarket, setSelectedMarket] = useState<number | null>(null);
  const [tradeAmount, setTradeAmount] = useState('');
  const [tradeDirection, setTradeDirection] = useState<'YES' | 'NO'>('YES');

  // Simulate price updates
  useEffect(() => {
    const interval = setInterval(() => {
      setMarkets(prev => prev.map(m => ({
        ...m,
        yesPrice: Math.max(0.01, Math.min(0.99, m.yesPrice + (Math.random() - 0.5) * 0.02)),
        noPrice: Math.max(0.01, Math.min(0.99, m.noPrice + (Math.random() - 0.5) * 0.02)),
      })));
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const typeColors: Record<string, string> = {
    PRICE: 'text-green-400 bg-green-500/20 border-green-500/30',
    TVL: 'text-blue-400 bg-blue-500/20 border-blue-500/30',
    YIELD: 'text-purple-400 bg-purple-500/20 border-purple-500/30',
    GOVERNANCE: 'text-orange-400 bg-orange-500/20 border-orange-500/30',
    CUSTOM: 'text-cyan-400 bg-cyan-500/20 border-cyan-500/30',
  };

  const totalVolume = markets.reduce((acc, m) => acc + parseInt(m.volume24h.replace(/,/g, '')), 0);
  const totalLiquidity = markets.reduce((acc, m) => acc + parseInt(m.totalLiquidity.replace(/,/g, '')), 0);

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-yellow-950/10 to-slate-950">
      {/* Header */}
      <div className="border-b border-yellow-500/20 bg-black/40 backdrop-blur-xl sticky top-0 z-50">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Link href="/x3/swarm" className="text-gray-400 hover:text-white transition">
                ← Back to Swarm
              </Link>
              <span className="text-2xl">🔮</span>
              <h1 className="text-2xl font-bold bg-gradient-to-r from-yellow-400 to-orange-400 bg-clip-text text-transparent">
                Prediction Markets
              </h1>
            </div>
            <button className="px-4 py-2 bg-gradient-to-r from-yellow-600 to-orange-600 text-white rounded-lg font-mono text-sm hover:opacity-90 transition">
              Create Market
            </button>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="container mx-auto px-6 py-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-black/60 border border-yellow-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Active Markets</div>
            <div className="text-2xl font-bold text-yellow-400 font-mono">{markets.length}</div>
          </div>
          <div className="bg-black/60 border border-yellow-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">24h Volume</div>
            <div className="text-2xl font-bold text-white font-mono">{totalVolume.toLocaleString()} ATLAS</div>
          </div>
          <div className="bg-black/60 border border-yellow-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">Total Liquidity</div>
            <div className="text-2xl font-bold text-cyan-400 font-mono">{totalLiquidity.toLocaleString()} ATLAS</div>
          </div>
          <div className="bg-black/60 border border-yellow-500/20 rounded-xl p-4">
            <div className="text-xs text-gray-500 font-mono uppercase">AI Signals Today</div>
            <div className="text-2xl font-bold text-purple-400 font-mono">{signals.length}</div>
          </div>
        </div>
      </div>

      <div className="container mx-auto px-6 pb-12">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Markets List */}
          <div className="lg:col-span-2 space-y-4">
            <h2 className="text-xl font-bold text-white font-mono mb-4">📊 Active Markets</h2>
            {markets.map(market => (
              <div 
                key={market.id}
                className={`p-6 bg-black/60 border rounded-xl cursor-pointer transition-all ${
                  selectedMarket === market.id 
                    ? 'border-yellow-500/50 ring-2 ring-yellow-500/20' 
                    : 'border-gray-800/50 hover:border-yellow-500/30'
                }`}
                onClick={() => setSelectedMarket(market.id)}
              >
                <div className="flex items-start justify-between mb-4">
                  <div className="flex-1">
                    <span className={`px-3 py-1 rounded-full text-xs font-mono border ${typeColors[market.type]}`}>
                      {market.type}
                    </span>
                    <h3 className="text-lg font-semibold text-white mt-2">{market.question}</h3>
                  </div>
                  <div className="text-right">
                    <div className="text-xs text-gray-500 font-mono">Resolves</div>
                    <div className="text-sm text-gray-300">{market.resolutionTime}</div>
                  </div>
                </div>

                {/* Price Bars */}
                <div className="mb-4">
                  <div className="flex items-center gap-4 mb-2">
                    <div className="flex-1">
                      <div className="flex justify-between text-sm mb-1">
                        <span className="text-green-400 font-mono">YES</span>
                        <span className="text-green-400 font-mono">{(market.yesPrice * 100).toFixed(1)}%</span>
                      </div>
                      <div className="h-3 bg-gray-800 rounded-full overflow-hidden">
                        <div 
                          className="h-full bg-gradient-to-r from-green-600 to-green-400 rounded-full transition-all"
                          style={{ width: `${market.yesPrice * 100}%` }}
                        />
                      </div>
                    </div>
                    <div className="flex-1">
                      <div className="flex justify-between text-sm mb-1">
                        <span className="text-red-400 font-mono">NO</span>
                        <span className="text-red-400 font-mono">{(market.noPrice * 100).toFixed(1)}%</span>
                      </div>
                      <div className="h-3 bg-gray-800 rounded-full overflow-hidden">
                        <div 
                          className="h-full bg-gradient-to-r from-red-600 to-red-400 rounded-full transition-all"
                          style={{ width: `${market.noPrice * 100}%` }}
                        />
                      </div>
                    </div>
                  </div>
                </div>

                {/* Stats Row */}
                <div className="flex items-center justify-between text-sm">
                  <div className="flex items-center gap-4">
                    <span className="text-gray-500">
                      💧 <span className="text-white">{market.totalLiquidity}</span>
                    </span>
                    <span className="text-gray-500">
                      📈 <span className="text-cyan-400">{market.volume24h}</span> 24h
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-500 text-xs">AI Consensus:</span>
                    <span className={market.aiConsensus.yes > market.aiConsensus.no ? 'text-green-400' : 'text-red-400'}>
                      {market.aiConsensus.yes > market.aiConsensus.no ? 'YES' : 'NO'} ({market.aiConsensus.avgConfidence}%)
                    </span>
                    <span className="text-gray-600 text-xs">
                      ({market.aiConsensus.yes}:{market.aiConsensus.no})
                    </span>
                  </div>
                </div>

                {/* Trade Panel (expanded) */}
                {selectedMarket === market.id && (
                  <div className="mt-6 pt-6 border-t border-gray-800">
                    <div className="grid grid-cols-2 gap-4 mb-4">
                      <button
                        onClick={(e) => { e.stopPropagation(); setTradeDirection('YES'); }}
                        className={`p-4 rounded-xl font-mono text-sm transition ${
                          tradeDirection === 'YES'
                            ? 'bg-green-500/30 text-green-400 border border-green-500/50'
                            : 'bg-black/40 text-gray-500 border border-gray-700 hover:border-green-500/30'
                        }`}
                      >
                        Buy YES @ {(market.yesPrice * 100).toFixed(1)}%
                      </button>
                      <button
                        onClick={(e) => { e.stopPropagation(); setTradeDirection('NO'); }}
                        className={`p-4 rounded-xl font-mono text-sm transition ${
                          tradeDirection === 'NO'
                            ? 'bg-red-500/30 text-red-400 border border-red-500/50'
                            : 'bg-black/40 text-gray-500 border border-gray-700 hover:border-red-500/30'
                        }`}
                      >
                        Buy NO @ {(market.noPrice * 100).toFixed(1)}%
                      </button>
                    </div>
                    <div className="flex items-center gap-4">
                      <input
                        type="number"
                        value={tradeAmount}
                        onChange={(e) => setTradeAmount(e.target.value)}
                        placeholder="Amount in ATLAS"
                        className="flex-1 bg-black/40 border border-gray-700 rounded-lg px-4 py-3 text-white font-mono focus:border-yellow-500/50 outline-none"
                        onClick={(e) => e.stopPropagation()}
                      />
                      <button className="px-6 py-3 bg-gradient-to-r from-yellow-600 to-orange-600 text-white rounded-lg font-mono hover:opacity-90 transition">
                        Place Trade
                      </button>
                    </div>
                    {tradeAmount && (
                      <div className="mt-3 text-sm text-gray-400 font-mono">
                        Estimated payout: {((parseFloat(tradeAmount) / (tradeDirection === 'YES' ? market.yesPrice : market.noPrice))).toFixed(2)} ATLAS if {tradeDirection} wins
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>

          {/* AI Signals Panel */}
          <div className="space-y-4">
            <h2 className="text-xl font-bold text-white font-mono mb-4">🤖 AI Signal Feed</h2>
            <div className="bg-black/60 border border-yellow-500/20 rounded-xl p-4">
              <div className="space-y-4">
                {signals.map((signal, idx) => (
                  <div key={idx} className="p-4 bg-black/40 rounded-lg border border-gray-800">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-xs text-gray-500 font-mono">{signal.agent}</span>
                      <span className="text-xs text-gray-600">{signal.timestamp}</span>
                    </div>
                    <div className="flex items-center gap-2 mb-2">
                      <span className={`px-2 py-1 rounded text-xs font-mono ${
                        signal.prediction ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'
                      }`}>
                        {signal.prediction ? 'YES' : 'NO'}
                      </span>
                      <span className="text-white text-sm">Market #{signal.market}</span>
                      <span className="text-cyan-400 text-xs font-mono">{signal.confidence}% conf</span>
                    </div>
                    <p className="text-gray-400 text-sm">{signal.reasoning}</p>
                  </div>
                ))}
              </div>
            </div>

            {/* Consensus Summary */}
            <div className="bg-black/60 border border-purple-500/20 rounded-xl p-4">
              <h3 className="text-sm font-bold text-purple-400 font-mono mb-4">📊 AI Consensus Summary</h3>
              <div className="space-y-3">
                {markets.slice(0, 3).map(market => (
                  <div key={market.id} className="flex items-center justify-between">
                    <span className="text-gray-400 text-xs truncate max-w-[150px]">
                      {market.question.slice(0, 30)}...
                    </span>
                    <div className="flex items-center gap-2">
                      <div className="w-16 h-2 bg-gray-800 rounded-full overflow-hidden">
                        <div 
                          className="h-full bg-gradient-to-r from-green-500 to-cyan-500 rounded-full"
                          style={{ width: `${(market.aiConsensus.yes / (market.aiConsensus.yes + market.aiConsensus.no)) * 100}%` }}
                        />
                      </div>
                      <span className="text-gray-500 text-xs font-mono">
                        {market.aiConsensus.yes}:{market.aiConsensus.no}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
