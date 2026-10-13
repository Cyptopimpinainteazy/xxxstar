'use client';

import React from 'react';
import Link from 'next/link';
import {
  Server,
  Shield,
  BarChart3,
  Activity,
  Globe,
  Users,
  ArrowRight,
  ExternalLink,
} from 'lucide-react';
import { HexagonCluster } from '../../components/frontend/ui/Logo';
import { useNetworkStats, useAuthorities } from '@/hooks/useSubstrate';

const sections = [
  {
    title: 'Become a Validator',
    description: 'Secure the network, earn rewards, and participate in governance',
    icon: <Shield className="w-8 h-8" />,
    href: '/network/validators',
    features: ['~8% APY Rewards', 'Governance Rights', 'Network Security'],
    color: 'from-orange-500 to-amber-500',
  },
  {
    title: 'RPC Providers',
    description: 'Connect to X3 STAR through reliable RPC infrastructure',
    icon: <Server className="w-8 h-8" />,
    href: '/network/rpc-providers',
    features: ['Free & Paid Options', 'Low Latency', 'Full Archive'],
    color: 'from-blue-500 to-cyan-500',
  },
  {
    title: 'Network Status',
    description: 'Real-time network health and performance metrics',
    icon: <Activity className="w-8 h-8" />,
    href: '/network/status',
    features: ['Live Monitoring', 'VM Status', 'Incident History'],
    color: 'from-emerald-500 to-green-500',
  },
  {
    title: 'On & Off Ramps',
    description: 'Buy and sell ATLAS through trusted exchanges and services',
    icon: <Globe className="w-8 h-8" />,
    href: '/network/ramps',
    features: ['Multiple Currencies', 'Low Fees', 'Fast Settlement'],
    color: 'from-purple-500 to-indigo-500',
  },
];

const explorers = [
  {
    name: 'X3scan',
    description: 'Official X3 STAR block explorer with full transaction history',
    url: '#',
    features: ['Full History', 'Contract Verification', 'API Access'],
  },
  {
    name: 'x3FM',
    description: 'Community explorer with advanced analytics and charting',
    url: '#',
    features: ['Analytics', 'Charts', 'Portfolio Tracking'],
  },
  {
    name: 'X3 Explorer',
    description: 'Simple and fast explorer for quick lookups',
    url: '#',
    features: ['Fast Search', 'Mobile Friendly', 'Real-time Updates'],
  },
  {
    name: 'Orb',
    description: 'Developer-focused explorer with debugging tools',
    url: '#',
    features: ['Debug Tools', 'Trace Calls', 'Gas Profiling'],
  },
];

const indexers = [
  {
    name: 'X3 Indexer',
    description: 'High-performance blockchain indexer for dApp developers',
    features: ['GraphQL API', 'Real-time Events', 'Custom Schemas'],
  },
  {
    name: 'SubQuery',
    description: 'Open-source indexing framework for Substrate chains',
    features: ['Flexible Indexing', 'Multi-chain', 'Open Source'],
  },
  {
    name: 'The Graph',
    description: 'Decentralized indexing protocol (coming soon)',
    features: ['Subgraphs', 'Decentralized', 'Community Curated'],
  },
];

