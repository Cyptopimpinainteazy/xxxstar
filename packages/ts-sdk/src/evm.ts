/**
 * EVM utilities for X3 Chain SDK
 *
 * Provides encoding, decoding, and conversion utilities for EVM interaction.
 */

import type { HexString } from '@polkadot/util/types';
import { hexToU8a, u8aToHex, isHex } from '@polkadot/util';

import type { AccountId } from './types';
import { ValidationError } from './errors';
import { EVM_ADDRESS_LENGTH, EVM_SELECTORS } from './constants';
import { decodeAccountId } from './utils';

// =============================================================================
// Types
// =============================================================================

/**
 * EVM transaction parameters
 */
export interface EvmTxParams {
  /** Target address (null for deployment) */
  to: HexString | null;
  /** Value to send in wei */
  value: bigint;
  /** Gas limit */
  gasLimit: bigint;
  /** Gas price */
  gasPrice: bigint;
  /** Calldata */
  data: HexString;
  /** Nonce */
  nonce: bigint;
}

/**
 * Function signature with selector
 */
export interface FunctionSignature {
  /** Function name */
  name: string;
  /** Full signature (e.g., "transfer(address,uint256)") */
  signature: string;
  /** 4-byte selector */
  selector: HexString;
}

/**
 * Decoded function call
 */
export interface DecodedCall {
  /** Function selector */
  selector: HexString;
  /** Encoded parameters */
  params: Uint8Array;
}

// =============================================================================
// Address Utilities
// =============================================================================

/**
 * Validate an EVM address
 */
export function isValidAddress(address: string): boolean {
  if (!isHex(address)) return false;
  const bytes = hexToU8a(address);
  return bytes.length === EVM_ADDRESS_LENGTH;
}

/**
 * Normalize an EVM address (lowercase with 0x prefix)
 */
export function normalizeAddress(address: string): HexString {
  if (!isValidAddress(address)) {
    throw new ValidationError('address', 'Invalid EVM address format', address);
  }
  return address.toLowerCase() as HexString;
}

/**
 * Checksum an EVM address (EIP-55)
 */
