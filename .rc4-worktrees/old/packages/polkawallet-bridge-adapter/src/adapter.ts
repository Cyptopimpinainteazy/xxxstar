/**
 * X3 Chain x3chain — Polkawallet Bridge Adapter Implementation
 *
 * Implements BaseCrossChainAdapter from @polkawallet/bridge for integration
 * into the Polkawallet mobile wallet. Supports:
 *   - Standard XCM transfers (DOT, KSM, X3, stablecoins)
 *   - Cross-VM transfers (EVM ↔ Substrate ↔ SVM)
 *   - Atomic swaps via x3chain's atomic-trade-engine pallet
 *   - .x3 domain resolution for recipient addresses
 */

import { ApiPromise } from '@polkadot/api';
import { x3chainRouteConfigs, x3chainTokensConfig, x3chainChainConfig } from './configs';
import type { X3ChainToken } from './configs';

/**
 * Minimal interface matching @polkawallet/bridge BaseCrossChainAdapter.
 * We define it here to avoid hard peer-dep on the bridge package at dev time.
 */
interface RouteConfigs {
  from: string;
  to: string;
  token: string;
  xcm?: {
    fee?: { token: string; amount: string };
    weightLimit?: string;
    deliveryFee?: { token: string; amount: string };
  };
}

interface TransferParams {
  address: string;
  amount: any; // FixedPointNumber from @acala-network/sdk-core
  to: string;
  token: string;
}

/**
 * X3ChainAdapter — the Polkawallet bridge adapter for X3 Chain.
 *
 * Drop-in compatible with the Polkawallet bridge SDK:
 *
 *   import { X3ChainAdapter } from '@x3-chain/polkawallet-bridge-adapter';
 *   const adapter = new X3ChainAdapter();
 *   await adapter.init(apiPromise);
 *   bridge = new Bridge({ adapters: [...existing, adapter] });
 */
export class X3ChainAdapter {
  readonly chain = x3chainChainConfig;
  private routers: RouteConfigs[] = x3chainRouteConfigs;
  private tokens: Record<string, X3ChainToken> = x3chainTokensConfig;
  private api?: ApiPromise;
  private findAdapterFn?: (chain: string) => any;

  /**
   * Initialize with a connected ApiPromise instance.
   */
  async init(api: ApiPromise) {
    this.api = api;

    // Verify chain connection
    const chain = await api.rpc.system.chain();
    console.log(`[x3chain-adapter] Connected to: ${chain.toHuman()}`);
  }

  getApi() {
    return this.api;
  }

  getRouters(): RouteConfigs[] {
    return this.routers;
  }

  getSS58Prefix(): number {
    return this.chain.ss58Prefix;
  }

  injectFindAdapter(fn: (chain: string) => any) {
    this.findAdapterFn = fn;
  }

  /**
   * Get token configuration.
   */
  getToken(token: string, _destChain?: string): X3ChainToken {
    const t = this.tokens[token];
    if (!t) {
      throw new Error(`Token ${token} not found on x3chain`);
    }
    return t;
  }

  /**
   * Get cross-chain fee for a transfer.
   */
  getCrossChainFee(token: string, destChain: string) {
    const route = this.routers.find(
      (r) => r.to === destChain && r.token === token
    );
    if (!route || !route.xcm?.fee) {
      throw new Error(`No route found: ${token} → ${destChain}`);
    }
    const feeToken = route.xcm.fee.token || token;
    const tokenConfig = this.tokens[feeToken];
    return {
      token: feeToken,
      amount: route.xcm.fee.amount,
      decimals: tokenConfig?.decimals ?? 18,
    };
  }

  /**
   * Subscribe to balance of an address.
   */
  subscribeBalances(address: string, callback: (balances: Record<string, any>) => void) {
    if (!this.api) throw new Error('API not initialized');

    return this.api.derive.balances.all(address, (result: any) => {
      callback({
        X3: {
          free: result.freeBalance.toString(),
          locked: result.lockedBalance.toString(),
          reserved: result.reservedBalance.toString(),
          available: result.availableBalance.toString(),
        },
      });
    });
  }

