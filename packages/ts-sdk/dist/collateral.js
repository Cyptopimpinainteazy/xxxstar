"use strict";
// Collateral module — TypeScript SDK for Bonding APIs
Object.defineProperty(exports, "__esModule", { value: true });
exports.CollateralManagerClient = void 0;
class CollateralManagerClient {
    endpoint;
    id = 0;
    constructor(endpoint) {
        this.endpoint = endpoint;
        this.endpoint = endpoint.replace(/\/$/, '');
    }
    async rpcCall(method, params) {
        const request = {
            jsonrpc: '2.0',
            method,
            params,
            id: ++this.id,
        };
        try {
            const response = await fetch(`${this.endpoint}/rpc`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request),
            });
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }
            const result = (await response.json());
            return result.result;
        }
        catch {
            // Fallback for demo/testing - remove in production
            throw new Error('RPC call failed - ensure X3 Chain node is running');
        }
    }
    async depositBond(account, asset, amount) {
        const result = await this.rpcCall('collateral_depositBond', { account, asset, amount: amount.toString() });
        return {
            bondId: result.bondId,
            txHash: result.txHash,
        };
    }
    async requestWithdrawBond(account, bondId) {
        const result = await this.rpcCall('collateral_requestWithdrawBond', { account, bondId });
        return {
            requestId: result.requestId,
            bondId,
            status: result.status,
        };
    }
    async finalizeWithdraw(requestId) {
        return this.rpcCall('collateral_finalizeWithdraw', { requestId });
    }
    async getBondState(bondId) {
        const result = await this.rpcCall('collateral_getBondState', { bondId });
        return result.state;
    }
    // ---------------------------------------------------------------------------
    // GAP-6 plan-named API surface
    //
    // The legacy bond-lifecycle methods above (depositBond / requestWithdrawBond
    // / finalizeWithdraw / getBondState) describe a multi-step bond. The methods
    // below expose the higher-level lock / unlock / status verbs called for in
    // GAPS_REPORT_2026_04_27 §GAP-6, mapping directly onto the same RPC layer.
    // They are real RPC calls, not stubs: any failure surfaces from rpcCall.
    // ---------------------------------------------------------------------------
    /** Get the total collateral balance for an account+asset pair. */
    async getCollateral(account, asset) {
        const result = await this.rpcCall('collateral_getBalance', { account, asset });
        return {
            account,
            asset,
            locked: BigInt(result.locked ?? '0'),
            available: BigInt(result.available ?? '0'),
        };
    }
    /** Lock collateral. Thin alias around `depositBond` for the named API. */
    async lockCollateral(account, asset, amount) {
        return this.depositBond(account, asset, amount);
    }
    /** Begin unlocking collateral. Returns the pending withdrawal request. */
    async unlockCollateral(account, bondId) {
        return this.requestWithdrawBond(account, bondId);
    }
    /** Status of a specific bond / collateral position. */
    async getCollateralStatus(bondId) {
        const state = await this.getBondState(bondId);
        return { bondId, state };
    }
}
exports.CollateralManagerClient = CollateralManagerClient;
//# sourceMappingURL=collateral.js.map