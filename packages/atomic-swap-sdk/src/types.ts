/**
 * @x3-chain/atomic-swap-sdk — Core Types
 *
 * Mirrors the Rust pallet types for cross-language compatibility.
 * Covers HTLCs, orders, swaps, trade routes, and chain identifiers.
 */

// ─── Chain & VM Identifiers ─────────────────────────────────────

export type VmType = "evm" | "svm" | "x3" | "cross-vm";

export type ChainId =
  | "ethereum"
  | "ethereum-sepolia"
  | "polygon"
  | "bsc"
  | "arbitrum"
  | "optimism"
  | "base"
  | "avalanche"
  | "solana"
  | "solana-devnet"
  | "bitcoin"
  | "bitcoin-testnet"
  | "x3-substrate"
  | string;

export type AmmProtocol =
  | "uniswap-v2"
  | "uniswap-v3"
  | "raydium"
  | "orca-whirlpool"
  | "x3-amm"
  | "constant-product"
  | "stable-swap";

// ─── HTLC Types ─────────────────────────────────────────────────

export type HTLCStatus =
  | "pending"
  | "funded"
  | "claimed"
  | "refunded"
  | "expired";

export interface HTLC {
  /** Unique HTLC identifier */
  id: string;
  /** Chain the HTLC lives on */
  chainId: ChainId;
  /** VM type */
  vmType: VmType;
  /** SHA-256 hash of the secret */
  hashLock: string;
  /** Unix timestamp after which refund is allowed */
  timeLock: number;
  /** Sender address */
  sender: string;
  /** Recipient address */
  recipient: string;
  /** Token address (native = "0x0" or equivalent) */
  tokenAddress: string;
  /** Amount in smallest unit (wei, lamports, satoshi) */
  amount: string;
  /** HTLC contract/program address on-chain */
  contractAddress: string;
  /** On-chain tx hash that funded the HTLC */
  fundingTxHash?: string;
  /** Secret preimage (only known after claim) */
  secret?: string;
  /** Current status */
  status: HTLCStatus;
  /** Creation timestamp */
  createdAt: number;
  /** Last update timestamp */
  updatedAt: number;
}

export interface HTLCCreateParams {
  chainId: ChainId;
  recipient: string;
  tokenAddress: string;
  amount: string;
  hashLock: string;
  timeLock: number;
  /** Optional: specify HTLC contract (default uses chain's deployed contract) */
  contractAddress?: string;
}

export interface HTLCClaimParams {
  htlcId: string;
  secret: string;
}

export interface HTLCRefundParams {
  htlcId: string;
}

// ─── Asset / Token Types ────────────────────────────────────────

export interface Asset {
  /** Unique cross-chain identifier */
  id: string;
  /** Human symbol (ETH, SOL, BTC, USDC) */
  symbol: string;
  /** Full name */
  name: string;
  /** Decimal precision */
  decimals: number;
  /** Chain-specific contract addresses */
  addresses: Record<ChainId, string>;
  /** Logo URL */
  logoUrl?: string;
  /** Coingecko ID for price feeds */
  coingeckoId?: string;
}

export interface TokenBalance {
  asset: Asset;
  chainId: ChainId;
  balance: string;
  balanceUsd?: number;
}

// ─── Order Types (Polkadex-style) ───────────────────────────────

export type OrderType = "market" | "limit" | "stop-loss" | "take-profit";
export type OrderSide = "buy" | "sell";
export type OrderStatus =
  | "open"
  | "partial"
  | "filled"
  | "cancelled"
  | "expired";

export interface Order {
  /** Unique order ID */
  id: string;
  /** Owner address (chain-agnostic account) */
  owner: string;
  /** Trading pair */
  pair: TradingPair;
  /** Order type */
  type: OrderType;
  /** Buy or sell */
  side: OrderSide;
  /** Price per unit of base token (in quote token) — 0 for market orders */
  price: string;
  /** Amount of base token */
  amount: string;
  /** Filled amount */
  filledAmount: string;
  /** Minimum output (slippage protection) */
  minOutput?: string;
  /** Time-in-force: GTC, IOC, FOK */
  timeInForce: "GTC" | "IOC" | "FOK";
  /** Order status */
  status: OrderStatus;
  /** Timestamp */
  createdAt: number;
  /** Expiry timestamp (0 = no expiry) */
  expiresAt: number;
  /** Preferred chain for settlement */
  settlementChain?: ChainId;
}

export interface OrderCreateParams {
  /** Trading pair (or just base/quote strings) */
  pair: TradingPair;
  /** Or specify base/quote directly */
  baseAsset?: string;
  quoteAsset?: string;
  type: OrderType;
  side: OrderSide;
  price: string;
  amount: string;
  timeInForce?: "GTC" | "IOC" | "FOK";
  expiresAt?: number;
  slippageBps?: number;
  settlementChain?: ChainId;
}

// ─── Trading Pair ───────────────────────────────────────────────

export interface TradingPair {
  /** Base token symbol (the one being bought/sold) */
  base: string;
  /** Quote token symbol (price denominated in this token) */
  quote: string;
  /** Minimum order size (base units) */
  minOrderSize?: string;
  /** Tick size for price precision */
  tickSize?: string;
  /** Lot size for quantity precision */
  lotSize?: string;
}

