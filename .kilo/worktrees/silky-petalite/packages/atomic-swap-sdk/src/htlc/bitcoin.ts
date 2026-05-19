/**
 * Bitcoin HTLC Adapter — Creates and manages HTLCs using Bitcoin Script.
 *
 * Uses P2SH or P2WSH scripts with:
 *   OP_IF
 *     OP_SHA256 <hashLock> OP_EQUALVERIFY <recipientPubKey> OP_CHECKSIG
 *   OP_ELSE
 *     <timeLock> OP_CHECKLOCKTIMEVERIFY OP_DROP <senderPubKey> OP_CHECKSIG
 *   OP_ENDIF
 *
 * Uses Blockstream/Esplora REST API for querying.
 */

import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";
import { type IHTLCAdapter, sha256FromHex, bytesToHex, hexToBytes } from "./base";

export class BitcoinHTLCAdapter implements IHTLCAdapter {
  readonly chainId: ChainId;
  private apiEndpoint: string; // Esplora REST API
  private network: "mainnet" | "testnet" | "signet";
  private htlcCache = new Map<
    string,
    {
      redeemScriptHex: string;
      contractAddress: string;
      timeLock: number;
      hashLock: string;
      sender: string;
      recipient: string;
      amount: string;
    }
  >();

  constructor(
    chainId: ChainId,
    apiEndpoint: string,
    network: "mainnet" | "testnet" | "signet" = "testnet",
  ) {
    this.chainId = chainId;
    this.apiEndpoint = apiEndpoint;
    this.network = network;
  }

  async createHTLC(params: HTLCCreateParams, signerKey: string): Promise<HTLC> {
    // Build the HTLC redeem script
    const redeemScript = this.buildRedeemScript(
      params.hashLock,
      params.recipient,
      signerKey,
      params.timeLock,
    );

    const scriptHash = sha256FromHex(bytesToHex(redeemScript));
    const contractAddress = await this.scriptHashToAddress(scriptHash, redeemScript);
    const txid = await this.fundHtlc(contractAddress, params.amount, signerKey);

    this.htlcCache.set(scriptHash, {
      redeemScriptHex: bytesToHex(redeemScript),
      contractAddress,
      timeLock: params.timeLock,
      hashLock: params.hashLock,
      sender: signerKey,
      recipient: params.recipient,
      amount: params.amount,
    });

    const now = Math.floor(Date.now() / 1000);
    return {
      id: scriptHash,
      chainId: this.chainId,
      vmType: "cross-vm", // Bitcoin doesn't have a "VM" per se
      hashLock: params.hashLock,
      timeLock: params.timeLock,
      sender: signerKey,
      recipient: params.recipient,
      tokenAddress: "BTC",
      amount: params.amount,
      contractAddress,
      fundingTxHash: txid,
      status: "funded",
      createdAt: now,
      updatedAt: now,
    };
  }

  async claimHTLC(params: HTLCClaimParams, signerKey: string): Promise<HTLC> {
    const htlc = await this.getHTLC(params.htlcId);
    if (!htlc) throw new Error(`BTC HTLC ${params.htlcId} not found`);

    const cached = this.htlcCache.get(params.htlcId);
    if (!cached) {
      throw new Error(`Missing local HTLC metadata for ${params.htlcId}`);
    }

    await this.spendHtlc({
      signerKey,
      destinationAddress: await this.addressFromPrivateKey(signerKey),
      redeemScriptHex: cached.redeemScriptHex,
      htlcAddress: cached.contractAddress,
      secretHex: params.secret,
      refund: false,
      locktime: cached.timeLock,
    });

    return {
      ...htlc,
      secret: params.secret,
      status: "claimed",
      updatedAt: Math.floor(Date.now() / 1000),
    };
  }

  async refundHTLC(params: HTLCRefundParams, signerKey: string): Promise<HTLC> {
    const htlc = await this.getHTLC(params.htlcId);
    if (!htlc) throw new Error(`BTC HTLC ${params.htlcId} not found`);

    const expired = await this.isHTLCExpired(params.htlcId);
    if (!expired) {
      throw new Error("HTLC timelock has not expired yet");
    }

    const cached = this.htlcCache.get(params.htlcId);
    if (!cached) {
      throw new Error(`Missing local HTLC metadata for ${params.htlcId}`);
    }

    await this.spendHtlc({
      signerKey,
      destinationAddress: await this.addressFromPrivateKey(signerKey),
      redeemScriptHex: cached.redeemScriptHex,
      htlcAddress: cached.contractAddress,
      refund: true,
      locktime: cached.timeLock,
    });

    return {
      ...htlc,
      status: "refunded",
      updatedAt: Math.floor(Date.now() / 1000),
    };
  }

