import { useState, useCallback } from 'react';
import { APIErrorHandler } from '../api-client';

interface UseApiErrorState {
  error: string | null;
  isError: boolean;
}

export const useApiError = () => {
  const [state, setState] = useState<UseApiErrorState>({
    error: null,
    isError: false,
  });

  const setError = useCallback((error: unknown) => {
    const message = APIErrorHandler.handleError(error);
    setState({ error: message, isError: true });
  }, []);

  const clearError = useCallback(() => {
    setState({ error: null, isError: false });
  }, []);

  return {
    ...state,
    setError,
    clearError,
  };
};
