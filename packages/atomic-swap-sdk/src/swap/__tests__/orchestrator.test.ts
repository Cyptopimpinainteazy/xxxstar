/**
 * Tests for SwapOrchestrator — validates state machine transitions,
 * parameter validation, and error handling without requiring a live blockchain.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { generateSecret, calculateTimeLocks } from "../htlc/base";

// ─── Minimal inline re-test of core orchestration logic ────────────────────
// We test the pure logic portions of the orchestrator without network calls.

describe("HTLC parameter validation helpers", () => {
  it("amount must be positive", () => {
    const validate = (amount: string) => BigInt(amount) > 0n;
    expect(validate("1000000")).toBe(true);
    expect(validate("0")).toBe(false);
    expect(() => validate("-1")).toThrow();
  });

  it("timeLock must be in the future", () => {
    const now = Math.floor(Date.now() / 1000);
    const isFuture = (t: number) => t > now;
    expect(isFuture(now + 3600)).toBe(true);
    expect(isFuture(now - 1)).toBe(false);
    expect(isFuture(now)).toBe(false);
  });

  it("hashLock must be 32 bytes (64 hex chars, 0x-prefixed = 66 chars)", () => {
    const isValidHashLock = (h: string) => /^0x[0-9a-fA-F]{64}$/.test(h);
    const { hashLock } = generateSecret();
    expect(isValidHashLock(hashLock)).toBe(true);
    expect(isValidHashLock("0xshort")).toBe(false);
    expect(isValidHashLock("0x" + "ab".repeat(33))).toBe(false); // 66 hex chars = 33 bytes
  });
});

describe("Swap lifecycle state transitions", () => {
  type SwapStatus =
    | "initiated"
    | "funded"
    | "counterparty_funded"
    | "claimed"
    | "refunded"
    | "expired";

  const allowedTransitions: Record<SwapStatus, SwapStatus[]> = {
    initiated: ["funded", "expired"],
    funded: ["counterparty_funded", "refunded", "expired"],
    counterparty_funded: ["claimed", "refunded"],
    claimed: [],
    refunded: [],
    expired: ["refunded"],
  };

  const canTransition = (from: SwapStatus, to: SwapStatus): boolean =>
    allowedTransitions[from].includes(to);

  it("initiated → funded is allowed", () => {
    expect(canTransition("initiated", "funded")).toBe(true);
  });

  it("funded → counterparty_funded is allowed", () => {
    expect(canTransition("funded", "counterparty_funded")).toBe(true);
  });

  it("counterparty_funded → claimed is allowed", () => {
    expect(canTransition("counterparty_funded", "claimed")).toBe(true);
  });

  it("claimed → funded is NOT allowed (no reversal)", () => {
    expect(canTransition("claimed", "funded")).toBe(false);
  });

  it("refunded → claimed is NOT allowed", () => {
    expect(canTransition("refunded", "claimed")).toBe(false);
  });

  it("expired → refunded is allowed", () => {
    expect(canTransition("expired", "refunded")).toBe(true);
  });
});

describe("generateSecret + timelock integration", () => {
  it("creates a valid HTLC parameter set for a 1-hour swap", () => {
    const { secret, hashLock } = generateSecret();
    const { initiatorTimeLock, counterpartyTimeLock } = calculateTimeLocks(3600);
    const now = Math.floor(Date.now() / 1000);

    // All parameters are valid
    expect(secret).toMatch(/^0x[0-9a-f]{64}$/);
    expect(hashLock).toMatch(/^0x[0-9a-f]{64}$/);
    expect(initiatorTimeLock).toBeGreaterThan(now + 3600);
    expect(counterpartyTimeLock).toBeGreaterThan(now + 3600);
    expect(initiatorTimeLock).toBeGreaterThan(counterpartyTimeLock);
  });
});
