'use client';

import { useMemo, useState } from 'react';
import { Button, Badge } from '@/components/x3/UIComponents';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from 'recharts';

const DEMO_PERFORMANCE = Array.from({ length: 30 }, (_, i) => {
  const day = String(i + 1).padStart(2, '0');
  const pnl = ((i * 37) % 600) - 100;
  const cumulative = -100 + i * 55 + ((i * 23) % 90);
  return {
    date: `2026-01-${day}`,
    pnl,
    cumulative,
  };
});

const DEMO_TRADES = [
  { id: 1, pair: 'PDEX/USDT', side: 'BUY', entry: 2.30, exit: 2.43, quantity: 1000, pnl: 130, roi: 5.7, date: '2024-02-10' },
  { id: 2, pair: 'DOT/USDT', side: 'SELL', entry: 8.50, exit: 8.32, quantity: 100, pnl: 1800, roi: 2.1, date: '2024-02-09' },
  { id: 3, pair: 'PDEX/USDT', side: 'BUY', entry: 2.20, exit: 2.40, quantity: 500, pnl: 100, roi: 9.1, date: '2024-02-08' },
  { id: 4, pair: 'KSM/USDT', side: 'BUY', entry: 190.00, exit: 195.42, quantity: 5, pnl: 27.10, roi: 2.9, date: '2024-02-07' },
  { id: 5, pair: 'BTC/USDT', side: 'SELL', entry: 88000, exit: 87234, quantity: 0.1, pnl: 76.60, roi: 0.9, date: '2024-02-06' },
];

const DEMO_PAIR_STATS = [
  { pair: 'PDEX/USDT', trades: 12, wins: 8, losses: 4, roi: 12.5 },
  { pair: 'DOT/USDT', trades: 8, wins: 6, losses: 2, roi: 8.2 },
  { pair: 'KSM/USDT', trades: 5, wins: 3, losses: 2, roi: 5.1 },
  { pair: 'BTC/USDT', trades: 3, wins: 2, losses: 1, roi: 3.4 },
];

