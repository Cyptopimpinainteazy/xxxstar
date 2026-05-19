/**
 * EVM utilities for X3 Chain SDK
 *
 * Provides encoding, decoding, and conversion utilities for EVM interaction.
 */
import type { HexString } from '@polkadot/util/types';
import type { AccountId } from './types';
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
/**
 * Validate an EVM address
 */
export declare function isValidAddress(address: string): boolean;
/**
 * Normalize an EVM address (lowercase with 0x prefix)
 */
export declare function normalizeAddress(address: string): HexString;
/**
 * Checksum an EVM address (EIP-55)
 */
export declare function checksumAddress(address: string): HexString;
/**
 * Convert Substrate AccountId to EVM address
 */
export declare function accountIdToAddress(accountId: AccountId): HexString;
/**
 * Convert EVM address to Substrate AccountId
 */
export declare function addressToAccountId(address: HexString): AccountId;
/**
 * Derive EVM address from public key
 */
export declare function publicKeyToAddress(publicKey: Uint8Array): HexString;
/**
 * Compute function selector from signature
 */
export declare function functionSelector(signature: string): HexString;
/**
 * Encode a uint256 value
 */
export declare function encodeUint256(value: bigint): Uint8Array;
/**
 * Decode a uint256 value
 */
export declare function decodeUint256(bytes: Uint8Array): bigint;
/**
 * Encode an address for ABI
 */
export declare function encodeAddress(address: HexString): Uint8Array;
/**
 * Decode an address from ABI encoding
 */
export declare function decodeAddress(bytes: Uint8Array): HexString;
/**
 * Encode bytes for ABI (dynamic type)
 */
export declare function encodeBytes(data: Uint8Array): Uint8Array;
/**
 * Encode a string for ABI (as bytes)
 */
export declare function encodeString(str: string): Uint8Array;
/**
 * Encode a boolean
 */
export declare function encodeBool(value: boolean): Uint8Array;
/**
 * Decode a boolean
 */
export declare function decodeBool(bytes: Uint8Array): boolean;
/**
 * Encode a function call with parameters
 */
export declare function encodeFunctionCall(signature: string, params: Uint8Array[]): Uint8Array;
/**
 * Decode a function call
 */
export declare function decodeFunctionCall(data: Uint8Array): DecodedCall;
/**
 * Encode ERC20 transfer call
 */
export declare function encodeTransfer(to: HexString, amount: bigint): Uint8Array;
/**
 * Encode ERC20 approve call
 */
export declare function encodeApprove(spender: HexString, amount: bigint): Uint8Array;
/**
 * Encode ERC20 transferFrom call
 */
export declare function encodeTransferFrom(from: HexString, to: HexString, amount: bigint): Uint8Array;
/**
 * Encode balanceOf call
 */
export declare function encodeBalanceOf(account: HexString): Uint8Array;
/**
 * Check if data is an Error(string) revert
 */
export declare function isErrorRevert(data: Uint8Array): boolean;
/**
 * Check if data is a Panic(uint256) revert
 */
export declare function isPanicRevert(data: Uint8Array): boolean;
/**
 * Decode Error(string) revert message
 */
export declare function decodeErrorMessage(data: Uint8Array): string | null;
/**
 * Decode Panic(uint256) code
 */
export declare function decodePanicCode(data: Uint8Array): bigint | null;
/**
 * Get human-readable panic message
 */
export declare function getPanicMessage(code: bigint): string;
//# sourceMappingURL=evm.d.ts.map