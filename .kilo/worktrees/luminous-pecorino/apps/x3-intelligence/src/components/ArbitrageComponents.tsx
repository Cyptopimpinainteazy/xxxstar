import React, { useEffect, useMemo, useState } from "react";
import { Button, Badge } from "./UIComponents";

export type Order = { price: number; size: number };

export function PriceSparkline({ data, color = "#00d4aa" }: { data: number[]; color?: string }) {
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
        points={points.join(" ")}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function OrderBook({ bids, asks }: { bids: Order[]; asks: Order[] }) {
  return (
    <div className="order-book">
      <div className="order-book-side">
        <div className="side-title">Bids</div>
        <div className="order-rows">
          {bids.map((b, i) => (
            <div key={i} className="order-row bid">
              <div className="order-price">{b.price.toFixed(2)}</div>
              <div className="order-size mono">{b.size}</div>
            </div>
          ))}
        </div>
      </div>

      <div className="order-book-mid">
        <div className="mid-title">Spread</div>
        <div className="mid-values">
          <div className="mid-price">{asks[0]?.price.toFixed(2) ?? "—"} / {bids[0]?.price.toFixed(2) ?? "—"}</div>
        </div>
      </div>

      <div className="order-book-side">
        <div className="side-title">Asks</div>
        <div className="order-rows">
          {asks.map((a, i) => (
            <div key={i} className="order-row ask">
              <div className="order-price">{a.price.toFixed(2)}</div>
              <div className="order-size mono">{a.size}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function FlashloanPanel({ providers, enabled, onToggle }: { providers: string[]; enabled: Record<string, boolean>; onToggle: (p: string) => void }) {
  return (
    <div className="flashloan-panel">
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <div style={{ fontWeight: 700 }}>Flashloan Providers</div>
        <div className="muted" style={{ fontSize: 12 }}>Toggle to include providers in simulation</div>
      </div>
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
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
  return (
    <div className={`price-diff ${diff > 0 ? 'positive' : diff < 0 ? 'negative' : 'neutral'}`}>
      <div style={{ fontSize: 14, fontWeight: 700 }}>{(Math.abs(diff)).toFixed(2)} USDC</div>
      <div className="muted">{pct.toFixed(2)}%</div>
    </div>
  );
}

// Helper to create sample orderbook data
export function generateOrderBook(mid: number, depth = 8, step = 0.25) {
  const bids: Order[] = [];
  const asks: Order[] = [];
  for (let i = depth; i >= 1; i--) {
    bids.push({ price: +(mid - i * step).toFixed(4), size: Math.round(Math.random() * 50 + 1) });
  }
  for (let i = 1; i <= depth; i++) {
    asks.push({ price: +(mid + i * step).toFixed(4), size: Math.round(Math.random() * 50 + 1) });
  }
  return { bids, asks };
}
