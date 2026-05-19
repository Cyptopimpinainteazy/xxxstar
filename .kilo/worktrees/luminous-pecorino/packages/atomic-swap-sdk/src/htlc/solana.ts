/**
 * Solana HTLC Adapter — Creates and manages HTLCs on Solana via a deployed Anchor program.
 *
 * The HTLC program stores state in a PDA derived from [b"htlc", hashLock].
 * Instructions: initialize, claim, refund, get_htlc.
 */

import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
import { type IHTLCAdapter, sha256FromHex, bytesToHex, hexToBytes } from "./base";
import * as web3 from "@solana/web3.js";

/** Anchor instruction discriminators (first 8 bytes of SHA-256 hash of instruction name) */
const IX_INITIALIZE = [175, 175, 109, 31, 13, 152, 155, 237]; // sha256("global:initialize")[..8]
const IX_CLAIM = [62, 198, 214, 193, 213, 159, 108, 210];     // sha256("global:claim")[..8]
const IX_REFUND = [132, 87, 16, 18, 159, 124, 252, 157];      // sha256("global:refund")[..8]

export class SolanaHTLCAdapter implements IHTLCAdapter {
  readonly chainId: ChainId;
  private rpcEndpoint: string;
  private programId: string;

  constructor(chainId: ChainId, rpcEndpoint: string, programId: string) {
    this.chainId = chainId;
    this.rpcEndpoint = rpcEndpoint;
    this.programId = programId;
  }

  async createHTLC(params: HTLCCreateParams, signerKey: string): Promise<HTLC> {
    const hashLockBytes = hexToBytes(params.hashLock);
    const htlcPda = this.deriveHTLCPda(hashLockBytes);

    // Build initialize instruction
    const instructionData = new Uint8Array([
      ...IX_INITIALIZE,
      ...hashLockBytes,                                    // hash_lock: [u8; 32]
      ...this.encodeLittleEndianU64(params.timeLock),      // time_lock: i64
      ...this.encodeLittleEndianU64(Number(params.amount)),// amount: u64
    ]);

    const txSig = await this.sendSolanaTransaction(
      this.programId,
      instructionData,
      signerKey,
      [
        { pubkey: signerKey, isSigner: true, isWritable: true },
        { pubkey: htlcPda, isSigner: false, isWritable: true },
        { pubkey: params.recipient, isSigner: false, isWritable: false },
      ],
    );

    const now = Math.floor(Date.now() / 1000);
    return {
      id: htlcPda,
      chainId: this.chainId,
      vmType: "svm",
      hashLock: params.hashLock,
      timeLock: params.timeLock,
      sender: signerKey,
      recipient: params.recipient,
      tokenAddress: params.tokenAddress,
      amount: params.amount,
      contractAddress: this.programId,
      fundingTxHash: txSig,
      status: "funded",
      createdAt: now,
      updatedAt: now,
    };
  }

  async claimHTLC(params: HTLCClaimParams, signerKey: string): Promise<HTLC> {
    const secretBytes = hexToBytes(params.secret);

    const instructionData = new Uint8Array([
      ...IX_CLAIM,
      ...secretBytes, // secret: [u8; 32]
    ]);

    await this.sendSolanaTransaction(
      this.programId,
      instructionData,
      signerKey,
      [
        { pubkey: signerKey, isSigner: true, isWritable: true },
        { pubkey: params.htlcId, isSigner: false, isWritable: true },
      ],
    );

    const htlc = await this.getHTLC(params.htlcId);
    if (!htlc) throw new Error(`HTLC ${params.htlcId} not found after claim`);

    return {
      ...htlc,
      secret: params.secret,
      status: "claimed",
      updatedAt: Math.floor(Date.now() / 1000),
    };
  }

  async refundHTLC(params: HTLCRefundParams, signerKey: string): Promise<HTLC> {
    const instructionData = new Uint8Array([...IX_REFUND]);

    await this.sendSolanaTransaction(
      this.programId,
      instructionData,
      signerKey,
      [
        { pubkey: signerKey, isSigner: true, isWritable: true },
        { pubkey: params.htlcId, isSigner: false, isWritable: true },
      ],
    );

    const htlc = await this.getHTLC(params.htlcId);
    if (!htlc) throw new Error(`HTLC ${params.htlcId} not found after refund`);

    return { ...htlc, status: "refunded", updatedAt: Math.floor(Date.now() / 1000) };
  }

  async getHTLC(htlcId: string): Promise<HTLC | null> {
    try {
      const accountInfo = await this.getAccountInfo(htlcId);
      if (!accountInfo || !accountInfo.data) return null;

      // Decode account data (Anchor discriminator + fields)
      const data = Buffer.from(accountInfo.data[0], "base64");
      if (data.length < 8 + 32 + 32 + 32 + 32 + 8 + 8 + 1) return null;

      let offset = 8; // skip discriminator
      const sender = this.bs58Encode(data.slice(offset, offset + 32)); offset += 32;
      const recipient = this.bs58Encode(data.slice(offset, offset + 32)); offset += 32;
      const token = this.bs58Encode(data.slice(offset, offset + 32)); offset += 32;
      const hashLock = bytesToHex(new Uint8Array(data.slice(offset, offset + 32))); offset += 32;
      const amount = this.decodeLittleEndianU64(data, offset).toString(); offset += 8;
      const timeLock = this.decodeLittleEndianU64(data, offset); offset += 8;
      const status = data[offset];

      const statusMap: Record<number, HTLC["status"]> = {
        0: "funded",
        1: "claimed",
        2: "refunded",
        3: "expired",
      };

      return {
        id: htlcId,
        chainId: this.chainId,
        vmType: "svm",
        hashLock,
        timeLock: Number(timeLock),
        sender,
        recipient,
        tokenAddress: token,
        amount,
        contractAddress: this.programId,
        status: statusMap[status] || "pending",
        createdAt: 0,
        updatedAt: Math.floor(Date.now() / 1000),
      };
    } catch {
      return null;
    }
  }

