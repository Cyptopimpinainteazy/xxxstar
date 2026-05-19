"use strict";
/**
 * Error types for X3 Chain SDK
 *
 * Provides typed error handling for all SDK operations.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.ValidationError = exports.SubscriptionError = exports.TimeoutError = exports.PayloadSizeError = exports.VerificationError = exports.SvmExecutionError = exports.EvmExecutionError = exports.RateLimitError = exports.UnauthorizedError = exports.InsufficientBalanceError = exports.InvalidNonceError = exports.ComitSubmissionError = exports.RpcError = exports.ConnectionError = exports.AtlasSphereError = void 0;
exports.reasonToError = reasonToError;
/**
 * Base class for all X3 Chain SDK errors
 */
class AtlasSphereError extends Error {
    constructor(message) {
        super(message);
        this.name = 'AtlasSphereError';
        Object.setPrototypeOf(this, AtlasSphereError.prototype);
    }
}
exports.AtlasSphereError = AtlasSphereError;
/**
 * Error thrown when connection to the node fails
 */
class ConnectionError extends AtlasSphereError {
    endpoint;
    cause;
    constructor(endpoint, cause) {
        super(`Failed to connect to X3 Chain node at ${endpoint}: ${cause?.message || 'Unknown error'}`);
        this.name = 'ConnectionError';
        this.endpoint = endpoint;
        this.cause = cause;
        Object.setPrototypeOf(this, ConnectionError.prototype);
    }
}
exports.ConnectionError = ConnectionError;
/**
 * Error thrown when RPC call fails
 */
class RpcError extends AtlasSphereError {
    method;
    code;
    data;
    constructor(method, message, code, data) {
        super(`RPC call '${method}' failed: ${message}`);
        this.name = 'RpcError';
        this.method = method;
        this.code = code;
        this.data = data;
        Object.setPrototypeOf(this, RpcError.prototype);
    }
}
exports.RpcError = RpcError;
/**
 * Error thrown when Comit submission fails
 */
class ComitSubmissionError extends AtlasSphereError {
    comitId;
    reason;
    constructor(message, comitId, reason) {
        super(message);
        this.name = 'ComitSubmissionError';
        this.comitId = comitId;
        this.reason = reason;
        Object.setPrototypeOf(this, ComitSubmissionError.prototype);
    }
}
exports.ComitSubmissionError = ComitSubmissionError;
/**
 * Error thrown when account nonce is invalid
 */
class InvalidNonceError extends ComitSubmissionError {
    expected;
    provided;
    constructor(expected, provided) {
        super(`Invalid nonce: expected ${expected}, got ${provided}`);
        this.name = 'InvalidNonceError';
        this.expected = expected;
        this.provided = provided;
        Object.setPrototypeOf(this, InvalidNonceError.prototype);
    }
}
exports.InvalidNonceError = InvalidNonceError;
/**
 * Error thrown when account has insufficient balance
 */
class InsufficientBalanceError extends ComitSubmissionError {
    required;
    available;
    constructor(required, available) {
        super(`Insufficient balance: required ${required}, available ${available}`);
        this.name = 'InsufficientBalanceError';
        this.required = required;
        this.available = available;
        Object.setPrototypeOf(this, InsufficientBalanceError.prototype);
    }
}
exports.InsufficientBalanceError = InsufficientBalanceError;
/**
 * Error thrown when account is not authorized
 */
class UnauthorizedError extends ComitSubmissionError {
    account;
    constructor(account) {
        super(`Account ${account} is not authorized to submit Comits`);
        this.name = 'UnauthorizedError';
        this.account = account;
        Object.setPrototypeOf(this, UnauthorizedError.prototype);
    }
}
exports.UnauthorizedError = UnauthorizedError;
/**
 * Error thrown when account exceeds rate limit
 */
