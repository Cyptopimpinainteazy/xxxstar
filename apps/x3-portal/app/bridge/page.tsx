'use client';

import React, { useState } from 'react';
import Link from 'next/link';
import {
  ArrowDown,
  ArrowRight,
  ChevronDown,
  Settings,
  RefreshCw,
  AlertCircle,
  CheckCircle,
  Clock,
  Zap,
  Shield,
  Layers,
  Wallet,
  ExternalLink,
  Info,
  Star,
  Gift,
} from 'lucide-react';

// Supported chains
const chains = [
  { id: 1, name: 'Ethereum', symbol: 'ETH', icon: '⟠', color: 'from-blue-500 to-indigo-600' },
  { id: 137, name: 'Polygon', symbol: 'MATIC', icon: '⬡', color: 'from-purple-500 to-violet-600' },
  { id: 42161, name: 'Arbitrum', symbol: 'ARB', icon: '🔵', color: 'from-blue-400 to-cyan-500' },
  { id: 10, name: 'Optimism', symbol: 'OP', icon: '🔴', color: 'from-red-500 to-rose-600' },
  { id: 8453, name: 'Base', symbol: 'BASE', icon: '🔷', color: 'from-blue-600 to-blue-700' },
  { id: 56, name: 'BNB Chain', symbol: 'BNB', icon: '💛', color: 'from-yellow-500 to-amber-600' },
  { id: 43114, name: 'Avalanche', symbol: 'AVAX', icon: '🔺', color: 'from-red-600 to-rose-700' },
  { id: 250, name: 'Fantom', symbol: 'FTM', icon: '👻', color: 'from-blue-500 to-cyan-600' },
  { id: 0, name: 'X3 Atlas Sphere', symbol: 'X3', icon: '🔥', color: 'from-orange-500 to-red-600', isDestination: true },
];

// Tokens
const tokens = [
  { symbol: 'ETH', name: 'Ethereum', balance: '2.5', icon: '⟠' },
  { symbol: 'USDC', name: 'USD Coin', balance: '5,420.00', icon: '💵' },
  { symbol: 'USDT', name: 'Tether', balance: '1,200.00', icon: '💲' },
  { symbol: 'WBTC', name: 'Wrapped Bitcoin', balance: '0.15', icon: '₿' },
  { symbol: 'DAI', name: 'Dai', balance: '850.00', icon: '◈' },
];

// Recent transactions
const recentTransactions = [
  { id: '0x1234...5678', from: 'Ethereum', to: 'X3 Atlas', amount: '1.5 ETH', status: 'completed', time: '2 min ago' },
  { id: '0x2345...6789', from: 'Polygon', to: 'X3 Atlas', amount: '500 USDC', status: 'pending', time: '5 min ago' },
  { id: '0x3456...7890', from: 'Arbitrum', to: 'X3 Atlas', amount: '0.5 ETH', status: 'completed', time: '1 hour ago' },
];

