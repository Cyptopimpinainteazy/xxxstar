/**
 * Generic Chain Adapter — fallback for chains without dedicated adapters.
 *
 * Returns mock/simulated data so every chain in the registry can
 * show a connector status and basic metrics in the UI.
 */
import { BaseChainAdapter } from "./base";
export class GenericAdapter extends BaseChainAdapter {
    chain;
    constructor(chain) {
        super();
        this.chain = chain;
    }
    async getLatestBlock() {
        return {
            hash: `0x${Date.now().toString(16)}`,
            number: Math.floor(Date.now() / (this.chain.avgBlockTimeSeconds * 1000)),
            parentHash: "0x0",
            timestamp: new Date().toISOString(),
            txCount: 0,
            size: 0,
        };
    }
    async getBlock(numberOrHash) {
        return {
            hash: typeof numberOrHash === "string" ? numberOrHash : `0x${numberOrHash.toString(16)}`,
            number: typeof numberOrHash === "number" ? numberOrHash : 0,
            parentHash: "0x0",
            timestamp: new Date().toISOString(),
            txCount: 0,
            size: 0,
        };
    }
    async getTransaction(hash) {
        return {
            hash,
            from: "",
            value: "0",
            nonce: 0,
            status: "pending",
        };
    }
    async getMetrics() {
        const base = await super.getMetrics();
        return {
            ...base,
            blockHeight: Math.floor(Date.now() / (this.chain.avgBlockTimeSeconds * 1000)),
        };
    }
}
//# sourceMappingURL=generic.js.map