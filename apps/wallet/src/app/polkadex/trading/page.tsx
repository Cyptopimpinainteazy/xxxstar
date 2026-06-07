'use client';

import { useState, useEffect, useMemo } from 'react';
import type { TradingPair, OrderBook } from '@/lib/polkadex/types';
import { OrderSide, OrderType } from '@/lib/polkadex/types';
import { Button } from '@/components/x3/UIComponents';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

const DEMO_PAIRS: TradingPair[] = [
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
];

const DEMO_TS = 1_700_000_060_000;

const formatDemoTime = (timestamp: number) => {
  const date = new Date(timestamp);
  const hh = String(date.getUTCHours()).padStart(2, '0');
  const mm = String(date.getUTCMinutes()).padStart(2, '0');
  return `${hh}:${mm} UTC`;
};

const DEMO_ORDERBOOK: OrderBook = {
  pair: 'PDEX/USDT',
  timestamp: DEMO_TS,
  bids: [
    { price: 2.44, amount: 1000, total: 2440 },
    { price: 2.43, amount: 2500, total: 6075 },
    { price: 2.42, amount: 1500, total: 3630 },
    { price: 2.41, amount: 3000, total: 7230 },
    { price: 2.40, amount: 2000, total: 4800 },
  ],
  asks: [
    { price: 2.46, amount: 1200, total: 2952 },
    { price: 2.47, amount: 2000, total: 4940 },
    { price: 2.48, amount: 1800, total: 4464 },
    { price: 2.49, amount: 2500, total: 6225 },
    { price: 2.50, amount: 3000, total: 7500 },
  ],
};

