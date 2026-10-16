import { NextResponse } from 'next/server';

type OrderRow = { price: string; amount: string; total: string; quantum?: boolean };
type OrderbookSnapshot = {
  pair: string;
  bids: OrderRow[];
  asks: OrderRow[];
  updatedAt: number;
  source: 'mock' | 'live';
};

function format(n: number, digits: number) {
  return n.toFixed(digits);
}

function mockOrderbook(pair: string): OrderbookSnapshot {
  const base = 1.2345;
  const bids: OrderRow[] = [];
  const asks: OrderRow[] = [];

  for (let i = 0; i < 12; i++) {
    const price = base - i * 0.0012 - Math.random() * 0.0006;
    const amount = 80 + Math.random() * 900;
    bids.push({
      price: format(price, 4),
      amount: format(amount, 1),
      total: format(price * amount, 1),
      quantum: i < 2,
    });
  }

  for (let i = 0; i < 12; i++) {
    const price = base + i * 0.0012 + Math.random() * 0.0006;
    const amount = 80 + Math.random() * 900;
    asks.push({
      price: format(price, 4),
      amount: format(amount, 1),
      total: format(price * amount, 1),
      quantum: i < 2,
    });
  }

  return {
    pair,
    bids,
    asks,
    updatedAt: Date.now(),
    source: 'mock',
  };
}

export async function GET(req: Request) {
  const { searchParams } = new URL(req.url);
  const pair = searchParams.get('pair') ?? 'ATLAS/USDC';

  // NOTE: This is intentionally a mocked surface.
  // On launch, swap this to a real data source (DEX indexer, offchain service, or on-chain orderbook pallet).
  return NextResponse.json(mockOrderbook(pair), {
    headers: {
      'Cache-Control': 'no-store',
    },
  });
}
