'use client';

import React, { useState, useMemo } from 'react';
import { Button } from './UIComponents';

interface AdvancedOrderFormProps {
  pair: string;
  bid: number;
  ask: number;
  balance: number;
  onOrderPlace: (order: any) => void;
}

export const AdvancedOrderForm: React.FC<AdvancedOrderFormProps> = ({
  pair,
  bid,
  ask,
  balance,
  onOrderPlace,
}) => {
  const [side, setSide] = useState<'BUY' | 'SELL'>('BUY');
  const [orderType, setOrderType] = useState<'LIMIT' | 'MARKET' | 'STOP_LOSS' | 'TAKE_PROFIT' | 'TRAILING_STOP'>('LIMIT');
  const [price, setPrice] = useState(side === 'BUY' ? bid.toString() : ask.toString());
  const [amount, setAmount] = useState('');
  const [triggerPrice, setTriggerPrice] = useState('');
  const [trailingPercent, setTrailingPercent] = useState('2');
  const [useAdvanced, setUseAdvanced] = useState(false);

  const total = useMemo(() => {
    const p = Number(price) || 0;
    const a = Number(amount) || 0;
    return (p * a).toFixed(2);
  }, [price, amount]);

  const canExecute = Number(amount) > 0 && Number(price) > 0;

  const handlePlaceOrder = () => {
    if (!canExecute) {
      alert('Please enter valid price and amount');
      return;
    }

    const orderData = {
      pair,
      side,
      orderType,
      price: Number(price),
      amount: Number(amount),
      ...(orderType === 'STOP_LOSS' && { triggerPrice: Number(triggerPrice) }),
      ...(orderType === 'TRAILING_STOP' && { trailingPercent: Number(trailingPercent) }),
    };

    onOrderPlace?.(orderData);
  };

  return (
    <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-5 rounded-lg border border-x3-dark-gray backdrop-blur-sm space-y-4">
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-lg font-bold">Place Order</h2>
        <button
          onClick={() => setUseAdvanced(!useAdvanced)}
          className="text-xs bg-x3-dark-gray px-3 py-1 rounded hover:bg-x3-orange/20 transition-colors"
        >
          {useAdvanced ? '← Basic' : 'Advanced →'}
        </button>
      </div>

      {/* Side Selector */}
      <div className="flex gap-2">
        <button
          onClick={() => {
            setSide('BUY');
            setPrice(bid.toString());
          }}
          className={`flex-1 py-2 rounded font-bold transition-all ${
            side === 'BUY'
              ? 'bg-green-600 text-white shadow-lg shadow-green-600/50'
              : 'bg-x3-dark-gray text-gray-300 hover:bg-x3-dark'
          }`}
        >
          📈 BUY
        </button>
        <button
          onClick={() => {
            setSide('SELL');
            setPrice(ask.toString());
          }}
          className={`flex-1 py-2 rounded font-bold transition-all ${
            side === 'SELL'
              ? 'bg-red-600 text-white shadow-lg shadow-red-600/50'
              : 'bg-x3-dark-gray text-gray-300 hover:bg-x3-dark'
          }`}
        >
          📉 SELL
        </button>
      </div>

      {/* Order Type Selector */}
      <div className="grid grid-cols-3 gap-2">
        {(['LIMIT', 'MARKET', 'STOP_LOSS'] as const).map((type) => (
          <button
            key={type}
            onClick={() => setOrderType(type)}
            className={`py-2 px-2 rounded text-xs font-bold transition-all ${
              orderType === type
                ? 'bg-x3-orange text-white'
                : 'bg-x3-dark-gray text-gray-400 hover:text-gray-300'
            }`}
          >
            {type === 'STOP_LOSS' ? 'Stop Loss' : type}
          </button>
        ))}
      </div>

      {useAdvanced && (
        <div className="grid grid-cols-2 gap-2">
          {(['TAKE_PROFIT', 'TRAILING_STOP'] as const).map((type) => (
            <button
              key={type}
              onClick={() => setOrderType(type)}
              className={`py-2 px-2 rounded text-xs font-bold transition-all ${
                orderType === type
                  ? 'bg-purple-600 text-white'
                  : 'bg-x3-dark-gray text-gray-400 hover:text-gray-300'
              }`}
            >
              {type === 'TAKE_PROFIT' ? 'Take Profit' : 'Trailing Stop'}
            </button>
          ))}
        </div>
      )}

      {/* Price Input */}
      <div>
        <label className="text-xs text-gray-400 block mb-1">
          Price {orderType === 'MARKET' && '(Market)'}
        </label>
        <input
          type="number"
          value={price}
          onChange={(e) => setPrice(e.target.value)}
          disabled={orderType === 'MARKET'}
          placeholder="0.00"
          className={`w-full bg-x3-dark text-white p-2 rounded text-sm border border-x3-dark-gray focus:border-x3-orange outline-none transition-colors ${
            orderType === 'MARKET' ? 'opacity-50 cursor-not-allowed' : ''
          }`}
        />
      </div>

      {/* Trigger Price for Stop Loss */}
      {(orderType === 'STOP_LOSS' || orderType === 'TAKE_PROFIT') && (
        <div>
          <label className="text-xs text-gray-400 block mb-1">
            {orderType === 'STOP_LOSS' ? 'Stop Price' : 'Take Profit Price'}
          </label>
          <input
            type="number"
            value={triggerPrice}
            onChange={(e) => setTriggerPrice(e.target.value)}
            placeholder="0.00"
            className="w-full bg-x3-dark text-white p-2 rounded text-sm border border-x3-dark-gray focus:border-x3-orange outline-none transition-colors"
          />
        </div>
      )}

      {/* Trailing Percent */}
      {orderType === 'TRAILING_STOP' && (
        <div>
          <label className="text-xs text-gray-400 block mb-1">Trailing Percent (%)</label>
          <input
            type="number"
            value={trailingPercent}
            onChange={(e) => setTrailingPercent(e.target.value)}
            step="0.1"
            min="0.1"
            placeholder="2.0"
            className="w-full bg-x3-dark text-white p-2 rounded text-sm border border-x3-dark-gray focus:border-x3-orange outline-none transition-colors"
          />
        </div>
      )}

      {/* Amount Input */}
      <div>
        <label className="text-xs text-gray-400 block mb-1">
          Amount ({pair.split('/')[0]})
        </label>
        <input
          type="number"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          placeholder="0.00"
          className="w-full bg-x3-dark text-white p-2 rounded text-sm border border-x3-dark-gray focus:border-x3-orange outline-none transition-colors"
        />
      </div>

      {/* Quick Buttons */}
      <div className="flex gap-2 text-xs">
        {[25, 50, 75, 100].map((percent) => (
          <button
            key={percent}
            onClick={() =>
              setAmount(
                String(
                  ((balance / (Number(price) || 1)) * percent) / 100 || 0
                )
              )
            }
            className="flex-1 bg-x3-dark-gray hover:bg-x3-orange/20 py-1 rounded transition-colors"
          >
            {percent}%
          </button>
        ))}
      </div>

      {/* Total */}
      <div className="bg-x3-dark p-3 rounded border border-x3-dark-gray">
        <div className="flex justify-between text-sm">
          <span className="text-gray-400">Total:</span>
          <span className="font-bold text-x3-orange">{total} USDT</span>
        </div>
        <div className="flex justify-between text-xs text-gray-500 mt-1">
          <span>Balance:</span>
          <span>${balance.toFixed(2)}</span>
        </div>
      </div>

      {/* Place Order Button */}
      <Button
        variant={side === 'BUY' ? 'success' : 'danger'}
        onClick={handlePlaceOrder}
        disabled={!canExecute}
        className="w-full font-bold"
      >
        {side === 'BUY' ? '📈 Place Buy Order' : '📉 Place Sell Order'}
        {orderType !== 'LIMIT' && ` (${orderType})`}
      </Button>

      {/* Fee Info */}
      <div className="text-xs text-gray-500 text-center">
        <p>Trading Fee: 0.1% | Slippage: {(Number(price) * 0.01).toFixed(4)}</p>
      </div>
    </div>
  );
};
