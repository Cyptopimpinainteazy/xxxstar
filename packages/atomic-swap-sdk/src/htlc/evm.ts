/**
 * EVM HTLC Adapter — Creates and manages Hash Time-Locked Contracts on EVM chains.
 *
 * Interacts with the AtlasHTLC.sol smart contract deployed on Ethereum, Polygon,
 * BSC, Arbitrum, Optimism, Base, etc.
 *
 * ABI: createHTLC(bytes32 hashLock, address recipient, address token, uint256 amount, uint256 timelock)
 *      claimHTLC(bytes32 htlcId, bytes32 secret)
 *      refundHTLC(bytes32 htlcId)
 */

import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
import { type IHTLCAdapter, sha256FromHex, bytesToHex, hexToBytes } from "./base";

// ─── ABI Selectors ──────────────────────────────────────────────
const SELECTOR_CREATE = "0x4b2f336d"; // createHTLC(bytes32,address,address,uint256,uint256)
const SELECTOR_CLAIM = "0x84cc315c"; // claimHTLC(bytes32,bytes32)
const SELECTOR_REFUND = "0x7249fbb6"; // refundHTLC(bytes32)
const SELECTOR_GET = "0x905d22a5"; // getHTLC(bytes32) → (sender,recipient,token,amount,hashLock,timeLock,status)

// HTLC status enum on contract
const EVM_STATUS_MAP: Record<number, HTLC["status"]> = {
  0: "pending",
  1: "funded",
  2: "claimed",
  3: "refunded",
  4: "expired",
};

export class EvmHTLCAdapter implements IHTLCAdapter {
  readonly chainId: ChainId;
  private rpcEndpoint: string;
  private htlcContractAddress: string;

  constructor(chainId: ChainId, rpcEndpoint: string, htlcContractAddress: string) {
    this.chainId = chainId;
    this.rpcEndpoint = rpcEndpoint;
    this.htlcContractAddress = htlcContractAddress;
  }

  async createHTLC(params: HTLCCreateParams, signerKey: string): Promise<HTLC> {
    const htlcId = this.computeHTLCId(params);
    const calldata = this.encodeCreateHTLC(params);

    const isNative = this.isNativeToken(params.tokenAddress);
    const txHash = await this.sendTransaction(
      this.htlcContractAddress,
      calldata,
      isNative ? params.amount : "0",
      signerKey,
    );

    const now = Math.floor(Date.now() / 1000);
    return {
      id: htlcId,
      chainId: this.chainId,
      vmType: "evm",
      hashLock: params.hashLock,
      timeLock: params.timeLock,
      sender: this.addressFromKey(signerKey),
      recipient: params.recipient,
      tokenAddress: params.tokenAddress,
      amount: params.amount,
      contractAddress: params.contractAddress || this.htlcContractAddress,
      fundingTxHash: txHash,
      status: "funded",
      createdAt: now,
      updatedAt: now,
    };
  }

  async claimHTLC(params: HTLCClaimParams, signerKey: string): Promise<HTLC> {
    const calldata = this.encodeClaimHTLC(params.htlcId, params.secret);
    const txHash = await this.sendTransaction(
      this.htlcContractAddress,
      calldata,
      "0",
      signerKey,
    );

    const htlc = await this.getHTLC(params.htlcId);
    if (!htlc) throw new Error(`HTLC ${params.htlcId} not found`);

    return {
      ...htlc,
      secret: params.secret,
      status: "claimed",
      updatedAt: Math.floor(Date.now() / 1000),
    };
  }

  async refundHTLC(params: HTLCRefundParams, signerKey: string): Promise<HTLC> {
    const calldata = this.encodeRefundHTLC(params.htlcId);
    await this.sendTransaction(
      this.htlcContractAddress,
      calldata,
      "0",
      signerKey,
    );

    const htlc = await this.getHTLC(params.htlcId);
    if (!htlc) throw new Error(`HTLC ${params.htlcId} not found`);

    return {
      ...htlc,
      status: "refunded",
      updatedAt: Math.floor(Date.now() / 1000),
    };
  }

  async getHTLC(htlcId: string): Promise<HTLC | null> {
    const calldata = SELECTOR_GET + this.padBytes32(htlcId).slice(2);
    const result = await this.ethCall(this.htlcContractAddress, calldata);

    if (!result || result === "0x" || result.length < 450) return null;

    const data = result.slice(2); // strip 0x
    const sender = "0x" + data.slice(24, 64);
    const recipient = "0x" + data.slice(88, 128);
    const token = "0x" + data.slice(152, 192);
    const amount = BigInt("0x" + data.slice(192, 256)).toString();
    const hashLock = "0x" + data.slice(256, 320);
    const timeLock = Number(BigInt("0x" + data.slice(320, 384)));
    const statusNum = Number(BigInt("0x" + data.slice(384, 448)));

    return {
      id: htlcId,
      chainId: this.chainId,
      vmType: "evm",
      hashLock,
      timeLock,
      sender,
      recipient,
      tokenAddress: token,
      amount,
      contractAddress: this.htlcContractAddress,
      status: EVM_STATUS_MAP[statusNum] || "pending",
      createdAt: 0,
      updatedAt: Math.floor(Date.now() / 1000),
    };
  }

