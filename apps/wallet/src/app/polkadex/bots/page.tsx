'use client';

import { useState } from 'react';
import { Button, Badge } from '@/components/x3/UIComponents';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

const GRID_BOT = {
  name: 'Grid Trading Bot',
  description: 'Automatically buys low and sells high within a price range',
  icon: '📊',
  profitData: Array.from({ length: 12 }, (_, i) => ({
    day: `Day ${i + 1}`,
    profit: 220 + i * 35 + (i % 3) * 40,
  })),
  stats: {
    active: true,
    roi: '12.5%',
    trades: 847,
    profit: '$3,240.50',
    pair: 'PDEX/USDT',
    gridLevels: 20,
  },
};

const DCA_BOT = {
  name: 'DCA Bot (Dollar Cost Averaging)',
  description: 'Invests fixed amount regularly, reducing impact of volatility',
  icon: '💰',
  profitData: Array.from({ length: 12 }, (_, i) => ({
    day: `Day ${i + 1}`,
    profit: i * 150 + (i % 4) * 25,
  })),
  stats: {
    active: true,
    roi: '8.2%',
    trades: 52,
    profit: '$1,856.30',
    pair: 'DOT/USDT',
    interval: 'Daily',
  },
};

const ARBITRAGE_BOT = {
  name: 'Arbitrage Bot',
  description: 'Exploits price differences across different trading pairs',
  icon: '⚡',
  profitData: Array.from({ length: 12 }, (_, i) => ({
    day: `Day ${i + 1}`,
    profit: 320 + i * 45 + (i % 2) * 90,
  })),
  stats: {
    active: true,
    roi: '18.7%',
    trades: 1243,
    profit: '$5,642.80',
    pair: 'BTC/ETH',
    spread: '0.15%',
  },
};

const LIQUIDITY_BOT = {
  name: 'Smart Liquidity Bot',
  description: 'Automatically provides liquidity and earns trading fees',
  icon: '💧',
  profitData: Array.from({ length: 12 }, (_, i) => ({
    day: `Day ${i + 1}`,
    profit: 450 + (i % 5) * 35,
  })),
  stats: {
    active: true,
    roi: '14.3%',
    trades: 3847,
    profit: '$4,125.60',
    pool: 'BTC/USDT LP',
    liquidity: '$124,560',
  },
};

const BOTS = [GRID_BOT, DCA_BOT, ARBITRAGE_BOT, LIQUIDITY_BOT];

