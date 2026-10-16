'use client';

import React, { useState, useEffect, useCallback } from 'react';
import Link from 'next/link';
import {
  Server,
  Box,
  Network,
  Activity,
  Zap,
  ExternalLink,
  RefreshCw,
  CheckCircle,
  AlertCircle
} from 'lucide-react';

interface NodeMetrics {
  blockHeight: number;
  finalizedBlock: number;
  peersCount: number;
  transactionsInPool: number;
  comitSubmissions: number;
  evmExecutions: number;
  svmExecutions: number;
  crossVmOps: number;
  isConnected: boolean;
}

interface PrometheusWidgetProps {
  endpoint?: string;
  refreshInterval?: number;
}

const DEFAULT_ENDPOINT = 'http://127.0.0.1:9615/metrics';

export default function PrometheusWidget({ 
  endpoint = DEFAULT_ENDPOINT,
  refreshInterval = 10000 
}: PrometheusWidgetProps) {
  const [metrics, setMetrics] = useState<NodeMetrics>({
    blockHeight: 0,
    finalizedBlock: 0,
    peersCount: 0,
    transactionsInPool: 0,
    comitSubmissions: 0,
    evmExecutions: 0,
    svmExecutions: 0,
    crossVmOps: 0,
    isConnected: false
  });
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());

  const fetchMetrics = useCallback(async () => {
    try {
      const response = await fetch(`/api/prometheus?endpoint=${encodeURIComponent(endpoint)}`);
      
      if (response.ok) {
        const text = await response.text();
        
        // Parse key metrics from Prometheus format
        const getMetricValue = (text: string, name: string, labels?: Record<string, string>): number => {
          const lines = text.split('\n');
          for (const line of lines) {
            if (line.startsWith(name)) {
              if (labels) {
                const hasAllLabels = Object.entries(labels).every(([k, v]) => 
                  line.includes(`${k}="${v}"`)
                );
                if (!hasAllLabels) continue;
              }
              const match = line.match(/\s+(\d+(?:\.\d+)?(?:e[+-]?\d+)?)\s*$/);
              if (match) return parseFloat(match[1]);
            }
          }
          return 0;
        };

        setMetrics({
          blockHeight: getMetricValue(text, 'substrate_block_height', { status: 'best' }),
          finalizedBlock: getMetricValue(text, 'substrate_block_height', { status: 'finalized' }),
          peersCount: getMetricValue(text, 'substrate_sub_libp2p_peers_count'),
          transactionsInPool: getMetricValue(text, 'substrate_ready_transactions_number'),
          comitSubmissions: getMetricValue(text, 'atlas_kernel_comit_submissions_total'),
          evmExecutions: getMetricValue(text, 'atlas_kernel_evm_executions_total'),
          svmExecutions: getMetricValue(text, 'atlas_kernel_svm_executions_total'),
          crossVmOps: getMetricValue(text, 'atlas_kernel_cross_vm_operations_total'),
          isConnected: true
        });
        setLastUpdate(new Date());
      } else {
        throw new Error('Failed to fetch');
      }
    } catch {
      // Use mock data when endpoint unavailable
      const mockBlock = 12847 + Math.floor(Date.now() / 6000) % 10000;
      setMetrics({
        blockHeight: mockBlock,
        finalizedBlock: mockBlock - 2,
        peersCount: 4 + Math.floor(Math.random() * 3),
        transactionsInPool: Math.floor(Math.random() * 15),
        comitSubmissions: 2847 + Math.floor(Math.random() * 50),
        evmExecutions: 15234 + Math.floor(Math.random() * 100),
        svmExecutions: 8921 + Math.floor(Math.random() * 50),
        crossVmOps: 1247 + Math.floor(Math.random() * 20),
        isConnected: false
      });
      setLastUpdate(new Date());
    } finally {
      setLoading(false);
    }
  }, [endpoint]);

  useEffect(() => {
    fetchMetrics();
    const interval = setInterval(fetchMetrics, refreshInterval);
    return () => clearInterval(interval);
  }, [fetchMetrics, refreshInterval]);

  const formatNumber = (num: number): string => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  if (loading) {
    return (
      <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 animate-pulse">
        <div className="h-8 w-48 bg-slate-700 rounded mb-4"></div>
        <div className="grid grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => (
            <div key={i} className="h-20 bg-slate-700 rounded-xl"></div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-gradient-to-r from-orange-500 to-red-500 rounded-xl flex items-center justify-center">
            <Server className="w-5 h-5 text-white" />
          </div>
          <div>
            <h3 className="text-xl font-bold text-white">Node Metrics</h3>
            <div className="flex items-center gap-2 text-sm">
              {metrics.isConnected ? (
                <span className="flex items-center gap-1 text-green-400">
                  <CheckCircle className="w-3 h-3" /> Live
                </span>
              ) : (
                <span className="flex items-center gap-1 text-yellow-400">
                  <AlertCircle className="w-3 h-3" /> Mock Data
                </span>
              )}
              <span className="text-gray-500">• {lastUpdate.toLocaleTimeString()}</span>
            </div>
          </div>
        </div>
        <Link
          href="/prometheus"
          className="flex items-center gap-2 px-4 py-2 bg-purple-600/20 text-purple-400 rounded-lg hover:bg-purple-600/30 transition-colors text-sm font-medium"
        >
          View All <ExternalLink className="w-4 h-4" />
        </Link>
      </div>

      {/* Primary Metrics */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
        <MetricBox
          icon={Box}
          label="Block Height"
          value={`#${formatNumber(metrics.blockHeight)}`}
          subValue={`Finalized: #${formatNumber(metrics.finalizedBlock)}`}
          color="blue"
        />
        <MetricBox
          icon={Network}
          label="Peers"
          value={metrics.peersCount.toString()}
          subValue="Connected nodes"
          color="green"
        />
        <MetricBox
          icon={Activity}
          label="TX Pool"
          value={metrics.transactionsInPool.toString()}
          subValue="Pending transactions"
          color="purple"
        />
        <MetricBox
          icon={Zap}
          label="Comits"
          value={formatNumber(metrics.comitSubmissions)}
          subValue="Total submissions"
          color="orange"
        />
      </div>

      {/* Atlas Kernel Stats */}
      <div className="bg-slate-900/30 rounded-xl p-4 border border-slate-700/50">
        <div className="flex items-center gap-2 mb-3">
          <Zap className="w-4 h-4 text-purple-400" />
          <span className="text-sm font-medium text-gray-400">Atlas Kernel Executions</span>
        </div>
        <div className="grid grid-cols-3 gap-4">
          <div className="text-center">
            <div className="text-xl font-bold text-blue-400">{formatNumber(metrics.evmExecutions)}</div>
            <div className="text-xs text-gray-500">EVM</div>
          </div>
          <div className="text-center">
            <div className="text-xl font-bold text-green-400">{formatNumber(metrics.svmExecutions)}</div>
            <div className="text-xs text-gray-500">SVM</div>
          </div>
          <div className="text-center">
            <div className="text-xl font-bold text-purple-400">{formatNumber(metrics.crossVmOps)}</div>
            <div className="text-xs text-gray-500">Cross-VM</div>
          </div>
        </div>
      </div>
    </div>
  );
}

function MetricBox({ 
  icon: Icon, 
  label, 
  value, 
  subValue, 
  color 
}: { 
  icon: React.ElementType; 
  label: string; 
  value: string; 
  subValue: string; 
  color: string;
}) {
  const colorClasses: Record<string, string> = {
    blue: 'text-blue-400',
    green: 'text-green-400',
    purple: 'text-purple-400',
    orange: 'text-orange-400',
  };

  return (
    <div className="bg-slate-900/30 rounded-xl p-4 border border-slate-700/50">
      <div className="flex items-center gap-2 mb-2">
        <Icon className={`w-4 h-4 ${colorClasses[color]}`} />
        <span className="text-xs text-gray-500">{label}</span>
      </div>
      <div className={`text-xl font-bold ${colorClasses[color]}`}>{value}</div>
      <div className="text-xs text-gray-500">{subValue}</div>
    </div>
  );
}
