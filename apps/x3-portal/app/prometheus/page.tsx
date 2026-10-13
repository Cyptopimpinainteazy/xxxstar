'use client';

import React, { useState } from 'react';
import PrometheusMetrics from '@/components/prometheus/PrometheusMetrics';
import {
  BarChart3,
  Server,
  Activity,
  Settings,
  ExternalLink,
  Info
} from 'lucide-react';

// Prometheus endpoint configurations
const PROMETHEUS_ENDPOINTS = {
  local: 'http://127.0.0.1:9615/metrics',
  testnet: 'http://rpc.testnet.atlas-sphere.io:9615/metrics',
};

export default function PrometheusPage() {
  const [selectedEndpoint, setSelectedEndpoint] = useState<keyof typeof PROMETHEUS_ENDPOINTS>('local');
  const [customEndpoint, setCustomEndpoint] = useState('');
  const [useCustom, setUseCustom] = useState(false);

  const currentEndpoint = useCustom && customEndpoint 
    ? customEndpoint 
    : PROMETHEUS_ENDPOINTS[selectedEndpoint];

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900">
      {/* Hero Section */}
      <div className="relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-r from-purple-600/20 to-blue-600/20 backdrop-blur-3xl"></div>
        <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-12">
          <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6">
            <div>
              <div className="flex items-center gap-3 mb-4">
                <div className="w-12 h-12 bg-gradient-to-r from-orange-500 to-red-500 rounded-xl flex items-center justify-center">
                  <BarChart3 className="w-7 h-7 text-white" />
                </div>
                <h1 className="text-4xl md:text-5xl font-bold bg-gradient-to-r from-white via-orange-200 to-red-200 bg-clip-text text-transparent">
                  Prometheus Metrics
                </h1>
              </div>
              <p className="text-xl text-gray-300 max-w-2xl">
                Real-time blockchain node metrics, system resources, and Atlas Kernel performance data
              </p>
            </div>
            
            {/* Endpoint Selector */}
            <div className="flex flex-col gap-3">
              <div className="flex items-center gap-2 bg-slate-800/50 rounded-xl p-1 border border-slate-700/50">
                {Object.keys(PROMETHEUS_ENDPOINTS).map((key) => (
                  <button
                    key={key}
                    onClick={() => {
                      setSelectedEndpoint(key as keyof typeof PROMETHEUS_ENDPOINTS);
                      setUseCustom(false);
                    }}
                    className={`px-4 py-2 rounded-lg font-medium transition-all capitalize ${
                      selectedEndpoint === key && !useCustom
                        ? 'bg-purple-600 text-white'
                        : 'text-gray-400 hover:text-white hover:bg-slate-700/50'
                    }`}
                  >
                    {key}
                  </button>
                ))}
                <button
                  onClick={() => setUseCustom(true)}
                  className={`px-4 py-2 rounded-lg font-medium transition-all ${
                    useCustom
                      ? 'bg-purple-600 text-white'
                      : 'text-gray-400 hover:text-white hover:bg-slate-700/50'
                  }`}
                >
                  Custom
                </button>
              </div>
              
              {useCustom && (
                <input
                  type="text"
                  placeholder="http://localhost:9615/metrics"
                  value={customEndpoint}
                  onChange={(e) => setCustomEndpoint(e.target.value)}
                  className="bg-slate-900/50 border border-slate-700 rounded-lg px-4 py-2 text-white placeholder-gray-500"
                />
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Info Banner */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
        <div className="bg-gradient-to-r from-blue-600/10 to-purple-600/10 rounded-xl p-4 border border-blue-500/20 flex items-start gap-4">
          <Info className="w-6 h-6 text-blue-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <h4 className="text-white font-medium mb-1">About Prometheus Metrics</h4>
            <p className="text-gray-400 text-sm">
              Atlas Sphere nodes expose Substrate-standard Prometheus metrics plus custom Atlas Kernel instrumentation. 
              Metrics include block production, consensus health, transaction pool status, network bandwidth, 
              and dual-VM execution statistics. Data auto-refreshes every 5 seconds.
            </p>
          </div>
          <a
            href={currentEndpoint}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 px-4 py-2 bg-slate-800/50 rounded-lg text-gray-400 hover:text-white transition-colors"
          >
            <ExternalLink className="w-4 h-4" />
            Raw Metrics
          </a>
        </div>
      </div>

      {/* Metrics Component */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <PrometheusMetrics endpoint={currentEndpoint} refreshInterval={5000} />
      </div>

      {/* Quick Links */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h3 className="text-2xl font-bold text-white mb-6">Related Dashboards</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <a
            href="/metrics"
            className="bg-slate-800/30 rounded-xl p-6 border border-slate-700/50 hover:border-purple-500/50 transition-all group"
          >
            <div className="flex items-center gap-3 mb-4">
              <Activity className="w-6 h-6 text-purple-400" />
              <h4 className="text-lg font-bold text-white">DeFi Metrics</h4>
            </div>
            <p className="text-gray-400 text-sm">Protocol performance, TVL, and trading analytics</p>
          </a>
          
          <a
            href="/network"
            className="bg-slate-800/30 rounded-xl p-6 border border-slate-700/50 hover:border-purple-500/50 transition-all group"
          >
            <div className="flex items-center gap-3 mb-4">
              <Server className="w-6 h-6 text-blue-400" />
              <h4 className="text-lg font-bold text-white">Network Status</h4>
            </div>
            <p className="text-gray-400 text-sm">Peer connections, sync status, and validators</p>
          </a>
          
          <a
            href="/ai-swarm"
            className="bg-slate-800/30 rounded-xl p-6 border border-slate-700/50 hover:border-purple-500/50 transition-all group"
          >
            <div className="flex items-center gap-3 mb-4">
              <Settings className="w-6 h-6 text-green-400" />
              <h4 className="text-lg font-bold text-white">GPU Swarm</h4>
            </div>
            <p className="text-gray-400 text-sm">AI task distribution and GPU resource monitoring</p>
          </a>
        </div>
      </div>
    </div>
  );
}
