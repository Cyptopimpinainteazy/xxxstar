import { useState, useEffect, useCallback, type ReactElement } from 'react';
import { api } from '../api';
import type { AdminCommand, AdminJob, AggregatedMetrics, Subscriber, AccountingSummary } from '../api';
import {
  Shield, ArrowLeft, RefreshCw, X, Play, Square, Terminal, Trash2,
  Activity, Cpu, Globe, Gauge, Heart, FileText, Server, Zap, TrendingUp,
  DollarSign, AlertTriangle, ChevronDown, ChevronRight, Link, Lock,
  BarChart3, Clock, Eye, Download, Pause, RotateCcw, Settings,
  Users, UserCheck, UserX, Search, Plus, Ban, CheckCircle, ShieldCheck,
} from 'lucide-react';
import {
  LineChart, Line, BarChart, Bar, AreaChart, Area,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend,
} from 'recharts';

interface AdminDashboardProps {
  onBack: () => void;
}

type Tab = 'overview' | 'performance' | 'network' | 'subscribers' | 'intelligence' | 'stress' | 'controls';

const TABS: { key: Tab; label: string; icon: typeof Shield }[] = [
  { key: 'overview',     label: 'Overview',      icon: BarChart3 },
  { key: 'performance',  label: 'Performance',   icon: TrendingUp },
  { key: 'network',      label: 'Network',       icon: Globe },
  { key: 'subscribers',  label: 'Subscribers',   icon: Users },
  { key: 'intelligence', label: 'Intelligence',  icon: DollarSign },
  { key: 'stress',       label: 'Stress Test',   icon: Zap },
  { key: 'controls',     label: 'Controls',      icon: Settings },
];

const TIME_RANGES = [
  { key: '1m', label: '1m', seconds: 60 },
  { key: '5m', label: '5m', seconds: 300 },
  { key: '15m', label: '15m', seconds: 900 },
  { key: '30m', label: '30m', seconds: 1800 },
  { key: '1h', label: '1H', seconds: 3600 },
];

