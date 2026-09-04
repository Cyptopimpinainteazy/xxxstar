/**
 * Tests for HTLC base utilities: generateSecret, sha256FromHex, bytesToHex,
 * hexToBytes, calculateTimeLocks.
 */

import { describe, it, expect } from "vitest";
import {
  generateSecret,
  sha256FromHex,
  sha256Hex,
  bytesToHex,
  hexToBytes,
  calculateTimeLocks,
} from "../base";

describe("bytesToHex / hexToBytes", () => {
  it("converts a byte array to a 0x-prefixed hex string", () => {
    const bytes = new Uint8Array([0, 1, 255, 16]);
    expect(bytesToHex(bytes)).toBe("0x0001ff10");
  });

  it("round-trips hexToBytes → bytesToHex", () => {
    const hex = "0xdeadbeef";
    expect(bytesToHex(hexToBytes(hex))).toBe(hex);
  });

  it("handles hex without 0x prefix", () => {
    const bytes = hexToBytes("deadbeef");
    expect(bytesToHex(bytes)).toBe("0xdeadbeef");
  });
});

describe("sha256Hex", () => {
  it("returns a 0x-prefixed 32-byte hash", () => {
    const input = new Uint8Array(32).fill(1);
    const hash = sha256Hex(input);
    expect(hash).toMatch(/^0x[0-9a-f]{64}$/);
  });

  it("produces different hashes for different inputs", () => {
    const a = sha256Hex(new Uint8Array(32).fill(1));
    const b = sha256Hex(new Uint8Array(32).fill(2));
    expect(a).not.toBe(b);
  });

  it("is deterministic", () => {
    const input = new Uint8Array([1, 2, 3, 4, 5]);
    expect(sha256Hex(input)).toBe(sha256Hex(input));
  });
});

describe("sha256FromHex", () => {
  it("accepts a 0x-prefixed hex string", () => {
    const result = sha256FromHex("0x0102030405");
    expect(result).toMatch(/^0x[0-9a-f]{64}$/);
  });

  it("accepts a hex string without 0x prefix", () => {
    const result = sha256FromHex("0102030405");
    expect(result).toMatch(/^0x[0-9a-f]{64}$/);
  });

  it("matches sha256Hex of the same bytes", () => {
    const hex = "0xaabbccdd";
    const fromHex = sha256FromHex(hex);
    const fromBytes = sha256Hex(hexToBytes(hex));
    expect(fromHex).toBe(fromBytes);
  });
});

describe("generateSecret", () => {
  it("returns a secret and hashLock", () => {
    const { secret, hashLock } = generateSecret();
    expect(secret).toMatch(/^0x[0-9a-f]{64}$/);
    expect(hashLock).toMatch(/^0x[0-9a-f]{64}$/);
  });

  it("hashLock is SHA-256 of the secret", () => {
    const { secret, hashLock } = generateSecret();
    expect(sha256FromHex(secret)).toBe(hashLock);
  });

  it("generates unique secrets each call", () => {
    const a = generateSecret();
    const b = generateSecret();
    expect(a.secret).not.toBe(b.secret);
    expect(a.hashLock).not.toBe(b.hashLock);
  });
});

describe("calculateTimeLocks", () => {
  it("initiatorTimeLock is 2x the base duration past now", () => {
    const base = 3600; // 1 hour
    const before = Math.floor(Date.now() / 1000);
    const { initiatorTimeLock, counterpartyTimeLock } = calculateTimeLocks(base);
    const after = Math.floor(Date.now() / 1000);

    expect(initiatorTimeLock).toBeGreaterThanOrEqual(before + base * 2);
    expect(initiatorTimeLock).toBeLessThanOrEqual(after + base * 2 + 1);
  });

  it("counterpartyTimeLock is 1x the base duration past now", () => {
    const base = 3600;
    const before = Math.floor(Date.now() / 1000);
    const { counterpartyTimeLock } = calculateTimeLocks(base);
    const after = Math.floor(Date.now() / 1000);

    expect(counterpartyTimeLock).toBeGreaterThanOrEqual(before + base);
    expect(counterpartyTimeLock).toBeLessThanOrEqual(after + base + 1);
  });

  it("initiatorTimeLock > counterpartyTimeLock", () => {
    const { initiatorTimeLock, counterpartyTimeLock } = calculateTimeLocks(600);
    expect(initiatorTimeLock).toBeGreaterThan(counterpartyTimeLock);
  });
});
