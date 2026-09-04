"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.OrderbookManager = exports.OrderbookEngine = void 0;
const eventemitter3_1 = require("eventemitter3");
// ─── Orderbook Engine ───────────────────────────────────────────
class OrderbookEngine extends eventemitter3_1.EventEmitter {
    pair;
    /** Bids sorted descending by price (highest first) */
    bids = new Map();
    /** Asks sorted ascending by price (lowest first) */
    asks = new Map();
    /** All orders by ID */
    orders = new Map();
    /** Stop orders waiting to be triggered */
    stopOrders = [];
    /** Last traded price */
    lastPrice = "0";
    /** Trade counter */
    tradeCounter = 0;
    /** Order counter */
    orderCounter = 0;
    constructor(pair) {
        super();
        this.pair = pair;
    }
    // ─── Public API ─────────────────────────────────────────────
    /**
     * Submit a new order to the engine.
     * Returns the order and any resulting trades.
     */
    submitOrder(params, owner) {
        const order = this.createInternalOrder(params, owner);
        this.orders.set(order.id, order);
        let trades = [];
        switch (order.type) {
            case "market":
                trades = this.executeMarketOrder(order);
                break;
            case "limit":
                trades = this.executeLimitOrder(order);
                break;
            case "stop-loss":
            case "take-profit":
                this.addStopOrder(order);
                break;
        }
        // Handle time-in-force
        if (order.timeInForce === "IOC" && order.status === "open") {
            // IOC: cancel unfilled portion
            this.cancelOrder(order.id);
        }
        else if (order.timeInForce === "FOK" && order.status !== "filled") {
            // FOK: cancel entire order if not fully filled
            this.cancelOrder(order.id);
            trades = []; // discard partial fills
        }
        // Emit events
        this.emit("order-created", this.toPublicOrder(order));
        for (const trade of trades) {
            this.emit("trade-executed", trade);
        }
        if (order.status === "filled") {
            this.emit("order-filled", this.toPublicOrder(order), trades);
        }
        this.emit("orderbook-update", this.getOrderbook());
        return { order: this.toPublicOrder(order), trades };
    }
    /**
     * Cancel an existing order.
     */
    cancelOrder(orderId) {
        const order = this.orders.get(orderId);
        if (!order || order.status === "filled" || order.status === "cancelled") {
            return null;
        }
        order.status = "cancelled";
        // Remove from orderbook
        if (order.side === "buy") {
            this.removeFromLevel(this.bids, order);
        }
        else {
            this.removeFromLevel(this.asks, order);
        }
        // Remove from stop orders
        this.stopOrders = this.stopOrders.filter((o) => o.id !== orderId);
        this.emit("order-cancelled", this.toPublicOrder(order));
        this.emit("orderbook-update", this.getOrderbook());
        return this.toPublicOrder(order);
    }
    /**
     * Get current orderbook snapshot.
     */
    getOrderbook() {
        const bidLevels = this.getLevels(this.bids, "desc");
        const askLevels = this.getLevels(this.asks, "asc");
        const bestBid = bidLevels[0]?.price || "0";
        const bestAsk = askLevels[0]?.price || "0";
        const spread = bestBid !== "0" && bestAsk !== "0"
            ? (parseFloat(bestAsk) - parseFloat(bestBid)).toFixed(8)
            : "0";
        const midPrice = (parseFloat(bestBid) + parseFloat(bestAsk)) / 2;
        const spreadPercent = midPrice > 0 ? ((parseFloat(spread) / midPrice) * 100).toFixed(4) : "0";
        return {
            pair: this.pair,
            bids: bidLevels,
            asks: askLevels,
            lastPrice: this.lastPrice,
            spread,
            spreadPercent,
            timestamp: Date.now(),
        };
    }
    /**
     * Get a specific order.
     */
    getOrder(orderId) {
        const order = this.orders.get(orderId);
        return order ? this.toPublicOrder(order) : null;
    }
    /**
     * Get all open orders for an owner.
     */
    getOpenOrders(owner) {
        const result = [];
        for (const order of this.orders.values()) {
            if (order.owner === owner && (order.status === "open" || order.status === "partial")) {
                result.push(this.toPublicOrder(order));
            }
        }
        return result;
    }
    /**
     * Get trade history.
     */
    getRecentTrades(limit = 50) {
        // In a real implementation, we'd store trades in a ring buffer.
        // For now, trades are emitted via events — caller should maintain history.
        return [];
    }
    /**
     * Trigger any stop orders based on current price.
     * Should be called after each trade.
     */
    checkStopOrders() {
        const currentPrice = parseFloat(this.lastPrice);
        if (currentPrice === 0)
            return [];
        const triggered = [];
        const remaining = [];
        for (const order of this.stopOrders) {
            const triggerPrice = parseFloat(order.price);
            if (order.type === "stop-loss") {
                // Stop-loss triggers when price falls below trigger price
                if ((order.side === "sell" && currentPrice <= triggerPrice) ||
                    (order.side === "buy" && currentPrice >= triggerPrice)) {
                    triggered.push(order);
                    continue;
                }
            }
            else if (order.type === "take-profit") {
                // Take-profit triggers when price rises above trigger price
                if ((order.side === "sell" && currentPrice >= triggerPrice) ||
                    (order.side === "buy" && currentPrice <= triggerPrice)) {
                    triggered.push(order);
                    continue;
                }
            }
            remaining.push(order);
        }
        this.stopOrders = remaining;
        // Execute triggered orders as market orders
        const allTrades = [];
        for (const order of triggered) {
            order.type = "market";
            const trades = this.executeMarketOrder(order);
            allTrades.push(...trades);
        }
        return allTrades;
    }
    // ─── Matching Engine Core ─────────────────────────────────────
    executeMarketOrder(order) {
        const trades = [];
        const book = order.side === "buy" ? this.asks : this.bids;
        const sortedLevels = this.getSortedPriceLevels(book, order.side === "buy" ? "asc" : "desc");
        for (const level of sortedLevels) {
            if (parseFloat(order.remainingAmount) <= 0)
                break;
            const matchTrades = this.matchAtLevel(order, level);
            trades.push(...matchTrades);
        }
        // Update order status
        if (parseFloat(order.remainingAmount) <= 0) {
            order.status = "filled";
        }
        else if (parseFloat(order.filledAmount) > 0) {
            order.status = "partial";
        }
        return trades;
    }
    executeLimitOrder(order) {
        const trades = [];
        const book = order.side === "buy" ? this.asks : this.bids;
        const sortedLevels = this.getSortedPriceLevels(book, order.side === "buy" ? "asc" : "desc");
        for (const level of sortedLevels) {
            if (parseFloat(order.remainingAmount) <= 0)
                break;
            // Check price compatibility
            const levelPrice = parseFloat(level.price);
            const orderPrice = parseFloat(order.price);
            if ((order.side === "buy" && levelPrice > orderPrice) ||
                (order.side === "sell" && levelPrice < orderPrice)) {
                break; // No more matching levels
            }
            const matchTrades = this.matchAtLevel(order, level);
            trades.push(...matchTrades);
        }
        // If unfilled, add to orderbook
        if (parseFloat(order.remainingAmount) > 0 && order.status !== "cancelled") {
            const targetBook = order.side === "buy" ? this.bids : this.asks;
            this.addToLevel(targetBook, order);
            if (parseFloat(order.filledAmount) > 0) {
                order.status = "partial";
            }
        }
        else if (parseFloat(order.remainingAmount) <= 0) {
            order.status = "filled";
        }
        return trades;
    }
    matchAtLevel(taker, level) {
        const trades = [];
        const toRemove = [];
        for (const maker of level.orders) {
            if (parseFloat(taker.remainingAmount) <= 0)
                break;
            const takerRemaining = parseFloat(taker.remainingAmount);
            const makerRemaining = parseFloat(maker.remainingAmount);
            const fillAmount = Math.min(takerRemaining, makerRemaining);
            const fillPrice = maker.price; // Price-time priority: use maker's price
            // Create trade
            const trade = {
                id: `trade_${++this.tradeCounter}`,
                pair: this.pair,
                price: fillPrice,
                amount: fillAmount.toFixed(8),
                side: taker.side,
                makerOrderId: maker.id,
                takerOrderId: taker.id,
                settlement: {
                    method: "atomic-swap",
                    chainId: taker.settlementChain || "x3-substrate",
                    status: "pending",
                },
                timestamp: Date.now(),
            };
            trades.push(trade);
            // Update amounts
            taker.remainingAmount = (takerRemaining - fillAmount).toFixed(8);
            taker.filledAmount = (parseFloat(taker.filledAmount) + fillAmount).toFixed(8);
            maker.remainingAmount = (makerRemaining - fillAmount).toFixed(8);
            maker.filledAmount = (parseFloat(maker.filledAmount) + fillAmount).toFixed(8);
            // Update maker status
            if (parseFloat(maker.remainingAmount) <= 0) {
                maker.status = "filled";
                toRemove.push(maker.id);
                this.emit("order-filled", this.toPublicOrder(maker), [trade]);
            }
            else {
                maker.status = "partial";
                this.emit("order-partial-fill", this.toPublicOrder(maker), trade);
            }
            // Update taker partial fill events
            if (parseFloat(taker.remainingAmount) > 0) {
                this.emit("order-partial-fill", this.toPublicOrder(taker), trade);
            }
            // Update last price
            this.lastPrice = fillPrice;
            this.emit("price-update", this.pair, fillPrice);
        }
        // Remove filled orders from level
        level.orders = level.orders.filter((o) => !toRemove.includes(o.id));
        level.totalAmount = level.orders
            .reduce((sum, o) => sum + parseFloat(o.remainingAmount), 0)
            .toFixed(8);
        // Check stop orders after price change
        if (trades.length > 0) {
            const stopTrades = this.checkStopOrders();
            trades.push(...stopTrades);
        }
        return trades;
    }
    // ─── Level Management ─────────────────────────────────────────
    addToLevel(book, order) {
        const price = order.price;
        let level = book.get(price);
        if (!level) {
            level = { price, orders: [], totalAmount: "0" };
            book.set(price, level);
        }
        level.orders.push(order);
        level.totalAmount = (parseFloat(level.totalAmount) + parseFloat(order.remainingAmount)).toFixed(8);
    }
    removeFromLevel(book, order) {
        const price = order.price;
        const level = book.get(price);
        if (!level)
            return;
        level.orders = level.orders.filter((o) => o.id !== order.id);
        level.totalAmount = level.orders
            .reduce((sum, o) => sum + parseFloat(o.remainingAmount), 0)
            .toFixed(8);
        if (level.orders.length === 0) {
            book.delete(price);
        }
    }
    addStopOrder(order) {
        this.stopOrders.push(order);
    }
    getSortedPriceLevels(book, direction) {
        const levels = Array.from(book.values());
        levels.sort((a, b) => {
            const diff = parseFloat(a.price) - parseFloat(b.price);
            return direction === "asc" ? diff : -diff;
        });
        return levels;
    }
    getLevels(book, direction) {
        const sorted = this.getSortedPriceLevels(book, direction);
        let cumulative = 0;
        return sorted.map((level) => {
            cumulative += parseFloat(level.totalAmount);
            return {
                price: level.price,
                amount: level.totalAmount,
                total: cumulative.toFixed(8),
                orderCount: level.orders.length,
            };
        });
    }
    // ─── Order Factory ────────────────────────────────────────────
    createInternalOrder(params, owner) {
        const id = `order_${++this.orderCounter}_${Date.now()}`;
        const now = Date.now();
        return {
            id,
            owner,
            pair: params.pair,
            type: params.type,
            side: params.side,
            price: params.price,
            amount: params.amount,
            filledAmount: "0",
            remainingAmount: params.amount,
            minOutput: params.slippageBps
                ? this.calculateMinOutput(params.amount, params.price, params.slippageBps)
                : undefined,
            timeInForce: params.timeInForce || "GTC",
            status: "open",
            createdAt: now,
            expiresAt: params.expiresAt || 0,
            settlementChain: params.settlementChain,
        };
    }
    calculateMinOutput(amount, price, slippageBps) {
        const output = parseFloat(amount) * parseFloat(price);
        const slippageFactor = 1 - slippageBps / 10000;
        return (output * slippageFactor).toFixed(8);
    }
    toPublicOrder(order) {
        const { remainingAmount, ...publicOrder } = order;
        return publicOrder;
    }
}
exports.OrderbookEngine = OrderbookEngine;
// ─── Multi-Pair Orderbook Manager ───────────────────────────────
class OrderbookManager extends eventemitter3_1.EventEmitter {
    engines = new Map();
    /**
     * Get or create an orderbook engine for a trading pair.
     */
    getEngine(pair) {
        const key = `${pair.base}/${pair.quote}`;
        let engine = this.engines.get(key);
        if (!engine) {
            engine = new OrderbookEngine(pair);
            // Forward events
            engine.on("trade-executed", (trade) => this.emit("trade-executed", trade));
            engine.on("orderbook-update", (book) => this.emit("orderbook-update", book));
            engine.on("price-update", (p, price) => this.emit("price-update", key, price));
            this.engines.set(key, engine);
        }
        return engine;
    }
    /**
     * Submit an order to the appropriate engine.
     */
    submitOrder(params, owner) {
        const engine = this.getEngine(params.pair);
        return engine.submitOrder(params, owner);
    }
    /**
     * Cancel an order.
     */
    cancelOrder(pairKey, orderId) {
        const engine = this.engines.get(pairKey);
        return engine?.cancelOrder(orderId) || null;
    }
    /**
     * Get orderbook for a pair.
     */
    getOrderbook(pairKey) {
        const engine = this.engines.get(pairKey);
        return engine?.getOrderbook() || null;
    }
    /**
     * List all active pairs.
     */
    listPairs() {
        return Array.from(this.engines.keys());
    }
}
exports.OrderbookManager = OrderbookManager;
//# sourceMappingURL=engine.js.map