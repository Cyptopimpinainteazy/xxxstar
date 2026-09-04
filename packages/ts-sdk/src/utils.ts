/**
 * Utility functions for X3 Chain SDK
 *
 * Provides encoding, hashing, and conversion utilities.
 */

import { blake2AsHex, blake2AsU8a, decodeAddress, encodeAddress } from '@polkadot/util-crypto';
import { hexToU8a, u8aToHex, stringToU8a, isHex } from '@polkadot/util';
import type { HexString } from '@polkadot/util/types';
import type { Hash, AccountId, Balance, Nonce } from './types';
import {
  MAX_EVM_PAYLOAD_SIZE,
  MAX_SVM_PAYLOAD_SIZE,
  MAX_COMBINED_PAYLOAD_SIZE,
  ACCOUNT_ID_LENGTH,
  EVM_ADDRESS_LENGTH,
  H256_LENGTH,
} from './constants';
import { ValidationError, PayloadSizeError } from './errors';

// =============================================================================
// Encoding Utilities
// =============================================================================

/**
 * Convert a hex string to Uint8Array
 */
export function hexToBytes(hex: HexString): Uint8Array {
  return hexToU8a(hex);
}

/**
 * Convert a Uint8Array to hex string
 */
export function bytesToHex(bytes: Uint8Array): HexString {
  return u8aToHex(bytes);
}

/**
 * Convert a string to Uint8Array (UTF-8)
 */
export function stringToBytes(str: string): Uint8Array {
  return stringToU8a(str);
}

/**
 * Ensure input is Uint8Array (convert from hex if needed)
 */
export function toBytes(input: HexString | Uint8Array): Uint8Array {
  if (input instanceof Uint8Array) {
    return input;
  }
  return hexToBytes(input);
}

/**
 * Ensure input is hex string (convert from bytes if needed)
 */
export function toHex(input: HexString | Uint8Array): HexString {
  if (typeof input === 'string') {
    return input.startsWith('0x') ? (input as HexString) : (`0x${input}` as HexString);
  }
  return bytesToHex(input);
}

// =============================================================================
// Hashing Utilities
// =============================================================================

/**
 * Compute BLAKE2-256 hash of input
 */
export function blake2_256(data: Uint8Array | string): Hash {
  const input = typeof data === 'string' ? stringToU8a(data) : data;
  return blake2AsHex(input, 256) as Hash;
}

/**
 * Compute BLAKE2-256 hash as bytes
 */
export function blake2_256_bytes(data: Uint8Array | string): Uint8Array {
  const input = typeof data === 'string' ? stringToU8a(data) : data;
  return blake2AsU8a(input, 256);
}

/**
 * Compute prepare_root for a Comit from its inputs
 * This matches the runtime's prepare_root computation
 */
