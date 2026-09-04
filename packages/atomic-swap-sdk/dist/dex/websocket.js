"use strict";
/**
 * WebSocket client for real-time DEX data streaming.
 *
 * Provides live orderbook updates, trade feeds, and price tickers
 * via WebSocket connection to the X3 DEX relay or node.
 *
 * If no WebSocket server is available, falls back to polling via
 * the AtlasDexClient.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.DexWebSocket = void 0;
const eventemitter3_1 = require("eventemitter3");
class DexWebSocket extends eventemitter3_1.EventEmitter {
    config;
    ws = null;
    reconnectAttempts = 0;
    subscriptions = new Set();
    pingTimer = null;
    reconnectTimer = null;
    constructor(config) {
        super();
        this.config = {
            autoReconnect: true,
            maxReconnectAttempts: 10,
            reconnectDelay: 3000,
            pingInterval: 30000,
            ...config,
        };
    }
    // ─── Connection ───────────────────────────────────────────
    /**
     * Connect to the WebSocket server.
     */
    connect() {
        if (this.ws?.readyState === WebSocket.OPEN)
            return;
        try {
            this.ws = new WebSocket(this.config.url);
            this.ws.onopen = () => {
                this.reconnectAttempts = 0;
                this.emit("open");
                // Re-subscribe to all channels
                for (const channel of this.subscriptions) {
                    this.sendMessage({ type: "subscribe", channel });
                }
                // Start ping
                this.startPing();
            };
            this.ws.onclose = (event) => {
                this.emit("close", event.code, event.reason);
                this.stopPing();
                if (this.config.autoReconnect) {
                    this.scheduleReconnect();
                }
            };
            this.ws.onerror = () => {
                this.emit("error", "WebSocket error");
            };
            this.ws.onmessage = (event) => {
                this.handleMessage(event.data);
            };
        }
        catch (err) {
            this.emit("error", err.message || "Connection failed");
        }
    }
    /**
     * Disconnect from the WebSocket server.
     */
    disconnect() {
        this.config.autoReconnect = false;
        this.stopPing();
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }
        if (this.ws) {
            this.ws.close(1000, "Client disconnect");
            this.ws = null;
        }
    }
    /**
     * Whether the WebSocket is connected.
     */
    get isConnected() {
        return this.ws?.readyState === WebSocket.OPEN;
    }
    // ─── Subscriptions ────────────────────────────────────────
    /**
     * Subscribe to orderbook updates for a trading pair.
     */
    subscribeOrderbook(pair) {
        const channel = `orderbook:${pair}`;
        this.subscriptions.add(channel);
        if (this.isConnected) {
            this.sendMessage({ type: "subscribe", channel });
        }
    }
    /**
     * Subscribe to trade feed for a trading pair.
     */
    subscribeTrades(pair) {
        const channel = `trades:${pair}`;
        this.subscriptions.add(channel);
        if (this.isConnected) {
            this.sendMessage({ type: "subscribe", channel });
        }
    }
    /**
     * Subscribe to price ticker for a trading pair.
     */
    subscribeTicker(pair) {
        const channel = `ticker:${pair}`;
        this.subscriptions.add(channel);
        if (this.isConnected) {
            this.sendMessage({ type: "subscribe", channel });
        }
    }
    /**
     * Subscribe to order updates for the current user.
     */
    subscribeOrders(address) {
        const channel = `orders:${address}`;
        this.subscriptions.add(channel);
        if (this.isConnected) {
            this.sendMessage({ type: "subscribe", channel });
        }
    }
    /**
     * Unsubscribe from a channel.
     */
    unsubscribe(channel) {
        this.subscriptions.delete(channel);
        if (this.isConnected) {
            this.sendMessage({ type: "unsubscribe", channel });
        }
    }
    /**
     * Unsubscribe from all channels.
     */
    unsubscribeAll() {
        for (const channel of this.subscriptions) {
            if (this.isConnected) {
                this.sendMessage({ type: "unsubscribe", channel });
            }
        }
        this.subscriptions.clear();
    }
    // ─── Internals ────────────────────────────────────────────
    handleMessage(raw) {
        let msg;
        try {
            msg = JSON.parse(raw);
        }
        catch {
            return;
        }
        switch (msg.type) {
            case "orderbook":
                if (msg.channel && msg.data) {
                    const pair = msg.channel.replace("orderbook:", "");
                    if (msg.data.snapshot) {
                        this.emit("orderbook-snapshot", pair, msg.data.book);
                    }
                    else {
                        this.emit("orderbook-delta", pair, msg.data.bids || [], msg.data.asks || []);
                    }
                }
                break;
            case "trade":
                if (msg.channel && msg.data) {
                    const pair = msg.channel.replace("trades:", "");
                    this.emit("trade", pair, msg.data.price, msg.data.quantity, msg.data.side);
                }
                break;
            case "ticker":
                if (msg.channel && msg.data) {
                    const pair = msg.channel.replace("ticker:", "");
                    this.emit("ticker", pair, msg.data);
                }
                break;
            case "order-update":
                if (msg.data) {
                    this.emit("order-update", msg.data);
                }
                break;
            case "pong":
                // Heartbeat response
                break;
            case "error":
                this.emit("error", msg.data?.message || "Server error");
                break;
        }
    }
    sendMessage(msg) {
        if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(msg));
        }
    }
    startPing() {
        this.stopPing();
        this.pingTimer = setInterval(() => {
            this.sendMessage({ type: "subscribe", channel: "ping" });
        }, this.config.pingInterval);
    }
    stopPing() {
        if (this.pingTimer) {
            clearInterval(this.pingTimer);
            this.pingTimer = null;
        }
    }
    scheduleReconnect() {
        if (this.reconnectAttempts >= (this.config.maxReconnectAttempts || 10)) {
            this.emit("error", "Max reconnect attempts reached");
            return;
        }
        const delay = (this.config.reconnectDelay || 3000) * Math.pow(1.5, this.reconnectAttempts);
        this.reconnectAttempts++;
        this.reconnectTimer = setTimeout(() => {
            this.connect();
        }, delay);
    }
}
exports.DexWebSocket = DexWebSocket;
//# sourceMappingURL=websocket.js.map