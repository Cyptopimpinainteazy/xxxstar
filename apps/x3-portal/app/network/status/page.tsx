'use client';

import React from 'react';
import Link from 'next/link';
import {
  Activity,
  Server,
  Clock,
  Cpu,
  HardDrive,
  Wifi,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  TrendingUp,
  BarChart3,
  Globe,
  RefreshCw,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';
import { useNetworkStats, useAuthorities } from '@/hooks/useSubstrate';

const vmStats = [
  {
    name: 'EVM',
    status: 'operational',
    gasPrice: '1 gwei',
    tps: '2,450',
    latency: '12ms',
  },
  {
    name: 'SVM',
    status: 'operational',
    gasPrice: '0.00025 ATLAS',
    tps: '8,200',
    latency: '8ms',
  },
  {
    name: 'Atlas Kernel',
    status: 'operational',
    gasPrice: 'Variable',
    tps: '1,200',
    latency: '15ms',
  },
];

const services = [
  { name: 'Block Production', status: 'operational', uptime: '99.99%' },
  { name: 'EVM Execution', status: 'operational', uptime: '99.98%' },
  { name: 'SVM Execution', status: 'operational', uptime: '99.97%' },
  { name: 'Cross-VM Bridge', status: 'operational', uptime: '99.95%' },
  { name: 'RPC Endpoints', status: 'operational', uptime: '99.99%' },
  { name: 'WebSocket', status: 'operational', uptime: '99.98%' },
  { name: 'Archival Nodes', status: 'operational', uptime: '99.99%' },
  { name: 'State Sync', status: 'operational', uptime: '99.96%' },
];

const recentIncidents = [
  {
    date: '2024-01-15',
    title: 'RPC Latency Spike',
    status: 'resolved',
    duration: '12 minutes',
    description: 'Brief increase in RPC latency due to traffic spike. Resolved by adding capacity.',
  },
  {
    date: '2024-01-08',
    title: 'Scheduled Maintenance',
    status: 'completed',
    duration: '30 minutes',
    description: 'Planned maintenance for protocol upgrade. No service interruption.',
  },
];

const regions = [
  { name: 'North America', status: 'operational', nodes: 18, latency: '15ms' },
  { name: 'Europe', status: 'operational', nodes: 15, latency: '22ms' },
  { name: 'Asia Pacific', status: 'operational', nodes: 12, latency: '45ms' },
  { name: 'South America', status: 'operational', nodes: 4, latency: '68ms' },
  { name: 'Africa', status: 'operational', nodes: 3, latency: '85ms' },
];

function StatusIndicator({ status }: { status: string }) {
  switch (status) {
    case 'operational':
      return <CheckCircle2 className="w-5 h-5 text-emerald-500" />;
    case 'degraded':
      return <AlertTriangle className="w-5 h-5 text-yellow-500" />;
    case 'outage':
      return <XCircle className="w-5 h-5 text-red-500" />;
    default:
      return <CheckCircle2 className="w-5 h-5 text-gray-500" />;
  }
}

export default function NetworkStatusPage() {
  const { data: stats, isLoading: statsLoading } = useNetworkStats();
  const { data: authorities } = useAuthorities();

  const networkStats = [
    {
      label: 'Block Height',
      value: stats?.blockNumber?.toLocaleString() ?? '—',
      trend: stats ? '+1/6s' : '',
    },
    {
      label: 'Peers',
      value: stats?.peerCount?.toString() ?? '—',
      trend: '',
    },
    {
      label: 'Active Validators',
      value: (authorities?.length ?? stats?.authorityCount ?? 0).toString(),
      trend: '',
    },
    {
      label: 'Syncing',
      value: stats ? (stats.isSyncing ? 'Yes' : 'No') : '—',
      trend: '',
    },
  ];
  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-12 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-30">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="flex items-center justify-between">
            <div>
              <Link href="/network" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
                ← Back to Network
              </Link>
              <h1 className="text-3xl font-bold text-white mt-4">Network Status</h1>
              <p className="text-gray-400 mt-2">Real-time status of the X3 STAR network</p>
            </div>
            <div className="flex items-center space-x-4">
              <div className="glass-card p-4 flex items-center">
                <div className="w-3 h-3 rounded-full bg-emerald-500 animate-pulse mr-3" />
                  <span className="text-emerald-400 font-medium">
                    {stats && !stats.isSyncing ? 'All Systems Operational' : 'Checking Status'}
                  </span>
              </div>
              <button className="p-3 glass-card hover:bg-[#1a1a1a] transition-colors">
                <RefreshCw className="w-5 h-5 text-gray-400" />
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {networkStats.map((stat, index) => (
              <div key={index} className="glass-card p-6">
                <p className="text-sm text-gray-400 mb-1">{stat.label}</p>
                <div className="flex items-center justify-between">
                  <p className="text-2xl font-bold text-white">{stat.value}</p>
                  <span className="text-sm text-emerald-400">{stat.trend}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* VM Status */}
      <section className="py-8">
        <div className="container-wide">
          <h2 className="text-xl font-bold text-white mb-6">Virtual Machine Status</h2>
          
          <div className="grid md:grid-cols-3 gap-6">
            {vmStats.map((vm, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="font-semibold text-white">{vm.name}</h3>
                  <StatusIndicator status={vm.status} />
                </div>
                <div className="space-y-3">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Gas Price</span>
                    <span className="text-white">{vm.gasPrice}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Current TPS</span>
                    <span className="text-white">{vm.tps}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Latency</span>
                    <span className="text-emerald-400">{vm.latency}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Services */}
      <section className="py-8 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-xl font-bold text-white mb-6">Services</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {services.map((service, index) => (
              <div key={index} className="glass-card p-4 flex items-center justify-between">
                <div className="flex items-center">
                  <StatusIndicator status={service.status} />
                  <span className="ml-3 text-white">{service.name}</span>
                </div>
                <span className="text-sm text-gray-400">{service.uptime}</span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Regional Status */}
      <section className="py-8">
        <div className="container-wide">
          <h2 className="text-xl font-bold text-white mb-6">
            <Globe className="inline w-5 h-5 mr-2" />
            Regional Status
          </h2>
          
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-[#1a1a1a]">
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Region</th>
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Status</th>
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Nodes</th>
                  <th className="text-left py-4 px-4 text-sm font-medium text-gray-400">Avg Latency</th>
                </tr>
              </thead>
              <tbody>
                {regions.map((region, index) => (
                  <tr key={index} className="border-b border-[#0a0a0a] hover:bg-[#0a0a0a]">
                    <td className="py-4 px-4 text-white font-medium">{region.name}</td>
                    <td className="py-4 px-4">
                      <div className="flex items-center">
                        <StatusIndicator status={region.status} />
                        <span className="ml-2 text-emerald-400 capitalize">{region.status}</span>
                      </div>
                    </td>
                    <td className="py-4 px-4 text-gray-400">{region.nodes}</td>
                    <td className="py-4 px-4 text-gray-400">{region.latency}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      {/* Recent Incidents */}
      <section className="py-8 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-xl font-bold text-white mb-6">Recent Incidents</h2>
          
          {recentIncidents.length === 0 ? (
            <div className="glass-card p-8 text-center">
              <CheckCircle2 className="w-12 h-12 text-emerald-500 mx-auto mb-4" />
              <p className="text-gray-400">No recent incidents</p>
            </div>
          ) : (
            <div className="space-y-4">
              {recentIncidents.map((incident, index) => (
                <div key={index} className="glass-card p-6">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center">
                      <span className={`badge ${incident.status === 'resolved' ? 'badge-success' : 'badge-warning'}`}>
                        {incident.status}
                      </span>
                      <span className="ml-3 font-medium text-white">{incident.title}</span>
                    </div>
                    <span className="text-sm text-gray-400">{incident.date}</span>
                  </div>
                  <p className="text-sm text-gray-400">{incident.description}</p>
                  <p className="text-xs text-gray-500 mt-2">Duration: {incident.duration}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      </section>

      {/* Performance Graph Placeholder */}
      <section className="py-8">
        <div className="container-wide">
          <h2 className="text-xl font-bold text-white mb-6">
            <BarChart3 className="inline w-5 h-5 mr-2" />
            Performance (24h)
          </h2>
          
          <div className="glass-card p-6">
            <div className="h-64 flex items-center justify-center border border-[#1a1a1a] rounded-xl">
              <div className="text-center">
                <Activity className="w-12 h-12 text-gray-600 mx-auto mb-4" />
                <p className="text-gray-400">Performance graph visualization</p>
                <p className="text-sm text-gray-500">Real-time TPS and latency metrics</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Subscribe */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-xl font-bold text-white mb-4">
            Stay Updated
          </h2>
          <p className="text-gray-400 mb-6">
            Subscribe to receive notifications about network status changes and maintenance windows.
          </p>
          <div className="flex justify-center gap-4">
            <Link href="/community/forum" className="btn-primary">
              Subscribe to Alerts
            </Link>
            <Link href="/developers/api" className="btn-secondary">
              Status API
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