export default function NetworkPage() {
  const { data: stats, isLoading: statsLoading, error: statsError } = useNetworkStats();
  const { data: authorities } = useAuthorities();

  const derivedNetworkStats = [
    {
      label: 'Block Height',
      value: stats?.blockNumber?.toLocaleString() ?? '—',
      icon: <Activity className="w-5 h-5" />,
    },
    {
      label: 'Active Validators',
      value: (authorities?.length ?? stats?.authorityCount ?? 0).toString(),
      icon: <Shield className="w-5 h-5" />,
    },
    {
      label: 'Peers',
      value: stats?.peerCount?.toString() ?? '—',
      icon: <Server className="w-5 h-5" />,
    },
    {
      label: 'Sync Status',
      value: statsError
        ? 'Error'
        : statsLoading
          ? 'Loading'
          : stats?.isSyncing
            ? 'Syncing'
            : 'Healthy',
      icon: <Activity className="w-5 h-5" />,
    },
  ];

  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero with unique Network section header */}
      <section className="py-20 relative overflow-hidden page-header-network">
        <div className="absolute inset-0 bg-gradient-to-br from-blue-950/20 via-black to-indigo-950/10" />
        <div className="absolute inset-0 mesh-gradient opacity-10" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-20">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <div className="inline-flex items-center px-3 py-1 rounded-full bg-blue-500/10 border border-blue-500/30 mb-4">
              <Globe className="w-4 h-4 mr-2 text-blue-400" />
              <span className="text-sm text-blue-300">Network</span>
            </div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              X3 STAR <span className="text-blue-400">Network</span>
            </h1>
            <p className="text-xl text-gray-500 mb-8">
              A decentralized network of validators, RPC providers, and infrastructure 
              services powering the dual-VM blockchain ecosystem.
            </p>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="py-8 border-t border-[#1a1a1a] bg-black">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {derivedNetworkStats.map((stat, index) => (
              <div key={index} className="glass-card p-6 flex items-center">
                <div className="p-3 rounded-xl bg-orange-500/20 text-orange-400 mr-4">
                  {stat.icon}
                </div>
                <div>
                  <p className="text-sm text-gray-500">{stat.label}</p>
                  <p className="text-2xl font-bold text-white">
                    {statsLoading && stat.label !== 'Sync Status' ? '…' : stat.value}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Main Sections */}
      <section className="py-16 bg-black">
        <div className="container-wide">
          <div className="grid md:grid-cols-2 gap-6">
            {sections.map((section, index) => (
              <Link
                key={index}
                href={section.href}
                className="glass-card-hover p-6 card-lift group"
              >
                <div className={`p-3 rounded-xl bg-gradient-to-br ${section.color} bg-opacity-20 w-fit mb-4`}>
                  <span className="text-white">{section.icon}</span>
                </div>
                <h3 className="text-xl font-semibold text-white mb-2 group-hover:text-orange-400 transition-colors">
                  {section.title}
                </h3>
                <p className="text-gray-500 mb-4">{section.description}</p>
                <div className="flex flex-wrap gap-2">
                  {section.features.map((feature, i) => (
                    <span key={i} className="badge badge-default">{feature}</span>
                  ))}
                </div>
              </Link>
            ))}
          </div>
        </div>
      </section>

      {/* Explorers */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Block Explorers</h2>
          
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {explorers.map((explorer, index) => (
              <a
                key={index}
                href={explorer.url}
                className="glass-card p-6 hover:border-orange-500/30 transition-colors group"
              >
                <div className="flex items-center justify-between mb-4">
                  <h3 className="font-semibold text-white group-hover:text-orange-400 transition-colors">
                    {explorer.name}
                  </h3>
                  <ExternalLink className="w-4 h-4 text-gray-600 group-hover:text-orange-400 transition-colors" />
                </div>
                <p className="text-sm text-gray-500 mb-4">{explorer.description}</p>
                <div className="flex flex-wrap gap-2">
                  {explorer.features.map((feature, i) => (
                    <span key={i} className="text-xs px-2 py-1 rounded bg-[#1a1a1a] text-gray-400">
                      {feature}
                    </span>
                  ))}
                </div>
              </a>
            ))}
          </div>
        </div>
      </section>

      {/* Indexers */}
      <section className="py-16 bg-black">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Data Indexers</h2>
          
          <div className="grid md:grid-cols-3 gap-6">
            {indexers.map((indexer, index) => (
              <div key={index} className="glass-card p-6">
                <h3 className="font-semibold text-white mb-2">{indexer.name}</h3>
                <p className="text-sm text-gray-500 mb-4">{indexer.description}</p>
                <div className="flex flex-wrap gap-2">
                  {indexer.features.map((feature, i) => (
                    <span key={i} className="badge badge-default">{feature}</span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a] bg-black">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-3xl font-bold text-white mb-4">
            Help Build the Network
          </h2>
          <p className="text-gray-500 mb-8">
            Become a validator, run an RPC node, or build infrastructure tools for the X3 STAR ecosystem.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/network/validators" className="btn-primary">
              Become a Validator
            </Link>
            <Link href="/community/grants" className="btn-secondary">
              Infrastructure Grants
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
