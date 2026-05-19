/**
 * Centralized error handling utilities
 */

export interface AppError {
  code: string;
  message: string;
  originalError?: Error;
  timestamp: string;
  context?: Record<string, any>;
}

export enum ErrorCode {
  TAURI_NOT_AVAILABLE = 'TAURI_NOT_AVAILABLE',
  NETWORK_ERROR = 'NETWORK_ERROR',
  TIMEOUT = 'TIMEOUT',
  IPC_FAILED = 'IPC_FAILED',
  UNKNOWN = 'UNKNOWN',
  INVALID_DATA = 'INVALID_DATA',
  OFFLINE = 'OFFLINE',
}

/**
 * Check if application is online
 */
export const isOnline = (): boolean => {
  return typeof navigator !== 'undefined' && navigator.onLine;
};

/**
 * Classify error and return user-friendly message
 */
export const classifyError = (error: any): AppError => {
  const timestamp = new Date().toISOString();

  if (!error) {
    return {
      code: ErrorCode.UNKNOWN,
      message: 'An unknown error occurred',
      timestamp,
    };
  }

  const errorMsg = (error.message || String(error)).toLowerCase();

  // Tauri not available
  if (
    errorMsg.includes('tauri') ||
    errorMsg.includes('not available') ||
    errorMsg.includes('ipc')
  ) {
    return {
      code: ErrorCode.TAURI_NOT_AVAILABLE,
      message:
        'Desktop environment not initialized. Please restart the application.',
      originalError: error,
      timestamp,
    };
  }

  // Network errors
  if (
    errorMsg.includes('network') ||
    errorMsg.includes('fetch') ||
    errorMsg.includes('econnrefused')
  ) {
    return {
      code: isOnline() ? ErrorCode.NETWORK_ERROR : ErrorCode.OFFLINE,
      message: isOnline()
        ? 'Network connection failed. Please check your connection.'
        : 'You are offline. Please check your internet connection.',
      originalError: error,
      timestamp,
    };
  }

  // Timeout
  if (errorMsg.includes('timeout')) {
    return {
      code: ErrorCode.TIMEOUT,
      message: 'Request took too long. Please try again.',
      originalError: error,
      timestamp,
    };
  }

  // Invalid data
  if (errorMsg.includes('parse') || errorMsg.includes('undefined')) {
    return {
      code: ErrorCode.INVALID_DATA,
      message: 'Received invalid data from server. Please try again.',
      originalError: error,
      timestamp,
    };
  }

  return {
    code: ErrorCode.UNKNOWN,
    message: `Error: ${error.message || String(error)}`,
    originalError: error,
    timestamp,
  };
};

// Error tracking service configuration
// Set these values from your application initialization
let errorTrackerConfig = {
  enabled: false,
  dsn: '',
  projectId: '',
  environment: 'development',
};

/**
 * Configure error tracking service
 */
export const configureErrorTracker = (config: {
  enabled: boolean;
  dsn?: string;
  projectId?: string;
  environment?: string;
}): void => {
  errorTrackerConfig = { ...errorTrackerConfig, ...config };
};

/**
 * Send error to tracking service (Sentry, LogRocket, etc.)
 */
const sendToErrorTracker = async (logEntry: Record<string, unknown>): Promise<void> => {
  if (!errorTrackerConfig.enabled || !errorTrackerConfig.dsn) {
    return;
  }

  try {
    // Build error payload for Sentry-compatible API
    const payload = {
      event_id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      level: 'error',
      message: logEntry.message,
      environment: errorTrackerConfig.environment,
      extra: logEntry,
      tags: {
        component: logEntry.context,
        code: logEntry.code,
      },
    };

    // Send to Sentry endpoint
    const response = await fetch(`${errorTrackerConfig.dsn}/api/${errorTrackerConfig.projectId}/store/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      console.warn('[ErrorTracker] Failed to send error:', response.statusText);
    }
  } catch (e) {
    // Silently fail - don't break the app for tracking failures
    console.warn('[ErrorTracker] Error tracking failed:', e);
  }
};

/**
 * Log error to console and optionally to external service
 */
export const logError = (error: AppError, context?: string): void => {
  const logEntry = {
    context,
    ...error,
    stackTrace: error.originalError?.stack,
  };

  console.error('[AppError]', logEntry);

  // Send to error tracking service if configured
  sendToErrorTracker(logEntry);
};

/**
 * Retry logic with exponential backoff
 */
export const withRetry = async <T>(
  fn: () => Promise<T>,
  options: {
    maxAttempts?: number;
    delayMs?: number;
    backoffMultiplier?: number;
    timeout?: number;
  } = {}
): Promise<T> => {
  const {
    maxAttempts = 3,
    delayMs = 1000,
    backoffMultiplier = 2,
    timeout = 10000,
  } = options;

  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      // Wrap with timeout
      return await Promise.race([
        fn(),
        new Promise<T>((_, reject) =>
          setTimeout(
            () => reject(new Error(`Timeout after ${timeout}ms`)),
            timeout
          )
        ),
      ]);
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      if (attempt === maxAttempts) break;

      const delay = delayMs * Math.pow(backoffMultiplier, attempt - 1);
      console.warn(
        `Attempt ${attempt} failed, retrying in ${delay}ms...`,
        error
      );
      await new Promise((resolve) => setTimeout(resolve, delay));
    }
  }

  throw lastError;
};

/**
 * Create error recovery context
 */
export const createErrorContext = (
  componentName: string,
  additionalInfo?: Record<string, any>
) => ({
  component: componentName,
  timestamp: new Date().toISOString(),
  userAgent: typeof navigator !== 'undefined' ? navigator.userAgent : 'unknown',
  isOnline: isOnline(),
  ...additionalInfo,
});
