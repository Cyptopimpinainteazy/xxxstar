/**
 * Substrate Adapter — minimal implementation using @polkadot/api
 */

import { ApiPromise, WsProvider } from "@polkadot/api";
import { blake2AsHex } from "@polkadot/util-crypto";
import type { Block, Transaction, ChainDescriptor } from "../types";
import { BaseChainAdapter } from "./base";

export class SubstrateAdapter extends BaseChainAdapter {
  readonly chain: ChainDescriptor;
  private api?: ApiPromise;

  constructor(chain: ChainDescriptor) {
    super();
    this.chain = chain;
  }

  async connect(endpoint: string): Promise<void> {
    this.endpoint = endpoint;
    const ws = endpoint.startsWith("ws")
      ? endpoint
      : endpoint.replace(/^http/, "ws");

    const provider = new WsProvider(ws);
    this.api = await ApiPromise.create({ provider });
    this.connected = true;
    this.startTime = Date.now();
  }

  async disconnect(): Promise<void> {
    if (this.api) {
      await this.api.disconnect();
      this.api = undefined;
    }
    this.connected = false;
  }

  isConnected(): boolean {
    return this.connected && !!this.api && this.api.isConnected;
  }

  async getLatestBlock(): Promise<Block> {
    if (!this.api) throw new Error("Not connected");
    const header = await this.api.rpc.chain.getHeader();
    const hash = header.hash.toHex();
    const number = header.number.toNumber();
    const timestamp = new Date().toISOString();

    return {
      hash,
      number,
      parentHash: header.parentHash.toHex(),
      timestamp,
      txCount: 0,
      size: 0,
      raw: header.toHex(),
    };
  }

  async getBlock(numberOrHash: string | number): Promise<Block> {
    if (!this.api) throw new Error("Not connected");
    const api = this.api!;
    let hash;
    if (typeof numberOrHash === "number") {
      hash = await api.rpc.chain.getBlockHash(numberOrHash);
    } else {
      hash = numberOrHash;
    }
    const block = await api.rpc.chain.getBlock(hash as any);
    const header = block.block.header;
    return {
      hash: hash.toString(),
      number: header.number.toNumber(),
      parentHash: header.parentHash.toHex(),
      timestamp: new Date().toISOString(),
      txCount: block.block.extrinsics.length,
      size: 0,
      raw: block.toHex(),
    };
  }

  async getTransaction(hash: string): Promise<Transaction> {
    if (!this.api) throw new Error("Not connected");
    const api = this.api!;
    // Best-effort: scan last N blocks looking for an extrinsic whose computed hash matches.
    const best = (await api.rpc.chain.getHeader()).number.toNumber();
    const scan = 200; // scan recent 200 blocks at most
    for (let i = best; i > Math.max(0, best - scan); i--) {
      try {
        const bh = await api.rpc.chain.getBlockHash(i);
        const block = await api.rpc.chain.getBlock(bh);
        for (const ext of block.block.extrinsics) {
          try {
            // Use the extrinsic's own .hash accessor when available (current
            // @polkadot/api). Fall back to blake2-256 over the SCALE-encoded
            // bytes — that is the canonical Substrate extrinsic hash and
            // works across all api versions, replacing the broken
            // `registry.hash(...)` API that was removed.
            let hashHex: string | undefined;
            const extAny = ext as any;
            if (extAny.hash && typeof extAny.hash.toHex === 'function') {
              hashHex = extAny.hash.toHex();
            } else {
              const u8a = ext.toU8a
                ? ext.toU8a()
                : ext.toHex
                  ? Buffer.from(ext.toHex().replace(/^0x/, ''), 'hex')
                  : null;
              if (u8a) {
                hashHex = blake2AsHex(u8a, 256);
              }
            }
            if (hashHex) {
              if (hashHex.toLowerCase().replace(/^0x/, '') === hash.toLowerCase().replace(/^0x/, '')) {
                // found
                return {
                  hash,
                  from: "",
                  value: "0",
                  nonce: 0,
                  raw: ext.toHex ? ext.toHex() : ext.toString(),
                } as any;
              }
            }
          } catch (e) {
            // ignore per-extrinsic errors
          }
        }
      } catch (e) {
        // continue
      }
    }

    throw new Error("Transaction not found in recent blocks");
  }

  async getMetrics(): Promise<any> {
    if (!this.api) return {
      blockHeight: 0,
      tps: 0,
      peerCount: 0,
      latencyMs: 0,
      totalRequests: this.requestCount,
      totalErrors: this.errorCount,
      uptimeSeconds: Math.floor((Date.now() - this.startTime) / 1000),
      finalityLag: 0,
    };

    const header = await this.api.rpc.chain.getHeader();
    return {
      blockHeight: header.number.toNumber(),
      tps: 0,
      peerCount: 0,
      latencyMs: 0,
      totalRequests: this.requestCount,
      totalErrors: this.errorCount,
      uptimeSeconds: Math.floor((Date.now() - this.startTime) / 1000),
      finalityLag: 0,
    } as any;
  }

  async getSystemHealth(): Promise<any> {
    if (!this.api) throw new Error("Not connected");
    return await this.api.rpc.system.health();
  }

  /** Subscribe to new blocks and call handler with a canonical Block payload */
  async subscribe(events: string[], filter: any, handler: (event: any) => void): Promise<{ unsubscribe: () => void }> {
    if (!this.api) throw new Error("Not connected");
    const api = this.api!;
    const unsub = await api.rpc.chain.subscribeNewHeads(async (head) => {
      try {
        const bh = head.hash.toHex();
        const block = await api.rpc.chain.getBlock(bh);
        const header = block.block.header;
        const payload = {
          id: bh,
          type: 'block',
          connectorId: '',
          chain: this.chain.id,
          network: this.chain.network,
          timestamp: new Date().toISOString(),
          payload: {
            hash: bh,
            number: header.number.toNumber(),
            parentHash: header.parentHash.toHex(),
            timestamp: new Date().toISOString(),
            txCount: block.block.extrinsics.length,
            raw: block.toHex(),
          }
        };
        handler(payload as any);
      } catch (e) {
        // swallow
      }
    });

    return { unsubscribe: async () => { try { await unsub(); } catch {} } };
  }
}
