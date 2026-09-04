/**
 * SVM (Solana VM) utilities for X3 Chain SDK
 *
 * Provides encoding, decoding, and conversion utilities for SVM interaction.
 */

import type { HexString } from '@polkadot/util/types';
import { hexToU8a, u8aToHex, isHex } from '@polkadot/util';
import { sha256AsU8a } from '@polkadot/util-crypto';

import type { AccountId } from './types';
import { ValidationError } from './errors';
import { SOLANA_PUBKEY_LENGTH, ACCOUNT_ID_LENGTH } from './constants';
import { base58DecodeSolana } from './utils';

// =============================================================================
// Types
// =============================================================================

/**
 * Solana public key (32 bytes)
 */
export type Pubkey = HexString;

/**
 * Account metadata for instruction
 */
export interface AccountMeta {
  /** Public key of the account */
  pubkey: Pubkey;
  /** Whether the account is a signer */
  isSigner: boolean;
  /** Whether the account is writable */
  isWritable: boolean;
}

/**
 * SVM instruction
 */
export interface Instruction {
  /** Program ID to invoke */
  programId: Pubkey;
  /** Accounts involved in the instruction */
  accounts: AccountMeta[];
  /** Instruction data */
  data: Uint8Array;
}

/**
 * Compact-u16 encoded length
 */
export interface CompactU16 {
  value: number;
  bytes: Uint8Array;
}

// =============================================================================
// Pubkey Utilities
// =============================================================================

/**
 * Validate a Solana pubkey
 */
export function isValidPubkey(pubkey: string): boolean {
  if (isHex(pubkey)) {
    const bytes = hexToU8a(pubkey);
    return bytes.length === SOLANA_PUBKEY_LENGTH;
  }

  // Check if base58 encoded (32-44 chars typically)
  if (pubkey.length >= 32 && pubkey.length <= 44) {
    // Base58 character set validation
    const base58Regex = /^[1-9A-HJ-NP-Za-km-z]+$/;
    return base58Regex.test(pubkey);
  }

  return false;
}

/**
 * Convert pubkey to bytes
 */
export function pubkeyToBytes(pubkey: Pubkey): Uint8Array {
  if (isHex(pubkey)) {
    const bytes = hexToU8a(pubkey);
    if (bytes.length !== SOLANA_PUBKEY_LENGTH) {
      throw new ValidationError('pubkey', `Pubkey must be ${SOLANA_PUBKEY_LENGTH} bytes`, bytes.length);
    }
    return bytes;
  }

  // Base58 decode for Solana-style pubkeys
  const decoded = base58DecodeSolana(pubkey);
  if (decoded.length !== SOLANA_PUBKEY_LENGTH) {
    throw new ValidationError('pubkey', `Pubkey must be ${SOLANA_PUBKEY_LENGTH} bytes after decode`, decoded.length);
  }
  return decoded;
}

/**
 * Convert bytes to pubkey
 */
export function bytesToPubkey(bytes: Uint8Array): Pubkey {
  if (bytes.length !== SOLANA_PUBKEY_LENGTH) {
    throw new ValidationError('bytes', `Pubkey bytes must be ${SOLANA_PUBKEY_LENGTH}`, bytes.length);
  }
  return u8aToHex(bytes);
}

/**
 * Create a zero pubkey (system program)
 */
export function zeroPubkey(): Pubkey {
  return u8aToHex(new Uint8Array(SOLANA_PUBKEY_LENGTH));
}

/**
 * Convert Substrate AccountId to Solana pubkey
 */
export function accountIdToPubkey(accountId: AccountId): Pubkey {
  if (isHex(accountId)) {
    const bytes = hexToU8a(accountId);
    if (bytes.length === ACCOUNT_ID_LENGTH) {
      return u8aToHex(bytes);
    }
  }
  throw new ValidationError('accountId', 'Invalid AccountId format');
}

/**
 * Convert Solana pubkey to Substrate AccountId
 */
export function pubkeyToAccountId(pubkey: Pubkey): AccountId {
  const bytes = pubkeyToBytes(pubkey);
  return u8aToHex(bytes);
}

/**
 * Derive program address (PDA)
 * Uses SHA256 seed hashing; curve validation is intentionally simplified.
 */