export function checksumAddress(address: string): HexString {
  if (!isValidAddress(address)) {
    throw new ValidationError('address', 'Invalid EVM address format', address);
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

  return result as HexString;
}

/**
 * Convert Substrate AccountId to EVM address
 */
export function accountIdToAddress(accountId: AccountId): HexString {
  let bytes: Uint8Array;

  if (isHex(accountId)) {
    bytes = hexToU8a(accountId);
  } else {
    // SS58 decode via utils
    bytes = decodeAccountId(accountId);
  }

  // Take first 20 bytes
  return u8aToHex(bytes.slice(0, EVM_ADDRESS_LENGTH));
}

/**
 * Convert EVM address to Substrate AccountId
 */
export function addressToAccountId(address: HexString): AccountId {
  if (!isValidAddress(address)) {
    throw new ValidationError('address', 'Invalid EVM address format', address);
  }

  const addressBytes = hexToU8a(address);
  const accountBytes = new Uint8Array(32);
  accountBytes.set(addressBytes);

  return u8aToHex(accountBytes);
}

/**
 * Derive EVM address from public key
 */
export function publicKeyToAddress(publicKey: Uint8Array): HexString {
  // In production, use keccak256
  // Take last 20 bytes of hash
  const addressBytes = publicKey.slice(-EVM_ADDRESS_LENGTH);
  return u8aToHex(addressBytes);
}

// =============================================================================
// ABI Encoding
// =============================================================================

/**
 * Compute function selector from signature
 */
export function functionSelector(signature: string): HexString {
  // In production, use keccak256(signature).slice(0, 4)
  // Simplified: hash the signature
  const hash = simpleHash(signature);
  return ('0x' + hash.slice(0, 8)) as HexString;
}

/**
 * Encode a uint256 value
 */
export function encodeUint256(value: bigint): Uint8Array {
  if (value < 0n) {
    throw new ValidationError('value', 'uint256 cannot be negative', value);
  }
  if (value >= 2n ** 256n) {
    throw new ValidationError('value', 'uint256 overflow', value);
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
export function decodeUint256(bytes: Uint8Array): bigint {
  if (bytes.length !== 32) {
    throw new ValidationError('bytes', 'uint256 must be 32 bytes', bytes.length);
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
export function encodeAddress(address: HexString): Uint8Array {
  if (!isValidAddress(address)) {
    throw new ValidationError('address', 'Invalid EVM address', address);
  }

  const addressBytes = hexToU8a(address);
  const result = new Uint8Array(32);
  // Address is right-aligned in 32 bytes
  result.set(addressBytes, 32 - EVM_ADDRESS_LENGTH);
  return result;
}

/**
 * Decode an address from ABI encoding
 */
export function decodeAddress(bytes: Uint8Array): HexString {
  if (bytes.length !== 32) {
    throw new ValidationError('bytes', 'ABI address must be 32 bytes', bytes.length);
  }

  return u8aToHex(bytes.slice(32 - EVM_ADDRESS_LENGTH));
}

/**
 * Encode bytes for ABI (dynamic type)
 */
export function encodeBytes(data: Uint8Array): Uint8Array {
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
export function encodeString(str: string): Uint8Array {
  const encoder = new TextEncoder();
  return encodeBytes(encoder.encode(str));
}

/**
 * Encode a boolean
 */
export function encodeBool(value: boolean): Uint8Array {
  return encodeUint256(value ? 1n : 0n);
}

/**
 * Decode a boolean
 */
export function decodeBool(bytes: Uint8Array): boolean {
  const value = decodeUint256(bytes);
  return value !== 0n;
}

// =============================================================================
// Function Call Encoding
// =============================================================================

/**
 * Encode a function call with parameters
 */
export function encodeFunctionCall(
  signature: string,
  params: Uint8Array[]
): Uint8Array {
  const selector = hexToU8a(functionSelector(signature));

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
export function decodeFunctionCall(data: Uint8Array): DecodedCall {
  if (data.length < 4) {
    throw new ValidationError('data', 'Function call must have at least 4 bytes', data.length);
  }

  return {
    selector: u8aToHex(data.slice(0, 4)),
    params: data.slice(4),
  };
}

// =============================================================================
// Common Function Encoders
// =============================================================================

/**
 * Encode ERC20 transfer call
 */
export function encodeTransfer(to: HexString, amount: bigint): Uint8Array {
  return encodeFunctionCall(
    'transfer(address,uint256)',
    [encodeAddress(to), encodeUint256(amount)]
  );
}

/**
 * Encode ERC20 approve call
 */
export function encodeApprove(spender: HexString, amount: bigint): Uint8Array {
  return encodeFunctionCall(
    'approve(address,uint256)',
    [encodeAddress(spender), encodeUint256(amount)]
  );
}

/**
 * Encode ERC20 transferFrom call
 */
export function encodeTransferFrom(
  from: HexString,
  to: HexString,
  amount: bigint
): Uint8Array {
  return encodeFunctionCall(
    'transferFrom(address,address,uint256)',
    [encodeAddress(from), encodeAddress(to), encodeUint256(amount)]
  );
}

/**
 * Encode balanceOf call
 */
export function encodeBalanceOf(account: HexString): Uint8Array {
  return encodeFunctionCall(
    'balanceOf(address)',
    [encodeAddress(account)]
  );
}

// =============================================================================
// Error Decoding
// =============================================================================

/**
 * Check if data is an Error(string) revert
 */
export function isErrorRevert(data: Uint8Array): boolean {
  if (data.length < 4) return false;
  const selector = u8aToHex(data.slice(0, 4));
  return selector === EVM_SELECTORS.error;
}

/**
 * Check if data is a Panic(uint256) revert
 */
export function isPanicRevert(data: Uint8Array): boolean {
  if (data.length < 4) return false;
  const selector = u8aToHex(data.slice(0, 4));
  return selector === EVM_SELECTORS.panic;
}

/**
 * Decode Error(string) revert message
 */
export function decodeErrorMessage(data: Uint8Array): string | null {
  if (!isErrorRevert(data) || data.length < 68) {
    return null;
  }

  try {
    // Skip selector (4) + offset (32)
    const length = Number(decodeUint256(data.slice(36, 68)));
    if (data.length < 68 + length) return null;

    const decoder = new TextDecoder();
    return decoder.decode(data.slice(68, 68 + length));
  } catch {
    return null;
  }
}

/**
 * Decode Panic(uint256) code
 */
export function decodePanicCode(data: Uint8Array): bigint | null {
  if (!isPanicRevert(data) || data.length < 36) {
    return null;
  }

  try {
    return decodeUint256(data.slice(4, 36));
  } catch {
    return null;
  }
}

/**
 * Get human-readable panic message
 */
export function getPanicMessage(code: bigint): string {
  const messages: Record<string, string> = {
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
function simpleHash(input: string): string {
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
