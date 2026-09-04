/**
 * NEAR Chain Adapter — NEAR Mainnet and Testnet.
 */

import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, ChainDescriptor } from "../types";

export class NearAdapter extends BaseChainAdapter {
  readonly chain: ChainDescriptor;

  constructor(chain: ChainDescriptor) {
    super();
    this.chain = chain;
  }

  async getLatestBlock(): Promise<Block> {
    const raw = await this.nearRpc("block", { finality: "final" });
    return this.parseBlock(raw);
  }

  async getBlock(numberOrHash: string | number): Promise<Block> {
    const params =
      typeof numberOrHash === "number" || /^\d+$/.test(String(numberOrHash))
        ? { block_id: Number(numberOrHash) }
        : { block_id: numberOrHash };
    const raw = await this.nearRpc("block", params);
    return this.parseBlock(raw);
  }

  async getTransaction(hash: string): Promise<Transaction> {
    // NEAR requires sender_id for tx lookup; use empty string for exploratory query
    const raw = await this.nearRpc("tx", [hash, "system"]).catch(() => null);

    return {
      hash,
      blockNumber: raw?.transaction_outcome?.block_hash ? 0 : undefined,
      from: raw?.transaction?.signer_id ?? "",
      to: raw?.transaction?.receiver_id,
      value: "0",
      nonce: raw?.transaction?.nonce ?? 0,
      status: raw?.status?.SuccessValue !== undefined ? "success" : "reverted",
      raw,
    };
  }

  async getValidators(): Promise<ValidatorInfo[]> {
    const raw = await this.nearRpc("validators", [null]);
    const current = raw?.current_validators ?? [];

    return current.map((v: any) => ({
      address: v.account_id,
      stake: v.stake,
      active: true,
      uptime: v.num_expected_blocks ? (v.num_produced_blocks / v.num_expected_blocks) * 100 : 0,
      blocksProduced: v.num_produced_blocks,
    }));
  }

  async getMetrics(): Promise<ConnectorMetrics> {
    const base = await super.getMetrics();

    try {
      const status = await this.nearRpc("status", {});
      const blockHeight = status?.sync_info?.latest_block_height ?? 0;

      return {
        ...base,
        blockHeight,
        peerCount: status?.sync_info?.num_peers ?? 0,
      };
    } catch {
      return base;
    }
  }

  private parseBlock(raw: any): Block {
    const header = raw?.header ?? {};
    return {
      hash: header.hash ?? "",
      number: header.height ?? 0,
      parentHash: header.prev_hash ?? "",
      timestamp: header.timestamp
        ? new Date(header.timestamp / 1e6).toISOString()
        : new Date().toISOString(),
      txCount: raw?.chunks?.reduce((acc: number, c: any) => acc + (c.tx_root !== "11111111111111111111111111111111" ? 1 : 0), 0) ?? 0,
      size: 0,
      raw,
    };
  }

  private async nearRpc(method: string, params: unknown): Promise<any> {
    const start = Date.now();
    try {
      const res = await fetch(this.endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: "x3", method, params }),
      });
      const json = await res.json();
      this.trackRequest(start);
      if (json.error) {
        this.trackError();
        throw new Error(`NEAR RPC error: ${json.error.message ?? JSON.stringify(json.error)}`);
      }
      return json.result;
    } catch (err) {
      this.trackError();
      this.trackRequest(start);
      throw err;
    }
  }
}
