/**
 * Polkadex-Inspired Orderbook Matching Engine
 *
 * A price-time priority orderbook that supports:
 * - Limit, Market, Stop-Loss, Take-Profit orders
 * - GTC (Good Till Cancel), IOC (Immediate or Cancel), FOK (Fill or Kill)
 * - Partial fills
 * - Cross-chain settlement via atomic swaps
 * - Price-time priority matching (FIFO at each price level)
 *
 * Architecture inspired by Polkadex's OCEX (Off-Chain Exchange) engine
 * but adapted for multi-chain atomic swap settlement.
 */
import { EventEmitter } from "eventemitter3";
import type { Order, OrderCreateParams, Orderbook, Trade, TradingPair } from "../types";
type MatchEvents = {
    "order-created": (order: Order) => void;
    "order-filled": (order: Order, trades: Trade[]) => void;
    "order-partial-fill": (order: Order, trade: Trade) => void;
    "order-cancelled": (order: Order) => void;
    "trade-executed": (trade: Trade) => void;
    "orderbook-update": (book: Orderbook) => void;
    "price-update": (pair: TradingPair, price: string) => void;
};
export declare class OrderbookEngine extends EventEmitter<MatchEvents> {
    private pair;
    /** Bids sorted descending by price (highest first) */
    private bids;
    /** Asks sorted ascending by price (lowest first) */
    private asks;
    /** All orders by ID */
    private orders;
    /** Stop orders waiting to be triggered */
    private stopOrders;
    /** Last traded price */
    private lastPrice;
    /** Trade counter */
    private tradeCounter;
    /** Order counter */
    private orderCounter;
    constructor(pair: TradingPair);
    /**
     * Submit a new order to the engine.
     * Returns the order and any resulting trades.
     */
    submitOrder(params: OrderCreateParams, owner: string): {
        order: Order;
        trades: Trade[];
    };
    /**
     * Cancel an existing order.
     */
    cancelOrder(orderId: string): Order | null;
    /**
     * Get current orderbook snapshot.
     */
    getOrderbook(): Orderbook;
    /**
     * Get a specific order.
     */
    getOrder(orderId: string): Order | null;
    /**
     * Get all open orders for an owner.
     */
    getOpenOrders(owner: string): Order[];
    /**
     * Get trade history.
     */
    getRecentTrades(limit?: number): Trade[];
    /**
     * Trigger any stop orders based on current price.
     * Should be called after each trade.
     */
    checkStopOrders(): Trade[];
    private executeMarketOrder;
    private executeLimitOrder;
    private matchAtLevel;
    private addToLevel;
    private removeFromLevel;
    private addStopOrder;
    private getSortedPriceLevels;
    private getLevels;
    private createInternalOrder;
    private calculateMinOutput;
    private toPublicOrder;
}
export declare class OrderbookManager extends EventEmitter {
    private engines;
    /**
     * Get or create an orderbook engine for a trading pair.
     */
    getEngine(pair: TradingPair): OrderbookEngine;
    /**
     * Submit an order to the appropriate engine.
     */
    submitOrder(params: OrderCreateParams, owner: string): {
        order: Order;
        trades: Trade[];
    };
    /**
     * Cancel an order.
     */
    cancelOrder(pairKey: string, orderId: string): Order | null;
    /**
     * Get orderbook for a pair.
     */
    getOrderbook(pairKey: string): Orderbook | null;
    /**
     * List all active pairs.
     */
    listPairs(): string[];
}
export {};
//# sourceMappingURL=engine.d.ts.map