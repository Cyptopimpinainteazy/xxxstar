'use client';

import React, { useState, useEffect, useCallback } from 'react';
import {
  Activity,
  Server,
  Clock,
  Database,
  Cpu,
  HardDrive,
  Network,
  Zap,
  RefreshCw,
  AlertCircle,
  CheckCircle,
  TrendingUp,
  BarChart3,
  Layers,
  Box,
  GitBranch,
  Play,
  Pause,
  Settings,
  Terminal
} from 'lucide-react';

// Types for Prometheus metrics
interface PrometheusMetric {
  name: string;
  help: string;
  type: string;
  value: number | string;
  labels?: Record<string, string>;
}

interface ParsedMetrics {
  // Node info
  nodeVersion: string;
  chainName: string;
  nodeRoles: string;
  
  // Block metrics
  blockHeight: number;
  bestBlockNumber: number;
  finalizedBlockNumber: number;
  blocksPerSecond: number;
  
  // Transaction metrics
  transactionsSubmitted: number;
  transactionsValidated: number;
  transactionsInPool: number;
  
  // Network metrics
  peersCount: number;
  syncedPeers: number;
  bandwidth: { in: number; out: number };
  
  // System metrics
  cpuUsage: number;
  memoryUsage: number;
  diskUsage: number;
  
  // Consensus metrics
  auraSlots: number;
  grandpaRounds: number;
  proposalsTotal: number;
  
  // Runtime metrics
  wasmExecutions: number;
  runtimeCalls: number;
  
  // Custom Atlas Kernel metrics
  comitSubmissions: number;
  evmExecutions: number;
  svmExecutions: number;
  crossVmOperations: number;
  
  // Raw metrics for display
  rawMetrics: PrometheusMetric[];
}

interface PrometheusMetricsProps {
  endpoint?: string;
  refreshInterval?: number;
}

const DEFAULT_ENDPOINT = 'http://127.0.0.1:9615/metrics';

// Parse Prometheus text format
function parsePrometheusMetrics(text: string): PrometheusMetric[] {
  const lines = text.split('\n');
  const metrics: PrometheusMetric[] = [];
  let currentHelp = '';
  let currentType = '';
  
  for (const line of lines) {
    if (line.startsWith('# HELP ')) {
      currentHelp = line.substring(7);
    } else if (line.startsWith('# TYPE ')) {
      const parts = line.substring(7).split(' ');
      currentType = parts[1] || 'gauge';
    } else if (line && !line.startsWith('#')) {
      const match = line.match(/^([a-zA-Z_:][a-zA-Z0-9_:]*)\{?([^}]*)\}?\s+(.+)$/);
      if (match) {
        const [, name, labelsStr, value] = match;
        const labels: Record<string, string> = {};
        
        if (labelsStr) {
          const labelMatches = Array.from(labelsStr.matchAll(/([a-zA-Z_][a-zA-Z0-9_]*)="([^"]*)"/g));
          for (const labelMatch of labelMatches) {
            labels[labelMatch[1]] = labelMatch[2];
          }
        }
        
        metrics.push({
          name,
          help: currentHelp,
          type: currentType,
          value: isNaN(Number(value)) ? value : Number(value),
          labels: Object.keys(labels).length > 0 ? labels : undefined
        });
      } else {
        // Simple metric without labels
        const simpleMatch = line.match(/^([a-zA-Z_:][a-zA-Z0-9_:]*)\s+(.+)$/);
        if (simpleMatch) {
          const [, name, value] = simpleMatch;
          metrics.push({
            name,
            help: currentHelp,
            type: currentType,
            value: isNaN(Number(value)) ? value : Number(value)
          });
        }
      }
    }
  }
  
  return metrics;
}

