/**
 * HTLC Base — Abstract interface for Hash Time-Locked Contracts across all chains.
 */

import type { HTLC, HTLCCreateParams, HTLCClaimParams, HTLCRefundParams, ChainId } from "../types";

export interface IHTLCAdapter {
  /** Chain this adapter handles */
  readonly chainId: ChainId;

  /**
   * Deploy / create an HTLC with the given parameters.
   * Returns an HTLC descriptor with funding tx hash.
   */
  createHTLC(params: HTLCCreateParams, signerKey: string): Promise<HTLC>;

  /**
   * Claim an HTLC by revealing the secret preimage.
   * Returns updated HTLC with "claimed" status.
   */
  claimHTLC(params: HTLCClaimParams, signerKey: string): Promise<HTLC>;

  /**
   * Refund an expired HTLC back to sender.
   * Returns updated HTLC with "refunded" status.
   */
  refundHTLC(params: HTLCRefundParams, signerKey: string): Promise<HTLC>;

  /**
   * Query on-chain state of an HTLC.
   */
  getHTLC(htlcId: string): Promise<HTLC | null>;

  /**
   * Check if an HTLC has been funded on-chain.
   */
  isHTLCFunded(htlcId: string): Promise<boolean>;

  /**
   * Check if an HTLC has been claimed (secret revealed).
   */
  isHTLCClaimed(htlcId: string): Promise<{ claimed: boolean; secret?: string }>;

  /**
   * Check if an HTLC is expired / refundable.
   */
  isHTLCExpired(htlcId: string): Promise<boolean>;
}

/**
 * Generate a cryptographically secure random secret for HTLC.
 * Returns { secret, hashLock } where hashLock = SHA-256(secret).
 */
export function generateSecret(): { secret: string; hashLock: string } {
  // Use crypto.getRandomValues or Node crypto
  const secretBytes = new Uint8Array(32);
  if (typeof globalThis.crypto !== "undefined" && globalThis.crypto.getRandomValues) {
    globalThis.crypto.getRandomValues(secretBytes);
  } else {
    // Fallback: require("crypto") for Node.js
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const nodeCrypto = require("crypto");
    const buf = nodeCrypto.randomBytes(32);
    secretBytes.set(new Uint8Array(buf));
  }

  const secret = bytesToHex(secretBytes);
  const hashLock = sha256Hex(secretBytes);

  return { secret, hashLock };
}

/**
 * Compute SHA-256 hash of hex-encoded data.
 */
export function sha256Hex(data: Uint8Array): string {
  // Use Web Crypto (sync fallback for Node)
  if (typeof globalThis.crypto !== "undefined" && globalThis.crypto.subtle) {
    // WebCrypto is async — but we use a synchronous fallback for simplicity.
    // In production, prefer async version.
  }

  // Use Node.js crypto
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const nodeCrypto = require("crypto");
    const hash = nodeCrypto.createHash("sha256").update(Buffer.from(data)).digest();
    return "0x" + Buffer.from(hash).toString("hex");
  } catch {
    // Minimal fallback SHA-256 — in production, use @noble/hashes
    throw new Error("SHA-256 not available. Install @noble/hashes or use Node.js.");
  }
}

/**
 * Compute SHA-256 from hex string.
 */
export function sha256FromHex(hexStr: string): string {
  const clean = hexStr.startsWith("0x") ? hexStr.slice(2) : hexStr;
  const bytes = new Uint8Array(clean.match(/.{1,2}/g)!.map((b) => parseInt(b, 16)));
  return sha256Hex(bytes);
}

export function bytesToHex(bytes: Uint8Array): string {
  return "0x" + Array.from(bytes).map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  return new Uint8Array(clean.match(/.{1,2}/g)!.map((b) => parseInt(b, 16)));
}

/**
 * Calculate a safe time lock:
 * - Initiator gets a longer timelock (e.g., 2x)
 * - Counterparty gets a shorter timelock
 * This ensures the initiator can always claim before refunding.
 */
export function calculateTimeLocks(baseDurationSeconds: number): {
  initiatorTimeLock: number;
  counterpartyTimeLock: number;
} {
  const now = Math.floor(Date.now() / 1000);
  return {
    initiatorTimeLock: now + baseDurationSeconds * 2,
    counterpartyTimeLock: now + baseDurationSeconds,
  };
}
