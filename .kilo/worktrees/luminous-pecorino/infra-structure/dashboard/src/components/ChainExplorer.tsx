import { useState, useEffect, useCallback, useRef } from 'react';
import {
  Globe, Search, Filter, ChevronLeft, ChevronRight, Database,
  Cpu, Link2, Activity, Layers, Zap, Shield, Eye, X,
  BarChart3, Hash, RefreshCw,
} from 'lucide-react';
import {
  BarChart, Bar, PieChart, Pie, Cell,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
} from 'recharts';

const CHAIN_DB_URL = import.meta.env.VITE_CHAIN_DB_URL || 'http://localhost:7070';

// ── Types ────────────────────────────────────────────────────────────────────

interface Chain {
  chain_id: string;
  chain_name: string;
  chain_numeric_id: number | null;
  ecosystem: string;
  chain_type: string;
  consensus: string;
  native_token: string | null;
  is_evm: number;
  is_svm: number;
  is_testnet: number;
  supports_gpu: number;
  status: string;
}

interface ChainDetail extends Chain {
  logo_url?: string;
  website_url?: string;
  explorer_url?: string;
  docs_url?: string;
  rpc_endpoints: Array<{
    url: string;
    protocol: string;
    provider: string;
    tier: string;
    is_primary: number;
    is_healthy: number;
    latency_ms: number | null;
  }>;
  gpu_stats: {
    sig_algorithm: string;
    hash_algorithm: string;
    sig_pubkey_size: number;
    sig_size: number;
    hash_output_size: number;
    gpu_verifications_total: number;
    gpu_verifications_failed: number;
    avg_verify_time_us: number | null;
  } | null;
}

interface OverviewStats {
  overview: {
    total_chains: number;
    active_chains: number;
    evm_chains: number;
    svm_chains: number;
    testnets: number;
    gpu_enabled: number;
    ecosystems: number;
    chain_types: number;
  };
  ecosystems: Array<{
    ecosystem: string;
    chain_count: number;
    active: number;
    testnets: number;
    gpu_enabled: number;
  }>;
  chain_types: Array<{ chain_type: string; count: number }>;
  consensus: Array<{ consensus: string; count: number }>;
}

interface Pagination {
  page: number;
  limit: number;
  total: number;
  total_pages: number;
  has_next: boolean;
  has_prev: boolean;
}

interface ChainExplorerProps {
  onBack: () => void;
}

// ── Palette ──────────────────────────────────────────────────────────────────

const ECO_COLORS: Record<string, string> = {
  evm: '#3B82F6',
  cosmos: '#8B5CF6',
  substrate: '#EC4899',
  svm: '#14B8A6',
  move: '#F59E0B',
  other: '#6B7280',
};

const ECO_BG: Record<string, string> = {
  evm: 'bg-blue-500/20 text-blue-300 border-blue-500/30',
  cosmos: 'bg-violet-500/20 text-violet-300 border-violet-500/30',
  substrate: 'bg-pink-500/20 text-pink-300 border-pink-500/30',
  svm: 'bg-teal-500/20 text-teal-300 border-teal-500/30',
  move: 'bg-amber-500/20 text-amber-300 border-amber-500/30',
  other: 'bg-gray-500/20 text-gray-300 border-gray-500/30',
};

const STATUS_BADGE: Record<string, string> = {
  active: 'bg-green-500/20 text-green-400',
  inactive: 'bg-red-500/20 text-red-400',
  deprecated: 'bg-yellow-500/20 text-yellow-400',
  unknown: 'bg-gray-500/20 text-gray-400',
};

// ── Component ────────────────────────────────────────────────────────────────

