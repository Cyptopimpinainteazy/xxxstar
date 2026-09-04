import { useState, useCallback } from 'react';
import { APIErrorHandler } from '../api-client';

interface UseApiState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  isError: boolean;
}

export const useApi = <T,>(
  initialData: T | null = null,
) => {
  const [state, setState] = useState<UseApiState<T>>({
    data: initialData,
    loading: false,
    error: null,
    isError: false,
  });

  const execute = useCallback(async (fn: () => Promise<T>) => {
    setState(prev => ({ ...prev, loading: true, error: null, isError: false }));
    try {
      const result = await fn();
      setState(prev => ({ ...prev, data: result, loading: false }));
      return result;
    } catch (error) {
      const message = APIErrorHandler.handleError(error);
      setState(prev => ({ ...prev, error: message, isError: true, loading: false }));
      throw error;
    }
  }, []);

  const clearError = useCallback(() => {
    setState(prev => ({ ...prev, error: null, isError: false }));
  }, []);

  const setData = useCallback((data: T) => {
    setState(prev => ({ ...prev, data }));
  }, []);

  return {
    ...state,
    execute,
    clearError,
    setData,
  };
};
