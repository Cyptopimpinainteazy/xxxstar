import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useTokenRefresh } from './useTokenRefresh';
import { api } from '../api';

const buildJwt = (expSecondsFromNow: number): string => {
  const header = btoa(JSON.stringify({ alg: 'HS256', typ: 'JWT' }));
  const payload = btoa(
    JSON.stringify({
      exp: Math.floor(Date.now() / 1000) + expSecondsFromNow,
      iat: Math.floor(Date.now() / 1000),
      sub: 'validator-test',
    }),
  );

  return `${header}.${payload}.signature`;
};

describe('useTokenRefresh', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    localStorage.clear();
    sessionStorage.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('should attempt token refresh when token is near expiry', async () => {
    const token = buildJwt(60);
    sessionStorage.setItem('infra_jwt_token', token);

    const refreshSpy = vi.spyOn(api, 'refreshToken').mockResolvedValue(true);

    renderHook(() => useTokenRefresh());

    await vi.advanceTimersByTimeAsync(1_050);

    expect(refreshSpy).toHaveBeenCalled();
  });

  it('should trigger logout when token refresh fails', async () => {
    const token = buildJwt(60);
    sessionStorage.setItem('infra_jwt_token', token);

    const refreshSpy = vi.spyOn(api, 'refreshToken').mockResolvedValue(false);
    const logoutSpy = vi.spyOn(api, 'logout').mockImplementation(() => {
      sessionStorage.removeItem('infra_jwt_token');
      sessionStorage.removeItem('infra_validator_id');
      localStorage.removeItem('infra_jwt_token');
      localStorage.removeItem('infra_api_key');
    });

    renderHook(() => useTokenRefresh());

    await vi.advanceTimersByTimeAsync(1_050);

    expect(refreshSpy).toHaveBeenCalled();
    expect(logoutSpy).toHaveBeenCalled();
    expect(sessionStorage.getItem('infra_jwt_token')).toBeNull();
  });
});