  async isHTLCFunded(htlcId: string): Promise<boolean> {
    const htlc = await this.getHTLC(htlcId);
    return htlc?.status === "funded";
  }

  async isHTLCClaimed(htlcId: string): Promise<{ claimed: boolean; secret?: string }> {
    const htlc = await this.getHTLC(htlcId);
    if (htlc?.status === "claimed") {
      return { claimed: true, secret: htlc.secret };
    }
    return { claimed: false };
  }

  async isHTLCExpired(htlcId: string): Promise<boolean> {
    const htlc = await this.getHTLC(htlcId);
    if (!htlc) return false;
    const now = Math.floor(Date.now() / 1000);
    return now > htlc.timeLock;
  }

  // ─── Encoding Helpers ───────────────────────────────────────────

  private encodeCreateHTLC(params: HTLCCreateParams): string {
    return (
      SELECTOR_CREATE +
      this.padBytes32(params.hashLock).slice(2) +
      this.padAddress(params.recipient).slice(2) +
      this.padAddress(params.tokenAddress).slice(2) +
      this.padUint256(params.amount).slice(2) +
      this.padUint256(params.timeLock.toString()).slice(2)
    );
  }

  private encodeClaimHTLC(htlcId: string, secret: string): string {
    return (
      SELECTOR_CLAIM +
      this.padBytes32(htlcId).slice(2) +
      this.padBytes32(secret).slice(2)
    );
  }

  private encodeRefundHTLC(htlcId: string): string {
    return SELECTOR_REFUND + this.padBytes32(htlcId).slice(2);
  }

  private computeHTLCId(params: HTLCCreateParams): string {
    // keccak256(abi.encodePacked(hashLock, sender, recipient, token, amount, timeLock))
    // Simplified: just hash the hashLock with the contract address
    return sha256FromHex(params.hashLock + this.htlcContractAddress.slice(2));
  }

  // ─── ABI Helpers ────────────────────────────────────────────────

  private padBytes32(hex: string): string {
    const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
    return "0x" + clean.padStart(64, "0");
  }

  private padAddress(addr: string): string {
    const clean = addr.startsWith("0x") ? addr.slice(2) : addr;
    return "0x" + clean.padStart(64, "0");
  }

  private padUint256(value: string): string {
    const big = BigInt(value);
    return "0x" + big.toString(16).padStart(64, "0");
  }

  private isNativeToken(addr: string): boolean {
    return (
      addr === "0x0000000000000000000000000000000000000000" ||
      addr === "0x0" ||
      addr === ""
    );
  }

  private addressFromKey(key: string): string {
    const clean = key.startsWith("0x") ? key : `0x${key}`;
    const addr = sha256FromHex(clean).slice(2, 42);
    return `0x${addr}`;
  }

  // ─── RPC Helpers ────────────────────────────────────────────────

  private async ethCall(to: string, data: string): Promise<string> {
    const body = {
      jsonrpc: "2.0",
      id: 1,
      method: "eth_call",
      params: [{ to, data }, "latest"],
    };
    const res = await fetch(this.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const json = await res.json();
    return (json as any).result || "0x";
  }

  private async sendTransaction(
    to: string,
    data: string,
    value: string,
    signerKey: string,
  ): Promise<string> {
    const ethersMod = await import("ethers");
    const provider = new ethersMod.JsonRpcProvider(this.rpcEndpoint);
    const wallet = new ethersMod.Wallet(
      signerKey.startsWith("0x") ? signerKey : `0x${signerKey}`,
      provider,
    );

    const nonce = await provider.getTransactionCount(wallet.address, "latest");
    const feeData = await provider.getFeeData();
    const gasPrice = feeData.gasPrice ?? ethersMod.parseUnits("20", "gwei");

    const txRequest = {
      to,
      data,
      value: BigInt(value),
      gasLimit: 300000n,
      gasPrice,
      nonce,
      chainId: (await provider.getNetwork()).chainId,
    };

    const signed = await wallet.signTransaction(txRequest);
    const response = await provider.broadcastTransaction(signed);
    await response.wait();
    return response.hash;
  }

  private async getTransactionCount(_signerKey: string): Promise<number> {
    const addr = this.addressFromKey(_signerKey);
    const body = {
      jsonrpc: "2.0",
      id: 1,
      method: "eth_getTransactionCount",
      params: [addr, "latest"],
    };
    const res = await fetch(this.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const json = await res.json();
    return parseInt((json as any).result || "0x0", 16);
  }

  private async getGasPrice(): Promise<string> {
    const body = {
      jsonrpc: "2.0",
      id: 1,
      method: "eth_gasPrice",
      params: [],
    };
    const res = await fetch(this.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const json = await res.json();
    return (json as any).result || "0x0";
  }
}

/**
 * Factory function to create an EVM HTLC adapter with env var configuration.
 * Reads X3_EVM_HTLC_CONTRACT from environment.
 */
export function createEvmHTLCAdapter(chainId: ChainId, rpcEndpoint: string): EvmHTLCAdapter {
  const contractAddress = process.env.X3_EVM_HTLC_CONTRACT;
  if (!contractAddress) {
    throw new Error("X3_EVM_HTLC_CONTRACT environment variable is required");
  }
  return new EvmHTLCAdapter(chainId, rpcEndpoint, contractAddress);
}
