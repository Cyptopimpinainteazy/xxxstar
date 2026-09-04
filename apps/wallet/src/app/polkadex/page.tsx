'use client';

import Link from 'next/link';
import { Button, Badge } from '@/components/x3/UIComponents';
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

const DEMO_CHART = Array.from({ length: 12 }, (_, i) => ({
  hour: `${i}:00`,
  price: 2.45 + Math.sin(i / 2) * 0.12 + (i % 3) * 0.01,
}));

export default function XDEXDashboard() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-x3-dark via-x3-dark to-[#0f0f13] p-6 space-y-6">
      {/* Hero Section */}
      <div className="bg-gradient-to-r from-x3-orange/10 to-purple-600/10 border border-x3-dark-gray rounded-lg p-8 backdrop-blur-sm">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-5xl font-bold text-white mb-2">X-DEX: The Future of Trading</h1>
            <p className="text-gray-400 text-lg">AI-Powered Decentralized Exchange with Automated Traders & Token Launchpad</p>
          </div>
          <Badge>🟢 Live Trading</Badge>
        </div>
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">24h Volume</div>
          <div className="text-3xl font-bold text-x3-orange">$342.8M</div>
          <div className="text-xs text-green-400 mt-2">↑ 28.5% with bots</div>
        </div>
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">Active Bot Traders</div>
          <div className="text-3xl font-bold text-blue-400">3,847</div>
          <div className="text-xs text-gray-400 mt-2">AI trading 24/7</div>
        </div>
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">Tokens Launched</div>
          <div className="text-3xl font-bold text-green-400">127</div>
          <div className="text-xs text-gray-400 mt-2">Via X-DEX Launchpad</div>
        </div>
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">Avg Spread</div>
          <div className="text-3xl font-bold text-purple-400">0.02%</div>
          <div className="text-xs text-gray-400 mt-2">Lowest in DeFi</div>
        </div>
      </div>

      {/* Feature Cards */}
      <div>
        <h2 className="text-2xl font-bold text-white mb-4">🚀 2025 Leading Solutions</h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <Link
            href="/polkadex/bots"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">🤖</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">AI Bot Traders</h3>
            <p className="text-xs text-gray-400">4 automated bots trading 24/7</p>
          </Link>

          <Link
            href="/polkadex/launchpad"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">🚀</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Token Launchpad</h3>
            <p className="text-xs text-gray-400">Launch X3 & new tokens</p>
          </Link>

          <Link
            href="/polkadex/trading"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">📊</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Basic Trading</h3>
            <p className="text-xs text-gray-400">Buy & Sell with limit orders</p>
          </Link>

          <Link
            href="/polkadex/advanced"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">⚡</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Advanced Trading</h3>
            <p className="text-xs text-gray-400">Professional charts & indicators</p>
          </Link>

          <Link
            href="/polkadex/scanner"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">🎯</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Market Scanner</h3>
            <p className="text-xs text-gray-400">Find profitable opportunities</p>
          </Link>

          <Link
            href="/polkadex/analytics"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">📈</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Analytics</h3>
            <p className="text-xs text-gray-400">P&L tracking & insights</p>
          </Link>

          <Link
            href="/polkadex/portfolio"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">💼</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Portfolio</h3>
            <p className="text-xs text-gray-400">Asset management & allocation</p>
          </Link>

          <Link
            href="/polkadex/orders"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">📋</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Orders</h3>
            <p className="text-xs text-gray-400">Order management & history</p>
          </Link>

          <Link
            href="/polkadex/markets"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">🔄</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Markets</h3>
            <p className="text-xs text-gray-400">Market data & statistics</p>
          </Link>

          <Link
            href="/polkadex/settings"
            className="group bg-gradient-to-br from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray hover:border-x3-orange backdrop-blur-sm transition-all hover:shadow-lg hover:shadow-x3-orange/20"
          >
            <div className="text-3xl mb-2">⚙️</div>
            <h3 className="font-bold text-white mb-1 group-hover:text-x3-orange transition-colors">Settings</h3>
            <p className="text-xs text-gray-400">Configure your preferences</p>
          </Link>
        </div>
      </div>

      {/* Live Data */}
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <h3 className="text-lg font-bold text-white mb-4">📈 PDEX/USDT Live</h3>
          <ResponsiveContainer width="100%" height={250}>
            <AreaChart data={DEMO_CHART}>
              <defs>
                <linearGradient id="colorPdex" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#00d4aa" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#00d4aa" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
              <XAxis dataKey="hour" stroke="#8a8a8e" />
              <YAxis stroke="#8a8a8e" domain={['dataMin - 0.1', 'dataMax + 0.1']} />
              <Tooltip contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #00d4aa' }} />
              <Area type="monotone" dataKey="price" stroke="#00d4aa" fill="url(#colorPdex)" />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray space-y-4 backdrop-blur-sm">
          <h3 className="text-lg font-bold text-white">🚀 Key Features</h3>
          <ul className="space-y-3 text-sm">
            <li className="flex items-center gap-2 text-gray-300">
              <span className="text-x3-orange">✓</span> Stop Loss & Take Profit Orders
            </li>
            <li className="flex items-center gap-2 text-gray-300">
              <span className="text-x3-orange">✓</span> Advanced Technical Indicators
            </li>
            <li className="flex items-center gap-2 text-gray-300">
              <span className="text-x3-orange">✓</span> Real-Time Market Scanner
            </li>
            <li className="flex items-center gap-2 text-gray-300">
              <span className="text-x3-orange">✓</span> Comprehensive Analytics
            </li>
            <li className="flex items-center gap-2 text-gray-300">
              <span className="text-x3-orange">✓</span> Multi-Asset Portfolio Management
            </li>
            <li className="flex items-center gap-2 text-gray-300">
              <span className="text-x3-orange">✓</span> Trailing Stop Orders
            </li>
          </ul>
          <Button variant="primary" className="w-full mt-4">
            Start Trading Now →
          </Button>
        </div>
      </div>
    </div>
  );
}
