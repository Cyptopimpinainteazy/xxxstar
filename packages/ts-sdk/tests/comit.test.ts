/**
 * Tests for ComitBuilder
 */

import type { HexString } from '@polkadot/util/types';
import {
  ComitBuilder,
  comit,
  evmComit,
  svmComit,
  dualComit,
  ValidationError,
  PayloadSizeError,
  MAX_EVM_PAYLOAD_SIZE,
} from '../src';

describe('ComitBuilder', () => {
  describe('construction', () => {
    it('should create empty builder', () => {
      const builder = new ComitBuilder();
      expect(builder.isValid()).toBe(false);
    });

    it('should create with factory function', () => {
      const builder = comit();
      expect(builder).toBeInstanceOf(ComitBuilder);
    });
  });

  describe('EVM payload', () => {
    it('should accept hex string payload', () => {
      const builder = new ComitBuilder()
        .withEvmPayload('0x1234')
        .withFee(1000n);

      expect(builder.isValid()).toBe(true);
    });

    it('should accept Uint8Array payload', () => {
      const builder = new ComitBuilder()
        .withEvmPayload(new Uint8Array([0x12, 0x34]))
        .withFee(1000n);

      expect(builder.isValid()).toBe(true);
    });

    it('should accept options object', () => {
      const builder = new ComitBuilder()
        .withEvmPayload({
          to: ('0x' + '11'.repeat(20)) as HexString,
          data: '0xabcd',
          value: 100n,
        })
        .withFee(1000n);

      expect(builder.isValid()).toBe(true);
    });

    it('should reject payload exceeding size limit', () => {
      const largePayload = new Uint8Array(MAX_EVM_PAYLOAD_SIZE + 1);

      expect(() => {
        new ComitBuilder().withEvmPayload(largePayload);
      }).toThrow(PayloadSizeError);
    });

    it('should set gas limit', () => {
      const builder = new ComitBuilder()
        .withEvmPayload('0x1234')
        .withEvmGasLimit(500000n)
        .withFee('auto');

      const input = builder.build();
      expect(input.fee).toBeGreaterThan(0n);
    });
  });

  describe('SVM payload', () => {
    it('should accept hex string payload', () => {
      const builder = new ComitBuilder()
        .withSvmPayload('0x5678')
        .withFee(1000n);

      expect(builder.isValid()).toBe(true);
    });

    it('should accept options object', () => {
      const builder = new ComitBuilder()
        .withSvmPayload({
          programId: ('0x' + '22'.repeat(32)) as HexString,
          data: '0xef01',
        })
        .withFee(1000n);

      expect(builder.isValid()).toBe(true);
    });

    it('should set compute units', () => {
      const builder = new ComitBuilder()
        .withSvmPayload('0x5678')
        .withSvmComputeUnits(100000n)
        .withFee('auto');

      const input = builder.build();
      expect(input.fee).toBeGreaterThan(0n);
    });
  });

  describe('dual-VM', () => {
    it('should accept both EVM and SVM payloads', () => {
      const builder = new ComitBuilder()
        .withEvmPayload('0x1234')
        .withSvmPayload('0x5678')
        .withFee(2000n);

      expect(builder.isValid()).toBe(true);

      const input = builder.build();
      expect(input.evmPayload).toBeDefined();
      expect(input.svmPayload).toBeDefined();
    });

    it('should work with dualComit factory', () => {
      const builder = dualComit('0x1234', '0x5678');

      expect(builder.isValid()).toBe(true);
    });
  });

  describe('fee handling', () => {
    it('should accept explicit fee', () => {
      const builder = new ComitBuilder()
        .withEvmPayload('0x1234')
        .withFee(5000n);

      const input = builder.build();
      expect(input.fee).toBe(5000n);
    });

    it('should calculate auto fee', () => {
      const builder = new ComitBuilder()
        .withEvmPayload('0x1234')
        .withFee('auto');

      const input = builder.build();
      expect(input.fee).toBeGreaterThan(0n);
    });

    it('should reject negative fee', () => {
      expect(() => {
        new ComitBuilder()
          .withEvmPayload('0x1234')
          .withFee(-100n);
      }).toThrow(ValidationError);
    });
  });

  describe('validation', () => {
    it('should require at least one payload', () => {
      const builder = new ComitBuilder().withFee(1000n);

      const errors = builder.validate();
      expect(errors.length).toBeGreaterThan(0);
      expect(errors[0]).toContain('payload');
    });

    it('should validate on build', () => {
      const builder = new ComitBuilder();

      expect(() => builder.build()).toThrow(ValidationError);
    });
  });

  describe('cloning', () => {
    it('should clone builder state', () => {
      const original = new ComitBuilder()
        .withEvmPayload('0x1234')
        .withFee(1000n);

      const clone = original.clone();

      // Modify clone
      clone.withSvmPayload('0x5678');

      // Original should be unchanged
      const originalInput = original.build();
      const cloneInput = clone.build();

      expect(originalInput.svmPayload?.length).toBe(0);
      expect(cloneInput.svmPayload?.length).toBeGreaterThan(0);
    });
  });

  describe('reset', () => {
    it('should reset builder to initial state', () => {
      const builder = new ComitBuilder()
        .withEvmPayload('0x1234')
        .withFee(1000n);

      expect(builder.isValid()).toBe(true);

      builder.reset();

      expect(builder.isValid()).toBe(false);
    });
  });
});

describe('factory functions', () => {
  it('evmComit should create EVM-only builder', () => {
    const builder = evmComit('0x1234');
    expect(builder.isValid()).toBe(true);
  });

  it('svmComit should create SVM-only builder', () => {
    const builder = svmComit('0x5678');
    expect(builder.isValid()).toBe(true);
  });

  it('dualComit should create dual-VM builder', () => {
    const builder = dualComit('0x1234', '0x5678');
    expect(builder.isValid()).toBe(true);
  });
});
