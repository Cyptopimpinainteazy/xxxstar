/**
 * ComitBuilder - Fluent builder for constructing Comit transactions
 *
 * Provides a type-safe, chainable API for building cross-VM transactions.
 */

import type { HexString } from '@polkadot/util/types';
import type {
  AccountId,
  Balance,
  Hash,
  Nonce,
  Comit,
  ComitInput,
} from './types';

import { ValidationError, PayloadSizeError } from './errors';
import {
  MAX_EVM_PAYLOAD_SIZE,
  MAX_SVM_PAYLOAD_SIZE,
  MAX_COMBINED_PAYLOAD_SIZE,
  BASE_COMIT_FEE,
  GAS_PRICE,
  COMPUTE_UNIT_PRICE,
  GAS_FEE_DIVISOR,
  COMPUTE_FEE_DIVISOR,
  DEFAULT_EVM_GAS_LIMIT,
  DEFAULT_SVM_COMPUTE_UNITS,
} from './constants';

import {
  toBytes,
  computePrepareRoot,
  computeComitId,
} from './utils';

// =============================================================================
// Types
// =============================================================================

/**
 * Options for EVM payload
 */
export interface EvmPayloadOptions {
  /** Target contract address */
  to?: HexString;
  /** Value to send (in wei) */
  value?: bigint;
  /** Calldata */
  data?: HexString | Uint8Array;
  /** Gas limit (optional, uses default) */
  gasLimit?: bigint;
}

/**
 * Options for SVM payload
 */
export interface SvmPayloadOptions {
  /** Program ID to call */
  programId?: HexString;
  /** Instruction data */
  data?: HexString | Uint8Array;
  /** Compute units (optional, uses default) */
  computeUnits?: bigint;
}

/**
 * Builder state tracking
 */
interface BuilderState {
  evmPayload?: Uint8Array;
  svmPayload?: Uint8Array;
  fee?: Balance;
  evmGas?: bigint;
  svmCompute?: bigint;
  prepareRoot?: Hash;
  origin?: AccountId;
  nonce?: Nonce;
}

// =============================================================================
// ComitBuilder
// =============================================================================

/**
 * Fluent builder for constructing Comit transactions
 *
 * @example
 * ```typescript
 * const comit = new ComitBuilder()
 *   .withEvmPayload({
 *     to: '0x1234...',
 *     data: '0xabcd...',
 *     value: 1000000000000000000n,
 *   })
 *   .withSvmPayload({
 *     programId: '0x5678...',
 *     data: instructionData,
 *   })
 *   .withFee('auto')
 *   .build();
 *
 * const result = await client.submitComit(comit, signerAccount);
 * ```
 */
export class ComitBuilder {
  private state: BuilderState = {};

  // ===========================================================================
  // EVM Payload Methods
  // ===========================================================================

  /**
   * Set the EVM payload
   */
  withEvmPayload(payload: HexString | Uint8Array | EvmPayloadOptions): this {
    if (payload instanceof Uint8Array) {
      this.state.evmPayload = payload;
    } else if (typeof payload === 'string') {
      this.state.evmPayload = toBytes(payload);
    } else {
      this.state.evmPayload = this.encodeEvmPayload(payload);
      if (payload.gasLimit) {
        this.state.evmGas = payload.gasLimit;
      }
    }

    this.validatePayloadSize('evm', this.state.evmPayload);
    return this;
  }

  /**
   * Set EVM contract call
   */
  evmCall(to: HexString, data: HexString | Uint8Array, value: bigint = 0n): this {
    return this.withEvmPayload({ to, data, value });
  }

  /**
   * Set EVM contract deployment
   */
  evmDeploy(bytecode: HexString | Uint8Array, constructorArgs?: Uint8Array): this {
    const code = toBytes(bytecode);
    const payload = constructorArgs
      ? new Uint8Array([...code, ...constructorArgs])
      : code;
    return this.withEvmPayload(payload);
  }

  /**
   * Set EVM gas limit
   */
  withEvmGasLimit(gasLimit: bigint): this {
    if (gasLimit <= 0n) {
      throw new ValidationError('gasLimit', 'Gas limit must be positive', gasLimit);
    }
    this.state.evmGas = gasLimit;
    return this;
  }

  // ===========================================================================
  // SVM Payload Methods
  // ===========================================================================

  /**
   * Set the SVM payload
   */
  withSvmPayload(payload: HexString | Uint8Array | SvmPayloadOptions): this {
    if (payload instanceof Uint8Array) {
      this.state.svmPayload = payload;
    } else if (typeof payload === 'string') {
      this.state.svmPayload = toBytes(payload);
    } else {
      this.state.svmPayload = this.encodeSvmPayload(payload);
      if (payload.computeUnits) {
        this.state.svmCompute = payload.computeUnits;
      }
    }

    this.validatePayloadSize('svm', this.state.svmPayload);
    return this;
  }

