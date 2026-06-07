/**
 * Utility functions for X3 Chain SDK
 *
 * Provides encoding, hashing, and conversion utilities.
 */
import type { HexString } from '@polkadot/util/types';
import type { Hash, AccountId, Balance, Nonce } from './types';
/**
 * Convert a hex string to Uint8Array
 */
export declare function hexToBytes(hex: HexString): Uint8Array;
/**
 * Convert a Uint8Array to hex string
 */
export declare function bytesToHex(bytes: Uint8Array): HexString;
/**
 * Convert a string to Uint8Array (UTF-8)
 */
export declare function stringToBytes(str: string): Uint8Array;
/**
 * Ensure input is Uint8Array (convert from hex if needed)
 */
export declare function toBytes(input: HexString | Uint8Array): Uint8Array;
/**
 * Ensure input is hex string (convert from bytes if needed)
 */
export declare function toHex(input: HexString | Uint8Array): HexString;
/**
 * Compute BLAKE2-256 hash of input
 */
export declare function blake2_256(data: Uint8Array | string): Hash;
/**
 * Compute BLAKE2-256 hash as bytes
 */
export declare function blake2_256_bytes(data: Uint8Array | string): Uint8Array;
/**
 * Compute prepare_root for a Comit from its inputs
 * This matches the runtime's prepare_root computation
 */
export declare function computePrepareRoot(origin: AccountId, evmPayload: Uint8Array, svmPayload: Uint8Array, nonce: Nonce, fee: Balance): Hash;
/**
 * Compute Comit ID from prepare_root
 */
export declare function computeComitId(prepareRoot: Hash): Hash;
/**
 * Encode a bigint as little-endian U128 (16 bytes)
 */
export declare function encodeU128(value: bigint): Uint8Array;
/**
 * Decode a little-endian U128 to bigint
 */
export declare function decodeU128(bytes: Uint8Array): bigint;
/**
 * Encode a bigint as little-endian U64 (8 bytes)
 */
export declare function encodeU64(value: bigint): Uint8Array;
/**
 * Decode a little-endian U64 to bigint
 */
export declare function decodeU64(bytes: Uint8Array): bigint;
/**
 * Decode Base58-encoded string to bytes (for Solana-style pubkeys)
 */
export declare function base58DecodeSolana(input: string): Uint8Array;
/**
 * @deprecated Use base58DecodeSolana instead. This function is kept for backward compatibility.
 */
export declare function base58Decode(input: string): Uint8Array;
/**
 * Decode SS58-encoded account ID to bytes
 * Uses @polkadot/util-crypto's decodeAddress which handles all prefix widths
 * and validates the 2-byte checksum.
 */
export declare function decodeAccountId(accountId: AccountId): Uint8Array;
/**
 * Encode bytes to SS58 account ID
 * Uses @polkadot/util-crypto's encodeAddress for consistent round-trip encoding.
 */
export declare function encodeAccountId(bytes: Uint8Array): AccountId;
/**
 * Convert Substrate AccountId to EVM address (take first 20 bytes)
 */
export declare function accountIdToEvmAddress(accountId: AccountId): HexString;
/**
 * Convert EVM address to Substrate AccountId (pad with zeros)
 */
export declare function evmAddressToAccountId(evmAddress: HexString): AccountId;
/**
 * Validate an EVM address format
 */
export declare function isValidEvmAddress(address: string): boolean;
/**
 * Validate a Solana pubkey format (32 bytes)
 */
export declare function isValidSolanaPubkey(pubkey: string): boolean;
/**
 * Validate payload sizes for Comit submission
 */
export declare function validatePayloadSizes(evmPayload: Uint8Array, svmPayload: Uint8Array): void;
/**
 * Validate a hash is proper H256 format
 */
export declare function isValidH256(hash: string): boolean;
/**
 * Validate balance is non-negative
 */
export declare function validateBalance(balance: bigint, field?: string): void;
/**
 * Validate nonce is non-negative
 */
export declare function validateNonce(nonce: bigint, field?: string): void;
/**
 * Format balance with decimals for display
 */
export declare function formatBalance(balance: Balance, decimals?: number): string;
/**
 * Parse balance string to bigint
 */
export declare function parseBalance(value: string, decimals?: number): Balance;
/**
 * Truncate hash for display (e.g., "0x1234...5678")
 */
export declare function truncateHash(hash: Hash, chars?: number): string;
/**
 * Sleep for specified milliseconds
 */
export declare function sleep(ms: number): Promise<void>;
/**
 * Retry an async operation with exponential backoff
 */
export declare function retry<T>(operation: () => Promise<T>, options?: {
    maxAttempts?: number;
    initialDelayMs?: number;
    maxDelayMs?: number;
    backoffFactor?: number;
}): Promise<T>;
//# sourceMappingURL=utils.d.ts.map