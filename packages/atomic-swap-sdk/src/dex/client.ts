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

import { EventEmitter } from "eventemitter3";
import type {
  DexConfig,
  DexEvent,
  DexEventType,
  Order,
  OrderCreateParams,
  Orderbook,
  TradeRoute,
  TradeQuote,
  TradingPair,
  Asset,
  ChainId,
  ApiResult,
  Trade,
} from "../types";
import { OrderbookEngine, OrderbookManager } from "../orderbook";
import { SwapOrchestrator } from "../swap/orchestrator";
import { SwapMonitor, type SwapMonitorConfig } from "../swap/monitor";

export interface DexClientEvents {
  "order-submitted": (order: Order) => void;
  "order-filled": (order: Order) => void;
  "order-cancelled": (order: Order) => void;
  "trade-executed": (maker: Order, taker: Order, price: number, quantity: number) => void;
  "price-update": (pair: string, price: number) => void;
  "orderbook-update": (pair: string, book: Orderbook) => void;
  "swap-initiated": (swapId: string) => void;
  "swap-completed": (swapId: string) => void;
  "error": (error: string) => void;
  connected: () => void;
  disconnected: () => void;
}

export class AtlasDexClient extends EventEmitter<DexClientEvents> {
  private config: DexConfig;
  private orderbookManager: OrderbookManager;
  private swapOrchestrator: SwapOrchestrator;
  private swapMonitor: SwapMonitor;
  private signerKey: string = "";
  private initialized = false;

  /** Registered trading pairs */
  private pairs: Map<string, TradingPair> = new Map();

  /** Known assets (indexed by symbol) */
  private assets: Map<string, Asset> = new Map();

  constructor(config: DexConfig) {
    super();
    this.config = config;

    // Initialize components
    this.orderbookManager = new OrderbookManager();
    this.swapOrchestrator = new SwapOrchestrator(config, 15_000);
    this.swapMonitor = new SwapMonitor({
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
  async initialize(): Promise<void> {
    // Register default pairs
    this.registerDefaultPairs();
    this.initialized = true;
    this.emit("connected");
  }

  /**
   * Shut down the DEX client, clean up monitors and intervals.
   */
  destroy(): void {
    this.swapOrchestrator.destroy();
    this.swapMonitor.destroy();
    this.removeAllListeners();
    this.initialized = false;
    this.emit("disconnected");
  }

  /**
   * Set the signer key used for HTLC operations.
   */
  setSigner(signerKey: string): void {
    this.signerKey = signerKey;
  }

  // ─── Order Management ─────────────────────────────────────

  /**
   * Submit a new order to the DEX.
   *
   * Orders are matched locally in the orderbook engine.
   * When a match occurs, an atomic swap is initiated for settlement.
   */
  async submitOrder(params: OrderCreateParams): Promise<ApiResult<Order>> {
    if (!this.initialized) return { success: false, error: "DEX not initialized" };

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
  cancelOrder(pairKey: string, orderId: string): ApiResult<void> {
    try {
      this.orderbookManager.cancelOrder(pairKey, orderId);
      return { success: true, data: undefined };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  }

  /**
   * Get all open orders for the current signer.
   */
  getOrders(pair: TradingPair): Order[] {
    const engine = this.orderbookManager.getEngine(pair);
    return engine.getOpenOrders(this.signerKey);
  }

  // ─── Orderbook ────────────────────────────────────────────

  /**
   * Get the current orderbook for a trading pair.
   */
  getOrderbook(pair: TradingPair): Orderbook {
    const engine = this.orderbookManager.getEngine(pair);
    return engine.getOrderbook();
  }

  /**
   * Get all registered trading pairs.
   */
  getTradingPairs(): TradingPair[] {
    return Array.from(this.pairs.values());
  }

  // ─── Quotes & Routes ─────────────────────────────────────

  /**
   * Get a price quote for swapping between assets.
   */
  getQuote(fromAsset: string, toAsset: string, amount: string): TradeQuote {
    const pair: TradingPair = { base: fromAsset, quote: toAsset };
    const reversePair: TradingPair = { base: toAsset, quote: fromAsset };

    let engine: OrderbookEngine;
    let isBuy: boolean;

    try {
      engine = this.orderbookManager.getEngine(pair);
      isBuy = true;
    } catch {
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
      if (remaining <= 0) break;
      const qty = parseFloat(level.amount);
      const px = parseFloat(level.price);
      const fillQty = Math.min(remaining, qty);
      outputAmount += fillQty * px;
      remaining -= fillQty;
    }

    const effectivePrice = amountNum > 0 ? outputAmount / amountNum : 0;
    const priceImpact =
      levels.length > 0
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
  async initiateSwap(
    sourceChain: ChainId,
    destChain: ChainId,
    sourceToken: string,
    destToken: string,
    amount: string,
    counterparty: string,
  ): Promise<ApiResult<string>> {
    if (!this.signerKey) return { success: false, error: "No signer key set" };

    try {
      const swap = await this.swapOrchestrator.initiateSwap(
        {
          sourceChain,
          destChain,
          sourceToken,
          destToken,
          amount,
          counterparty,
          timeLockSeconds: this.config.defaultTimeLockInitiator,
        },
        this.signerKey,
      );

      this.swapMonitor.watch(swap);
      this.emit("swap-initiated", swap.id);

      return { success: true, data: swap.id };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  }

  /**
   * Claim a swap (as initiator — reveals secret on dest chain).
   */
  async claimSwap(swapId: string): Promise<ApiResult<void>> {
    if (!this.signerKey) return { success: false, error: "No signer key set" };

    try {
      await this.swapOrchestrator.claimSwap(swapId, this.signerKey);
      this.emit("swap-completed", swapId);
      return { success: true, data: undefined };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  }

  /**
   * Refund an expired swap.
   */
  async refundSwap(swapId: string): Promise<ApiResult<void>> {
    if (!this.signerKey) return { success: false, error: "No signer key set" };

    try {
      await this.swapOrchestrator.refundSwap(swapId, this.signerKey);
      return { success: true, data: undefined };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  }

  // ─── Internals ────────────────────────────────────────────

  private wireEvents(): void {
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
  private async autoSettle(trade: Trade): Promise<void> {
    if (trade.settlement.method !== "atomic-swap") return;
    // In a full implementation: look up addresses, initiate swap, monitor
  }

  private registerDefaultPairs(): void {
    const defaultPairs: TradingPair[] = [
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
