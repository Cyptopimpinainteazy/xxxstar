/**
 * @x3-chain/atomic-swap-sdk
 *
 * Polkadex-inspired cross-chain DEX with atomic swap settlement.
 * Supports EVM, Solana, Bitcoin, and Substrate chains.
 *
 * Usage:
 *   import { AtlasDexClient, DexWebSocket } from '@x3-chain/atomic-swap-sdk';
 *
 *   const dex = new AtlasDexClient({
 *     chainEndpoints: { ethereum: 'https://...', solana: 'https://...', bitcoin: 'https://...' },
 *     htlcContracts: { ethereum: '0x...' },
 *     defaultTimeLockInitiator: 7200,
 *     defaultTimeLockCounterparty: 3600,
 *   });
 *
 *   await dex.initialize();
 *   dex.setSigner(privateKey);
 *
 *   const quote = dex.getQuote('ETH', 'SOL', '1.0');
 *   const { data: order } = await dex.submitOrder({ ... });
 */

// ─── Core Types ─────────────────────────────────────────
export type {
  VmType,
  ChainId,
  AmmProtocol,
  HTLC,
  HTLCCreateParams,
  HTLCClaimParams,
  HTLCRefundParams,
  SwapStatus,
  Asset,
  TokenBalance,
  Order,
  OrderCreateParams,
  OrderType,
  OrderSide,

  TradingPair,
  Orderbook,
  OrderbookLevel,
  Trade,
  TradeSettlement,
  RouteStep,
  TradeRoute,
  TradeQuote,
  AtomicSwap,
  SwapInitParams,
  DexConfig,
  DexEvent,
  DexEventType,
  ApiResult,
} from "./types";

// ─── HTLC Adapters ─────────────────────────────────────
export {
  type IHTLCAdapter,
  type HTLCAdapterConfig,
  createHTLCAdapter,
  generateSecret,
  sha256Hex,
  sha256FromHex,
  calculateTimeLocks,
  bytesToHex,
  hexToBytes,
} from "./htlc";

export { EvmHTLCAdapter } from "./htlc/evm";
export { SolanaHTLCAdapter } from "./htlc/solana";
export { BitcoinHTLCAdapter } from "./htlc/bitcoin";
export { SubstrateHTLCAdapter } from "./htlc/substrate";

// ─── Orderbook Engine ──────────────────────────────────
export { OrderbookEngine, OrderbookManager } from "./orderbook";

// ─── Swap Orchestrator ─────────────────────────────────
export { SwapOrchestrator } from "./swap/orchestrator";
export { SwapMonitor, type SwapMonitorConfig, type SwapHealthReport } from "./swap/monitor";

// ─── DEX Client ────────────────────────────────────────
export { AtlasDexClient, type DexClientEvents } from "./dex/client";
export { DexWebSocket, type WsConfig, type TickerData } from "./dex/websocket";
