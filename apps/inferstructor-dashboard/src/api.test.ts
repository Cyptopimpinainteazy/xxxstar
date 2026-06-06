import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// Mock axios before importing api
vi.mock('axios');

import axios from 'axios';
import { api, buildGpuLaneUrls } from './api';

const mockedAxios = axios as any;

describe('InferstructorAPI', () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    api.logout();
    api.adminLogout();
  });

  describe('Authentication', () => {
    it('should register a new validator', async () => {
      const mockResponse = {
        data: {
          success: true,
          credentials: {
            validator_id: 'val_123',
            chain: 'solana',
            api_key: 'key_123',
            api_secret: 'secret_123',
            sla_tier: 'pro',
            max_tps: 1000,
            bridge_endpoint: 'http://bridge.local',
            toll_booth_endpoint: 'http://tollbooth.local',
            jwt_token: 'jwt_token_123',
          },
        },
      };
      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      const result = await api.register('solana', 'user@example.com', 'pro');

      expect(result.validator_id).toBe('val_123');
      expect(result.api_key).toBe('key_123');
    });

    it('should throw error on registration failure', async () => {
      mockedAxios.post.mockResolvedValueOnce({
        data: { success: false },
      });

      await expect(api.register('solana', 'user@example.com')).rejects.toThrow('Registration failed');
    });

    it('should login with credentials', async () => {
      const mockResponse = {
        data: {
          success: true,
          token: 'jwt_token_456',
          validator: { id: 'val_456' },
        },
      };
      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      await api.login('key_456', 'secret_456');

      // JWT should be stored in sessionStorage (not localStorage)
      expect(sessionStorage.getItem('infra_jwt_token')).toBe('jwt_token_456');
      // API key should NOT be persisted (in memory only)
      expect(api.getAPIKey()).toBe('key_456');
    });

    it('should logout and clear credentials', () => {
      sessionStorage.setItem('infra_jwt_token', 'token_123');
      sessionStorage.setItem('infra_validator_id', 'val_123');
      localStorage.setItem('infra_jwt_token', 'legacy_token_123');
      localStorage.setItem('infra_api_key', 'legacy_key_123');

      api.logout();

      expect(sessionStorage.getItem('infra_jwt_token')).toBeNull();
      expect(localStorage.getItem('infra_jwt_token')).toBeNull();
      expect(localStorage.getItem('infra_api_key')).toBeNull();
      // API key is memory-only, no need to check storage
      expect(api.getAPIKey()).toBeNull();
    });

    it('should not persist JWT token to localStorage on login', async () => {
      const mockResponse = {
        data: {
          success: true,
          token: 'jwt_token_localstorage_check',
          validator: { id: 'val_789' },
        },
      };
      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      await api.login('key_789', 'secret_789');

      expect(sessionStorage.getItem('infra_jwt_token')).toBe('jwt_token_localstorage_check');
      expect(localStorage.getItem('infra_jwt_token')).toBeNull();
      expect(localStorage.getItem('infra_api_key')).toBeNull();
    });

    it('integration: login then logout clears localStorage credential keys', async () => {
      const mockResponse = {
        data: {
          success: true,
          token: 'jwt_token_integration_1',
          validator: { id: 'val_integration_1' },
        },
      };
      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      await api.login('key_integration_1', 'secret_integration_1');

      expect(sessionStorage.getItem('infra_jwt_token')).toBe('jwt_token_integration_1');
      expect(localStorage.getItem('infra_jwt_token')).toBeNull();

      api.logout();

      expect(sessionStorage.getItem('infra_jwt_token')).toBeNull();
      expect(localStorage.getItem('infra_jwt_token')).toBeNull();
      expect(localStorage.getItem('infra_api_key')).toBeNull();
    });
  });

  describe('GPU Lane URL configuration', () => {
    it('should build localhost fallback URLs when env vars are missing', () => {
      const urls = buildGpuLaneUrls({});
      expect(urls).toEqual([
        'http://localhost:9001/health',
        'http://localhost:9002/health',
        'http://localhost:9003/health',
      ]);
    });

    it('should use base URL with per-lane override support', () => {
      const urls = buildGpuLaneUrls({
        VITE_GPU_LANE_BASE: 'https://gpu-lanes.x3star.net/',
        VITE_GPU_LANE_2_URL: 'https://custom-lane-2.x3star.net/health',
      });
      expect(urls).toEqual([
        'https://gpu-lanes.x3star.net:9001/health',
        'https://custom-lane-2.x3star.net/health',
        'https://gpu-lanes.x3star.net:9003/health',
      ]);
    });
  });

  describe('Admin Operations', () => {
    it('should admin login', async () => {
      // Mock CSRF token fetch
      const csrfMockResponse = {
        data: {
          token: 'csrf_token_123',
        },
      };
      // Mock admin login response
      const loginMockResponse = {
        data: {
          success: true,
          token: 'admin_token_123',
          expires_in: 3600,
        },
      };
      mockedAxios.get.mockResolvedValueOnce(csrfMockResponse);
      mockedAxios.get.mockResolvedValueOnce(csrfMockResponse);
      mockedAxios.post.mockResolvedValueOnce(loginMockResponse);

      const result = await api.adminLogin('password123');
      expect(result.token).toBe('admin_token_123');
    });

    it('should include CSRF token header on admin login request', async () => {
      const csrfMockResponse = {
        data: {
          token: 'csrf_token_header_check',
        },
      };
      const loginMockResponse = {
        data: {
          success: true,
          token: 'admin_token_header_check',
          expires_in: 3600,
        },
      };

      mockedAxios.get
        .mockResolvedValueOnce(csrfMockResponse)
        .mockResolvedValueOnce(csrfMockResponse);
      mockedAxios.post.mockResolvedValueOnce(loginMockResponse);

      await api.adminLogin('password123');

      expect(mockedAxios.post).toHaveBeenCalledWith(
        expect.stringContaining('/admin/login'),
        { password: 'password123' },
        { headers: { 'X-CSRF-Token': 'csrf_token_header_check' } }
      );
    });

    it('should fail admin login when CSRF token is unavailable', async () => {
      mockedAxios.get.mockResolvedValueOnce({ data: {} });

      await expect(api.adminLogin('password123')).rejects.toThrow('CSRF token unavailable');
      expect(mockedAxios.post).not.toHaveBeenCalled();
      expect(sessionStorage.getItem('infra_admin_token')).toBeNull();
      expect(sessionStorage.getItem('infra_csrf_token')).toBeNull();
    });

    it('should fail admin login with 403 when CSRF validation fails', async () => {
      mockedAxios.get.mockResolvedValueOnce({ data: { token: 'csrf_token_403' } });
      mockedAxios.post.mockRejectedValueOnce({
        isAxiosError: true,
        response: { status: 403 },
      });

      await expect(api.adminLogin('password123')).rejects.toThrow('Admin login failed: CSRF validation failed');
      expect(sessionStorage.getItem('infra_admin_token')).toBeNull();
      expect(sessionStorage.getItem('infra_csrf_token')).toBeNull();
    });

    it('should admin logout', () => {
      sessionStorage.setItem('infra_admin_token', 'admin_token_test');
      api.adminLogout();
      expect(sessionStorage.getItem('infra_admin_token')).toBeNull();
    });
  });

  describe('Token Refresh', () => {
    it('should refresh token and update sessionStorage on success', async () => {
      mockedAxios.post
        .mockResolvedValueOnce({
          data: {
            success: true,
            token: 'jwt_token_before_refresh',
            validator: { id: 'val_refresh_1' },
          },
        })
        .mockResolvedValueOnce({
          data: {
            success: true,
            token: 'jwt_token_after_refresh',
          },
        });

      await api.login('key_refresh_1', 'secret_refresh_1');
      const refreshed = await api.refreshToken();

      expect(refreshed).toBe(true);
      expect(api.getJWTToken()).toBe('jwt_token_after_refresh');
      expect(sessionStorage.getItem('infra_jwt_token')).toBe('jwt_token_after_refresh');
    });

    it('should logout when refresh fails', async () => {
      mockedAxios.post
        .mockResolvedValueOnce({
          data: {
            success: true,
            token: 'jwt_token_for_failed_refresh',
            validator: { id: 'val_refresh_2' },
          },
        })
        .mockRejectedValueOnce(new Error('refresh failed'));

      await api.login('key_refresh_2', 'secret_refresh_2');
      localStorage.setItem('infra_jwt_token', 'legacy_token_refresh_2');
      localStorage.setItem('infra_api_key', 'legacy_key_refresh_2');

      const refreshed = await api.refreshToken();

      expect(refreshed).toBe(false);
      expect(api.getJWTToken()).toBeNull();
      expect(api.getAPIKey()).toBeNull();
      expect(sessionStorage.getItem('infra_jwt_token')).toBeNull();
      expect(localStorage.getItem('infra_jwt_token')).toBeNull();
      expect(localStorage.getItem('infra_api_key')).toBeNull();
    });
  });

  describe('Error Handling', () => {
    it('should throw error if not authenticated when getting stats', async () => {
      localStorage.clear();

      await expect(api.getStats()).rejects.toThrow('Not authenticated');
    });

    it('should throw error if no API key when testing connection', async () => {
      localStorage.clear();

      await expect(api.testConnection()).rejects.toThrow('No API key');
    });
  });
});
