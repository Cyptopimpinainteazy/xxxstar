'use client';

import { useState } from 'react';
import { Clock, X } from 'lucide-react';

interface LimitOrderInterfaceProps {
  walletConnected: boolean;
  walletId: string;
}

const TOKENS = [
  { symbol: 'X3', address: '0x' + '2'.repeat(64), decimals: 18 },
  { symbol: 'USDC', address: '0x' + '3'.repeat(64), decimals: 6 },
  { symbol: 'BTC', address: '0x' + '4'.repeat(64), decimals: 8 },
  { symbol: 'ETH', address: '0x' + '5'.repeat(64), decimals: 18 },
];

interface LimitOrder {
  id: string;
  type: 'buy' | 'sell';
  tokenIn: string;
  tokenOut: string;
  amountIn: string;
  limitPrice: string;
  filled: string;
  status: 'active' | 'filled' | 'cancelled';
  createdAt: Date;
  expiresAt: Date;
}

export function LimitOrderInterface({ walletConnected, walletId }: LimitOrderInterfaceProps) {
  const [orderType, setOrderType] = useState<'buy' | 'sell'>('buy');
  const [tokenIn, setTokenIn] = useState(TOKENS[0]);
  const [tokenOut, setTokenOut] = useState(TOKENS[1]);
  const [amountIn, setAmountIn] = useState('');
  const [limitPrice, setLimitPrice] = useState('');
  const [expiry, setExpiry] = useState('24'); // hours
  const [placing, setPlacing] = useState(false);
  const [orders, setOrders] = useState<LimitOrder[]>([]);

  const handlePlaceOrder = async () => {
    if (!walletConnected) {
      alert('Please connect your wallet first');
      return;
    }

    if (!amountIn || !limitPrice) {
      alert('Please fill in all fields');
      return;
    }

    setPlacing(true);
    try {
      // Call limit order RPC (to be implemented)
      const order: LimitOrder = {
        id: '0x' + Math.random().toString(16).slice(2),
        type: orderType,
        tokenIn: tokenIn.address,
        tokenOut: tokenOut.address,
        amountIn,
        limitPrice,
        filled: '0',
        status: 'active',
        createdAt: new Date(),
        expiresAt: new Date(Date.now() + parseInt(expiry) * 60 * 60 * 1000),
      };

      // Mock order placement
      await new Promise(resolve => setTimeout(resolve, 1500));
      setOrders([order, ...orders]);
      
      // Reset form
      setAmountIn('');
      setLimitPrice('');
      
      alert(`${orderType.toUpperCase()} order placed successfully!`);
    } catch (error) {
      console.error('Failed to place order:', error);
      alert('Order placement failed. Please try again.');
    } finally {
      setPlacing(false);
    }
  };

  const handleCancelOrder = async (orderId: string) => {
    try {
      // Call cancel order RPC (to be implemented)
      await new Promise(resolve => setTimeout(resolve, 1000));
      setOrders(orders.filter(o => o.id !== orderId));
    } catch (error) {
      console.error('Failed to cancel order:', error);
      alert('Order cancellation failed.');
    }
  };

  return (
    <div className="space-y-6">
      {/* Order Placement Form */}
      <div className="bg-gray-800 rounded-2xl p-6 border border-gray-700">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-bold">Limit Order</h2>
          <div className="flex gap-2">
            <button
              onClick={() => setOrderType('buy')}
              className={`px-4 py-2 rounded-lg font-semibold transition ${
                orderType === 'buy'
                  ? 'bg-green-600 text-white'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
              }`}
            >
              Buy
            </button>
            <button
              onClick={() => setOrderType('sell')}
              className={`px-4 py-2 rounded-lg font-semibold transition ${
                orderType === 'sell'
                  ? 'bg-red-600 text-white'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
              }`}
            >
              Sell
            </button>
          </div>
        </div>

        {/* Token Selection */}
        <div className="grid grid-cols-2 gap-4 mb-4">
          <div>
            <label className="text-sm text-gray-400 mb-2 block">You pay</label>
            <select
              value={tokenIn.symbol}
              onChange={(e) => setTokenIn(TOKENS.find(t => t.symbol === e.target.value)!)}
              className="w-full bg-gray-900 px-4 py-3 rounded-lg font-semibold border border-gray-700 focus:border-blue-500 focus:outline-none"
            >
              {TOKENS.map(token => (
                <option key={token.symbol} value={token.symbol}>
                  {token.symbol}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="text-sm text-gray-400 mb-2 block">You receive</label>
            <select
              value={tokenOut.symbol}
              onChange={(e) => setTokenOut(TOKENS.find(t => t.symbol === e.target.value)!)}
              className="w-full bg-gray-900 px-4 py-3 rounded-lg font-semibold border border-gray-700 focus:border-blue-500 focus:outline-none"
            >
              {TOKENS.map(token => (
                <option key={token.symbol} value={token.symbol}>
                  {token.symbol}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Amount and Limit Price */}
        <div className="space-y-4 mb-4">
          <div>
            <label className="text-sm text-gray-400 mb-2 block">Amount ({tokenIn.symbol})</label>
            <input
              type="number"
              value={amountIn}
              onChange={(e) => setAmountIn(e.target.value)}
              placeholder="0.0"
              className="w-full bg-gray-900 px-4 py-3 rounded-lg text-lg font-semibold border border-gray-700 focus:border-blue-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="text-sm text-gray-400 mb-2 block">
              Limit Price ({tokenOut.symbol} per {tokenIn.symbol})
            </label>
            <input
              type="number"
              value={limitPrice}
              onChange={(e) => setLimitPrice(e.target.value)}
              placeholder="0.0"
              className="w-full bg-gray-900 px-4 py-3 rounded-lg text-lg font-semibold border border-gray-700 focus:border-blue-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="text-sm text-gray-400 mb-2 block">Expiry (hours)</label>
            <select
              value={expiry}
              onChange={(e) => setExpiry(e.target.value)}
              className="w-full bg-gray-900 px-4 py-3 rounded-lg font-semibold border border-gray-700 focus:border-blue-500 focus:outline-none"
            >
              <option value="1">1 hour</option>
              <option value="6">6 hours</option>
              <option value="24">24 hours</option>
              <option value="72">3 days</option>
              <option value="168">7 days</option>
            </select>
          </div>
        </div>

        {/* Order Summary */}
        {amountIn && limitPrice && (
          <div className="bg-gray-900 rounded-xl p-3 mb-4 text-sm">
            <div className="flex justify-between mb-1">
              <span className="text-gray-400">Total {tokenOut.symbol}</span>
              <span>{(parseFloat(amountIn) * parseFloat(limitPrice)).toFixed(6)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Order Type</span>
              <span className={orderType === 'buy' ? 'text-green-400' : 'text-red-400'}>
                {orderType.toUpperCase()}
              </span>
            </div>
          </div>
        )}

        {/* Place Order Button */}
        <button
          onClick={handlePlaceOrder}
          disabled={!walletConnected || placing || !amountIn || !limitPrice}
          className={`w-full py-3 rounded-lg font-bold text-lg transition disabled:opacity-50 disabled:cursor-not-allowed ${
            orderType === 'buy'
              ? 'bg-gradient-to-r from-green-600 to-green-700 hover:from-green-700 hover:to-green-800'
              : 'bg-gradient-to-r from-red-600 to-red-700 hover:from-red-700 hover:to-red-800'
          }`}
        >
          {placing ? 'Placing Order...' : `Place ${orderType.toUpperCase()} Order`}
        </button>
      </div>

      {/* Active Orders */}
      {orders.length > 0 && (
        <div className="bg-gray-800 rounded-2xl p-6 border border-gray-700">
          <h3 className="text-lg font-bold mb-4">Your Orders</h3>
          <div className="space-y-3">
            {orders.map((order) => (
              <div key={order.id} className="bg-gray-900 rounded-lg p-4 border border-gray-700">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <span className={`px-2 py-1 rounded text-xs font-bold ${
                      order.type === 'buy' ? 'bg-green-600/20 text-green-400' : 'bg-red-600/20 text-red-400'
                    }`}>
                      {order.type.toUpperCase()}
                    </span>
                    <span className="font-mono text-sm">
                      {TOKENS.find(t => t.address === order.tokenIn)?.symbol} → {TOKENS.find(t => t.address === order.tokenOut)?.symbol}
                    </span>
                  </div>
                  <button
                    onClick={() => handleCancelOrder(order.id)}
                    className="text-gray-400 hover:text-red-400 transition"
                  >
                    <X className="w-5 h-5" />
                  </button>
                </div>
                <div className="grid grid-cols-3 gap-4 text-sm">
                  <div>
                    <div className="text-gray-400">Amount</div>
                    <div className="font-semibold">{order.amountIn}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Limit Price</div>
                    <div className="font-semibold">{order.limitPrice}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Filled</div>
                    <div className="font-semibold">{order.filled}%</div>
                  </div>
                </div>
                <div className="flex items-center gap-2 mt-3 text-xs text-gray-500">
                  <Clock className="w-4 h-4" />
                  <span>Expires {order.expiresAt.toLocaleString()}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
