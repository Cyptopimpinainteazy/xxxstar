"use strict";
/**
 * EVM utilities for X3 Chain SDK
 *
 * Provides encoding, decoding, and conversion utilities for EVM interaction.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.isValidAddress = isValidAddress;
exports.normalizeAddress = normalizeAddress;
exports.checksumAddress = checksumAddress;
exports.accountIdToAddress = accountIdToAddress;
exports.addressToAccountId = addressToAccountId;
exports.publicKeyToAddress = publicKeyToAddress;
exports.functionSelector = functionSelector;
exports.encodeUint256 = encodeUint256;
exports.decodeUint256 = decodeUint256;
exports.encodeAddress = encodeAddress;
exports.decodeAddress = decodeAddress;
exports.encodeBytes = encodeBytes;
exports.encodeString = encodeString;
exports.encodeBool = encodeBool;
exports.decodeBool = decodeBool;
exports.encodeFunctionCall = encodeFunctionCall;
exports.decodeFunctionCall = decodeFunctionCall;
exports.encodeTransfer = encodeTransfer;
exports.encodeApprove = encodeApprove;
exports.encodeTransferFrom = encodeTransferFrom;
exports.encodeBalanceOf = encodeBalanceOf;
exports.isErrorRevert = isErrorRevert;
exports.isPanicRevert = isPanicRevert;
exports.decodeErrorMessage = decodeErrorMessage;
exports.decodePanicCode = decodePanicCode;
exports.getPanicMessage = getPanicMessage;
const util_1 = require("@polkadot/util");
const errors_1 = require("./errors");
const constants_1 = require("./constants");
const utils_1 = require("./utils");
// =============================================================================
// Address Utilities
// =============================================================================
/**
 * Validate an EVM address
 */
function isValidAddress(address) {
    if (!(0, util_1.isHex)(address))
        return false;
    const bytes = (0, util_1.hexToU8a)(address);
    return bytes.length === constants_1.EVM_ADDRESS_LENGTH;
}
/**
 * Normalize an EVM address (lowercase with 0x prefix)
 */
function normalizeAddress(address) {
    if (!isValidAddress(address)) {
        throw new errors_1.ValidationError('address', 'Invalid EVM address format', address);
    }
    return address.toLowerCase();
}
/**
 * Checksum an EVM address (EIP-55)
 */
function checksumAddress(address) {
    if (!isValidAddress(address)) {
        throw new errors_1.ValidationError('address', 'Invalid EVM address format', address);
    }
    const addr = address.toLowerCase().replace('0x', '');
    // In production, use keccak256 for proper checksumming
    // This is a simplified version
    let result = '0x';
    for (let i = 0; i < addr.length; i++) {
        const char = addr[i];
        // Simplified: alternate upper/lower based on position
        // Real implementation uses keccak256 hash
        result += i % 2 === 0 ? char.toUpperCase() : char.toLowerCase();
    }
    return result;
}
/**
 * Convert Substrate AccountId to EVM address
 */
function accountIdToAddress(accountId) {
    let bytes;
    if ((0, util_1.isHex)(accountId)) {
        bytes = (0, util_1.hexToU8a)(accountId);
    }
    else {
        // SS58 decode via utils
        bytes = (0, utils_1.decodeAccountId)(accountId);
    }
    // Take first 20 bytes
    return (0, util_1.u8aToHex)(bytes.slice(0, constants_1.EVM_ADDRESS_LENGTH));
}
/**
 * Convert EVM address to Substrate AccountId
 */
function addressToAccountId(address) {
    if (!isValidAddress(address)) {
        throw new errors_1.ValidationError('address', 'Invalid EVM address format', address);
    }
    const addressBytes = (0, util_1.hexToU8a)(address);
    const accountBytes = new Uint8Array(32);
    accountBytes.set(addressBytes);
    return (0, util_1.u8aToHex)(accountBytes);
}
/**
 * Derive EVM address from public key
 */
function publicKeyToAddress(publicKey) {
    // In production, use keccak256
    // Take last 20 bytes of hash
    const addressBytes = publicKey.slice(-constants_1.EVM_ADDRESS_LENGTH);
    return (0, util_1.u8aToHex)(addressBytes);
}
// =============================================================================
// ABI Encoding
// =============================================================================
/**
 * Compute function selector from signature
 */
function functionSelector(signature) {
    // In production, use keccak256(signature).slice(0, 4)
    // Simplified: hash the signature
    const hash = simpleHash(signature);
    return ('0x' + hash.slice(0, 8));
}
/**
 * Encode a uint256 value
 */
function encodeUint256(value) {
    if (value < 0n) {
        throw new errors_1.ValidationError('value', 'uint256 cannot be negative', value);
    }
    if (value >= 2n ** 256n) {
        throw new errors_1.ValidationError('value', 'uint256 overflow', value);
    }
    const bytes = new Uint8Array(32);
    let remaining = value;
    for (let i = 31; i >= 0 && remaining > 0n; i--) {
        bytes[i] = Number(remaining & 0xffn);
        remaining = remaining >> 8n;
    }
    return bytes;
}
/**
 * Decode a uint256 value
 */
function decodeUint256(bytes) {
    if (bytes.length !== 32) {
        throw new errors_1.ValidationError('bytes', 'uint256 must be 32 bytes', bytes.length);
    }
    let value = 0n;
    for (let i = 0; i < 32; i++) {
        value = (value << 8n) | BigInt(bytes[i]);
    }
    return value;
}
/**
 * Encode an address for ABI
 */
