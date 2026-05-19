/**
 * Tests for SVM utilities
 */

import type { HexString } from '@polkadot/util/types';
import {
  // Pubkey utilities
  isValidPubkey,
  pubkeyToBytes,
  bytesToPubkey,
  zeroPubkey,
  
  // Encoding
  encodeCompactU16,
  decodeCompactU16,
  encodeU8,
  encodeU16,
  encodeU32,
  encodeU64,
  decodeU64,
  encodeSvmString,
  encodeVec,
  encodeOption,
  
  // Common programs
  SYSTEM_PROGRAM_ID,
  encodeSystemTransfer,
  encodeTokenTransfer,
  createTransferAccounts,
  
  // Anchor
  anchorDiscriminator,
  anchorAccountDiscriminator,
  
  // Errors
  ValidationError,
} from '../src';

describe('SVM pubkey utilities', () => {
  describe('isValidPubkey', () => {
    it('should accept valid hex pubkey', () => {
      const pubkey = '0x' + '12'.repeat(32);
      expect(isValidPubkey(pubkey)).toBe(true);
    });

    it('should reject short pubkey', () => {
      const pubkey = '0x' + '12'.repeat(31);
      expect(isValidPubkey(pubkey)).toBe(false);
    });

    it('should accept base58-like string', () => {
      const pubkey = '11111111111111111111111111111111';
      expect(isValidPubkey(pubkey)).toBe(true);
    });
  });

  describe('pubkeyToBytes', () => {
    it('should convert hex to bytes', () => {
      const pubkey = ('0x' + '12'.repeat(32)) as HexString;
      const bytes = pubkeyToBytes(pubkey);
      expect(bytes.length).toBe(32);
      expect(bytes.every(b => b === 0x12)).toBe(true);
    });

    it('should reject wrong length', () => {
      expect(() => pubkeyToBytes(('0x' + '12'.repeat(16)) as HexString)).toThrow(ValidationError);
    });
  });

  describe('bytesToPubkey', () => {
    it('should convert bytes to hex', () => {
      const bytes = new Uint8Array(32).fill(0x12);
      const pubkey = bytesToPubkey(bytes);
      expect(pubkey).toBe('0x' + '12'.repeat(32));
    });

    it('should reject wrong length', () => {
      expect(() => bytesToPubkey(new Uint8Array(16))).toThrow(ValidationError);
    });
  });

  describe('zeroPubkey', () => {
    it('should return 32 zero bytes', () => {
      const pubkey = zeroPubkey();
      const bytes = pubkeyToBytes(pubkey);
      expect(bytes.every(b => b === 0)).toBe(true);
    });
  });
});

describe('compact u16 encoding', () => {
  describe('encodeCompactU16', () => {
    it('should encode small values in 1 byte', () => {
      expect(encodeCompactU16(0).length).toBe(1);
      expect(encodeCompactU16(127).length).toBe(1);
    });

    it('should encode medium values in 2 bytes', () => {
      expect(encodeCompactU16(128).length).toBe(2);
      expect(encodeCompactU16(16383).length).toBe(2);
    });

    it('should encode large values in 3 bytes', () => {
      expect(encodeCompactU16(16384).length).toBe(3);
      expect(encodeCompactU16(65535).length).toBe(3);
    });

    it('should reject out of range', () => {
      expect(() => encodeCompactU16(-1)).toThrow(ValidationError);
      expect(() => encodeCompactU16(65536)).toThrow(ValidationError);
    });
  });

  describe('decodeCompactU16', () => {
    it('should round-trip', () => {
      for (const value of [0, 1, 127, 128, 16383, 16384, 65535]) {
        const encoded = encodeCompactU16(value);
        const decoded = decodeCompactU16(encoded);
        expect(decoded.value).toBe(value);
        expect(decoded.bytes.length).toBe(encoded.length);
      }
    });
  });
});

