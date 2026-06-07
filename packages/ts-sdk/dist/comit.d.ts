/**
 * ComitBuilder - Fluent builder for constructing Comit transactions
 *
 * Provides a type-safe, chainable API for building cross-VM transactions.
 */
import type { HexString } from '@polkadot/util/types';
import type { AccountId, Balance, Hash, Nonce, Comit, ComitInput } from './types';
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
export declare class ComitBuilder {
    private state;
    /**
     * Set the EVM payload
     */
    withEvmPayload(payload: HexString | Uint8Array | EvmPayloadOptions): this;
    /**
     * Set EVM contract call
     */
    evmCall(to: HexString, data: HexString | Uint8Array, value?: bigint): this;
    /**
     * Set EVM contract deployment
     */
    evmDeploy(bytecode: HexString | Uint8Array, constructorArgs?: Uint8Array): this;
    /**
     * Set EVM gas limit
     */
    withEvmGasLimit(gasLimit: bigint): this;
    /**
     * Set the SVM payload
     */
    withSvmPayload(payload: HexString | Uint8Array | SvmPayloadOptions): this;
    /**
     * Set SVM program call
     */
    svmCall(programId: HexString, data: HexString | Uint8Array): this;
    /**
     * Set SVM compute units
     */
    withSvmComputeUnits(computeUnits: bigint): this;
    /**
     * Set the fee explicitly or calculate automatically
     *
     * @param fee - Explicit fee in smallest unit, or 'auto' to calculate
     */
    withFee(fee: Balance | 'auto'): this;
    /**
     * Calculate the fee based on payloads and gas/compute units
     */
    calculateFee(): Balance;
    /**
     * Set the origin account (for prepare_root computation)
     */
    withOrigin(origin: AccountId): this;
    /**
     * Set the nonce (for prepare_root computation)
     */
    withNonce(nonce: Nonce): this;
    /**
     * Set an explicit prepare_root (overrides computation)
     */
    withPrepareRoot(prepareRoot: Hash): this;
    /**
     * Validate the current builder state
     */
    validate(): string[];
    /**
     * Check if builder is in a valid state for building
     */
    isValid(): boolean;
    /**
     * Build the ComitInput for submission
     *
     * @throws ValidationError if builder state is invalid
     */
    build(): ComitInput;
    /**
     * Build the complete Comit structure (requires origin and nonce)
     *
     * @throws ValidationError if origin or nonce not set
     */
    buildComit(): Comit;
    /**
     * Reset the builder to initial state
     */
    reset(): this;
    /**
     * Clone the builder with current state
     */
    clone(): ComitBuilder;
    private encodeEvmPayload;
    private encodeSvmPayload;
    private validatePayloadSize;
    private canAutoCalculateFee;
}
/**
 * Create a new ComitBuilder
 */
export declare function comit(): ComitBuilder;
/**
 * Create a ComitBuilder with EVM-only payload
 */
export declare function evmComit(payload: HexString | Uint8Array | EvmPayloadOptions): ComitBuilder;
/**
 * Create a ComitBuilder with SVM-only payload
 */
export declare function svmComit(payload: HexString | Uint8Array | SvmPayloadOptions): ComitBuilder;
/**
 * Create a ComitBuilder with both EVM and SVM payloads
 */
export declare function dualComit(evmPayload: HexString | Uint8Array | EvmPayloadOptions, svmPayload: HexString | Uint8Array | SvmPayloadOptions): ComitBuilder;
//# sourceMappingURL=comit.d.ts.map