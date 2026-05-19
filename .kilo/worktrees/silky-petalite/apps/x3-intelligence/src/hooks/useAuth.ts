import { useEffect, useState, useCallback } from 'react';
import authService from '../services/authService';

interface User {
  id: string;
  username: string;
  email?: string;
}

/**
 * useAuth Hook
 * Provides authentication state and methods throughout the app
 * 
 * @example
 * const { user, isAuthenticated, loading, login, logout } = useAuth();
 */
export const useAuth = () => {
  const [user, setUser] = useState<User | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Initialize auth state on mount
  useEffect(() => {
    const currentUser = authService.getCurrentUser();
    if (currentUser) {
      setUser(currentUser);
      setIsAuthenticated(true);
    }
    setLoading(false);
  }, []);

  // Login handler
  const login = useCallback(
    async (username: string, password: string) => {
      setError(null);
      setLoading(true);

      try {
        const response = await authService.login({ username, password });
        setUser(response.user);
        setIsAuthenticated(true);
        return true;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Login failed';
        setError(errorMessage);
        setIsAuthenticated(false);
        return false;
      } finally {
        setLoading(false);
      }
    },
    []
  );

  // Logout handler
  const logout = useCallback(() => {
    authService.logout();
    setUser(null);
    setIsAuthenticated(false);
    setError(null);
  }, []);

  // Change password handler
  const changePassword = useCallback(
    async (currentPassword: string, newPassword: string) => {
      setError(null);

      try {
        const success = await authService.changePassword(
          currentPassword,
          newPassword
        );

        if (!success) {
          setError('Failed to change password');
          return false;
        }

        return true;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Password change failed';
        setError(errorMessage);
        return false;
      }
    },
    []
  );

  // Validate token handler
  const validateToken = useCallback(async () => {
    try {
      const isValid = await authService.validateToken();
      setIsAuthenticated(isValid);
      return isValid;
    } catch (err) {
      setIsAuthenticated(false);
      return false;
    }
  }, []);

  // Refresh token handler
  const refreshToken = useCallback(async () => {
    try {
      const newToken = await authService.refreshToken();
      const valid = newToken !== null;
      setIsAuthenticated(valid);
      return valid;
    } catch (err) {
      setIsAuthenticated(false);
      return false;
    }
  }, []);

  // Get auth header for API calls
  const getAuthHeader = useCallback(() => {
    return authService.getAuthHeader();
  }, []);

  return {
    user,
    isAuthenticated,
    loading,
    error,
    login,
    logout,
    changePassword,
    validateToken,
    refreshToken,
    getAuthHeader,
  };
};

export default useAuth;
