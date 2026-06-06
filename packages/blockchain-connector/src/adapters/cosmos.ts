/**
 * Cosmos Chain Adapter — Cosmos Hub, Osmosis, and other Tendermint chains.
 *
 * Uses CometBFT/Tendermint REST + RPC endpoints.
 */

import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, ChainDescriptor } from "../types";

export class CosmosAdapter extends BaseChainAdapter {
  readonly chain: ChainDescriptor;

  constructor(chain: ChainDescriptor) {
    super();
    this.chain = chain;
  }

  async getLatestBlock(): Promise<Block> {
    const result = await this.rpcCall<any>("block");
    return this.parseBlock(result);
  }

  async getBlock(numberOrHash: string | number): Promise<Block> {
    const height = String(numberOrHash);
    const result = await this.rpcCall<any>("block", [height]);
    return this.parseBlock(result);
  }

  async getTransaction(hash: string): Promise<Transaction> {
    const raw = await this.rpcCall<any>("tx", [hash, false]);

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

  async getValidators(): Promise<ValidatorInfo[]> {
    const result = await this.rpcCall<any>("validators", ["1", "1", "100"]);
    const validators = result?.validators ?? [];

    return validators.map((v: any) => ({
      address: v.address,
      stake: v.voting_power,
      active: true,
      identity: v.pub_key?.value,
    }));
  }

  async getMetrics(): Promise<ConnectorMetrics> {
    const base = await super.getMetrics();

    try {
      const status = await this.rpcCall<any>("status");
      const blockHeight = parseInt(status?.sync_info?.latest_block_height ?? "0");
      const peers = parseInt(status?.sync_info?.catching_up ? "0" : "1");

      return {
        ...base,
        blockHeight,
        peerCount: peers,
      };
    } catch {
      return base;
    }
  }

  private parseBlock(raw: any): Block {
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
