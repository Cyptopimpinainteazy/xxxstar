"use strict";
/**
 * Utility functions for X3 Chain SDK
 *
 * Provides encoding, hashing, and conversion utilities.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.hexToBytes = hexToBytes;
exports.bytesToHex = bytesToHex;
exports.stringToBytes = stringToBytes;
exports.toBytes = toBytes;
exports.toHex = toHex;
exports.blake2_256 = blake2_256;
exports.blake2_256_bytes = blake2_256_bytes;
exports.computePrepareRoot = computePrepareRoot;
exports.computeComitId = computeComitId;
exports.encodeU128 = encodeU128;
exports.decodeU128 = decodeU128;
exports.encodeU64 = encodeU64;
exports.decodeU64 = decodeU64;
exports.base58DecodeSolana = base58DecodeSolana;
exports.base58Decode = base58Decode;
exports.decodeAccountId = decodeAccountId;
exports.encodeAccountId = encodeAccountId;
exports.accountIdToEvmAddress = accountIdToEvmAddress;
exports.evmAddressToAccountId = evmAddressToAccountId;
exports.isValidEvmAddress = isValidEvmAddress;
exports.isValidSolanaPubkey = isValidSolanaPubkey;
exports.validatePayloadSizes = validatePayloadSizes;
exports.isValidH256 = isValidH256;
exports.validateBalance = validateBalance;
exports.validateNonce = validateNonce;
exports.formatBalance = formatBalance;
exports.parseBalance = parseBalance;
exports.truncateHash = truncateHash;
exports.sleep = sleep;
exports.retry = retry;
const util_crypto_1 = require("@polkadot/util-crypto");
const util_1 = require("@polkadot/util");
const constants_1 = require("./constants");
const errors_1 = require("./errors");
// =============================================================================
// Encoding Utilities
// =============================================================================
/**
 * Convert a hex string to Uint8Array
 */
function hexToBytes(hex) {
    return (0, util_1.hexToU8a)(hex);
}
/**
 * Convert a Uint8Array to hex string
 */
function bytesToHex(bytes) {
    return (0, util_1.u8aToHex)(bytes);
}
/**
 * Convert a string to Uint8Array (UTF-8)
 */
function stringToBytes(str) {
    return (0, util_1.stringToU8a)(str);
}
/**
 * Ensure input is Uint8Array (convert from hex if needed)
 */
function toBytes(input) {
    if (input instanceof Uint8Array) {
        return input;
    }
    return hexToBytes(input);
}
/**
 * Ensure input is hex string (convert from bytes if needed)
 */
function toHex(input) {
    if (typeof input === 'string') {
        return input.startsWith('0x') ? input : `0x${input}`;
    }
    return bytesToHex(input);
}
// =============================================================================
// Hashing Utilities
// =============================================================================
/**
 * Compute BLAKE2-256 hash of input
 */
function blake2_256(data) {
    const input = typeof data === 'string' ? (0, util_1.stringToU8a)(data) : data;
    return (0, util_crypto_1.blake2AsHex)(input, 256);
}
/**
 * Compute BLAKE2-256 hash as bytes
 */
function blake2_256_bytes(data) {
    const input = typeof data === 'string' ? (0, util_1.stringToU8a)(data) : data;
    return (0, util_crypto_1.blake2AsU8a)(input, 256);
}
/**
 * Compute prepare_root for a Comit from its inputs
 * This matches the runtime's prepare_root computation
 */
function computePrepareRoot(origin, evmPayload, svmPayload, nonce, fee) {
    // Concatenate all inputs in order
    const originBytes = decodeAccountId(origin);
    const nonceBytes = encodeU128(nonce);
    const feeBytes = encodeU128(fee);
    const combined = new Uint8Array(originBytes.length + evmPayload.length + svmPayload.length + nonceBytes.length + feeBytes.length);
    let offset = 0;
    combined.set(originBytes, offset);
    offset += originBytes.length;
    combined.set(evmPayload, offset);
    offset += evmPayload.length;
    combined.set(svmPayload, offset);
    offset += svmPayload.length;
    combined.set(nonceBytes, offset);
    offset += nonceBytes.length;
    combined.set(feeBytes, offset);
    return blake2_256(combined);
}
/**
 * Compute Comit ID from prepare_root
 */
function computeComitId(prepareRoot) {
    return blake2_256(hexToBytes(prepareRoot));
}
// =============================================================================
// Number Encoding
// =============================================================================
/**
 * Encode a bigint as little-endian U128 (16 bytes)
 */
function encodeU128(value) {
    const bytes = new Uint8Array(16);
    let remaining = value;
    for (let i = 0; i < 16; i++) {
        bytes[i] = Number(remaining & 0xffn);
        remaining = remaining >> 8n;
    }
    return bytes;
}
/**
 * Decode a little-endian U128 to bigint
 */