export default function BotTradersPage() {
  const [selectedBot, setSelectedBot] = useState<(typeof BOTS)[number]>(GRID_BOT);
  const [botStates, setBotStates] = useState({
    active: [true, true, true, true],
  });

  const toggleBot = (index: number) => {
    const newState = [...botStates.active];
    newState[index] = !newState[index];
    setBotStates({ ...botStates, active: newState });
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-x3-dark via-x3-dark to-[#0f0f13] p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-4xl font-bold text-white">🤖 AI Bot Traders</h1>
          <p className="text-gray-400 mt-1">4 Automated Trading Bots - Trade & Earn 24/7</p>
        </div>
        <div className="flex gap-2">
          <Badge>🟢 4 Bots Active</Badge>
          <Badge variant="green">📈 $15,000+ Daily Profit</Badge>
        </div>
      </div>

      {/* Bot Overview Cards */}
      <div className="grid grid-cols-4 gap-4">
        {BOTS.map((bot, idx) => (
          <div
            key={idx}
            onClick={() => setSelectedBot(bot)}
            className={`p-4 rounded-lg border cursor-pointer transition-all ${
              selectedBot.name === bot.name
                ? 'bg-x3-orange border-x3-orange'
                : 'bg-x3-dark border-x3-dark-gray hover:border-x3-orange'
            }`}
          >
            <div className="text-3xl mb-2">{bot.icon}</div>
            <div className={`font-bold text-sm mb-1 ${selectedBot.name === bot.name ? 'text-white' : 'text-gray-300'}`}>
              {bot.name}
            </div>
            <div className={`text-xs ${selectedBot.name === bot.name ? 'text-white' : 'text-gray-400'}`}>
              {botStates.active[idx] ? '🟢 Active' : '🔴 Inactive'}
            </div>
            <div className={`text-lg font-bold mt-2 ${selectedBot.name === bot.name ? 'text-white' : 'text-x3-orange'}`}>
              {Array.isArray(BOTS[idx]?.stats) 
                ? BOTS[idx]?.stats[0]?.roi 
                : BOTS[idx]?.stats?.roi}
            </div>
          </div>
        ))}
      </div>

      {/* Selected Bot Details */}
      <div className="grid grid-cols-3 gap-6">
        {/* Chart & Stats */}
        <div className="col-span-2 space-y-4">
          {/* Profit Chart */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
            <h2 className="text-lg font-bold text-white mb-4">
              {selectedBot.icon} {selectedBot.name} - 12 Day Performance
            </h2>
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={selectedBot.profitData}>
                <defs>
                  <linearGradient id="colorProfit" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#00d4aa" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#00d4aa" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
                <XAxis dataKey="day" stroke="#8a8a8e" />
                <YAxis stroke="#8a8a8e" />
                <Tooltip
                  contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #00d4aa' }}
                  labelStyle={{ color: '#00d4aa' }}
                />
                <Area
                  type="monotone"
                  dataKey="profit"
                  stroke="#00d4aa"
                  fill="url(#colorProfit)"
                  strokeWidth={2}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {/* Description */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
            <h3 className="text-lg font-bold text-white mb-3">How It Works</h3>
            <p className="text-gray-400 text-sm leading-relaxed">{selectedBot.description}</p>
            <div className="mt-4 space-y-2">
              {selectedBot.name === 'Grid Trading Bot' && (
                <>
                  <p className="text-xs text-gray-500">• Sets up a grid of buy and sell orders within your specified price range</p>
                  <p className="text-xs text-gray-500">• Automatically captures profits from short-term price fluctuations</p>
                  <p className="text-xs text-gray-500">• No need to time the market perfectly - profits from volatility</p>
                </>
              )}
              {selectedBot.name === 'DCA Bot (Dollar Cost Averaging)' && (
                <>
                  <p className="text-xs text-gray-500">• Invests a fixed amount at regular intervals regardless of price</p>
                  <p className="text-xs text-gray-500">• Reduces the impact of market volatility on your investments</p>
                  <p className="text-xs text-gray-500">• Ideal for building positions over time with less risk</p>
                </>
              )}
              {selectedBot.name === 'Arbitrage Bot' && (
                <>
                  <p className="text-xs text-gray-500">• Identifies and exploits price differences across pairs simultaneously</p>
                  <p className="text-xs text-gray-500">• Low-risk strategy - profits from inefficiencies in the market</p>
                  <p className="text-xs text-gray-500">• Executes trades in milliseconds before price normalizes</p>
                </>
              )}
              {selectedBot.name === 'Smart Liquidity Bot' && (
                <>
                  <p className="text-xs text-gray-500">• Provides liquidity to pairs and earns trading fees automatically</p>
                  <p className="text-xs text-gray-500">• Rebalances positions based on market moving averages</p>
                  <p className="text-xs text-gray-500">• Passive income generation from your locked liquidity</p>
                </>
              )}
            </div>
          </div>
        </div>

        {/* Control Panel */}
        <div className="space-y-4">
          {/* Bot Stats */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray space-y-3">
            <h3 className="font-bold text-white">Current Stats</h3>

            {selectedBot.stats && Object.entries(selectedBot.stats).map(([key, value]) => (
              <div key={key} className="flex justify-between pb-3 border-b border-x3-dark-gray last:border-0 last:pb-0">
                <span className="text-gray-400 text-sm capitalize">{key.replace(/([A-Z])/g, ' $1')}:</span>
                <span className="font-bold text-white">
                  {key === 'active' ? (value ? '✅ Yes' : '❌ No') : value}
                </span>
              </div>
            ))}
          </div>

          {/* Control Buttons */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray space-y-2">
            <Button
              variant={botStates.active[BOTS.indexOf(selectedBot)] ? 'danger' : 'success'}
              onClick={() => toggleBot(BOTS.indexOf(selectedBot))}
              className="w-full"
            >
              {botStates.active[BOTS.indexOf(selectedBot)] ? '⏹ Stop Bot' : '▶ Start Bot'}
            </Button>
            <Button variant="primary" className="w-full">
              ⚙️ Configure
            </Button>
            <Button variant="secondary" className="w-full">
              📊 View History
            </Button>
          </div>

          {/* Performance Badge */}
          <div className="bg-gradient-to-br from-green-900/20 to-transparent p-4 rounded-lg border border-green-600/30">
            <div className="text-sm font-bold text-green-400 mb-2">💯 Performance</div>
            <div className="space-y-1">
              <div className="flex justify-between text-xs">
                <span className="text-gray-400">Total Profit:</span>
                <span className="text-green-400 font-bold">{Object.values(selectedBot.stats)[3]}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-gray-400">ROI:</span>
                <span className="text-green-400 font-bold">{Object.values(selectedBot.stats)[0]}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-gray-400">Trades:</span>
                <span className="text-green-400 font-bold">{Object.values(selectedBot.stats)[1]}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Key Features */}
      <div>
        <h2 className="text-2xl font-bold text-white mb-4">✨ Key Features</h2>
        <div className="grid grid-cols-4 gap-4">
          <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
            <div className="text-2xl mb-2">🔐</div>
            <h3 className="font-bold text-white mb-1">Secure</h3>
            <p className="text-xs text-gray-400">All bots execute through secure smart contracts with no direct access to funds</p>
          </div>
          <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
            <div className="text-2xl mb-2">⏱️</div>
            <h3 className="font-bold text-white mb-1">24/7 Trading</h3>
            <p className="text-xs text-gray-400">Bots trade automatically around the clock without manual intervention</p>
          </div>
          <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
            <div className="text-2xl mb-2">⚙️</div>
            <h3 className="font-bold text-white mb-1">Customizable</h3>
            <p className="text-xs text-gray-400">Configure each bot with your own parameters and risk preferences</p>
          </div>
          <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
            <div className="text-2xl mb-2">📈</div>
            <h3 className="font-bold text-white mb-1">Data-Driven</h3>
            <p className="text-xs text-gray-400">AI analyzes market conditions and optimizes strategies in real-time</p>
          </div>
        </div>
      </div>
    </div>
  );
}
