"use strict";
/**
 * X3 DEX Client — main entry point for the Polkadex-style DEX.
 *
 * Combines the orderbook matching engine with the atomic swap orchestrator
 * to provide a complete cross-chain decentralized exchange.
 *
 * Usage:
 *   const dex = new AtlasDexClient(config);
 *   await dex.initialize();
 *   const quote = dex.getQuote('ETH', 'SOL', '1.0');
 *   const order = await dex.submitOrder({ ... });
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.AtlasDexClient = void 0;
const eventemitter3_1 = require("eventemitter3");
const orderbook_1 = require("../orderbook");
const orchestrator_1 = require("../swap/orchestrator");
const monitor_1 = require("../swap/monitor");
class AtlasDexClient extends eventemitter3_1.EventEmitter {
    config;
    orderbookManager;
    swapOrchestrator;
    swapMonitor;
    signerKey = "";
    initialized = false;
    /** Registered trading pairs */
    pairs = new Map();
    /** Known assets (indexed by symbol) */
    assets = new Map();
    constructor(config) {
        super();
        this.config = config;
        // Initialize components
        this.orderbookManager = new orderbook_1.OrderbookManager();
        this.swapOrchestrator = new orchestrator_1.SwapOrchestrator(config, 15_000);
        this.swapMonitor = new monitor_1.SwapMonitor({
            pollInterval: 10_000,
            endpoints: config.chainEndpoints,
            htlcContracts: config.htlcContracts,
        });
        // Wire up events
        this.wireEvents();
    }
    // ─── Lifecycle ──────────────────────────────────────────────
    /**
     * Initialize the DEX client with default trading pairs and assets.
     */
    async initialize() {
        // Register default pairs
        this.registerDefaultPairs();
        this.initialized = true;
        this.emit("connected");
    }
    /**
     * Shut down the DEX client, clean up monitors and intervals.
     */
    destroy() {
        this.swapOrchestrator.destroy();
        this.swapMonitor.destroy();
        this.removeAllListeners();
        this.initialized = false;
        this.emit("disconnected");
    }
    /**
     * Set the signer key used for HTLC operations.
     */
    setSigner(signerKey) {
        this.signerKey = signerKey;
    }
    // ─── Order Management ─────────────────────────────────────
    /**
     * Submit a new order to the DEX.
     *
     * Orders are matched locally in the orderbook engine.
     * When a match occurs, an atomic swap is initiated for settlement.
     */
    async submitOrder(params) {
        if (!this.initialized)
            return { success: false, error: "DEX not initialized" };
        const pair = params.pair || { base: params.baseAsset || "", quote: params.quoteAsset || "" };
        const engine = this.orderbookManager.getEngine(pair);
        const { order, trades } = engine.submitOrder(params, this.signerKey || "anonymous");
        this.emit("order-submitted", order);
        // Auto-settle matched trades via atomic swap
        for (const trade of trades) {
            this.autoSettle(trade).catch((err) => {
                this.emit("error", `Auto-settle failed for trade ${trade.id}: ${err.message}`);
            });
        }
        return { success: true, data: order };
    }
    /**
     * Cancel an existing order.
     */
    cancelOrder(pairKey, orderId) {
        try {
            this.orderbookManager.cancelOrder(pairKey, orderId);
            return { success: true, data: undefined };
        }
        catch (err) {
            return { success: false, error: err.message };
        }
    }
    /**
     * Get all open orders for the current signer.
     */
    getOrders(pair) {
        const engine = this.orderbookManager.getEngine(pair);
        return engine.getOpenOrders(this.signerKey);
    }
    // ─── Orderbook ────────────────────────────────────────────
    /**
     * Get the current orderbook for a trading pair.
     */
    getOrderbook(pair) {
        const engine = this.orderbookManager.getEngine(pair);
        return engine.getOrderbook();
    }
    /**
     * Get all registered trading pairs.
     */
    getTradingPairs() {
        return Array.from(this.pairs.values());
    }
    // ─── Quotes & Routes ─────────────────────────────────────
    /**
     * Get a price quote for swapping between assets.
     */
    getQuote(fromAsset, toAsset, amount) {
        const pair = { base: fromAsset, quote: toAsset };
        const reversePair = { base: toAsset, quote: fromAsset };
        let engine;
        let isBuy;
        try {
            engine = this.orderbookManager.getEngine(pair);
            isBuy = true;
        }
        catch {
            engine = this.orderbookManager.getEngine(reversePair);
            isBuy = false;
        }
        const book = engine.getOrderbook();
        const amountNum = parseFloat(amount);
        let outputAmount = 0;
        let remaining = amountNum;
        // Walk through the book to estimate fill
        const levels = isBuy ? book.asks : book.bids;
        for (const level of levels) {
            if (remaining <= 0)
                break;
            const qty = parseFloat(level.amount);
            const px = parseFloat(level.price);
            const fillQty = Math.min(remaining, qty);
            outputAmount += fillQty * px;
            remaining -= fillQty;
        }
        const effectivePrice = amountNum > 0 ? outputAmount / amountNum : 0;
        const priceImpact = levels.length > 0
            ? Math.abs(effectivePrice - parseFloat(levels[0].price)) / parseFloat(levels[0].price)
            : 0;
        return {
            fromAsset,
            toAsset,
            inputAmount: amount,
            outputAmount: outputAmount.toString(),
            effectivePrice: effectivePrice.toString(),
            priceImpact: isNaN(priceImpact) ? 0 : priceImpact,
            route: [
                {
                    poolId: `${fromAsset}/${toAsset}`,
                    tokenIn: fromAsset,
                    tokenOut: toAsset,
                    protocol: "x3-amm",
                    vmType: "x3",
                    expectedAmountOut: outputAmount.toString(),
                },
            ],
            estimatedGas: "0",
            validUntil: Date.now() + 30_000,
        };
    }
    // ─── Atomic Swap Settlement ───────────────────────────────
    /**
     * Initiate an atomic swap to settle a matched trade.
     *
     * This is called automatically when orders match, or can be called
     * manually for OTC trades.
     */
    async initiateSwap(sourceChain, destChain, sourceToken, destToken, amount, counterparty) {
        if (!this.signerKey)
            return { success: false, error: "No signer key set" };
        try {
            const swap = await this.swapOrchestrator.initiateSwap({
                sourceChain,
                destChain,
                sourceToken,
                destToken,
                amount,
                counterparty,
                timeLockSeconds: this.config.defaultTimeLockInitiator,
            }, this.signerKey);
            this.swapMonitor.watch(swap);
            this.emit("swap-initiated", swap.id);
            return { success: true, data: swap.id };
        }
        catch (err) {
            return { success: false, error: err.message };
        }
    }
    /**
     * Claim a swap (as initiator — reveals secret on dest chain).
     */
    async claimSwap(swapId) {
        if (!this.signerKey)
            return { success: false, error: "No signer key set" };
        try {
            await this.swapOrchestrator.claimSwap(swapId, this.signerKey);
            this.emit("swap-completed", swapId);
            return { success: true, data: undefined };
        }
        catch (err) {
            return { success: false, error: err.message };
        }
    }
    /**
     * Refund an expired swap.
     */
    async refundSwap(swapId) {
        if (!this.signerKey)
            return { success: false, error: "No signer key set" };
        try {
            await this.swapOrchestrator.refundSwap(swapId, this.signerKey);
            return { success: true, data: undefined };
        }
        catch (err) {
            return { success: false, error: err.message };
        }
    }
    // ─── Internals ────────────────────────────────────────────
    wireEvents() {
        // Forward swap events
        this.swapOrchestrator.on("swap-claimed", (swap) => {
            this.emit("swap-completed", swap.id);
        });
        this.swapOrchestrator.on("swap-failed", (swap, err) => {
            this.emit("error", `Swap ${swap.id} failed: ${err}`);
        });
    }
    /**
     * Auto-settle a matched trade via atomic swap.
     */
    async autoSettle(trade) {
        if (trade.settlement.method !== "atomic-swap")
            return;
        // In a full implementation: look up addresses, initiate swap, monitor
    }
    registerDefaultPairs() {
        const defaultPairs = [
            { base: "ETH", quote: "USDT", minOrderSize: "0.001", tickSize: "0.01", lotSize: "0.001" },
            { base: "ETH", quote: "USDC", minOrderSize: "0.001", tickSize: "0.01", lotSize: "0.001" },
            { base: "BTC", quote: "USDT", minOrderSize: "0.0001", tickSize: "0.1", lotSize: "0.0001" },
            { base: "BTC", quote: "ETH", minOrderSize: "0.0001", tickSize: "0.0001", lotSize: "0.0001" },
            { base: "SOL", quote: "USDT", minOrderSize: "0.01", tickSize: "0.001", lotSize: "0.01" },
            { base: "SOL", quote: "ETH", minOrderSize: "0.01", tickSize: "0.0001", lotSize: "0.01" },
            { base: "DOT", quote: "USDT", minOrderSize: "0.1", tickSize: "0.001", lotSize: "0.1" },
            { base: "AVAX", quote: "USDT", minOrderSize: "0.01", tickSize: "0.01", lotSize: "0.01" },
            { base: "MATIC", quote: "USDT", minOrderSize: "1", tickSize: "0.0001", lotSize: "1" },
            { base: "ARB", quote: "USDT", minOrderSize: "1", tickSize: "0.0001", lotSize: "1" },
            { base: "OP", quote: "USDT", minOrderSize: "1", tickSize: "0.0001", lotSize: "1" },
            { base: "X3", quote: "USDT", minOrderSize: "1", tickSize: "0.0001", lotSize: "1" },
            { base: "X3", quote: "ETH", minOrderSize: "1", tickSize: "0.00001", lotSize: "1" },
        ];
        for (const pair of defaultPairs) {
            const key = `${pair.base}/${pair.quote}`;
            this.pairs.set(key, pair);
            this.orderbookManager.getEngine(pair);
        }
    }
}
exports.AtlasDexClient = AtlasDexClient;
//# sourceMappingURL=client.js.map