  async getHTLC(htlcId: string): Promise<HTLC | null> {
    const cached = this.htlcCache.get(htlcId);
    if (!cached) return null;
    const address = cached.contractAddress;

    try {
      const utxos = await this.fetchJson<any[]>(`address/${address}/utxo`);

      if (!utxos || utxos.length === 0) {
        // No UTXOs — either claimed/refunded or never funded
        return null;
      }

      // HTLC is funded if there are unspent outputs
      const totalSats = utxos.reduce((sum: number, u: any) => sum + (u.value || 0), 0);

      return {
        id: htlcId,
        chainId: this.chainId,
        vmType: "cross-vm",
        hashLock: cached.hashLock,
        timeLock: cached.timeLock,
        sender: cached.sender,
        recipient: cached.recipient,
        tokenAddress: "BTC",
        amount: totalSats.toString() || cached.amount,
        contractAddress: address,
        status: "funded",
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
    // Check if the HTLC UTXO has been spent.
    // If spent via the OP_IF branch, extract the secret from the spending tx's scriptSig.
    const cached = this.htlcCache.get(htlcId);
    if (!cached) return { claimed: false };
    const address = cached.contractAddress;

    try {
      const txs = await this.fetchJson<any[]>(`address/${address}/txs`);
      if (!txs) return { claimed: false };

      // Look for a spending transaction that reveals the secret
      for (const tx of txs) {
        for (const vin of tx.vin || []) {
          if (vin.witness && vin.witness.length >= 3) {
            // P2WSH: witness = [sig, secret, redeemScript]
            const possibleSecret = vin.witness[1];
            if (possibleSecret && possibleSecret.length === 64) {
              return { claimed: true, secret: "0x" + possibleSecret };
            }
          }
          if (vin.scriptsig_asm) {
            // P2SH: look for the 32-byte secret in scriptSig
            const parts = vin.scriptsig_asm.split(" ");
            for (const part of parts) {
              if (part.length === 64 && /^[0-9a-f]+$/i.test(part)) {
                return { claimed: true, secret: "0x" + part };
              }
            }
          }
        }
      }
    } catch {
      // ignore
    }

    return { claimed: false };
  }

  async isHTLCExpired(htlcId: string): Promise<boolean> {
    const cached = this.htlcCache.get(htlcId);
    if (!cached) return false;

    const tip = await this.fetchJson<number>("blocks/tip/height");
    return tip >= cached.timeLock;
  }

  // ─── Bitcoin Script Builder ───────────────────────────────────

  /**
   * Build an HTLC redeem script.
   *
   * OP_IF
   *   OP_SHA256 <hashLock> OP_EQUALVERIFY <recipientPubKey> OP_CHECKSIG
   * OP_ELSE
   *   <timeLock> OP_CHECKLOCKTIMEVERIFY OP_DROP <senderPubKey> OP_CHECKSIG
   * OP_ENDIF
   */
  private buildRedeemScript(
    hashLock: string,
    recipientPubKey: string,
    senderPubKey: string,
    timeLock: number,
  ): Uint8Array {
    const hlBytes = hexToBytes(hashLock);
    const recipBytes = hexToBytes(recipientPubKey);
    const senderBytes = hexToBytes(senderPubKey);
    const tlBytes = this.encodeScriptNumber(timeLock);

    const script: number[] = [];

    // OP_IF
    script.push(0x63);

    // OP_SHA256
    script.push(0xa8);

    // PUSH hashLock (32 bytes)
    script.push(0x20); // OP_PUSHBYTES_32
    script.push(...hlBytes);

    // OP_EQUALVERIFY
    script.push(0x88);

    // PUSH recipientPubKey
    script.push(recipBytes.length);
    script.push(...recipBytes);

    // OP_CHECKSIG
    script.push(0xac);

    // OP_ELSE
    script.push(0x67);

    // PUSH timeLock
    script.push(tlBytes.length);
    script.push(...tlBytes);

    // OP_CHECKLOCKTIMEVERIFY
    script.push(0xb1);

    // OP_DROP
    script.push(0x75);

    // PUSH senderPubKey
    script.push(senderBytes.length);
    script.push(...senderBytes);

    // OP_CHECKSIG
    script.push(0xac);

    // OP_ENDIF
    script.push(0x68);

    return new Uint8Array(script);
  }

  private encodeScriptNumber(num: number): Uint8Array {
    if (num === 0) return new Uint8Array([0]);

    const negative = num < 0;
    let absNum = Math.abs(num);
    const result: number[] = [];

    while (absNum > 0) {
      result.push(absNum & 0xff);
      absNum >>= 8;
    }

    // If the most significant byte has the high bit set, add a sign byte
    if (result[result.length - 1] & 0x80) {
      result.push(negative ? 0x80 : 0x00);
    } else if (negative) {
      result[result.length - 1] |= 0x80;
    }

    return new Uint8Array(result);
  }

  private async scriptHashToAddress(_scriptHash: string, redeemScript: Uint8Array): Promise<string> {
    const bitcoin = await import("bitcoinjs-lib");
    const network = this.getBitcoinNetwork(bitcoin);
    const payment = bitcoin.payments.p2wsh({
      redeem: { output: Buffer.from(redeemScript) },
      network,
    });

    if (!payment.address) {
      throw new Error("Unable to derive P2WSH address");
    }

    return payment.address;
  }

  // ─── API Helpers ──────────────────────────────────────────────

  private async fetchJson<T>(path: string): Promise<T> {
    const url = this.apiEndpoint.endsWith("/")
      ? this.apiEndpoint + path
      : `${this.apiEndpoint}/${path}`;
    const res = await fetch(url);
    return res.json() as Promise<T>;
  }

  private async fetchText(path: string): Promise<string> {
    const url = this.apiEndpoint.endsWith("/")
      ? this.apiEndpoint + path
      : `${this.apiEndpoint}/${path}`;
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error(`HTTP ${res.status} for ${path}`);
    }
    return res.text();
  }

  private getBitcoinNetwork(bitcoin: any) {
    if (this.network === "mainnet") return bitcoin.networks.bitcoin;
    return bitcoin.networks.testnet;
  }

  private async addressFromPrivateKey(signerKey: string): Promise<string> {
    const bitcoin = await import("bitcoinjs-lib");
    const ecc = await import("tiny-secp256k1");
    const ecpairMod = await import("ecpair");
    const ECPair = ecpairMod.ECPairFactory(ecc.default);

    const network = this.getBitcoinNetwork(bitcoin);
    const keyPair = signerKey.startsWith("K") || signerKey.startsWith("L") || signerKey.startsWith("c")
      ? ECPair.fromWIF(signerKey, network)
      : ECPair.fromPrivateKey(Buffer.from((signerKey.startsWith("0x") ? signerKey.slice(2) : signerKey), "hex"), { network });

    const p2wpkh = bitcoin.payments.p2wpkh({ pubkey: Buffer.from(keyPair.publicKey), network });
    if (!p2wpkh.address) {
      throw new Error("Unable to derive sender address");
    }
    return p2wpkh.address;
  }

  private async fundHtlc(htlcAddress: string, amountSats: string, signerKey: string): Promise<string> {
    const bitcoin = await import("bitcoinjs-lib");
    const ecc = await import("tiny-secp256k1");
    const ecpairMod = await import("ecpair");
    const ECPair = ecpairMod.ECPairFactory(ecc.default);

    const network = this.getBitcoinNetwork(bitcoin);
    const senderAddress = await this.addressFromPrivateKey(signerKey);
    const utxos = await this.fetchJson<Array<{ txid: string; vout: number; value: number }>>(
      `address/${senderAddress}/utxo`,
    );

    if (!utxos.length) {
      throw new Error(`No spendable UTXOs found for ${senderAddress}`);
    }

    const target = Number(amountSats);
    const fee = 1000;
    let selected: Array<{ txid: string; vout: number; value: number }> = [];
    let totalIn = 0;

    for (const utxo of utxos) {
      selected.push(utxo);
      totalIn += utxo.value;
      if (totalIn >= target + fee) break;
    }

    if (totalIn < target + fee) {
      throw new Error("Insufficient BTC balance for HTLC funding");
    }

    const keyPair = signerKey.startsWith("K") || signerKey.startsWith("L") || signerKey.startsWith("c")
      ? ECPair.fromWIF(signerKey, network)
      : ECPair.fromPrivateKey(Buffer.from((signerKey.startsWith("0x") ? signerKey.slice(2) : signerKey), "hex"), { network });

    const p2wpkh = bitcoin.payments.p2wpkh({ pubkey: Buffer.from(keyPair.publicKey), network });
    if (!p2wpkh.output) throw new Error("Unable to derive sender witness program");

    const psbt = new bitcoin.Psbt({ network });
    for (const utxo of selected) {
      psbt.addInput({
        hash: utxo.txid,
        index: utxo.vout,
        witnessUtxo: {
          script: p2wpkh.output,
          value: utxo.value,
        },
      });
    }

    psbt.addOutput({
      address: htlcAddress,
      value: target,
    });

    const change = totalIn - target - fee;
    if (change > 546) {
      psbt.addOutput({
        address: senderAddress,
        value: change,
      });
    }

    for (let i = 0; i < selected.length; i++) {
      psbt.signInput(i, keyPair as any);
    }
    psbt.finalizeAllInputs();

    const rawHex = psbt.extractTransaction().toHex();
    return this.broadcastRawTransaction(rawHex);
  }

  private async spendHtlc(params: {
    signerKey: string;
    destinationAddress: string;
    redeemScriptHex: string;
    htlcAddress: string;
    secretHex?: string;
    refund: boolean;
    locktime: number;
  }): Promise<string> {
    const bitcoin = await import("bitcoinjs-lib");
    const ecc = await import("tiny-secp256k1");
    const ecpairMod = await import("ecpair");
    const ECPair = ecpairMod.ECPairFactory(ecc.default);

    const network = this.getBitcoinNetwork(bitcoin);
    const utxos = await this.fetchJson<Array<{ txid: string; vout: number; value: number }>>(
      `address/${params.htlcAddress}/utxo`,
    );
    if (!utxos.length) {
      throw new Error("No spendable HTLC UTXO found");
    }

    const utxo = utxos[0];
    const fee = 800;
    const outValue = utxo.value - fee;
    if (outValue <= 546) {
      throw new Error("HTLC UTXO too small to spend after fee");
    }

    const keyPair = params.signerKey.startsWith("K") || params.signerKey.startsWith("L") || params.signerKey.startsWith("c")
      ? ECPair.fromWIF(params.signerKey, network)
      : ECPair.fromPrivateKey(Buffer.from((params.signerKey.startsWith("0x") ? params.signerKey.slice(2) : params.signerKey), "hex"), { network });

    const redeemScript = Buffer.from(
      params.redeemScriptHex.startsWith("0x") ? params.redeemScriptHex.slice(2) : params.redeemScriptHex,
      "hex",
    );

    const p2wsh = bitcoin.payments.p2wsh({ redeem: { output: redeemScript }, network });
    if (!p2wsh.output) throw new Error("Unable to derive HTLC witness program");

    const psbt = new bitcoin.Psbt({ network });
    psbt.setVersion(2);
    if (params.refund) {
      psbt.setLocktime(params.locktime);
    }

    psbt.addInput({
      hash: utxo.txid,
      index: utxo.vout,
      witnessUtxo: {
        script: p2wsh.output,
        value: utxo.value,
      },
      witnessScript: redeemScript,
      sequence: params.refund ? 0xfffffffe : 0xffffffff,
    });

    psbt.addOutput({
      address: params.destinationAddress,
      value: outValue,
    });

    psbt.signInput(0, keyPair as any);
    psbt.finalizeInput(0, (inputIndex: number, input: any) => {
      const sig = input.partialSig?.[0]?.signature;
      if (!sig) {
        throw new Error(`Missing signature for input ${inputIndex}`);
      }

      const witness = params.refund
        ? [sig, Buffer.alloc(0), redeemScript]
        : [
            sig,
            Buffer.from((params.secretHex || "").replace(/^0x/, ""), "hex"),
            Buffer.from([1]),
            redeemScript,
          ];

      return {
        finalScriptWitness: this.witnessStackToScriptWitness(witness),
      };
    });

    const rawHex = psbt.extractTransaction().toHex();
    return this.broadcastRawTransaction(rawHex);
  }

  private witnessStackToScriptWitness(witness: Buffer[]): Buffer {
    const varuintEncode = (value: number): Buffer => {
      if (value < 0xfd) return Buffer.from([value]);
      if (value <= 0xffff) {
        const b = Buffer.allocUnsafe(3);
        b[0] = 0xfd;
        b.writeUInt16LE(value, 1);
        return b;
      }
      if (value <= 0xffffffff) {
        const b = Buffer.allocUnsafe(5);
        b[0] = 0xfe;
        b.writeUInt32LE(value, 1);
        return b;
      }
      const b = Buffer.allocUnsafe(9);
      b[0] = 0xff;
      b.writeBigUInt64LE(BigInt(value), 1);
      return b;
    };

    const parts: Buffer[] = [varuintEncode(witness.length)];
    for (const item of witness) {
      parts.push(varuintEncode(item.length));
      parts.push(item);
    }
    return Buffer.concat(parts);
  }

  private async broadcastRawTransaction(rawHex: string): Promise<string> {
    const url = this.apiEndpoint.endsWith("/")
      ? `${this.apiEndpoint}tx`
      : `${this.apiEndpoint}/tx`;

    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: rawHex,
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Esplora broadcast failed: ${res.status} ${text}`);
    }

    return (await res.text()).trim();
  }
}
