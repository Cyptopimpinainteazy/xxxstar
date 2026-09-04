/**
 * Bitcoin Chain Adapter — Uses Blockstream/Esplora REST API.
 */

import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ConnectorMetrics, ChainDescriptor } from "../types";

export class BitcoinAdapter extends BaseChainAdapter {
  readonly chain: ChainDescriptor;

  constructor(chain: ChainDescriptor) {
    super();
    this.chain = chain;
  }

  async getLatestBlock(): Promise<Block> {
    const tipHash = await this.httpGetText("blocks/tip/hash");
    return this.getBlock(tipHash);
  }

  async getBlock(numberOrHash: string | number): Promise<Block> {
    let hash: string;
    if (typeof numberOrHash === "number" || /^\d+$/.test(String(numberOrHash))) {
      hash = await this.httpGetText(`block-height/${numberOrHash}`);
    } else {
      hash = String(numberOrHash);
    }

    const raw = await this.httpGet<any>(`block/${hash}`);

    return {
      hash: raw.id,
      number: raw.height,
      parentHash: raw.previousblockhash ?? "",
      timestamp: raw.timestamp ? new Date(raw.timestamp * 1000).toISOString() : new Date().toISOString(),
      txCount: raw.tx_count ?? 0,
      size: raw.size ?? 0,
      miner: undefined,
      difficulty: raw.difficulty?.toString(),
      nonce: raw.nonce?.toString(),
      raw,
    };
  }

  async getTransaction(hash: string): Promise<Transaction> {
    const raw = await this.httpGet<any>(`tx/${hash}`);

    const firstInput = raw.vin?.[0];
    const firstOutput = raw.vout?.[0];

    return {
      hash: raw.txid,
      blockHash: raw.status?.block_hash,
      blockNumber: raw.status?.block_height,
      from: firstInput?.prevout?.scriptpubkey_address ?? "coinbase",
      to: firstOutput?.scriptpubkey_address,
      value: (firstOutput?.value ?? 0).toString(),
      nonce: 0,
      status: raw.status?.confirmed ? "success" : "pending",
      raw,
    };
  }

  async getMetrics(): Promise<ConnectorMetrics> {
    const base = await super.getMetrics();

    try {
      const tipHeight = await this.httpGetText("blocks/tip/height");
      const hashrate = await this.httpGet<any>("mining/hashrate/1w").catch(() => null);

      return {
        ...base,
        blockHeight: parseInt(tipHeight),
        hashRate: hashrate?.currentHashrate?.toString(),
        tps: 7, // Bitcoin's approximate TPS
      };
    } catch {
      return base;
    }
  }

  private async httpGetText(path: string): Promise<string> {
    const start = Date.now();
    try {
      const url = this.endpoint.endsWith("/") ? this.endpoint + path : `${this.endpoint}/${path}`;
      const res = await fetch(url);
      const text = await res.text();
      this.trackRequest(start);
      return text.trim();
    } catch (err) {
      this.trackError();
      this.trackRequest(start);
      throw err;
    }
  }
}