  /**
   * Set SVM program call
   */
  svmCall(programId: HexString, data: HexString | Uint8Array): this {
    return this.withSvmPayload({ programId, data });
  }

  /**
   * Set SVM compute units
   */
  withSvmComputeUnits(computeUnits: bigint): this {
    if (computeUnits <= 0n) {
      throw new ValidationError('computeUnits', 'Compute units must be positive', computeUnits);
    }
    this.state.svmCompute = computeUnits;
    return this;
  }

  // ===========================================================================
  // Fee Methods
  // ===========================================================================

  /**
   * Set the fee explicitly or calculate automatically
   *
   * @param fee - Explicit fee in smallest unit, or 'auto' to calculate
   */
  withFee(fee: Balance | 'auto'): this {
    if (fee === 'auto') {
      this.state.fee = this.calculateFee();
    } else {
      if (fee < 0n) {
        throw new ValidationError('fee', 'Fee cannot be negative', fee);
      }
      this.state.fee = fee;
    }
    return this;
  }

  /**
   * Calculate the fee based on payloads and gas/compute units
   */
  calculateFee(): Balance {
    let fee = BASE_COMIT_FEE;

    // EVM gas fee
    const evmGas = this.state.evmGas ?? DEFAULT_EVM_GAS_LIMIT;
    fee += (evmGas * GAS_PRICE) / GAS_FEE_DIVISOR;

    // SVM compute fee
    const svmCompute = this.state.svmCompute ?? DEFAULT_SVM_COMPUTE_UNITS;
    fee += (svmCompute * COMPUTE_UNIT_PRICE) / COMPUTE_FEE_DIVISOR;

    return fee;
  }

  // ===========================================================================
  // Identity Methods
  // ===========================================================================

  /**
   * Set the origin account (for prepare_root computation)
   */
  withOrigin(origin: AccountId): this {
    this.state.origin = origin;
    return this;
  }

  /**
   * Set the nonce (for prepare_root computation)
   */
  withNonce(nonce: Nonce): this {
    if (nonce < 0n) {
      throw new ValidationError('nonce', 'Nonce cannot be negative', nonce);
    }
    this.state.nonce = nonce;
    return this;
  }

  /**
   * Set an explicit prepare_root (overrides computation)
   */
  withPrepareRoot(prepareRoot: Hash): this {
    this.state.prepareRoot = prepareRoot;
    return this;
  }

  // ===========================================================================
  // Build Methods
  // ===========================================================================

  /**
   * Validate the current builder state
   */
  validate(): string[] {
    const errors: string[] = [];

    // Must have at least one payload
    if (!this.state.evmPayload?.length && !this.state.svmPayload?.length) {
      errors.push('At least one payload (EVM or SVM) must be provided');
    }

    // Validate combined size
    const evmSize = this.state.evmPayload?.length ?? 0;
    const svmSize = this.state.svmPayload?.length ?? 0;
    if (evmSize + svmSize > MAX_COMBINED_PAYLOAD_SIZE) {
      errors.push(`Combined payload size ${evmSize + svmSize} exceeds maximum ${MAX_COMBINED_PAYLOAD_SIZE}`);
    }

    // Fee required for build
    if (this.state.fee === undefined && !this.canAutoCalculateFee()) {
      // Will auto-calculate, so not an error
    }

    return errors;
  }

  /**
   * Check if builder is in a valid state for building
   */
  isValid(): boolean {
    return this.validate().length === 0;
  }

  /**
   * Build the ComitInput for submission
   *
   * @throws ValidationError if builder state is invalid
   */
  build(): ComitInput {
    const errors = this.validate();
    if (errors.length > 0) {
      throw new ValidationError('ComitBuilder', errors.join('; '));
    }

    const evmPayload = this.state.evmPayload ?? new Uint8Array(0);
    const svmPayload = this.state.svmPayload ?? new Uint8Array(0);
    const fee = this.state.fee ?? this.calculateFee();

    // Compute prepare_root if origin and nonce are set
    let prepareRoot = this.state.prepareRoot;
    if (!prepareRoot && this.state.origin && this.state.nonce !== undefined) {
      prepareRoot = computePrepareRoot(
        this.state.origin,
        evmPayload,
        svmPayload,
        this.state.nonce,
        fee
      );
    }

    return {
      evmPayload,
      svmPayload,
      fee,
      prepareRoot,
    };
  }

