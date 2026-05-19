'use client';

import React from 'react';
import { Button } from './UIComponents';

export type Order = { price: number; size: number };

export function PriceSparkline({ data, color = '#00d4aa' }: { data: number[]; color?: string }) {
  const w = 160;
  const h = 40;
  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const points = data.map((v, i) => {
    const x = (i / Math.max(1, data.length - 1)) * w;
    const y = h - ((v - min) / (max - min || 1)) * h;
    return `${x},${y}`;
  });
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`}>
      <polyline
        fill="none"
        stroke={color}
        strokeWidth={2}
        points={points.join(' ')}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function OrderBook({ bids, asks }: { bids: Order[]; asks: Order[] }) {
  return (
    <div className="grid grid-cols-3 gap-3 text-sm">
      <div>
        <div className="font-bold text-xs text-green-400 mb-2">BIDS</div>
        <div className="space-y-1">
          {bids.map((b, i) => (
            <div key={i} className="flex justify-between px-2 py-1 bg-x3-dark-gray rounded text-green-500">
              <div>{b.price.toFixed(2)}</div>
              <div className="font-mono text-xs">{b.size}</div>
            </div>
          ))}
        </div>
      </div>

      <div className="flex flex-col justify-center items-center">
        <div className="text-xs text-gray-500 mb-1">SPREAD</div>
        <div className="text-xs font-mono">{asks[0]?.price.toFixed(2) ?? '—'} / {bids[0]?.price.toFixed(2) ?? '—'}</div>
      </div>

      <div>
        <div className="font-bold text-xs text-red-400 mb-2">ASKS</div>
        <div className="space-y-1">
          {asks.map((a, i) => (
            <div key={i} className="flex justify-between px-2 py-1 bg-x3-dark-gray rounded text-red-500">
              <div>{a.price.toFixed(2)}</div>
              <div className="font-mono text-xs">{a.size}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function FlashloanPanel({ providers, enabled, onToggle }: { providers: string[]; enabled: Record<string, boolean>; onToggle: (p: string) => void }) {
  return (
    <div className="space-y-2">
      <div className="text-sm font-bold">Flashloan Providers</div>
      <div className="flex flex-wrap gap-2">
        {providers.map((p) => (
          <Button key={p} variant={enabled[p] ? 'success' : 'secondary'} size="sm" onClick={() => onToggle(p)}>
            {enabled[p] ? '✓' : '○'} {p}
          </Button>
        ))}
      </div>
    </div>
  );
}

export function PriceDifference({ priceA, priceB }: { priceA: number; priceB: number }) {
  const diff = priceA - priceB;
  const pct = (diff / ((priceA + priceB) / 2)) * 100;
  const isPositive = diff > 0;

  return (
    <div className={`p-3 rounded ${isPositive ? 'bg-green-900 text-green-300' : diff < 0 ? 'bg-red-900 text-red-300' : 'bg-gray-700 text-gray-300'}`}>
      <div className="text-sm font-bold">{Math.abs(diff).toFixed(2)} USDC</div>
      <div className="text-xs opacity-75">{pct.toFixed(2)}%</div>
    </div>
  );
}

export function generateOrderBook(mid: number, depth = 8, step = 0.25) {
  // Use integer arithmetic only to keep SSR/CSR outputs identical.
  const deterministicSize = (seed: number) => {
    const n = Math.abs(Math.round(seed));
    return ((n * 1103515245 + 12345) >>> 0) % 50 + 1;
  };
  const bids: Order[] = [];
  const asks: Order[] = [];
  for (let i = depth; i >= 1; i--) {
    const seed = mid * 1000 + i * 17;
    bids.push({ price: +(mid - i * step).toFixed(4), size: deterministicSize(seed) });
  }
  for (let i = 1; i <= depth; i++) {
    const seed = mid * 1000 + i * 31;
    asks.push({ price: +(mid + i * step).toFixed(4), size: deterministicSize(seed) });
  }
  return { bids, asks };
}
