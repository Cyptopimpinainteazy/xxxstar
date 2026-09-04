/**
 * WebSocket client for real-time DEX data streaming.
 *
 * Provides live orderbook updates, trade feeds, and price tickers
 * via WebSocket connection to the X3 DEX relay or node.
 *
 * If no WebSocket server is available, falls back to polling via
 * the AtlasDexClient.
 */
import { EventEmitter } from "eventemitter3";
import type { Orderbook, Order } from "../types";
export interface WsConfig {
    /** WebSocket endpoint URL */
    url: string;
    /** Reconnect automatically on disconnect */
    autoReconnect?: boolean;
    /** Max reconnect attempts */
    maxReconnectAttempts?: number;
    /** Reconnect delay in ms */
    reconnectDelay?: number;
    /** Ping interval in ms */
    pingInterval?: number;
}
type WsEvents = {
    open: () => void;
    close: (code: number, reason: string) => void;
    error: (error: string) => void;
    "orderbook-snapshot": (pair: string, book: Orderbook) => void;
    "orderbook-delta": (pair: string, bids: any[], asks: any[]) => void;
    trade: (pair: string, price: number, quantity: number, side: "buy" | "sell") => void;
    ticker: (pair: string, data: TickerData) => void;
    "order-update": (order: Partial<Order>) => void;
};
export interface TickerData {
    pair: string;
    lastPrice: number;
    high24h: number;
    low24h: number;
    volume24h: number;
    change24h: number;
    changePct24h: number;
}
export declare class DexWebSocket extends EventEmitter<WsEvents> {
    private config;
    private ws;
    private reconnectAttempts;
    private subscriptions;
    private pingTimer;
    private reconnectTimer;
    constructor(config: WsConfig);
    /**
     * Connect to the WebSocket server.
     */
    connect(): void;
    /**
     * Disconnect from the WebSocket server.
     */
    disconnect(): void;
    /**
     * Whether the WebSocket is connected.
     */
    get isConnected(): boolean;
    /**
     * Subscribe to orderbook updates for a trading pair.
     */
    subscribeOrderbook(pair: string): void;
    /**
     * Subscribe to trade feed for a trading pair.
     */
    subscribeTrades(pair: string): void;
    /**
     * Subscribe to price ticker for a trading pair.
     */
    subscribeTicker(pair: string): void;
    /**
     * Subscribe to order updates for the current user.
     */
    subscribeOrders(address: string): void;
    /**
     * Unsubscribe from a channel.
     */
    unsubscribe(channel: string): void;
    /**
     * Unsubscribe from all channels.
     */
    unsubscribeAll(): void;
    private handleMessage;
    private sendMessage;
    private startPing;
    private stopPing;
    private scheduleReconnect;
}
export {};
//# sourceMappingURL=websocket.d.ts.map