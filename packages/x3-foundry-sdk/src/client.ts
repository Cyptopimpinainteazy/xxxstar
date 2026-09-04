/**
 * HTTP client for the X3 Foundry API.
 *
 * Provides a configurable Axios-based client with authentication,
 * automatic retries, error handling, and request/response logging.
 *
 * @module @x3/foundry-sdk/client
 */

import axios, {
  AxiosInstance,
  AxiosRequestConfig,
  AxiosResponse,
  AxiosError,
} from 'axios';

// =============================================================================
// Types
// =============================================================================

/**
 * Configuration for the API client.
 */
export interface ApiClientConfig {
  /** Base URL for the Foundry API */
  baseUrl: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Maximum number of retry attempts (default: 3) */
  maxRetries?: number;
  /** Authentication token or API key */
  apiKey?: string;
  /** Additional headers to include in all requests */
  headers?: Record<string, string>;
}

/**
 * Resolved configuration after defaults are applied. Scalar fields are
 * required (defaults applied in the constructor); `apiKey` and `headers`
 * remain optional because they can be cleared or added at runtime.
 */
interface ResolvedApiClientConfig {
  /** Base URL for the Foundry API */
  baseUrl: string;
  /** Request timeout in milliseconds */
  timeout: number;
  /** Maximum number of retry attempts */
  maxRetries: number;
  /** Authentication token or API key (optional, cleared via clearAuthToken) */
  apiKey?: string;
  /** Additional headers to include in all requests */
  headers: Record<string, string>;
}

/**
 * Retry configuration for failed requests.
 */
interface RetryConfig {
  /** Maximum number of retry attempts */
  maxRetries: number;
  /** Base delay in milliseconds between retries */
  baseDelayMs: number;
  /** HTTP status codes that should trigger a retry */
  retryableStatuses: number[];
}

// =============================================================================
// Error Classes
// =============================================================================

/**
 * Base error class for API client errors.
 */
export class ApiClientError extends Error {
  /** HTTP status code */
  public readonly statusCode: number;
  /** Error code from the API */
  public readonly code: string;
  /** Request ID for tracing */
  public readonly requestId?: string;
  /** Detailed error information */
  public readonly details?: Record<string, unknown>;

  constructor(
    message: string,
    statusCode: number,
    code: string,
    requestId?: string,
    details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'ApiClientError';
    this.statusCode = statusCode;
    this.code = code;
    this.requestId = requestId;
    this.details = details;
  }
}

/**
 * Error thrown when authentication fails.
 */
export class AuthenticationError extends ApiClientError {
  constructor(message = 'Authentication failed', requestId?: string) {
    super(message, 401, 'UNAUTHORIZED', requestId);
    this.name = 'AuthenticationError';
  }
}

/**
 * Error thrown when a request rate limit is exceeded.
 */
export class RateLimitError extends ApiClientError {
  /** Time in seconds until the rate limit resets */
  public readonly retryAfterSeconds: number;

  constructor(retryAfterSeconds: number, requestId?: string) {
    super(
      `Rate limit exceeded. Retry after ${retryAfterSeconds} seconds`,
      429,
      'RATE_LIMITED',
      requestId
    );
    this.name = 'RateLimitError';
    this.retryAfterSeconds = retryAfterSeconds;
  }
}

/**
 * Error thrown when a requested resource is not found.
 */
export class NotFoundError extends ApiClientError {
  constructor(resource: string, requestId?: string) {
    super(`Resource not found: ${resource}`, 404, 'NOT_FOUND', requestId);
    this.name = 'NotFoundError';
  }
}

/**
 * Error thrown when a validation error occurs.
 */
export class ValidationError extends ApiClientError {
  constructor(message: string, details?: Record<string, unknown>, requestId?: string) {
    super(message, 422, 'VALIDATION_ERROR', requestId, details);
    this.name = 'ValidationError';
  }
}

// =============================================================================
// ApiClient
// =============================================================================

/**
 * Configurable HTTP client for the Foundry API.
 *
 * Provides typed methods for GET, POST, PUT, and DELETE requests
 * with automatic authentication header injection, retry logic,
 * and comprehensive error handling.
 *
 * @example
 * ```typescript
 * const client = new ApiClient({
 *   baseUrl: 'https://api.foundry.x3.ai',
 *   apiKey: 'your-api-key',
 *   timeout: 15000,
 * });
 *
 * const projects = await client.get('/projects');
 * const project = await client.post('/projects', { name: 'My DApp' });
 * ```
 */
export class ApiClient {
  private readonly client: AxiosInstance;
  private readonly config: ResolvedApiClientConfig;
  private readonly retryConfig: RetryConfig;