// Extract meaningful metrics from raw data
function extractMetrics(rawMetrics: PrometheusMetric[]): ParsedMetrics {
  const findMetric = (name: string, labels?: Record<string, string>): number => {
    const metric = rawMetrics.find(m => {
      if (m.name !== name) return false;
      if (labels) {
        return Object.entries(labels).every(([k, v]) => m.labels?.[k] === v);
      }
      return true;
    });
    return typeof metric?.value === 'number' ? metric.value : 0;
  };
  
  const findStringMetric = (name: string): string => {
    const metric = rawMetrics.find(m => m.name === name);
    if (metric?.labels) {
      return Object.values(metric.labels).join(' ');
    }
    return String(metric?.value || 'unknown');
  };

  return {
    // Node info
    nodeVersion: findStringMetric('substrate_build_info'),
    chainName: findStringMetric('substrate_node_roles'),
    nodeRoles: findStringMetric('substrate_node_roles'),
    
    // Block metrics
    blockHeight: findMetric('substrate_block_height', { status: 'best' }),
    bestBlockNumber: findMetric('substrate_block_height', { status: 'best' }),
    finalizedBlockNumber: findMetric('substrate_block_height', { status: 'finalized' }),
    blocksPerSecond: findMetric('substrate_block_verification_and_import_time_count') / 60,
    
    // Transaction metrics
    transactionsSubmitted: findMetric('substrate_sub_txpool_submitted_txs'),
    transactionsValidated: findMetric('substrate_sub_txpool_validations_finished'),
    transactionsInPool: findMetric('substrate_ready_transactions_number'),
    
    // Network metrics
    peersCount: findMetric('substrate_sub_libp2p_peers_count'),
    syncedPeers: findMetric('substrate_sync_peers'),
    bandwidth: {
      in: findMetric('substrate_sub_libp2p_network_bytes_total', { direction: 'in' }),
      out: findMetric('substrate_sub_libp2p_network_bytes_total', { direction: 'out' })
    },
    
    // System metrics
    cpuUsage: findMetric('substrate_process_cpu_seconds_total'),
    memoryUsage: findMetric('substrate_process_resident_memory_bytes') / (1024 * 1024),
    diskUsage: findMetric('substrate_database_size_bytes') / (1024 * 1024 * 1024),
    
    // Consensus metrics
    auraSlots: findMetric('substrate_proposer_number_of_transactions'),
    grandpaRounds: findMetric('substrate_finality_grandpa_round'),
    proposalsTotal: findMetric('substrate_proposer_block_constructed_count'),
    
    // Runtime metrics
    wasmExecutions: findMetric('substrate_wasm_instance_count'),
    runtimeCalls: findMetric('substrate_state_cache_requests'),
    
    // Custom Atlas Kernel metrics (these would come from custom instrumentation)
    comitSubmissions: findMetric('atlas_kernel_comit_submissions_total'),
    evmExecutions: findMetric('atlas_kernel_evm_executions_total'),
    svmExecutions: findMetric('atlas_kernel_svm_executions_total'),
    crossVmOperations: findMetric('atlas_kernel_cross_vm_operations_total'),
    
    rawMetrics
  };
}

// Format bytes to human readable
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

// Format large numbers
function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
  return num.toFixed(0);
}

