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

export { X3ChainAdapter } from './adapter';
export { x3chainRouteConfigs } from './configs';
export { x3chainTokensConfig, x3chainChainConfig } from './configs';
export type { X3ChainToken } from './configs';
