import { create } from 'zustand';

export interface Trade {
  id: string;
  pair: string;
  side: 'BUY' | 'SELL';
  price: number;
  amount: number;
  total: number;
  timestamp: number;
  status: 'FILLED' | 'PARTIAL' | 'PENDING' | 'CANCELLED';
}

export interface ActiveOrder {
  id: string;
  pair: string;
  side: 'BUY' | 'SELL';
  orderType: 'LIMIT' | 'MARKET' | 'STOP_LOSS' | 'TAKE_PROFIT' | 'TRAILING_STOP';
  price: number;
  triggerPrice?: number;
  trailingPercent?: number;
  amount: number;
  filled: number;
  timestamp: number;
  profit?: number;
}

interface TradingStore {
  trades: Trade[];
  activeOrders: ActiveOrder[];
  selectedPair: string;
  totalProfit: number;
  
  addTrade: (trade: Trade) => void;
  addOrder: (order: ActiveOrder) => void;
  cancelOrder: (id: string) => void;
  updateOrder: (id: string, partial: Partial<ActiveOrder>) => void;
  setSelectedPair: (pair: string) => void;
  calculatePnL: () => number;
}

export const useTradingStore = create<TradingStore>((set, get) => ({
  trades: [],
  activeOrders: [],
  selectedPair: 'PDEX/USDT',
  totalProfit: 0,
  
  addTrade: (trade) =>
    set((state) => {
      const trades = [trade, ...state.trades];
      let totalCost = 0;
      let totalRevenue = 0;

      trades.forEach((t) => {
        const value = t.price * t.amount;
        if (t.side === 'BUY') {
          totalCost += value;
        } else {
          totalRevenue += value;
        }
      });

      return {
        trades,
        totalProfit: totalRevenue - totalCost,
      };
    }),
  
  addOrder: (order) => set((state) => ({
    activeOrders: [order, ...state.activeOrders],
  })),
  
  cancelOrder: (id) => set((state) => ({
    activeOrders: state.activeOrders.filter((o) => o.id !== id),
  })),
  
  updateOrder: (id, partial) => set((state) => ({
    activeOrders: state.activeOrders.map((o) =>
      o.id === id ? { ...o, ...partial } : o
    ),
  })),
  
  setSelectedPair: (pair) => set({ selectedPair: pair }),
  
  calculatePnL: () => {
    const { trades } = get();
    let totalCost = 0;
    let totalRevenue = 0;
    
    trades.forEach((trade) => {
      const value = trade.price * trade.amount;
      if (trade.side === 'BUY') {
        totalCost += value;
      } else {
        totalRevenue += value;
      }
    });
    
    return totalRevenue - totalCost;
  },
}));
