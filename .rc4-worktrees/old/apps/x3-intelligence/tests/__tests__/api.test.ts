import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as api from '../../src/services/api';

describe('API Service', () => {
  const mockFetch = vi.fn();
  let originalFetch: typeof global.fetch;

  beforeEach(() => {
    originalFetch = global.fetch;
    global.fetch = mockFetch;
  });

  afterEach(() => {
    global.fetch = originalFetch;
    mockFetch.mockClear();
  });

  describe('getFloorStats', () => {
    it('should fetch floor stats', async () => {
      const mockStats = {
        activeAgents: 47,
        totalIntents: 12849,
        totalVolume: '84,291,003.21',
        totalSlashes: 23,
        totalDisputes: 7,
        avgSuccessRate: 94.7,
        activeFlashloans: 3,
      };

      mockFetch.mockResolvedValueOnce(
        new Response(JSON.stringify(mockStats), { status: 200 })
      );

      const stats = await api.getFloorStats();
      expect(stats).toEqual(mockStats);
      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8001/api/v1/floor/stats');
    });

    it('should throw on API error', async () => {
      mockFetch.mockResolvedValueOnce(
        new Response('Not Found', { status: 404 })
      );

      await expect(api.getFloorStats()).rejects.toThrow();
    });
  });

  describe('getIntents', () => {
    it('should fetch intents with pagination', async () => {
      const mockResponse = {
        items: [],
        total: 100,
        page: 1,
        pageSize: 25,
      };

      mockFetch.mockResolvedValueOnce(
        new Response(JSON.stringify(mockResponse), { status: 200 })
      );

      const result = await api.getIntents(1, 25);
      expect(result).toEqual(mockResponse);
      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8001/api/v1/intents?page=1&pageSize=25');
    });
  });

  describe('getAgents', () => {
    it('should fetch agents with pagination', async () => {
      const mockResponse = {
        items: [],
        total: 50,
        page: 1,
        pageSize: 25,
      };

      mockFetch.mockResolvedValueOnce(
        new Response(JSON.stringify(mockResponse), { status: 200 })
      );

      const result = await api.getAgents(1, 25);
      expect(result).toEqual(mockResponse);
    });
  });

  describe('getSlashEvents', () => {
    it('should fetch slash events', async () => {
      const mockResponse = {
        items: [],
        total: 23,
        page: 1,
        pageSize: 25,
      };

      mockFetch.mockResolvedValueOnce(
        new Response(JSON.stringify(mockResponse), { status: 200 })
      );

      const result = await api.getSlashEvents(undefined, 1, 25);
      expect(result).toEqual(mockResponse);
    });

    it('should fetch slash events for specific agent', async () => {
      const mockResponse = {
        items: [],
        total: 3,
        page: 1,
        pageSize: 25,
      };

      mockFetch.mockResolvedValueOnce(
        new Response(JSON.stringify(mockResponse), { status: 200 })
      );

      const result = await api.getSlashEvents('agent-id', 1, 25);
      expect(result).toEqual(mockResponse);
      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:8001/api/v1/slashes?page=1&pageSize=25&agentId=agent-id'
      );
    });
  });

  describe('estimateFee', () => {
    it('should estimate fees with given parameters', async () => {
      const mockFeeVector = {
        baseFee: 10,
        complexityFee: 5,
        capitalFee: 3,
        reputationDiscount: 0,
        totalFee: 18,
      };

      mockFetch.mockResolvedValueOnce(
        new Response(JSON.stringify(mockFeeVector), { status: 200 })
      );

      const result = await api.estimateFee({
        legs: 2,
        stateTouches: 10,
        capitalAmount: 50000,
      });

      expect(result).toEqual(mockFeeVector);
      expect(mockFetch).toHaveBeenCalled();
    });
  });

  describe('getBondState', () => {
    it('should fetch bond state', async () => {
      const mockBondState = {
        balance: 1000,
        lockedUntil: null,
        pendingWithdrawals: [],
      };

      mockFetch.mockResolvedValueOnce(
        new Response(JSON.stringify(mockBondState), { status: 200 })
      );

      const result = await api.getBondState();
      expect(result).toEqual(mockBondState);
      expect(mockFetch).toHaveBeenCalledWith('http://localhost:8001/api/v1/bonds/state');
    });
  });
});
