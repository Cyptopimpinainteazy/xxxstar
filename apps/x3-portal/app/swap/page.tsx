'use client';

import SwapInterface from '@/components/swap/SwapInterface';
import { ArrowLeftRight, Zap, Shield, Globe } from 'lucide-react';

export default function SwapPage() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-950 via-slate-900 to-slate-950">
      {/* Hero Section */}
      <div className="container mx-auto px-4 py-12">
        <div className="text-center mb-12">
          <h1 className="text-4xl md:text-5xl font-bold text-white mb-4">
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500">
              Atomic
            </span>{' '}
            Cross-Chain Swaps
          </h1>
          <p className="text-xl text-slate-400 max-w-2xl mx-auto">
            Swap tokens across 103 EVM chains in 6 seconds. No bridges. No waiting. No risk.
          </p>
        </div>

        {/* Features */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-4xl mx-auto mb-12">
          <div className="bg-slate-800/50 rounded-xl p-6 border border-slate-700">
            <Zap className="w-10 h-10 text-cyan-400 mb-4" />
            <h3 className="text-lg font-bold text-white mb-2">6 Second Finality</h3>
            <p className="text-slate-400 text-sm">
              Traditional bridges take 15-45 minutes. Comit transactions execute in 1 block.
            </p>
          </div>
          <div className="bg-slate-800/50 rounded-xl p-6 border border-slate-700">
            <Shield className="w-10 h-10 text-green-400 mb-4" />
            <h3 className="text-lg font-bold text-white mb-2">Zero Risk</h3>
            <p className="text-slate-400 text-sm">
              Atomic execution means both sides complete or neither does. No stuck funds.
            </p>
          </div>
          <div className="bg-slate-800/50 rounded-xl p-6 border border-slate-700">
            <Globe className="w-10 h-10 text-blue-400 mb-4" />
            <h3 className="text-lg font-bold text-white mb-2">103 Chains</h3>
            <p className="text-slate-400 text-sm">
              Ethereum, Arbitrum, Base, Polygon, BSC, Avalanche, zkSync, Scroll, and 95 more.
            </p>
          </div>
        </div>

        {/* Swap Interface */}
        <SwapInterface />

        {/* How it works */}
        <div className="mt-16 max-w-3xl mx-auto">
          <h2 className="text-2xl font-bold text-white text-center mb-8">
            How Atomic Swaps Work
          </h2>
          
          <div className="relative">
            {/* Timeline line */}
            <div className="absolute left-1/2 top-0 bottom-0 w-0.5 bg-gradient-to-b from-cyan-500 to-blue-500" />
            
            {/* Steps */}
            <div className="space-y-8">
              <div className="flex items-center gap-8">
                <div className="flex-1 text-right">
                  <h3 className="text-white font-bold">1. Select Chains & Tokens</h3>
                  <p className="text-slate-400 text-sm">Choose source and destination from 103 chains</p>
                </div>
                <div className="w-10 h-10 rounded-full bg-cyan-500 flex items-center justify-center z-10">
                  <span className="text-white font-bold">1</span>
                </div>
                <div className="flex-1" />
              </div>
              
              <div className="flex items-center gap-8">
                <div className="flex-1" />
                <div className="w-10 h-10 rounded-full bg-cyan-500 flex items-center justify-center z-10">
                  <span className="text-white font-bold">2</span>
                </div>
                <div className="flex-1 text-left">
                  <h3 className="text-white font-bold">2. Router Finds Best Path</h3>
                  <p className="text-slate-400 text-sm">Optimal route through liquidity pools</p>
                </div>
              </div>
              
              <div className="flex items-center gap-8">
                <div className="flex-1 text-right">
                  <h3 className="text-white font-bold">3. Comit Bundle Created</h3>
                  <p className="text-slate-400 text-sm">Atomic transaction with prepare_root commitment</p>
                </div>
                <div className="w-10 h-10 rounded-full bg-cyan-500 flex items-center justify-center z-10">
                  <span className="text-white font-bold">3</span>
                </div>
                <div className="flex-1" />
              </div>
              
              <div className="flex items-center gap-8">
                <div className="flex-1" />
                <div className="w-10 h-10 rounded-full bg-green-500 flex items-center justify-center z-10">
                  <span className="text-white font-bold">✓</span>
                </div>
                <div className="flex-1 text-left">
                  <h3 className="text-white font-bold">4. Atomic Execution</h3>
                  <p className="text-slate-400 text-sm">All legs execute in 1 Atlas block (6 seconds)</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Supported Chains Grid */}
        <div className="mt-16 max-w-4xl mx-auto">
          <h2 className="text-2xl font-bold text-white text-center mb-8">
            Supported Chains
          </h2>
          <div className="grid grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-4">
            {[
              { name: 'Ethereum', logo: '⟠' },
              { name: 'Arbitrum', logo: '🔷' },
              { name: 'Base', logo: '🔵' },
              { name: 'Polygon', logo: '🟣' },
              { name: 'BSC', logo: '🟡' },
              { name: 'Avalanche', logo: '🔺' },
              { name: 'Optimism', logo: '🔴' },
              { name: 'zkSync', logo: '⚡' },
              { name: 'Fantom', logo: '👻' },
              { name: 'Cronos', logo: '🔷' },
              { name: 'Klaytn', logo: '🔶' },
              { name: 'Scroll', logo: '📜' },
              { name: 'Gnosis', logo: '🦉' },
              { name: 'Metis', logo: '🌀' },
              { name: 'Moonbeam', logo: '🌙' },
              { name: '+88 more', logo: '🌐' },
            ].map((chain, i) => (
              <div
                key={i}
                className="bg-slate-800/50 rounded-lg p-3 text-center hover:bg-slate-700/50 transition cursor-pointer border border-slate-700"
              >
                <div className="text-2xl mb-1">{chain.logo}</div>
                <div className="text-xs text-slate-400 truncate">{chain.name}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
