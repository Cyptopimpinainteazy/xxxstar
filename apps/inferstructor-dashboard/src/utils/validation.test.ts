import { describe, it, expect } from 'vitest';
import {
  validateEmail,
  validatePassword,
  validateSolanaAddress,
  validateEthereumAddress,
  validateBlockchainAddress,
  validateApiKey,
  validateTPS,
  validatePort,
  validateUrl,
  validateRpcEndpoint,
  validatePercentage,
  validateLatency,
  validateChain,
  validateSlaTier,
  validateAdminActionParams,
  validateForm,
} from '../utils/validation';

describe('Validation Utilities', () => {
  describe('validateEmail', () => {
    it('should accept valid emails', () => {
      expect(validateEmail('test@example.com').valid).toBe(true);
      expect(validateEmail('user+tag@domain.co.uk').valid).toBe(true);
    });

    it('should reject invalid emails', () => {
      expect(validateEmail('').valid).toBe(false);
      expect(validateEmail('notanemail').valid).toBe(false);
      expect(validateEmail('test@').valid).toBe(false);
    });

    it('should reject overly long emails', () => {
      const longEmail = 'a'.repeat(250) + '@example.com';
      expect(validateEmail(longEmail).valid).toBe(false);
    });
  });

  describe('validatePassword', () => {
    it('should accept strong passwords', () => {
      expect(validatePassword('StrongPass123').valid).toBe(true);
      expect(validatePassword('MySecurePassword456').valid).toBe(true);
    });

    it('should reject weak passwords', () => {
      expect(validatePassword('weak').valid).toBe(false);
      expect(validatePassword('nouppercase123').valid).toBe(false);
      expect(validatePassword('NOLOWERCASE123').valid).toBe(false);
      expect(validatePassword('NoNumbers').valid).toBe(false);
    });
  });

  describe('validateSolanaAddress', () => {
    it('should accept valid Solana addresses', () => {
      // Valid Solana address format (base58, 32-88 chars)
      expect(validateSolanaAddress('11111111111111111111111111111112').valid).toBe(true);
    });

    it('should reject invalid Solana addresses', () => {
      expect(validateSolanaAddress('').valid).toBe(false);
      expect(validateSolanaAddress('invalid-solana-address').valid).toBe(false);
      expect(validateSolanaAddress('0x' + 'a'.repeat(40)).valid).toBe(false); // Ethereum format
    });
  });

  describe('validateEthereumAddress', () => {
    it('should accept valid Ethereum addresses', () => {
      expect(validateEthereumAddress('0x' + 'a'.repeat(40)).valid).toBe(true);
      expect(validateEthereumAddress('0x' + 'f'.repeat(40)).valid).toBe(true);
    });

    it('should reject invalid Ethereum addresses', () => {
      expect(validateEthereumAddress('').valid).toBe(false);
      expect(validateEthereumAddress('0x' + 'g'.repeat(40)).valid).toBe(false); // Invalid hex
      expect(validateEthereumAddress('0x' + 'a'.repeat(39)).valid).toBe(false); // Too short
    });
  });

  describe('validateBlockchainAddress', () => {
    it('should validate addresses by chain', () => {
      expect(validateBlockchainAddress('0x' + 'a'.repeat(40), 'ethereum').valid).toBe(true);
      expect(validateBlockchainAddress('11111111111111111111111111111112', 'solana').valid).toBe(true);
    });

    it('should reject addresses with wrong chain', () => {
      expect(validateBlockchainAddress('0x' + 'a'.repeat(40), 'solana').valid).toBe(false);
    });
  });

  describe('validateApiKey', () => {
    it('should accept valid API keys', () => {
      expect(validateApiKey('a'.repeat(32)).valid).toBe(true);
      expect(validateApiKey('key_' + 'a'.repeat(32)).valid).toBe(true);
    });

    it('should reject invalid API keys', () => {
      expect(validateApiKey('').valid).toBe(false);
      expect(validateApiKey('short').valid).toBe(false);
      expect(validateApiKey('a'.repeat(300)).valid).toBe(false); // Too long
      expect(validateApiKey('invalid@key#format').valid).toBe(false); // Invalid chars
    });
  });

  describe('validateTPS', () => {
    it('should accept valid TPS values', () => {
      expect(validateTPS('100').valid).toBe(true);
      expect(validateTPS(1000).valid).toBe(true);
      expect(validateTPS('1000000').valid).toBe(true);
    });

    it('should reject invalid TPS values', () => {
      expect(validateTPS('0').valid).toBe(false);
      expect(validateTPS('-100').valid).toBe(false);
      expect(validateTPS('1000001').valid).toBe(false); // Exceeds max
    });
  });

  describe('validatePort', () => {
    it('should accept valid port numbers', () => {
      expect(validatePort('8080').valid).toBe(true);
      expect(validatePort(3000).valid).toBe(true);
      expect(validatePort('65535').valid).toBe(true);
    });

    it('should reject invalid port numbers', () => {
      expect(validatePort('0').valid).toBe(false);
      expect(validatePort('65536').valid).toBe(false);
      expect(validatePort('-1').valid).toBe(false);
    });
  });

  describe('validateUrl', () => {
    it('should accept valid URLs', () => {
      expect(validateUrl('http://localhost:8080').valid).toBe(true);
      expect(validateUrl('https://example.com').valid).toBe(true);
    });

    it('should reject invalid URLs', () => {
      expect(validateUrl('').valid).toBe(false);
      expect(validateUrl('not-a-url').valid).toBe(false);
    });
  });

  describe('validateRpcEndpoint', () => {
    it('should accept valid RPC endpoints', () => {
      expect(validateRpcEndpoint('http://localhost:8545').valid).toBe(true);
      expect(validateRpcEndpoint('https://rpc.example.com').valid).toBe(true);
    });

    it('should reject invalid RPC endpoints', () => {
      expect(validateRpcEndpoint('ftp://invalid.com').valid).toBe(false);
      expect(validateRpcEndpoint('localhost:8545').valid).toBe(false); // Missing protocol
    });
  });

  describe('validatePercentage', () => {
    it('should accept valid percentages', () => {
      expect(validatePercentage('0').valid).toBe(true);
      expect(validatePercentage('50').valid).toBe(true);
      expect(validatePercentage(100).valid).toBe(true);
    });

    it('should reject invalid percentages', () => {
      expect(validatePercentage('-1').valid).toBe(false);
      expect(validatePercentage('101').valid).toBe(false);
    });
  });

  describe('validateLatency', () => {
    it('should accept valid latency values', () => {
      expect(validateLatency('10').valid).toBe(true);
      expect(validateLatency(1000).valid).toBe(true);
      expect(validateLatency('60000').valid).toBe(true);
    });

    it('should reject invalid latency values', () => {
      expect(validateLatency('-10').valid).toBe(false);
      expect(validateLatency('60001').valid).toBe(false);
    });
  });

  describe('validateChain', () => {
    it('should accept valid chains', () => {
      expect(validateChain('solana').valid).toBe(true);
      expect(validateChain('ethereum').valid).toBe(true);
      expect(validateChain('Polygon').valid).toBe(true); // Case insensitive
    });

    it('should reject invalid chains', () => {
      expect(validateChain('').valid).toBe(false);
      expect(validateChain('invalid-chain').valid).toBe(false);
    });
  });

  describe('validateSlaTier', () => {
    it('should accept valid SLA tiers', () => {
      expect(validateSlaTier('free').valid).toBe(true);
      expect(validateSlaTier('pro').valid).toBe(true);
      expect(validateSlaTier('ENTERPRISE').valid).toBe(true); // Case insensitive
    });

    it('should reject invalid SLA tiers', () => {
      expect(validateSlaTier('').valid).toBe(false);
      expect(validateSlaTier('invalid-tier').valid).toBe(false);
    });
  });

  describe('validateAdminActionParams', () => {
    it('should validate faucet_config params', () => {
      expect(validateAdminActionParams('faucet_config', { amount: 100 }).valid).toBe(true);
      expect(validateAdminActionParams('faucet_config', { amount: 'invalid' }).valid).toBe(false);
    });

    it('should validate emergency_pause params', () => {
      expect(validateAdminActionParams('emergency_pause', { duration: 300 }).valid).toBe(true);
      expect(validateAdminActionParams('emergency_pause', { duration: 'invalid' }).valid).toBe(false);
    });
  });

  describe('validateForm', () => {
    it('should validate multiple fields', () => {
      const schema = {
        email: validateEmail,
        password: validatePassword,
        chain: validateChain,
      };

      const validData = {
        email: 'test@example.com',
        password: 'SecurePass123',
        chain: 'solana',
      };

      const result = validateForm(validData, schema);
      expect(result.valid).toBe(true);
      expect(Object.keys(result.errors).length).toBe(0);
    });

    it('should collect multiple validation errors', () => {
      const schema = {
        email: validateEmail,
        password: validatePassword,
      };

      const invalidData = {
        email: 'invalid-email',
        password: 'weak',
      };

      const result = validateForm(invalidData, schema);
      expect(result.valid).toBe(false);
      expect(result.errors.email).toBeDefined();
      expect(result.errors.password).toBeDefined();
    });
  });
});
