/**
 * EVM Chain Adapter — Ethereum, Polygon, BSC, Arbitrum, Optimism, Base, etc.
 *
 * Uses JSON-RPC over HTTP. Compatible with any eth_* endpoint.
 */

import { BaseChainAdapter } from "./base";
import type { Block, Transaction, ValidatorInfo, ConnectorMetrics, ChainDescriptor } from "../types";

interface EvmBlockRaw {
  hash: string;
  number: string;
  parentHash: string;
  timestamp: string;
  transactions: string[] | object[];
  size: string;
  miner: string;
  gasUsed: string;
  gasLimit: string;
  baseFeePerGas?: string;
  difficulty: string;
  nonce: string;
  stateRoot: string;
}

interface EvmTxRaw {
  hash: string;
  blockHash: string;
  blockNumber: string;
  from: string;
  to: string | null;
  value: string;
  input: string;
  nonce: string;
  gasPrice: string;
  gas: string;
}

export class EvmAdapter extends BaseChainAdapter {
  readonly chain: ChainDescriptor;

  constructor(chain: ChainDescriptor) {
    super();
    this.chain = chain;
  }

  async getLatestBlock(): Promise<Block> {
    const raw = await this.rpcCall<EvmBlockRaw>("eth_getBlockByNumber", ["latest", false]);
    return this.parseBlock(raw);
  }

  async getBlock(numberOrHash: string | number): Promise<Block> {
    if (typeof numberOrHash === "number" || /^\d+$/.test(String(numberOrHash))) {
      const hex = "0x" + Number(numberOrHash).toString(16);
      const raw = await this.rpcCall<EvmBlockRaw>("eth_getBlockByNumber", [hex, false]);
      return this.parseBlock(raw);
    }
    const raw = await this.rpcCall<EvmBlockRaw>("eth_getBlockByHash", [numberOrHash, false]);
    return this.parseBlock(raw);
  }

  async getTransaction(hash: string): Promise<Transaction> {
    const raw = await this.rpcCall<EvmTxRaw>("eth_getTransactionByHash", [hash]);
    return {
      hash: raw.hash,
      blockHash: raw.blockHash,
      blockNumber: parseInt(raw.blockNumber, 16),
      from: raw.from,
      to: raw.to ?? undefined,
      value: BigInt(raw.value).toString(),
      data: raw.input,
      nonce: parseInt(raw.nonce, 16),
      gasPrice: BigInt(raw.gasPrice).toString(),
      gasLimit: BigInt(raw.gas).toString(),
      raw,
    };
  }

  async getMetrics(): Promise<ConnectorMetrics> {
    const base = await super.getMetrics();

    try {
      const [heightHex, gasPriceHex, peerCountHex] = await Promise.all([
        this.rpcCall<string>("eth_blockNumber"),
        this.rpcCall<string>("eth_gasPrice").catch(() => "0x0"),
        this.rpcCall<string>("net_peerCount").catch(() => "0x0"),
      ]);

      return {
        ...base,
        blockHeight: parseInt(heightHex, 16),
        gasPrice: BigInt(gasPriceHex).toString(),
        peerCount: parseInt(peerCountHex, 16),
      };
    } catch {
      return base;
    }
  }

  async submitRawTx(signedTx: string): Promise<{ txHash: string }> {
    const hash = await this.rpcCall<string>("eth_sendRawTransaction", [signedTx]);
    return { txHash: hash };
  }

  async getBalance(address: string): Promise<string> {
    const balanceHex = await this.rpcCall<string>("eth_getBalance", [address, "latest"]);
    return BigInt(balanceHex).toString();
  }

  private parseBlock(raw: EvmBlockRaw): Block {
    return {
      hash: raw.hash,
      number: parseInt(raw.number, 16),
      parentHash: raw.parentHash,
      timestamp: new Date(parseInt(raw.timestamp, 16) * 1000).toISOString(),
      txCount: raw.transactions?.length ?? 0,
      size: parseInt(raw.size, 16),
      miner: raw.miner,
      gasUsed: BigInt(raw.gasUsed).toString(),
      gasLimit: BigInt(raw.gasLimit).toString(),
      baseFeePerGas: raw.baseFeePerGas ? BigInt(raw.baseFeePerGas).toString() : undefined,
      difficulty: raw.difficulty,
      nonce: raw.nonce,
      stateRoot: raw.stateRoot,
      raw,
    };
  }
}
