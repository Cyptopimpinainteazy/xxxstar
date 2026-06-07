"use strict";
/**
 * ComitBuilder - Fluent builder for constructing Comit transactions
 *
 * Provides a type-safe, chainable API for building cross-VM transactions.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.ComitBuilder = void 0;
exports.comit = comit;
exports.evmComit = evmComit;
exports.svmComit = svmComit;
exports.dualComit = dualComit;
const errors_1 = require("./errors");
const constants_1 = require("./constants");
const utils_1 = require("./utils");
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
class ComitBuilder {
    state = {};
    // ===========================================================================
    // EVM Payload Methods
    // ===========================================================================
    /**
     * Set the EVM payload
     */
    withEvmPayload(payload) {
        if (payload instanceof Uint8Array) {
            this.state.evmPayload = payload;
        }
        else if (typeof payload === 'string') {
            this.state.evmPayload = (0, utils_1.toBytes)(payload);
        }
        else {
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
    evmCall(to, data, value = 0n) {
        return this.withEvmPayload({ to, data, value });
    }
    /**
     * Set EVM contract deployment
     */
    evmDeploy(bytecode, constructorArgs) {
        const code = (0, utils_1.toBytes)(bytecode);
        const payload = constructorArgs
            ? new Uint8Array([...code, ...constructorArgs])
            : code;
        return this.withEvmPayload(payload);
    }
    /**
     * Set EVM gas limit
     */
    withEvmGasLimit(gasLimit) {
        if (gasLimit <= 0n) {
            throw new errors_1.ValidationError('gasLimit', 'Gas limit must be positive', gasLimit);
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
    withSvmPayload(payload) {
        if (payload instanceof Uint8Array) {
            this.state.svmPayload = payload;
        }
        else if (typeof payload === 'string') {
            this.state.svmPayload = (0, utils_1.toBytes)(payload);
        }
        else {
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
    svmCall(programId, data) {
        return this.withSvmPayload({ programId, data });
    }
    /**
     * Set SVM compute units
     */
    withSvmComputeUnits(computeUnits) {
        if (computeUnits <= 0n) {
            throw new errors_1.ValidationError('computeUnits', 'Compute units must be positive', computeUnits);
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
    withFee(fee) {
        if (fee === 'auto') {
            this.state.fee = this.calculateFee();
        }
        else {
            if (fee < 0n) {
                throw new errors_1.ValidationError('fee', 'Fee cannot be negative', fee);
            }
            this.state.fee = fee;
        }
        return this;
    }
    /**
     * Calculate the fee based on payloads and gas/compute units
     */
    calculateFee() {
        let fee = constants_1.BASE_COMIT_FEE;
        // EVM gas fee
        const evmGas = this.state.evmGas ?? constants_1.DEFAULT_EVM_GAS_LIMIT;
        fee += (evmGas * constants_1.GAS_PRICE) / constants_1.GAS_FEE_DIVISOR;
        // SVM compute fee
        const svmCompute = this.state.svmCompute ?? constants_1.DEFAULT_SVM_COMPUTE_UNITS;
        fee += (svmCompute * constants_1.COMPUTE_UNIT_PRICE) / constants_1.COMPUTE_FEE_DIVISOR;
        return fee;
    }
    // ===========================================================================
    // Identity Methods
    // ===========================================================================
    /**
     * Set the origin account (for prepare_root computation)
     */
    withOrigin(origin) {
        this.state.origin = origin;
        return this;
    }
    /**
     * Set the nonce (for prepare_root computation)
     */
    withNonce(nonce) {
        if (nonce < 0n) {
            throw new errors_1.ValidationError('nonce', 'Nonce cannot be negative', nonce);
        }
        this.state.nonce = nonce;
        return this;
    }
    /**
     * Set an explicit prepare_root (overrides computation)
     */
    withPrepareRoot(prepareRoot) {
        this.state.prepareRoot = prepareRoot;
        return this;
    }
    // ===========================================================================
    // Build Methods
    // ===========================================================================
    /**
     * Validate the current builder state
     */
    validate() {
        const errors = [];
        // Must have at least one payload
        if (!this.state.evmPayload?.length && !this.state.svmPayload?.length) {
            errors.push('At least one payload (EVM or SVM) must be provided');
        }
        // Validate combined size
        const evmSize = this.state.evmPayload?.length ?? 0;
        const svmSize = this.state.svmPayload?.length ?? 0;
        if (evmSize + svmSize > constants_1.MAX_COMBINED_PAYLOAD_SIZE) {
            errors.push(`Combined payload size ${evmSize + svmSize} exceeds maximum ${constants_1.MAX_COMBINED_PAYLOAD_SIZE}`);
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
    isValid() {
        return this.validate().length === 0;
    }
    /**
     * Build the ComitInput for submission
     *
     * @throws ValidationError if builder state is invalid
     */
    build() {
        const errors = this.validate();
        if (errors.length > 0) {
            throw new errors_1.ValidationError('ComitBuilder', errors.join('; '));
        }
        const evmPayload = this.state.evmPayload ?? new Uint8Array(0);
        const svmPayload = this.state.svmPayload ?? new Uint8Array(0);
        const fee = this.state.fee ?? this.calculateFee();
        // Compute prepare_root if origin and nonce are set
        let prepareRoot = this.state.prepareRoot;
        if (!prepareRoot && this.state.origin && this.state.nonce !== undefined) {
            prepareRoot = (0, utils_1.computePrepareRoot)(this.state.origin, evmPayload, svmPayload, this.state.nonce, fee);
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
    buildComit() {
        if (!this.state.origin) {
            throw new errors_1.ValidationError('origin', 'Origin must be set to build complete Comit');
        }
        if (this.state.nonce === undefined) {
            throw new errors_1.ValidationError('nonce', 'Nonce must be set to build complete Comit');
        }
        const errors = this.validate();
        if (errors.length > 0) {
            throw new errors_1.ValidationError('ComitBuilder', errors.join('; '));
        }
        const evmPayload = this.state.evmPayload ?? new Uint8Array(0);
        const svmPayload = this.state.svmPayload ?? new Uint8Array(0);
        const fee = this.state.fee ?? this.calculateFee();
        const prepareRoot = this.state.prepareRoot ?? (0, utils_1.computePrepareRoot)(this.state.origin, evmPayload, svmPayload, this.state.nonce, fee);
        const comitId = (0, utils_1.computeComitId)(prepareRoot);
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
    reset() {
        this.state = {};
        return this;
    }
    /**
     * Clone the builder with current state
     */
    clone() {
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
    encodeEvmPayload(options) {
        // Simple encoding: to (20 bytes) + value (32 bytes) + data
        // In production, use RLP encoding or ethers.js
        const parts = [];
        if (options.to) {
            parts.push((0, utils_1.toBytes)(options.to));
        }
        else {
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
            parts.push((0, utils_1.toBytes)(options.data));
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
    encodeSvmPayload(options) {
        // Simple encoding: programId (32 bytes) + data
        const parts = [];
        if (options.programId) {
            const programIdBytes = (0, utils_1.toBytes)(options.programId);
            // Pad to 32 bytes if needed
            if (programIdBytes.length < 32) {
                const padded = new Uint8Array(32);
                padded.set(programIdBytes);
                parts.push(padded);
            }
            else {
                parts.push(programIdBytes.slice(0, 32));
            }
        }
        else {
            parts.push(new Uint8Array(32)); // Zero program ID
        }
        if (options.data) {
            parts.push((0, utils_1.toBytes)(options.data));
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
    validatePayloadSize(type, payload) {
        const maxSize = type === 'evm' ? constants_1.MAX_EVM_PAYLOAD_SIZE : constants_1.MAX_SVM_PAYLOAD_SIZE;
        if (payload.length > maxSize) {
            throw new errors_1.PayloadSizeError(type, payload.length, maxSize);
        }
    }
    canAutoCalculateFee() {
        // Can always auto-calculate with defaults
        return true;
    }
}
exports.ComitBuilder = ComitBuilder;
// =============================================================================
// Factory Functions
// =============================================================================
/**
 * Create a new ComitBuilder
 */
function comit() {
    return new ComitBuilder();
}
/**
 * Create a ComitBuilder with EVM-only payload
 */
function evmComit(payload) {
    return new ComitBuilder().withEvmPayload(payload);
}
/**
 * Create a ComitBuilder with SVM-only payload
 */
function svmComit(payload) {
    return new ComitBuilder().withSvmPayload(payload);
}
/**
 * Create a ComitBuilder with both EVM and SVM payloads
 */
function dualComit(evmPayload, svmPayload) {
    return new ComitBuilder()
        .withEvmPayload(evmPayload)
        .withSvmPayload(svmPayload);
}
//# sourceMappingURL=comit.js.map