export default function TradingAnalyticsPage() {
  const [sortBy, setSortBy] = useState<'date' | 'pnl' | 'roi'>('date');
  const [filterPair, setFilterPair] = useState<string>('ALL');

  const stats = useMemo(() => {
    const totalTrades = DEMO_TRADES.length;
    const winningTrades = DEMO_TRADES.filter((t) => t.pnl > 0).length;
    const losingTrades = totalTrades - winningTrades;
    const totalPnL = DEMO_TRADES.reduce((sum, t) => sum + t.pnl, 0);
    const avgRoi = (DEMO_TRADES.reduce((sum, t) => sum + t.roi, 0) / totalTrades).toFixed(2);
    const winRate = ((winningTrades / totalTrades) * 100).toFixed(1);

    return {
      totalTrades,
      winningTrades,
      losingTrades,
      totalPnL,
      avgRoi,
      winRate,
    };
  }, []);

  const sortedTrades = useMemo(() => {
    let filtered = filterPair === 'ALL' ? DEMO_TRADES : DEMO_TRADES.filter((t) => t.pair === filterPair);

    return [...filtered].sort((a, b) => {
      if (sortBy === 'date') return new Date(b.date).getTime() - new Date(a.date).getTime();
      if (sortBy === 'pnl') return b.pnl - a.pnl;
      if (sortBy === 'roi') return b.roi - a.roi;
      return 0;
    });
  }, [sortBy, filterPair]);

  const pairDistribution = DEMO_PAIR_STATS.map((p) => ({ name: p.pair, value: p.trades }));
  const colors = ['#00d4aa', '#4488ff', '#ffaa00', '#ff4444'];

  return (
    <div className="min-h-screen bg-gradient-to-br from-x3-dark via-x3-dark to-[#0f0f13] p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-4xl font-bold text-white">Trading Analytics</h1>
          <p className="text-gray-400 mt-1">Performance metrics and trade history</p>
        </div>
        <div className="flex gap-2">
          <Button variant="primary" size="sm">
            📊 Export Report
          </Button>
          <Button variant="secondary" size="sm">
            📈 Performance
          </Button>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">Total P&L</div>
          <div className="text-3xl font-bold text-x3-orange">${stats.totalPnL.toFixed(2)}</div>
          <div className="text-xs text-gray-500 mt-2">From {stats.totalTrades} trades</div>
        </div>

        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">Win Rate</div>
          <div className="text-3xl font-bold text-green-400">{stats.winRate}%</div>
          <div className="text-xs text-gray-500 mt-2">
            {stats.winningTrades}W / {stats.losingTrades}L
          </div>
        </div>

        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">Avg ROI</div>
          <div className="text-3xl font-bold text-blue-400">{stats.avgRoi}%</div>
          <div className="text-xs text-gray-500 mt-2">Per trade</div>
        </div>

        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <div className="text-sm text-gray-400 mb-2">Best Trade</div>
          <div className="text-3xl font-bold text-x3-orange">
            ${Math.max(...DEMO_TRADES.map((t) => t.pnl)).toFixed(2)}
          </div>
          <div className="text-xs text-gray-500 mt-2">Max gain</div>
        </div>
      </div>

      {/* Charts Grid */}
      <div className="grid grid-cols-3 gap-4">
        {/* P&L Over Time */}
        <div className="col-span-2 bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <h2 className="text-lg font-bold text-white mb-4">📈 P&L Progress (30 Days)</h2>
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={DEMO_PERFORMANCE}>
              <defs>
                <linearGradient id="colorPnl" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#00d4aa" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#00d4aa" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
              <XAxis dataKey="date" stroke="#8a8a8e" tick={{ fontSize: 11 }} />
              <YAxis stroke="#8a8a8e" />
              <Tooltip
                contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #00d4aa' }}
                labelStyle={{ color: '#00d4aa' }}
              />
              <Area type="monotone" dataKey="cumulative" stroke="#00d4aa" fill="url(#colorPnl)" />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        {/* Pairs Distribution */}
        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <h2 className="text-lg font-bold text-white mb-4">📊 Pair Distribution</h2>
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={pairDistribution}
                cx="50%"
                cy="50%"
                labelLine={false}
                label={({ name, value }: any) => `${name}: ${value}`}
                outerRadius={70}
                fill="#8884d8"
                dataKey="value"
              >
                {pairDistribution.map((_, index) => (
                  <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
                ))}
              </Pie>
              <Tooltip contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }} />
            </PieChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Pair Performance */}
      <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
        <h2 className="text-lg font-bold text-white mb-4">💰 Performance by Pair</h2>
        <div className="grid grid-cols-4 gap-4">
          {DEMO_PAIR_STATS.map((stat) => (
            <div
              key={stat.pair}
              onClick={() => setFilterPair(stat.pair)}
              className={`p-4 rounded-lg border cursor-pointer transition-all ${
                filterPair === stat.pair
                  ? 'bg-x3-orange/20 border-x3-orange'
                  : 'bg-x3-dark border-x3-dark-gray hover:border-x3-orange'
              }`}
            >
              <div className="font-bold text-sm text-white mb-3">{stat.pair}</div>
              <div className="space-y-2 text-xs">
                <div className="flex justify-between">
                  <span className="text-gray-400">Trades:</span>
                  <span className="text-white font-bold">{stat.trades}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">W/L:</span>
                  <span className="text-white font-bold">
                    <span className="text-green-400">{stat.wins}</span>/<span className="text-red-400">{stat.losses}</span>
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">ROI:</span>
                  <span className={`font-bold ${stat.roi > 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {stat.roi > 0 ? '+' : ''}{stat.roi}%
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Trade History */}
      <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-lg font-bold text-white">📜 Trade History</h2>
          <div className="flex gap-2">
            <select
              value={filterPair}
              onChange={(e) => setFilterPair(e.target.value)}
              className="px-3 py-1 bg-x3-dark border border-x3-dark-gray rounded text-sm text-gray-400 focus:border-x3-orange outline-none"
            >
              <option value="ALL">All Pairs</option>
              {DEMO_PAIR_STATS.map((p) => (
                <option key={p.pair} value={p.pair}>
                  {p.pair}
                </option>
              ))}
            </select>
            <div className="flex gap-1">
              {(['date', 'pnl', 'roi'] as const).map((sb) => (
                <button
                  key={sb}
                  onClick={() => setSortBy(sb)}
                  className={`px-3 py-1 rounded text-xs font-bold transition-colors ${
                    sortBy === sb
                      ? 'bg-x3-orange text-white'
                      : 'bg-x3-dark-gray text-gray-400 hover:text-gray-300'
                  }`}
                >
                  {sb === 'date' ? '📅' : sb === 'pnl' ? '💰' : '📊'} {sb.toUpperCase()}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-x3-dark-gray">
                <th className="text-left py-2 px-3 text-gray-400 font-bold">Pair</th>
                <th className="text-left py-2 px-3 text-gray-400 font-bold">Side</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Entry</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Exit</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Qty</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">P&L</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">ROI</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Date</th>
              </tr>
            </thead>
            <tbody>
              {sortedTrades.map((trade) => (
                <tr key={trade.id} className="border-b border-x3-dark-gray hover:bg-x3-dark/50 transition-colors">
                  <td className="py-3 px-3 font-bold text-white">{trade.pair}</td>
                  <td className="py-3 px-3">
                    <Badge variant={trade.side === 'BUY' ? 'green' : 'red'}>{trade.side}</Badge>
                  </td>
                  <td className="py-3 px-3 text-right text-gray-400">${trade.entry.toFixed(4)}</td>
                  <td className="py-3 px-3 text-right text-gray-400">${trade.exit.toFixed(4)}</td>
                  <td className="py-3 px-3 text-right text-gray-400">{trade.quantity}</td>
                  <td className={`py-3 px-3 text-right font-bold ${trade.pnl > 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {trade.pnl > 0 ? '+' : ''} ${trade.pnl.toFixed(2)}
                  </td>
                  <td className={`py-3 px-3 text-right font-bold ${trade.roi > 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {trade.roi > 0 ? '+' : ''}{trade.roi}%
                  </td>
                  <td className="py-3 px-3 text-right text-gray-400 text-xs">{trade.date}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