  constructor(config: ApiClientConfig) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/+$/, ''),
      timeout: config.timeout ?? 30000,
      maxRetries: config.maxRetries ?? 3,
      apiKey: config.apiKey,
      headers: config.headers ?? {},
    };

    this.retryConfig = {
      maxRetries: this.config.maxRetries,
      baseDelayMs: 1000,
      retryableStatuses: [408, 429, 500, 502, 503, 504],
    };

    this.client = axios.create({
      baseURL: this.config.baseUrl,
      timeout: this.config.timeout,
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
        ...this.config.headers,
      },
    });

    this.setupInterceptors();
  }

  /**
   * Get the base URL of the API client.
   */
  get baseUrl(): string {
    return this.config.baseUrl;
  }

  /**
   * Update the authentication token.
   */
  setAuthToken(token: string): void {
    this.config.apiKey = token;
  }

  /**
   * Clear the authentication token.
   */
  clearAuthToken(): void {
    this.config.apiKey = undefined;
  }

  /**
   * Perform a GET request.
   *
   * @param path - API endpoint path (e.g., '/projects')
   * @param params - Optional query parameters
   * @param config - Optional Axios request configuration overrides
   * @returns The response data
   */
  async get<T = unknown>(
    path: string,
    params?: Record<string, unknown>,
    config?: AxiosRequestConfig
  ): Promise<T> {
    const response = await this.executeWithRetry<T>({
      method: 'GET',
      url: path,
      params,
      ...config,
    });
    return response;
  }

  /**
   * Perform a POST request.
   *
   * @param path - API endpoint path
   * @param data - Request body
   * @param config - Optional Axios request configuration overrides
   * @returns The response data
   */
  async post<T = unknown>(
    path: string,
    data?: unknown,
    config?: AxiosRequestConfig
  ): Promise<T> {
    const response = await this.executeWithRetry<T>({
      method: 'POST',
      url: path,
      data,
      ...config,
    });
    return response;
  }

  /**
   * Perform a PUT request.
   *
   * @param path - API endpoint path
   * @param data - Request body
   * @param config - Optional Axios request configuration overrides
   * @returns The response data
   */
  async put<T = unknown>(
    path: string,
    data?: unknown,
    config?: AxiosRequestConfig
  ): Promise<T> {
    const response = await this.executeWithRetry<T>({
      method: 'PUT',
      url: path,
      data,
      ...config,
    });
    return response;
  }

  /**
   * Perform a DELETE request.
   *
   * @param path - API endpoint path
   * @param config - Optional Axios request configuration overrides
   * @returns The response data
   */
  async delete<T = unknown>(
    path: string,
    config?: AxiosRequestConfig
  ): Promise<T> {
    const response = await this.executeWithRetry<T>({
      method: 'DELETE',
      url: path,
      ...config,
    });
    return response;
  }

  // ===========================================================================
  // Private Methods
  // ===========================================================================

  /**
   * Set up Axios request and response interceptors.
   */
  private setupInterceptors(): void {
    // Request interceptor: inject auth token
    this.client.interceptors.request.use(
      (config) => {
        if (this.config.apiKey) {
          config.headers.Authorization = `Bearer ${this.config.apiKey}`;
        }
        return config;
      },
      (error) => Promise.reject(error)
    );

    // Response interceptor: transform errors
    this.client.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        throw this.transformError(error);
      }
    );
  }

  /**
   * Execute a request with automatic retry logic.
   *
   * Retries are performed for retryable status codes (408, 429, 5xx)
   * with exponential backoff and jitter.
   */
  private async executeWithRetry<T>(
    requestConfig: AxiosRequestConfig
  ): Promise<T> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= this.retryConfig.maxRetries; attempt++) {
      try {
        const response: AxiosResponse<T> = await this.client.request(requestConfig);
        return response.data;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        // Don't retry if this was the last attempt
        if (attempt >= this.retryConfig.maxRetries) {
          break;
        }

        // Only retry on specific errors
        if (error instanceof ApiClientError) {
          if (!this.retryConfig.retryableStatuses.includes(error.statusCode)) {
            break;
          }
        } else if (axios.isAxiosError(error)) {
          // Retry on network errors
          if (!error.response && error.code !== 'ECONNABORTED') {
            // Network error, retry
          } else if (error.response) {
            const status = error.response.status;
            if (!this.retryConfig.retryableStatuses.includes(status)) {
              break;
            }
          } else {
            // Timeout or other error, don't retry
            break;
          }
        } else {
          // Non-Axios error, don't retry
          break;
        }

        // Exponential backoff with jitter
        const delay = this.calculateBackoff(attempt);
        await this.sleep(delay);
      }
    }

    throw lastError ?? new Error('Request failed after retries');
  }

  /**
   * Calculate exponential backoff delay with jitter.
   */
  private calculateBackoff(attempt: number): number {
    const exponentialDelay = this.retryConfig.baseDelayMs * Math.pow(2, attempt);
    const jitter = Math.random() * 0.3 * exponentialDelay;
    return Math.min(exponentialDelay + jitter, 30000); // Cap at 30 seconds
  }

  /**
   * Transform an Axios error into a typed API error.
   */
  private transformError(error: AxiosError): Error {
    if (error.response) {
      const { status, data } = error.response;
      const body = data as Record<string, unknown> | undefined;
      const message = (body?.message as string) ?? error.message;
      const code = (body?.code as string) ?? 'UNKNOWN_ERROR';
      const requestId = body?.requestId as string | undefined;
      const details = body?.details as Record<string, unknown> | undefined;

      switch (status) {
        case 401:
          return new AuthenticationError(message, requestId);
        case 404:
          return new NotFoundError(message, requestId);
        case 429: {
          const retryAfter = parseInt(
            error.response.headers['retry-after'] as string,
            10
          ) || 60;
          return new RateLimitError(retryAfter, requestId);
        }
        case 422:
          return new ValidationError(message, details, requestId);
        default:
          return new ApiClientError(message, status, code, requestId, details);
      }
    }

    if (error.code === 'ECONNABORTED') {
      return new ApiClientError(
        `Request timed out after ${this.config.timeout}ms`,
        408,
        'TIMEOUT'
      );
    }

    if (!error.response) {
      return new ApiClientError(
        `Network error: ${error.message}`,
        0,
        'NETWORK_ERROR'
      );
    }

    return new ApiClientError(error.message, 0, 'UNKNOWN_ERROR');
  }

  /**
   * Promise-based sleep utility.
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}
