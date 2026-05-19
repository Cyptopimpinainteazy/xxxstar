/**
 * Error types for X3 Chain SDK
 *
 * Provides typed error handling for all SDK operations.
 */

import type { ComitFailureReason, Hash, Balance, Nonce } from './types';

/**
 * Base class for all X3 Chain SDK errors
 */
export class AtlasSphereError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'AtlasSphereError';
    Object.setPrototypeOf(this, AtlasSphereError.prototype);
  }
}

/**
 * Error thrown when connection to the node fails
 */
export class ConnectionError extends AtlasSphereError {
  public readonly endpoint: string;
  public readonly cause?: Error;

  constructor(endpoint: string, cause?: Error) {
    super(`Failed to connect to X3 Chain node at ${endpoint}: ${cause?.message || 'Unknown error'}`);
    this.name = 'ConnectionError';
    this.endpoint = endpoint;
    this.cause = cause;
    Object.setPrototypeOf(this, ConnectionError.prototype);
  }
}

/**
 * Error thrown when RPC call fails
 */
export class RpcError extends AtlasSphereError {
  public readonly method: string;
  public readonly code?: number;
  public readonly data?: unknown;

  constructor(method: string, message: string, code?: number, data?: unknown) {
    super(`RPC call '${method}' failed: ${message}`);
    this.name = 'RpcError';
    this.method = method;
    this.code = code;
    this.data = data;
    Object.setPrototypeOf(this, RpcError.prototype);
  }
}

/**
 * Error thrown when Comit submission fails
 */
export class ComitSubmissionError extends AtlasSphereError {
  public readonly comitId?: Hash;
  public readonly reason?: ComitFailureReason;

  constructor(message: string, comitId?: Hash, reason?: ComitFailureReason) {
    super(message);
    this.name = 'ComitSubmissionError';
    this.comitId = comitId;
    this.reason = reason;
    Object.setPrototypeOf(this, ComitSubmissionError.prototype);
  }
}

/**
 * Error thrown when account nonce is invalid
 */
export class InvalidNonceError extends ComitSubmissionError {
  public readonly expected: Nonce;
  public readonly provided: Nonce;

  constructor(expected: Nonce, provided: Nonce) {
    super(`Invalid nonce: expected ${expected}, got ${provided}`);
    this.name = 'InvalidNonceError';
    this.expected = expected;
    this.provided = provided;
    Object.setPrototypeOf(this, InvalidNonceError.prototype);
  }
}

/**
 * Error thrown when account has insufficient balance
 */
export class InsufficientBalanceError extends ComitSubmissionError {
  public readonly required: Balance;
  public readonly available: Balance;

  constructor(required: Balance, available: Balance) {
    super(`Insufficient balance: required ${required}, available ${available}`);
    this.name = 'InsufficientBalanceError';
    this.required = required;
    this.available = available;
    Object.setPrototypeOf(this, InsufficientBalanceError.prototype);
  }
}

/**
 * Error thrown when account is not authorized
 */
export class UnauthorizedError extends ComitSubmissionError {
  public readonly account: string;

  constructor(account: string) {
    super(`Account ${account} is not authorized to submit Comits`);
    this.name = 'UnauthorizedError';
    this.account = account;
    Object.setPrototypeOf(this, UnauthorizedError.prototype);
  }
}

/**
 * Error thrown when account exceeds rate limit
 */
export class RateLimitError extends ComitSubmissionError {
  public readonly account: string;
  public readonly window?: number;

  constructor(account: string, window?: number) {
    const msg = window
      ? `Account ${account} exceeded rate limit (window: ${window}s)`
      : `Account ${account} exceeded rate limit`;
    super(msg);
    this.name = 'RateLimitError';
    this.account = account;
    this.window = window;
    Object.setPrototypeOf(this, RateLimitError.prototype);
  }
}

/**
 * Error thrown when EVM execution fails
 */
export class EvmExecutionError extends AtlasSphereError {
  public readonly gasUsed: bigint;
  public readonly revertData?: Uint8Array;

  constructor(message: string, gasUsed: bigint, revertData?: Uint8Array) {
    super(`EVM execution failed: ${message}`);
    this.name = 'EvmExecutionError';
    this.gasUsed = gasUsed;
    this.revertData = revertData;
    Object.setPrototypeOf(this, EvmExecutionError.prototype);
  }