export default function TradingPage() {
  const [selectedPair, setSelectedPair] = useState<TradingPair>(DEMO_PAIRS[0]);
  const [orderBook] = useState<OrderBook>(DEMO_ORDERBOOK);
  const [side, setSide] = useState<OrderSide>(OrderSide.Buy);
  const [type, setType] = useState<OrderType>(OrderType.Limit);
  const [price, setPrice] = useState<string>(selectedPair.bid.toString());
  const [amount, setAmount] = useState<string>('');
  const [chartData, setChartData] = useState<any[]>([]);

  // Generate mock chart data
  useEffect(() => {
    const now = DEMO_TS;
    const data = Array.from({ length: 24 }, (_, i) => ({
      time: formatDemoTime(now - (24 - i) * 3600000),
      price: selectedPair.lastPrice + Math.sin(i * 0.7 + selectedPair.lastPrice) * 0.1,
    }));
    setChartData(data);
  }, [selectedPair]);

  const total = useMemo(() => {
    const p = Number(price) || 0;
    const a = Number(amount) || 0;
    return (p * a).toFixed(2);
  }, [price, amount]);

  const handlePlaceOrder = () => {
    const p = Number(price);
    const a = Number(amount);
    if (!p || !a) {
      alert('Please enter price and amount');
      return;
    }
    alert(
      `Order ${side} ${a} ${selectedPair.baseAsset} @ ${p} ${selectedPair.quoteAsset}\nTotal: ${total} ${selectedPair.quoteAsset}`
    );
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-baseline gap-4">
        <h1 className="text-3xl font-bold">POLKADEX Trading</h1>
        <span className="text-gray-400">Decentralized Exchange</span>
      </div>

      {/* Main Trading Grid */}
      <div className="grid grid-cols-4 gap-4">
        {/* Left: Trading Pairs */}
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold mb-3">Markets</h2>
          <div className="space-y-2">
            {DEMO_PAIRS.map((pair) => (
              <button
                key={pair.id}
                onClick={() => {
                  setSelectedPair(pair);
                  setPrice(pair.bid.toString());
                }}
                className={`w-full p-3 rounded text-left transition-colors ${
                  selectedPair.id === pair.id
                    ? 'bg-x3-orange text-white'
                    : 'bg-x3-dark-gray hover:bg-x3-dark-gray text-gray-300'
                }`}
              >
                <div className="font-bold text-sm">{pair.symbol}</div>
                <div className="text-xs">
                  <div className="text-gray-400">{pair.lastPrice.toFixed(3)}</div>
                  <div className={pair.change24h > 0 ? 'text-green-400' : 'text-red-400'}>
                    {pair.change24h > 0 ? '+' : ''}{pair.change24h}%
                  </div>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Middle: Order Book & Chart */}
        <div className="col-span-2 space-y-4">
          {/* Chart */}
          <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
            <h2 className="text-lg font-bold mb-3">24h Chart</h2>
            <ResponsiveContainer width="100%" height={250}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
                <XAxis dataKey="time" stroke="#8a8a8e" tick={{ fontSize: 11 }} />
                <YAxis stroke="#8a8a8e" tick={{ fontSize: 11 }} domain={['auto', 'auto']} /><Tooltip
                  contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
                  labelStyle={{ color: '#e0e0e0' }}
                />
                <Line
                  type="monotone"
                  dataKey="price"
                  stroke="#00d4aa"
                  strokeWidth={2}
                  dot={false}
                  isAnimationActive={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>

          {/* Order Book */}
          <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
            <h2 className="text-lg font-bold mb-3">Order Book</h2>
            <div className="grid grid-cols-2 gap-4 text-xs">
              <div>
                <div className="font-bold text-green-400 mb-2">BIDS</div>
                {orderBook.bids.map((bid, i) => (
                  <div key={i} className="flex justify-between py-1 border-b border-x3-dark-gray">
                    <span>{bid.price.toFixed(3)}</span>
                    <span className="text-gray-400">{bid.amount}</span>
                  </div>
                ))}
              </div>
              <div>
                <div className="font-bold text-red-400 mb-2">ASKS</div>
                {orderBook.asks.map((ask, i) => (
                  <div key={i} className="flex justify-between py-1 border-b border-x3-dark-gray">
                    <span>{ask.price.toFixed(3)}</span>
                    <span className="text-gray-400">{ask.amount}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Right: Order Form */}
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray h-fit">
          <h2 className="text-lg font-bold mb-4">Place Order</h2>

          {/* Side Selector */}
          <div className="flex gap-2 mb-4">
            <button
              onClick={() => setSide(OrderSide.Buy)}
              className={`flex-1 py-2 rounded font-bold transition-colors ${
                side === OrderSide.Buy
                  ? 'bg-green-600 text-white'
                  : 'bg-x3-dark-gray text-gray-300'
              }`}
            >
              BUY
            </button>
            <button
              onClick={() => setSide(OrderSide.Sell)}
              className={`flex-1 py-2 rounded font-bold transition-colors ${
                side === OrderSide.Sell ? 'bg-red-600 text-white' : 'bg-x3-dark-gray text-gray-300'
              }`}
            >
              SELL
            </button>
          </div>

          {/* Order Type */}
          <select
            value={type}
            onChange={(e) => setType(e.target.value as OrderType)}
            className="w-full bg-x3-dark-gray text-white p-2 rounded mb-4 text-sm border border-x3-dark-gray"
          >
            <option value={OrderType.Limit}>Limit</option>
            <option value={OrderType.Market}>Market</option>
          </select>

          {/* Price Input */}
          <div className="mb-3">
            <label className="text-xs text-gray-400 block mb-1">Price (USDT)</label>
            <input
              type="number"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              placeholder="0.00"
              className="w-full bg-x3-dark-gray text-white p-2 rounded text-sm border border-x3-dark-gray"
            />
          </div>

          {/* Amount Input */}
          <div className="mb-3">
            <label className="text-xs text-gray-400 block mb-1">Amount ({selectedPair.baseAsset})</label>
            <input
              type="number"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.00"
              className="w-full bg-x3-dark-gray text-white p-2 rounded text-sm border border-x3-dark-gray"
            />
          </div>

          {/* Total */}
          <div className="bg-x3-dark-gray p-2 rounded mb-4 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-400">Total:</span>
              <span className="font-bold">{total} USDT</span>
            </div>
          </div>

          {/* Place Order Button */}
          <Button
            variant={side === OrderSide.Buy ? 'success' : 'danger'}
            onClick={handlePlaceOrder}
            className="w-full"
          >
            {side === OrderSide.Buy ? '📈 Buy' : '📉 Sell'} {selectedPair.baseAsset}
          </Button>
        </div>
      </div>
    </div>
  );
}
