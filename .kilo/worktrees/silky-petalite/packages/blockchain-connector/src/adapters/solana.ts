/**
 * Solana Chain Adapter — Mainnet-beta, Devnet, Testnet.
 *
 * Uses Solana JSON-RPC over HTTP.
 */

import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, ChainDescriptor } from "../types";

export class SolanaAdapter extends BaseChainAdapter {
  readonly chain: ChainDescriptor;

  constructor(chain: ChainDescriptor) {
    super();
    this.chain = chain;
  }

  async getLatestBlock(): Promise<Block> {
    const slot = await this.rpcCall<number>("getSlot");
    return this.getBlock(slot);
  }

  async getBlock(numberOrHash: string | number): Promise<Block> {
    const slot = typeof numberOrHash === "string" ? parseInt(numberOrHash) : numberOrHash;
    const raw = await this.rpcCall<any>("getBlock", [
      slot,
      { encoding: "json", transactionDetails: "signatures", maxSupportedTransactionVersion: 0 },
    ]);

    return {
      hash: raw?.blockhash ?? "",
      number: slot,
      parentHash: raw?.previousBlockhash ?? "",
      timestamp: raw?.blockTime ? new Date(raw.blockTime * 1000).toISOString() : new Date().toISOString(),
      txCount: raw?.signatures?.length ?? raw?.transactions?.length ?? 0,
      size: 0,
      raw,
    };
  }

  async getTransaction(hash: string): Promise<Transaction> {
    const raw = await this.rpcCall<any>("getTransaction", [
      hash,
      { encoding: "json", maxSupportedTransactionVersion: 0 },
    ]);

    return {
      hash,
      blockNumber: raw?.slot,
      from: raw?.transaction?.message?.accountKeys?.[0] ?? "",
      to: raw?.transaction?.message?.accountKeys?.[1] ?? undefined,
      value: "0",
      nonce: 0,
      status: raw?.meta?.err ? "reverted" : "success",
      raw,
    };
  }

  async getValidators(): Promise<ValidatorInfo[]> {
    const result = await this.rpcCall<{ current: any[]; delinquent: any[] }>("getVoteAccounts");

    return [
      ...result.current.map((v: any) => ({
        address: v.votePubkey,
        stake: v.activatedStake?.toString(),
        commission: v.commission,
        active: true,
        uptime: 100,
        lastVotedSlot: v.lastVote,
        identity: v.nodePubkey,
      })),
      ...result.delinquent.map((v: any) => ({
        address: v.votePubkey,
        stake: v.activatedStake?.toString(),
        commission: v.commission,
        active: false,
        uptime: 0,
        lastVotedSlot: v.lastVote,
        identity: v.nodePubkey,
      })),
    ];
  }

  async getMetrics(): Promise<ConnectorMetrics> {
    const base = await super.getMetrics();

    try {
      const [slot, perf, health] = await Promise.all([
        this.rpcCall<number>("getSlot"),
        this.rpcCall<any[]>("getRecentPerformanceSamples", [1]).catch(() => []),
        this.rpcCall<string>("getHealth").catch(() => "unknown"),
      ]);

      const tps = perf?.[0] ? perf[0].numTransactions / perf[0].samplePeriodSecs : 0;

      return {
        ...base,
        blockHeight: slot,
        tps: Math.round(tps),
        peerCount: 0,
      };
    } catch {
      return base;
    }
  }
}
