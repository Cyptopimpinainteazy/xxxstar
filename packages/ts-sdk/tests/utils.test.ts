/**
 * Tests for utility functions
 */

import {
  // Encoding utilities
  hexToBytes,
  bytesToHex,
  toBytes,
  toHex,
  
  // Hashing
  blake2_256,
  
  // Number encoding
  encodeU128,
  decodeU128,
  
  // Address utilities
  accountIdToEvmAddress,
  isValidEvmAddress,
  isValidH256,
  
  // Validation
  validatePayloadSizes,
  validateBalance,
  validateNonce,
  
  // Format utilities
  formatBalance,
  parseBalance,
  truncateHash,
  
  // Async utilities
  sleep,
  retry,
  
  // Errors
  PayloadSizeError,
  ValidationError,
  MAX_EVM_PAYLOAD_SIZE,
  MAX_SVM_PAYLOAD_SIZE,
} from '../src';

describe('encoding utilities', () => {
  describe('hexToBytes', () => {
    it('should convert hex string to bytes', () => {
      const bytes = hexToBytes('0x1234');
      expect(bytes).toEqual(new Uint8Array([0x12, 0x34]));
    });

    it('should handle empty hex', () => {
      const bytes = hexToBytes('0x');
      expect(bytes).toEqual(new Uint8Array([]));
    });
  });

  describe('bytesToHex', () => {
    it('should convert bytes to hex string', () => {
      const hex = bytesToHex(new Uint8Array([0x12, 0x34]));
      expect(hex).toBe('0x1234');
    });

    it('should handle empty bytes', () => {
      const hex = bytesToHex(new Uint8Array([]));
      expect(hex).toBe('0x');
    });
  });

  describe('toBytes', () => {
    it('should pass through Uint8Array', () => {
      const input = new Uint8Array([1, 2, 3]);
      const result = toBytes(input);
      expect(result).toBe(input);
    });

    it('should convert hex string', () => {
      const result = toBytes('0x010203');
      expect(result).toEqual(new Uint8Array([1, 2, 3]));
    });
  });

  describe('toHex', () => {
    it('should convert Uint8Array to hex', () => {
      const result = toHex(new Uint8Array([1, 2, 3]));
      expect(result).toBe('0x010203');
    });

    it('should pass through hex string', () => {
      const result = toHex('0x010203');
      expect(result).toBe('0x010203');
    });
  });
});

describe('number encoding', () => {
  describe('encodeU128', () => {
    it('should encode zero', () => {
      const encoded = encodeU128(0n);
      expect(encoded).toEqual(new Uint8Array(16));
    });

    it('should encode small number', () => {
      const encoded = encodeU128(256n);
      expect(encoded[0]).toBe(0);
      expect(encoded[1]).toBe(1);
    });

    it('should encode large number', () => {
      const encoded = encodeU128(BigInt('0xffffffffffffffffffffffffffff'));
      expect(encoded.length).toBe(16);
    });
  });

  describe('decodeU128', () => {
    it('should decode zero', () => {
      const decoded = decodeU128(new Uint8Array(16));
      expect(decoded).toBe(0n);
    });

    it('should round-trip', () => {
      const original = 12345678901234567890n;
      const encoded = encodeU128(original);
      const decoded = decodeU128(encoded);
      expect(decoded).toBe(original);
    });

    it('should reject wrong length', () => {
      expect(() => decodeU128(new Uint8Array(8))).toThrow(ValidationError);
    });
  });
});

describe('address utilities', () => {
  describe('isValidEvmAddress', () => {
    it('should accept valid address', () => {
      expect(isValidEvmAddress('0x' + '12'.repeat(20))).toBe(true);
    });

    it('should reject short address', () => {
      expect(isValidEvmAddress('0x' + '12'.repeat(19))).toBe(false);
    });

    it('should reject long address', () => {
      expect(isValidEvmAddress('0x' + '12'.repeat(21))).toBe(false);
    });

    it('should reject non-hex', () => {
      expect(isValidEvmAddress('not-hex')).toBe(false);
    });
  });

  describe('accountIdToEvmAddress', () => {
    it('should take first 20 bytes', () => {
      const accountId = '0x' + '11'.repeat(32);
      const address = accountIdToEvmAddress(accountId);
      expect(address).toBe('0x' + '11'.repeat(20));
    });
  });
});

