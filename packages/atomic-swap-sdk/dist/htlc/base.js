"use strict";
/**
 * HTLC Base — Abstract interface for Hash Time-Locked Contracts across all chains.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateSecret = generateSecret;
exports.sha256Hex = sha256Hex;
exports.sha256FromHex = sha256FromHex;
exports.bytesToHex = bytesToHex;
exports.hexToBytes = hexToBytes;
exports.calculateTimeLocks = calculateTimeLocks;
/**
 * Generate a cryptographically secure random secret for HTLC.
 * Returns { secret, hashLock } where hashLock = SHA-256(secret).
 */
function generateSecret() {
    // Use crypto.getRandomValues or Node crypto
    const secretBytes = new Uint8Array(32);
    if (typeof globalThis.crypto !== "undefined" && globalThis.crypto.getRandomValues) {
        globalThis.crypto.getRandomValues(secretBytes);
    }
    else {
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
function sha256Hex(data) {
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
    }
    catch {
        // Minimal fallback SHA-256 — in production, use @noble/hashes
        throw new Error("SHA-256 not available. Install @noble/hashes or use Node.js.");
    }
}
/**
 * Compute SHA-256 from hex string.
 */
function sha256FromHex(hexStr) {
    const clean = hexStr.startsWith("0x") ? hexStr.slice(2) : hexStr;
    const bytes = new Uint8Array(clean.match(/.{1,2}/g).map((b) => parseInt(b, 16)));
    return sha256Hex(bytes);
}
function bytesToHex(bytes) {
    return "0x" + Array.from(bytes).map((b) => b.toString(16).padStart(2, "0")).join("");
}
function hexToBytes(hex) {
    const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
    return new Uint8Array(clean.match(/.{1,2}/g).map((b) => parseInt(b, 16)));
}
/**
 * Calculate a safe time lock:
 * - Initiator gets a longer timelock (e.g., 2x)
 * - Counterparty gets a shorter timelock
 * This ensures the initiator can always claim before refunding.
 */
function calculateTimeLocks(baseDurationSeconds) {
    const now = Math.floor(Date.now() / 1000);
    return {
        initiatorTimeLock: now + baseDurationSeconds * 2,
        counterpartyTimeLock: now + baseDurationSeconds,
    };
}
//# sourceMappingURL=base.js.map