  /**
   * Build the complete Comit structure (requires origin and nonce)
   *
   * @throws ValidationError if origin or nonce not set
   */
  buildComit(): Comit {
    if (!this.state.origin) {
      throw new ValidationError('origin', 'Origin must be set to build complete Comit');
    }
    if (this.state.nonce === undefined) {
      throw new ValidationError('nonce', 'Nonce must be set to build complete Comit');
    }

    const errors = this.validate();
    if (errors.length > 0) {
      throw new ValidationError('ComitBuilder', errors.join('; '));
    }

    const evmPayload = this.state.evmPayload ?? new Uint8Array(0);
    const svmPayload = this.state.svmPayload ?? new Uint8Array(0);
    const fee = this.state.fee ?? this.calculateFee();

    const prepareRoot = this.state.prepareRoot ?? computePrepareRoot(
      this.state.origin,
      evmPayload,
      svmPayload,
      this.state.nonce,
      fee
    );

    const comitId = computeComitId(prepareRoot);

    return {
      comitId,
      origin: this.state.origin,
      evmPayload,
      svmPayload,
      nonce: this.state.nonce,
      fee,
      prepareRoot,
    };
  }

  /**
   * Reset the builder to initial state
   */
  reset(): this {
    this.state = {};
    return this;
  }

  /**
   * Clone the builder with current state
   */
  clone(): ComitBuilder {
    const clone = new ComitBuilder();
    clone.state = { ...this.state };
    if (this.state.evmPayload) {
      clone.state.evmPayload = new Uint8Array(this.state.evmPayload);
    }
    if (this.state.svmPayload) {
      clone.state.svmPayload = new Uint8Array(this.state.svmPayload);
    }
    return clone;
  }

  // ===========================================================================
  // Private Methods
  // ===========================================================================

  private encodeEvmPayload(options: EvmPayloadOptions): Uint8Array {
    // Simple encoding: to (20 bytes) + value (32 bytes) + data
    // In production, use RLP encoding or ethers.js
    const parts: Uint8Array[] = [];

    if (options.to) {
      parts.push(toBytes(options.to));
    } else {
      parts.push(new Uint8Array(20)); // Zero address for deployment
    }

    // Encode value as 32-byte big-endian
    const valueBytes = new Uint8Array(32);
    if (options.value) {
      let val = options.value;
      for (let i = 31; i >= 0 && val > 0n; i--) {
        valueBytes[i] = Number(val & 0xffn);
        val = val >> 8n;
      }
    }
    parts.push(valueBytes);

    if (options.data) {
      parts.push(toBytes(options.data));
    }

    // Concatenate all parts
    const totalLength = parts.reduce((sum, p) => sum + p.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const part of parts) {
      result.set(part, offset);
      offset += part.length;
    }

    return result;
  }

  private encodeSvmPayload(options: SvmPayloadOptions): Uint8Array {
    // Simple encoding: programId (32 bytes) + data
    const parts: Uint8Array[] = [];

    if (options.programId) {
      const programIdBytes = toBytes(options.programId);
      // Pad to 32 bytes if needed
      if (programIdBytes.length < 32) {
        const padded = new Uint8Array(32);
        padded.set(programIdBytes);
        parts.push(padded);
      } else {
        parts.push(programIdBytes.slice(0, 32));
      }
    } else {
      parts.push(new Uint8Array(32)); // Zero program ID
    }

    if (options.data) {
      parts.push(toBytes(options.data));
    }

    // Concatenate all parts
    const totalLength = parts.reduce((sum, p) => sum + p.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const part of parts) {
      result.set(part, offset);
      offset += part.length;
    }

    return result;
  }

  private validatePayloadSize(type: 'evm' | 'svm', payload: Uint8Array): void {
    const maxSize = type === 'evm' ? MAX_EVM_PAYLOAD_SIZE : MAX_SVM_PAYLOAD_SIZE;
    if (payload.length > maxSize) {
      throw new PayloadSizeError(type, payload.length, maxSize);
    }
  }

  private canAutoCalculateFee(): boolean {
    // Can always auto-calculate with defaults
    return true;
  }
}

// =============================================================================
// Factory Functions
// =============================================================================

/**
 * Create a new ComitBuilder
 */
export function comit(): ComitBuilder {
  return new ComitBuilder();
}

/**
 * Create a ComitBuilder with EVM-only payload
 */
export function evmComit(payload: HexString | Uint8Array | EvmPayloadOptions): ComitBuilder {
  return new ComitBuilder().withEvmPayload(payload);
}

/**
 * Create a ComitBuilder with SVM-only payload
 */
export function svmComit(payload: HexString | Uint8Array | SvmPayloadOptions): ComitBuilder {
  return new ComitBuilder().withSvmPayload(payload);
}

/**
 * Create a ComitBuilder with both EVM and SVM payloads
 */
export function dualComit(
  evmPayload: HexString | Uint8Array | EvmPayloadOptions,
  svmPayload: HexString | Uint8Array | SvmPayloadOptions
): ComitBuilder {
  return new ComitBuilder()
    .withEvmPayload(evmPayload)
    .withSvmPayload(svmPayload);
}
