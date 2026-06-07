/**
 * SVM (Solana VM) utilities for X3 Chain SDK
 *
 * Provides encoding, decoding, and conversion utilities for SVM interaction.
 */
import type { HexString } from '@polkadot/util/types';
import type { AccountId } from './types';
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
/**
 * Validate a Solana pubkey
 */
export declare function isValidPubkey(pubkey: string): boolean;
/**
 * Convert pubkey to bytes
 */
export declare function pubkeyToBytes(pubkey: Pubkey): Uint8Array;
/**
 * Convert bytes to pubkey
 */
export declare function bytesToPubkey(bytes: Uint8Array): Pubkey;
/**
 * Create a zero pubkey (system program)
 */
export declare function zeroPubkey(): Pubkey;
/**
 * Convert Substrate AccountId to Solana pubkey
 */
export declare function accountIdToPubkey(accountId: AccountId): Pubkey;
/**
 * Convert Solana pubkey to Substrate AccountId
 */
export declare function pubkeyToAccountId(pubkey: Pubkey): AccountId;
/**
 * Derive program address (PDA)
 * Uses SHA256 seed hashing; curve validation is intentionally simplified.
 */
export declare function findProgramAddress(seeds: Uint8Array[], programId: Pubkey): {
    address: Pubkey;
    bump: number;
};
/**
 * Encode a compact u16 length
 */
export declare function encodeCompactU16(value: number): Uint8Array;
/**
 * Decode a compact u16 length
 */
export declare function decodeCompactU16(bytes: Uint8Array, offset?: number): CompactU16;
/**
 * Encode an instruction
 */
export declare function encodeInstruction(instruction: Instruction): Uint8Array;
/**
 * Encode instruction data with discriminator
 */
export declare function encodeInstructionData(discriminator: Uint8Array | number[], params: Uint8Array[]): Uint8Array;
/**
 * Encode a u8
 */
export declare function encodeU8(value: number): Uint8Array;
/**
 * Encode a u16 (little-endian)
 */
export declare function encodeU16(value: number): Uint8Array;
/**
 * Encode a u32 (little-endian)
 */
export declare function encodeU32(value: number): Uint8Array;
/**
 * Encode a u64 (little-endian)
 */
export declare function encodeU64(value: bigint): Uint8Array;
/**
 * Decode a u64 (little-endian)
 */
export declare function decodeU64(bytes: Uint8Array): bigint;
/**
 * Encode a string (with length prefix)
 */
export declare function encodeString(str: string): Uint8Array;
/**
 * Encode a vector/array
 */
export declare function encodeVec<T>(items: T[], encodeItem: (item: T) => Uint8Array): Uint8Array;
/**
 * Encode an optional value
 */
export declare function encodeOption<T>(value: T | null | undefined, encodeValue: (v: T) => Uint8Array): Uint8Array;
/**
 * System program ID
 */
export declare const SYSTEM_PROGRAM_ID: `0x${string}`;
/**
 * Token program ID (SPL Token)
 */
export declare const TOKEN_PROGRAM_ID: string;
/**
 * Associated Token program ID
 */
export declare const ASSOCIATED_TOKEN_PROGRAM_ID: string;
/**
 * Encode a transfer instruction (System program)
 */
export declare function encodeSystemTransfer(amount: bigint): Uint8Array;
/**
 * Encode a SPL Token transfer instruction
 */
export declare function encodeTokenTransfer(amount: bigint): Uint8Array;
/**
 * Create account metas for a transfer
 */
export declare function createTransferAccounts(from: Pubkey, to: Pubkey): AccountMeta[];
/**
 * Compute Anchor instruction discriminator
 * Discriminator is first 8 bytes of SHA256("global:<instruction_name>")
 */
export declare function anchorDiscriminator(instructionName: string): Uint8Array;
/**
 * Compute Anchor account discriminator
 * Discriminator is first 8 bytes of SHA256("account:<AccountName>")
 */
export declare function anchorAccountDiscriminator(accountName: string): Uint8Array;
//# sourceMappingURL=svm.d.ts.map