function decodeU128(bytes) {
    if (bytes.length !== 16) {
        throw new errors_1.ValidationError('bytes', 'U128 must be 16 bytes', bytes.length);
    }
    let value = 0n;
    for (let i = 15; i >= 0; i--) {
        value = (value << 8n) | BigInt(bytes[i]);
    }
    return value;
}
/**
 * Encode a bigint as little-endian U64 (8 bytes)
 */
function encodeU64(value) {
    const bytes = new Uint8Array(8);
    let remaining = value;
    for (let i = 0; i < 8; i++) {
        bytes[i] = Number(remaining & 0xffn);
        remaining = remaining >> 8n;
    }
    return bytes;
}
/**
 * Decode a little-endian U64 to bigint
 */
function decodeU64(bytes) {
    if (bytes.length !== 8) {
        throw new errors_1.ValidationError('bytes', 'U64 must be 8 bytes', bytes.length);
    }
    let value = 0n;
    for (let i = 7; i >= 0; i--) {
        value = (value << 8n) | BigInt(bytes[i]);
    }
    return value;
}
// =============================================================================
// Address Utilities
// =============================================================================
// =============================================================================
// Base58 Utilities (for Solana-style pubkeys, not SS58)
// =============================================================================
// Base58 alphabet (Bitcoin-style)
const BASE58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
/**
 * Decode Base58-encoded string to bytes (for Solana-style pubkeys)
 */
function base58DecodeSolana(input) {
    // Base58 decoding algorithm
    let result = [0];
    for (let i = 0; i < input.length; i++) {
        const charIndex = BASE58_ALPHABET.indexOf(input[i]);
        if (charIndex === -1) {
            throw new errors_1.ValidationError('base58', 'Invalid Base58 character', input[i]);
        }
        let carry = charIndex;
        for (let j = 0; j < result.length; j++) {
            carry += result[j] * 58;
            result[j] = carry % 256;
            carry = Math.floor(carry / 256);
        }
        while (carry > 0) {
            result.push(carry % 256);
            carry = Math.floor(carry / 256);
        }
    }
    // Add leading zeros
    for (let i = 0; i < input.length && input[i] === '1'; i++) {
        result.push(0);
    }
    return new Uint8Array(result.reverse());
}
/**
 * @deprecated Use base58DecodeSolana instead. This function is kept for backward compatibility.
 */
function base58Decode(input) {
    return base58DecodeSolana(input);
}
/**
 * Decode SS58-encoded account ID to bytes
 * Uses @polkadot/util-crypto's decodeAddress which handles all prefix widths
 * and validates the 2-byte checksum.
 */
function decodeAccountId(accountId) {
    // decodeAddress from @polkadot/util-crypto handles both hex and SS58 formats
    // and validates the checksum automatically
    try {
        const bytes = (0, util_crypto_1.decodeAddress)(accountId);
        if (bytes.length !== constants_1.ACCOUNT_ID_LENGTH) {
            throw new errors_1.ValidationError('accountId', `Account ID must be ${constants_1.ACCOUNT_ID_LENGTH} bytes`, bytes.length);
        }
        return bytes;
    }
    catch (error) {
        if (error instanceof errors_1.ValidationError)
            throw error;
        throw new errors_1.ValidationError('accountId', 'Failed to decode account ID', accountId);
    }
}
/**
 * Encode bytes to SS58 account ID
 * Uses @polkadot/util-crypto's encodeAddress for consistent round-trip encoding.
 */
function encodeAccountId(bytes) {
    if (bytes.length !== constants_1.ACCOUNT_ID_LENGTH) {
        throw new errors_1.ValidationError('bytes', `Account ID bytes must be ${constants_1.ACCOUNT_ID_LENGTH}`, bytes.length);
    }
    // encodeAddress returns the SS58 format by default
    return (0, util_crypto_1.encodeAddress)(bytes);
}
/**
 * Convert Substrate AccountId to EVM address (take first 20 bytes)
 */
function accountIdToEvmAddress(accountId) {
    const bytes = decodeAccountId(accountId);
    return (0, util_1.u8aToHex)(bytes.slice(0, constants_1.EVM_ADDRESS_LENGTH));
}
/**
 * Convert EVM address to Substrate AccountId (pad with zeros)
 */
