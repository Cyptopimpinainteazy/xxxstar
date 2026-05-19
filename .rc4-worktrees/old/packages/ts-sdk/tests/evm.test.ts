/**
 * Tests for EVM utilities
 */

import {
  // Address utilities
  isValidAddress,
  normalizeAddress,
  addressToAccountId,
  accountIdToAddress,
  
  // ABI encoding
  encodeUint256,
  decodeUint256,
  encodeAddress,
  decodeAddress,
  encodeBool,
  decodeBool,
  
  // Function encoding
  functionSelector,
  encodeFunctionCall,
  decodeFunctionCall,
  
  // Common function encoders
  encodeTransfer,
  encodeApprove,
  encodeBalanceOf,
  
  // Error decoding
  isErrorRevert,
  isPanicRevert,
  getPanicMessage,
  
  // Errors
  ValidationError,
} from '../src';

import type { HexString } from '@polkadot/util/types';

describe('EVM address utilities', () => {
  const validAddress = ('0x' + '12'.repeat(20)) as HexString;
  const shortAddress = '0x' + '12'.repeat(19);
  const longAddress = '0x' + '12'.repeat(21);

  describe('isValidAddress', () => {
    it('should accept valid address', () => {
      expect(isValidAddress(validAddress)).toBe(true);
    });

    it('should reject short address', () => {
      expect(isValidAddress(shortAddress)).toBe(false);
    });

    it('should reject long address', () => {
      expect(isValidAddress(longAddress)).toBe(false);
    });

    it('should reject non-hex', () => {
      expect(isValidAddress('not-an-address')).toBe(false);
    });
  });

  describe('normalizeAddress', () => {
    it('should lowercase address', () => {
      const upper = '0x' + 'AB'.repeat(20);
      expect(normalizeAddress(upper)).toBe('0x' + 'ab'.repeat(20));
    });

    it('should throw on invalid address', () => {
      expect(() => normalizeAddress('invalid')).toThrow(ValidationError);
    });
  });

  describe('addressToAccountId', () => {
    it('should convert to 32-byte account', () => {
      const accountId = addressToAccountId(validAddress);
      expect(accountId.length).toBe(66); // 0x + 64 chars
    });
  });

  describe('accountIdToAddress', () => {
    it('should take first 20 bytes', () => {
      const accountId = '0x' + '12'.repeat(32);
      const address = accountIdToAddress(accountId);
      expect(address).toBe('0x' + '12'.repeat(20));
    });
  });
});

describe('ABI encoding', () => {
  describe('encodeUint256', () => {
    it('should encode zero', () => {
      const encoded = encodeUint256(0n);
      expect(encoded.length).toBe(32);
      expect(encoded.every(b => b === 0)).toBe(true);
    });

    it('should encode 1', () => {
      const encoded = encodeUint256(1n);
      expect(encoded[31]).toBe(1);
      expect(encoded.slice(0, 31).every(b => b === 0)).toBe(true);
    });

    it('should encode large number', () => {
      const value = BigInt('0x' + 'ff'.repeat(32));
      const encoded = encodeUint256(value);
      expect(encoded.every(b => b === 0xff)).toBe(true);
    });

    it('should reject negative', () => {
      expect(() => encodeUint256(-1n)).toThrow(ValidationError);
    });

    it('should reject overflow', () => {
      expect(() => encodeUint256(2n ** 256n)).toThrow(ValidationError);
    });
  });

  describe('decodeUint256', () => {
    it('should decode zero', () => {
      expect(decodeUint256(new Uint8Array(32))).toBe(0n);
    });

    it('should round-trip', () => {
      const original = 12345678901234567890n;
      const decoded = decodeUint256(encodeUint256(original));
      expect(decoded).toBe(original);
    });

    it('should reject wrong length', () => {
      expect(() => decodeUint256(new Uint8Array(16))).toThrow(ValidationError);
    });
  });

  describe('encodeAddress', () => {
    it('should right-align in 32 bytes', () => {
      const address = ('0x' + '12'.repeat(20)) as HexString;
      const encoded = encodeAddress(address);
      
      expect(encoded.length).toBe(32);
      expect(encoded.slice(0, 12).every(b => b === 0)).toBe(true);
      expect(encoded.slice(12).every(b => b === 0x12)).toBe(true);
    });
  });

  describe('decodeAddress', () => {
    it('should round-trip', () => {
      const original = ('0x' + '12'.repeat(20)) as HexString;
      const decoded = decodeAddress(encodeAddress(original));
      expect(decoded).toBe(original);
    });
  });

  describe('encodeBool/decodeBool', () => {
    it('should encode true', () => {
      const encoded = encodeBool(true);
      expect(encoded[31]).toBe(1);
    });

    it('should encode false', () => {
      const encoded = encodeBool(false);
      expect(encoded.every(b => b === 0)).toBe(true);
    });

    it('should round-trip', () => {
      expect(decodeBool(encodeBool(true))).toBe(true);
      expect(decodeBool(encodeBool(false))).toBe(false);
    });
  });
});

