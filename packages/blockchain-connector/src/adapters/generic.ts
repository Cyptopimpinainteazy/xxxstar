/**
 * Generic Chain Adapter — fallback for chains without dedicated adapters.
 *
 * Returns mock/simulated data so every chain in the registry can
 * show a connector status and basic metrics in the UI.
 */

import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ConnectorMetrics, ChainDescriptor } from "../types";

export class GenericAdapter extends BaseChainAdapter {
  readonly chain: ChainDescriptor;

  constructor(chain: ChainDescriptor) {
    super();
    this.chain = chain;
  }

  async getLatestBlock(): Promise<Block> {
    return {
      hash: `0x${Date.now().toString(16)}`,
      number: Math.floor(Date.now() / (this.chain.avgBlockTimeSeconds * 1000)),
      parentHash: "0x0",
      timestamp: new Date().toISOString(),
      txCount: 0,
      size: 0,
    };
  }

  async getBlock(numberOrHash: string | number): Promise<Block> {
    return {
      hash: typeof numberOrHash === "string" ? numberOrHash : `0x${numberOrHash.toString(16)}`,
      number: typeof numberOrHash === "number" ? numberOrHash : 0,
      parentHash: "0x0",
      timestamp: new Date().toISOString(),
      txCount: 0,
      size: 0,
    };
  }

  async getTransaction(hash: string): Promise<Transaction> {
    return {
      hash,
      from: "",
      value: "0",
      nonce: 0,
      status: "pending",
    };
  }

  async getMetrics(): Promise<ConnectorMetrics> {
    const base = await super.getMetrics();
    return {
      ...base,
      blockHeight: Math.floor(Date.now() / (this.chain.avgBlockTimeSeconds * 1000)),
    };
  }
}
