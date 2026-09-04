"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.X3ChainAdapter = void 0;
const configs_1 = require("./configs");
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
class X3ChainAdapter {
    constructor() {
        this.chain = configs_1.x3chainChainConfig;
        this.routers = configs_1.x3chainRouteConfigs;
        this.tokens = configs_1.x3chainTokensConfig;
    }
    /**
     * Initialize with a connected ApiPromise instance.
     */
    async init(api) {
        this.api = api;
        // Verify chain connection
        const chain = await api.rpc.system.chain();
        console.log(`[x3chain-adapter] Connected to: ${chain.toHuman()}`);
    }
    getApi() {
        return this.api;
    }
    getRouters() {
        return this.routers;
    }
    getSS58Prefix() {
        return this.chain.ss58Prefix;
    }
    injectFindAdapter(fn) {
        this.findAdapterFn = fn;
    }
    /**
     * Get token configuration.
     */
    getToken(token, _destChain) {
        const t = this.tokens[token];
        if (!t) {
            throw new Error(`Token ${token} not found on x3chain`);
        }
        return t;
    }
    /**
     * Get cross-chain fee for a transfer.
     */
    getCrossChainFee(token, destChain) {
        const route = this.routers.find((r) => r.to === destChain && r.token === token);
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
    subscribeBalances(address, callback) {
        if (!this.api)
            throw new Error('API not initialized');
        return this.api.derive.balances.all(address, (result) => {
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
    createTx(params) {
        if (!this.api)
            throw new Error('API not initialized');
        const { address, amount, to, token } = params;
        const toChainConfig = this._getChainConfig(to);
        // Determine if this is an XCM parachain transfer or cross-VM bridge
        if (toChainConfig?.type === 'substrate') {
            return this._createXcmTransfer(address, amount, to, token);
        }
        else {
            return this._createCrossVmTransfer(address, amount, to, token);
        }
    }
    /**
     * Create an atomic swap via the x3chain trade engine.
     * This is a unique x3chain capability beyond standard XCM.
     */
    createAtomicSwapTx(params) {
        if (!this.api)
            throw new Error('API not initialized');
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
    async resolveX3Domain(domain) {
        if (!this.api)
            throw new Error('API not initialized');
        try {
            const records = await this.api.rpc.x3Domains.getRecords(domain);
            if (!records || records.isNone)
                return null;
            const recordList = records.toJSON();
            const x3addr = recordList.find((r) => r.record_type === 'X3ADDR');
            return x3addr?.value || null;
        }
        catch {
            return null;
        }
    }
    // ─── Private Helpers ───
    _createXcmTransfer(address, amount, to, token) {
        const api = this.api;
        const accountId = api.createType('AccountId32', address).toHex();
        // Use xTokens pallet for cross-chain transfers
        const tokenId = this._tokenToAssetId(token);
        return api.tx.xTokens.transfer(tokenId, amount.toString(), {
            V3: {
                parents: 1,
                interior: {
                    X2: [
                        { Parachain: this._getParachainId(to) },
                        { AccountId32: { id: accountId, network: null } },
                    ],
                },
            },
        }, 'Unlimited');
    }
    _createCrossVmTransfer(address, amount, to, token) {
        const api = this.api;
        // Use the cross-VM bridge for EVM/SVM targets
        return api.tx.crossVmBridge?.transfer?.(address, amount.toString(), this._chainKind(to), this._tokenToAssetId(token)) || api.tx.balances.transferKeepAlive(address, amount.toString());
    }
    _tokenToAssetId(token) {
        const mapping = {
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
    _getParachainId(chain) {
        const mapping = {
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
    _getChainConfig(chain) {
        // Default to substrate type; EVM chains are special-cased
        const evmChains = ['ethereum', 'bsc', 'polygon', 'avalanche'];
        return {
            type: evmChains.includes(chain) ? 'ethereum' : 'substrate',
        };
    }
    _chainKind(chain) {
        const evmChains = ['ethereum', 'bsc', 'polygon', 'avalanche', 'moonbeam'];
        if (evmChains.includes(chain))
            return { Evm: 1 };
        return 'X3';
    }
}
exports.X3ChainAdapter = X3ChainAdapter;
//# sourceMappingURL=adapter.js.map