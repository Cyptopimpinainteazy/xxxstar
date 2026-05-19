import { useState, useEffect, useCallback } from 'react';

/**
 * Custom hook for fetching and managing live X3 Chain statistics
 * @param {number} refreshInterval - Polling interval in milliseconds (default: 5000)
 * @returns {Object} Live stats data, loading state, error state, and refresh function
 */
export default function useLiveStats(refreshInterval = 5000) {
  const [stats, setStats] = useState({
    validators: 0,
    tps: 0,
    blockHeight: 0,
    tvlUsd: 0,
    recentTransactions: [],
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  /**
   * Calculate average TPS from validator array
   * @param {Array} validators - Array of validator objects
   * @returns {number} Average TPS across all validators
   */
  const calculateAvgTps = (validators) => {
    if (!validators || validators.length === 0) return 0;
    const totalTps = validators.reduce((sum, v) => sum + (v.tps || 0), 0);
    return Math.round(totalTps / validators.length);
  };

  /**
   * Fetch data from the API
   */
  const fetchData = useCallback(async () => {
    try {
      // Try to fetch from API first
      const response = await fetch('/api/site/dashboard');
      
      if (!response.ok) {
        // Fallback to static data if API is unavailable
        const staticData = await import('../../data/business-store.json');
        setStats({
          validators: staticData.networkTelemetry?.validators?.length || 42,
          tps: calculateAvgTps(staticData.networkTelemetry?.validators),
          blockHeight: 1847341,
          tvlUsd: staticData.staking?.totalValueLockedUsd || 48200000,
          recentTransactions: staticData.marketWhales?.events || [],
        });
      } else {
        const data = await response.json();
        setStats({
          validators: data.networkTelemetry?.validators?.length || 42,
          tps: calculateAvgTps(data.networkTelemetry?.validators),
          blockHeight: data.dashboard?.blockNumber || 1847341,
          tvlUsd: data.staking?.totalValueLockedUsd || 48200000,
          recentTransactions: data.marketWhales?.events || [],
        });
      }
      setError(null);
    } catch (err) {
      // On error, try to use static data as fallback
      try {
        const staticData = await import('../../data/business-store.json');
        setStats({
          validators: staticData.networkTelemetry?.validators?.length || 42,
          tps: calculateAvgTps(staticData.networkTelemetry?.validators),
          blockHeight: 1847341,
          tvlUsd: staticData.staking?.totalValueLockedUsd || 48200000,
          recentTransactions: staticData.marketWhales?.events || [],
        });
        setError(null);
      } catch {
        setError(err.message);
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // Initial fetch
    fetchData();

    // Set up polling interval
    const interval = setInterval(fetchData, refreshInterval);

    // CRITICAL: Clear interval in cleanup to prevent memory leaks
    return () => clearInterval(interval);
  }, [fetchData, refreshInterval]);

  return {
    ...stats,
    loading,
    error,
    refresh: fetchData,
  };
}