"use strict";
/**
 * Substrate HTLC Adapter — Manages HTLCs via the atomic-trade-engine pallet on X3 Chain.
 *
 * Uses Substrate RPC to interact with the pallet extrinsics:
 * - atomicTradeEngine.createTradeBatch
 * - atomicTradeEngine.executeTradeBatch
 * - atomicTradeEngine.cancelTradeBatch
 *
 * For direct HTLC-like behavior, we use single-leg trade batches
 * with the x3-amm protocol.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SubstrateHTLCAdapter = void 0;
exports.createSubstrateHTLCAdapter = createSubstrateHTLCAdapter;
const base_1 = require("./base");
class SubstrateHTLCAdapter {
    chainId;
    wsEndpoint;
    rpcEndpoint;
    constructor(chainId, rpcEndpoint, wsEndpoint) {
        this.chainId = chainId;
        this.rpcEndpoint = rpcEndpoint;
        this.wsEndpoint = wsEndpoint || rpcEndpoint.replace(/^http/, "ws");
    }
    async createHTLC(params, signerKey) {
        // On Substrate, an HTLC is represented as a single-leg TradeBatch
        // with the AtlasAmm protocol. The pallet handles the locking/unlocking.
        const nonce = await this.getTradeNonce(signerKey);
        const currentBlock = await this.getCurrentBlockNumber();
        const leg = {
            amm_protocol: "AtlasAmm",
            vm_type: "X3",
            asset_in: params.hashLock, // Use hashLock as asset_in identifier for HTLC correlation
            asset_out: params.tokenAddress,
            amount_in: params.amount,
            min_amount_out: params.amount, // 1:1 for HTLC (not a swap)
            route_data: this.encodeHTLCRouteData(params),
        };
        // Submit extrinsic: atomicTradeEngine.createTradeBatch
        const batchId = (0, base_1.sha256FromHex)((0, base_1.bytesToHex)(new TextEncoder().encode(`htlc:${params.hashLock}:${nonce}:${signerKey}`)));
        const extrinsic = this.buildExtrinsic("atomicTradeEngine", "createTradeBatch", [
            [leg],
            100, // slippage_tolerance_bps (not relevant for HTLC)
            currentBlock + 1000, // deadline: 1000 blocks
            nonce,
        ]);
        const txHash = await this.submitExtrinsic(extrinsic, signerKey);
        const now = Math.floor(Date.now() / 1000);
        return {
            id: batchId,
            chainId: this.chainId,
            vmType: "x3",
            hashLock: params.hashLock,
            timeLock: params.timeLock,
            sender: signerKey,
            recipient: params.recipient,
            tokenAddress: params.tokenAddress,
            amount: params.amount,
            contractAddress: "pallet:atomic-trade-engine",
            fundingTxHash: txHash,
            status: "funded",
            createdAt: now,
            updatedAt: now,
        };
    }
    async claimHTLC(params, signerKey) {
        // Execute the trade batch to "claim" the HTLC
        // The secret is embedded in the execution call
        const extrinsic = this.buildExtrinsic("atomicTradeEngine", "executeTradeBatch", [
            params.htlcId,
        ]);
        await this.submitExtrinsic(extrinsic, signerKey);
        const htlc = await this.getHTLC(params.htlcId);
        if (!htlc)
            throw new Error(`Substrate HTLC ${params.htlcId} not found`);
        return {
            ...htlc,
            secret: params.secret,
            status: "claimed",
            updatedAt: Math.floor(Date.now() / 1000),
        };
    }
    async refundHTLC(params, signerKey) {
        // Cancel the trade batch to "refund" the HTLC
        const extrinsic = this.buildExtrinsic("atomicTradeEngine", "cancelTradeBatch", [
            params.htlcId,
        ]);
        await this.submitExtrinsic(extrinsic, signerKey);
        const htlc = await this.getHTLC(params.htlcId);
        if (!htlc)
            throw new Error(`Substrate HTLC ${params.htlcId} not found`);
        return {
            ...htlc,
            status: "refunded",
            updatedAt: Math.floor(Date.now() / 1000),
        };
    }
    async getHTLC(htlcId) {
        // Query pallet storage: atomicTradeEngine.tradeBatches(htlcId)
        try {
            const result = await this.queryStorage("atomicTradeEngine", "tradeBatches", [htlcId]);
            if (!result)
                return null;
            const batch = result;
            const statusMap = {
                Pending: "funded",
                Executing: "funded",
                Completed: "claimed",
                Failed: "expired",
                Cancelled: "refunded",
            };
            return {
                id: htlcId,
                chainId: this.chainId,
                vmType: "x3",
                hashLock: batch.legs?.[0]?.asset_in || "",
                timeLock: batch.deadline || 0,
                sender: batch.origin || "",
                recipient: "",
                tokenAddress: batch.legs?.[0]?.asset_out || "",
                amount: batch.legs?.[0]?.amount_in?.toString() || "0",
                contractAddress: "pallet:atomic-trade-engine",
                status: statusMap[batch.status] || "pending",
                createdAt: batch.created_at || 0,
                updatedAt: Math.floor(Date.now() / 1000),
            };
        }
        catch {
            return null;
        }
    }
    async isHTLCFunded(htlcId) {
        const htlc = await this.getHTLC(htlcId);
        return htlc?.status === "funded";
    }
    async isHTLCClaimed(htlcId) {
        const htlc = await this.getHTLC(htlcId);
        return { claimed: htlc?.status === "claimed", secret: htlc?.secret };
    }
    async isHTLCExpired(htlcId) {
        const htlc = await this.getHTLC(htlcId);
        if (!htlc)
            return false;
        const currentBlock = await this.getCurrentBlockNumber();
        return currentBlock > htlc.timeLock;
    }
    // ─── Substrate RPC Helpers ────────────────────────────────────
    async rpcCall(method, params) {
        const resp = await fetch(this.rpcEndpoint, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
        });
        const json = await resp.json();
        if (json.error)
            throw new Error(`RPC error: ${json.error.message}`);
        return json.result;
    }
    async queryStorage(pallet, storageItem, keys) {
        const [{ ApiPromise, WsProvider }, utilCrypto, util, keyringMod] = await Promise.all([
            import("@polkadot/api"),
            import("@polkadot/util-crypto"),
            import("@polkadot/util"),
            import("@polkadot/keyring"),
        ]);
        await utilCrypto.cryptoWaitReady();
        const provider = new WsProvider(this.wsEndpoint);
        const api = await ApiPromise.create({ provider });
        try {
            const storageFn = api.query?.[pallet]?.[storageItem];
            if (!storageFn) {
                throw new Error(`Storage item ${pallet}.${storageItem} is not available`);
            }
            const normalizedKeys = keys.map((key) => {
                if (typeof key !== "string") {
                    return key;
                }
                if (key.startsWith("0x") && key.length === 66) {
                    // H256-style key
                    return key;
                }
                // If this looks like a mnemonic/URI, map to account address.
                if (key.startsWith("//") || key.includes(" ")) {
                    const keyring = new keyringMod.Keyring({ type: "sr25519" });
                    return keyring.addFromUri(key).address;
                }
                // If already SS58/hex account style, pass through.
                if (key.startsWith("5") || key.startsWith("0x")) {
                    return key;
                }
                // Fallback to bytes for raw strings.
                return util.stringToU8a(key);
            });
            const result = await storageFn(...normalizedKeys);
            return result?.toJSON?.() ?? result?.toHuman?.() ?? result;
        }
        finally {
            await api.disconnect();
        }
    }
    buildExtrinsic(pallet, call, args) {
        return { pallet, call, args };
    }
    async submitExtrinsic(encodedExtrinsic, signerKey) {
        const [{ ApiPromise, WsProvider, Keyring }, utilCrypto] = await Promise.all([
            import("@polkadot/api"),
            import("@polkadot/util-crypto"),
        ]);
        await utilCrypto.cryptoWaitReady();
        const provider = new WsProvider(this.wsEndpoint);
        const api = await ApiPromise.create({ provider });
        const keyring = new Keyring({ type: "sr25519" });
        const pair = keyring.addFromUri(signerKey);
        const txBuilder = api.tx?.[encodedExtrinsic.pallet]?.[encodedExtrinsic.call];
        if (!txBuilder) {
            await api.disconnect();
            throw new Error(`Extrinsic ${encodedExtrinsic.pallet}.${encodedExtrinsic.call} is not available`);
        }
        const tx = txBuilder(...encodedExtrinsic.args);
        const txHash = await new Promise((resolve, reject) => {
            tx.signAndSend(pair, ({ status, dispatchError, txHash }) => {
                if (dispatchError) {
                    reject(new Error(`Extrinsic failed: ${dispatchError.toString()}`));
                    return;
                }
                if (status?.isInBlock || status?.isFinalized) {
                    resolve(txHash.toHex());
                }
            }).catch(reject);
        });
        await api.disconnect();
        return txHash;
    }
    async getTradeNonce(account) {
        try {
            const result = await this.queryStorage("atomicTradeEngine", "tradeNonces", [account]);
            return Number(result) || 0;
        }
        catch {
            return 0;
        }
    }
    async getCurrentBlockNumber() {
        const header = await this.rpcCall("chain_getHeader", []);
        return parseInt(header.number, 16);
    }
    encodeHTLCRouteData(params) {
        // Encode HTLC-specific data into the route_data field
        const data = new TextEncoder().encode(JSON.stringify({
            hashLock: params.hashLock,
            timeLock: params.timeLock,
            recipient: params.recipient,
            token: params.tokenAddress,
        }));
        return (0, base_1.bytesToHex)(data);
    }
}
exports.SubstrateHTLCAdapter = SubstrateHTLCAdapter;
/**
 * Factory function to create a Substrate HTLC adapter with env var configuration.
 * Reads X3_RPC_ENDPOINT for HTTP endpoint and X3_WS_ENDPOINT for WebSocket endpoint.
 */
function createSubstrateHTLCAdapter(chainId) {
    const rpcEndpoint = process.env.X3_RPC_ENDPOINT;
    const wsEndpoint = process.env.X3_WS_ENDPOINT;
    if (!rpcEndpoint) {
        throw new Error("X3_RPC_ENDPOINT environment variable is required");
    }
    return new SubstrateHTLCAdapter(chainId, rpcEndpoint, wsEndpoint);
}
//# sourceMappingURL=substrate.js.map