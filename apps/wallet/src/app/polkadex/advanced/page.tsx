'use client';

import { useState, useEffect } from 'react';
import { CandlestickChart } from '@/components/x3/AdvancedChart';
import { AdvancedOrderForm } from '@/components/x3/AdvancedOrderForm';
import { Badge } from '@/components/x3/UIComponents';
import toast from 'react-hot-toast';

interface TradingPair {
  id: string;
  symbol: string;
  lastPrice: number;
  change24h: number;
  volume24h: number;
  high24h: number;
  low24h: number;
  bid: number;
  ask: number;
}

interface CandleData {
  time: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

const DEMO_PAIRS: TradingPair[] = [
  {
    id: 'PDEX/USDT',
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
    id: 'BTC/USDT',
    symbol: 'BTC/USDT',
    lastPrice: 87234.50,
    change24h: 8.2,
    volume24h: 12_456_789,
    high24h: 89000,
    low24h: 85000,
    ask: 87245,
    bid: 87224,
  },
  {
    id: 'ETH/USDT',
    symbol: 'ETH/USDT',
    lastPrice: 4562.30,
    change24h: 6.8,
    volume24h: 8_765_432,
    high24h: 4700,
    low24h: 4300,
    ask: 4563,
    bid: 4562,
  },
];

const DEMO_TS = 1_700_000_120_000;

const formatDemoTime = (timestamp: number) => {
  const date = new Date(timestamp);
  const hh = String(date.getUTCHours()).padStart(2, '0');
  const mm = String(date.getUTCMinutes()).padStart(2, '0');
  return `${hh}:${mm} UTC`;
};

const generateCandleData = (): CandleData[] => {
  const now = DEMO_TS;
  return Array.from({ length: 48 }, (_, i) => {
    const basePrice = 2.45 + Math.sin(i / 4) * 0.04;
    const close = basePrice + Math.cos(i / 5) * 0.03;
    const high = Math.max(basePrice, close) + 0.05 + (i % 3) * 0.005;
    const low = Math.min(basePrice, close) - 0.05 - (i % 2) * 0.004;

    return {
      time: formatDemoTime(now - (48 - i) * 1800000),
      open: basePrice,
      high,
      low,
      close,
      volume: 800 + ((i * 173) % 4200),
    };
  });
};

export default function AdvancedTradingPage() {
  const [selectedPair, setSelectedPair] = useState<TradingPair>(DEMO_PAIRS[0]);
  const [candleData, setCandleData] = useState<CandleData[]>([]);
  const [indicator, setIndicator] = useState<'SMA' | 'EMA' | 'RSI' | 'MACD' | 'BOLLINGER'>('SMA');
  const [timeframe, setTimeframe] = useState<'1H' | '4H' | '1D' | '1W'>('1H');
  const [openOrders, setOpenOrders] = useState([
    {
      id: '1',
      pair: 'PDEX/USDT',
      side: 'BUY',
      price: 2.40,
      amount: 1000,
      filled: 250,
      orderType: 'LIMIT',
      timestamp: DEMO_TS - 300000,
    },
  ]);
  const [recentTrades] = useState([
    {
      id: '1',
      pair: 'PDEX/USDT',
      side: 'BUY',
      price: 2.43,
      amount: 500,
      timestamp: DEMO_TS - 600000,
    },
    {
      id: '2',
      pair: 'DOT/USDT',
      side: 'SELL',
      price: 8.40,
      amount: 10,
      timestamp: DEMO_TS - 1200000,
    },
  ]);

  useEffect(() => {
    setCandleData(generateCandleData());
  }, [selectedPair, timeframe]);

  const handlePlaceOrder = (orderData: any) => {
    const nextTimestamp = DEMO_TS + (openOrders.length + 1) * 60_000;
    setOpenOrders([
      {
        id: `order-${nextTimestamp}`,
        ...orderData,
        filled: 0,
        timestamp: nextTimestamp,
      },
      ...openOrders,
    ]);
    toast.success(`${orderData.side} order placed for ${orderData.amount} ${orderData.pair}`, {
      duration: 3000,
    });
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-x3-dark via-x3-dark to-[#0f0f13] p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-4xl font-bold text-white">Advanced Trading</h1>
          <p className="text-gray-400 mt-1">Professional trading with advanced order types</p>
        </div>
        <div className="flex gap-2">
          <Badge variant="green">🟢 Connected</Badge>
          <Badge>Balance: $50,000</Badge>
        </div>
      </div>

      {/* Timeframe Selector */}
      <div className="flex gap-2 mb-4">
        {(['1H', '4H', '1D', '1W'] as const).map((tf) => (
          <button
            key={tf}
            onClick={() => setTimeframe(tf)}
            className={`px-4 py-2 rounded font-bold transition-all ${
              timeframe === tf
                ? 'bg-x3-orange text-white'
                : 'bg-x3-dark border border-x3-dark-gray hover:border-x3-orange text-gray-400'
            }`}
          >
            {tf}
          </button>
        ))}
      </div>

      {/* Main Trading Grid */}
      <div className="grid grid-cols-12 gap-6">
        {/* Left Panel: Markets */}
        <div className="col-span-2 space-y-4">
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm space-y-2">
            <h2 className="text-lg font-bold text-white mb-3">📊 Markets</h2>
            <div className="space-y-2 max-h-96 overflow-y-auto">
              {DEMO_PAIRS.map((pair) => (
                <button
                  key={pair.id}
                  onClick={() => setSelectedPair(pair)}
                  className={`w-full p-3 rounded-lg transition-all text-left ${
                    selectedPair.id === pair.id
                      ? 'bg-x3-orange/20 border border-x3-orange'
                      : 'bg-x3-dark border border-x3-dark-gray hover:border-x3-orange'
                  }`}
                >
                  <div className="font-bold text-sm">{pair.symbol}</div>
                  <div className="flex justify-between text-xs mt-1">
                    <span className="text-x3-orange">${pair.lastPrice.toFixed(4)}</span>
                    <span className={pair.change24h > 0 ? 'text-green-400' : 'text-red-400'}>
                      {pair.change24h > 0 ? '▲' : '▼'} {Math.abs(pair.change24h)}%
                    </span>
                  </div>
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Center Panel: Charts & Order Book */}
        <div className="col-span-7 space-y-4">
          {/* Chart */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
            <div className="flex justify-between items-center mb-3">
              <h2 className="text-lg font-bold text-white flex items-center gap-2">
                <span>📈</span> {selectedPair.symbol} - {timeframe}
                <span className="text-2xl text-x3-orange font-bold">${selectedPair.lastPrice.toFixed(4)}</span>
              </h2>
              <div className="flex gap-1">
                {(['SMA', 'EMA', 'RSI', 'MACD', 'BOLLINGER'] as const).map((ind) => (
                  <button
                    key={ind}
                    onClick={() => setIndicator(ind)}
                    className={`px-3 py-1 rounded text-xs font-bold transition-colors ${
                      indicator === ind
                        ? 'bg-x3-orange text-white'
                        : 'bg-x3-dark-gray text-gray-400 hover:text-gray-300'
                    }`}
                  >
                    {ind}
                  </button>
                ))}
              </div>
            </div>
            <CandlestickChart data={candleData} height={350} indicator={indicator} />
          </div>

          {/* Order Book & Trades */}
          <div className="grid grid-cols-2 gap-4">
            {/* Order Book */}
            <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
              <h3 className="text-lg font-bold mb-3 text-white">Order Book</h3>
              <div className="space-y-2 text-sm">
                <div>
                  <div className="font-bold text-green-400 mb-2">BIDS</div>
                  {[2.44, 2.43, 2.42, 2.41, 2.40].map((price, i) => (
                    <div key={i} className="flex justify-between py-1 hover:bg-x3-dark-gray px-2 rounded cursor-pointer">
                      <span className="text-green-400">{price.toFixed(4)}</span>
                      <span className="text-gray-400">{(1000 + i * 500).toLocaleString()}</span>
                    </div>
                  ))}
                </div>
                <div className="py-2 border-t border-x3-dark-gray text-center text-gray-500 text-xs">
                  Spread: 0.02 USDT
                </div>
                <div>
                  <div className="font-bold text-red-400 mb-2">ASKS</div>
                  {[2.46, 2.47, 2.48, 2.49, 2.50].map((price, i) => (
                    <div key={i} className="flex justify-between py-1 hover:bg-x3-dark-gray px-2 rounded cursor-pointer">
                      <span className="text-red-400">{price.toFixed(4)}</span>
                      <span className="text-gray-400">{(1200 + i * 500).toLocaleString()}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Recent Trades */}
            <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
              <h3 className="text-lg font-bold mb-3 text-white">Recent Trades</h3>
              <div className="space-y-2 text-sm">
                {recentTrades.slice(0, 6).map((trade) => (
                  <div key={trade.id} className="flex justify-between items-center py-2 px-2 hover:bg-x3-dark-gray rounded">
                    <div className="flex-1">
                      <div className="font-bold">{trade.pair}</div>
                      <div className="text-xs text-gray-400">
                        {formatDemoTime(trade.timestamp)}
                      </div>
                    </div>
                    <div className={`font-bold ${trade.side === 'BUY' ? 'text-green-400' : 'text-red-400'}`}>
                      {trade.side === 'BUY' ? '▲' : '▼'} {trade.amount}
                    </div>
                    <div className="text-x3-orange font-bold">${(trade.price * trade.amount).toFixed(2)}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Right Panel: Order Form */}
        <div className="col-span-3 space-y-4">
          <AdvancedOrderForm
            pair={selectedPair.symbol}
            bid={selectedPair.bid}
            ask={selectedPair.ask}
            balance={50000}
            onOrderPlace={handlePlaceOrder}
          />

          {/* Open Orders */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
            <h3 className="text-lg font-bold mb-3 text-white">Open Orders ({openOrders.length})</h3>
            <div className="space-y-2 text-sm max-h-64 overflow-y-auto">
              {openOrders.length === 0 ? (
                <div className="text-gray-500 py-4 text-center">No open orders</div>
              ) : (
                openOrders.map((order) => (
                  <div
                    key={order.id}
                    className="p-2 bg-x3-dark border border-x3-dark-gray rounded hover:border-x3-orange transition-colors"
                  >
                    <div className="flex justify-between items-start mb-1">
                      <div className="font-bold text-sm">{order.pair}</div>
                      <Badge variant={order.side === 'BUY' ? 'green' : 'red'}>{order.side}</Badge>
                    </div>
                    <div className="text-xs text-gray-400 space-y-1">
                      <div className="flex justify-between">
                        <span>Price:</span>
                        <span className="text-white">${order.price.toFixed(4)}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Amount:</span>
                        <span className="text-white">{order.amount}</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Filled:</span>
                        <span className="text-white">{order.filled} ({Math.round((order.filled / order.amount) * 100)}%)</span>
                      </div>
                    </div>
                    <button className="mt-2 w-full py-1 text-xs bg-red-600/20 hover:bg-red-600/40 text-red-400 rounded transition-colors">
                      Cancel
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Stats */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
            <h3 className="text-lg font-bold mb-3 text-white">Stats</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between py-1 border-b border-x3-dark-gray">
                <span className="text-gray-400">24h High:</span>
                <span className="font-bold text-white">${selectedPair.high24h.toFixed(4)}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-x3-dark-gray">
                <span className="text-gray-400">24h Low:</span>
                <span className="font-bold text-white">${selectedPair.low24h.toFixed(4)}</span>
              </div>
              <div className="flex justify-between py-1">
                <span className="text-gray-400">24h Volume:</span>
                <span className="font-bold text-white">${(selectedPair.volume24h / 1_000_000).toFixed(2)}M</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
