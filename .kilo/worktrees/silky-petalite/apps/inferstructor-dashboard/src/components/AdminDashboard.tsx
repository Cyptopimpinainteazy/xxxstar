import { lazy, Suspense, useState, useEffect, useCallback, type ReactElement } from 'react';
import { api } from '../api';
import type {
  AccountingSummary,
  AdminCommand,
  AdminJob,
  AggregatedMetrics,
  ApprovalCase,
  BenchmarkReport,
  EvidenceBundle,
  OrchestraIntent,
  Subscriber,
  TpsBenchmarkStatus,
  VoteTally,
  VoteWindow,
} from '../api';
import { useToast } from './toast-context';
import {
  Shield, ArrowLeft, RefreshCw, X, Play, Square, Terminal, Trash2,
  Activity, Cpu, Globe, Gauge, Heart, FileText, Server, Zap, TrendingUp,
  DollarSign, ChevronDown, ChevronRight, Lock,
  BarChart3, Download, Pause, RotateCcw, Settings,
  Users, UserCheck, UserX, Search, Plus, Ban, CheckCircle, ShieldCheck,
} from 'lucide-react';

interface AdminDashboardProps {
  onBack: () => void;
}

type Tab = 'overview' | 'performance' | 'network' | 'orchestra' | 'subscribers' | 'intelligence' | 'stress' | 'controls';

