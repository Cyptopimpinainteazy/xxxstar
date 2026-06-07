// Polkadex DEX Types

export enum OrderSide {
  Buy = 'Buy',
  Sell = 'Sell',
}

export enum OrderStatus {
  Open = 'Open',
  PartiallyFilled = 'PartiallyFilled',
  Filled = 'Filled',
  Cancelled = 'Cancelled',
  Expired = 'Expired',
}

export enum OrderType {
  Limit = 'Limit',
  Market = 'Market',
  StopLoss = 'StopLoss',
}

export interface TradingPair {
  id: string;
  baseAsset: string;
  quoteAsset: string;
  symbol: string;
  lastPrice: number;
  change24h: number;
  volume24h: number;
  high24h: number;
  low24h: number;
  ask: number;
  bid: number;
}

export interface OrderBook {
  pair: string;
  bids: OrderLevel[];
  asks: OrderLevel[];
  timestamp: number;
}

export interface OrderLevel {
  price: number;
  amount: number;
  total: number;
}

export interface Order {
  id: string;
  tradingPair: string;
  side: OrderSide;
  type: OrderType;
  status: OrderStatus;
  price: number;
  amount: number;
  filled: number;
  createdAt: number;
  updatedAt: number;
  fee?: number;
}

export interface Account {
  balances: Balance[];
  totalValueUSDT: number;
  availableUSDT: number;
  lockedUSDT: number;
}

export interface Balance {
  asset: string;
  free: number;
  locked: number;
  total: number;
  valueUSDT?: number;
}

export interface Trade {
  id: string;
  tradingPair: string;
  side: OrderSide;
  price: number;
  amount: number;
  fee: number;
  timestamp: number;
}

export interface Market {
  pair: string;
  baseAsset: string;
  quoteAsset: string;
  status: 'active' | 'maintenance' | 'halted';
  minOrderValue: number;
  takerFee: number;
  makerFee: number;
}

export interface ChartCandle {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface PolkadexStats {
  totalTradingVolume24h: number;
  totalUsers: number;
  totalMarkets: number;
  topGainer: string;
  topLoser: string;
}