export function ChainExplorer({ onBack }: ChainExplorerProps) {
  const [chains, setChains] = useState<Chain[]>([]);
  const [overviewStats, setOverviewStats] = useState<OverviewStats | null>(null);
  const [pagination, setPagination] = useState<Pagination | null>(null);
  const [selectedChain, setSelectedChain] = useState<ChainDetail | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [showFilters, setShowFilters] = useState(false);
  const [filters, setFilters] = useState({
    ecosystem: '',
    chain_type: '',
    status: '',
    is_testnet: '',
  });
  const searchTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  // ── Data fetching ──────────────────────────────────────────────────────

  const fetchChains = useCallback(async (p: number = 1) => {
    setLoading(true);
    try {
      const params = new URLSearchParams({ page: String(p), limit: '50' });
      if (filters.ecosystem) params.set('ecosystem', filters.ecosystem);
      if (filters.chain_type) params.set('chain_type', filters.chain_type);
      if (filters.status) params.set('status', filters.status);
      if (filters.is_testnet) params.set('is_testnet', filters.is_testnet);

      const res = await fetch(`${CHAIN_DB_URL}/api/chains?${params}`);
      const data = await res.json();
      setChains(data.chains);
      setPagination(data.pagination);
    } catch (err) {
      console.error('Failed to fetch chains:', err);
    } finally {
      setLoading(false);
    }
  }, [filters]);

  const searchChains = useCallback(async (q: string) => {
    if (!q.trim()) {
      setIsSearching(false);
      fetchChains(1);
      return;
    }
    setIsSearching(true);
    setLoading(true);
    try {
      const res = await fetch(`${CHAIN_DB_URL}/api/chains/search?q=${encodeURIComponent(q)}&limit=100`);
      const data = await res.json();
      setChains(data.results);
      setPagination(null);
    } catch (err) {
      console.error('Search failed:', err);
    } finally {
      setLoading(false);
    }
  }, [fetchChains]);

  const fetchOverview = useCallback(async () => {
    try {
      const res = await fetch(`${CHAIN_DB_URL}/api/chains/stats/overview`);
      const data = await res.json();
      setOverviewStats(data);
    } catch (err) {
      console.error('Failed to fetch overview:', err);
    }
  }, []);

  const fetchChainDetail = useCallback(async (chainId: string) => {
    try {
      const res = await fetch(`${CHAIN_DB_URL}/api/chains/${encodeURIComponent(chainId)}`);
      const data = await res.json();
      setSelectedChain(data);
    } catch (err) {
      console.error('Failed to fetch chain detail:', err);
    }
  }, []);

  useEffect(() => {
    fetchChains(page);
    fetchOverview();
  }, [page, fetchChains, fetchOverview]);

  // Debounced search
  const handleSearch = (value: string) => {
    setSearchQuery(value);
    if (searchTimer.current) clearTimeout(searchTimer.current);
    searchTimer.current = setTimeout(() => searchChains(value), 300);
  };

  const resetFilters = () => {
    setFilters({ ecosystem: '', chain_type: '', status: '', is_testnet: '' });
    setPage(1);
  };

  const fmtK = (n: number) => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
  };

  const ov = overviewStats?.overview;

  // ── Chain Detail Modal ─────────────────────────────────────────────────

  if (selectedChain) {
    const c = selectedChain;
    return (
      <div className="min-h-screen p-6">
        <div className="max-w-5xl mx-auto">
          {/* Back button */}
          <button
            onClick={() => setSelectedChain(null)}
            className="flex items-center gap-2 text-gray-400 hover:text-white mb-6 transition-colors"
          >
            <ChevronLeft className="w-5 h-5" />
            Back to Explorer
          </button>

          {/* Chain Header */}
          <div className="bg-gray-900/80 backdrop-blur-sm rounded-2xl p-8 border border-gray-800 mb-6">
            <div className="flex items-start justify-between">
              <div>
                <div className="flex items-center gap-3 mb-2">
                  <h1 className="text-3xl font-bold text-white">{c.chain_name}</h1>
                  <span className={`px-2 py-0.5 rounded text-xs font-medium border ${ECO_BG[c.ecosystem] || ECO_BG.other}`}>
                    {c.ecosystem.toUpperCase()}
                  </span>
                  <span className={`px-2 py-0.5 rounded text-xs font-medium ${STATUS_BADGE[c.status] || STATUS_BADGE.unknown}`}>
                    {c.status}
                  </span>
                </div>
                <div className="flex items-center gap-4 text-sm text-gray-400">
                  <span className="flex items-center gap-1"><Hash className="w-3.5 h-3.5" /> {c.chain_id}</span>
                  {c.chain_numeric_id && <span className="flex items-center gap-1"><Layers className="w-3.5 h-3.5" /> Chain ID: {c.chain_numeric_id}</span>}
                  {c.native_token && <span className="flex items-center gap-1"><Zap className="w-3.5 h-3.5" /> {c.native_token}</span>}
                  <span className="flex items-center gap-1"><Shield className="w-3.5 h-3.5" /> {c.consensus}</span>
                </div>
              </div>
              <div className="flex gap-2">
                {c.is_evm ? <span className="px-2 py-1 bg-blue-500/15 text-blue-400 text-xs rounded">EVM</span> : null}
                {c.is_svm ? <span className="px-2 py-1 bg-teal-500/15 text-teal-400 text-xs rounded">SVM</span> : null}
                {c.supports_gpu ? <span className="px-2 py-1 bg-green-500/15 text-green-400 text-xs rounded flex items-center gap-1"><Cpu className="w-3 h-3" /> GPU</span> : null}
              </div>
            </div>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* RPC Endpoints */}
            <div className="bg-gray-900/80 backdrop-blur-sm rounded-2xl p-6 border border-gray-800">
              <h2 className="text-lg font-bold text-white mb-4 flex items-center gap-2">
                <Link2 className="w-5 h-5 text-blue-400" />
                RPC Endpoints ({c.rpc_endpoints?.length || 0})
              </h2>
              <div className="space-y-3">
                {c.rpc_endpoints?.map((rpc, i) => (
                  <div key={i} className="bg-gray-800/60 rounded-lg p-3">
                    <div className="flex items-center justify-between mb-1">
                      <div className="flex items-center gap-2">
                        <span className={`w-2 h-2 rounded-full ${rpc.is_healthy ? 'bg-green-400' : 'bg-red-400'}`} />
                        <span className="text-sm text-gray-300">{rpc.provider || 'Unknown'}</span>
                        {rpc.is_primary ? <span className="text-[10px] px-1.5 py-0.5 bg-blue-500/20 text-blue-400 rounded">PRIMARY</span> : null}
                      </div>
                      <span className="text-xs text-gray-500">{rpc.protocol} · {rpc.tier}</span>
                    </div>
                    <code className="text-xs text-gray-400 break-all">{rpc.url}</code>
                    {rpc.latency_ms && <span className="text-xs text-yellow-400 ml-2">{rpc.latency_ms}ms</span>}
                  </div>
                ))}
                {!c.rpc_endpoints?.length && <p className="text-gray-500 text-sm">No RPC endpoints registered</p>}
              </div>
            </div>

            {/* GPU Validation Stats */}
            <div className="bg-gray-900/80 backdrop-blur-sm rounded-2xl p-6 border border-gray-800">
              <h2 className="text-lg font-bold text-white mb-4 flex items-center gap-2">
                <Cpu className="w-5 h-5 text-green-400" />
                GPU Validation
              </h2>
              {c.gpu_stats ? (
                <div className="space-y-4">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="bg-gray-800/60 rounded-lg p-3">
                      <p className="text-xs text-gray-400 mb-1">Signature Algorithm</p>
                      <p className="text-sm font-mono text-white">{c.gpu_stats.sig_algorithm}</p>
                    </div>
                    <div className="bg-gray-800/60 rounded-lg p-3">
                      <p className="text-xs text-gray-400 mb-1">Hash Algorithm</p>
                      <p className="text-sm font-mono text-white">{c.gpu_stats.hash_algorithm}</p>
                    </div>
                    <div className="bg-gray-800/60 rounded-lg p-3">
                      <p className="text-xs text-gray-400 mb-1">Pubkey Size</p>
                      <p className="text-sm font-mono text-white">{c.gpu_stats.sig_pubkey_size} bytes</p>
                    </div>
                    <div className="bg-gray-800/60 rounded-lg p-3">
                      <p className="text-xs text-gray-400 mb-1">Signature Size</p>
                      <p className="text-sm font-mono text-white">{c.gpu_stats.sig_size} bytes</p>
                    </div>
                  </div>
                  <div className="bg-gray-800/60 rounded-lg p-3">
                    <div className="flex justify-between">
                      <div>
                        <p className="text-xs text-gray-400">Total GPU Verifications</p>
                        <p className="text-xl font-bold text-green-400">{fmtK(c.gpu_stats.gpu_verifications_total)}</p>
                      </div>
                      <div className="text-right">
                        <p className="text-xs text-gray-400">Failed</p>
                        <p className="text-xl font-bold text-red-400">{c.gpu_stats.gpu_verifications_failed}</p>
                      </div>
                    </div>
                  </div>
                </div>
              ) : (
                <p className="text-gray-500 text-sm">No GPU validation data available</p>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  }

  // ── Main Explorer View ─────────────────────────────────────────────────

  return (
    <div className="min-h-screen p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div className="flex items-center gap-4">
            <button
              onClick={onBack}
              className="flex items-center gap-2 text-gray-400 hover:text-white transition-colors"
            >
              <ChevronLeft className="w-5 h-5" />
            </button>
            <div>
              <h1 className="text-3xl font-bold text-white flex items-center gap-3">
                <Database className="w-8 h-8 text-blue-400" />
                Chain Explorer
              </h1>
              <p className="text-gray-400 mt-1">
                {ov ? `${fmtK(ov.total_chains)} blockchains across ${ov.ecosystems} ecosystems` : 'Loading...'}
              </p>
            </div>
          </div>
          <button
            onClick={() => { fetchChains(page); fetchOverview(); }}
            className="flex items-center gap-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 hover:text-white rounded-lg transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
        </div>

        {/* Overview Stats Cards */}
        {ov && (
          <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4 mb-8">
            <StatCard icon={<Globe className="w-5 h-5 text-blue-400" />} label="Total Chains" value={fmtK(ov.total_chains)} />
            <StatCard icon={<Activity className="w-5 h-5 text-green-400" />} label="Active" value={fmtK(ov.active_chains)} />
            <StatCard icon={<Layers className="w-5 h-5 text-indigo-400" />} label="EVM Chains" value={fmtK(ov.evm_chains)} />
            <StatCard icon={<Zap className="w-5 h-5 text-teal-400" />} label="SVM Chains" value={fmtK(ov.svm_chains)} />
            <StatCard icon={<Cpu className="w-5 h-5 text-emerald-400" />} label="GPU Enabled" value={fmtK(ov.gpu_enabled)} />
            <StatCard icon={<BarChart3 className="w-5 h-5 text-amber-400" />} label="Testnets" value={fmtK(ov.testnets)} />
          </div>
        )}

        {/* Ecosystem Breakdown Charts */}
        {overviewStats && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
            {/* Ecosystem pie chart */}
            <div className="bg-gray-900/80 backdrop-blur-sm rounded-2xl p-6 border border-gray-800">
              <h2 className="text-lg font-bold text-white mb-4">Ecosystem Distribution</h2>
              <div className="h-64">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={overviewStats.ecosystems.map(e => ({ name: e.ecosystem, value: e.chain_count }))}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={100}
                      paddingAngle={2}
                      dataKey="value"
                      label={({ name, percent }: { name: string; percent?: number }) => `${name} ${((percent ?? 0) * 100).toFixed(0)}%`}
                    >
                      {overviewStats.ecosystems.map((e) => (
                        <Cell key={e.ecosystem} fill={ECO_COLORS[e.ecosystem] || ECO_COLORS.other} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{ backgroundColor: '#1F2937', border: '1px solid #374151', borderRadius: '8px', color: '#fff' }}
                      formatter={(value: number | undefined) => [(value ?? 0).toLocaleString(), 'Chains']}
                    />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            </div>

            {/* Chain types bar chart */}
            <div className="bg-gray-900/80 backdrop-blur-sm rounded-2xl p-6 border border-gray-800">
              <h2 className="text-lg font-bold text-white mb-4">Chain Types</h2>
              <div className="h-64">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={overviewStats.chain_types} layout="vertical">
                    <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                    <XAxis type="number" stroke="#9CA3AF" fontSize={11} tickFormatter={(v) => fmtK(v)} />
                    <YAxis type="category" dataKey="chain_type" stroke="#9CA3AF" fontSize={11} width={80} />
                    <Tooltip
                      contentStyle={{ backgroundColor: '#1F2937', border: '1px solid #374151', borderRadius: '8px', color: '#fff' }}
                      formatter={(value: number | undefined) => [(value ?? 0).toLocaleString(), 'Chains']}
                    />
                    <Bar dataKey="count" fill="#3B82F6" radius={[0, 4, 4, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </div>
          </div>
        )}

        {/* Search + Filters */}
        <div className="bg-gray-900/80 backdrop-blur-sm rounded-2xl p-6 border border-gray-800 mb-6">
          <div className="flex flex-col md:flex-row gap-4">
            {/* Search */}
            <div className="flex-1 relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-500" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => handleSearch(e.target.value)}
                placeholder="Search 62,500+ blockchains... (ethereum, solana, polygon, cosmos...)"
                className="w-full pl-11 pr-4 py-3 bg-gray-800/80 border border-gray-700 rounded-xl text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500/30 transition-all"
              />
              {searchQuery && (
                <button onClick={() => { setSearchQuery(''); setIsSearching(false); fetchChains(1); }} className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-500 hover:text-white">
                  <X className="w-4 h-4" />
                </button>
              )}
            </div>

            {/* Filter toggle */}
            <button
              onClick={() => setShowFilters(!showFilters)}
              className={`flex items-center gap-2 px-5 py-3 rounded-xl border transition-all ${
                showFilters ? 'bg-blue-600 border-blue-500 text-white' : 'bg-gray-800/80 border-gray-700 text-gray-300 hover:text-white hover:border-gray-600'
              }`}
            >
              <Filter className="w-4 h-4" />
              Filters
            </button>
          </div>

          {/* Filter dropdowns */}
          {showFilters && (
            <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-3">
              <FilterSelect
                label="Ecosystem"
                value={filters.ecosystem}
                onChange={(v) => { setFilters(f => ({ ...f, ecosystem: v })); setPage(1); }}
                options={[
                  { value: '', label: 'All Ecosystems' },
                  { value: 'evm', label: 'EVM' },
                  { value: 'svm', label: 'SVM (Solana)' },
                  { value: 'cosmos', label: 'Cosmos' },
                  { value: 'substrate', label: 'Substrate' },
                  { value: 'move', label: 'Move' },
                  { value: 'other', label: 'Other' },
                ]}
              />
              <FilterSelect
                label="Chain Type"
                value={filters.chain_type}
                onChange={(v) => { setFilters(f => ({ ...f, chain_type: v })); setPage(1); }}
                options={[
                  { value: '', label: 'All Types' },
                  { value: 'L1', label: 'Layer 1' },
                  { value: 'L2', label: 'Layer 2' },
                  { value: 'L3', label: 'Layer 3' },
                  { value: 'appchain', label: 'App Chain' },
                  { value: 'sidechain', label: 'Sidechain' },
                  { value: 'parachain', label: 'Parachain' },
                ]}
              />
              <FilterSelect
                label="Status"
                value={filters.status}
                onChange={(v) => { setFilters(f => ({ ...f, status: v })); setPage(1); }}
                options={[
                  { value: '', label: 'All Status' },
                  { value: 'active', label: 'Active' },
                  { value: 'inactive', label: 'Inactive' },
                  { value: 'deprecated', label: 'Deprecated' },
                ]}
              />
              <FilterSelect
                label="Network"
                value={filters.is_testnet}
                onChange={(v) => { setFilters(f => ({ ...f, is_testnet: v })); setPage(1); }}
                options={[
                  { value: '', label: 'All Networks' },
                  { value: '0', label: 'Mainnet Only' },
                  { value: '1', label: 'Testnet Only' },
                ]}
              />
              <button
                onClick={resetFilters}
                className="col-span-2 md:col-span-4 text-sm text-gray-400 hover:text-white transition-colors text-center py-1"
              >
                Reset all filters
              </button>
            </div>
          )}
        </div>

        {/* Chain List */}
        <div className="bg-gray-900/80 backdrop-blur-sm rounded-2xl border border-gray-800 overflow-hidden">
          {/* Table Header */}
          <div className="grid grid-cols-12 gap-2 px-6 py-3 bg-gray-800/50 text-xs font-semibold text-gray-400 uppercase tracking-wider">
            <div className="col-span-4">Chain</div>
            <div className="col-span-2">Ecosystem</div>
            <div className="col-span-1">Type</div>
            <div className="col-span-1">Consensus</div>
            <div className="col-span-1">Token</div>
            <div className="col-span-1">GPU</div>
            <div className="col-span-1">Status</div>
            <div className="col-span-1 text-right">Action</div>
          </div>

          {/* Loading */}
          {loading && (
            <div className="flex items-center justify-center py-16">
              <RefreshCw className="w-8 h-8 text-blue-400 animate-spin" />
            </div>
          )}

          {/* Chain rows */}
          {!loading && chains.map((chain) => (
            <div
              key={chain.chain_id}
              className="grid grid-cols-12 gap-2 px-6 py-3.5 border-t border-gray-800/50 hover:bg-gray-800/30 transition-colors cursor-pointer group"
              onClick={() => fetchChainDetail(chain.chain_id)}
            >
              <div className="col-span-4 flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold bg-gradient-to-br from-blue-600/30 to-purple-600/30 border border-gray-700 text-white">
                  {chain.chain_name.charAt(0)}
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium text-white truncate group-hover:text-blue-400 transition-colors">
                    {chain.chain_name}
                  </p>
                  <p className="text-xs text-gray-500 truncate">{chain.chain_id}</p>
                </div>
              </div>
              <div className="col-span-2 flex items-center">
                <span className={`px-2 py-0.5 rounded text-xs font-medium border ${ECO_BG[chain.ecosystem] || ECO_BG.other}`}>
                  {chain.ecosystem}
                </span>
              </div>
              <div className="col-span-1 flex items-center text-xs text-gray-400">{chain.chain_type}</div>
              <div className="col-span-1 flex items-center text-xs text-gray-400">{chain.consensus}</div>
              <div className="col-span-1 flex items-center text-xs text-gray-300 font-mono">{chain.native_token || '—'}</div>
              <div className="col-span-1 flex items-center">
                {chain.supports_gpu ? (
                  <Cpu className="w-4 h-4 text-green-400" />
                ) : (
                  <span className="text-gray-600">—</span>
                )}
              </div>
              <div className="col-span-1 flex items-center">
                <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${STATUS_BADGE[chain.status] || STATUS_BADGE.unknown}`}>
                  {chain.status}
                </span>
              </div>
              <div className="col-span-1 flex items-center justify-end">
                <Eye className="w-4 h-4 text-gray-600 group-hover:text-blue-400 transition-colors" />
              </div>
            </div>
          ))}

          {/* Empty state */}
          {!loading && chains.length === 0 && (
            <div className="text-center py-16">
              <Database className="w-12 h-12 text-gray-600 mx-auto mb-3" />
              <p className="text-gray-400">No chains found</p>
              <p className="text-sm text-gray-500 mt-1">Try adjusting your search or filters</p>
            </div>
          )}

          {/* Pagination */}
          {pagination && !isSearching && (
            <div className="flex items-center justify-between px-6 py-4 bg-gray-800/30 border-t border-gray-800">
              <p className="text-sm text-gray-400">
                Showing {((pagination.page - 1) * pagination.limit) + 1}–{Math.min(pagination.page * pagination.limit, pagination.total)} of {pagination.total.toLocaleString()} chains
              </p>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => setPage(p => Math.max(1, p - 1))}
                  disabled={!pagination.has_prev}
                  className="p-2 rounded-lg bg-gray-800 text-gray-400 hover:text-white hover:bg-gray-700 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
                >
                  <ChevronLeft className="w-4 h-4" />
                </button>
                <span className="text-sm text-gray-400 px-3">
                  Page {pagination.page} of {pagination.total_pages.toLocaleString()}
                </span>
                <button
                  onClick={() => setPage(p => p + 1)}
                  disabled={!pagination.has_next}
                  className="p-2 rounded-lg bg-gray-800 text-gray-400 hover:text-white hover:bg-gray-700 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
                >
                  <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}

          {/* Search result count */}
          {isSearching && !loading && (
            <div className="px-6 py-4 bg-gray-800/30 border-t border-gray-800">
              <p className="text-sm text-gray-400">
                Found {chains.length} chain{chains.length !== 1 ? 's' : ''} matching "{searchQuery}"
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Sub-components ───────────────────────────────────────────────────────────

function StatCard({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="bg-gray-900/80 backdrop-blur-sm rounded-xl p-4 border border-gray-800">
      <div className="flex items-center gap-2 mb-2">{icon}</div>
      <p className="text-2xl font-bold text-white">{value}</p>
      <p className="text-xs text-gray-400">{label}</p>
    </div>
  );
}

function FilterSelect({
  label, value, onChange, options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: Array<{ value: string; label: string }>;
}) {
  return (
    <div>
      <label className="block text-xs text-gray-400 mb-1">{label}</label>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full px-3 py-2 bg-gray-800/80 border border-gray-700 rounded-lg text-sm text-white focus:outline-none focus:border-blue-500 appearance-none cursor-pointer"
      >
        {options.map(o => (
          <option key={o.value} value={o.value}>{o.label}</option>
        ))}
      </select>
    </div>
  );
}
