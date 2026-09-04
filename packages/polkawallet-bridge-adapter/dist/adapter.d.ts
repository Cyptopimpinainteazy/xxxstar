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
        fee?: {
            token: string;
            amount: string;
        };
        weightLimit?: string;
        deliveryFee?: {
            token: string;
            amount: string;
        };
    };
}
interface TransferParams {
    address: string;
    amount: any;
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
export declare class X3ChainAdapter {
    readonly chain: {
        id: "x3chain";
        display: string;
        type: "substrate";
        icon: string;
        paraChainId: number;
        ss58Prefix: number;
    };
    private routers;
    private tokens;
    private api?;
    private findAdapterFn?;
    /**
     * Initialize with a connected ApiPromise instance.
     */
    init(api: ApiPromise): Promise<void>;
    getApi(): ApiPromise | undefined;
    getRouters(): RouteConfigs[];
    getSS58Prefix(): number;
    injectFindAdapter(fn: (chain: string) => any): void;
    /**
     * Get token configuration.
     */
    getToken(token: string, _destChain?: string): X3ChainToken;
    /**
     * Get cross-chain fee for a transfer.
     */
    getCrossChainFee(token: string, destChain: string): {
        token: string;
        amount: string;
        decimals: number;
    };
    /**
     * Subscribe to balance of an address.
     */
    subscribeBalances(address: string, callback: (balances: Record<string, any>) => void): import("@polkadot/api-base/types").UnsubscribePromise;
    /**
     * Create a cross-chain transfer extrinsic.
     *
     * Routes through x3chain's XCM or cross-VM bridge depending on target.
     */
    createTx(params: TransferParams): import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>;
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
    }): import("@polkadot/api-base/types").SubmittableExtrinsic<"promise", import("@polkadot/types/types").ISubmittableResult>;
    /**
     * Resolve a .x3 domain to a substrate address for transfers.
     */
    resolveX3Domain(domain: string): Promise<string | null>;
    private _createXcmTransfer;
    private _createCrossVmTransfer;
    private _tokenToAssetId;
    private _getParachainId;
    private _getChainConfig;
    private _chainKind;
}
export {};
//# sourceMappingURL=adapter.d.ts.map