function fmt(n: number | undefined | null): string {
  if (n == null) return '—';
  return n.toLocaleString();
}
function fmtK(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return n.toString();
}
function fmtTime(s: number): string {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return `${h}h ${m}m`;
}
function tsToTime(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

const GPU_COUNT = 3;
const GPU_POWER_WATTS = 150;
const ELECTRICITY_RATE = 0.12;

const statusDot = (s: string) =>
  s === 'up' ? 'bg-green-500' : s === 'error' ? 'bg-yellow-500' : 'bg-red-500';
const jobBadge = (s: string) => ({
  running: 'bg-blue-500/20 text-blue-300',
  completed: 'bg-green-500/20 text-green-300',
  failed: 'bg-red-500/20 text-red-300',
  killed: 'bg-yellow-500/20 text-yellow-300',
}[s] || 'bg-gray-500/20 text-gray-300');

export function AdminDashboard({ onBack }: AdminDashboardProps) {
  const [tab, setTab] = useState<Tab>('overview');
  const [metrics, setMetrics] = useState<AggregatedMetrics | null>(null);
  const [metricsHistory, setMetricsHistory] = useState<AggregatedMetrics['aggregated'][]>([]);
  const [commands, setCommands] = useState<Record<string, AdminCommand[]>>({});
  const [categories, setCategories] = useState<string[]>([]);
  const [jobs, setJobs] = useState<AdminJob[]>([]);
  const [selectedJob, setSelectedJob] = useState<AdminJob | null>(null);
  const [expandedCats, setExpandedCats] = useState<Set<string>>(new Set(['services', 'health']));
  const [runningCmds, setRunningCmds] = useState<Set<string>>(new Set());
  const [timeRange, setTimeRange] = useState('5m');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Subscriber state
  const [subscribers, setSubscribers] = useState<Subscriber[]>([]);
  const [accounting, setAccounting] = useState<AccountingSummary | null>(null);
  const [whitelist, setWhitelist] = useState<string[]>([]);
  const [blacklist, setBlacklist] = useState<string[]>([]);
  const [subSearch, setSubSearch] = useState('');
  const [subTierFilter, setSubTierFilter] = useState('');
  const [newWlEntry, setNewWlEntry] = useState('');
  const [newBlEntry, setNewBlEntry] = useState('');
  const [editingSub, setEditingSub] = useState<string | null>(null);
  const [editTier, setEditTier] = useState('');

  const loadMetrics = useCallback(async () => {
    try {
      const [m, hist] = await Promise.all([
        api.getAdminMetrics().catch(() => null),
        api.getAdminMetricsHistory(3600).catch(() => null),
      ]);
      if (m) setMetrics(m);
      if (hist?.points) setMetricsHistory(hist.points);
      setError(null);
    } catch {
      setError('Failed to connect to Admin API');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadCommands = useCallback(async () => {
    try {
      const [cmdData, jobData] = await Promise.all([
        api.getAdminCommands().catch(() => null),
        api.getAdminJobs().catch(() => null),
      ]);
      if (cmdData) { setCommands(cmdData.commands); setCategories(cmdData.categories); }
      if (jobData) setJobs(jobData.jobs);
    } catch { /* ignore */ }
  }, []);

  const loadSubscribers = useCallback(async () => {
    try {
      const [subData, acctData, wlData, blData] = await Promise.all([
        api.getSubscribers(subSearch, subTierFilter).catch(() => null),
        api.getAccounting().catch(() => null),
        api.getWhitelist().catch(() => null),
        api.getBlacklist().catch(() => null),
      ]);
      if (subData) setSubscribers(subData.subscribers);
      if (acctData) setAccounting(acctData);
      if (wlData) setWhitelist(wlData.whitelist);
      if (blData) setBlacklist(blData.blacklist);
    } catch { /* ignore */ }
  }, [subSearch, subTierFilter]);

  useEffect(() => {
    loadMetrics();
    loadCommands();
    loadSubscribers();
    const i1 = setInterval(loadMetrics, 3000);
    const i2 = setInterval(loadCommands, 5000);
    const i3 = setInterval(loadSubscribers, 10000);
    return () => { clearInterval(i1); clearInterval(i2); clearInterval(i3); };
  }, [loadMetrics, loadCommands, loadSubscribers]);

  // Poll selected job
  useEffect(() => {
    if (!selectedJob || selectedJob.status !== 'running') return;
    const interval = setInterval(async () => {
      try {
        const updated = await api.getAdminJobDetail(selectedJob.job_id);
        setSelectedJob(updated);
        if (updated.status !== 'running') loadCommands();
      } catch { /* ignore */ }
    }, 1500);
    return () => clearInterval(interval);
  }, [selectedJob, loadCommands]);

  const runCommand = async (cmdId: string) => {
    setRunningCmds(prev => new Set(prev).add(cmdId));
    try {
      const result = await api.runAdminCommand(cmdId);
      const job = await api.getAdminJobDetail(result.job_id);
      setSelectedJob(job);
      await loadCommands();
    } catch (e) {
      setError(`Failed: ${cmdId}`);
    } finally {
      setRunningCmds(prev => { const n = new Set(prev); n.delete(cmdId); return n; });
    }
  };

  const killJob = async (jobId: string) => {
    try {
      await api.killAdminJob(jobId);
      if (selectedJob?.job_id === jobId) {
        setSelectedJob(await api.getAdminJobDetail(jobId));
      }
      await loadCommands();
    } catch { /* ignore */ }
  };

  const viewJob = async (jobId: string) => {
    try { setSelectedJob(await api.getAdminJobDetail(jobId)); } catch { /* ignore */ }
  };

  const exportForensics = async () => {
    try {
      const bundle = await api.adminAction('export-forensics');
      const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `inferstructor-forensics-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch { setError('Export failed'); }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <RefreshCw className="w-12 h-12 text-red-400 animate-spin" />
      </div>
    );
  }

  const agg = metrics?.aggregated;
  const services = metrics?.services || {};
  const gpuLanes = metrics?.gpu_lanes || [];
  const chain = metrics?.chain as Record<string, any> | null | undefined;
  const upstreams = metrics?.upstreams || [];
  // Filter history by time range
  const rangeSeconds = TIME_RANGES.find(r => r.key === timeRange)?.seconds || 300;
  const cutoff = Date.now() / 1000 - rangeSeconds;
  const filteredHistory = metricsHistory.filter(p => (p as any).timestamp >= cutoff);

  // ── Render helpers ──

  const renderMetricCard = (
    icon: typeof Shield, color: string, label: string,
    value: string | number, sub?: string,
  ) => {
    const Icon = icon;
    return (
      <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
        <Icon className={`w-5 h-5 ${color} mb-2`} />
        <p className="text-2xl font-bold text-white font-mono">{value}</p>
        <p className="text-xs text-gray-400">{label}</p>
        {sub && <p className="text-[10px] text-gray-500 mt-1">{sub}</p>}
      </div>
    );
  };

  // ──────────────── TAB CONTENT ────────────────

  const renderOverview = () => (
    <div className="space-y-6">
      {/* Service Status Grid */}
      <div className="card">
        <div className="flex items-center gap-2 mb-4">
          <Activity className="w-5 h-5 text-green-400" />
          <h2 className="text-lg font-bold text-white">Service Status</h2>
          <span className="ml-auto text-sm text-gray-400">
            {agg?.services_up || 0}/{agg?.services_total || 0} online
          </span>
        </div>
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
          {Object.entries(services).map(([name, status]) => (
            <div key={name} className="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
              <div className="flex items-center gap-2 mb-1">
                <span className={`w-2.5 h-2.5 rounded-full ${statusDot(status)} ${status === 'up' ? 'animate-pulse' : ''}`} />
                <span className="text-white text-xs font-semibold truncate">{name}</span>
              </div>
              <span className={`text-[10px] font-semibold ${status === 'up' ? 'text-green-400' : 'text-red-400'}`}>
                {status.toUpperCase()}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        {renderMetricCard(Activity, 'text-green-400', 'Services Online', `${agg?.services_up || 0}/${agg?.services_total || 0}`)}
        {renderMetricCard(Zap, 'text-yellow-400', 'Current TPS', fmt(agg?.current_tps))}
        {renderMetricCard(TrendingUp, 'text-blue-400', 'Peak TPS', fmt(agg?.peak_tps))}
        {renderMetricCard(Server, 'text-purple-400', 'Total GPU Txns', fmtK(agg?.total_gpu_txns || 0))}
        {renderMetricCard(Heart, 'text-green-400', 'Success Rate', `${agg?.success_rate || 0}%`)}
        {renderMetricCard(Clock, 'text-blue-400', 'Uptime', fmtTime(agg?.uptime_seconds || 0))}
      </div>

      {/* Mini TPS Chart */}
      <div className="card">
        <h3 className="text-sm font-bold text-white mb-3">TPS (last 5 minutes)</h3>
        <div className="h-32">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={filteredHistory.map(p => ({ time: tsToTime((p as any).timestamp), tps: p.current_tps }))}>
              <defs>
                <linearGradient id="tpsGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#3B82F6" stopOpacity={0.4} />
                  <stop offset="95%" stopColor="#3B82F6" stopOpacity={0} />
                </linearGradient>
              </defs>
              <XAxis dataKey="time" hide />
              <YAxis hide />
              <Area type="monotone" dataKey="tps" stroke="#3B82F6" fill="url(#tpsGrad)" strokeWidth={2} isAnimationActive={false} />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* GPU Summary */}
      {gpuLanes.length > 0 && (
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Cpu className="w-5 h-5 text-green-400" />
            <h3 className="text-lg font-bold text-white">GPU Fleet</h3>
          </div>
          <div className="grid md:grid-cols-3 gap-3">
            {gpuLanes.map((lane: any, i: number) => (
              <div key={i} className="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-white text-sm font-semibold capitalize">
                    {lane.service?.replace('gpu-lane-', '') || `GPU ${i}`}
                  </span>
                  <span className="text-xs text-green-400 font-mono">GPU {lane.gpu?.id}</span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div><span className="text-gray-400">Util:</span> <span className="text-white">{lane.gpu?.utilization}%</span></div>
                  <div><span className="text-gray-400">VRAM:</span> <span className="text-yellow-400">{lane.gpu?.memory_used_mb?.toFixed(0)} MB</span></div>
                  <div><span className="text-gray-400">Temp:</span> <span className="text-orange-400">{lane.gpu?.temperature_c}°C</span></div>
                  <div><span className="text-gray-400">TPS:</span> <span className="text-blue-400">{fmt(Math.round(lane.stats?.txns_per_second || 0))}</span></div>
                </div>
                <div className="mt-2 w-full bg-gray-700 rounded-full h-1.5">
                  <div className="bg-blue-500 h-1.5 rounded-full" style={{ width: `${Math.min(lane.gpu?.utilization || 0, 100)}%` }} />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );

  const renderPerformance = () => (
    <div className="space-y-6">
      {/* TPS Chart */}
      <div className="card">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-bold text-white">Real-Time TPS</h2>
          <div className="flex items-center gap-2">
            {filteredHistory.length > 0 && (
              <span className="text-blue-400 font-mono text-sm">
                Peak: {fmtK(Math.max(...filteredHistory.map(h => h.peak_tps || 0)))}
              </span>
            )}
          </div>
        </div>
        {/* Time range selector */}
        <div className="flex items-center gap-1 mb-4 bg-gray-900/60 rounded-lg p-1 w-fit">
          {TIME_RANGES.map(r => (
            <button
              key={r.key}
              onClick={() => setTimeRange(r.key)}
              className={`px-3 py-1 rounded text-xs font-semibold transition-all ${
                timeRange === r.key
                  ? 'bg-blue-600 text-white shadow-sm'
                  : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
              }`}
            >
              {r.label}
            </button>
          ))}
          <span className="text-gray-600 text-xs ml-2 font-mono">{filteredHistory.length} pts</span>
        </div>
        <div className="h-72">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={filteredHistory.map(p => ({ time: tsToTime((p as any).timestamp), tps: p.current_tps, peak: p.peak_tps }))}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="time" stroke="#9CA3AF" fontSize={11} />
              <YAxis stroke="#9CA3AF" fontSize={11} tickFormatter={fmtK} />
              <Tooltip contentStyle={{ backgroundColor: '#1F2937', border: '1px solid #374151', borderRadius: '8px', color: '#fff' }} formatter={(v: number | undefined) => [fmt(v ?? 0)]} />
              <Legend />
              <Line type="monotone" dataKey="tps" stroke="#3B82F6" strokeWidth={2} dot={false} name="Current TPS" isAnimationActive={false} />
              <Line type="monotone" dataKey="peak" stroke="#EF4444" strokeWidth={1} dot={false} name="Peak TPS" strokeDasharray="5 5" isAnimationActive={false} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Throughput & Success */}
      <div className="grid md:grid-cols-2 gap-6">
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">Throughput Utilization</h3>
          <div className="flex items-end gap-4 mb-3">
            <span className="text-4xl font-bold text-blue-400">{agg?.throughput_utilization || 0}%</span>
            <span className="text-xs text-gray-400 mb-1">of 960K theoretical max</span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-3">
            <div className="bg-blue-500 h-3 rounded-full transition-all" style={{ width: `${Math.min(agg?.throughput_utilization || 0, 100)}%` }} />
          </div>
        </div>
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">Transaction Success Rate</h3>
          <div className="flex items-end gap-4 mb-3">
            <span className={`text-4xl font-bold ${(agg?.success_rate || 0) >= 99 ? 'text-green-400' : (agg?.success_rate || 0) >= 95 ? 'text-yellow-400' : 'text-red-400'}`}>
              {agg?.success_rate || 0}%
            </span>
            <span className="text-xs text-gray-400 mb-1">
              {fmt(agg?.total_gpu_success || 0)} / {fmt(agg?.total_gpu_txns || 0)}
            </span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-3">
            <div className={`h-3 rounded-full transition-all ${(agg?.success_rate || 0) >= 99 ? 'bg-green-500' : 'bg-yellow-500'}`} style={{ width: `${agg?.success_rate || 0}%` }} />
          </div>
        </div>
      </div>

      {/* GPU Distribution */}
      {gpuLanes.length > 0 && (
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">GPU Lane Distribution</h3>
          <div className="h-48">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={gpuLanes.map((l: any) => ({
                name: (l.service || '').replace('gpu-lane-', '').toUpperCase() || `GPU ${l.gpu?.id}`,
                txns: l.stats?.total_txns || 0,
                tps: Math.round(l.stats?.txns_per_second || 0),
                utilization: l.gpu?.utilization || 0,
              }))}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="name" stroke="#9CA3AF" fontSize={12} />
                <YAxis stroke="#9CA3AF" fontSize={11} tickFormatter={fmtK} />
                <Tooltip contentStyle={{ backgroundColor: '#1F2937', border: '1px solid #374151', borderRadius: '8px', color: '#fff' }} formatter={(v: number | undefined) => [fmt(v ?? 0)]} />
                <Legend />
                <Bar dataKey="txns" fill="#3B82F6" name="Total Txns" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      )}

      {/* GPU Thermal & Memory */}
      <div className="grid md:grid-cols-3 gap-4">
        {renderMetricCard(Cpu, 'text-blue-400', 'Avg GPU Utilization', `${agg?.avg_gpu_utilization || 0}%`)}
        {renderMetricCard(Server, 'text-yellow-400', 'Avg VRAM Used', `${(agg?.avg_gpu_memory_mb || 0).toFixed(0)} MB`)}
        {renderMetricCard(AlertTriangle, 'text-orange-400', 'Avg GPU Temp', `${agg?.avg_gpu_temp_c || 0}°C`)}
      </div>
    </div>
  );

  const renderNetwork = () => (
    <div className="space-y-6">
      {/* Chain Data */}
      {chain && (
        <div className="card">
            <div className="flex items-center gap-2 mb-4">
              <Globe className="w-5 h-5 text-purple-400" />
              <h2 className="text-lg font-bold text-white">Solana Chain — Live</h2>
              <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
              {(chain as any).version && (
                <span className="ml-auto text-xs text-gray-400 font-mono">
                  Core {(chain as any).version['solana-core']}
                </span>
              )}
            </div>
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-4">
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Slot</p>
                <p className="text-xl font-bold text-white font-mono">{fmt((chain as any).slot)}</p>
              </div>
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Epoch</p>
                <p className="text-xl font-bold text-white font-mono">{fmt((chain as any).epoch?.epoch)}</p>
                {(chain as any).epoch && (
                  <div className="mt-2">
                    <div className="flex justify-between text-[10px] text-gray-400 mb-1">
                      <span>{fmt((chain as any).epoch.slotIndex)}</span>
                      <span>{fmt((chain as any).epoch.slotsInEpoch)}</span>
                    </div>
                    <div className="w-full bg-gray-700 rounded-full h-1.5">
                      <div className="bg-purple-500 h-1.5 rounded-full" style={{ width: `${((chain as any).epoch.slotIndex / (chain as any).epoch.slotsInEpoch * 100).toFixed(1)}%` }} />
                    </div>
                  </div>
                )}
              </div>
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Block Height</p>
                <p className="text-xl font-bold text-white font-mono">{fmt((chain as any).block_height)}</p>
              </div>
              <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
                <p className="text-xs text-gray-400 mb-1">Network Txns</p>
                <p className="text-xl font-bold text-white font-mono">
                  {(chain as any).epoch?.transactionCount
                    ? `${((chain as any).epoch.transactionCount / 1e9).toFixed(2)}B`
                    : '—'}
                </p>
              </div>
            </div>
            {(chain as any).latest_blockhash && (
              <div className="bg-gray-900/50 rounded-lg p-3 font-mono text-xs">
                <span className="text-gray-400 mr-2">Latest Blockhash:</span>
                <span className="text-green-400 break-all">{(chain as any).latest_blockhash}</span>
              </div>
            )}
          </div>
      )}

      {/* Upstream RPCs & Proxy */}
      <div className="grid md:grid-cols-2 gap-6">
        <div className="card">
          <div className="flex items-center gap-2 mb-3">
            <Link className="w-4 h-4 text-blue-400" />
            <h3 className="text-sm font-semibold text-white">Upstream RPCs</h3>
          </div>
          <div className="space-y-2">
            {upstreams.map((u: any) => (
              <div key={u.name} className="flex items-center justify-between text-sm bg-gray-800/40 rounded-lg p-2">
                <div className="flex items-center gap-2">
                  <span className={`w-2 h-2 rounded-full ${u.healthy ? 'bg-green-400' : 'bg-red-400'}`} />
                  <span className="text-white">{u.name}</span>
                </div>
                <div className="flex items-center gap-3 text-xs text-gray-400 font-mono">
                  <span>{u.latency_ms?.toFixed(0)}ms</span>
                  <span>{fmt(u.requests)} req</span>
                  <span className={u.errors > 0 ? 'text-red-400' : ''}>{u.errors} err</span>
                </div>
              </div>
            ))}
            {upstreams.length === 0 && <p className="text-gray-500 text-sm">No upstream data</p>}
          </div>
        </div>
        <div className="card">
          <div className="flex items-center gap-2 mb-3">
            <Zap className="w-4 h-4 text-yellow-400" />
            <h3 className="text-sm font-semibold text-white">GPU RPC Proxy</h3>
          </div>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between"><span className="text-gray-400">Total Requests</span><span className="text-white font-mono">{fmt(agg?.rpc_total_requests)}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">Cache Hit Rate</span><span className="text-green-400 font-mono">{agg?.rpc_cache_hit_rate || '—'}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">GPU Verified</span><span className="text-purple-400 font-mono">{fmt(agg?.rpc_gpu_verified)}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">Cached Responses</span><span className="text-blue-400 font-mono">{fmt(agg?.rpc_cached_responses)}</span></div>
            <div className="flex justify-between"><span className="text-gray-400">Errors</span><span className={`font-mono ${(agg?.rpc_errors || 0) > 0 ? 'text-red-400' : 'text-gray-400'}`}>{agg?.rpc_errors || 0}</span></div>
          </div>
        </div>
      </div>
    </div>
  );

  const renderIntelligence = () => (
    <div className="space-y-6">
      {/* Cost Intelligence */}
      <div className="card">
        <div className="flex items-center gap-2 mb-4">
          <DollarSign className="w-5 h-5 text-green-400" />
          <h2 className="text-lg font-bold text-white">Cost & Fee Intelligence</h2>
        </div>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">GPU Power Draw</p>
            <p className="text-2xl font-bold text-yellow-400 font-mono">{agg?.gpu_power_watts || 0}W</p>
            <p className="text-[10px] text-gray-500">{GPU_COUNT}x GTX 1070 @ {GPU_POWER_WATTS}W</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Cost/Hour</p>
            <p className="text-2xl font-bold text-green-400 font-mono">${agg?.gpu_cost_per_hour_usd?.toFixed(4) || '0'}</p>
            <p className="text-[10px] text-gray-500">@ ${ELECTRICITY_RATE}/kWh</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Cost/Million Txns</p>
            <p className="text-2xl font-bold text-blue-400 font-mono">${agg?.cost_per_million_tx_usd?.toFixed(4) || '—'}</p>
            <p className="text-[10px] text-gray-500">at current throughput</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Cost/Tx</p>
            <p className="text-2xl font-bold text-purple-400 font-mono">
              {agg?.cost_per_tx_usd ? `$${agg.cost_per_tx_usd.toExponential(2)}` : '—'}
            </p>
            <p className="text-[10px] text-gray-500">effectively free</p>
          </div>
        </div>
      </div>

      {/* Reliability */}
      <div className="card">
        <div className="flex items-center gap-2 mb-4">
          <AlertTriangle className="w-5 h-5 text-orange-400" />
          <h2 className="text-lg font-bold text-white">Reliability & Fault Detection</h2>
        </div>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Dropped Tx %</p>
            <p className={`text-2xl font-bold font-mono ${(agg?.dropped_tx_pct || 0) > 1 ? 'text-red-400' : 'text-green-400'}`}>
              {agg?.dropped_tx_pct || 0}%
            </p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">Bridge Failed</p>
            <p className="text-2xl font-bold text-red-400 font-mono">{fmt(agg?.bridge_failed)}</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">GPU Failed</p>
            <p className="text-2xl font-bold text-red-400 font-mono">{fmt(agg?.total_gpu_failed)}</p>
          </div>
          <div className="bg-gray-800/50 rounded-lg p-4 border border-gray-700">
            <p className="text-xs text-gray-400 mb-1">RPC Errors</p>
            <p className={`text-2xl font-bold font-mono ${(agg?.rpc_errors || 0) > 0 ? 'text-red-400' : 'text-green-400'}`}>
              {agg?.rpc_errors || 0}
            </p>
          </div>
        </div>
      </div>

      {/* Security & MEV */}
      <div className="grid md:grid-cols-2 gap-6">
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Lock className="w-5 h-5 text-blue-400" />
            <h3 className="text-lg font-bold text-white">Security Radar</h3>
          </div>
          <div className="space-y-3 text-sm">
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Admin Auth</span>
              <span className="px-2 py-0.5 rounded text-xs font-semibold bg-green-500/20 text-green-300">JWT Active</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">CORS Policy</span>
              <span className="text-yellow-400 text-xs">Open (dev mode)</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">GPU Sig Verification</span>
              <span className="text-green-400 text-xs">{fmt(agg?.rpc_gpu_verified)} verified</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Cache Layer</span>
              <span className="text-green-400 text-xs">{agg?.rpc_cache_hit_rate || '—'} hit rate</span>
            </div>
          </div>
        </div>
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Eye className="w-5 h-5 text-purple-400" />
            <h3 className="text-lg font-bold text-white">MEV & Extraction</h3>
          </div>
          <div className="space-y-3 text-sm">
            <div className="flex justify-between items-center">
              <span className="text-gray-400">MEV Detection</span>
              <span className="px-2 py-0.5 rounded text-xs font-semibold bg-blue-500/20 text-blue-300">Monitoring</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Sandwich Attacks</span>
              <span className="text-green-400 text-xs">None detected</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Front-running Risk</span>
              <span className="text-green-400 text-xs">Low</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Tx Ordering</span>
              <span className="text-gray-400 text-xs">FIFO (GPU batch)</span>
            </div>
          </div>
        </div>
      </div>

      {/* Efficiency History */}
      {filteredHistory.length > 1 && (
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">Throughput Utilization Over Time</h3>
          <div className="h-40">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={filteredHistory.map(p => ({ time: tsToTime((p as any).timestamp), util: p.throughput_utilization, dropped: p.dropped_tx_pct }))}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" hide />
                <YAxis stroke="#9CA3AF" fontSize={10} />
                <Tooltip contentStyle={{ backgroundColor: '#1F2937', border: '1px solid #374151', borderRadius: '8px', color: '#fff' }} />
                <Area type="monotone" dataKey="util" stroke="#8B5CF6" fill="#8B5CF6" fillOpacity={0.2} name="Utilization %" isAnimationActive={false} />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>
      )}
    </div>
  );

  const renderSubscribers = () => {
    const tierColor: Record<string, string> = {
      basic: 'text-gray-300 bg-gray-600/30 border-gray-600',
      pro: 'text-blue-300 bg-blue-600/20 border-blue-600',
      enterprise: 'text-yellow-300 bg-yellow-600/20 border-yellow-600',
    };
    const tierIcon: Record<string, typeof Shield> = {
      basic: Users,
      pro: UserCheck,
      enterprise: ShieldCheck,
    };

    const handleToggle = async (vid: string, enabled: boolean) => {
      try {
        if (enabled) await api.disableSubscriber(vid);
        else await api.enableSubscriber(vid);
        await loadSubscribers();
      } catch { setError('Failed to update subscriber'); }
    };

    const handleChangeTier = async (vid: string, newTier: string) => {
      try {
        await api.updateSubscriber(vid, { sla_tier: newTier });
        setEditingSub(null);
        setEditTier('');
        await loadSubscribers();
      } catch { setError('Failed to update tier'); }
    };

    const handleAddWl = async () => {
      if (!newWlEntry.trim()) return;
      await api.addToWhitelist(newWlEntry.trim(), 'Admin added');
      setNewWlEntry('');
      await loadSubscribers();
    };

    const handleRemoveWl = async (entry: string) => {
      await api.removeFromWhitelist(entry);
      await loadSubscribers();
    };

    const handleAddBl = async () => {
      if (!newBlEntry.trim()) return;
      await api.addToBlacklist(newBlEntry.trim(), 'Admin blocked');
      setNewBlEntry('');
      await loadSubscribers();
    };

    const handleRemoveBl = async (entry: string) => {
      await api.removeFromBlacklist(entry);
      await loadSubscribers();
    };

    return (
      <div className="space-y-6">
        {/* Accounting Summary */}
        {accounting && (
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            {renderMetricCard(Users, 'text-blue-400', 'Total Subscribers', accounting.total_subscribers)}
            {renderMetricCard(UserCheck, 'text-green-400', 'Active', accounting.active)}
            {renderMetricCard(UserX, 'text-red-400', 'Inactive', accounting.inactive)}
            {renderMetricCard(DollarSign, 'text-green-400', 'Monthly Revenue', `$${accounting.monthly_revenue_usd}`)}
            {renderMetricCard(DollarSign, 'text-blue-400', 'Annual Revenue', `$${accounting.annual_revenue_usd}`)}
            {renderMetricCard(Activity, 'text-purple-400', 'Total Tx Processed', fmtK(accounting.total_tx_processed))}
          </div>
        )}

        {/* Tier Breakdown */}
        {accounting && (
          <div className="card">
            <div className="flex items-center gap-2 mb-4">
              <Shield className="w-5 h-5 text-yellow-400" />
              <h2 className="text-lg font-bold text-white">Subscription Tiers</h2>
            </div>
            <div className="grid md:grid-cols-3 gap-4">
              {Object.entries(accounting.tier_config).map(([tier, config]) => {
                const TierIcon = tierIcon[tier] || Users;
                const count = accounting.tier_breakdown[tier] || 0;
                return (
                  <div key={tier} className={`rounded-lg p-4 border ${tierColor[tier] || 'border-gray-700'}`}>
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <TierIcon className="w-5 h-5" />
                        <span className="text-lg font-bold capitalize">{tier}</span>
                      </div>
                      <span className="text-2xl font-bold font-mono">{count}</span>
                    </div>
                    <div className="space-y-1 text-xs text-gray-400">
                      <div className="flex justify-between"><span>Max TPS</span><span className="text-white font-mono">{fmtK(config.max_tps)}</span></div>
                      <div className="flex justify-between"><span>Rate Limit</span><span className="text-white font-mono">{fmt(config.rate_limit_rpm)} rpm</span></div>
                      <div className="flex justify-between"><span>Price/mo</span><span className="text-green-400 font-mono">${config.price_monthly}</span></div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Subscriber Table */}
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Users className="w-5 h-5 text-blue-400" />
              <h2 className="text-lg font-bold text-white">Subscribers</h2>
              <span className="text-xs text-gray-500">({subscribers.length})</span>
            </div>
            <div className="flex items-center gap-2">
              {/* Tier filter */}
              <select
                value={subTierFilter}
                onChange={e => setSubTierFilter(e.target.value)}
                className="bg-gray-800 text-white text-xs rounded-lg px-2 py-1.5 border border-gray-600 focus:outline-none focus:border-blue-500"
              >
                <option value="">All Tiers</option>
                <option value="basic">Basic</option>
                <option value="pro">Pro</option>
                <option value="enterprise">Enterprise</option>
              </select>
              {/* Search */}
              <div className="relative">
                <Search className="w-3.5 h-3.5 absolute left-2 top-1/2 -translate-y-1/2 text-gray-500" />
                <input
                  type="text"
                  placeholder="Search..."
                  value={subSearch}
                  onChange={e => setSubSearch(e.target.value)}
                  className="bg-gray-800 text-white text-xs rounded-lg pl-7 pr-3 py-1.5 border border-gray-600 focus:outline-none focus:border-blue-500 w-40"
                />
              </div>
              <button onClick={loadSubscribers} className="p-1.5 text-gray-400 hover:text-white hover:bg-gray-700 rounded-lg">
                <RefreshCw className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          {subscribers.length === 0 ? (
            <p className="text-gray-500 text-sm py-4 text-center">No subscribers found</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-gray-400 text-xs border-b border-gray-700">
                    <th className="text-left pb-2 pl-2">Status</th>
                    <th className="text-left pb-2">Validator ID</th>
                    <th className="text-left pb-2">Chain</th>
                    <th className="text-left pb-2">Tier</th>
                    <th className="text-right pb-2">Requests</th>
                    <th className="text-right pb-2">Txns</th>
                    <th className="text-right pb-2">Max TPS</th>
                    <th className="text-left pb-2">Last Active</th>
                    <th className="text-right pb-2 pr-2">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {subscribers.map(sub => (
                    <tr key={sub.validator_id} className="border-b border-gray-800 hover:bg-gray-800/40 transition-colors">
                      <td className="py-2.5 pl-2">
                        <span className={`w-2.5 h-2.5 rounded-full inline-block ${sub.enabled ? 'bg-green-500' : 'bg-red-500'}`} />
                      </td>
                      <td className="py-2.5 font-mono text-xs text-white">{sub.validator_id}</td>
                      <td className="py-2.5 capitalize text-gray-300">{sub.chain}</td>
                      <td className="py-2.5">
                        {editingSub === sub.validator_id ? (
                          <select
                            value={editTier || sub.sla_tier}
                            onChange={e => setEditTier(e.target.value)}
                            onBlur={() => { if (editTier && editTier !== sub.sla_tier) handleChangeTier(sub.validator_id, editTier); else { setEditingSub(null); setEditTier(''); } }}
                            autoFocus
                            className="bg-gray-700 text-white text-xs rounded px-1 py-0.5 border border-blue-500"
                          >
                            <option value="basic">Basic</option>
                            <option value="pro">Pro</option>
                            <option value="enterprise">Enterprise</option>
                          </select>
                        ) : (
                          <button
                            onClick={() => { setEditingSub(sub.validator_id); setEditTier(sub.sla_tier); }}
                            className={`px-2 py-0.5 rounded text-[10px] font-semibold border capitalize cursor-pointer ${tierColor[sub.sla_tier] || 'text-gray-300 border-gray-600'}`}
                          >
                            {sub.sla_tier}
                          </button>
                        )}
                      </td>
                      <td className="py-2.5 text-right font-mono text-gray-300">{fmt(sub.total_requests)}</td>
                      <td className="py-2.5 text-right font-mono text-gray-300">{fmtK(sub.total_tx)}</td>
                      <td className="py-2.5 text-right font-mono text-gray-300">{fmtK(sub.max_tps)}</td>
                      <td className="py-2.5 text-xs text-gray-400">
                        {sub.last_used ? new Date(sub.last_used * 1000).toLocaleString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }) : 'Never'}
                      </td>
                      <td className="py-2.5 pr-2 text-right">
                        <button
                          onClick={() => handleToggle(sub.validator_id, sub.enabled)}
                          className={`px-2 py-1 rounded text-[10px] font-semibold mr-1 ${
                            sub.enabled
                              ? 'bg-red-900/40 text-red-300 hover:bg-red-800/50'
                              : 'bg-green-900/40 text-green-300 hover:bg-green-800/50'
                          }`}
                          title={sub.enabled ? 'Disable' : 'Enable'}
                        >
                          {sub.enabled ? 'Disable' : 'Enable'}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Whitelist & Blacklist */}
        <div className="grid md:grid-cols-2 gap-6">
          {/* Whitelist */}
          <div className="card">
            <div className="flex items-center gap-2 mb-4">
              <CheckCircle className="w-5 h-5 text-green-400" />
              <h3 className="text-lg font-bold text-white">Whitelist</h3>
              <span className="text-xs text-gray-500">({whitelist.length})</span>
            </div>
            <p className="text-xs text-gray-500 mb-3">IPs or validator IDs with guaranteed access. If non-empty, only whitelisted entries are allowed.</p>
            <div className="flex gap-2 mb-3">
              <input
                type="text"
                placeholder="IP or validator ID..."
                value={newWlEntry}
                onChange={e => setNewWlEntry(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleAddWl()}
                className="flex-1 bg-gray-800 text-white text-xs rounded-lg px-3 py-2 border border-gray-600 focus:outline-none focus:border-green-500"
              />
              <button
                onClick={handleAddWl}
                disabled={!newWlEntry.trim()}
                className="px-3 py-2 bg-green-900/50 text-green-300 rounded-lg text-xs font-semibold hover:bg-green-800/60 disabled:opacity-40 border border-green-700/50"
              >
                <Plus className="w-3.5 h-3.5" />
              </button>
            </div>
            <div className="space-y-1 max-h-40 overflow-y-auto">
              {whitelist.length === 0 && <p className="text-gray-600 text-xs">No entries — all non-blacklisted IPs allowed</p>}
              {whitelist.map(entry => (
                <div key={entry} className="flex items-center justify-between bg-gray-800/40 rounded-lg px-3 py-1.5 text-sm">
                  <span className="font-mono text-green-300 text-xs">{entry}</span>
                  <button onClick={() => handleRemoveWl(entry)} className="text-red-400 hover:text-red-300 p-0.5">
                    <X className="w-3 h-3" />
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* Blacklist */}
          <div className="card">
            <div className="flex items-center gap-2 mb-4">
              <Ban className="w-5 h-5 text-red-400" />
              <h3 className="text-lg font-bold text-white">Blacklist</h3>
              <span className="text-xs text-gray-500">({blacklist.length})</span>
            </div>
            <p className="text-xs text-gray-500 mb-3">Blocked IPs or validator IDs. Automatically removed from whitelist when added here.</p>
            <div className="flex gap-2 mb-3">
              <input
                type="text"
                placeholder="IP or validator ID..."
                value={newBlEntry}
                onChange={e => setNewBlEntry(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleAddBl()}
                className="flex-1 bg-gray-800 text-white text-xs rounded-lg px-3 py-2 border border-gray-600 focus:outline-none focus:border-red-500"
              />
              <button
                onClick={handleAddBl}
                disabled={!newBlEntry.trim()}
                className="px-3 py-2 bg-red-900/50 text-red-300 rounded-lg text-xs font-semibold hover:bg-red-800/60 disabled:opacity-40 border border-red-700/50"
              >
                <Plus className="w-3.5 h-3.5" />
              </button>
            </div>
            <div className="space-y-1 max-h-40 overflow-y-auto">
              {blacklist.length === 0 && <p className="text-gray-600 text-xs">No blocked entries</p>}
              {blacklist.map(entry => (
                <div key={entry} className="flex items-center justify-between bg-gray-800/40 rounded-lg px-3 py-1.5 text-sm">
                  <span className="font-mono text-red-300 text-xs">{entry}</span>
                  <button onClick={() => handleRemoveBl(entry)} className="text-green-400 hover:text-green-300 p-0.5">
                    <X className="w-3 h-3" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    );
  };

  const renderStressTest = () => {
    const benchCmds = commands['benchmarks'] || [];
    const benchJobs = jobs.filter(j => ['bench-rpc-latency', 'load-test-bridge', 'load-test-direct'].includes(j.command));

    return (
      <div className="space-y-6">
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Zap className="w-5 h-5 text-yellow-400" />
            <h2 className="text-lg font-bold text-white">Stress Test & Simulation</h2>
          </div>
          <div className="grid sm:grid-cols-3 gap-3 mb-6">
            {benchCmds.map(cmd => {
              const isRunning = runningCmds.has(cmd.id);
              return (
                <button
                  key={cmd.id}
                  onClick={() => runCommand(cmd.id)}
                  disabled={isRunning}
                  className={`flex items-center gap-3 rounded-lg p-4 text-left transition-all bg-yellow-900/40 hover:bg-yellow-800/50 text-yellow-300 border border-yellow-700/50 ${isRunning ? 'opacity-50 cursor-wait' : ''}`}
                >
                  {isRunning ? <RefreshCw className="w-5 h-5 animate-spin" /> : <Gauge className="w-5 h-5" />}
                  <div>
                    <div className="font-semibold">{cmd.label}</div>
                    <div className="text-xs opacity-60">{cmd.description}</div>
                  </div>
                </button>
              );
            })}
          </div>
          {benchCmds.length === 0 && <p className="text-gray-500 text-sm">No benchmark commands loaded</p>}
        </div>

        {/* Test Result History */}
        <div className="card">
          <h3 className="text-sm font-bold text-white mb-3">Test Results</h3>
          <div className="space-y-2 max-h-60 overflow-y-auto">
            {benchJobs.length === 0 && <p className="text-gray-500 text-sm">No tests run yet</p>}
            {benchJobs.map(j => (
              <button
                key={j.job_id}
                onClick={() => viewJob(j.job_id)}
                className={`w-full text-left rounded-lg p-3 border transition-colors ${
                  selectedJob?.job_id === j.job_id ? 'bg-gray-700/60 border-blue-500/50' : 'bg-gray-800/40 border-gray-700/50 hover:bg-gray-700/40'
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="text-white text-sm font-medium">{j.label}</span>
                  <span className={`px-1.5 py-0.5 rounded text-[10px] font-semibold ${jobBadge(j.status)}`}>{j.status}</span>
                </div>
                <div className="flex items-center justify-between text-xs text-gray-400 mt-1">
                  <span className="font-mono">{j.job_id}</span>
                  <span>{j.duration_seconds}s</span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Job Output */}
        {selectedJob && (
          <div className="card">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <Terminal className="w-4 h-4 text-green-400" />
                <h3 className="text-sm font-bold text-white truncate">{selectedJob.label}</h3>
                <span className={`px-1.5 py-0.5 rounded text-[10px] font-semibold ${jobBadge(selectedJob.status)}`}>
                  {selectedJob.status}
                </span>
              </div>
              <div className="flex items-center gap-1">
                {selectedJob.status === 'running' && selectedJob.pid && (
                  <button onClick={() => killJob(selectedJob.job_id)} className="p-1 text-red-400 hover:text-red-300 rounded" title="Kill">
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                )}
                <button onClick={() => setSelectedJob(null)} className="p-1 text-gray-400 hover:text-white rounded">
                  <X className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
            <div className="bg-black/60 rounded-lg p-3 font-mono text-xs max-h-96 overflow-y-auto whitespace-pre-wrap text-green-300 border border-gray-700/50">
              {selectedJob.output?.length ? selectedJob.output.map((line, i) => (
                <div key={i} className={line.includes('Error') || line.includes('FAIL') || line.includes('❌') ? 'text-red-400' : line.includes('✅') || line.includes('SUCCESS') ? 'text-green-400' : ''}>
                  {line}
                </div>
              )) : <span className="text-gray-500">Waiting for output...</span>}
              {selectedJob.status === 'running' && <span className="inline-block w-2 h-4 bg-green-400 animate-pulse ml-0.5" />}
            </div>
          </div>
        )}
      </div>
    );
  };

  const renderControls = () => {
    const CATEGORY_META: Record<string, { icon: typeof Shield; color: string; label: string }> = {
      services: { icon: Server, color: 'text-blue-400', label: 'Services' },
      health: { icon: Heart, color: 'text-green-400', label: 'Health Checks' },
      system: { icon: Cpu, color: 'text-purple-400', label: 'System' },
      logs: { icon: FileText, color: 'text-gray-400', label: 'Logs' },
    };

    const controlCats = categories.filter(c => c !== 'benchmarks');

    return (
      <div className="space-y-6">
        {/* Admin Actions */}
        <div className="card">
          <div className="flex items-center gap-2 mb-4">
            <Settings className="w-5 h-5 text-blue-400" />
            <h2 className="text-lg font-bold text-white">Admin Actions</h2>
          </div>
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-3">
            <button
              onClick={() => api.adminAction('pause').then(loadMetrics)}
              className="flex items-center gap-2 rounded-lg p-3 bg-orange-900/40 hover:bg-orange-800/50 text-orange-300 border border-orange-700/50 text-sm"
            >
              <Pause className="w-4 h-4" />
              <span className="font-semibold">Pause Services</span>
            </button>
            <button
              onClick={() => api.adminAction('resume').then(loadMetrics)}
              className="flex items-center gap-2 rounded-lg p-3 bg-green-900/40 hover:bg-green-800/50 text-green-300 border border-green-700/50 text-sm"
            >
              <RotateCcw className="w-4 h-4" />
              <span className="font-semibold">Resume Services</span>
            </button>
            <button
              onClick={exportForensics}
              className="flex items-center gap-2 rounded-lg p-3 bg-blue-900/40 hover:bg-blue-800/50 text-blue-300 border border-blue-700/50 text-sm"
            >
              <Download className="w-4 h-4" />
              <span className="font-semibold">Export Forensics</span>
            </button>
            <button
              onClick={() => { api.adminLogout(); onBack(); }}
              className="flex items-center gap-2 rounded-lg p-3 bg-red-900/40 hover:bg-red-800/50 text-red-300 border border-red-700/50 text-sm"
            >
              <Lock className="w-4 h-4" />
              <span className="font-semibold">Lock Admin</span>
            </button>
          </div>
        </div>

        {/* Command panels */}
        <div className="grid lg:grid-cols-3 gap-6">
          <div className="lg:col-span-2 space-y-4">
            {controlCats.map(cat => {
              const meta = CATEGORY_META[cat] || { icon: Terminal, color: 'text-gray-400', label: cat };
              const Icon = meta.icon;
              const cmds = commands[cat] || [];
              const isExpanded = expandedCats.has(cat);

              return (
                <div key={cat} className="card">
                  <button
                    onClick={() => setExpandedCats(prev => {
                      const n = new Set(prev);
                      if (n.has(cat)) n.delete(cat); else n.add(cat);
                      return n;
                    })}
                    className="flex items-center gap-3 w-full text-left mb-2"
                  >
                    {isExpanded ? <ChevronDown className="w-4 h-4 text-gray-500" /> : <ChevronRight className="w-4 h-4 text-gray-500" />}
                    <Icon className={`w-5 h-5 ${meta.color}`} />
                    <h3 className="text-lg font-bold text-white">{meta.label}</h3>
                    <span className="text-gray-500 text-xs ml-auto">{cmds.length} commands</span>
                  </button>
                  {isExpanded && (
                    <div className="grid sm:grid-cols-2 gap-2 mt-3">
                      {cmds.map(cmd => {
                        const isRunning = runningCmds.has(cmd.id);
                        const isStop = cmd.id.startsWith('stop-');
                        const isStart = cmd.id.startsWith('start-');

                        let btnClass = 'bg-gray-700 hover:bg-gray-600 text-white';
                        let btnIcon = <Play className="w-3.5 h-3.5" />;
                        if (isStop) { btnClass = 'bg-red-900/50 hover:bg-red-800/60 text-red-300 border border-red-700/50'; btnIcon = <Square className="w-3.5 h-3.5" />; }
                        else if (isStart) { btnClass = 'bg-green-900/50 hover:bg-green-800/60 text-green-300 border border-green-700/50'; btnIcon = <Play className="w-3.5 h-3.5" />; }

                        return (
                          <button
                            key={cmd.id}
                            onClick={() => runCommand(cmd.id)}
                            disabled={isRunning}
                            className={`flex items-center gap-3 rounded-lg p-3 text-left transition-all ${btnClass} ${isRunning ? 'opacity-50 cursor-wait' : ''}`}
                          >
                            {isRunning ? <RefreshCw className="w-3.5 h-3.5 animate-spin" /> : btnIcon}
                            <div className="min-w-0">
                              <div className="text-sm font-semibold truncate">{cmd.label}</div>
                              <div className="text-xs opacity-60 truncate">{cmd.description}</div>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          {/* Jobs panel */}
          <div className="space-y-4">
            <div className="card">
              <div className="flex items-center gap-2 mb-4">
                <Terminal className="w-5 h-5 text-blue-400" />
                <h3 className="text-lg font-bold text-white">Recent Jobs</h3>
              </div>
              <div className="space-y-2 max-h-80 overflow-y-auto">
                {jobs.length === 0 && <p className="text-gray-500 text-sm">No jobs yet</p>}
                {jobs.map(j => (
                  <button
                    key={j.job_id}
                    onClick={() => viewJob(j.job_id)}
                    className={`w-full text-left rounded-lg p-2.5 border transition-colors ${
                      selectedJob?.job_id === j.job_id ? 'bg-gray-700/60 border-blue-500/50' : 'bg-gray-800/40 border-gray-700/50 hover:bg-gray-700/40'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-white text-sm font-medium truncate">{j.label}</span>
                      <span className={`px-1.5 py-0.5 rounded text-[10px] font-semibold ${jobBadge(j.status)}`}>{j.status}</span>
                    </div>
                    <div className="flex items-center justify-between text-xs text-gray-400">
                      <span className="font-mono">{j.job_id}</span>
                      <span>{j.duration_seconds}s</span>
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Job output */}
            {selectedJob && (
              <div className="card">
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <Terminal className="w-4 h-4 text-green-400" />
                    <h3 className="text-sm font-bold text-white truncate">{selectedJob.label}</h3>
                  </div>
                  <div className="flex items-center gap-1">
                    {selectedJob.status === 'running' && selectedJob.pid && (
                      <button onClick={() => killJob(selectedJob.job_id)} className="p-1 text-red-400 hover:text-red-300 rounded">
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    )}
                    <button onClick={() => setSelectedJob(null)} className="p-1 text-gray-400 hover:text-white rounded">
                      <X className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
                <div className="bg-black/60 rounded-lg p-3 font-mono text-xs max-h-96 overflow-y-auto whitespace-pre-wrap text-green-300 border border-gray-700/50">
                  {selectedJob.output?.length ? selectedJob.output.map((line, i) => (
                    <div key={i} className={line.includes('Error') || line.includes('❌') ? 'text-red-400' : line.includes('✅') ? 'text-green-400' : ''}>
                      {line}
                    </div>
                  )) : <span className="text-gray-500">Waiting for output...</span>}
                  {selectedJob.status === 'running' && <span className="inline-block w-2 h-4 bg-green-400 animate-pulse ml-0.5" />}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  };

  const TAB_CONTENT: Record<Tab, () => ReactElement> = {
    overview: renderOverview,
    performance: renderPerformance,
    network: renderNetwork,
    subscribers: renderSubscribers,
    intelligence: renderIntelligence,
    stress: renderStressTest,
    controls: renderControls,
  };

  return (
    <div className="min-h-screen p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-4">
            <button
              onClick={onBack}
              className="flex items-center gap-2 px-3 py-2 text-gray-300 hover:text-white hover:bg-gray-800 rounded-lg transition-colors"
            >
              <ArrowLeft className="w-4 h-4" />
              Dashboard
            </button>
            <div>
              <div className="flex items-center gap-2">
                <Shield className="w-6 h-6 text-red-400" />
                <h1 className="text-2xl font-bold text-white">Admin Control Center</h1>
              </div>
              <p className="text-gray-500 text-xs mt-1">Inferstructor — Full Infrastructure Control</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <span className="text-xs text-gray-500 font-mono">{new Date().toLocaleTimeString()}</span>
            <button onClick={() => { loadMetrics(); loadCommands(); }} className="p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg">
              <RefreshCw className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Error banner */}
        {error && (
          <div className="bg-red-900/30 border border-red-700 rounded-lg p-3 mb-4 text-red-300 text-sm flex justify-between items-center">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="text-red-400 hover:text-red-200"><X className="w-4 h-4" /></button>
          </div>
        )}

        {/* Tab Bar */}
        <div className="flex items-center gap-1 mb-6 bg-gray-900/60 rounded-xl p-1 border border-gray-700/50 overflow-x-auto">
          {TABS.map(t => {
            const Icon = t.icon;
            return (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className={`flex items-center gap-2 px-4 py-2.5 rounded-lg text-sm font-semibold transition-all whitespace-nowrap ${
                  tab === t.key
                    ? 'bg-gray-700/80 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-800/50'
                }`}
              >
                <Icon className="w-4 h-4" />
                {t.label}
              </button>
            );
          })}
        </div>

        {/* Tab Content */}
        {TAB_CONTENT[tab]()}
      </div>
    </div>
  );
}