  /**
   * Attempt to decode the revert reason from revert data
   */
  getRevertReason(): string | null {
    if (!this.revertData || this.revertData.length < 4) {
      return null;
    }

    // Check for Error(string) selector: 0x08c379a0
    const selector = this.revertData.slice(0, 4);
    if (
      selector[0] === 0x08 &&
      selector[1] === 0xc3 &&
      selector[2] === 0x79 &&
      selector[3] === 0xa0
    ) {
      try {
        // Skip selector (4) + offset (32) + length (32), decode string
        const length = Number(
          BigInt(
            '0x' +
              Buffer.from(this.revertData.slice(36, 68))
                .toString('hex')
          )
        );
        const text = new TextDecoder().decode(
          this.revertData.slice(68, 68 + length)
        );
        return text;
      } catch {
        return null;
      }
    }

    return null;
  }
}

/**
 * Error thrown when SVM execution fails
 */
export class SvmExecutionError extends AtlasSphereError {
  public readonly computeUnits: bigint;
  public readonly programError?: string;

  constructor(message: string, computeUnits: bigint, programError?: string) {
    super(`SVM execution failed: ${message}`);
    this.name = 'SvmExecutionError';
    this.computeUnits = computeUnits;
    this.programError = programError;
    Object.setPrototypeOf(this, SvmExecutionError.prototype);
  }
}

/**
 * Error thrown when Comit verification fails
 */
export class VerificationError extends AtlasSphereError {
  public readonly expectedRoot?: Hash;
  public readonly computedRoot?: Hash;

  constructor(message: string, expectedRoot?: Hash, computedRoot?: Hash) {
    super(`Comit verification failed: ${message}`);
    this.name = 'VerificationError';
    this.expectedRoot = expectedRoot;
    this.computedRoot = computedRoot;
    Object.setPrototypeOf(this, VerificationError.prototype);
  }
}

/**
 * Error thrown when payload size exceeds limits
 */
export class PayloadSizeError extends AtlasSphereError {
  public readonly payloadType: 'evm' | 'svm' | 'combined';
  public readonly size: number;
  public readonly maxSize: number;

  constructor(payloadType: 'evm' | 'svm' | 'combined', size: number, maxSize: number) {
    super(`${payloadType.toUpperCase()} payload size ${size} exceeds maximum ${maxSize} bytes`);
    this.name = 'PayloadSizeError';
    this.payloadType = payloadType;
    this.size = size;
    this.maxSize = maxSize;
    Object.setPrototypeOf(this, PayloadSizeError.prototype);
  }
}

/**
 * Error thrown when an operation times out
 */
export class TimeoutError extends AtlasSphereError {
  public readonly operation: string;
  public readonly timeoutMs: number;

  constructor(operation: string, timeoutMs: number) {
    super(`Operation '${operation}' timed out after ${timeoutMs}ms`);
    this.name = 'TimeoutError';
    this.operation = operation;
    this.timeoutMs = timeoutMs;
    Object.setPrototypeOf(this, TimeoutError.prototype);
  }
}

/**
 * Error thrown when subscription fails
 */
export class SubscriptionError extends AtlasSphereError {
  public readonly subscriptionType: string;

  constructor(subscriptionType: string, message: string) {
    super(`Subscription '${subscriptionType}' failed: ${message}`);
    this.name = 'SubscriptionError';
    this.subscriptionType = subscriptionType;
    Object.setPrototypeOf(this, SubscriptionError.prototype);
  }
}

/**
 * Error thrown for invalid input parameters
 */
export class ValidationError extends AtlasSphereError {
  public readonly field: string;
  public readonly value?: unknown;

  constructor(field: string, message: string, value?: unknown) {
    super(`Validation error for '${field}': ${message}`);
    this.name = 'ValidationError';
    this.field = field;
    this.value = value;
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

/**
 * Convert a ComitFailureReason to the appropriate error type
 */
export function reasonToError(reason: ComitFailureReason, comitId?: Hash): ComitSubmissionError {
  switch (reason.type) {
    case 'InvalidNonce':
      return new InvalidNonceError(reason.expected, reason.provided);
    case 'InsufficientBalance':
      return new InsufficientBalanceError(reason.required, reason.available);
    case 'Unauthorized':
      return new UnauthorizedError('unknown');
    case 'RateLimitExceeded':
      return new RateLimitError('unknown');
    case 'EvmExecutionFailed':
      return new ComitSubmissionError(
        `EVM execution failed: ${reason.error}`,
        comitId,
        reason
      );
    case 'SvmExecutionFailed':
      return new ComitSubmissionError(
        `SVM execution failed: ${reason.error}`,
        comitId,
        reason
      );
    case 'VerificationFailed':
      return new ComitSubmissionError(
        `Verification failed: ${reason.reason}`,
        comitId,
        reason
      );
    case 'DuplicateComitId':
      return new ComitSubmissionError(
        `Duplicate Comit ID: ${comitId}`,
        comitId,
        reason
      );
    default:
      return new ComitSubmissionError('Unknown error', comitId, reason);
  }
}
