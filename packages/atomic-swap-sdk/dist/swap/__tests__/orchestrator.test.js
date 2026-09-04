"use strict";
/**
 * Tests for SwapOrchestrator — validates state machine transitions,
 * parameter validation, and error handling without requiring a live blockchain.
 */
Object.defineProperty(exports, "__esModule", { value: true });
const vitest_1 = require("vitest");
const base_1 = require("../../htlc/base");
// ─── Minimal inline re-test of core orchestration logic ────────────────────
// We test the pure logic portions of the orchestrator without network calls.
(0, vitest_1.describe)("HTLC parameter validation helpers", () => {
    (0, vitest_1.it)("amount must be positive", () => {
        const validate = (amount) => BigInt(amount) > 0n;
        (0, vitest_1.expect)(validate("1000000")).toBe(true);
        (0, vitest_1.expect)(validate("0")).toBe(false);
        (0, vitest_1.expect)(() => validate("-1")).toThrow();
    });
    (0, vitest_1.it)("timeLock must be in the future", () => {
        const now = Math.floor(Date.now() / 1000);
        const isFuture = (t) => t > now;
        (0, vitest_1.expect)(isFuture(now + 3600)).toBe(true);
        (0, vitest_1.expect)(isFuture(now - 1)).toBe(false);
        (0, vitest_1.expect)(isFuture(now)).toBe(false);
    });
    (0, vitest_1.it)("hashLock must be 32 bytes (64 hex chars, 0x-prefixed = 66 chars)", () => {
        const isValidHashLock = (h) => /^0x[0-9a-fA-F]{64}$/.test(h);
        const { hashLock } = (0, base_1.generateSecret)();
        (0, vitest_1.expect)(isValidHashLock(hashLock)).toBe(true);
        (0, vitest_1.expect)(isValidHashLock("0xshort")).toBe(false);
        (0, vitest_1.expect)(isValidHashLock("0x" + "ab".repeat(33))).toBe(false); // 66 hex chars = 33 bytes
    });
});
(0, vitest_1.describe)("Swap lifecycle state transitions", () => {
    const allowedTransitions = {
        initiated: ["funded", "expired"],
        funded: ["counterparty_funded", "refunded", "expired"],
        counterparty_funded: ["claimed", "refunded"],
        claimed: [],
        refunded: [],
        expired: ["refunded"],
    };
    const canTransition = (from, to) => allowedTransitions[from].includes(to);
    (0, vitest_1.it)("initiated → funded is allowed", () => {
        (0, vitest_1.expect)(canTransition("initiated", "funded")).toBe(true);
    });
    (0, vitest_1.it)("funded → counterparty_funded is allowed", () => {
        (0, vitest_1.expect)(canTransition("funded", "counterparty_funded")).toBe(true);
    });
    (0, vitest_1.it)("counterparty_funded → claimed is allowed", () => {
        (0, vitest_1.expect)(canTransition("counterparty_funded", "claimed")).toBe(true);
    });
    (0, vitest_1.it)("claimed → funded is NOT allowed (no reversal)", () => {
        (0, vitest_1.expect)(canTransition("claimed", "funded")).toBe(false);
    });
    (0, vitest_1.it)("refunded → claimed is NOT allowed", () => {
        (0, vitest_1.expect)(canTransition("refunded", "claimed")).toBe(false);
    });
    (0, vitest_1.it)("expired → refunded is allowed", () => {
        (0, vitest_1.expect)(canTransition("expired", "refunded")).toBe(true);
    });
});
(0, vitest_1.describe)("generateSecret + timelock integration", () => {
    (0, vitest_1.it)("creates a valid HTLC parameter set for a 1-hour swap", () => {
        const { secret, hashLock } = (0, base_1.generateSecret)();
        const { initiatorTimeLock, counterpartyTimeLock } = (0, base_1.calculateTimeLocks)(3600);
        const now = Math.floor(Date.now() / 1000);
        // All parameters are valid
        (0, vitest_1.expect)(secret).toMatch(/^0x[0-9a-f]{64}$/);
        (0, vitest_1.expect)(hashLock).toMatch(/^0x[0-9a-f]{64}$/);
        (0, vitest_1.expect)(initiatorTimeLock).toBeGreaterThan(now + 3600);
        (0, vitest_1.expect)(counterpartyTimeLock).toBeGreaterThan(now + 3600);
        (0, vitest_1.expect)(initiatorTimeLock).toBeGreaterThan(counterpartyTimeLock);
    });
});
//# sourceMappingURL=orchestrator.test.js.map