"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.DexWebSocket = exports.AtlasDexClient = exports.SwapMonitor = exports.SwapOrchestrator = exports.OrderbookManager = exports.OrderbookEngine = exports.SubstrateHTLCAdapter = exports.BitcoinHTLCAdapter = exports.SolanaHTLCAdapter = exports.EvmHTLCAdapter = exports.hexToBytes = exports.bytesToHex = exports.calculateTimeLocks = exports.sha256FromHex = exports.sha256Hex = exports.generateSecret = exports.createHTLCAdapter = void 0;
// ─── HTLC Adapters ─────────────────────────────────────
var htlc_1 = require("./htlc");
Object.defineProperty(exports, "createHTLCAdapter", { enumerable: true, get: function () { return htlc_1.createHTLCAdapter; } });
Object.defineProperty(exports, "generateSecret", { enumerable: true, get: function () { return htlc_1.generateSecret; } });
Object.defineProperty(exports, "sha256Hex", { enumerable: true, get: function () { return htlc_1.sha256Hex; } });
Object.defineProperty(exports, "sha256FromHex", { enumerable: true, get: function () { return htlc_1.sha256FromHex; } });
Object.defineProperty(exports, "calculateTimeLocks", { enumerable: true, get: function () { return htlc_1.calculateTimeLocks; } });
Object.defineProperty(exports, "bytesToHex", { enumerable: true, get: function () { return htlc_1.bytesToHex; } });
Object.defineProperty(exports, "hexToBytes", { enumerable: true, get: function () { return htlc_1.hexToBytes; } });
var evm_1 = require("./htlc/evm");
Object.defineProperty(exports, "EvmHTLCAdapter", { enumerable: true, get: function () { return evm_1.EvmHTLCAdapter; } });
var solana_1 = require("./htlc/solana");
Object.defineProperty(exports, "SolanaHTLCAdapter", { enumerable: true, get: function () { return solana_1.SolanaHTLCAdapter; } });
var bitcoin_1 = require("./htlc/bitcoin");
Object.defineProperty(exports, "BitcoinHTLCAdapter", { enumerable: true, get: function () { return bitcoin_1.BitcoinHTLCAdapter; } });
var substrate_1 = require("./htlc/substrate");
Object.defineProperty(exports, "SubstrateHTLCAdapter", { enumerable: true, get: function () { return substrate_1.SubstrateHTLCAdapter; } });
// ─── Orderbook Engine ──────────────────────────────────
var orderbook_1 = require("./orderbook");
Object.defineProperty(exports, "OrderbookEngine", { enumerable: true, get: function () { return orderbook_1.OrderbookEngine; } });
Object.defineProperty(exports, "OrderbookManager", { enumerable: true, get: function () { return orderbook_1.OrderbookManager; } });
// ─── Swap Orchestrator ─────────────────────────────────
var orchestrator_1 = require("./swap/orchestrator");
Object.defineProperty(exports, "SwapOrchestrator", { enumerable: true, get: function () { return orchestrator_1.SwapOrchestrator; } });
var monitor_1 = require("./swap/monitor");
Object.defineProperty(exports, "SwapMonitor", { enumerable: true, get: function () { return monitor_1.SwapMonitor; } });
// ─── DEX Client ────────────────────────────────────────
var client_1 = require("./dex/client");
Object.defineProperty(exports, "AtlasDexClient", { enumerable: true, get: function () { return client_1.AtlasDexClient; } });
var websocket_1 = require("./dex/websocket");
Object.defineProperty(exports, "DexWebSocket", { enumerable: true, get: function () { return websocket_1.DexWebSocket; } });
//# sourceMappingURL=index.js.map