// ─── Orderbook ──────────────────────────────────────────────────

export interface OrderbookLevel {
  price: string;
  amount: string;
  total: string;
  orderCount: number;
}

export interface Orderbook {
  pair: TradingPair;
  bids: OrderbookLevel[];
  asks: OrderbookLevel[];
  lastPrice: string;
  spread: string;
  spreadPercent: string;
  timestamp: number;
}

// ─── Trade / Fill ───────────────────────────────────────────────

export interface Trade {
  id: string;
  pair: TradingPair;
  price: string;
  amount: string;
  side: OrderSide;
  makerOrderId: string;
  takerOrderId: string;
  /** Settlement info */
  settlement: TradeSettlement;
  timestamp: number;
}

export interface TradeSettlement {
  /** How the trade is settled */
  method: "atomic-swap" | "on-chain" | "off-chain";
  /** HTLC involved (for atomic swaps) */
  htlcs?: HTLC[];
  /** Settlement chain */
  chainId: ChainId;
  /** Settlement tx hash */
  txHash?: string;
  /** Settlement status */
  status: "pending" | "settling" | "settled" | "failed";
}

// ─── Route / Path Types ─────────────────────────────────────────

export interface RouteStep {
  poolId: string;
  tokenIn: string;
  tokenOut: string;
  protocol: AmmProtocol;
  vmType: VmType;
  expectedAmountOut?: string;
}

export interface TradeRoute {
  steps: RouteStep[];
  tokenStart: string;
  tokenEnd: string;
  amountIn: string;
  expectedAmountOut: string;
  estimatedGas: number;
  priceImpactBps: number;
}

export interface TradeQuote {
  fromAsset: string;
  toAsset: string;
  inputAmount: string;
  outputAmount: string;
  effectivePrice: string;
  priceImpact: number;
  route: RouteStep[];
  estimatedGas: string;
  validUntil: number;
}

// ─── Atomic Swap Types ──────────────────────────────────────────

export type SwapStatus =
  | "initiated"
  | "htlc-created-source"
  | "htlc-created-destination"
  | "claimed"
  | "refunded"
  | "expired"
  | "failed";

export interface AtomicSwap {
  /** Unique swap ID */
  id: string;
  /** Initiator address */
  initiator: string;
  /** Counterparty address */
  counterparty: string;
  /** Source chain */
  sourceChain: ChainId;
  /** Destination chain */
  destChain: ChainId;
  /** Source token */
  sourceToken: string;
  /** Destination token */
  destToken: string;
  /** Source amount */
  sourceAmount: string;
  /** Destination amount */
  destAmount: string;
  /** HTLC on source chain */
  sourceHtlc?: HTLC;
  /** HTLC on destination chain */
  destHtlc?: HTLC;
  /** Secret hash (shared) */
  hashLock: string;
  /** Secret (only initiator knows, revealed on claim) */
  secret?: string;
  /** Swap status */
  status: SwapStatus;
  /** Creation timestamp */
  createdAt: number;
  /** Expiry timestamp */
  expiresAt: number;
}

export interface SwapInitParams {
  /** Source chain */
  sourceChain: ChainId;
  /** Destination chain */
  destChain: ChainId;
  /** Source token address */
  sourceToken: string;
  /** Destination token address */
  destToken: string;
  /** Amount to swap (source chain units) */
  amount: string;
  /** Counterparty address on destination chain */
  counterparty: string;
  /** Time lock duration in seconds (default: 3600 for source, 1800 for dest) */
  timeLockSeconds?: number;
}

// ─── DEX Configuration ──────────────────────────────────────────

export interface DexConfig {
  /** Substrate RPC endpoint for x3-kernel */
  substrateRpc: string;
  /** Substrate WebSocket endpoint */
  substrateWs: string;
  /** RPC endpoints per chain */
  chainEndpoints: Partial<Record<ChainId, string>>;
  /** HTLC contract addresses per chain */
  htlcContracts: Partial<Record<ChainId, string>>;
  /** Default slippage in basis points */
  defaultSlippageBps: number;
  /** Default time lock for initiator (seconds) */
  defaultTimeLockInitiator: number;
  /** Default time lock for counterparty (seconds) */
  defaultTimeLockCounterparty: number;
  /** Orderbook WebSocket endpoint */
  orderbookWsUrl?: string;
  /** API base URL for off-chain orderbook */
  apiBaseUrl?: string;
}

// ─── Events ─────────────────────────────────────────────────────

export type DexEventType =
  | "order-created"
  | "order-filled"
  | "order-partial-fill"
  | "order-cancelled"
  | "trade-executed"
  | "swap-initiated"
  | "swap-htlc-funded"
  | "swap-claimed"
  | "swap-refunded"
  | "swap-expired"
  | "orderbook-update"
  | "price-update"
  | "error";

export interface DexEvent {
  type: DexEventType;
  timestamp: number;
  data: unknown;
}

// ─── API Responses ──────────────────────────────────────────────

export interface ApiResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}