export default function BridgePage() {
  const [sourceChain, setSourceChain] = useState(chains[0]);
  const [destChain, setDestChain] = useState(chains.find(c => c.isDestination) || chains[8]);
  const [selectedToken, setSelectedToken] = useState(tokens[0]);
  const [amount, setAmount] = useState('');
  const [showSourceChains, setShowSourceChains] = useState(false);
  const [showTokens, setShowTokens] = useState(false);
  const [isConnected, setIsConnected] = useState(false);

  const estimatedGas = '~$2.50';
  const estimatedTime = '~2-5 min';
  const pointsEarned = amount ? Math.floor(parseFloat(amount || '0') * 100) : 0;

  return (
    <div className="relative bg-black min-h-screen pt-24 pb-16">
      {/* Background effects */}
      <div className="absolute inset-0 bg-black" />
      <div className="absolute inset-0 mesh-gradient opacity-30" />
      <div className="absolute inset-0 grid-pattern opacity-20" />
      
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-orange-600/10 rounded-full blur-3xl animate-float" />
      <div className="absolute bottom-1/4 right-1/4 w-80 h-80 bg-red-600/10 rounded-full blur-3xl animate-float" style={{ animationDelay: '2s' }} />

      <div className="relative z-10 container-wide">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl md:text-4xl font-bold mb-3">
            <span className="text-white">Bridge to </span>
            <span className="gradient-text">X3 Atlas Sphere</span>
          </h1>
          <p className="text-gray-400 max-w-xl mx-auto">
            Bring your assets from any chain to X3 Atlas Sphere. Fast, secure, and earn points with every bridge.
          </p>
        </div>

        <div className="max-w-lg mx-auto">
          {/* Points Banner */}
          <div className="mb-6 p-3 rounded-xl bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/20 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Gift className="w-5 h-5 text-orange-400" />
              <span className="text-sm text-gray-300">
                Earn <span className="text-orange-400 font-semibold">2x Points</span> on all bridges today!
              </span>
            </div>
            <Link href="/earn" className="text-xs text-orange-400 hover:text-orange-300">
              Learn more
            </Link>
          </div>

          {/* Bridge Card */}
          <div className="glass-card p-6">
            {/* Settings */}
            <div className="flex items-center justify-between mb-6">
              <h2 className="font-semibold text-white">Bridge Assets</h2>
              <button className="p-2 rounded-lg hover:bg-[#1a1a1a] transition-colors">
                <Settings className="w-5 h-5 text-gray-400" />
              </button>
            </div>

            {/* From Section */}
            <div className="mb-2">
              <label className="text-sm text-gray-500 mb-2 block">From</label>
              <div className="p-4 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a]">
                {/* Chain Selector */}
                <div className="relative mb-3">
                  <button
                    onClick={() => setShowSourceChains(!showSourceChains)}
                    className="flex items-center gap-2 px-3 py-2 rounded-lg bg-[#1a1a1a] hover:bg-[#252525] transition-colors"
                  >
                    <span className="text-lg">{sourceChain.icon}</span>
                    <span className="text-white font-medium">{sourceChain.name}</span>
                    <ChevronDown className="w-4 h-4 text-gray-400" />
                  </button>
                  
                  {showSourceChains && (
                    <div className="absolute top-full left-0 mt-2 w-64 rounded-xl bg-[#111111] border border-[#1a1a1a] shadow-2xl z-20 max-h-60 overflow-y-auto">
                      {chains.filter(c => !c.isDestination).map((chain) => (
                        <button
                          key={chain.id}
                          onClick={() => {
                            setSourceChain(chain);
                            setShowSourceChains(false);
                          }}
                          className="w-full flex items-center gap-3 px-4 py-3 hover:bg-[#1a1a1a] transition-colors"
                        >
                          <span className="text-lg">{chain.icon}</span>
                          <span className="text-white">{chain.name}</span>
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                {/* Token and Amount */}
                <div className="flex items-center gap-3">
                  <div className="relative">
                    <button
                      onClick={() => setShowTokens(!showTokens)}
                      className="flex items-center gap-2 px-3 py-2 rounded-lg bg-[#1a1a1a] hover:bg-[#252525] transition-colors"
                    >
                      <span className="text-lg">{selectedToken.icon}</span>
                      <span className="text-white font-medium">{selectedToken.symbol}</span>
                      <ChevronDown className="w-4 h-4 text-gray-400" />
                    </button>
                    
                    {showTokens && (
                      <div className="absolute top-full left-0 mt-2 w-48 rounded-xl bg-[#111111] border border-[#1a1a1a] shadow-2xl z-20">
                        {tokens.map((token) => (
                          <button
                            key={token.symbol}
                            onClick={() => {
                              setSelectedToken(token);
                              setShowTokens(false);
                            }}
                            className="w-full flex items-center justify-between px-4 py-3 hover:bg-[#1a1a1a] transition-colors"
                          >
                            <div className="flex items-center gap-2">
                              <span>{token.icon}</span>
                              <span className="text-white">{token.symbol}</span>
                            </div>
                            <span className="text-xs text-gray-500">{token.balance}</span>
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                  
                  <input
                    type="number"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    placeholder="0.0"
                    className="flex-1 bg-transparent text-2xl text-white text-right outline-none placeholder-gray-600"
                  />
                </div>
                
                <div className="flex items-center justify-between mt-2 text-xs">
                  <span className="text-gray-600">Balance: {selectedToken.balance} {selectedToken.symbol}</span>
                  <button 
                    onClick={() => setAmount(selectedToken.balance.replace(',', ''))}
                    className="text-orange-400 hover:text-orange-300"
                  >
                    MAX
                  </button>
                </div>
              </div>
            </div>

            {/* Arrow */}
            <div className="flex justify-center -my-2 relative z-10">
              <div className="w-10 h-10 rounded-xl bg-[#1a1a1a] border border-[#252525] flex items-center justify-center">
                <ArrowDown className="w-5 h-5 text-orange-400" />
              </div>
            </div>

            {/* To Section */}
            <div className="mt-2 mb-6">
              <label className="text-sm text-gray-500 mb-2 block">To</label>
              <div className="p-4 rounded-xl bg-[#0a0a0a] border border-orange-500/30">
                <div className="flex items-center gap-3 mb-3">
                  <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gradient-to-r from-orange-500/20 to-red-500/20 border border-orange-500/30">
                    <span className="text-lg">🔥</span>
                    <span className="text-white font-medium">X3 Atlas Sphere</span>
                    <span className="px-1.5 py-0.5 text-[10px] bg-orange-500 text-white rounded font-bold">LIVE</span>
                  </div>
                </div>
                
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">You will receive</span>
                  <span className="text-2xl font-semibold text-white">
                    {amount || '0.0'} {selectedToken.symbol}
                  </span>
                </div>
              </div>
            </div>

            {/* Estimate Info */}
            {amount && (
              <div className="mb-6 p-4 rounded-xl bg-[#0a0a0a] border border-[#1a1a1a] space-y-3">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500 flex items-center gap-1">
                    <Clock className="w-4 h-4" />
                    Estimated time
                  </span>
                  <span className="text-white">{estimatedTime}</span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500 flex items-center gap-1">
                    <Zap className="w-4 h-4" />
                    Network fee
                  </span>
                  <span className="text-white">{estimatedGas}</span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-500 flex items-center gap-1">
                    <Gift className="w-4 h-4" />
                    Points earned
                  </span>
                  <span className="text-orange-400 font-semibold">+{pointsEarned * 2} pts (2x)</span>
                </div>
              </div>
            )}

            {/* Action Button */}
            {isConnected ? (
              <button
                disabled={!amount}
                className={`w-full py-4 rounded-xl font-semibold text-white transition-all ${
                  amount
                    ? 'bg-gradient-to-r from-orange-500 to-red-500 hover:from-orange-400 hover:to-red-400 shadow-lg shadow-orange-500/25'
                    : 'bg-[#1a1a1a] text-gray-500 cursor-not-allowed'
                }`}
              >
                {amount ? 'Bridge Assets' : 'Enter an amount'}
              </button>
            ) : (
              <button
                onClick={() => setIsConnected(true)}
                className="w-full py-4 rounded-xl font-semibold text-white bg-gradient-to-r from-orange-500 to-red-500 hover:from-orange-400 hover:to-red-400 shadow-lg shadow-orange-500/25 flex items-center justify-center gap-2"
              >
                <Wallet className="w-5 h-5" />
                Connect Wallet
              </button>
            )}
          </div>

          {/* Features */}
          <div className="mt-6 grid grid-cols-3 gap-3">
            {[
              { icon: <Shield className="w-5 h-5" />, label: 'Secure' },
              { icon: <Zap className="w-5 h-5" />, label: 'Fast' },
              { icon: <Star className="w-5 h-5" />, label: 'Earn Points' },
            ].map((feature, i) => (
              <div key={i} className="p-3 rounded-xl bg-[#111111] border border-[#1a1a1a] text-center">
                <span className="text-orange-400 flex justify-center mb-1">{feature.icon}</span>
                <span className="text-xs text-gray-400">{feature.label}</span>
              </div>
            ))}
          </div>

          {/* Recent Transactions */}
          {isConnected && (
            <div className="mt-8">
              <h3 className="font-semibold text-white mb-4">Recent Transactions</h3>
              <div className="space-y-3">
                {recentTransactions.map((tx) => (
                  <div key={tx.id} className="p-4 rounded-xl bg-[#111111] border border-[#1a1a1a] flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className={`w-8 h-8 rounded-lg flex items-center justify-center ${
                        tx.status === 'completed' ? 'bg-emerald-500/20' : 'bg-amber-500/20'
                      }`}>
                        {tx.status === 'completed' ? (
                          <CheckCircle className="w-4 h-4 text-emerald-400" />
                        ) : (
                          <RefreshCw className="w-4 h-4 text-amber-400 animate-spin" />
                        )}
                      </div>
                      <div>
                        <p className="text-sm text-white">{tx.from} → {tx.to}</p>
                        <p className="text-xs text-gray-500">{tx.time}</p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="text-sm text-white font-medium">{tx.amount}</p>
                      <p className={`text-xs ${tx.status === 'completed' ? 'text-emerald-400' : 'text-amber-400'}`}>
                        {tx.status}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
