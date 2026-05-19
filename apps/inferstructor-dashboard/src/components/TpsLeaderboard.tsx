import { lazy, Suspense, useState, useEffect, useCallback } from 'react';
import { api } from '../api';
import type { TpsLeaderboardEntry, TpsLeaderboardResponse, TpsBenchmarkStatus, RpcPoolStats } from '../api';
import { ArrowLeft, Trophy, Zap, Clock, Globe, TrendingUp, Activity, Timer, ChevronUp, ChevronDown, RefreshCw, Flame } from 'lucide-react';

interface TpsLeaderboardProps {
  onBack: () => void;
}

type SortField = 'tps_current' | 'tps_peak' | 'tps_theoretical' | 'latency' | 'finality';
type Category = 'chain' | 'ecosystem' | 'provider';

const ECOSYSTEM_COLORS: Record<string, string> = {
  x3: '#FFD700',
  evm: '#627EEA',
  svm: '#14F195',
  cosmos: '#6F7390',
  substrate: '#E6007A',
  move: '#4FC1FF',
  other: '#888',
};

const ECOSYSTEM_LABELS: Record<string, string> = {
  x3: 'X3 Chain',
  evm: 'EVM',
  svm: 'Solana VM',
  cosmos: 'Cosmos',
  substrate: 'Substrate',
  move: 'Move',
  other: 'Other',
};

const TpsLeaderboardChart = lazy(() => import('./TpsLeaderboardChart').then(module => ({ default: module.TpsLeaderboardChart })));

