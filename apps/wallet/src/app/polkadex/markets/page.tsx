'use client';

import { useState } from 'react';
import type { TradingPair } from '@/lib/polkadex/types';
import Link from 'next/link';

const DEMO_MARKETS: TradingPair[] = [
  {
    id: 'PDEX/USDT',
    baseAsset: 'PDEX',
    quoteAsset: 'USDT',
    symbol: 'PDEX/USDT',
    lastPrice: 2.45,
    change24h: 12.5,
    volume24h: 1_234_567,
    high24h: 2.58,
    low24h: 2.12,
    ask: 2.46,
    bid: 2.44,
  },
  {
    id: 'DOT/USDT',
    baseAsset: 'DOT',
    quoteAsset: 'USDT',
    symbol: 'DOT/USDT',
    lastPrice: 8.32,
    change24h: 5.2,
    volume24h: 5_678_900,
    high24h: 8.55,
    low24h: 7.80,
    ask: 8.33,
    bid: 8.31,
  },
  {
    id: 'KSM/USDT',
    baseAsset: 'KSM',
    quoteAsset: 'USDT',
    symbol: 'KSM/USDT',
    lastPrice: 195.42,
    change24h: -3.1,
    volume24h: 432_100,
    high24h: 205.00,
    low24h: 190.00,
    ask: 195.5,
    bid: 195.3,
  },
  {
    id: 'AUSD/USDT',
    baseAsset: 'AUSD',
    quoteAsset: 'USDT',
    symbol: 'AUSD/USDT',
    lastPrice: 1.001,
    change24h: 0.1,
    volume24h: 234_567,
    high24h: 1.01,
    low24h: 0.99,
    ask: 1.002,
    bid: 1.0,
  },
  {
    id: 'LRNA/USDT',
    baseAsset: 'LRNA',
    quoteAsset: 'USDT',
    symbol: 'LRNA/USDT',
    lastPrice: 0.156,
    change24h: -8.4,
    volume24h: 89_234,
    high24h: 0.175,
    low24h: 0.145,
    ask: 0.157,
    bid: 0.155,
  },
];

export default function MarketsPage() {
  const [sortBy, setSortBy] = useState<'price' | 'change' | 'volume'>('volume');

  const sortedMarkets = [...DEMO_MARKETS].sort((a, b) => {
    if (sortBy === 'price') return b.lastPrice - a.lastPrice;
    if (sortBy === 'change') return b.change24h - a.change24h;
    return b.volume24h - a.volume24h;
  });

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-baseline gap-4 justify-between">
        <div>
          <h1 className="text-3xl font-bold">Markets</h1>
          <span className="text-gray-400">All trading pairs on POLKADEX</span>
        </div>
        <div className="flex gap-2">
          {(['volume', 'change', 'price'] as const).map((sort) => (
            <button
              key={sort}
              onClick={() => setSortBy(sort)}
              className={`px-4 py-2 rounded font-medium text-sm transition-colors ${
                sortBy === sort
                  ? 'bg-x3-orange text-white'
                  : 'bg-x3-dark-gray text-gray-300 hover:bg-x3-dark-gray'
              }`}
            >
              {sort === 'volume' ? '📊' : sort === 'change' ? '📈' : '💰'} {sort}
            </button>
          ))}
        </div>
      </div>

      {/* Market Stats */}
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Total Markets</div>
          <div className="text-2xl font-bold">{DEMO_MARKETS.length}</div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">24h Volume</div>
          <div className="text-2xl font-bold">
            ${(
              DEMO_MARKETS.reduce((s, m) => s + m.volume24h, 0) / 1_000_000
            ).toFixed(1)}M
          </div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Top Gainer</div>
          <div className="text-lg font-bold text-green-400">
            {Math.max(...DEMO_MARKETS.map((m) => m.change24h)).toFixed(2)}%
          </div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Top Loser</div>
          <div className="text-lg font-bold text-red-400">
            {Math.min(...DEMO_MARKETS.map((m) => m.change24h)).toFixed(2)}%
          </div>
        </div>
      </div>

      {/* Markets Table */}
      <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="text-xs text-gray-400 uppercase tracking-wider border-b border-x3-dark-gray">
            <tr>
              <th className="text-left py-3">Pair</th>
              <th className="text-right py-3">Last Price</th>
              <th className="text-right py-3">24h Change</th>
              <th className="text-right py-3">24h High</th>
              <th className="text-right py-3">24h Low</th>
              <th className="text-right py-3">24h Volume</th>
              <th className="text-right py-3">Bid / Ask</th>
              <th className="text-center py-3">Action</th>
            </tr>
          </thead>
          <tbody>
            {sortedMarkets.map((market) => (
              <tr key={market.id} className="border-b border-x3-dark hover:bg-x3-dark-gray transition-colors">
                <td className="py-3 font-bold">{market.symbol}</td>
                <td className="py-3 text-right">${market.lastPrice.toFixed(3)}</td>
                <td className={`py-3 text-right font-bold ${market.change24h > 0 ? 'text-green-400' : 'text-red-400'}`}>
                  {market.change24h > 0 ? '+' : ''}{market.change24h.toFixed(2)}%
                </td>
                <td className="py-3 text-right">${market.high24h.toFixed(3)}</td>
                <td className="py-3 text-right">${market.low24h.toFixed(3)}</td>
                <td className="py-3 text-right">${(market.volume24h / 1_000_000).toFixed(2)}M</td>
                <td className="py-3 text-right text-xs text-gray-400">
                  {market.bid.toFixed(3)} / {market.ask.toFixed(3)}
                </td>
                <td className="py-3 text-center">
                  <Link
                    href="/polkadex/trading"
                    className="px-3 py-1 bg-x3-orange hover:bg-orange-600 rounded text-xs font-bold transition-colors"
                  >
                    Trade
                  </Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
