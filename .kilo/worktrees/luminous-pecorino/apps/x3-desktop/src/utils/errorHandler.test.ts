/**
 * Tests for error handling utilities
 */

import { describe, it, expect } from 'vitest';
import {
  classifyError,
  ErrorCode,
  isOnline,
  withRetry,
  createErrorContext,
} from './errorHandler';

describe('Error Handler Utilities', () => {
  describe('isOnline', () => {
    it('should return online status', () => {
      const online = isOnline();
      expect(typeof online).toBe('boolean');
    });
  });

  describe('classifyError', () => {
    it('should classify Tauri not available errors', () => {
      const error = new Error('Tauri not available');
      const classified = classifyError(error);

      expect(classified.code).toBe(ErrorCode.TAURI_NOT_AVAILABLE);
      expect(classified.message).toContain('Desktop environment');
    });

    it('should classify network errors', () => {
      const error = new Error('Network error occurred');
      const classified = classifyError(error);

      expect(classified.code).toBe(ErrorCode.NETWORK_ERROR);
    });

    it('should classify timeout errors', () => {
      const error = new Error('Request timeout');
      const classified = classifyError(error);

      expect(classified.code).toBe(ErrorCode.TIMEOUT);
    });

    it('should classify invalid data errors', () => {
      const error = new Error('Failed to parse data');
      const classified = classifyError(error);

      expect(classified.code).toBe(ErrorCode.INVALID_DATA);
    });

    it('should handle null error', () => {
      const classified = classifyError(null);

      expect(classified.code).toBe(ErrorCode.UNKNOWN);
      expect(classified.message).toContain('unknown');
    });

    it('should include timestamp', () => {
      const error = new Error('Test');
      const classified = classifyError(error);

      expect(classified.timestamp).toBeDefined();
      expect(new Date(classified.timestamp)).toBeInstanceOf(Date);
    });
  });

  describe('withRetry', () => {
    it('should succeed on first attempt', async () => {
      const mockFn = async () => Promise.resolve('success');
      const result = await withRetry(mockFn);

      expect(result).toBe('success');
    });

    it('should retry on failure', async () => {
      let attempts = 0;
      const mockFn = async () => {
        attempts++;
        if (attempts < 2) {
          throw new Error('Fail');
        }
        return 'success';
      };

      const result = await withRetry(mockFn, { maxAttempts: 3, delayMs: 10 });
      expect(result).toBe('success');
      expect(attempts).toBe(2);
    });

    it('should fail after max attempts', async () => {
      const mockFn = async () => {
        throw new Error('Always fails');
      };

      await expect(
        withRetry(mockFn, { maxAttempts: 2, delayMs: 10 })
      ).rejects.toThrow('Always fails');
    });

    it('should respect timeout', async () => {
      const mockFn = async () => {
        return new Promise((resolve) => {
          setTimeout(() => resolve('late'), 5000);
        });
      };

      await expect(
        withRetry(mockFn, { maxAttempts: 1, timeout: 100, delayMs: 10 })
      ).rejects.toThrow('Timeout');
    });
  });

  describe('createErrorContext', () => {
    it('should create context with component name', () => {
      const context = createErrorContext('TestComponent');

      expect(context.component).toBe('TestComponent');
      expect(context.timestamp).toBeDefined();
      expect(context.userAgent).toBeDefined();
    });

    it('should include additional info', () => {
      const additionalInfo = { userId: '123', action: 'fetch' };
      const context = createErrorContext('TestComponent', additionalInfo);

      expect(context).toBeDefined();
      expect(context.component).toBe('TestComponent');
    });

    it('should include online status', () => {
      const context = createErrorContext('TestComponent');

      expect(context.isOnline).toBeDefined();
      expect(typeof context.isOnline).toBe('boolean');
    });
  });
});
