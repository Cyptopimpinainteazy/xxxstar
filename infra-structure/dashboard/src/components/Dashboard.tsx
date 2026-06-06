import { useState, useEffect } from 'react';
import { api } from '../api';
import type { ValidatorStats, BridgeStats, GPULaneHealth, ChainStats } from '../api';
import { Activity, Zap, TrendingUp, Clock, Server, LogOut, RefreshCw, Cpu, Globe, Link, Shield, Database } from 'lucide-react';
import { LineChart, Line, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';

interface DashboardProps {
  onLogout: () => void;
  onAdmin?: () => void;
  onChains?: () => void;
}

export function Dashboard({ onLogout, onAdmin, onChains }: DashboardProps) {
  const [stats, setStats] = useState<ValidatorStats | null>(null);
  const [bridgeStats, setBridgeStats] = useState<BridgeStats | null>(null);
  const [gpuLanes, setGpuLanes] = useState<GPULaneHealth[]>([]);
  const [chainStats, setChainStats] = useState<ChainStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [tpsHistory, setTpsHistory] = useState<Array<{ time: string; ts: number; tps: number; forwarded: number; received: number }>>([]);
  const [timeRange, setTimeRange] = useState<'1m' | '5m' | '15m' | '30m' | '1h' | 'all'>('5m');
  const loadStats = async () => {
    try {
      // Fetch bridge stats and GPU lane stats independently (no auth needed)
      const [bridgeStatsData, gpuData, chainData] = await Promise.all([
        api.getBridgeStats().catch(() => null),
        api.getGPULaneStats().catch(() => []),
        api.getChainStats().catch(() => null),
      ]);
      
      if (bridgeStatsData) {
        setBridgeStats(bridgeStatsData);
        
        // Update TPS history — keep up to 1 hour of data (1800 points at 2s)
        setTpsHistory(prev => {
          const now = Date.now();
          const newPoint = {
            time: new Date(now).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' }),
            ts: now,
            tps: Math.round(bridgeStatsData.current_tps),
            forwarded: bridgeStatsData.total_forwarded,
            received: bridgeStatsData.total_received,
          };
          return [...prev.slice(-1800), newPoint];
        });
      }
      
      setGpuLanes(gpuData);
      if (chainData) setChainStats(chainData);
      
      // Try validator stats (needs auth, may fail)
      try {
        const validatorStats = await api.getStats();
        setStats(validatorStats);
      } catch {
        // JWT may be expired, still show bridge/GPU data
      }
    } catch (error) {
      console.error('Failed to load stats:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, 2000); // Update every 2s
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <RefreshCw className="w-12 h-12 text-blue-400 animate-spin mx-auto mb-4" />
          <p className="text-gray-400">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  const formatNumber = (num: number) => num.toLocaleString();
  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  const timeRangeMs: Record<string, number> = {
    '1m': 60_000,
    '5m': 5 * 60_000,
    '15m': 15 * 60_000,
    '30m': 30 * 60_000,
    '1h': 60 * 60_000,
  };

  const filteredHistory = timeRange === 'all'
    ? tpsHistory
    : tpsHistory.filter(p => p.ts >= Date.now() - timeRangeMs[timeRange]);

  const timeRangeOptions = [
    { key: '1m' as const, label: '1m' },
    { key: '5m' as const, label: '5m' },
    { key: '15m' as const, label: '15m' },
    { key: '30m' as const, label: '30m' },
    { key: '1h' as const, label: '1H' },
    { key: 'all' as const, label: 'ALL' },
  ];

  return (
    <div className="min-h-screen p-6">
      {/* Header */}
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold text-white mb-2">Inferstructor Dashboard</h1>
            <p className="text-gray-400">Validator: {api.getValidatorId()}</p>
          </div>
          <div className="flex items-center gap-3">
            {onChains && (
              <button
                onClick={onChains}
                className="flex items-center gap-2 px-4 py-2 text-blue-300 hover:text-white bg-blue-900/30 hover:bg-blue-800/40 border border-blue-700/50 rounded-lg transition-colors"
              >
                <Database className="w-4 h-4" />
                Chain Explorer
              </button>
            )}
            {onAdmin && (
              <button
                onClick={onAdmin}
                className="flex items-center gap-2 px-4 py-2 text-red-300 hover:text-white bg-red-900/30 hover:bg-red-800/40 border border-red-700/50 rounded-lg transition-colors"
              >
                <Shield className="w-4 h-4" />
                Admin
              </button>
            )}
            <button
              onClick={onLogout}
              className="flex items-center gap-2 px-4 py-2 text-gray-300 hover:text-white hover:bg-gray-800 rounded-lg transition-colors"
            >
              <LogOut className="w-4 h-4" />
              Logout
            </button>
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          {/* SLA Tier */}
          <div className="card">
            <div className="flex items-center justify-between mb-2">
              <Server className="w-5 h-5 text-blue-400" />
              <span className={`
                px-2 py-1 rounded text-xs font-semibold
                ${stats?.sla_tier === 'enterprise' ? 'bg-purple-500/20 text-purple-300' : ''}
                ${stats?.sla_tier === 'pro' ? 'bg-blue-500/20 text-blue-300' : ''}
                ${stats?.sla_tier === 'basic' ? 'bg-gray-500/20 text-gray-300' : ''}
              `}>
                {stats?.sla_tier?.toUpperCase()}
              </span>
            </div>
            <p className="text-2xl font-bold text-white">{formatNumber(stats?.max_tps || 0)}</p>
            <p className="text-sm text-gray-400">Max TPS</p>
          </div>

          {/* Current TPS */}
          <div className="card">
            <div className="flex items-center justify-between mb-2">
              <Zap className="w-5 h-5 text-yellow-400" />
              <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></span>
            </div>
            <p className="text-2xl font-bold text-white">{formatNumber(bridgeStats?.current_tps || 0)}</p>
            <p className="text-sm text-gray-400">Current TPS</p>
          </div>

          {/* Total Transactions */}
          <div className="card">
            <TrendingUp className="w-5 h-5 text-green-400 mb-2" />
            <p className="text-2xl font-bold text-white">{formatNumber(stats?.usage.total_tx || 0)}</p>
            <p className="text-sm text-gray-400">Total Transactions</p>
          </div>

          {/* Uptime */}
          <div className="card">
            <Activity className="w-5 h-5 text-blue-400 mb-2" />
            <p className="text-2xl font-bold text-white">{formatUptime(bridgeStats?.uptime_seconds || 0)}</p>
            <p className="text-sm text-gray-400">Uptime</p>
          </div>
        </div>

        {/* TPS Chart */}
        <div className="card mb-8">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-bold text-white">Real-Time TPS Performance</h2>
            <div className="flex items-center gap-3 text-sm">
              {filteredHistory.length > 0 && (
                <span className="text-blue-400 font-mono">
                  Peak: {Math.max(...filteredHistory.map(h => h.tps)).toLocaleString()} TPS
                </span>
              )}
              <span className="flex items-center gap-1 text-gray-500">
                <Clock className="w-3.5 h-3.5" />
                2s
              </span>
            </div>
          </div>

          {/* TradingView-style time range bar */}
          <div className="flex items-center gap-1 mb-4 bg-gray-900/60 rounded-lg p-1 w-fit">
            {timeRangeOptions.map(opt => (
              <button
                key={opt.key}
                onClick={() => setTimeRange(opt.key)}
                className={`px-3 py-1 rounded text-xs font-semibold transition-all ${
                  timeRange === opt.key
                    ? 'bg-blue-600 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
                }`}
              >
                {opt.label}
              </button>
            ))}
            <span className="text-gray-600 text-xs ml-2 font-mono">
              {filteredHistory.length} pts
            </span>
          </div>
          
          <div className="h-72">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={filteredHistory}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9CA3AF" fontSize={11} />
                <YAxis stroke="#9CA3AF" fontSize={11} tickFormatter={(v) => v >= 1000 ? `${(v/1000).toFixed(0)}K` : v} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1F2937',
                    border: '1px solid #374151',
                    borderRadius: '8px',
                    color: '#fff',
                  }}
                  formatter={(value: number | undefined) => [value?.toLocaleString() ?? '0', 'TPS']}
                />
                <Line
                  type="monotone"
                  dataKey="tps"
                  stroke="#3B82F6"
                  strokeWidth={2}
                  dot={false}
                  name="TPS"
                  isAnimationActive={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* GPU Lanes Status */}
        {gpuLanes.length > 0 && (
          <div className="card mb-8">
            <div className="flex items-center gap-2 mb-6">
              <Cpu className="w-5 h-5 text-green-400" />
              <h2 className="text-xl font-bold text-white">GPU Lanes ({gpuLanes.length} Active)</h2>
            </div>
            
            <div className="grid md:grid-cols-3 gap-4 mb-6">
              {gpuLanes.map((lane) => (
                <div key={lane.service} className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                  <div className="flex items-center justify-between mb-3">
                    <span className="text-white font-semibold capitalize">
                      {lane.service.replace('gpu-lane-', '')}
                    </span>
                    <span className={`px-2 py-0.5 rounded text-xs font-semibold ${
                      lane.gpu.available ? 'bg-green-500/20 text-green-300' : 'bg-red-500/20 text-red-300'
                    }`}>
                      GPU {lane.gpu.id}
                    </span>
                  </div>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Txns Processed</span>
                      <span className="text-white font-mono">{formatNumber(lane.stats.total_txns)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Success Rate</span>
                      <span className="text-green-400 font-mono">{(lane.stats.success_rate * 100).toFixed(1)}%</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">TPS (avg)</span>
                      <span className="text-blue-400 font-mono">{formatNumber(Math.round(lane.stats.txns_per_second))}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">VRAM Used</span>
                      <span className="text-yellow-400 font-mono">{lane.gpu.memory_used_mb.toFixed(0)} MB</span>
                    </div>
                  </div>
                  {/* Usage bar */}
                  <div className="mt-3">
                    <div className="w-full bg-gray-700 rounded-full h-1.5">
                      <div
                        className="bg-blue-500 h-1.5 rounded-full transition-all"
                        style={{ width: `${Math.min(lane.gpu.utilization, 100)}%` }}
                      ></div>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {/* GPU distribution bar chart */}
            <div className="h-48">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={gpuLanes.map(l => ({
                  name: l.service.replace('gpu-lane-', '').toUpperCase(),
                  txns: l.stats.total_txns,
                  tps: Math.round(l.stats.txns_per_second),
                }))}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                  <XAxis dataKey="name" stroke="#9CA3AF" fontSize={12} />
                  <YAxis stroke="#9CA3AF" fontSize={11} tickFormatter={(v) => v >= 1000000 ? `${(v/1000000).toFixed(1)}M` : v >= 1000 ? `${(v/1000).toFixed(0)}K` : v} />
                  <Tooltip
                    contentStyle={{ backgroundColor: '#1F2937', border: '1px solid #374151', borderRadius: '8px', color: '#fff' }}
                    formatter={(value: number | undefined) => [value?.toLocaleString() ?? '0']}
                  />
                  <Legend />
                  <Bar dataKey="txns" fill="#3B82F6" name="Total Txns" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>
        )}

        {/* Solana Chain Stats */}
        {chainStats && (
          <div className="card mb-8">
            <div className="flex items-center gap-2 mb-6">
              <Globe className="w-5 h-5 text-purple-400" />
              <h2 className="text-xl font-bold text-white">Solana Chain — Live</h2>
              <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></span>
              {chainStats.chain.version && (
                <span className="ml-auto text-xs text-gray-400 font-mono">
                  Solana Core {chainStats.chain.version['solana-core']}
                </span>
              )}
            </div>

            {/* Chain overview cards */}
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Slot</p>
                <p className="text-xl font-bold text-white font-mono">
                  {chainStats.chain.slot?.toLocaleString() ?? '—'}
                </p>
              </div>
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Epoch</p>
                <p className="text-xl font-bold text-white font-mono">
                  {chainStats.chain.epoch?.epoch?.toLocaleString() ?? '—'}
                </p>
                {chainStats.chain.epoch && (
                  <div className="mt-2">
                    <div className="flex justify-between text-[10px] text-gray-400 mb-1">
                      <span>{chainStats.chain.epoch.slotIndex.toLocaleString()}</span>
                      <span>{chainStats.chain.epoch.slotsInEpoch.toLocaleString()}</span>
                    </div>
                    <div className="w-full bg-gray-700 rounded-full h-1.5">
                      <div
                        className="bg-purple-500 h-1.5 rounded-full transition-all"
                        style={{ width: `${(chainStats.chain.epoch.slotIndex / chainStats.chain.epoch.slotsInEpoch * 100).toFixed(1)}%` }}
                      ></div>
                    </div>
                  </div>
                )}
              </div>
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Block Height</p>
                <p className="text-xl font-bold text-white font-mono">
                  {chainStats.chain.block_height?.toLocaleString() ?? '—'}
                </p>
              </div>
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Network Txns</p>
                <p className="text-xl font-bold text-white font-mono">
                  {chainStats.chain.epoch?.transactionCount
                    ? `${(chainStats.chain.epoch.transactionCount / 1_000_000_000).toFixed(2)}B`
                    : '—'}
                </p>
              </div>
            </div>

            {/* Blockhash */}
            {chainStats.chain.latest_blockhash && (
              <div className="bg-gray-900/50 rounded-lg p-3 mb-6 font-mono text-xs">
                <span className="text-gray-400 mr-2">Latest Blockhash:</span>
                <span className="text-green-400 break-all">{chainStats.chain.latest_blockhash}</span>
              </div>
            )}

            {/* Upstream RPCs + Proxy metrics */}
            <div className="grid md:grid-cols-2 gap-4">
              {/* Upstreams */}
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <div className="flex items-center gap-2 mb-3">
                  <Link className="w-4 h-4 text-blue-400" />
                  <h3 className="text-sm font-semibold text-white">Upstream RPCs</h3>
                </div>
                <div className="space-y-2">
                  {chainStats.upstreams.map((u) => (
                    <div key={u.name} className="flex items-center justify-between text-sm">
                      <div className="flex items-center gap-2">
                        <span className={`w-2 h-2 rounded-full ${u.healthy ? 'bg-green-400' : 'bg-red-400'}`}></span>
                        <span className="text-white">{u.name}</span>
                      </div>
                      <span className="text-gray-400 font-mono">{u.latency_ms.toFixed(0)}ms</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Proxy metrics */}
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <div className="flex items-center gap-2 mb-3">
                  <Zap className="w-4 h-4 text-yellow-400" />
                  <h3 className="text-sm font-semibold text-white">GPU RPC Proxy</h3>
                </div>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Total Requests</span>
                    <span className="text-white font-mono">{formatNumber(chainStats.proxy.total_requests)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Cache Hit Rate</span>
                    <span className="text-green-400 font-mono">{chainStats.proxy.cache_hit_rate}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">GPU Verified</span>
                    <span className="text-purple-400 font-mono">{formatNumber(chainStats.gpu_verifier.total_verified)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Uptime</span>
                    <span className="text-white font-mono">{formatUptime(chainStats.proxy.uptime_seconds)}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Details Grid */}
        <div className="grid md:grid-cols-2 gap-6">
          {/* Validator Info */}
          <div className="card">
            <h3 className="text-lg font-bold text-white mb-4">Validator Information</h3>
            <div className="space-y-3">
              <div className="flex justify-between py-2 border-b border-gray-700">
                <span className="text-gray-400">Chain</span>
                <span className="text-white font-semibold capitalize">{stats?.chain}</span>
              </div>
              <div className="flex justify-between py-2 border-b border-gray-700">
                <span className="text-gray-400">Status</span>
                <span className={`
                  px-2 py-1 rounded text-xs font-semibold
                  ${stats?.status === 'enabled' ? 'bg-green-500/20 text-green-300' : 'bg-red-500/20 text-red-300'}
                `}>
                  {stats?.status}
                </span>
              </div>
              <div className="flex justify-between py-2 border-b border-gray-700">
                <span className="text-gray-400">API Key</span>
                <code className="text-white font-mono text-xs">{api.getApiKey()?.slice(0, 20)}...</code>
              </div>
              <div className="flex justify-between py-2">
                <span className="text-gray-400">Last Used</span>
                <span className="text-white">
                  {stats?.usage.last_used
                    ? new Date(stats.usage.last_used * 1000).toLocaleString()
                    : 'Never'}
                </span>
              </div>
            </div>
          </div>

          {/* Bridge Statistics */}
          <div className="card">
            <h3 className="text-lg font-bold text-white mb-4">Bridge Statistics</h3>
            <div className="space-y-3">
              <div className="flex justify-between py-2 border-b border-gray-700">
                <span className="text-gray-400">Total Received</span>
                <span className="text-white font-semibold">{formatNumber(bridgeStats?.total_received || 0)}</span>
              </div>
              <div className="flex justify-between py-2 border-b border-gray-700">
                <span className="text-gray-400">Total Forwarded</span>
                <span className="text-white font-semibold">{formatNumber(bridgeStats?.total_forwarded || 0)}</span>
              </div>
              <div className="flex justify-between py-2 border-b border-gray-700">
                <span className="text-gray-400">Total Failed</span>
                <span className="text-white font-semibold">{formatNumber(bridgeStats?.total_failed || 0)}</span>
              </div>
              <div className="flex justify-between py-2">
                <span className="text-gray-400">Success Rate</span>
                <span className="text-green-400 font-semibold">
                  {bridgeStats?.total_received
                    ? ((bridgeStats.total_forwarded / bridgeStats.total_received) * 100).toFixed(2) + '%'
                    : '0%'}
                </span>
              </div>
            </div>
          </div>
        </div>

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
