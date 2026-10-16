'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';

interface EvolutionMetrics {
  current_epoch: number;
  total_mutations: number;
  active_proposals: number;
  auto_evolution_enabled: boolean;
  last_mutation_block: number;
  mutation_success_rate: number;
}

interface MutationProposal {
  id: number;
  mutation_type: string;
  metric_id: string;
  proposed_value: string;
  reason: string;
  status: 'pending' | 'approved' | 'rejected' | 'executed';
  proposed_at: number;
  votes_for: number;
  votes_against: number;
}

export default function EvolutionDashboard() {
  const [metrics, setMetrics] = useState<EvolutionMetrics | null>(null);
  const [proposals, setProposals] = useState<MutationProposal[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Mock data for now
    setMetrics({
      current_epoch: 1247,
      total_mutations: 89,
      active_proposals: 3,
      auto_evolution_enabled: true,
      last_mutation_block: 1234567,
      mutation_success_rate: 94.2,
    });

    setProposals([
      {
        id: 1,
        mutation_type: 'GasPrice',
        metric_id: 'avg_gas_price',
        proposed_value: '15 gwei',
        reason: 'High network congestion detected',
        status: 'pending',
        proposed_at: 1234500,
        votes_for: 45,
        votes_against: 12,
      },
      {
        id: 2,
        mutation_type: 'BlockWeight',
        metric_id: 'block_utilization',
        proposed_value: '+10%',
        reason: 'Consistent 95%+ block fullness',
        status: 'approved',
        proposed_at: 1234000,
        votes_for: 78,
        votes_against: 5,
      },
      {
        id: 3,
        mutation_type: 'VmSwitch',
        metric_id: 'evm_demand',
        proposed_value: 'Increase EVM allocation',
        reason: 'EVM transactions up 300%',
        status: 'pending',
        proposed_at: 1234200,
        votes_for: 32,
        votes_against: 28,
      },
    ]);

    setLoading(false);
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900 flex items-center justify-center">
        <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-purple-500"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900">
      {/* Header */}
      <header className="border-b border-purple-800/30 backdrop-blur-xl bg-black/20">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <Link href="/" className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent">
                X3 Sphere
              </Link>
              <span className="text-purple-400/60">/</span>
              <span className="text-white font-medium">Evolution Dashboard</span>
            </div>
            <nav className="flex items-center space-x-6">
              <Link href="/x3/swarm" className="text-gray-400 hover:text-white transition">Swarm</Link>
              <Link href="/x3/verifier" className="text-gray-400 hover:text-white transition">Verifier</Link>
              <Link href="/x3/scripts" className="text-gray-400 hover:text-white transition">Scripts</Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-6 py-8">
        {/* Hero Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          <StatCard
            title="Current Epoch"
            value={metrics?.current_epoch.toLocaleString() || '0'}
            icon="🕐"
            trend="+1 every 100 blocks"
          />
          <StatCard
            title="Total Mutations"
            value={metrics?.total_mutations.toString() || '0'}
            icon="🧬"
            trend={`${metrics?.mutation_success_rate}% success rate`}
          />
          <StatCard
            title="Active Proposals"
            value={metrics?.active_proposals.toString() || '0'}
            icon="📋"
            trend="Awaiting votes"
          />
          <StatCard
            title="Auto-Evolution"
            value={metrics?.auto_evolution_enabled ? 'ENABLED' : 'DISABLED'}
            icon={metrics?.auto_evolution_enabled ? '✅' : '⏸️'}
            trend="AI-driven optimization"
            highlight={metrics?.auto_evolution_enabled}
          />
        </div>

        {/* Main Content Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Mutation Timeline */}
          <div className="lg:col-span-2 bg-black/40 rounded-2xl border border-purple-800/30 p-6">
            <h2 className="text-xl font-semibold text-white mb-6 flex items-center gap-2">
              <span className="text-2xl">🧬</span> Mutation Proposals
            </h2>
            <div className="space-y-4">
              {proposals.map((proposal) => (
                <ProposalCard key={proposal.id} proposal={proposal} />
              ))}
            </div>
          </div>

          {/* Live Metrics */}
          <div className="bg-black/40 rounded-2xl border border-purple-800/30 p-6">
            <h2 className="text-xl font-semibold text-white mb-6 flex items-center gap-2">
              <span className="text-2xl">📊</span> Live Metrics
            </h2>
            <div className="space-y-4">
              <MetricRow label="Gas Price" value="12 gwei" trend="stable" />
              <MetricRow label="Block Weight" value="78%" trend="up" />
              <MetricRow label="EVM Load" value="65%" trend="up" />
              <MetricRow label="SVM Load" value="23%" trend="down" />
              <MetricRow label="Validator Count" value="127" trend="stable" />
              <MetricRow label="MEV Extracted" value="45.2 ETH" trend="up" />
            </div>

            <div className="mt-6 pt-6 border-t border-purple-800/30">
              <h3 className="text-sm font-medium text-gray-400 mb-3">AI Agent Status</h3>
              <div className="flex items-center justify-between text-sm">
                <span className="text-gray-300">Optimization Agent</span>
                <span className="flex items-center gap-2 text-green-400">
                  <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></span>
                  Active
                </span>
              </div>
              <div className="flex items-center justify-between text-sm mt-2">
                <span className="text-gray-300">Security Monitor</span>
                <span className="flex items-center gap-2 text-green-400">
                  <span className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></span>
                  Active
                </span>
              </div>
            </div>
          </div>
        </div>

        {/* Evolution History Chart Placeholder */}
        <div className="mt-8 bg-black/40 rounded-2xl border border-purple-800/30 p-6">
          <h2 className="text-xl font-semibold text-white mb-6 flex items-center gap-2">
            <span className="text-2xl">📈</span> Evolution History
          </h2>
          <div className="h-64 flex items-center justify-center text-gray-500">
            <div className="text-center">
              <p className="text-lg">Interactive Chart Coming Soon</p>
              <p className="text-sm mt-2">Visualizing mutation timeline and network adaptation</p>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

function StatCard({ title, value, icon, trend, highlight }: {
  title: string;
  value: string;
  icon: string;
  trend: string;
  highlight?: boolean;
}) {
  return (
    <div className={`bg-black/40 rounded-2xl border ${highlight ? 'border-green-500/50' : 'border-purple-800/30'} p-6`}>
      <div className="flex items-center justify-between mb-2">
        <span className="text-gray-400 text-sm">{title}</span>
        <span className="text-2xl">{icon}</span>
      </div>
      <div className={`text-3xl font-bold ${highlight ? 'text-green-400' : 'text-white'}`}>
        {value}
      </div>
      <div className="text-sm text-gray-500 mt-1">{trend}</div>
    </div>
  );
}

function ProposalCard({ proposal }: { proposal: MutationProposal }) {
  const statusColors = {
    pending: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    approved: 'bg-green-500/20 text-green-400 border-green-500/30',
    rejected: 'bg-red-500/20 text-red-400 border-red-500/30',
    executed: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
  };

  return (
    <div className="bg-black/20 rounded-xl border border-purple-800/20 p-4">
      <div className="flex items-start justify-between mb-2">
        <div>
          <span className="text-white font-medium">{proposal.mutation_type}</span>
          <span className="text-gray-500 text-sm ml-2">#{proposal.id}</span>
        </div>
        <span className={`px-2 py-1 rounded-full text-xs border ${statusColors[proposal.status]}`}>
          {proposal.status.toUpperCase()}
        </span>
      </div>
      <p className="text-gray-400 text-sm mb-3">{proposal.reason}</p>
      <div className="flex items-center justify-between text-sm">
        <div className="flex items-center gap-4">
          <span className="text-green-400">👍 {proposal.votes_for}</span>
          <span className="text-red-400">👎 {proposal.votes_against}</span>
        </div>
        <span className="text-gray-500">Block #{proposal.proposed_at}</span>
      </div>
    </div>
  );
}

function MetricRow({ label, value, trend }: { label: string; value: string; trend: 'up' | 'down' | 'stable' }) {
  const trendIcons = {
    up: '↑',
    down: '↓',
    stable: '→',
  };
  const trendColors = {
    up: 'text-green-400',
    down: 'text-red-400',
    stable: 'text-gray-400',
  };

  return (
    <div className="flex items-center justify-between py-2 border-b border-purple-800/20 last:border-0">
      <span className="text-gray-400">{label}</span>
      <div className="flex items-center gap-2">
        <span className="text-white font-medium">{value}</span>
        <span className={trendColors[trend]}>{trendIcons[trend]}</span>
      </div>
    </div>
  );
}