export function findProgramAddress(
  seeds: Uint8Array[],
  programId: Pubkey
): { address: Pubkey; bump: number } {
  const programBytes = pubkeyToBytes(programId);

  for (let bump = 255; bump >= 0; bump--) {
    const allSeeds = [...seeds, new Uint8Array([bump])];
    const hash = simpleHash(programBytes, allSeeds);

    // Check if on curve (ed25519)
    // A point is on the ed25519 curve if its high bit is 0 (little-endian)
    // For PDA derivation, we need to check if the hash represents a valid ed25519 point
    const highBitSet = (hash[31] & 0x80) !== 0;
    if (highBitSet) {
      // Point is not on curve, continue to next bump
      continue;
    }
    
    return {
      address: u8aToHex(hash),
      bump,
    };
  }

  throw new Error('Unable to find valid PDA');
}

// =============================================================================
// Instruction Encoding
// =============================================================================

/**
 * Encode a compact u16 length
 */
export function encodeCompactU16(value: number): Uint8Array {
  if (value < 0 || value > 65535) {
    throw new ValidationError('value', 'Compact u16 out of range', value);
  }

  if (value < 128) {
    return new Uint8Array([value]);
  } else if (value < 16384) {
    return new Uint8Array([
      (value & 0x7f) | 0x80,
      value >> 7,
    ]);
  } else {
    return new Uint8Array([
      (value & 0x7f) | 0x80,
      ((value >> 7) & 0x7f) | 0x80,
      value >> 14,
    ]);
  }
}

/**
 * Decode a compact u16 length
 */
export function decodeCompactU16(bytes: Uint8Array, offset: number = 0): CompactU16 {
  let value = 0;
  let shift = 0;
  let len = 0;

  while (offset + len < bytes.length) {
    const byte = bytes[offset + len];
    value |= (byte & 0x7f) << shift;
    len++;

    if ((byte & 0x80) === 0) {
      break;
    }
    shift += 7;

    if (len > 3) {
      throw new ValidationError('bytes', 'Invalid compact u16 encoding');
    }
  }

  return {
    value,
    bytes: bytes.slice(offset, offset + len),
  };
}

/**
 * Encode an instruction
 */
export function encodeInstruction(instruction: Instruction): Uint8Array {
  const parts: Uint8Array[] = [];

  // Program ID index (we'll use 0 as placeholder - real encoding needs account list)
  parts.push(new Uint8Array([0]));

  // Account indices (compact array)
  parts.push(encodeCompactU16(instruction.accounts.length));
  for (const _account of instruction.accounts) {
    // Account index placeholder
    parts.push(new Uint8Array([0]));
  }

  // Data (compact array)
  parts.push(encodeCompactU16(instruction.data.length));
  parts.push(instruction.data);

  // Combine all parts
  const totalLength = parts.reduce((sum, p) => sum + p.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }

  return result;
}

/**
 * Encode instruction data with discriminator
 */
export function encodeInstructionData(
  discriminator: Uint8Array | number[],
  params: Uint8Array[]
): Uint8Array {
  const disc = discriminator instanceof Uint8Array
    ? discriminator
    : new Uint8Array(discriminator);

  const totalLength = disc.length + params.reduce((sum, p) => sum + p.length, 0);
  const result = new Uint8Array(totalLength);

  result.set(disc, 0);
  let offset = disc.length;
  for (const param of params) {
    result.set(param, offset);
    offset += param.length;
  }

  return result;
}

// =============================================================================
// Data Type Encoding
// =============================================================================

/**
 * Encode a u8
 */
export function encodeU8(value: number): Uint8Array {
  if (value < 0 || value > 255) {
    throw new ValidationError('value', 'u8 out of range', value);
  }
  return new Uint8Array([value]);
}

/**
 * Encode a u16 (little-endian)
 */
export function encodeU16(value: number): Uint8Array {
  if (value < 0 || value > 65535) {
    throw new ValidationError('value', 'u16 out of range', value);
  }
  return new Uint8Array([value & 0xff, value >> 8]);
}

/**
 * Encode a u32 (little-endian)
 */
export function encodeU32(value: number): Uint8Array {
  if (value < 0 || value > 0xffffffff) {
    throw new ValidationError('value', 'u32 out of range', value);
  }
  return new Uint8Array([
    value & 0xff,
    (value >> 8) & 0xff,
    (value >> 16) & 0xff,
    (value >> 24) & 0xff,
  ]);
}

/**
 * Encode a u64 (little-endian)
 */
export function encodeU64(value: bigint): Uint8Array {
  if (value < 0n || value >= 2n ** 64n) {
    throw new ValidationError('value', 'u64 out of range', value);
  }

  const bytes = new Uint8Array(8);
  let remaining = value;
  for (let i = 0; i < 8; i++) {
    bytes[i] = Number(remaining & 0xffn);
    remaining = remaining >> 8n;
  }
  return bytes;
}

/**
 * Decode a u64 (little-endian)
 */