export function TpsLeaderboard({ onBack }: TpsLeaderboardProps) {
  const [data, setData] = useState<TpsLeaderboardResponse | null>(null);
  const [benchStatus, setBenchStatus] = useState<TpsBenchmarkStatus | null>(null);
  const [rpcStats, setRpcStats] = useState<RpcPoolStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [category, setCategory] = useState<Category>('chain');
  const [sortField, setSortField] = useState<SortField>('tps_current');
  const [sortOrder, setSortOrder] = useState<'desc' | 'asc'>('desc');
  const [ecosystem, setEcosystem] = useState<string>('');
  const [limit] = useState(100);
  const [adPulse, setAdPulse] = useState(true);

  // Pulse the AD banner
  useEffect(() => {
    const iv = setInterval(() => setAdPulse(p => !p), 2000);
    return () => clearInterval(iv);
  }, []);

  const loadData = useCallback(async () => {
    try {
      const [lb, status, rpc] = await Promise.all([
        api.getTpsLeaderboard({ category, sort: sortField, order: sortOrder, ecosystem: ecosystem || undefined, limit }),
        api.getTpsBenchmarkStatus().catch(() => null),
        api.getRpcStats().catch(() => null),
      ]);
      setData(lb);
      if (status) setBenchStatus(status);
      if (rpc) setRpcStats(rpc);
    } catch (err) {
      console.error('Failed to load leaderboard:', err);
    } finally {
      setLoading(false);
    }
  }, [category, sortField, sortOrder, ecosystem, limit]);

  useEffect(() => {
    setLoading(true);
    loadData();
    const iv = setInterval(loadData, 15000);
    return () => clearInterval(iv);
  }, [loadData]);

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortOrder(o => o === 'desc' ? 'asc' : 'desc');
    } else {
      setSortField(field);
      setSortOrder(field === 'latency' || field === 'finality' ? 'asc' : 'desc');
    }
  };

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortField !== field) return <ChevronDown className="w-3 h-3 opacity-30" />;
    return sortOrder === 'desc'
      ? <ChevronDown className="w-3 h-3 text-blue-400" />
      : <ChevronUp className="w-3 h-3 text-blue-400" />;
  };

  const getMedal = (i: number) => {
    if (i === 0) return '🥇';
    if (i === 1) return '🥈';
    if (i === 2) return '🥉';
    return `${i + 1}`;
  };

  const formatTps = (tps: number) => {
    if (tps >= 100000) return `${(tps / 1000).toFixed(0)}K`;
    if (tps >= 1000) return `${(tps / 1000).toFixed(1)}K`;
    return tps.toFixed(1);
  };

  const getSpeedClass = (tps: number) => {
    if (tps >= 1000) return 'text-green-400';
    if (tps >= 100) return 'text-blue-400';
    if (tps >= 10) return 'text-yellow-400';
    return 'text-gray-400';
  };

  // Top chart data — show top 20 chains by TPS
  const chartData = (data?.leaderboard || [])
    .filter((e: any) => (e.tps_current || 0) > 0)
    .slice(0, 20)
    .map((e: any) => ({
      name: (e.chain_name || e.chain_id || e.ecosystem || e.provider || '').slice(0, 12),
      tps: e.tps_current || e.avg_tps || e.total_rps || 0,
      color: ECOSYSTEM_COLORS[e.ecosystem] || '#3B82F6',
    }));

  return (
    <div className="min-h-screen p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-4">
            <button onClick={onBack} className="flex items-center gap-2 px-3 py-2 text-gray-400 hover:text-white bg-gray-800/50 hover:bg-gray-700/50 rounded-lg transition-colors">
              <ArrowLeft className="w-4 h-4" />
              Back
            </button>
            <div>
              <h1 className="text-3xl font-bold text-white flex items-center gap-3">
                <Trophy className="w-8 h-8 text-yellow-400" />
                TPS Leaderboard
              </h1>
              <p className="text-gray-400 text-sm mt-1">Real-time chain performance rankings across {rpcStats?.chains_covered?.toLocaleString() || '62,000+'} chains</p>
            </div>
          </div>
          <button onClick={loadData} className="flex items-center gap-2 px-4 py-2 bg-blue-600/20 hover:bg-blue-600/30 text-blue-400 rounded-lg transition-colors">
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
        </div>

        {/* AD-style Speed Banner */}
        <div className={`mb-6 rounded-xl border overflow-hidden transition-all duration-1000 ${adPulse ? 'border-yellow-500/60 shadow-lg shadow-yellow-500/10' : 'border-yellow-600/30'}`}
          style={{ background: 'linear-gradient(135deg, #1a1100 0%, #0d1117 40%, #0a1628 100%)' }}>
          <div className="px-6 py-5 flex items-center gap-6">
            <div className="flex-shrink-0">
              <div className={`text-5xl transition-transform duration-1000 ${adPulse ? 'scale-110' : 'scale-100'}`}>⚡</div>
            </div>
            <div className="flex-1">
              <div className="flex items-center gap-2 mb-1">
                <span className="text-yellow-400 font-bold text-lg">SPEED TEST YOUR CHAINS</span>
                <span className={`inline-block w-2 h-2 rounded-full transition-colors duration-1000 ${adPulse ? 'bg-green-400' : 'bg-green-600'}`}></span>
                <span className="text-green-400 text-xs font-semibold">LIVE</span>
              </div>
              <p className="text-gray-300 text-sm">
                Only takes a couple minutes to see your speed get faster. We dial in these TPS. 
                <span className="text-yellow-400 font-semibold"> Optimize the hell out of them</span> then run all chains and post the leaderboard.
              </p>
              <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
                <span className="flex items-center gap-1"><Flame className="w-3 h-3 text-orange-400" /> {benchStatus?.measured || 0} chains benchmarked</span>
                <span className="flex items-center gap-1"><Timer className="w-3 h-3 text-blue-400" /> Updates every 15s</span>
                <span className="flex items-center gap-1"><Globe className="w-3 h-3 text-purple-400" /> {rpcStats?.total_endpoints?.toLocaleString() || '62K+'} endpoints</span>
              </div>
            </div>
            <div className="flex-shrink-0 text-right">
              <div className="text-3xl font-black text-white font-mono">
                {data?.stats?.max_tps_all ? formatTps(data.stats.max_tps_all) : '—'}
              </div>
              <div className="text-xs text-gray-400">Peak TPS</div>
            </div>
          </div>
          {/* Progress bar */}
          {benchStatus && benchStatus.total > 0 && (
            <div className="px-6 pb-3">
              <div className="flex items-center justify-between text-xs text-gray-500 mb-1">
                <span>Benchmark Progress</span>
                <span>{benchStatus.progress_pct}% ({benchStatus.measured}/{benchStatus.total})</span>
              </div>
              <div className="w-full h-1.5 bg-gray-800 rounded-full overflow-hidden">
                <div className="h-full bg-gradient-to-r from-yellow-500 via-green-400 to-blue-500 rounded-full transition-all duration-1000"
                  style={{ width: `${benchStatus.progress_pct}%` }} />
              </div>
            </div>
          )}
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4 mb-6">
          <div className="card !p-4">
            <div className="flex items-center gap-2 mb-1">
              <Zap className="w-4 h-4 text-yellow-400" />
              <span className="text-xs text-gray-400">Chains Measured</span>
            </div>
            <p className="text-xl font-bold text-white">{data?.stats?.total_chains_measured?.toLocaleString() || '0'}</p>
          </div>
          <div className="card !p-4">
            <div className="flex items-center gap-2 mb-1">
              <TrendingUp className="w-4 h-4 text-green-400" />
              <span className="text-xs text-gray-400">Avg TPS</span>
            </div>
            <p className="text-xl font-bold text-white">{data?.stats?.avg_tps_all ? formatTps(data.stats.avg_tps_all) : '—'}</p>
          </div>
          <div className="card !p-4">
            <div className="flex items-center gap-2 mb-1">
              <Trophy className="w-4 h-4 text-yellow-400" />
              <span className="text-xs text-gray-400">Max TPS</span>
            </div>
            <p className="text-xl font-bold text-green-400">{data?.stats?.max_tps_all ? formatTps(data.stats.max_tps_all) : '—'}</p>
          </div>
          <div className="card !p-4">
            <div className="flex items-center gap-2 mb-1">
              <Activity className="w-4 h-4 text-blue-400" />
              <span className="text-xs text-gray-400">Peak TPS</span>
            </div>
            <p className="text-xl font-bold text-blue-400">{data?.stats?.peak_tps_all ? formatTps(data.stats.peak_tps_all) : '—'}</p>
          </div>
          <div className="card !p-4">
            <div className="flex items-center gap-2 mb-1">
              <Globe className="w-4 h-4 text-purple-400" />
              <span className="text-xs text-gray-400">Endpoints</span>
            </div>
            <p className="text-xl font-bold text-white">{rpcStats?.total_endpoints?.toLocaleString() || '—'}</p>
          </div>
          <div className="card !p-4">
            <div className="flex items-center gap-2 mb-1">
              <Clock className="w-4 h-4 text-orange-400" />
              <span className="text-xs text-gray-400">Last Updated</span>
            </div>
            <p className="text-sm font-bold text-white">{benchStatus?.last_updated ? new Date(benchStatus.last_updated).toLocaleTimeString() : '—'}</p>
          </div>
        </div>

        {/* Category + Ecosystem Filter */}
        <div className="flex flex-wrap items-center gap-3 mb-4">
          <div className="flex bg-gray-900/60 rounded-lg p-1 gap-1">
            {(['chain', 'ecosystem', 'provider'] as Category[]).map(cat => (
              <button key={cat} onClick={() => setCategory(cat)}
                className={`px-4 py-1.5 rounded text-xs font-semibold transition-all capitalize ${category === cat ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white hover:bg-gray-700/50'}`}>
                {cat === 'chain' ? '⛓️ By Chain' : cat === 'ecosystem' ? '🌐 By Ecosystem' : '🏢 By Provider'}
              </button>
            ))}
          </div>
          
          {category === 'chain' && (
            <div className="flex bg-gray-900/60 rounded-lg p-1 gap-1">
              <button onClick={() => setEcosystem('')}
                className={`px-3 py-1.5 rounded text-xs font-semibold transition-all ${!ecosystem ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-white'}`}>
                All
              </button>
              {Object.entries(ECOSYSTEM_LABELS).map(([key, label]) => (
                <button key={key} onClick={() => setEcosystem(key)}
                  className={`px-3 py-1.5 rounded text-xs font-semibold transition-all ${ecosystem === key ? 'text-white' : 'text-gray-400 hover:text-white'}`}
                  style={ecosystem === key ? { background: ECOSYSTEM_COLORS[key] + '33' } : {}}>
                  {label}
                </button>
              ))}
            </div>
          )}
          
          <span className="ml-auto text-xs text-gray-500">
            {data?.total?.toLocaleString() || 0} results
          </span>
        </div>

        {/* TPS Bar Chart — Top 20 */}
        {chartData.length > 0 && (
          <Suspense
            fallback={(
              <div className="card mb-6">
                <div className="flex items-center gap-2 text-sm text-gray-300">
                  <RefreshCw className="h-4 w-4 animate-spin text-blue-400" />
                  Loading TPS chart...
                </div>
              </div>
            )}
          >
            <TpsLeaderboardChart chartData={chartData} />
          </Suspense>
        )}

        {/* Leaderboard Table */}
        <div className="card overflow-hidden">
          {loading && !data ? (
            <div className="flex items-center justify-center py-20">
              <RefreshCw className="w-8 h-8 text-blue-400 animate-spin" />
              <span className="ml-3 text-gray-400">Loading leaderboard...</span>
            </div>
          ) : category === 'chain' ? (
            /* Chain leaderboard */
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-700 text-gray-400 text-xs uppercase tracking-wider">
                    <th className="px-4 py-3 text-left w-12">#</th>
                    <th className="px-4 py-3 text-left">Chain</th>
                    <th className="px-4 py-3 text-left">Ecosystem</th>
                    <th className="px-4 py-3 text-right cursor-pointer hover:text-white select-none" onClick={() => toggleSort('tps_current')}>
                      <span className="inline-flex items-center gap-1">Current TPS <SortIcon field="tps_current" /></span>
                    </th>
                    <th className="px-4 py-3 text-right cursor-pointer hover:text-white select-none" onClick={() => toggleSort('tps_peak')}>
                      <span className="inline-flex items-center gap-1">Peak <SortIcon field="tps_peak" /></span>
                    </th>
                    <th className="px-4 py-3 text-right cursor-pointer hover:text-white select-none" onClick={() => toggleSort('tps_theoretical')}>
                      <span className="inline-flex items-center gap-1">Theoretical <SortIcon field="tps_theoretical" /></span>
                    </th>
                    <th className="px-4 py-3 text-right cursor-pointer hover:text-white select-none" onClick={() => toggleSort('latency')}>
                      <span className="inline-flex items-center gap-1">Latency <SortIcon field="latency" /></span>
                    </th>
                    <th className="px-4 py-3 text-right cursor-pointer hover:text-white select-none" onClick={() => toggleSort('finality')}>
                      <span className="inline-flex items-center gap-1">Finality <SortIcon field="finality" /></span>
                    </th>
                    <th className="px-4 py-3 text-right">Endpoints</th>
                  </tr>
                </thead>
                <tbody>
                  {(data?.leaderboard || []).map((entry: TpsLeaderboardEntry, i: number) => (
                    <tr key={entry.chain_id} className="border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors">
                      <td className="px-4 py-3 text-center font-bold text-lg">
                        {getMedal(i)}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-white">{entry.chain_name || entry.chain_id}</span>
                          {entry.is_testnet === 1 && <span className="px-1.5 py-0.5 rounded text-[10px] bg-yellow-500/20 text-yellow-400">TEST</span>}
                          {entry.native_token && <span className="text-xs text-gray-500">{entry.native_token}</span>}
                        </div>
                        <div className="text-xs text-gray-500 mt-0.5">{entry.chain_type}</div>
                      </td>
                      <td className="px-4 py-3">
                        <span className="px-2 py-1 rounded text-xs font-semibold"
                          style={{ background: (ECOSYSTEM_COLORS[entry.ecosystem] || '#888') + '22', color: ECOSYSTEM_COLORS[entry.ecosystem] || '#888' }}>
                          {ECOSYSTEM_LABELS[entry.ecosystem] || entry.ecosystem}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-right">
                        <span className={`font-bold font-mono ${getSpeedClass(entry.tps_current)}`}>
                          {formatTps(entry.tps_current)}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-right font-mono text-gray-300">
                        {entry.tps_peak > 0 ? formatTps(entry.tps_peak) : '—'}
                      </td>
                      <td className="px-4 py-3 text-right font-mono text-gray-500">
                        {entry.tps_theoretical > 0 ? formatTps(entry.tps_theoretical) : '—'}
                      </td>
                      <td className="px-4 py-3 text-right font-mono">
                        {entry.best_latency_ms ? (
                          <span className={entry.best_latency_ms < 50 ? 'text-green-400' : entry.best_latency_ms < 200 ? 'text-yellow-400' : 'text-red-400'}>
                            {entry.best_latency_ms}ms
                          </span>
                        ) : '—'}
                      </td>
                      <td className="px-4 py-3 text-right font-mono">
                        {entry.finality_seconds > 0 ? (
                          <span className={entry.finality_seconds < 2 ? 'text-green-400' : entry.finality_seconds < 15 ? 'text-yellow-400' : 'text-orange-400'}>
                            {entry.finality_seconds < 1 ? `${(entry.finality_seconds * 1000).toFixed(0)}ms` : `${entry.finality_seconds.toFixed(1)}s`}
                          </span>
                        ) : '—'}
                      </td>
                      <td className="px-4 py-3 text-right text-gray-400 font-mono">
                        {entry.endpoint_count}
                      </td>
                    </tr>
                  ))}
                  {(!data?.leaderboard || data.leaderboard.length === 0) && (
                    <tr>
                      <td colSpan={9} className="px-4 py-16 text-center">
                        <div className="text-4xl mb-3">🏁</div>
                        <div className="text-gray-400 font-semibold mb-2">No benchmark data yet</div>
                        <div className="text-gray-500 text-xs max-w-md mx-auto">
                          Run the TPS benchmark to populate the leaderboard:
                          <code className="block mt-2 p-2 bg-gray-900 rounded text-green-400 text-xs">
                            python3 infra-structure/services/rpc-crawler/tps_benchmark.py --top 200
                          </code>
                        </div>
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          ) : category === 'ecosystem' ? (
            /* Ecosystem leaderboard */
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-700 text-gray-400 text-xs uppercase tracking-wider">
                    <th className="px-4 py-3 text-left w-12">#</th>
                    <th className="px-4 py-3 text-left">Ecosystem</th>
                    <th className="px-4 py-3 text-right">Chains</th>
                    <th className="px-4 py-3 text-right">Avg TPS</th>
                    <th className="px-4 py-3 text-right">Max TPS</th>
                    <th className="px-4 py-3 text-right">Peak TPS</th>
                    <th className="px-4 py-3 text-right">Avg Latency</th>
                    <th className="px-4 py-3 text-right">Best Latency</th>
                    <th className="px-4 py-3 text-right">Endpoints</th>
                  </tr>
                </thead>
                <tbody>
                  {(data?.leaderboard || []).map((entry: any, i: number) => (
                    <tr key={entry.ecosystem} className="border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors">
                      <td className="px-4 py-3 text-center font-bold text-lg">{getMedal(i)}</td>
                      <td className="px-4 py-3">
                        <span className="px-3 py-1.5 rounded text-sm font-bold"
                          style={{ background: (ECOSYSTEM_COLORS[entry.ecosystem] || '#888') + '22', color: ECOSYSTEM_COLORS[entry.ecosystem] || '#888' }}>
                          {ECOSYSTEM_LABELS[entry.ecosystem] || entry.ecosystem}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-right font-mono text-white">{entry.chain_count?.toLocaleString()}</td>
                      <td className="px-4 py-3 text-right font-mono font-bold text-green-400">{entry.avg_tps ? formatTps(entry.avg_tps) : '—'}</td>
                      <td className="px-4 py-3 text-right font-mono text-blue-400">{entry.max_tps ? formatTps(entry.max_tps) : '—'}</td>
                      <td className="px-4 py-3 text-right font-mono text-gray-300">{entry.peak_tps ? formatTps(entry.peak_tps) : '—'}</td>
                      <td className="px-4 py-3 text-right font-mono text-gray-400">{entry.avg_latency_ms ? `${Math.round(entry.avg_latency_ms)}ms` : '—'}</td>
                      <td className="px-4 py-3 text-right font-mono text-green-400">{entry.best_latency_ms ? `${Math.round(entry.best_latency_ms)}ms` : '—'}</td>
                      <td className="px-4 py-3 text-right font-mono text-gray-400">{entry.total_endpoints?.toLocaleString()}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            /* Provider leaderboard */
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-700 text-gray-400 text-xs uppercase tracking-wider">
                    <th className="px-4 py-3 text-left w-12">#</th>
                    <th className="px-4 py-3 text-left">Provider</th>
                    <th className="px-4 py-3 text-right">Endpoints</th>
                    <th className="px-4 py-3 text-right">Chains</th>
                    <th className="px-4 py-3 text-right">Total RPS</th>
                    <th className="px-4 py-3 text-right">Avg Latency</th>
                    <th className="px-4 py-3 text-right">Best Latency</th>
                    <th className="px-4 py-3 text-right">Total Requests</th>
                    <th className="px-4 py-3 text-right">Avg Chain TPS</th>
                  </tr>
                </thead>
                <tbody>
                  {(data?.leaderboard || []).map((entry: any, i: number) => (
                    <tr key={entry.provider} className="border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors">
                      <td className="px-4 py-3 text-center font-bold text-lg">{getMedal(i)}</td>
                      <td className="px-4 py-3 font-semibold text-white capitalize">{entry.provider}</td>
                      <td className="px-4 py-3 text-right font-mono text-white">{entry.endpoint_count?.toLocaleString()}</td>
                      <td className="px-4 py-3 text-right font-mono text-gray-300">{entry.chains_covered?.toLocaleString()}</td>
                      <td className="px-4 py-3 text-right font-mono font-bold text-blue-400">{entry.total_rps?.toLocaleString()}</td>
                      <td className="px-4 py-3 text-right font-mono text-gray-400">{entry.avg_latency_ms ? `${Math.round(entry.avg_latency_ms)}ms` : '—'}</td>
                      <td className="px-4 py-3 text-right font-mono text-green-400">{entry.best_latency_ms ? `${Math.round(entry.best_latency_ms)}ms` : '—'}</td>
                      <td className="px-4 py-3 text-right font-mono text-gray-400">{entry.total_requests?.toLocaleString()}</td>
                      <td className="px-4 py-3 text-right font-mono text-green-400">{entry.avg_chain_tps ? formatTps(entry.avg_chain_tps) : '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* RPC Pool Stats Footer */}
        {rpcStats && (
          <div className="mt-6 card">
            <h3 className="text-sm font-semibold text-gray-400 mb-4 uppercase tracking-wider">💰 Gas Savings — Your RPC Pool</h3>
            <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
              <div>
                <div className="text-xs text-gray-500 mb-1">Healthy Endpoints</div>
                <div className="text-lg font-bold text-white">{rpcStats.healthy_endpoints.toLocaleString()}</div>
              </div>
              <div>
                <div className="text-xs text-gray-500 mb-1">Combined RPS</div>
                <div className="text-lg font-bold text-blue-400">{rpcStats.combined_rps.toLocaleString()}</div>
              </div>
              <div>
                <div className="text-xs text-gray-500 mb-1">Avg Latency</div>
                <div className="text-lg font-bold text-yellow-400">{rpcStats.avg_latency_ms}ms</div>
              </div>
              <div>
                <div className="text-xs text-gray-500 mb-1">Monthly Saved</div>
                <div className="text-lg font-bold text-green-400">${rpcStats.gas_savings.total_monthly_saved.toLocaleString()}</div>
              </div>
              <div>
                <div className="text-xs text-gray-500 mb-1">Your Cost</div>
                <div className="text-lg font-bold text-green-400">$0 🎯</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
