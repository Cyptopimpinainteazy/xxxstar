/**
 * Cosmos Chain Adapter — Cosmos Hub, Osmosis, and other Tendermint chains.
 *
 * Uses CometBFT/Tendermint REST + RPC endpoints.
 */
import { BaseChainAdapter } from "./base";
export class CosmosAdapter extends BaseChainAdapter {
    chain;
    constructor(chain) {
        super();
        this.chain = chain;
    }
    async getLatestBlock() {
        const result = await this.rpcCall("block");
        return this.parseBlock(result);
    }
    async getBlock(numberOrHash) {
        const height = String(numberOrHash);
        const result = await this.rpcCall("block", [height]);
        return this.parseBlock(result);
    }
    async getTransaction(hash) {
        const raw = await this.rpcCall("tx", [hash, false]);
        return {
            hash: raw?.hash ?? hash,
            blockNumber: parseInt(raw?.height ?? "0"),
            from: "",
            value: "0",
            nonce: 0,
            status: raw?.tx_result?.code === 0 ? "success" : "reverted",
            raw,
        };
    }
    async getValidators() {
        const result = await this.rpcCall("validators", ["1", "1", "100"]);
        const validators = result?.validators ?? [];
        return validators.map((v) => ({
            address: v.address,
            stake: v.voting_power,
            active: true,
            identity: v.pub_key?.value,
        }));
    }
    async getMetrics() {
        const base = await super.getMetrics();
        try {
            const status = await this.rpcCall("status");
            const blockHeight = parseInt(status?.sync_info?.latest_block_height ?? "0");
            const peers = parseInt(status?.sync_info?.catching_up ? "0" : "1");
            return {
                ...base,
                blockHeight,
                peerCount: peers,
            };
        }
        catch {
            return base;
        }
    }
    parseBlock(raw) {
        const block = raw?.block ?? raw;
        const header = block?.header ?? {};
        return {
            hash: raw?.block_id?.hash ?? "",
            number: parseInt(header.height ?? "0"),
            parentHash: header.last_block_id?.hash ?? "",
            timestamp: header.time ?? new Date().toISOString(),
            txCount: block?.data?.txs?.length ?? 0,
            size: 0,
            raw,
        };
    }
}
//# sourceMappingURL=cosmos.js.map