describe('function encoding', () => {
  describe('functionSelector', () => {
    it('should return 4-byte selector', () => {
      const selector = functionSelector('transfer(address,uint256)');
      expect(selector).toMatch(/^0x[0-9a-f]{8}$/);
    });

    it('should be consistent', () => {
      const s1 = functionSelector('foo()');
      const s2 = functionSelector('foo()');
      expect(s1).toBe(s2);
    });

    it('should differ for different signatures', () => {
      const s1 = functionSelector('foo()');
      const s2 = functionSelector('bar()');
      expect(s1).not.toBe(s2);
    });
  });

  describe('encodeFunctionCall', () => {
    it('should include selector and params', () => {
      const call = encodeFunctionCall(
        'transfer(address,uint256)',
        [encodeAddress(('0x' + '12'.repeat(20)) as HexString), encodeUint256(100n)]
      );

      expect(call.length).toBe(4 + 32 + 32); // selector + 2 params
    });
  });

  describe('decodeFunctionCall', () => {
    it('should extract selector and params', () => {
      const call = encodeFunctionCall(
        'transfer(address,uint256)',
        [encodeAddress(('0x' + '12'.repeat(20)) as HexString), encodeUint256(100n)]
      );

      const decoded = decodeFunctionCall(call);
      expect(decoded.selector.length).toBe(10); // 0x + 8 chars
      expect(decoded.params.length).toBe(64); // 2 x 32 bytes
    });

    it('should reject short data', () => {
      expect(() => decodeFunctionCall(new Uint8Array(3))).toThrow(ValidationError);
    });
  });
});

describe('common function encoders', () => {
  describe('encodeTransfer', () => {
    it('should encode ERC20 transfer', () => {
      const call = encodeTransfer(('0x' + '12'.repeat(20)) as HexString, 1000n);
      expect(call.length).toBe(4 + 32 + 32);
    });
  });

  describe('encodeApprove', () => {
    it('should encode ERC20 approve', () => {
      const call = encodeApprove(('0x' + '12'.repeat(20)) as HexString, 1000n);
      expect(call.length).toBe(4 + 32 + 32);
    });
  });

  describe('encodeBalanceOf', () => {
    it('should encode balanceOf', () => {
      const call = encodeBalanceOf(('0x' + '12'.repeat(20)) as HexString);
      expect(call.length).toBe(4 + 32);
    });
  });
});

describe('error decoding', () => {
  describe('isErrorRevert', () => {
    it('should detect Error(string)', () => {
      const data = new Uint8Array([0x08, 0xc3, 0x79, 0xa0, ...new Array(60).fill(0)]);
      expect(isErrorRevert(data)).toBe(true);
    });

    it('should reject other selectors', () => {
      const data = new Uint8Array([0x00, 0x00, 0x00, 0x00]);
      expect(isErrorRevert(data)).toBe(false);
    });

    it('should reject short data', () => {
      const data = new Uint8Array([0x08, 0xc3]);
      expect(isErrorRevert(data)).toBe(false);
    });
  });

  describe('isPanicRevert', () => {
    it('should detect Panic(uint256)', () => {
      const data = new Uint8Array([0x4e, 0x48, 0x7b, 0x71, ...new Array(32).fill(0)]);
      expect(isPanicRevert(data)).toBe(true);
    });
  });

  describe('getPanicMessage', () => {
    it('should return known panic messages', () => {
      expect(getPanicMessage(1n)).toBe('Assert failed');
      expect(getPanicMessage(17n)).toBe('Arithmetic overflow/underflow');
      expect(getPanicMessage(18n)).toBe('Division or modulo by zero');
    });

    it('should handle unknown codes', () => {
      expect(getPanicMessage(999n)).toContain('Unknown');
    });
  });
});