  async isHTLCFunded(htlcId: string): Promise<boolean> {
    const htlc = await this.getHTLC(htlcId);
    return htlc?.status === "funded";
  }

  async isHTLCClaimed(htlcId: string): Promise<{ claimed: boolean; secret?: string }> {
    const htlc = await this.getHTLC(htlcId);
    return { claimed: htlc?.status === "claimed", secret: htlc?.secret };
  }

  async isHTLCExpired(htlcId: string): Promise<boolean> {
    const htlc = await this.getHTLC(htlcId);
    if (!htlc) return false;
    return Math.floor(Date.now() / 1000) > htlc.timeLock;
  }

  // ─── Solana Helpers ───────────────────────────────────────────

  private deriveHTLCPda(hashLockBytes: Uint8Array): string {
    // PDA = findProgramAddress([b"htlc", hashLock], programId)
    const [pda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("htlc"), ...hashLockBytes],
      new web3.PublicKey(this.programId)
    );
    return pda.toBase58();
  }

  private encodeLittleEndianU64(value: number): Uint8Array {
    const buf = new ArrayBuffer(8);
    const view = new DataView(buf);
    view.setBigUint64(0, BigInt(value), true); // little-endian
    return new Uint8Array(buf);
  }

  private decodeLittleEndianU64(data: Buffer, offset: number): bigint {
    const view = new DataView(data.buffer, data.byteOffset + offset, 8);
    return view.getBigUint64(0, true);
  }

  private bs58Encode(bytes: Buffer | Uint8Array): string {
    // Minimal base58 — in production use @solana/web3.js PublicKey
    const ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let num = BigInt("0x" + Buffer.from(bytes).toString("hex"));
    let result = "";
    while (num > 0) {
      const remainder = Number(num % 58n);
      num = num / 58n;
      result = ALPHABET[remainder] + result;
    }
    // Preserve leading zeros
    for (const byte of bytes) {
      if (byte === 0) result = "1" + result;
      else break;
    }
    return result || "1";
  }

  private async getAccountInfo(pubkey: string): Promise<any> {
    const body = {
      jsonrpc: "2.0",
      id: 1,
      method: "getAccountInfo",
      params: [pubkey, { encoding: "base64" }],
    };
    const res = await fetch(this.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const json = await res.json();
    return (json as any)?.result?.value;
  }

  private async sendSolanaTransaction(
    programId: string,
    instructionData: Uint8Array,
    signerKey: string,
    accounts: Array<{ pubkey: string; isSigner: boolean; isWritable: boolean }>,
  ): Promise<string> {
    const web3 = await import("@solana/web3.js");
    const bs58 = await import("bs58");

    const connection = new web3.Connection(this.rpcEndpoint, "confirmed");

    const raw = signerKey.startsWith("0x")
      ? hexToBytes(signerKey)
      : new Uint8Array(bs58.default.decode(signerKey));

    const keypair = web3.Keypair.fromSecretKey(Uint8Array.from(raw));
    const ix = new web3.TransactionInstruction({
      programId: new web3.PublicKey(programId),
      keys: accounts.map((a) => ({
        pubkey: new web3.PublicKey(a.pubkey),
        isSigner: a.isSigner,
        isWritable: a.isWritable,
      })),
      data: Buffer.from(instructionData),
    });

    const blockhash = await connection.getLatestBlockhash("confirmed");
    const tx = new web3.Transaction({
      feePayer: keypair.publicKey,
      blockhash: blockhash.blockhash,
      lastValidBlockHeight: blockhash.lastValidBlockHeight,
    }).add(ix);

    const signature = await connection.sendTransaction(tx, [keypair], {
      skipPreflight: false,
      preflightCommitment: "confirmed",
    });
    await connection.confirmTransaction(
      {
        signature,
        blockhash: blockhash.blockhash,
        lastValidBlockHeight: blockhash.lastValidBlockHeight,
      },
      "confirmed",
    );
    return signature;
  }
}

/**
 * Factory function to create a Solana HTLC adapter with env var configuration.
 * Reads X3_SOLANA_HTLC_PROGRAM_ID from environment.
 */
export function createSolanaHTLCAdapter(chainId: ChainId, rpcEndpoint: string): SolanaHTLCAdapter {
  const programId = process.env.X3_SOLANA_HTLC_PROGRAM_ID;
  if (!programId) {
    throw new Error("X3_SOLANA_HTLC_PROGRAM_ID environment variable is required");
  }
  return new SolanaHTLCAdapter(chainId, rpcEndpoint, programId);
}
