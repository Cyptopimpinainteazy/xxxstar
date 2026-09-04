'use client';

import Link from 'next/link';

export default function Home() {
  return (
    <main className="min-h-screen bg-gradient-to-b from-slate-900 via-slate-800 to-slate-900">
      {/* Navigation Header */}
      <nav className="border-b border-slate-700 bg-slate-800/50 backdrop-blur">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <h1 className="text-2xl font-bold text-white">X3 Chain Wallet</h1>
        </div>
      </nav>

      {/* Hero Section */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-20">
        <div className="text-center mb-20">
          <h2 className="text-5xl font-bold text-white mb-4">Welcome to X3 Chain</h2>
          <p className="text-xl text-slate-300 mb-8">
            Trading & DeFi Platform Unite
          </p>
        </div>

        {/* Module Cards */}
        <div className="grid md:grid-cols-2 gap-8 max-w-4xl mx-auto">
          {/* X3 Trading Module */}
          <Link
            href="/trading/floor"
            className="group card hover:border-indigo-500 hover:shadow-xl hover:shadow-indigo-500/20 transition-all duration-300 cursor-pointer block"
          >
            <div className="mb-4 text-5xl">📊</div>
            <h3 className="text-2xl font-bold text-white mb-2 group-hover:text-indigo-400 transition-colors">
              X3 Trading
            </h3>
            <p className="text-slate-400 mb-6">
              Real-time arbitrage floor monitoring, agent management, and slashing tracking.
            </p>
            <div className="flex items-center gap-2 text-indigo-400 group-hover:gap-3 transition-all">
              <span>Explore</span>
              <span className="text-xl">→</span>
            </div>
            <div className="mt-4 text-sm text-slate-500">
              • Floor Stats • Arbitrage • Agents • Intents • Slashing
            </div>
          </Link>

          {/* Polkadex DEX Module */}
          <Link
            href="/polkadex/trading"
            className="group card hover:border-pink-500 hover:shadow-xl hover:shadow-pink-500/20 transition-all duration-300 cursor-pointer block"
          >
            <div className="mb-4 text-5xl">🔄</div>
            <h3 className="text-2xl font-bold text-white mb-2 group-hover:text-pink-400 transition-colors">
              Polkadex
            </h3>
            <p className="text-slate-400 mb-6">
              Decentralized exchange with full trading capabilities, order management, and portfolio tracking.
            </p>
            <div className="flex items-center gap-2 text-pink-400 group-hover:gap-3 transition-all">
              <span>Explore</span>
              <span className="text-xl">→</span>
            </div>
            <div className="mt-4 text-sm text-slate-500">
              • Markets • Trading • Orders • Portfolio • Settings
            </div>
          </Link>
        </div>

        {/* Feature Grid */}
        <div className="mt-20 grid md:grid-cols-3 gap-6 max-w-4xl mx-auto">
          <div className="p-6 bg-slate-700/30 rounded-lg border border-slate-600">
            <h4 className="text-lg font-semibold text-white mb-2">⚡ Real-Time Data</h4>
            <p className="text-slate-400 text-sm">
              Live order books, price updates, and market data
            </p>
          </div>
          <div className="p-6 bg-slate-700/30 rounded-lg border border-slate-600">
            <h4 className="text-lg font-semibold text-white mb-2">🔐 Secure Trading</h4>
            <p className="text-slate-400 text-sm">
              WalletConnect integration for signing orders
            </p>
          </div>
          <div className="p-6 bg-slate-700/30 rounded-lg border border-slate-600">
            <h4 className="text-lg font-semibold text-white mb-2">📈 Advanced Charts</h4>
            <p className="text-slate-400 text-sm">
              Professional charting with technical indicators
            </p>
          </div>
        </div>
      </div>
    </main>
  );
}