function evmAddressToAccountId(evmAddress) {
    const addressBytes = (0, util_1.hexToU8a)(evmAddress);
    if (addressBytes.length !== constants_1.EVM_ADDRESS_LENGTH) {
        throw new errors_1.ValidationError('evmAddress', `EVM address must be ${constants_1.EVM_ADDRESS_LENGTH} bytes`, addressBytes.length);
    }
    const accountBytes = new Uint8Array(constants_1.ACCOUNT_ID_LENGTH);
    accountBytes.set(addressBytes);
    // Remaining bytes are zero-filled
    return encodeAccountId(accountBytes);
}
/**
 * Validate an EVM address format
 */
function isValidEvmAddress(address) {
    if (!(0, util_1.isHex)(address))
        return false;
    const bytes = (0, util_1.hexToU8a)(address);
    return bytes.length === constants_1.EVM_ADDRESS_LENGTH;
}
/**
 * Validate a Solana pubkey format (32 bytes)
 */
function isValidSolanaPubkey(pubkey) {
    // Check if it's valid hex of correct length first
    if ((0, util_1.isHex)(pubkey)) {
        const bytes = (0, util_1.hexToU8a)(pubkey);
        return bytes.length === 32;
    }
    // Base58 validation - Solana uses base58 encoding
    if (pubkey.length < 32 || pubkey.length > 44) {
        return false;
    }
    // Verify all characters are valid Base58
    const BASE58_CHARS = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
    for (const char of pubkey) {
        if (!BASE58_CHARS.includes(char)) {
            return false;
        }
    }
    return true;
}
// =============================================================================
// Validation Utilities
// =============================================================================
/**
 * Validate payload sizes for Comit submission
 */
function validatePayloadSizes(evmPayload, svmPayload) {
    if (evmPayload.length > constants_1.MAX_EVM_PAYLOAD_SIZE) {
        throw new errors_1.PayloadSizeError('evm', evmPayload.length, constants_1.MAX_EVM_PAYLOAD_SIZE);
    }
    if (svmPayload.length > constants_1.MAX_SVM_PAYLOAD_SIZE) {
        throw new errors_1.PayloadSizeError('svm', svmPayload.length, constants_1.MAX_SVM_PAYLOAD_SIZE);
    }
    const combinedSize = evmPayload.length + svmPayload.length;
    if (combinedSize > constants_1.MAX_COMBINED_PAYLOAD_SIZE) {
        throw new errors_1.PayloadSizeError('combined', combinedSize, constants_1.MAX_COMBINED_PAYLOAD_SIZE);
    }
}
/**
 * Validate a hash is proper H256 format
 */
function isValidH256(hash) {
    if (!(0, util_1.isHex)(hash))
        return false;
    const bytes = (0, util_1.hexToU8a)(hash);
    return bytes.length === constants_1.H256_LENGTH;
}
/**
 * Validate balance is non-negative
 */
function validateBalance(balance, field = 'balance') {
    if (balance < 0n) {
        throw new errors_1.ValidationError(field, 'Balance cannot be negative', balance);
    }
}
/**
 * Validate nonce is non-negative
 */
function validateNonce(nonce, field = 'nonce') {
    if (nonce < 0n) {
        throw new errors_1.ValidationError(field, 'Nonce cannot be negative', nonce);
    }
}
// =============================================================================
// Format Utilities
// =============================================================================
/**
 * Format balance with decimals for display
 */
function formatBalance(balance, decimals = 18) {
    const str = balance.toString().padStart(decimals + 1, '0');
    const intPart = str.slice(0, -decimals) || '0';
    const decPart = str.slice(-decimals).replace(/0+$/, '');
    return decPart ? `${intPart}.${decPart}` : intPart;
}
/**
 * Parse balance string to bigint
 */
function parseBalance(value, decimals = 18) {
    const [intPart, decPart = ''] = value.split('.');
    const paddedDec = decPart.padEnd(decimals, '0').slice(0, decimals);
    return BigInt(intPart + paddedDec);
}
/**
 * Truncate hash for display (e.g., "0x1234...5678")
 */
function truncateHash(hash, chars = 4) {
    if (hash.length <= chars * 2 + 4)
        return hash;
    return `${hash.slice(0, chars + 2)}...${hash.slice(-chars)}`;
}
// =============================================================================
// Async Utilities
// =============================================================================
/**
 * Sleep for specified milliseconds
 */
function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
/**
 * Retry an async operation with exponential backoff
 */
async function retry(operation, options = {}) {
    const { maxAttempts = 3, initialDelayMs = 1000, maxDelayMs = 30000, backoffFactor = 2, } = options;
    let lastError;
    let delay = initialDelayMs;
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        try {
            return await operation();
        }
        catch (error) {
            lastError = error instanceof Error ? error : new Error(String(error));
            if (attempt < maxAttempts) {
                await sleep(delay);
                delay = Math.min(delay * backoffFactor, maxDelayMs);
            }
        }
    }
    throw lastError;
}
//# sourceMappingURL=utils.js.map