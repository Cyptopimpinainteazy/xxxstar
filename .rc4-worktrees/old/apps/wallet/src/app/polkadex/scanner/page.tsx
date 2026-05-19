'use client';

import { useState, useMemo } from 'react';
import { Button, Badge } from '@/components/x3/UIComponents';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

interface MarketPair {
  pair: string;
  price: number;
  change24h: number;
  change1h: number;
  volume24h: number;
  volumeChange: number;
  bid: number;
  ask: number;
  spread: number;
  momentum: number;
  trend: 'UP' | 'DOWN' | 'NEUTRAL';
}

const DEMO_MARKETS: MarketPair[] = [
  {
    pair: 'PDEX/USDT',
    price: 2.45,
    change24h: 12.5,
    change1h: 2.3,
    volume24h: 1_234_567,
    volumeChange: 45.2,
    bid: 2.44,
    ask: 2.46,
    spread: 0.08,
    momentum: 78,
    trend: 'UP',
  },
  {
    pair: 'DOT/USDT',
    price: 8.32,
    change24h: 5.2,
    change1h: -1.1,
    volume24h: 5_678_900,
    volumeChange: -12.3,
    bid: 8.31,
    ask: 8.33,
    spread: 0.025,
    momentum: 52,
    trend: 'NEUTRAL',
  },
  {
    pair: 'KSM/USDT',
    price: 195.42,
    change24h: -3.1,
    change1h: 0.8,
    volume24h: 432_100,
    volumeChange: 22.1,
    bid: 195.3,
    ask: 195.5,
    spread: 0.10,
    momentum: 48,
    trend: 'DOWN',
  },
  {
    pair: 'BTC/USDT',
    price: 87234.50,
    change24h: 8.2,
    change1h: 1.5,
    volume24h: 12_456_789,
    volumeChange: 65.3,
    bid: 87224,
    ask: 87245,
    spread: 0.024,
    momentum: 85,
    trend: 'UP',
  },
  {
    pair: 'ETH/USDT',
    price: 4562.30,
    change24h: 6.8,
    change1h: 0.5,
    volume24h: 8_765_432,
    volumeChange: 28.5,
    bid: 4562,
    ask: 4563,
    spread: 0.022,
    momentum: 72,
    trend: 'UP',
  },
  {
    pair: 'ADA/USDT',
    price: 1.45,
    change24h: -2.5,
    change1h: -0.3,
    volume24h: 2_345_678,
    volumeChange: 5.2,
    bid: 1.44,
    ask: 1.46,
    spread: 0.014,
    momentum: 35,
    trend: 'DOWN',
  },
];