describe('hash utilities', () => {
  describe('isValidH256', () => {
    it('should accept valid hash', () => {
      expect(isValidH256('0x' + '00'.repeat(32))).toBe(true);
    });

    it('should reject short hash', () => {
      expect(isValidH256('0x' + '00'.repeat(31))).toBe(false);
    });

    it('should reject non-hex', () => {
      expect(isValidH256('not-a-hash')).toBe(false);
    });
  });

  describe('blake2_256', () => {
    it('should hash bytes', () => {
      const hash = blake2_256(new Uint8Array([1, 2, 3]));
      expect(hash).toMatch(/^0x[0-9a-f]{64}$/);
    });

    it('should hash string', () => {
      const hash = blake2_256('hello');
      expect(hash).toMatch(/^0x[0-9a-f]{64}$/);
    });

    it('should produce consistent results', () => {
      const hash1 = blake2_256('test');
      const hash2 = blake2_256('test');
      expect(hash1).toBe(hash2);
    });
  });
});

describe('validation', () => {
  describe('validatePayloadSizes', () => {
    it('should accept valid sizes', () => {
      expect(() => {
        validatePayloadSizes(
          new Uint8Array(1000),
          new Uint8Array(1000)
        );
      }).not.toThrow();
    });

    it('should reject oversized EVM payload', () => {
      expect(() => {
        validatePayloadSizes(
          new Uint8Array(MAX_EVM_PAYLOAD_SIZE + 1),
          new Uint8Array(0)
        );
      }).toThrow(PayloadSizeError);
    });

    it('should reject oversized SVM payload', () => {
      expect(() => {
        validatePayloadSizes(
          new Uint8Array(0),
          new Uint8Array(MAX_SVM_PAYLOAD_SIZE + 1)
        );
      }).toThrow(PayloadSizeError);
    });
  });

  describe('validateBalance', () => {
    it('should accept positive balance', () => {
      expect(() => validateBalance(100n)).not.toThrow();
    });

    it('should accept zero balance', () => {
      expect(() => validateBalance(0n)).not.toThrow();
    });

    it('should reject negative balance', () => {
      expect(() => validateBalance(-1n)).toThrow(ValidationError);
    });
  });

  describe('validateNonce', () => {
    it('should accept positive nonce', () => {
      expect(() => validateNonce(100n)).not.toThrow();
    });

    it('should accept zero nonce', () => {
      expect(() => validateNonce(0n)).not.toThrow();
    });

    it('should reject negative nonce', () => {
      expect(() => validateNonce(-1n)).toThrow(ValidationError);
    });
  });
});

describe('format utilities', () => {
  describe('formatBalance', () => {
    it('should format whole numbers', () => {
      expect(formatBalance(1000000000000000000n)).toBe('1');
    });

    it('should format with decimals', () => {
      expect(formatBalance(1500000000000000000n)).toBe('1.5');
    });

    it('should format small amounts', () => {
      expect(formatBalance(1000000000000000n)).toBe('0.001');
    });

    it('should handle zero', () => {
      expect(formatBalance(0n)).toBe('0');
    });
  });

  describe('parseBalance', () => {
    it('should parse whole numbers', () => {
      expect(parseBalance('1')).toBe(1000000000000000000n);
    });

    it('should parse decimals', () => {
      expect(parseBalance('1.5')).toBe(1500000000000000000n);
    });

    it('should round-trip', () => {
      const original = 1234567890123456789n;
      const formatted = formatBalance(original);
      const parsed = parseBalance(formatted);
      expect(parsed).toBe(original);
    });
  });

  describe('truncateHash', () => {
    it('should truncate long hash', () => {
      const hash = '0x' + '12'.repeat(32);
      const truncated = truncateHash(hash as any);
      expect(truncated).toMatch(/^0x1212\.\.\.1212$/);
    });

    it('should not truncate short hash', () => {
      const hash = '0x1234';
      const truncated = truncateHash(hash as any, 4);
      expect(truncated).toBe(hash);
    });
  });
});

describe('async utilities', () => {
  describe('sleep', () => {
    it('should delay execution', async () => {
      const start = Date.now();
      await sleep(50);
      const elapsed = Date.now() - start;
      expect(elapsed).toBeGreaterThanOrEqual(45);
    });
  });

  describe('retry', () => {
    it('should succeed on first try', async () => {
      let attempts = 0;
      const result = await retry(async () => {
        attempts++;
        return 'success';
      });

      expect(result).toBe('success');
      expect(attempts).toBe(1);
    });

    it('should retry on failure', async () => {
      let attempts = 0;
      const result = await retry(
        async () => {
          attempts++;
          if (attempts < 3) throw new Error('fail');
          return 'success';
        },
        { maxAttempts: 3, initialDelayMs: 10 }
      );

      expect(result).toBe('success');
      expect(attempts).toBe(3);
    });

    it('should give up after max attempts', async () => {
      let attempts = 0;

      await expect(
        retry(
          async () => {
            attempts++;
            throw new Error('always fail');
          },
          { maxAttempts: 2, initialDelayMs: 10 }
        )
      ).rejects.toThrow('always fail');

      expect(attempts).toBe(2);
    });
  });
});