describe('data type encoding', () => {
  describe('encodeU8', () => {
    it('should encode valid u8', () => {
      expect(encodeU8(0)).toEqual(new Uint8Array([0]));
      expect(encodeU8(255)).toEqual(new Uint8Array([255]));
    });

    it('should reject out of range', () => {
      expect(() => encodeU8(-1)).toThrow(ValidationError);
      expect(() => encodeU8(256)).toThrow(ValidationError);
    });
  });

  describe('encodeU16', () => {
    it('should encode little-endian', () => {
      expect(encodeU16(0x0102)).toEqual(new Uint8Array([0x02, 0x01]));
    });
  });

  describe('encodeU32', () => {
    it('should encode little-endian', () => {
      expect(encodeU32(0x01020304)).toEqual(new Uint8Array([0x04, 0x03, 0x02, 0x01]));
    });
  });

  describe('encodeU64', () => {
    it('should encode little-endian', () => {
      const encoded = encodeU64(0x0102030405060708n);
      expect(encoded.length).toBe(8);
      expect(encoded[0]).toBe(0x08);
      expect(encoded[7]).toBe(0x01);
    });

    it('should reject out of range', () => {
      expect(() => encodeU64(-1n)).toThrow(ValidationError);
      expect(() => encodeU64(2n ** 64n)).toThrow(ValidationError);
    });
  });

  describe('decodeU64', () => {
    it('should round-trip', () => {
      const original = 12345678901234567890n;
      const decoded = decodeU64(encodeU64(original));
      expect(decoded).toBe(original);
    });

    it('should reject wrong length', () => {
      expect(() => decodeU64(new Uint8Array(4))).toThrow(ValidationError);
    });
  });

  describe('encodeSvmString', () => {
    it('should include length prefix', () => {
      const encoded = encodeSvmString('hello');
      // 4-byte length + 5-byte string
      expect(encoded.length).toBe(4 + 5);
      expect(encoded[0]).toBe(5);
    });

    it('should handle empty string', () => {
      const encoded = encodeSvmString('');
      expect(encoded.length).toBe(4);
      expect(encoded[0]).toBe(0);
    });
  });

  describe('encodeVec', () => {
    it('should include length prefix', () => {
      const encoded = encodeVec([1, 2, 3], encodeU8);
      expect(encoded.length).toBe(4 + 3);
      expect(encoded[0]).toBe(3);
    });

    it('should handle empty vec', () => {
      const encoded = encodeVec([], encodeU8);
      expect(encoded.length).toBe(4);
      expect(encoded[0]).toBe(0);
    });
  });

  describe('encodeOption', () => {
    it('should encode None as single 0 byte', () => {
      const encoded = encodeOption(null, encodeU8);
      expect(encoded).toEqual(new Uint8Array([0]));
    });

    it('should encode Some with 1 byte prefix', () => {
      const encoded = encodeOption(42, encodeU8);
      expect(encoded).toEqual(new Uint8Array([1, 42]));
    });
  });
});

describe('common programs', () => {
  describe('SYSTEM_PROGRAM_ID', () => {
    it('should be zero pubkey', () => {
      expect(SYSTEM_PROGRAM_ID).toBe(zeroPubkey());
    });
  });

  describe('encodeSystemTransfer', () => {
    it('should encode transfer instruction', () => {
      const encoded = encodeSystemTransfer(1000000000n);
      // u32 discriminator (4) + u64 amount (8)
      expect(encoded.length).toBe(4 + 8);
    });
  });

  describe('encodeTokenTransfer', () => {
    it('should encode SPL token transfer', () => {
      const encoded = encodeTokenTransfer(100n);
      // u8 discriminator (1) + u64 amount (8)
      expect(encoded.length).toBe(1 + 8);
    });
  });

  describe('createTransferAccounts', () => {
    it('should create correct account metas', () => {
      const from = ('0x' + '11'.repeat(32)) as HexString;
      const to = ('0x' + '22'.repeat(32)) as HexString;

      const accounts = createTransferAccounts(from, to);

      expect(accounts.length).toBe(2);

      expect(accounts[0].pubkey).toBe(from);
      expect(accounts[0].isSigner).toBe(true);
      expect(accounts[0].isWritable).toBe(true);

      expect(accounts[1].pubkey).toBe(to);
      expect(accounts[1].isSigner).toBe(false);
      expect(accounts[1].isWritable).toBe(true);
    });
  });
});

describe('anchor support', () => {
  describe('anchorDiscriminator', () => {
    it('should return 8 bytes', () => {
      const disc = anchorDiscriminator('initialize');
      expect(disc.length).toBe(8);
    });

    it('should be consistent', () => {
      const d1 = anchorDiscriminator('foo');
      const d2 = anchorDiscriminator('foo');
      expect(d1).toEqual(d2);
    });

    it('should differ for different names', () => {
      const d1 = anchorDiscriminator('foo');
      const d2 = anchorDiscriminator('bar');
      expect(d1).not.toEqual(d2);
    });
  });

  describe('anchorAccountDiscriminator', () => {
    it('should return 8 bytes', () => {
      const disc = anchorAccountDiscriminator('MyAccount');
      expect(disc.length).toBe(8);
    });

    it('should differ from instruction discriminator', () => {
      const instDisc = anchorDiscriminator('initialize');
      const acctDisc = anchorAccountDiscriminator('initialize');
      expect(instDisc).not.toEqual(acctDisc);
    });
  });
});