  /**
   * Create a cross-chain transfer extrinsic.
   *
   * Routes through x3chain's XCM or cross-VM bridge depending on target.
   */
  createTx(params: TransferParams) {
    if (!this.api) throw new Error('API not initialized');

    const { address, amount, to, token } = params;
    const toChainConfig = this._getChainConfig(to);

    // Determine if this is an XCM parachain transfer or cross-VM bridge
    if (toChainConfig?.type === 'substrate') {
      return this._createXcmTransfer(address, amount, to, token);
    } else {
      return this._createCrossVmTransfer(address, amount, to, token);
    }
  }

  /**
   * Create an atomic swap via the x3chain trade engine.
   * This is a unique x3chain capability beyond standard XCM.
   */
  createAtomicSwapTx(params: {
    tokenIn: string;
    tokenOut: string;
    amountIn: string;
    minAmountOut: string;
    chainTarget: 'Native' | 'Evm' | 'Svm' | 'X3';
  }) {
    if (!this.api) throw new Error('API not initialized');

    const assetIn = this._tokenToAssetId(params.tokenIn);
    const assetOut = this._tokenToAssetId(params.tokenOut);

    return this.api.tx.atomicTradeEngine.createTradeBatch([
      {
        asset_in: assetIn,
        asset_out: assetOut,
        amount_in: params.amountIn,
        min_amount_out: params.minAmountOut,
        chain_target: params.chainTarget,
      },
    ]);
  }

  /**
   * Resolve a .x3 domain to a substrate address for transfers.
   */
  async resolveX3Domain(domain: string): Promise<string | null> {
    if (!this.api) throw new Error('API not initialized');

    try {
      const records = await (this.api.rpc as any).x3Domains.getRecords(domain);
      if (!records || records.isNone) return null;
      const recordList = records.toJSON() as any[];
      const x3addr = recordList.find((r: any) => r.record_type === 'X3ADDR');
      return x3addr?.value || null;
    } catch {
      return null;
    }
  }

  // ─── Private Helpers ───

  private _createXcmTransfer(address: string, amount: any, to: string, token: string) {
    const api = this.api!;
    const accountId = api.createType('AccountId32', address).toHex();

    // Use xTokens pallet for cross-chain transfers
    const tokenId = this._tokenToAssetId(token);

    return api.tx.xTokens.transfer(
      tokenId,
      amount.toString(),
      {
        V3: {
          parents: 1,
          interior: {
            X2: [
              { Parachain: this._getParachainId(to) },
              { AccountId32: { id: accountId, network: null } },
            ],
          },
        },
      },
      'Unlimited'
    );
  }

  private _createCrossVmTransfer(address: string, amount: any, to: string, token: string) {
    const api = this.api!;

    // Use the cross-VM bridge for EVM/SVM targets
    return api.tx.crossVmBridge?.transfer?.(
      address,
      amount.toString(),
      this._chainKind(to),
      this._tokenToAssetId(token)
    ) || api.tx.balances.transferKeepAlive(address, amount.toString());
  }

  private _tokenToAssetId(token: string): number {
    const mapping: Record<string, number> = {
      X3: 0,
      DOT: 1,
      KSM: 2,
      USDT: 3,
      USDC: 4,
      WETH: 5,
      WBTC: 6,
    };
    return mapping[token] ?? 0;
  }

  private _getParachainId(chain: string): number {
    const mapping: Record<string, number> = {
      acala: 2000,
      moonbeam: 2004,
      astar: 2006,
      hydradx: 2034,
      interlay: 2032,
      bifrost: 2030,
      assetHubPolkadot: 1000,
      assetHubKusama: 1000,
      khala: 2004,
    };
    return mapping[chain] ?? 0;
  }

  private _getChainConfig(chain: string) {
    // Default to substrate type; EVM chains are special-cased
    const evmChains = ['ethereum', 'bsc', 'polygon', 'avalanche'];
    return {
      type: evmChains.includes(chain) ? 'ethereum' : 'substrate' as const,
    };
  }

  private _chainKind(chain: string) {
    const evmChains = ['ethereum', 'bsc', 'polygon', 'avalanche', 'moonbeam'];
    if (evmChains.includes(chain)) return { Evm: 1 };
    return 'X3';
  }
}
