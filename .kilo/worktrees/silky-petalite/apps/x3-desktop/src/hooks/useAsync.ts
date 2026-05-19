/**
 * Custom React Hook for safe async operations with error handling and retry logic
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { withRetry, classifyError, logError, createErrorContext, AppError } from '../utils/errorHandler';

interface UseAsyncState<T> {
  data: T | null;
  loading: boolean;
  error: AppError | null;
  retryCount: number;
}

interface UseAsyncOptions {
  maxRetries?: number;
  delayMs?: number;
  timeout?: number;
  autoFetch?: boolean;
}

/**
 * Hook for handling async operations with automatic retry and error handling
 */
export const useAsync = <T>(
  asyncFn: () => Promise<T>,
  options: UseAsyncOptions = {}
) => {
  const {
    maxRetries = 3,
    delayMs = 1000,
    timeout = 10000,
    autoFetch = true,
  } = options;

  const [state, setState] = useState<UseAsyncState<T>>({
    data: null,
    loading: true,
    error: null,
    retryCount: 0,
  });

  const componentNameRef = useRef('UnknownComponent');

  const executeAsync = useCallback(async () => {
    setState((prev) => ({ ...prev, loading: true, error: null }));

    try {
      const result = await withRetry(asyncFn, {
        maxAttempts: maxRetries,
        delayMs,
        timeout,
      });

      setState({
        data: result,
        loading: false,
        error: null,
        retryCount: 0,
      });
    } catch (err) {
      const appError = classifyError(err);
      const context = createErrorContext(componentNameRef.current, {
        timestamp: new Date().toISOString(),
      });
      appError.context = context;

      logError(appError);

      setState((prev) => ({
        data: null,
        loading: false,
        error: appError,
        retryCount: prev.retryCount + 1,
      }));
    }
  }, [asyncFn, maxRetries, delayMs, timeout]);

  const retry = useCallback(() => {
    setState((prev) => (prev.retryCount < maxRetries ? prev : prev));
    executeAsync();
  }, [executeAsync, maxRetries]);

  useEffect(() => {
    if (autoFetch) {
      executeAsync();
    }
  }, [autoFetch, executeAsync]);

  return {
    ...state,
    retry,
    refetch: executeAsync,
  };
};

/**
 * Hook for managing network status
 */
export const useNetworkStatus = () => {
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== 'undefined' && navigator.onLine
  );

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return isOnline;
};

/**
 * Hook for detecting Tauri availability
 */
export const useTauriAvailable = () => {
  const [available, setAvailable] = useState(true);

  useEffect(() => {
    // Check if __TAURI__ is available
    const checkTauri = async () => {
      try {
        // Simple check - see if we can access tauri
        if (typeof (window as any).__TAURI__ === 'undefined') {
          setAvailable(false);
        }
      } catch {
        setAvailable(false);
      }
    };

    checkTauri();
  }, []);

  return available;
};
