"use strict";
/**
 * Tests for HTLC base utilities: generateSecret, sha256FromHex, bytesToHex,
 * hexToBytes, calculateTimeLocks.
 */
Object.defineProperty(exports, "__esModule", { value: true });
const vitest_1 = require("vitest");
const base_1 = require("../base");
(0, vitest_1.describe)("bytesToHex / hexToBytes", () => {
    (0, vitest_1.it)("converts a byte array to a 0x-prefixed hex string", () => {
        const bytes = new Uint8Array([0, 1, 255, 16]);
        (0, vitest_1.expect)((0, base_1.bytesToHex)(bytes)).toBe("0x0001ff10");
    });
    (0, vitest_1.it)("round-trips hexToBytes → bytesToHex", () => {
        const hex = "0xdeadbeef";
        (0, vitest_1.expect)((0, base_1.bytesToHex)((0, base_1.hexToBytes)(hex))).toBe(hex);
    });
    (0, vitest_1.it)("handles hex without 0x prefix", () => {
        const bytes = (0, base_1.hexToBytes)("deadbeef");
        (0, vitest_1.expect)((0, base_1.bytesToHex)(bytes)).toBe("0xdeadbeef");
    });
});
(0, vitest_1.describe)("sha256Hex", () => {
    (0, vitest_1.it)("returns a 0x-prefixed 32-byte hash", () => {
        const input = new Uint8Array(32).fill(1);
        const hash = (0, base_1.sha256Hex)(input);
        (0, vitest_1.expect)(hash).toMatch(/^0x[0-9a-f]{64}$/);
    });
    (0, vitest_1.it)("produces different hashes for different inputs", () => {
        const a = (0, base_1.sha256Hex)(new Uint8Array(32).fill(1));
        const b = (0, base_1.sha256Hex)(new Uint8Array(32).fill(2));
        (0, vitest_1.expect)(a).not.toBe(b);
    });
    (0, vitest_1.it)("is deterministic", () => {
        const input = new Uint8Array([1, 2, 3, 4, 5]);
        (0, vitest_1.expect)((0, base_1.sha256Hex)(input)).toBe((0, base_1.sha256Hex)(input));
    });
});
(0, vitest_1.describe)("sha256FromHex", () => {
    (0, vitest_1.it)("accepts a 0x-prefixed hex string", () => {
        const result = (0, base_1.sha256FromHex)("0x0102030405");
        (0, vitest_1.expect)(result).toMatch(/^0x[0-9a-f]{64}$/);
    });
    (0, vitest_1.it)("accepts a hex string without 0x prefix", () => {
        const result = (0, base_1.sha256FromHex)("0102030405");
        (0, vitest_1.expect)(result).toMatch(/^0x[0-9a-f]{64}$/);
    });
    (0, vitest_1.it)("matches sha256Hex of the same bytes", () => {
        const hex = "0xaabbccdd";
        const fromHex = (0, base_1.sha256FromHex)(hex);
        const fromBytes = (0, base_1.sha256Hex)((0, base_1.hexToBytes)(hex));
        (0, vitest_1.expect)(fromHex).toBe(fromBytes);
    });
});
(0, vitest_1.describe)("generateSecret", () => {
    (0, vitest_1.it)("returns a secret and hashLock", () => {
        const { secret, hashLock } = (0, base_1.generateSecret)();
        (0, vitest_1.expect)(secret).toMatch(/^0x[0-9a-f]{64}$/);
        (0, vitest_1.expect)(hashLock).toMatch(/^0x[0-9a-f]{64}$/);
    });
    (0, vitest_1.it)("hashLock is SHA-256 of the secret", () => {
        const { secret, hashLock } = (0, base_1.generateSecret)();
        (0, vitest_1.expect)((0, base_1.sha256FromHex)(secret)).toBe(hashLock);
    });
    (0, vitest_1.it)("generates unique secrets each call", () => {
        const a = (0, base_1.generateSecret)();
        const b = (0, base_1.generateSecret)();
        (0, vitest_1.expect)(a.secret).not.toBe(b.secret);
        (0, vitest_1.expect)(a.hashLock).not.toBe(b.hashLock);
    });
});
(0, vitest_1.describe)("calculateTimeLocks", () => {
    (0, vitest_1.it)("initiatorTimeLock is 2x the base duration past now", () => {
        const base = 3600; // 1 hour
        const before = Math.floor(Date.now() / 1000);
        const { initiatorTimeLock, counterpartyTimeLock } = (0, base_1.calculateTimeLocks)(base);
        const after = Math.floor(Date.now() / 1000);
        (0, vitest_1.expect)(initiatorTimeLock).toBeGreaterThanOrEqual(before + base * 2);
        (0, vitest_1.expect)(initiatorTimeLock).toBeLessThanOrEqual(after + base * 2 + 1);
    });
    (0, vitest_1.it)("counterpartyTimeLock is 1x the base duration past now", () => {
        const base = 3600;
        const before = Math.floor(Date.now() / 1000);
        const { counterpartyTimeLock } = (0, base_1.calculateTimeLocks)(base);
        const after = Math.floor(Date.now() / 1000);
        (0, vitest_1.expect)(counterpartyTimeLock).toBeGreaterThanOrEqual(before + base);
        (0, vitest_1.expect)(counterpartyTimeLock).toBeLessThanOrEqual(after + base + 1);
    });
    (0, vitest_1.it)("initiatorTimeLock > counterpartyTimeLock", () => {
        const { initiatorTimeLock, counterpartyTimeLock } = (0, base_1.calculateTimeLocks)(600);
        (0, vitest_1.expect)(initiatorTimeLock).toBeGreaterThan(counterpartyTimeLock);
    });
});
//# sourceMappingURL=base.test.js.map