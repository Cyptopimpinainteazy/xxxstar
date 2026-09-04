/**
 * HTTP Interceptor for API integration
 * Provides centralized error handling, token injection, and retry logic
 */

import axios, { AxiosError } from 'axios';
import type { AxiosResponse, InternalAxiosRequestConfig } from 'axios';

export class APIErrorHandler {
  static handleError(error: unknown): string {
    if (axios.isAxiosError(error)) {
      const axiosError = error as AxiosError;
      
      if (axiosError.response) {
        // Server responded with error status
        const status = axiosError.response.status;
        const data = axiosError.response.data as Record<string, unknown>;
        
        if (status === 401) {
          return 'Unauthorized: Please log in again';
        }
        if (status === 403) {
          return 'Forbidden: You do not have permission for this action';
        }
        if (status === 404) {
          return 'Not found: The requested resource does not exist';
        }
        if (status === 429) {
          return 'Rate limited: Too many requests. Please try again later';
        }
        if (status >= 500) {
          return 'Server error: Please try again later';
        }
        
        return (data?.message as string) || `Request failed with status ${status}`;
      }
      
      if (axiosError.request) {
        // Request made but no response
        return 'Network error: Unable to reach the server';
      }
    }
    
    if (error instanceof Error) {
      return error.message;
    }
    
    return 'An unexpected error occurred';
  }
}

export class APIClient {
  private retryAttempts = 3;
  private retryDelay = 1000;

  /**
   * Retry wrapper for failed requests
   */
  async withRetry<T>(
    fn: () => Promise<T>,
    attempt = 0,
  ): Promise<T> {
    try {
      return await fn();
    } catch (error) {
      if (attempt < this.retryAttempts && this.isRetryableError(error)) {
        await new Promise(resolve => setTimeout(resolve, this.retryDelay * Math.pow(2, attempt)));
        return this.withRetry(fn, attempt + 1);
      }
      throw error;
    }
  }

  /**
   * Determine if error is retryable
   */
  private isRetryableError(error: unknown): boolean {
    if (!axios.isAxiosError(error)) {
      return false;
    }

    const status = error.response?.status;
    if (!status) {
      // Network error, likely retryable
      return true;
    }

    // Don't retry client errors (4xx), except 429 (rate limit)
    // Do retry server errors (5xx)
    return status === 429 || status >= 500;
  }

  /**
   * Create axios interceptor for API responses
   */
  static createInterceptor(client: any) {
    client.interceptors.request.use(
      (config: InternalAxiosRequestConfig) => {
        // Security: JWT is session-scoped only.
        const token = sessionStorage.getItem('infra_jwt_token');
        if (token) {
          config.headers.Authorization = `Bearer ${token}`;
        }
        return config;
      },
      (error: AxiosError) => Promise.reject(error),
    );

    client.interceptors.response.use(
      (response: AxiosResponse) => response,
      (error: AxiosError) => {
        if (error.response?.status === 401) {
          // Clear stored credentials on 401
          sessionStorage.removeItem('infra_jwt_token');
          // Clear any legacy persisted keys from older builds.
          localStorage.removeItem('infra_jwt_token');
          localStorage.removeItem('infra_api_key');
          sessionStorage.removeItem('infra_admin_token');
          // Could emit logout event here if needed
        }
        return Promise.reject(error);
      },
    );
  }
}
