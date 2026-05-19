'use client';

import { useMemo } from 'react';
import type { Balance } from '@/lib/polkadex/types';
import { Button, ProgressBar } from '@/components/x3/UIComponents';
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
} from 'recharts';

const DEMO_BALANCES: Balance[] = [
  { asset: 'USDT', free: 5000, locked: 1000, total: 6000, valueUSDT: 6000 },
  { asset: 'PDEX', free: 2000, locked: 500, total: 2500, valueUSDT: 6125 },
  { asset: 'DOT', free: 50, locked: 10, total: 60, valueUSDT: 499.2 },
  { asset: 'KSM', free: 2, locked: 0, total: 2, valueUSDT: 390.84 },
];

export default function PortfolioPage() {
  const totalValue = useMemo(() => DEMO_BALANCES.reduce((sum, b) => sum + (b.valueUSDT || 0), 0), []);

  const chartData = DEMO_BALANCES.map((b) => ({
    name: b.asset,
    value: b.valueUSDT || 0,
  }));

  const colors = ['#00d4aa', '#4488ff', '#ffaa00', '#ff4444'];

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-baseline gap-4 justify-between">
        <div>
          <h1 className="text-3xl font-bold">Portfolio</h1>
          <span className="text-gray-400">Your asset balances and distribution</span>
        </div>
        <div className="flex gap-2">
          <Button variant="primary" size="sm">
            + Deposit
          </Button>
          <Button variant="secondary" size="sm">
            → Withdraw
          </Button>
        </div>
      </div>

      {/* Portfolio Summary */}
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Total Portfolio Value</div>
          <div className="text-3xl font-bold text-x3-orange">${totalValue.toFixed(2)}</div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Available Balance</div>
          <div className="text-2xl font-bold">
            ${DEMO_BALANCES.filter((b) => b.asset === 'USDT')[0]?.free.toFixed(2)}
          </div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Locked in Orders</div>
          <div className="text-2xl font-bold text-amber-400">
            ${DEMO_BALANCES.filter((b) => b.asset === 'USDT')[0]?.locked.toFixed(2)}
          </div>
        </div>
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <div className="text-xs text-gray-400 mb-2">Total Assets</div>
          <div className="text-2xl font-bold">{DEMO_BALANCES.length}</div>
        </div>
      </div>

      {/* Asset Distribution & Balances */}
      <div className="grid grid-cols-3 gap-4">
        {/* Pie Chart */}
        <div className="col-span-1 bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold mb-4">Distribution</h2>
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={chartData}
                cx="50%"
                cy="50%"
                labelLine={false}
                label={({ name, value }: any) => `${name}: $${value.toFixed(0)}`}
                outerRadius={80}
                fill="#8884d8"
                dataKey="value"
              >
                {chartData.map((_, index) => (
                  <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
                ))}
              </Pie>
            </PieChart>
          </ResponsiveContainer>
        </div>

        {/* Balances Table */}
        <div className="col-span-2 bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold mb-4">Balances</h2>
          <div className="space-y-4">
            {DEMO_BALANCES.map((balance) => (
              <div key={balance.asset} className="pb-4 border-b border-x3-dark-gray last:border-b-0 last:pb-0">
                <div className="flex justify-between mb-2">
                  <div className="font-bold">{balance.asset}</div>
                  <div className="text-x3-orange">${(balance.valueUSDT || 0).toFixed(2)}</div>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs mb-2">
                  <div className="text-gray-400">
                    Free: <span className="text-green-400">{balance.free}</span>
                  </div>
                  <div className="text-gray-400">
                    Locked: <span className="text-amber-400">{balance.locked}</span>
                  </div>
                </div>
                <ProgressBar
                  value={balance.free}
                  max={balance.total}
                  color="green"
                />
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Recent Transactions */}
      <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
        <h2 className="text-lg font-bold mb-4">Recent Activity</h2>
        <div className="text-center text-gray-400 py-8">
          <p>No recent deposits or withdrawals</p>
        </div>
      </div>
    </div>
  );
}
