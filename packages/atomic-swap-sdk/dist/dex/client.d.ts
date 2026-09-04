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
import type { DexConfig, Order, OrderCreateParams, Orderbook, TradeQuote, TradingPair, ChainId, ApiResult } from "../types";
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
export declare class AtlasDexClient extends EventEmitter<DexClientEvents> {
    private config;
    private orderbookManager;
    private swapOrchestrator;
    private swapMonitor;
    private signerKey;
    private initialized;
    /** Registered trading pairs */
    private pairs;
    /** Known assets (indexed by symbol) */
    private assets;
    constructor(config: DexConfig);
    /**
     * Initialize the DEX client with default trading pairs and assets.
     */
    initialize(): Promise<void>;
    /**
     * Shut down the DEX client, clean up monitors and intervals.
     */
    destroy(): void;
    /**
     * Set the signer key used for HTLC operations.
     */
    setSigner(signerKey: string): void;
    /**
     * Submit a new order to the DEX.
     *
     * Orders are matched locally in the orderbook engine.
     * When a match occurs, an atomic swap is initiated for settlement.
     */
    submitOrder(params: OrderCreateParams): Promise<ApiResult<Order>>;
    /**
     * Cancel an existing order.
     */
    cancelOrder(pairKey: string, orderId: string): ApiResult<void>;
    /**
     * Get all open orders for the current signer.
     */
    getOrders(pair: TradingPair): Order[];
    /**
     * Get the current orderbook for a trading pair.
     */
    getOrderbook(pair: TradingPair): Orderbook;
    /**
     * Get all registered trading pairs.
     */
    getTradingPairs(): TradingPair[];
    /**
     * Get a price quote for swapping between assets.
     */
    getQuote(fromAsset: string, toAsset: string, amount: string): TradeQuote;
    /**
     * Initiate an atomic swap to settle a matched trade.
     *
     * This is called automatically when orders match, or can be called
     * manually for OTC trades.
     */
    initiateSwap(sourceChain: ChainId, destChain: ChainId, sourceToken: string, destToken: string, amount: string, counterparty: string): Promise<ApiResult<string>>;
    /**
     * Claim a swap (as initiator — reveals secret on dest chain).
     */
    claimSwap(swapId: string): Promise<ApiResult<void>>;
    /**
     * Refund an expired swap.
     */
    refundSwap(swapId: string): Promise<ApiResult<void>>;
    private wireEvents;
    /**
     * Auto-settle a matched trade via atomic swap.
     */
    private autoSettle;
    private registerDefaultPairs;
}
//# sourceMappingURL=client.d.ts.map