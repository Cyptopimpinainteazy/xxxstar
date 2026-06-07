import type {
  TradingPair,
  OrderBook,
  Order,
  Account,
  Trade,
  Market,
  ChartCandle,
  PolkadexStats,
  OrderSide,
  OrderType,
} from '../types';

const API_BASE = '/api/polkadex/v1';

async function fetchJson<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, options);
  if (!res.ok) {
    throw new Error(`Polkadex API error ${res.status}`);
  }
  return res.json();
}

// ─── Trading Pairs ────────────────────────────────────────────

export function getTradingPairs(): Promise<TradingPair[]> {
  return fetchJson('/markets');
}

export function getTradingPair(symbol: string): Promise<TradingPair> {
  return fetchJson(`/markets/${symbol}`);
}

// ─── Order Book ───────────────────────────────────────────────

export function getOrderBook(tradingPair: string, depth = 20): Promise<OrderBook> {
  return fetchJson(`/orderbook/${tradingPair}?depth=${depth}`);
}

// ─── Orders ───────────────────────────────────────────────────

export function getOrders(status?: string): Promise<Order[]> {
  const query = status ? `?status=${status}` : '';
  return fetchJson(`/orders${query}`);
}

export function getOrder(orderId: string): Promise<Order> {
  return fetchJson(`/orders/${orderId}`);
}

export function placeOrder(params: {
  tradingPair: string;
  side: OrderSide;
  type: OrderType;
  price: number;
  amount: number;
}): Promise<Order> {
  return fetchJson('/orders', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export function cancelOrder(orderId: string): Promise<{ success: boolean }> {
  return fetchJson(`/orders/${orderId}`, { method: 'DELETE' });
}

// ─── Account & Balances ───────────────────────────────────────

export function getAccount(): Promise<Account> {
  return fetchJson('/account');
}

export function getBalance(asset: string): Promise<{ asset: string; free: number; locked: number }> {
  return fetchJson(`/account/balance/${asset}`);
}

// ─── Trade History ────────────────────────────────────────────

export function getTrades(tradingPair?: string, limit = 50): Promise<Trade[]> {
  const query = tradingPair ? `?pair=${tradingPair}&limit=${limit}` : `?limit=${limit}`;
  return fetchJson(`/trades${query}`);
}

// ─── Chart Data ───────────────────────────────────────────────

export function getChartData(
  tradingPair: string,
  interval: '1m' | '5m' | '15m' | '1h' | '4h' | '1d' = '1h',
  limit = 100
): Promise<ChartCandle[]> {
  return fetchJson(`/chart/${tradingPair}?interval=${interval}&limit=${limit}`);
}

// ─── Market Info ──────────────────────────────────────────────

export function getMarkets(): Promise<Market[]> {
  return fetchJson('/market-info');
}

export function getStats(): Promise<PolkadexStats> {
  return fetchJson('/stats');
}

// ─── Ticker Data ──────────────────────────────────────────────

export function getTicker(tradingPair: string): Promise<TradingPair> {
  return fetchJson(`/ticker/${tradingPair}`);
}

export function getAllTickers(): Promise<TradingPair[]> {
  return fetchJson('/ticker');
}
