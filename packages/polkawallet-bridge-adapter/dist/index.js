"use strict";
/**
 * X3 Chain x3chain — Polkawallet Bridge Adapter
 *
 * Implements the `BaseCrossChainAdapter` interface from @polkawallet/bridge
 * to enable XCM transfers to/from x3chain, plus native x3chain atomic swaps
 * and cross-VM transfers.
 *
 * Supports:
 *   - DOT/KSM/X3 cross-chain via XCM
 *   - EVM↔x3chain asset transfers
 *   - SVM↔x3chain asset transfers
 *   - Atomic trade batches routed through the bridge
 *
 * Usage (in polkawallet-io/sdk js_api bridge.ts):
 *   import { X3ChainAdapter } from '@x3-chain/polkawallet-bridge-adapter';
 *   const x3chain = new X3ChainAdapter();
 *   await x3chain.init(x3chainApi);
 *   bridge = new Bridge({ adapters: [...existing, x3chain] });
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.x3chainChainConfig = exports.x3chainTokensConfig = exports.x3chainRouteConfigs = exports.X3ChainAdapter = void 0;
var adapter_1 = require("./adapter");
Object.defineProperty(exports, "X3ChainAdapter", { enumerable: true, get: function () { return adapter_1.X3ChainAdapter; } });
var configs_1 = require("./configs");
Object.defineProperty(exports, "x3chainRouteConfigs", { enumerable: true, get: function () { return configs_1.x3chainRouteConfigs; } });
var configs_2 = require("./configs");
Object.defineProperty(exports, "x3chainTokensConfig", { enumerable: true, get: function () { return configs_2.x3chainTokensConfig; } });
Object.defineProperty(exports, "x3chainChainConfig", { enumerable: true, get: function () { return configs_2.x3chainChainConfig; } });
//# sourceMappingURL=index.js.map