const TABS: { key: Tab; label: string; icon: typeof Shield }[] = [
  { key: 'overview',     label: 'Overview',      icon: BarChart3 },
  { key: 'performance',  label: 'Performance',   icon: TrendingUp },
  { key: 'network',      label: 'Network',       icon: Globe },
  { key: 'orchestra',    label: 'Operator',      icon: Shield },
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

const renderMetricCard = (
  icon: typeof Shield,
  color: string,
  label: string,
  value: string | number,
  sub?: string,
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

const jobBadge = (s: string) => ({
  running: 'bg-blue-500/20 text-blue-300',
  completed: 'bg-green-500/20 text-green-300',
  failed: 'bg-red-500/20 text-red-300',
  killed: 'bg-yellow-500/20 text-yellow-300',
}[s] || 'bg-gray-500/20 text-gray-300');

const OrchestraOperationsPanel = lazy(() => import('./OrchestraOperationsPanel').then(module => ({ default: module.OrchestraOperationsPanel })));
const OverviewPanel = lazy(() => import('./AdminDashboardTelemetryPanels').then(module => ({ default: module.OverviewPanel })));
const PerformancePanel = lazy(() => import('./AdminDashboardTelemetryPanels').then(module => ({ default: module.PerformancePanel })));
const NetworkPanel = lazy(() => import('./AdminDashboardTelemetryPanels').then(module => ({ default: module.NetworkPanel })));
const IntelligencePanel = lazy(() => import('./AdminDashboardTelemetryPanels').then(module => ({ default: module.IntelligencePanel })));

export function AdminDashboard({ onBack }: AdminDashboardProps) {
  const { addToast } = useToast();
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

  const [orchestraIntents, setOrchestraIntents] = useState<OrchestraIntent[]>([]);
  const [approvalCases, setApprovalCases] = useState<ApprovalCase[]>([]);
  const [voteWindows, setVoteWindows] = useState<VoteWindow[]>([]);
  const [evidenceBundles, setEvidenceBundles] = useState<EvidenceBundle[]>([]);
  const [benchmarkStatus, setBenchmarkStatus] = useState<TpsBenchmarkStatus | null>(null);
  const [benchmarkReports, setBenchmarkReports] = useState<BenchmarkReport[]>([]);
  const [orchestraLoading, setOrchestraLoading] = useState(true);
  const [orchestraError, setOrchestraError] = useState<string | null>(null);

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

  const loadOrchestra = useCallback(async () => {
    try {
      const [intents, approvals, windows, evidence, benchmark, reports] = await Promise.all([
        api.listOrchestraIntents(50, 0).catch(() => null),
        api.listApprovalCases(50, 0).catch(() => null),
        api.listVoteWindows(50, 0).catch(() => null),
        api.listEvidenceBundles(50, 0).catch(() => null),
        api.getTpsBenchmarkStatus().catch(() => null),
        api.getBenchmarkReports(20, 0).catch(() => null),
      ]);

      if (intents) setOrchestraIntents(intents);
      if (approvals) setApprovalCases(approvals);
      if (windows) setVoteWindows(windows);
      if (evidence) setEvidenceBundles(evidence);
      if (benchmark) setBenchmarkStatus(benchmark);
      if (reports) setBenchmarkReports(reports);

      const unavailableFeeds = [
        intents ? null : 'intents',
        approvals ? null : 'approval cases',
        windows ? null : 'vote windows',
        evidence ? null : 'evidence bundles',
        benchmark ? null : 'benchmark status',
        reports ? null : 'benchmark reports',
      ].filter(Boolean);

      setOrchestraError(
        unavailableFeeds.length > 0
          ? `Some operator feeds are unavailable: ${unavailableFeeds.join(', ')}.`
          : null,
      );
    } catch {
      setOrchestraError('Failed to load operator workflow data.');
    } finally {
      setOrchestraLoading(false);
    }
  }, []);

  useEffect(() => {
    loadMetrics();
    loadCommands();
    loadSubscribers();
    loadOrchestra();
    const i1 = setInterval(loadMetrics, 3000);
    const i2 = setInterval(loadCommands, 5000);
    const i3 = setInterval(loadSubscribers, 10000);
    const i4 = setInterval(loadOrchestra, 15000);
    return () => { clearInterval(i1); clearInterval(i2); clearInterval(i3); clearInterval(i4); };
  }, [loadMetrics, loadCommands, loadSubscribers, loadOrchestra]);

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

  // ──────────────── TAB CONTENT ────────────────

  const renderTelemetryFallback = (label: string) => (
    <div className="card">
      <div className="flex items-center gap-2 text-sm text-gray-300">
        <RefreshCw className="h-4 w-4 animate-spin text-blue-400" />
        Loading {label}...
      </div>
    </div>
  );

  const renderOverview = () => (
    <Suspense fallback={renderTelemetryFallback('overview')}>
      <OverviewPanel
        services={services}
        aggregated={agg}
        gpuLanes={gpuLanes}
        filteredHistory={filteredHistory}
      />
    </Suspense>
  );

  const renderPerformance = () => (
    <Suspense fallback={renderTelemetryFallback('performance metrics')}>
      <PerformancePanel
        aggregated={agg}
        gpuLanes={gpuLanes}
        filteredHistory={filteredHistory}
        timeRange={timeRange}
        setTimeRange={setTimeRange}
        timeRanges={TIME_RANGES}
      />
    </Suspense>
  );

  const renderNetwork = () => (
    <Suspense fallback={renderTelemetryFallback('network telemetry')}>
      <NetworkPanel
        aggregated={agg}
        chain={chain}
        upstreams={upstreams}
      />
    </Suspense>
  );

  const renderIntelligence = () => (
    <Suspense fallback={renderTelemetryFallback('cost intelligence')}>
      <IntelligencePanel
        aggregated={agg}
        filteredHistory={filteredHistory}
      />
    </Suspense>
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

  const renderOrchestra = () => (
    <Suspense
      fallback={(
        <div className="card">
          <div className="flex items-center gap-2 text-sm text-gray-300">
            <RefreshCw className="h-4 w-4 animate-spin text-blue-400" />
            Loading operator workflow...
          </div>
        </div>
      )}
    >
      <OrchestraOperationsPanel
        services={services}
        aggregated={agg}
        intents={orchestraIntents}
        approvalCases={approvalCases}
        voteWindows={voteWindows}
        evidenceBundles={evidenceBundles}
        benchmarkStatus={benchmarkStatus}
        benchmarkReports={benchmarkReports}
        jobs={jobs}
        loading={orchestraLoading}
        error={orchestraError}
        onRefresh={loadOrchestra}
        onCloseVoteWindow={async (windowId: string) => {
          const previousVoteWindows = voteWindows;
          const optimisticUpdatedAt = new Date().toISOString();

          try {
            setOrchestraError(null);
            setVoteWindows(current => current.map(window => (
              window.window_id === windowId
                ? { ...window, status: 'closed', updated_at: optimisticUpdatedAt }
                : window
            )));

            const response = await api.closeVoteWindow(windowId);

            setVoteWindows(current => current.map(window => (
              window.window_id === windowId ? response.vote_window : window
            )));
            setApprovalCases(current => current.map(approvalCase => (
              approvalCase.case_id === response.approval_case.case_id ? response.approval_case : approvalCase
            )));
            setEvidenceBundles(current => {
              const filtered = current.filter(bundle => bundle.bundle_id !== response.evidence.bundle_id);
              return [response.evidence, ...filtered];
            });
            addToast(`Vote window ${windowId} closed.`, 'success');
            void loadOrchestra();
          } catch {
            setVoteWindows(previousVoteWindows);
            setOrchestraError(`Failed to close vote window ${windowId}.`);
            addToast(`Failed to close vote window ${windowId}.`, 'error');
            throw new Error('close vote window failed');
          }
        }}
        onImportVoteTally={async (windowId: string): Promise<VoteTally> => {
          const previousVoteWindows = voteWindows;
          try {
            setOrchestraError(null);
            const tally = await api.importVoteWindowTally(windowId);
            setVoteWindows(current => current.map(window => (
              window.window_id === windowId ? { ...window, tally, updated_at: new Date().toISOString() } : window
            )));
            addToast(`Imported tally for ${windowId}.`, 'success');
            void loadOrchestra();
            return tally;
          } catch {
            setVoteWindows(previousVoteWindows);
            setOrchestraError(`Failed to import tally for vote window ${windowId}.`);
            addToast(`Failed to import tally for ${windowId}.`, 'error');
            throw new Error('import tally failed');
          }
        }}
      />
    </Suspense>
  );

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
    orchestra: renderOrchestra,
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
