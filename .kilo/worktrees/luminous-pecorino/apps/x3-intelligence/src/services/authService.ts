/**
 * Authentication Service
 * Handles login, logout, session validation, and token management
 */

interface AuthResponse {
  token: string;
  user: {
    id: string;
    username: string;
    email?: string;
  };
  expiresIn: number;
}

interface LoginRequest {
  username: string;
  password: string;
}

class AuthService {
  private apiBaseUrl: string;
  private tokenKey = 'authToken';
  private userKey = 'user';
  private expiresAtKey = 'tokenExpiresAt';

  constructor() {
    this.apiBaseUrl = import.meta.env.VITE_API_BASE_URL || import.meta.env.VITE_X3_API_URL || 'https://api.x3star.net';
  }

  /**
   * Login with username and password
   * @param credentials Login credentials
   * @returns Authentication token and user data
   */
  async login(credentials: LoginRequest): Promise<AuthResponse> {
    const response = await fetch(`${this.apiBaseUrl}/api/auth/login`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(credentials),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'Login failed');
    }

    const data: AuthResponse = await response.json();

    // Store token and user data
    const expiresAt = Date.now() + data.expiresIn * 1000;
    localStorage.setItem(this.tokenKey, data.token);
    localStorage.setItem(this.userKey, JSON.stringify(data.user));
    localStorage.setItem(this.expiresAtKey, expiresAt.toString());

    return data;
  }

  /**
   * Logout and clear session
   */
  logout(): void {
    localStorage.removeItem(this.tokenKey);
    localStorage.removeItem(this.userKey);
    localStorage.removeItem(this.expiresAtKey);
  }

  /**
   * Get current authentication token
   * @returns Token string or null if not authenticated
   */
  getToken(): string | null {
    const token = localStorage.getItem(this.tokenKey);
    if (!token) return null;

    // Check if token is expired
    const expiresAt = localStorage.getItem(this.expiresAtKey);
    if (expiresAt && Date.now() > parseInt(expiresAt)) {
      this.logout();
      return null;
    }

    return token;
  }

  /**
   * Check if user is authenticated
   * @returns True if user has a valid token
   */
  isAuthenticated(): boolean {
    return this.getToken() !== null;
  }

  /**
   * Get current user data
   * @returns User object or null if not authenticated
   */
  getCurrentUser() {
    const user = localStorage.getItem(this.userKey);
    return user ? JSON.parse(user) : null;
  }

  /**
   * Validate token with server
   * @returns True if token is valid, false otherwise
   */
  async validateToken(): Promise<boolean> {
    const token = this.getToken();
    if (!token) return false;

    try {
      const response = await fetch(`${this.apiBaseUrl}/api/auth/validate`, {
        method: 'GET',
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        this.logout();
        return false;
      }

      return true;
    } catch (error) {
      console.error('Token validation failed:', error);
      return false;
    }
  }

  /**
   * Get authorization header for API requests
   * @returns Header object with authorization token
   */
  getAuthHeader(): Record<string, string> {
    const token = this.getToken();
    if (!token) {
      return {};
    }
    return {
      Authorization: `Bearer ${token}`,
    };
  }

  /**
   * Refresh token (extends session)
   * @returns New token if successful
   */
  async refreshToken(): Promise<string | null> {
    const token = this.getToken();
    if (!token) return null;

    try {
      const response = await fetch(`${this.apiBaseUrl}/api/auth/refresh`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        this.logout();
        return null;
      }

      const data: AuthResponse = await response.json();

      // Update stored token
      const expiresAt = Date.now() + data.expiresIn * 1000;
      localStorage.setItem(this.tokenKey, data.token);
      localStorage.setItem(this.expiresAtKey, expiresAt.toString());

      return data.token;
    } catch (error) {
      console.error('Token refresh failed:', error);
      return null;
    }
  }

  /**
   * Change user password
   * @param currentPassword Current password
   * @param newPassword New password to set
   * @returns True if password was changed
   */
  async changePassword(currentPassword: string, newPassword: string): Promise<boolean> {
    const token = this.getToken();
    if (!token) return false;

    try {
      const response = await fetch(`${this.apiBaseUrl}/api/auth/change-password`, {
        method: 'POST',
        headers: {
          ...this.getAuthHeader(),
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          currentPassword,
          newPassword,
        }),
      });

      return response.ok;
    } catch (error) {
      console.error('Password change failed:', error);
      return false;
    }
  }
}

// Export singleton instance
export const authService = new AuthService();
export default authService;