export function computePrepareRoot(
  origin: AccountId,
  evmPayload: Uint8Array,
  svmPayload: Uint8Array,
  nonce: Nonce,
  fee: Balance
): Hash {
  // Concatenate all inputs in order
  const originBytes = decodeAccountId(origin);
  const nonceBytes = encodeU128(nonce);
  const feeBytes = encodeU128(fee);

  const combined = new Uint8Array(
    originBytes.length + evmPayload.length + svmPayload.length + nonceBytes.length + feeBytes.length
  );

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
export function computeComitId(prepareRoot: Hash): Hash {
  return blake2_256(hexToBytes(prepareRoot));
}

// =============================================================================
// Number Encoding
// =============================================================================

/**
 * Encode a bigint as little-endian U128 (16 bytes)
 */
export function encodeU128(value: bigint): Uint8Array {
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
export function decodeU128(bytes: Uint8Array): bigint {
  if (bytes.length !== 16) {
    throw new ValidationError('bytes', 'U128 must be 16 bytes', bytes.length);
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
export function encodeU64(value: bigint): Uint8Array {
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
export function decodeU64(bytes: Uint8Array): bigint {
  if (bytes.length !== 8) {
    throw new ValidationError('bytes', 'U64 must be 8 bytes', bytes.length);
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
export function base58DecodeSolana(input: string): Uint8Array {
  // Base58 decoding algorithm
  let result: number[] = [0];
  
  for (let i = 0; i < input.length; i++) {
    const charIndex = BASE58_ALPHABET.indexOf(input[i]);
    if (charIndex === -1) {
      throw new ValidationError('base58', 'Invalid Base58 character', input[i]);
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
export function base58Decode(input: string): Uint8Array {
  return base58DecodeSolana(input);
}

/**
 * Decode SS58-encoded account ID to bytes
 * Uses @polkadot/util-crypto's decodeAddress which handles all prefix widths
 * and validates the 2-byte checksum.
 */
export function decodeAccountId(accountId: AccountId): Uint8Array {
  // decodeAddress from @polkadot/util-crypto handles both hex and SS58 formats
  // and validates the checksum automatically
  try {
    const bytes = decodeAddress(accountId);
    if (bytes.length !== ACCOUNT_ID_LENGTH) {
      throw new ValidationError(
        'accountId',
        `Account ID must be ${ACCOUNT_ID_LENGTH} bytes`,
        bytes.length
      );
    }
    return bytes;
  } catch (error) {
    if (error instanceof ValidationError) throw error;
    throw new ValidationError('accountId', 'Failed to decode account ID', accountId);
  }
}

/**
 * Encode bytes to SS58 account ID
 * Uses @polkadot/util-crypto's encodeAddress for consistent round-trip encoding.
 */
export function encodeAccountId(bytes: Uint8Array): AccountId {
  if (bytes.length !== ACCOUNT_ID_LENGTH) {
    throw new ValidationError(
      'bytes',
      `Account ID bytes must be ${ACCOUNT_ID_LENGTH}`,
      bytes.length
    );
  }
  // encodeAddress returns the SS58 format by default
  return encodeAddress(bytes);
}

/**
 * Convert Substrate AccountId to EVM address (take first 20 bytes)
 */
export function accountIdToEvmAddress(accountId: AccountId): HexString {
  const bytes = decodeAccountId(accountId);
  return u8aToHex(bytes.slice(0, EVM_ADDRESS_LENGTH));
}

/**
 * Convert EVM address to Substrate AccountId (pad with zeros)
 */
export function evmAddressToAccountId(evmAddress: HexString): AccountId {
  const addressBytes = hexToU8a(evmAddress);
  if (addressBytes.length !== EVM_ADDRESS_LENGTH) {
    throw new ValidationError(
      'evmAddress',
      `EVM address must be ${EVM_ADDRESS_LENGTH} bytes`,
      addressBytes.length
    );
  }

  const accountBytes = new Uint8Array(ACCOUNT_ID_LENGTH);
  accountBytes.set(addressBytes);
  // Remaining bytes are zero-filled
  return encodeAccountId(accountBytes);
}

/**
 * Validate an EVM address format
 */
export function isValidEvmAddress(address: string): boolean {
  if (!isHex(address)) return false;
  const bytes = hexToU8a(address);
  return bytes.length === EVM_ADDRESS_LENGTH;
}

/**
 * Validate a Solana pubkey format (32 bytes)
 */
export function isValidSolanaPubkey(pubkey: string): boolean {
  // Check if it's valid hex of correct length first
  if (isHex(pubkey)) {
    const bytes = hexToU8a(pubkey);
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
export function validatePayloadSizes(
  evmPayload: Uint8Array,
  svmPayload: Uint8Array
): void {
  if (evmPayload.length > MAX_EVM_PAYLOAD_SIZE) {
    throw new PayloadSizeError('evm', evmPayload.length, MAX_EVM_PAYLOAD_SIZE);
  }

  if (svmPayload.length > MAX_SVM_PAYLOAD_SIZE) {
    throw new PayloadSizeError('svm', svmPayload.length, MAX_SVM_PAYLOAD_SIZE);
  }

  const combinedSize = evmPayload.length + svmPayload.length;
  if (combinedSize > MAX_COMBINED_PAYLOAD_SIZE) {
    throw new PayloadSizeError('combined', combinedSize, MAX_COMBINED_PAYLOAD_SIZE);
  }
}

/**
 * Validate a hash is proper H256 format
 */
export function isValidH256(hash: string): boolean {
  if (!isHex(hash)) return false;
  const bytes = hexToU8a(hash);
  return bytes.length === H256_LENGTH;
}

/**
 * Validate balance is non-negative
 */
export function validateBalance(balance: bigint, field: string = 'balance'): void {
  if (balance < 0n) {
    throw new ValidationError(field, 'Balance cannot be negative', balance);
  }
}

/**
 * Validate nonce is non-negative
 */
export function validateNonce(nonce: bigint, field: string = 'nonce'): void {
  if (nonce < 0n) {
    throw new ValidationError(field, 'Nonce cannot be negative', nonce);
  }
}

// =============================================================================
// Format Utilities
// =============================================================================

/**
 * Format balance with decimals for display
 */
export function formatBalance(balance: Balance, decimals: number = 18): string {
  const str = balance.toString().padStart(decimals + 1, '0');
  const intPart = str.slice(0, -decimals) || '0';
  const decPart = str.slice(-decimals).replace(/0+$/, '');
  return decPart ? `${intPart}.${decPart}` : intPart;
}

/**
 * Parse balance string to bigint
 */
export function parseBalance(value: string, decimals: number = 18): Balance {
  const [intPart, decPart = ''] = value.split('.');
  const paddedDec = decPart.padEnd(decimals, '0').slice(0, decimals);
  return BigInt(intPart + paddedDec);
}

/**
 * Truncate hash for display (e.g., "0x1234...5678")
 */
export function truncateHash(hash: Hash, chars: number = 4): string {
  if (hash.length <= chars * 2 + 4) return hash;
  return `${hash.slice(0, chars + 2)}...${hash.slice(-chars)}`;
}

// =============================================================================
// Async Utilities
// =============================================================================

/**
 * Sleep for specified milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Retry an async operation with exponential backoff
 */
export async function retry<T>(
  operation: () => Promise<T>,
  options: {
    maxAttempts?: number;
    initialDelayMs?: number;
    maxDelayMs?: number;
    backoffFactor?: number;
  } = {}
): Promise<T> {
  const {
    maxAttempts = 3,
    initialDelayMs = 1000,
    maxDelayMs = 30000,
    backoffFactor = 2,
  } = options;

  let lastError: Error | undefined;
  let delay = initialDelayMs;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      if (attempt < maxAttempts) {
        await sleep(delay);
        delay = Math.min(delay * backoffFactor, maxDelayMs);
      }
    }
  }

  throw lastError;
}