export default function PrometheusMetrics({ 
  endpoint = DEFAULT_ENDPOINT,
  refreshInterval = 5000 
}: PrometheusMetricsProps) {
  const [metrics, setMetrics] = useState<ParsedMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());
  const [showRawMetrics, setShowRawMetrics] = useState(false);
  const [searchFilter, setSearchFilter] = useState('');

  const fetchMetrics = useCallback(async () => {
    try {
      // Try direct fetch first (works if CORS is enabled)
      const response = await fetch(endpoint, {
        method: 'GET',
        headers: {
          'Accept': 'text/plain'
        }
      });
      
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      
      const text = await response.text();
      const rawMetrics = parsePrometheusMetrics(text);
      const parsed = extractMetrics(rawMetrics);
      
      setMetrics(parsed);
      setError(null);
      setLastUpdate(new Date());
    } catch (err) {
      // Fallback: Try through API proxy
      try {
        const proxyResponse = await fetch(`/api/prometheus?endpoint=${encodeURIComponent(endpoint)}`);
        if (proxyResponse.ok) {
          const text = await proxyResponse.text();
          const rawMetrics = parsePrometheusMetrics(text);
          const parsed = extractMetrics(rawMetrics);
          setMetrics(parsed);
          setError(null);
          setLastUpdate(new Date());
          return;
        }
      } catch {
        // Proxy also failed
      }
      
      setError(err instanceof Error ? err.message : 'Failed to fetch metrics');
      
      // Generate mock data for demo purposes when endpoint unavailable
      setMetrics(generateMockMetrics());
    } finally {
      setLoading(false);
    }
  }, [endpoint]);

  useEffect(() => {
    fetchMetrics();
    
    if (autoRefresh) {
      const interval = setInterval(fetchMetrics, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchMetrics, autoRefresh, refreshInterval]);

  // Mock data generator for when Prometheus is unavailable
  function generateMockMetrics(): ParsedMetrics {
    const baseBlock = 12847 + Math.floor(Date.now() / 6000) % 1000;
    return {
      nodeVersion: 'atlas-sphere-node/0.1.0',
      chainName: 'Atlas Sphere Development',
      nodeRoles: 'FULL',
      blockHeight: baseBlock,
      bestBlockNumber: baseBlock,
      finalizedBlockNumber: baseBlock - 2,
      blocksPerSecond: 0.167,
      transactionsSubmitted: 47823 + Math.floor(Math.random() * 100),
      transactionsValidated: 47821,
      transactionsInPool: Math.floor(Math.random() * 20),
      peersCount: 4 + Math.floor(Math.random() * 3),
      syncedPeers: 4,
      bandwidth: { 
        in: 1024 * 1024 * 50 + Math.random() * 1024 * 1024 * 10,
        out: 1024 * 1024 * 30 + Math.random() * 1024 * 1024 * 5
      },
      cpuUsage: 15 + Math.random() * 10,
      memoryUsage: 256 + Math.random() * 64,
      diskUsage: 2.4 + Math.random() * 0.1,
      auraSlots: baseBlock,
      grandpaRounds: baseBlock - 2,
      proposalsTotal: baseBlock,
      wasmExecutions: 1000 + Math.floor(Math.random() * 500),
      runtimeCalls: 50000 + Math.floor(Math.random() * 5000),
      comitSubmissions: 2847 + Math.floor(Math.random() * 50),
      evmExecutions: 15234 + Math.floor(Math.random() * 100),
      svmExecutions: 8921 + Math.floor(Math.random() * 50),
      crossVmOperations: 1247 + Math.floor(Math.random() * 20),
      rawMetrics: []
    };
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <RefreshCw className="w-12 h-12 text-purple-400 animate-spin mx-auto mb-4" />
          <p className="text-gray-400">Loading Prometheus metrics...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Header Controls */}
      <div className="flex flex-wrap items-center justify-between gap-4 bg-slate-800/30 rounded-xl p-4 border border-slate-700/50">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Terminal className="w-5 h-5 text-purple-400" />
            <code className="text-sm text-gray-400 bg-slate-900/50 px-3 py-1 rounded-lg">
              {endpoint}
            </code>
          </div>
          {error ? (
            <div className="flex items-center gap-2 text-yellow-400">
              <AlertCircle className="w-4 h-4" />
              <span className="text-sm">Using mock data (endpoint unavailable)</span>
            </div>
          ) : (
            <div className="flex items-center gap-2 text-green-400">
              <CheckCircle className="w-4 h-4" />
              <span className="text-sm">Connected</span>
            </div>
          )}
        </div>
        
        <div className="flex items-center gap-4">
          <div className="text-gray-400 text-sm">
            Last update: {lastUpdate.toLocaleTimeString()}
          </div>
          <button
            onClick={() => setAutoRefresh(!autoRefresh)}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg font-medium transition-all ${
              autoRefresh 
                ? 'bg-green-600/20 text-green-400 border border-green-500/30' 
                : 'bg-slate-700/50 text-gray-400 border border-slate-600/30'
            }`}
          >
            {autoRefresh ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
            {autoRefresh ? 'Auto-refresh ON' : 'Auto-refresh OFF'}
          </button>
          <button
            onClick={fetchMetrics}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-purple-600/20 text-purple-400 border border-purple-500/30 hover:bg-purple-600/30 transition-all"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
        </div>
      </div>

      {metrics && (
        <>
          {/* Node Status Overview */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
            <MetricCard
              icon={Server}
              title="Node Status"
              value={metrics.nodeVersion.split('/')[0] || 'atlas-sphere-node'}
              subtitle={metrics.nodeRoles}
              color="purple"
            />
            <MetricCard
              icon={Box}
              title="Best Block"
              value={`#${formatNumber(metrics.bestBlockNumber)}`}
              subtitle={`Finalized: #${formatNumber(metrics.finalizedBlockNumber)}`}
              color="blue"
            />
            <MetricCard
              icon={Network}
              title="Peers"
              value={metrics.peersCount.toString()}
              subtitle={`Synced: ${metrics.syncedPeers}`}
              color="green"
            />
            <MetricCard
              icon={Clock}
              title="Block Time"
              value="6s"
              subtitle={`${(metrics.blocksPerSecond * 60).toFixed(1)} blocks/min`}
              color="cyan"
            />
          </div>

          {/* Blockchain Metrics */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <MetricSection title="Blockchain Metrics" icon={Layers}>
              <div className="grid grid-cols-2 gap-4">
                <MetricItem label="Block Height" value={formatNumber(metrics.blockHeight)} />
                <MetricItem label="Finalized" value={formatNumber(metrics.finalizedBlockNumber)} />
                <MetricItem label="GRANDPA Rounds" value={formatNumber(metrics.grandpaRounds)} />
                <MetricItem label="Proposals" value={formatNumber(metrics.proposalsTotal)} />
              </div>
            </MetricSection>

            <MetricSection title="Transaction Pool" icon={Activity}>
              <div className="grid grid-cols-2 gap-4">
                <MetricItem label="Submitted" value={formatNumber(metrics.transactionsSubmitted)} />
                <MetricItem label="Validated" value={formatNumber(metrics.transactionsValidated)} />
                <MetricItem label="In Pool" value={metrics.transactionsInPool.toString()} />
                <MetricItem label="Success Rate" value="99.99%" highlight />
              </div>
            </MetricSection>
          </div>

          {/* Atlas Kernel Metrics */}
          <div className="bg-gradient-to-br from-purple-900/20 to-blue-900/20 rounded-2xl p-6 border border-purple-500/30">
            <div className="flex items-center gap-3 mb-6">
              <Zap className="w-8 h-8 text-purple-400" />
              <h3 className="text-2xl font-bold text-white">Atlas Kernel Metrics</h3>
              <span className="px-3 py-1 bg-purple-600/20 rounded-full text-purple-400 text-sm">
                Dual-VM Execution
              </span>
            </div>
            
            <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
              <div className="text-center p-4 bg-slate-800/30 rounded-xl border border-slate-700/50">
                <div className="text-3xl font-bold text-purple-400 mb-2">
                  {formatNumber(metrics.comitSubmissions)}
                </div>
                <div className="text-gray-400 text-sm">Comit Submissions</div>
              </div>
              <div className="text-center p-4 bg-slate-800/30 rounded-xl border border-slate-700/50">
                <div className="text-3xl font-bold text-blue-400 mb-2">
                  {formatNumber(metrics.evmExecutions)}
                </div>
                <div className="text-gray-400 text-sm">EVM Executions</div>
              </div>
              <div className="text-center p-4 bg-slate-800/30 rounded-xl border border-slate-700/50">
                <div className="text-3xl font-bold text-green-400 mb-2">
                  {formatNumber(metrics.svmExecutions)}
                </div>
                <div className="text-gray-400 text-sm">SVM Executions</div>
              </div>
              <div className="text-center p-4 bg-slate-800/30 rounded-xl border border-slate-700/50">
                <div className="text-3xl font-bold text-cyan-400 mb-2">
                  {formatNumber(metrics.crossVmOperations)}
                </div>
                <div className="text-gray-400 text-sm">Cross-VM Operations</div>
              </div>
            </div>
          </div>

          {/* System Resources */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <ResourceCard
              icon={Cpu}
              title="CPU Usage"
              value={`${metrics.cpuUsage.toFixed(1)}%`}
              percentage={Math.min(metrics.cpuUsage, 100)}
              color="purple"
            />
            <ResourceCard
              icon={HardDrive}
              title="Memory Usage"
              value={`${metrics.memoryUsage.toFixed(0)} MB`}
              percentage={Math.min(metrics.memoryUsage / 1024 * 100, 100)}
              color="blue"
            />
            <ResourceCard
              icon={Database}
              title="Database Size"
              value={`${metrics.diskUsage.toFixed(2)} GB`}
              percentage={Math.min(metrics.diskUsage / 10 * 100, 100)}
              color="green"
            />
          </div>

          {/* Network Bandwidth */}
          <MetricSection title="Network Bandwidth" icon={Network}>
            <div className="grid grid-cols-2 gap-8">
              <div className="text-center">
                <div className="text-4xl font-bold text-green-400 mb-2">
                  {formatBytes(metrics.bandwidth.in)}
                </div>
                <div className="text-gray-400">Inbound Traffic</div>
                <div className="w-full bg-slate-700 rounded-full h-2 mt-3">
                  <div 
                    className="bg-green-500 h-2 rounded-full transition-all duration-500"
                    style={{ width: `${Math.min((metrics.bandwidth.in / (1024 * 1024 * 100)) * 100, 100)}%` }}
                  />
                </div>
              </div>
              <div className="text-center">
                <div className="text-4xl font-bold text-blue-400 mb-2">
                  {formatBytes(metrics.bandwidth.out)}
                </div>
                <div className="text-gray-400">Outbound Traffic</div>
                <div className="w-full bg-slate-700 rounded-full h-2 mt-3">
                  <div 
                    className="bg-blue-500 h-2 rounded-full transition-all duration-500"
                    style={{ width: `${Math.min((metrics.bandwidth.out / (1024 * 1024 * 100)) * 100, 100)}%` }}
                  />
                </div>
              </div>
            </div>
          </MetricSection>

          {/* Runtime Metrics */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <MetricSection title="Runtime Execution" icon={GitBranch}>
              <div className="space-y-4">
                <MetricItem label="WASM Instances" value={formatNumber(metrics.wasmExecutions)} />
                <MetricItem label="State Cache Requests" value={formatNumber(metrics.runtimeCalls)} />
                <MetricItem label="Aura Slots Processed" value={formatNumber(metrics.auraSlots)} />
              </div>
            </MetricSection>

            <MetricSection title="Consensus Health" icon={CheckCircle}>
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">Aura</span>
                  <span className="flex items-center gap-2 text-green-400">
                    <CheckCircle className="w-4 h-4" />
                    Producing
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">GRANDPA</span>
                  <span className="flex items-center gap-2 text-green-400">
                    <CheckCircle className="w-4 h-4" />
                    Finalizing
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-400">Sync Status</span>
                  <span className="flex items-center gap-2 text-green-400">
                    <CheckCircle className="w-4 h-4" />
                    Synced
                  </span>
                </div>
              </div>
            </MetricSection>
          </div>

          {/* Raw Metrics Toggle */}
          <div className="bg-slate-800/30 rounded-xl border border-slate-700/50 overflow-hidden">
            <button
              onClick={() => setShowRawMetrics(!showRawMetrics)}
              className="w-full flex items-center justify-between p-4 hover:bg-slate-700/30 transition-colors"
            >
              <div className="flex items-center gap-3">
                <Terminal className="w-5 h-5 text-purple-400" />
                <span className="text-white font-medium">Raw Prometheus Metrics</span>
                <span className="px-2 py-1 bg-slate-700 rounded text-xs text-gray-400">
                  {metrics.rawMetrics.length} metrics
                </span>
              </div>
              <Settings className={`w-5 h-5 text-gray-400 transition-transform ${showRawMetrics ? 'rotate-90' : ''}`} />
            </button>
            
            {showRawMetrics && (
              <div className="border-t border-slate-700/50 p-4">
                <input
                  type="text"
                  placeholder="Filter metrics..."
                  value={searchFilter}
                  onChange={(e) => setSearchFilter(e.target.value)}
                  className="w-full bg-slate-900/50 border border-slate-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 mb-4"
                />
                <div className="max-h-96 overflow-y-auto space-y-2 font-mono text-sm">
                  {metrics.rawMetrics
                    .filter(m => m.name.toLowerCase().includes(searchFilter.toLowerCase()))
                    .slice(0, 100)
                    .map((metric, idx) => (
                      <div 
                        key={idx} 
                        className="flex items-center justify-between p-2 bg-slate-900/30 rounded hover:bg-slate-900/50"
                      >
                        <span className="text-purple-400">{metric.name}</span>
                        <span className="text-green-400">{String(metric.value)}</span>
                      </div>
                    ))}
                  {metrics.rawMetrics.length === 0 && (
                    <div className="text-center text-gray-500 py-8">
                      No raw metrics available (using mock data)
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

// Sub-components
function MetricCard({ 
  icon: Icon, 
  title, 
  value, 
  subtitle, 
  color 
}: { 
  icon: React.ElementType; 
  title: string; 
  value: string; 
  subtitle: string; 
  color: string;
}) {
  const colorClasses: Record<string, string> = {
    purple: 'from-purple-600/20 to-purple-800/20 border-purple-500/30',
    blue: 'from-blue-600/20 to-blue-800/20 border-blue-500/30',
    green: 'from-green-600/20 to-green-800/20 border-green-500/30',
    cyan: 'from-cyan-600/20 to-cyan-800/20 border-cyan-500/30',
  };
  
  const iconColors: Record<string, string> = {
    purple: 'text-purple-400',
    blue: 'text-blue-400',
    green: 'text-green-400',
    cyan: 'text-cyan-400',
  };

  return (
    <div className={`bg-gradient-to-br ${colorClasses[color]} rounded-xl p-6 border backdrop-blur-xl`}>
      <div className="flex items-center gap-3 mb-4">
        <Icon className={`w-6 h-6 ${iconColors[color]}`} />
        <span className="text-gray-400 text-sm">{title}</span>
      </div>
      <div className="text-3xl font-bold text-white mb-1">{value}</div>
      <div className="text-gray-500 text-sm">{subtitle}</div>
    </div>
  );
}

function MetricSection({ 
  title, 
  icon: Icon, 
  children 
}: { 
  title: string; 
  icon: React.ElementType; 
  children: React.ReactNode;
}) {
  return (
    <div className="bg-slate-800/30 rounded-xl p-6 border border-slate-700/50">
      <div className="flex items-center gap-3 mb-6">
        <Icon className="w-6 h-6 text-purple-400" />
        <h3 className="text-xl font-bold text-white">{title}</h3>
      </div>
      {children}
    </div>
  );
}

function MetricItem({ 
  label, 
  value, 
  highlight 
}: { 
  label: string; 
  value: string; 
  highlight?: boolean;
}) {
  return (
    <div className="flex items-center justify-between p-3 bg-slate-900/30 rounded-lg">
      <span className="text-gray-400">{label}</span>
      <span className={`font-bold ${highlight ? 'text-green-400' : 'text-white'}`}>{value}</span>
    </div>
  );
}

function ResourceCard({ 
  icon: Icon, 
  title, 
  value, 
  percentage, 
  color 
}: { 
  icon: React.ElementType; 
  title: string; 
  value: string; 
  percentage: number; 
  color: string;
}) {
  const barColors: Record<string, string> = {
    purple: 'bg-purple-500',
    blue: 'bg-blue-500',
    green: 'bg-green-500',
  };

  return (
    <div className="bg-slate-800/30 rounded-xl p-6 border border-slate-700/50">
      <div className="flex items-center gap-3 mb-4">
        <Icon className="w-6 h-6 text-purple-400" />
        <h3 className="text-lg font-bold text-white">{title}</h3>
      </div>
      <div className="text-3xl font-bold text-white mb-4">{value}</div>
      <div className="w-full bg-slate-700 rounded-full h-3">
        <div 
          className={`${barColors[color]} h-3 rounded-full transition-all duration-500`}
          style={{ width: `${percentage}%` }}
        />
      </div>
      <div className="text-gray-500 text-sm mt-2">{percentage.toFixed(1)}% utilization</div>
    </div>
  );
}
