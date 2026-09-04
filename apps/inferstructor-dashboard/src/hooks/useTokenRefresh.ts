import { useEffect, useRef, useCallback } from 'react';
import { api } from '../api';
import { TOKEN_REFRESH_BUFFER_MS, TOKEN_REFRESH_MIN_INTERVAL_MS } from '../constants';

/**
 * Hook to automatically refresh JWT token before expiry
 * 
 * Decodes JWT payload to determine expiry time, then schedules
 * token refresh 5 minutes before expiration. On refresh failure,
 * triggers logout.
 */

interface TokenPayload {
  exp?: number;
  iat?: number;
  sub?: string;
  [key: string]: any;
}

interface UseTokenRefreshReturn {
  logout: () => void;
  isTokenValid: () => boolean;
  getTokenExpiry: () => number | null;
}

export const useTokenRefresh = (): UseTokenRefreshReturn => {
  const refreshTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const logoutCallbackRef = useRef<(() => void) | null>(null);

  /**
   * Decode JWT payload (without verification - for client-side expiry check only)
   */
  const decodeJWT = useCallback((token: string): TokenPayload | null => {
    try {
      const parts = token.split('.');
      if (parts.length !== 3) {
        console.warn('Invalid JWT format');
        return null;
      }

      const payload = parts[1];
      const decoded = JSON.parse(
        decodeURIComponent(
          atob(payload)
            .split('')
            .map((c) => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
            .join('')
        )
      );

      return decoded as TokenPayload;
    } catch (error) {
      console.error('Failed to decode JWT:', error);
      return null;
    }
  }, []);

  /**
   * Get token expiry time in milliseconds from now
   */
  const getTokenExpiry = useCallback((): number | null => {
    // Security: Read JWT from sessionStorage (not localStorage)
    const token = sessionStorage.getItem('infra_jwt_token');
    if (!token) return null;

    const payload = decodeJWT(token);
    if (!payload || !payload.exp) return null;

    // exp is in seconds, convert to milliseconds
    const expiryTime = payload.exp * 1000;
    const now = Date.now();

    return expiryTime - now;
  }, [decodeJWT]);

  /**
   * Check if token is still valid
   */
  const isTokenValid = useCallback((): boolean => {
    const expiryIn = getTokenExpiry();
    return expiryIn !== null && expiryIn > 0;
  }, [getTokenExpiry]);

  /**
   * Attempt to refresh the token
   */
  const refreshToken = useCallback(async (): Promise<boolean> => {
    try {
      console.log('Attempting token refresh...');
      
      // Call the new refreshToken method on api
      const success = await api.refreshToken();
      
      if (success) {
        console.log('Token refreshed successfully');
        return true;
      }
      
      console.warn('Token refresh failed - logging out');
      return false;
    } catch (error) {
      console.error('Token refresh failed:', error);
      return false;
    }
  }, []);

  /**
   * Perform logout
   */
  const logout = useCallback((): void => {
    if (refreshTimeoutRef.current) {
      clearTimeout(refreshTimeoutRef.current);
      refreshTimeoutRef.current = null;
    }

    api.logout();

    // Call registered logout callback if available
    if (logoutCallbackRef.current) {
      logoutCallbackRef.current();
    }
  }, []);

  /**
   * Schedule token refresh
   */
  const scheduleTokenRefresh = useCallback((): void => {
    // Clear any existing timeout
    if (refreshTimeoutRef.current) {
      clearTimeout(refreshTimeoutRef.current);
    }

    const expiryIn = getTokenExpiry();

    // If no token or already expired, logout
    if (expiryIn === null || expiryIn <= 0) {
      logout();
      return;
    }

    // Refresh 5 minutes before expiry
    const refreshIn = Math.max(expiryIn - TOKEN_REFRESH_BUFFER_MS, TOKEN_REFRESH_MIN_INTERVAL_MS);

    console.log(`Token refresh scheduled in ${(refreshIn / 1000 / 60).toFixed(1)} minutes`);

    refreshTimeoutRef.current = setTimeout(async () => {
      const success = await refreshToken();

      if (success) {
        // Schedule next refresh
        scheduleTokenRefresh();
      } else {
        // Refresh failed, logout
        logout();
      }
    }, refreshIn);
  }, [getTokenExpiry, refreshToken, logout]);

  /**
   * Set external logout callback (e.g., redirect to login)
   */
  const setLogoutCallback = useCallback((callback: () => void): void => {
    logoutCallbackRef.current = callback;
  }, []);

  /**
   * Main effect: schedule token refresh on mount and when token changes
   */
  useEffect(() => {
    scheduleTokenRefresh();

    // Also check token validity on visibility change (browser tab becomes active)
    const handleVisibilityChange = (): void => {
      if (!document.hidden && !isTokenValid()) {
        logout();
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);

    // Cleanup on unmount
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      if (refreshTimeoutRef.current) {
        clearTimeout(refreshTimeoutRef.current);
      }
    };
  }, [scheduleTokenRefresh, isTokenValid, logout]);

  // Expose setLogoutCallback through the hook
  (useTokenRefresh as any).setLogoutCallback = setLogoutCallback;

  return {
    logout,
    isTokenValid,
    getTokenExpiry,
  };
};

/**
 * Set logout callback for useTokenRefresh hook
 * Call this in your root component to handle token expiry redirects
 */
export const setTokenRefreshLogoutCallback = (callback: () => void): void => {
  (useTokenRefresh as any).setLogoutCallback?.(callback);
};