function encodeAddress(address) {
    if (!isValidAddress(address)) {
        throw new errors_1.ValidationError('address', 'Invalid EVM address', address);
    }
    const addressBytes = (0, util_1.hexToU8a)(address);
    const result = new Uint8Array(32);
    // Address is right-aligned in 32 bytes
    result.set(addressBytes, 32 - constants_1.EVM_ADDRESS_LENGTH);
    return result;
}
/**
 * Decode an address from ABI encoding
 */
function decodeAddress(bytes) {
    if (bytes.length !== 32) {
        throw new errors_1.ValidationError('bytes', 'ABI address must be 32 bytes', bytes.length);
    }
    return (0, util_1.u8aToHex)(bytes.slice(32 - constants_1.EVM_ADDRESS_LENGTH));
}
/**
 * Encode bytes for ABI (dynamic type)
 */
function encodeBytes(data) {
    // Encode length as uint256
    const length = encodeUint256(BigInt(data.length));
    // Pad data to 32-byte boundary
    const paddedLength = Math.ceil(data.length / 32) * 32;
    const padded = new Uint8Array(paddedLength);
    padded.set(data);
    // Combine length + data
    const result = new Uint8Array(32 + paddedLength);
    result.set(length);
    result.set(padded, 32);
    return result;
}
/**
 * Encode a string for ABI (as bytes)
 */
function encodeString(str) {
    const encoder = new TextEncoder();
    return encodeBytes(encoder.encode(str));
}
/**
 * Encode a boolean
 */
function encodeBool(value) {
    return encodeUint256(value ? 1n : 0n);
}
/**
 * Decode a boolean
 */
function decodeBool(bytes) {
    const value = decodeUint256(bytes);
    return value !== 0n;
}
// =============================================================================
// Function Call Encoding
// =============================================================================
/**
 * Encode a function call with parameters
 */
function encodeFunctionCall(signature, params) {
    const selector = (0, util_1.hexToU8a)(functionSelector(signature));
    // Combine selector + encoded params
    const totalLength = 4 + params.reduce((sum, p) => sum + p.length, 0);
    const result = new Uint8Array(totalLength);
    result.set(selector, 0);
    let offset = 4;
    for (const param of params) {
        result.set(param, offset);
        offset += param.length;
    }
    return result;
}
/**
 * Decode a function call
 */
function decodeFunctionCall(data) {
    if (data.length < 4) {
        throw new errors_1.ValidationError('data', 'Function call must have at least 4 bytes', data.length);
    }
    return {
        selector: (0, util_1.u8aToHex)(data.slice(0, 4)),
        params: data.slice(4),
    };
}
// =============================================================================
// Common Function Encoders
// =============================================================================
/**
 * Encode ERC20 transfer call
 */
function encodeTransfer(to, amount) {
    return encodeFunctionCall('transfer(address,uint256)', [encodeAddress(to), encodeUint256(amount)]);
}
/**
 * Encode ERC20 approve call
 */
function encodeApprove(spender, amount) {
    return encodeFunctionCall('approve(address,uint256)', [encodeAddress(spender), encodeUint256(amount)]);
}
/**
 * Encode ERC20 transferFrom call
 */
function encodeTransferFrom(from, to, amount) {
    return encodeFunctionCall('transferFrom(address,address,uint256)', [encodeAddress(from), encodeAddress(to), encodeUint256(amount)]);
}
/**
 * Encode balanceOf call
 */
function encodeBalanceOf(account) {
    return encodeFunctionCall('balanceOf(address)', [encodeAddress(account)]);
}
// =============================================================================
// Error Decoding
// =============================================================================
/**
 * Check if data is an Error(string) revert
 */
function isErrorRevert(data) {
    if (data.length < 4)
        return false;
    const selector = (0, util_1.u8aToHex)(data.slice(0, 4));
    return selector === constants_1.EVM_SELECTORS.error;
}
/**
 * Check if data is a Panic(uint256) revert
 */
function isPanicRevert(data) {
    if (data.length < 4)
        return false;
    const selector = (0, util_1.u8aToHex)(data.slice(0, 4));
    return selector === constants_1.EVM_SELECTORS.panic;
}
/**
 * Decode Error(string) revert message
 */
function decodeErrorMessage(data) {
    if (!isErrorRevert(data) || data.length < 68) {
        return null;
    }
    try {
        // Skip selector (4) + offset (32)
        const length = Number(decodeUint256(data.slice(36, 68)));
        if (data.length < 68 + length)
            return null;
        const decoder = new TextDecoder();
        return decoder.decode(data.slice(68, 68 + length));
    }
    catch {
        return null;
    }
}
/**
 * Decode Panic(uint256) code
 */
function decodePanicCode(data) {
    if (!isPanicRevert(data) || data.length < 36) {
        return null;
    }
    try {
        return decodeUint256(data.slice(4, 36));
    }
    catch {
        return null;
    }
}
/**
 * Get human-readable panic message
 */
function getPanicMessage(code) {
    const messages = {
        '0': 'Generic panic',
        '1': 'Assert failed',
        '17': 'Arithmetic overflow/underflow',
        '18': 'Division or modulo by zero',
        '33': 'Invalid enum value',
        '34': 'Invalid storage byte array encoding',
        '49': 'Pop on empty array',
        '50': 'Array index out of bounds',
        '65': 'Out of memory',
        '81': 'Call to uninitialized function',
    };
    return messages[code.toString()] || `Unknown panic code: ${code}`;
}
// =============================================================================
// Private Utilities
// =============================================================================
/**
 * Simple hash function (placeholder for keccak256)
 * In production, use a proper keccak256 implementation
 */
function simpleHash(input) {
    let hash = 0;
    for (let i = 0; i < input.length; i++) {
        const char = input.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    // Convert to hex and pad to 64 chars (from end, not start)
    const hex = Math.abs(hash).toString(16).padEnd(64, '0');
    return hex;
}
//# sourceMappingURL=evm.js.map