class RateLimitError extends ComitSubmissionError {
    account;
    window;
    constructor(account, window) {
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
exports.RateLimitError = RateLimitError;
/**
 * Error thrown when EVM execution fails
 */
class EvmExecutionError extends AtlasSphereError {
    gasUsed;
    revertData;
    constructor(message, gasUsed, revertData) {
        super(`EVM execution failed: ${message}`);
        this.name = 'EvmExecutionError';
        this.gasUsed = gasUsed;
        this.revertData = revertData;
        Object.setPrototypeOf(this, EvmExecutionError.prototype);
    }
    /**
     * Attempt to decode the revert reason from revert data
     */
    getRevertReason() {
        if (!this.revertData || this.revertData.length < 4) {
            return null;
        }
        // Check for Error(string) selector: 0x08c379a0
        const selector = this.revertData.slice(0, 4);
        if (selector[0] === 0x08 &&
            selector[1] === 0xc3 &&
            selector[2] === 0x79 &&
            selector[3] === 0xa0) {
            try {
                // Skip selector (4) + offset (32) + length (32), decode string
                const length = Number(BigInt('0x' +
                    Buffer.from(this.revertData.slice(36, 68))
                        .toString('hex')));
                const text = new TextDecoder().decode(this.revertData.slice(68, 68 + length));
                return text;
            }
            catch {
                return null;
            }
        }
        return null;
    }
}
exports.EvmExecutionError = EvmExecutionError;
/**
 * Error thrown when SVM execution fails
 */
class SvmExecutionError extends AtlasSphereError {
    computeUnits;
    programError;
    constructor(message, computeUnits, programError) {
        super(`SVM execution failed: ${message}`);
        this.name = 'SvmExecutionError';
        this.computeUnits = computeUnits;
        this.programError = programError;
        Object.setPrototypeOf(this, SvmExecutionError.prototype);
    }
}
exports.SvmExecutionError = SvmExecutionError;
/**
 * Error thrown when Comit verification fails
 */
class VerificationError extends AtlasSphereError {
    expectedRoot;
    computedRoot;
    constructor(message, expectedRoot, computedRoot) {
        super(`Comit verification failed: ${message}`);
        this.name = 'VerificationError';
        this.expectedRoot = expectedRoot;
        this.computedRoot = computedRoot;
        Object.setPrototypeOf(this, VerificationError.prototype);
    }
}
exports.VerificationError = VerificationError;
/**
 * Error thrown when payload size exceeds limits
 */
class PayloadSizeError extends AtlasSphereError {
    payloadType;
    size;
    maxSize;
    constructor(payloadType, size, maxSize) {
        super(`${payloadType.toUpperCase()} payload size ${size} exceeds maximum ${maxSize} bytes`);
        this.name = 'PayloadSizeError';
        this.payloadType = payloadType;
        this.size = size;
        this.maxSize = maxSize;
        Object.setPrototypeOf(this, PayloadSizeError.prototype);
    }
}
exports.PayloadSizeError = PayloadSizeError;
/**
 * Error thrown when an operation times out
 */
class TimeoutError extends AtlasSphereError {
    operation;
    timeoutMs;
    constructor(operation, timeoutMs) {
        super(`Operation '${operation}' timed out after ${timeoutMs}ms`);
        this.name = 'TimeoutError';
        this.operation = operation;
        this.timeoutMs = timeoutMs;
        Object.setPrototypeOf(this, TimeoutError.prototype);
    }
}
exports.TimeoutError = TimeoutError;
/**
 * Error thrown when subscription fails
 */
class SubscriptionError extends AtlasSphereError {
    subscriptionType;
    constructor(subscriptionType, message) {
        super(`Subscription '${subscriptionType}' failed: ${message}`);
        this.name = 'SubscriptionError';
        this.subscriptionType = subscriptionType;
        Object.setPrototypeOf(this, SubscriptionError.prototype);
    }
}
exports.SubscriptionError = SubscriptionError;
/**
 * Error thrown for invalid input parameters
 */
class ValidationError extends AtlasSphereError {
    field;
    value;
    constructor(field, message, value) {
        super(`Validation error for '${field}': ${message}`);
        this.name = 'ValidationError';
        this.field = field;
        this.value = value;
        Object.setPrototypeOf(this, ValidationError.prototype);
    }
}
exports.ValidationError = ValidationError;
/**
 * Convert a ComitFailureReason to the appropriate error type
 */
function reasonToError(reason, comitId) {
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
            return new ComitSubmissionError(`EVM execution failed: ${reason.error}`, comitId, reason);
        case 'SvmExecutionFailed':
            return new ComitSubmissionError(`SVM execution failed: ${reason.error}`, comitId, reason);
        case 'VerificationFailed':
            return new ComitSubmissionError(`Verification failed: ${reason.reason}`, comitId, reason);
        case 'DuplicateComitId':
            return new ComitSubmissionError(`Duplicate Comit ID: ${comitId}`, comitId, reason);
        default:
            return new ComitSubmissionError('Unknown error', comitId, reason);
    }
}
//# sourceMappingURL=errors.js.map