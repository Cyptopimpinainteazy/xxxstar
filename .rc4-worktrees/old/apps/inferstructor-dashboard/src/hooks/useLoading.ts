import { useState, useCallback } from 'react';

interface UseLoadingState {
  loading: boolean;
  isLoading: boolean;
  startLoading: () => void;
  stopLoading: () => void;
}

export const useLoading = (): UseLoadingState => {
  const [loading, setLoading] = useState(false);

  const startLoading = useCallback(() => {
    setLoading(true);
  }, []);

  const stopLoading = useCallback(() => {
    setLoading(false);
  }, []);

  return {
    loading,
    isLoading: loading,
    startLoading,
    stopLoading,
  };
};