export function decodeU64(bytes: Uint8Array): bigint {
  if (bytes.length !== 8) {
    throw new ValidationError('bytes', 'u64 must be 8 bytes', bytes.length);
  }

  let value = 0n;
  for (let i = 7; i >= 0; i--) {
    value = (value << 8n) | BigInt(bytes[i]);
  }
  return value;
}

/**
 * Encode a string (with length prefix)
 */
export function encodeString(str: string): Uint8Array {
  const encoder = new TextEncoder();
  const strBytes = encoder.encode(str);

  // 4-byte length prefix (u32 little-endian)
  const length = encodeU32(strBytes.length);

  const result = new Uint8Array(4 + strBytes.length);
  result.set(length, 0);
  result.set(strBytes, 4);

  return result;
}

/**
 * Encode a vector/array
 */
export function encodeVec<T>(
  items: T[],
  encodeItem: (item: T) => Uint8Array
): Uint8Array {
  const encoded = items.map(encodeItem);

  // 4-byte length prefix
  const length = encodeU32(items.length);
  const dataLength = encoded.reduce((sum, e) => sum + e.length, 0);

  const result = new Uint8Array(4 + dataLength);
  result.set(length, 0);

  let offset = 4;
  for (const item of encoded) {
    result.set(item, offset);
    offset += item.length;
  }

  return result;
}

/**
 * Encode an optional value
 */
export function encodeOption<T>(
  value: T | null | undefined,
  encodeValue: (v: T) => Uint8Array
): Uint8Array {
  if (value === null || value === undefined) {
    return new Uint8Array([0]); // None
  }

  const encoded = encodeValue(value);
  const result = new Uint8Array(1 + encoded.length);
  result[0] = 1; // Some
  result.set(encoded, 1);
  return result;
}

// =============================================================================
// Common Program Interfaces
// =============================================================================

/**
 * System program ID
 */
export const SYSTEM_PROGRAM_ID = zeroPubkey();

/**
 * Token program ID (SPL Token)
 */
export const TOKEN_PROGRAM_ID = '0x' + '06ddf6e1d765a193d9cbe146ceeb79ac1cb485ed5f5b37913a8cf5857eff00a9';

/**
 * Associated Token program ID
 */
export const ASSOCIATED_TOKEN_PROGRAM_ID = '0x' + '8c97258f4e2489f1bb3d1029148e0d830b5a1399daff1084048e7bd8dbe9f859';

/**
 * Encode a transfer instruction (System program)
 */
export function encodeSystemTransfer(amount: bigint): Uint8Array {
  // System program transfer discriminator: 2 (u32)
  return encodeInstructionData(
    encodeU32(2),
    [encodeU64(amount)]
  );
}

/**
 * Encode a SPL Token transfer instruction
 */
export function encodeTokenTransfer(amount: bigint): Uint8Array {
  // SPL Token transfer discriminator: 3
  return encodeInstructionData(
    [3],
    [encodeU64(amount)]
  );
}

/**
 * Create account metas for a transfer
 */
export function createTransferAccounts(
  from: Pubkey,
  to: Pubkey
): AccountMeta[] {
  return [
    { pubkey: from, isSigner: true, isWritable: true },
    { pubkey: to, isSigner: false, isWritable: true },
  ];
}

// =============================================================================
// Anchor Discriminator
// =============================================================================

/**
 * Compute Anchor instruction discriminator
 * Discriminator is first 8 bytes of SHA256("global:<instruction_name>")
 */
export function anchorDiscriminator(instructionName: string): Uint8Array {
  const input = `global:${instructionName}`;
  const hash = sha256AsU8a(new TextEncoder().encode(input));
  return hash.slice(0, 8);
}

/**
 * Compute Anchor account discriminator
 * Discriminator is first 8 bytes of SHA256("account:<AccountName>")
 */
export function anchorAccountDiscriminator(accountName: string): Uint8Array {
  const input = `account:${accountName}`;
  const hash = sha256AsU8a(new TextEncoder().encode(input));
  return hash.slice(0, 8);
}

// =============================================================================
// Private Utilities
// =============================================================================

/**
 * Hash seeds + programId with SHA256 for PDA derivation
 */
function simpleHash(programId: Uint8Array, seeds: Uint8Array[]): Uint8Array {
  const combined = new Uint8Array(
    programId.length + seeds.reduce((sum, s) => sum + s.length, 0)
  );

  let offset = 0;
  combined.set(programId, offset);
  offset += programId.length;

  for (const seed of seeds) {
    combined.set(seed, offset);
    offset += seed.length;
  }

  return sha256AsU8a(combined);
}


