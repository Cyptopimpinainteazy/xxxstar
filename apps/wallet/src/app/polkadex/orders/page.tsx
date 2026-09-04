'use client';

import { useState } from 'react';
import type { Order } from '@/lib/polkadex/types';
import { OrderSide, OrderStatus, OrderType } from '@/lib/polkadex/types';
import { Badge, Button } from '@/components/x3/UIComponents';

const DEMO_TS = 1_700_000_120_000;

const DEMO_ORDERS: Order[] = [
  {
    id: 'order-001',
    tradingPair: 'PDEX/USDT',
    side: OrderSide.Buy,
    type: OrderType.Limit,
    status: OrderStatus.Open,
    price: 2.40,
    amount: 1000,
    filled: 245,
    createdAt: DEMO_TS - 3600000,
    updatedAt: DEMO_TS - 1800000,
    fee: 0.5,
  },
  {
    id: 'order-002',
    tradingPair: 'DOT/USDT',
    side: OrderSide.Sell,
    type: OrderType.Limit,
    status: OrderStatus.PartiallyFilled,
    price: 8.50,
    amount: 500,
    filled: 250,
    createdAt: DEMO_TS - 7200000,
    updatedAt: DEMO_TS - 600000,
    fee: 1.2,
  },
  {
    id: 'order-003',
    tradingPair: 'KSM/USDT',
    side: OrderSide.Buy,
    type: OrderType.Market,
    status: OrderStatus.Filled,
    price: 195.42,
    amount: 100,
    filled: 100,
    createdAt: DEMO_TS - 86400000,
    updatedAt: DEMO_TS - 86000000,
    fee: 2.4,
  },
];

function statusColor(status: OrderStatus): 'green' | 'amber' | 'blue' | 'muted' {
  switch (status) {
    case OrderStatus.Open:
      return 'blue';
    case OrderStatus.PartiallyFilled:
      return 'amber';
    case OrderStatus.Filled:
      return 'green';
    default:
      return 'muted';
  }
}

function sideColor(side: OrderSide): string {
  return side === OrderSide.Buy ? 'text-green-400' : 'text-red-400';
}

export default function OrdersPage() {
  const [orders] = useState<Order[]>(DEMO_ORDERS);
  const [filter, setFilter] = useState<OrderStatus | 'all'>('all');

  const filtered =
    filter === 'all' ? orders : orders.filter((o) => o.status === filter);

  const openOrders = orders.filter((o) => o.status === OrderStatus.Open);
  const filledOrders = orders.filter((o) => o.status === OrderStatus.Filled);
  const totalFees = orders.reduce((sum, o) => sum + (o.fee || 0), 0);

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-baseline gap-4 justify-between">
        <div>
          <h1 className="text-3xl font-bold">Orders</h1>
          <span className="text-gray-400">View and manage your trading orders</span>
        </div>
        <Button variant="primary" size="sm">
          + New Order
        </Button>
      </div>

      {/* Order Stats */}
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Open Orders</div>
          <div className="text-2xl font-bold text-blue-400">{openOrders.length}</div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Filled Orders</div>
          <div className="text-2xl font-bold text-green-400">{filledOrders.length}</div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Total Orders</div>
          <div className="text-2xl font-bold">{orders.length}</div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Total Fees Paid</div>
          <div className="text-2xl font-bold text-amber-400">${totalFees.toFixed(2)}</div>
        </div>
      </div>

      {/* Filter */}
      <div className="flex gap-2 flex-wrap">
        {['all', OrderStatus.Open, OrderStatus.PartiallyFilled, OrderStatus.Filled].map((s) => (
          <button
            key={s}
            onClick={() => setFilter(s as OrderStatus | 'all')}
            className={`px-4 py-2 rounded font-medium text-sm transition-colors ${
              filter === s
                ? 'bg-x3-orange text-white'
                : 'bg-x3-dark-gray text-gray-300 hover:bg-x3-dark-gray'
            }`}
          >
            {s}
          </button>
        ))}
      </div>

      {/* Orders Table */}
      <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="text-xs text-gray-400 uppercase tracking-wider border-b border-x3-dark-gray">
            <tr>
              <th className="text-left py-3">ID</th>
              <th className="text-left py-3">Pair</th>
              <th className="text-left py-3">Side</th>
              <th className="text-right py-3">Price</th>
              <th className="text-right py-3">Amount</th>
              <th className="text-right py-3">Filled</th>
              <th className="text-center py-3">Status</th>
              <th className="text-left py-3">Type</th>
              <th className="text-right py-3">Fee</th>
              <th className="text-center py-3">Actions</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((order) => (
              <tr key={order.id} className="border-b border-x3-dark hover:bg-x3-dark-gray">
                <td className="py-3 font-mono text-xs">{order.id.slice(-6)}</td>
                <td className="py-3 font-bold">{order.tradingPair}</td>
                <td className={`py-3 font-bold ${sideColor(order.side)}`}>{order.side}</td>
                <td className="py-3 text-right">${order.price.toFixed(2)}</td>
                <td className="py-3 text-right">{order.amount}</td>
                <td className="py-3 text-right">
                  <div className="text-xs">
                    {order.filled} ({((order.filled / order.amount) * 100).toFixed(1)}%)
                  </div>
                </td>
                <td className="py-3 text-center">
                  <Badge variant={statusColor(order.status)}>{order.status}</Badge>
                </td>
                <td className="py-3">{order.type}</td>
                <td className="py-3 text-right text-amber-400">${(order.fee || 0).toFixed(2)}</td>
                <td className="py-3 text-center">
                  {order.status === OrderStatus.Open && (
                    <button className="px-2 py-1 bg-red-600 hover:bg-red-700 text-white rounded text-xs font-bold">
                      Cancel
                    </button>
                  )}
                  {order.status !== OrderStatus.Open && (
                    <span className="text-gray-500 text-xs">—</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
