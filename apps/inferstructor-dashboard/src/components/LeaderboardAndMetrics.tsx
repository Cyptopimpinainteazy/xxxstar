import { lazy, Suspense, useEffect, useState } from 'react';
import { AlertCircle, Loader } from 'lucide-react';
import { api } from '../api';
import type { Subscriber } from '../api';

interface RankEntry {
  rank: number;
  validatorId: string;
  name: string;
  chain: string;
  tps: number;
  latency: number;
  uptime: number;
  gasEfficiency: number;
}

interface MetricsSnapshot {
  timestamp: string;
  avgTps: number;
  avgLatency: number;
  avgUptime: number;
  avgGasEfficiency: number;
}

const SummaryMetricsPanel = lazy(() => import('./LeaderboardAndMetricsPanels').then(module => ({ default: module.SummaryMetricsPanel })));
const SnapshotsPanel = lazy(() => import('./LeaderboardAndMetricsPanels').then(module => ({ default: module.SnapshotsPanel })));
const RankingsPanel = lazy(() => import('./LeaderboardAndMetricsPanels').then(module => ({ default: module.RankingsPanel })));

export function LeaderboardAndMetrics() {
  const [rankings, setRankings] = useState<RankEntry[]>([]);
  const [snapshots, setSnapshots] = useState<MetricsSnapshot[]>([]);
  
  const [sortBy, setSortBy] = useState<'tps' | 'latency' | 'uptime' | 'gasEfficiency'>('tps');
  const [filterChain, setFilterChain] = useState<'all' | 'Ethereum' | 'Solana'>('all');
  const [adminOverride, setAdminOverride] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load metrics data on mount
  useEffect(() => {
    loadMetricsData();
  }, []);

  const loadMetricsData = async () => {
    try {
      setLoading(true);
      setError(null);

      // Fetch subscribers for rankings
      const subscribersData = await api.getSubscribers().catch(() => ({ subscribers: [], total: 0 }));
      
      // Transform subscribers into rankings with mock performance metrics
      if (subscribersData.subscribers && Array.isArray(subscribersData.subscribers)) {
        const transformed: RankEntry[] = subscribersData.subscribers
          .map((sub: Subscriber, idx: number) => ({
            rank: idx + 1,
            validatorId: sub.validator_id,
            name: `Validator ${sub.validator_id.substring(0, 8)}`,
            chain: sub.chain,
            tps: sub.max_tps - Math.floor(Math.random() * 100), // Simulated performance
            latency: 30 + Math.floor(Math.random() * 50),
            uptime: 98 + Math.random() * 2,
            gasEfficiency: 80 + Math.random() * 20,
          }))
          .sort((a, b) => b.tps - a.tps) // Sort by TPS
          .map((entry, idx) => ({ ...entry, rank: idx + 1 }));
        
        setRankings(transformed);
      } else {
        // Fallback mock data
        setRankings([
          {
            rank: 1,
            validatorId: 'val-eth-001',
            name: 'EthPro Validator',
            chain: 'Ethereum',
            tps: 450,
            latency: 45,
            uptime: 99.98,
            gasEfficiency: 92.5,
          },
          {
            rank: 2,
            validatorId: 'val-sol-001',
            name: 'SolSpeed Validator',
            chain: 'Solana',
            tps: 2800,
            latency: 32,
            uptime: 99.87,
            gasEfficiency: 88.3,
          },
          {
            rank: 3,
            validatorId: 'val-eth-002',
            name: 'SecureValidator',
            chain: 'Ethereum',
            tps: 420,
            latency: 52,
            uptime: 99.92,
            gasEfficiency: 85.7,
          },
          {
            rank: 4,
            validatorId: 'val-sol-002',
            name: 'FastNode',
            chain: 'Solana',
            tps: 2650,
            latency: 38,
            uptime: 98.5,
            gasEfficiency: 81.2,
          },
        ]);
      }

      // Fetch metrics history for snapshots
      const metricsData = await api.getAdminMetricsHistory(3600).catch(() => ({ points: [] }));
      
      if (metricsData.points && Array.isArray(metricsData.points) && metricsData.points.length > 0) {
         const snapshots: MetricsSnapshot[] = metricsData.points.slice(0, 10).map((point: any) => ({
          timestamp: new Date(point.timestamp || Date.now()).toLocaleString('en-US', { 
            year: 'numeric', 
            month: '2-digit', 
            day: '2-digit', 
            hour: '2-digit', 
            minute: '2-digit' 
          }),
          avgTps: point.aggregated?.[0]?.tps || 1500 + Math.floor(Math.random() * 200),
          avgLatency: point.aggregated?.[0]?.latency || 40 + Math.floor(Math.random() * 10),
          avgUptime: point.aggregated?.[0]?.uptime || 99.8 + Math.random() * 0.2,
          avgGasEfficiency: point.aggregated?.[0]?.gas_efficiency || 85 + Math.random() * 5,
        }));
        setSnapshots(snapshots);
      } else {
        // Fallback mock data
        setSnapshots([
          { timestamp: '2024-04-06 10:00', avgTps: 1550, avgLatency: 42, avgUptime: 99.8, avgGasEfficiency: 87.0 },
          { timestamp: '2024-04-06 11:00', avgTps: 1620, avgLatency: 41, avgUptime: 99.85, avgGasEfficiency: 88.2 },
          { timestamp: '2024-04-06 12:00', avgTps: 1580, avgLatency: 43, avgUptime: 99.82, avgGasEfficiency: 86.5 },
        ]);
      }

      // Load persisted snapshots from localStorage
      const stored = localStorage.getItem('metricsSnapshots');
      if (stored) {
        try {
          const parsed = JSON.parse(stored);
          setSnapshots(prev => [...prev, ...parsed]);
        } catch (e) {
          console.error('Failed to load stored snapshots:', e);
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load metrics data');
    } finally {
      setLoading(false);
    }
  };

  // Save snapshots to localStorage whenever they change
  useEffect(() => {
    localStorage.setItem('metricsSnapshots', JSON.stringify(snapshots));
  }, [snapshots]);

  const handleAddSnapshot = () => {
    const now = new Date();
    const timestamp = now.toLocaleString('en-US', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
    
    const newSnapshot: MetricsSnapshot = {
      timestamp,
      avgTps: Math.floor(1500 + Math.random() * 200),
      avgLatency: Math.floor(40 + Math.random() * 10),
      avgUptime: 99.8 + Math.random() * 0.2,
      avgGasEfficiency: 85 + Math.random() * 5,
    };
    setSnapshots([...snapshots, newSnapshot]);
  };

  const handleExportCSV = () => {
    const headers = ['Timestamp', 'Avg TPS', 'Avg Latency (ms)', 'Avg Uptime (%)', 'Gas Efficiency (%)'];
    const rows = snapshots.map((s) => [
      s.timestamp,
      s.avgTps,
      s.avgLatency,
      s.avgUptime.toFixed(2),
      s.avgGasEfficiency.toFixed(1),
    ]);
    
    const csv = [headers, ...rows].map((row) => row.join(',')).join('\n');
    const blob = new Blob([csv], { type: 'text/csv' });
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `metrics-export-${new Date().toISOString().split('T')[0]}.csv`;
    a.click();
    window.URL.revokeObjectURL(url);
  };

  const filteredRankings = rankings
    .filter((r) => filterChain === 'all' || r.chain === filterChain)
    .sort((a, b) => {
      if (sortBy === 'tps') return b.tps - a.tps;
      if (sortBy === 'latency') return a.latency - b.latency;
      if (sortBy === 'uptime') return b.uptime - a.uptime;
      if (sortBy === 'gasEfficiency') return b.gasEfficiency - a.gasEfficiency;
      return 0;
    });

  const renderPanelFallback = (label: string) => (
    <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6 mb-8 text-sm text-gray-400">
      {label} loading...
    </div>
  );

  return (
    <div className="px-6">
      <div className="max-w-6xl">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-white mb-2">Leaderboard & Metrics</h1>
          <p className="text-gray-400">Chain and validator performance rankings with real-time metrics</p>
        </div>

        {/* Error Banner */}
        {error && (
          <div className="mb-6 p-4 bg-red-900/20 border border-red-700 rounded-lg flex items-start gap-3">
            <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
            <div>
              <p className="text-red-300 font-medium">Error</p>
              <p className="text-red-200 text-sm">{error}</p>
            </div>
            <button
              onClick={() => setError(null)}
              className="ml-auto text-red-400 hover:text-red-300"
            >
              ✕
            </button>
          </div>
        )}

        {/* Loading State */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <Loader className="w-6 h-6 text-blue-400 animate-spin" />
            <span className="ml-2 text-gray-400">Loading metrics...</span>
          </div>
        )}

        {!loading && (
          <>
        <Suspense fallback={renderPanelFallback('Summary metrics')}>
          <SummaryMetricsPanel snapshots={snapshots} />
        </Suspense>

        <Suspense fallback={renderPanelFallback('Snapshots panel')}>
          <SnapshotsPanel
            snapshots={snapshots}
            adminOverride={adminOverride}
            onExportCSV={handleExportCSV}
            onAddSnapshot={handleAddSnapshot}
            onSetAdminOverride={setAdminOverride}
          />
        </Suspense>

        <Suspense fallback={renderPanelFallback('Rankings panel')}>
          <RankingsPanel
            filteredRankings={filteredRankings}
            sortBy={sortBy}
            filterChain={filterChain}
            onSetSortBy={setSortBy}
            onSetFilterChain={setFilterChain}
          />
        </Suspense>
          </>
        )}
      </div>
    </div>
  );
}
