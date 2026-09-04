/**
 * Error types for X3 Chain SDK
 *
 * Provides typed error handling for all SDK operations.
 */
import type { ComitFailureReason, Hash, Balance, Nonce } from './types';
/**
 * Base class for all X3 Chain SDK errors
 */
export declare class AtlasSphereError extends Error {
    constructor(message: string);
}
/**
 * Error thrown when connection to the node fails
 */
export declare class ConnectionError extends AtlasSphereError {
    readonly endpoint: string;
    readonly cause?: Error;
    constructor(endpoint: string, cause?: Error);
}
/**
 * Error thrown when RPC call fails
 */
export declare class RpcError extends AtlasSphereError {
    readonly method: string;
    readonly code?: number;
    readonly data?: unknown;
    constructor(method: string, message: string, code?: number, data?: unknown);
}
/**
 * Error thrown when Comit submission fails
 */
export declare class ComitSubmissionError extends AtlasSphereError {
    readonly comitId?: Hash;
    readonly reason?: ComitFailureReason;
    constructor(message: string, comitId?: Hash, reason?: ComitFailureReason);
}
/**
 * Error thrown when account nonce is invalid
 */
export declare class InvalidNonceError extends ComitSubmissionError {
    readonly expected: Nonce;
    readonly provided: Nonce;
    constructor(expected: Nonce, provided: Nonce);
}
/**
 * Error thrown when account has insufficient balance
 */
export declare class InsufficientBalanceError extends ComitSubmissionError {
    readonly required: Balance;
    readonly available: Balance;
    constructor(required: Balance, available: Balance);
}
/**
 * Error thrown when account is not authorized
 */
export declare class UnauthorizedError extends ComitSubmissionError {
    readonly account: string;
    constructor(account: string);
}
/**
 * Error thrown when account exceeds rate limit
 */
export declare class RateLimitError extends ComitSubmissionError {
    readonly account: string;
    readonly window?: number;
    constructor(account: string, window?: number);
}
/**
 * Error thrown when EVM execution fails
 */
export declare class EvmExecutionError extends AtlasSphereError {
    readonly gasUsed: bigint;
    readonly revertData?: Uint8Array;
    constructor(message: string, gasUsed: bigint, revertData?: Uint8Array);
    /**
     * Attempt to decode the revert reason from revert data
     */
    getRevertReason(): string | null;
}
/**
 * Error thrown when SVM execution fails
 */
export declare class SvmExecutionError extends AtlasSphereError {
    readonly computeUnits: bigint;
    readonly programError?: string;
    constructor(message: string, computeUnits: bigint, programError?: string);
}
/**
 * Error thrown when Comit verification fails
 */
export declare class VerificationError extends AtlasSphereError {
    readonly expectedRoot?: Hash;
    readonly computedRoot?: Hash;
    constructor(message: string, expectedRoot?: Hash, computedRoot?: Hash);
}
/**
 * Error thrown when payload size exceeds limits
 */
export declare class PayloadSizeError extends AtlasSphereError {
    readonly payloadType: 'evm' | 'svm' | 'combined';
    readonly size: number;
    readonly maxSize: number;
    constructor(payloadType: 'evm' | 'svm' | 'combined', size: number, maxSize: number);
}
/**
 * Error thrown when an operation times out
 */
export declare class TimeoutError extends AtlasSphereError {
    readonly operation: string;
    readonly timeoutMs: number;
    constructor(operation: string, timeoutMs: number);
}
/**
 * Error thrown when subscription fails
 */
export declare class SubscriptionError extends AtlasSphereError {
    readonly subscriptionType: string;
    constructor(subscriptionType: string, message: string);
}
/**
 * Error thrown for invalid input parameters
 */
export declare class ValidationError extends AtlasSphereError {
    readonly field: string;
    readonly value?: unknown;
    constructor(field: string, message: string, value?: unknown);
}
/**
 * Convert a ComitFailureReason to the appropriate error type
 */
export declare function reasonToError(reason: ComitFailureReason, comitId?: Hash): ComitSubmissionError;
//# sourceMappingURL=errors.d.ts.map