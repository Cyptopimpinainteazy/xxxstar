import { useState, useEffect } from 'react';
import { api } from '../api';
import type { ValidatorStats, BridgeStats, GPULaneHealth, ChainStats } from '../api';
import { RefreshCw } from 'lucide-react';
import { DashboardHeader } from './dashboard/DashboardHeader';
import { ErrorBanner } from './dashboard/ErrorBanner';
import { StatsGrid } from './dashboard/StatsGrid';
import { TpsChart } from './dashboard/TpsChart';
import { GPULanesSection } from './dashboard/GPULanesSection';
import { SolanaChainSection } from './dashboard/SolanaChainSection';
import { DetailsGrid } from './dashboard/DetailsGrid';
import {
  MAX_TPS_HISTORY_POINTS,
  TPS_COLLECTION_INTERVAL_MS,
} from '../constants';

interface DashboardProps {
  onLogout: () => void;
  onAdmin?: () => void;
  onLeaderboard?: () => void;
}

interface TpsPoint {
  time: string;
  ts: number;
  tps: number;
  forwarded: number;
  received: number;
}

export function Dashboard({ onLogout, onAdmin, onLeaderboard }: DashboardProps) {
  const [stats, setStats] = useState<ValidatorStats | null>(null);
  const [bridgeStats, setBridgeStats] = useState<BridgeStats | null>(null);
  const [gpuLanes, setGpuLanes] = useState<GPULaneHealth[]>([]);
  const [chainStats, setChainStats] = useState<ChainStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tpsHistory, setTpsHistory] = useState<TpsPoint[]>([]);
  const [timeRange, setTimeRange] = useState<'1m' | '5m' | '15m' | '30m' | '1h' | 'all'>('5m');

  const loadStats = async () => {
    try {
      setError(null);
      // Use Promise.allSettled to show partial data if one endpoint fails
      const results = await Promise.allSettled([
        api.getBridgeStats(),
        api.getGPULaneStats(),
        api.getChainStats(),
      ]);

      // Process results - extract fulfilled values or use defaults for rejected
      const bridgeStatsData = results[0].status === 'fulfilled' ? results[0].value : null;
      const gpuData = results[1].status === 'fulfilled' ? results[1].value : [];
      const chainData = results[2].status === 'fulfilled' ? results[2].value : null;

      // Log any failures for debugging
      results.forEach((result, index) => {
        if (result.status === 'rejected') {
          const endpoints = ['bridge stats', 'GPU lane stats', 'chain stats'];
          console.error(`Failed to fetch ${endpoints[index]}:`, result.reason);
        }
      });
      
      if (bridgeStatsData) {
        setBridgeStats(bridgeStatsData);
        
        // Update TPS history — keep up to 10 minutes of data (300 points at 2s)
        setTpsHistory(prev => {
          const now = Date.now();
          const newPoint = {
            time: new Date(now).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' }),
            ts: now,
            tps: Math.round(bridgeStatsData.current_tps),
            forwarded: bridgeStatsData.total_forwarded,
            received: bridgeStatsData.total_received,
          };
          return [...prev.slice(-MAX_TPS_HISTORY_POINTS), newPoint];
        });
      }
      
      setGpuLanes(gpuData);
      if (chainData) setChainStats(chainData);
      
      // Try validator stats (needs auth, may fail)
      try {
        const validatorStats = await api.getStats();
        setStats(validatorStats);
      } catch (err) {
        // JWT may be expired, still show bridge/GPU data
        console.warn('Failed to fetch validator stats:', err);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : 'Failed to load dashboard stats';
      setError(errorMsg);
      console.error('Failed to load stats:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, TPS_COLLECTION_INTERVAL_MS);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center" role="status" aria-live="polite" aria-label="Loading dashboard">
        <div className="text-center">
          <RefreshCw className="w-12 h-12 text-blue-400 animate-spin mx-auto mb-4" aria-hidden="true" />
          <p className="text-gray-400">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen p-6">
      <div className="max-w-7xl mx-auto">
        {/* Error Banner */}
        <ErrorBanner error={error} onDismiss={() => setError(null)} />

        {/* Header */}
        <DashboardHeader onLogout={onLogout} onAdmin={onAdmin} onLeaderboard={onLeaderboard} />

        {/* Stats Grid */}
        <StatsGrid stats={stats} bridgeStats={bridgeStats} />

        {/* TPS Chart */}
        <TpsChart tpsHistory={tpsHistory} timeRange={timeRange} onTimeRangeChange={setTimeRange} />

        {/* GPU Lanes Section */}
        <GPULanesSection gpuLanes={gpuLanes} />

        {/* Solana Chain Section */}
        <SolanaChainSection chainStats={chainStats} />

        {/* Details Grid */}
        <DetailsGrid stats={stats} bridgeStats={bridgeStats} />

        {/* Quick Actions */}
        <div className="card mt-6">
          <h3 className="text-lg font-bold text-white mb-4">Quick Start</h3>
          <div className="bg-gray-900/50 rounded-lg p-4 font-mono text-sm">
            <p className="text-gray-400 mb-2"># Test your acceleration:</p>
            <div className="text-green-400 whitespace-pre-wrap">
              {`curl -X POST http://localhost:9999/accelerate \\
  -H "X-API-Key: ${api.getApiKey()}" \\
  -d '{"tx_hash":"test","tx_data":"48656c6c6f","chain":"${stats?.chain}"}'`}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