export default function MarketScannerPage() {
  const [sortBy, setSortBy] = useState<'price' | 'change24h' | 'volume' | 'momentum'>('change24h');
  const [filterTrend, setFilterTrend] = useState<'ALL' | 'UP' | 'DOWN'>('ALL');
  const [minVolume, setMinVolume] = useState(1000000);
  const [minMomentum, setMinMomentum] = useState(0);
  const [watchlist, setWatchlist] = useState<Set<string>>(new Set());

  const filtered = useMemo(() => {
    return DEMO_MARKETS.filter((m) => {
      const trendMatch = filterTrend === 'ALL' || m.trend === filterTrend;
      const volumeMatch = m.volume24h >= minVolume;
      const momentumMatch = m.momentum >= minMomentum;
      return trendMatch && volumeMatch && momentumMatch;
    }).sort((a, b) => {
      if (sortBy === 'change24h') return b.change24h - a.change24h;
      if (sortBy === 'volume') return b.volume24h - a.volume24h;
      if (sortBy === 'momentum') return b.momentum - a.momentum;
      return b.price - a.price;
    });
  }, [sortBy, filterTrend, minVolume, minMomentum]);

  const toggleWatchlist = (pair: string) => {
    const newSet = new Set(watchlist);
    if (newSet.has(pair)) {
      newSet.delete(pair);
    } else {
      newSet.add(pair);
    }
    setWatchlist(newSet);
  };

  // Mock momentum chart data
  const momentumData = filtered.slice(0, 5).map((m) => ({
    pair: m.pair.split('/')[0],
    momentum: m.momentum,
  }));

  // Volume comparison
  const volumeData = filtered.slice(0, 5).map((m) => ({
    pair: m.pair.split('/')[0],
    volume: m.volume24h / 1_000_000,
  }));

  const topGainers = [...filtered]
    .sort((a, b) => b.change24h - a.change24h)
    .slice(0, 3);

  const topLosers = [...DEMO_MARKETS]
    .sort((a, b) => a.change24h - b.change24h)
    .slice(0, 3);

  return (
    <div className="min-h-screen bg-gradient-to-br from-x3-dark via-x3-dark to-[#0f0f13] p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-4xl font-bold text-white">🎯 Market Scanner</h1>
          <p className="text-gray-400 mt-1">Find trading opportunities across all pairs</p>
        </div>
        <div className="flex gap-2">
          <Button variant="primary" size="sm">
            ⭐ Watchlist ({watchlist.size})
          </Button>
          <Button variant="secondary" size="sm">
            🔄 Refresh
          </Button>
        </div>
      </div>

      {/* Filters */}
      <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
        <div className="grid grid-cols-5 gap-4">
          <div>
            <label className="text-xs text-gray-400 block mb-2">Trend</label>
            <div className="flex gap-1">
              {(['ALL', 'UP', 'DOWN'] as const).map((trend) => (
                <button
                  key={trend}
                  onClick={() => setFilterTrend(trend)}
                  className={`flex-1 py-2 rounded text-xs font-bold transition-colors ${
                    filterTrend === trend
                      ? 'bg-x3-orange text-white'
                      : 'bg-x3-dark border border-x3-dark-gray text-gray-400 hover:text-gray-300'
                  }`}
                >
                  {trend === 'UP' ? '📈' : trend === 'DOWN' ? '📉' : '➡️'} {trend}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="text-xs text-gray-400 block mb-2">Min Volume ($)</label>
            <select
              value={minVolume}
              onChange={(e) => setMinVolume(Number(e.target.value))}
              className="w-full px-3 py-2 bg-x3-dark border border-x3-dark-gray rounded text-sm text-gray-400 focus:border-x3-orange outline-none"
            >
              <option value={100000}>$100K</option>
              <option value={1000000}>$1M</option>
              <option value={5000000}>$5M</option>
              <option value={10000000}>$10M+</option>
            </select>
          </div>

          <div>
            <label className="text-xs text-gray-400 block mb-2">Min Momentum</label>
            <input
              type="range"
              value={minMomentum}
              onChange={(e) => setMinMomentum(Number(e.target.value))}
              min="0"
              max="100"
              className="w-full"
            />
            <div className="text-xs text-center text-gray-400 mt-1">{minMomentum}</div>
          </div>

          <div>
            <label className="text-xs text-gray-400 block mb-2">Sort By</label>
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as any)}
              className="w-full px-3 py-2 bg-x3-dark border border-x3-dark-gray rounded text-sm text-gray-400 focus:border-x3-orange outline-none"
            >
              <option value="change24h">Change 24h</option>
              <option value="volume">Volume</option>
              <option value="momentum">Momentum</option>
              <option value="price">Price</option>
            </select>
          </div>

          <div className="flex items-end">
            <Button
              variant="primary"
              size="sm"
              onClick={() => {
                setSortBy('change24h');
                setFilterTrend('ALL');
                setMinVolume(1000000);
                setMinMomentum(0);
              }}
              className="w-full"
            >
              Reset Filters
            </Button>
          </div>
        </div>
      </div>

      {/* Charts Grid */}
      <div className="grid grid-cols-2 gap-4">
        {/* Momentum Chart */}
        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <h2 className="text-lg font-bold text-white mb-4">📊 Momentum Leaders</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={momentumData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
              <XAxis dataKey="pair" stroke="#8a8a8e" />
              <YAxis stroke="#8a8a8e" domain={[0, 100]} />
              <Tooltip
                contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
                labelStyle={{ color: '#00d4aa' }}
              />
              <Bar dataKey="momentum" fill="#00d4aa" />
            </BarChart>
          </ResponsiveContainer>
        </div>

        {/* Volume Chart */}
        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <h2 className="text-lg font-bold text-white mb-4">💰 24h Volume (Top 5)</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={volumeData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
              <XAxis dataKey="pair" stroke="#8a8a8e" />
              <YAxis stroke="#8a8a8e" />
              <Tooltip
                contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
                labelStyle={{ color: '#4488ff' }}
              />
              <Bar dataKey="volume" fill="#4488ff" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Pairs Table */}
      <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
        <h2 className="text-lg font-bold text-white mb-4">📈 Scanning {filtered.length} Pair(s)</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-x3-dark-gray">
                <th className="text-left py-2 px-3 text-gray-400 font-bold">Pair</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Price</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">24h Change</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">1h Change</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">24h Volume</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Vol Change</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Spread</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Momentum</th>
                <th className="text-right py-2 px-3 text-gray-400 font-bold">Trend</th>
                <th className="text-center py-2 px-3 text-gray-400 font-bold">Action</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((pair) => (
                <tr key={pair.pair} className="border-b border-x3-dark-gray hover:bg-x3-dark/50 transition-colors">
                  <td className="py-3 px-3 font-bold text-white">{pair.pair}</td>
                  <td className="py-3 px-3 text-right text-gray-400">${pair.price.toFixed(4)}</td>
                  <td className={`py-3 px-3 text-right font-bold ${pair.change24h > 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {pair.change24h > 0 ? '▲' : '▼'} {Math.abs(pair.change24h).toFixed(2)}%
                  </td>
                  <td className={`py-3 px-3 text-right font-bold ${pair.change1h > 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {pair.change1h > 0 ? '▲' : '▼'} {Math.abs(pair.change1h).toFixed(2)}%
                  </td>
                  <td className="py-3 px-3 text-right text-gray-400">${(pair.volume24h / 1_000_000).toFixed(2)}M</td>
                  <td className={`py-3 px-3 text-right ${pair.volumeChange > 0 ? 'text-green-400' : 'text-red-400'}`}>
                    {pair.volumeChange > 0 ? '+' : ''}{pair.volumeChange.toFixed(1)}%
                  </td>
                  <td className="py-3 px-3 text-right text-gray-400">{(pair.spread * 100).toFixed(3)}%</td>
                  <td className="py-3 px-3 text-right">
                    <div className="inline-block w-16 h-2 bg-x3-dark-gray rounded-full overflow-hidden">
                      <div
                        className="h-full bg-x3-orange transition-all"
                        style={{ width: `${pair.momentum}%` }}
                      />
                    </div>
                  </td>
                  <td className="py-3 px-3 text-right">
                    <Badge
                      variant={pair.trend === 'UP' ? 'green' : pair.trend === 'DOWN' ? 'red' : 'amber'}
                    >
                      {pair.trend === 'UP' ? '📈' : pair.trend === 'DOWN' ? '📉' : '➡️'} {pair.trend}
                    </Badge>
                  </td>
                  <td className="py-3 px-3 text-center">
                    <button
                      onClick={() => toggleWatchlist(pair.pair)}
                      className={`px-3 py-1 rounded text-xs font-bold transition-colors ${
                        watchlist.has(pair.pair)
                          ? 'bg-x3-orange text-white'
                          : 'bg-x3-dark-gray text-gray-400 hover:text-gray-300'
                      }`}
                    >
                      {watchlist.has(pair.pair) ? '⭐ Watched' : '☆ Watch'}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Alerts Section */}
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <h3 className="text-lg font-bold text-white mb-4">🔔 Top Gainers</h3>
          <div className="space-y-2">
            {topGainers.map((pair) => (
                <div key={pair.pair} className="flex justify-between items-center p-2 bg-x3-dark rounded hover:bg-x3-dark-gray transition-colors">
                  <div>
                    <div className="font-bold text-white text-sm">{pair.pair}</div>
                    <div className="text-xs text-gray-400">+{pair.change24h.toFixed(2)}% in 24h</div>
                  </div>
                  <div className="text-green-400 font-bold">${pair.price.toFixed(4)}</div>
                </div>
              ))}
          </div>
        </div>

        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
          <h3 className="text-lg font-bold text-white mb-4">🚨 Top Losers</h3>
          <div className="space-y-2">
            {topLosers.map((pair) => (
                <div key={pair.pair} className="flex justify-between items-center p-2 bg-x3-dark rounded hover:bg-x3-dark-gray transition-colors">
                  <div>
                    <div className="font-bold text-white text-sm">{pair.pair}</div>
                    <div className="text-xs text-gray-400">{pair.change24h.toFixed(2)}% in 24h</div>
                  </div>
                  <div className="text-red-400 font-bold">${pair.price.toFixed(4)}</div>
                </div>
              ))}
          </div>
        </div>
      </div>
    </div>
  );
}
