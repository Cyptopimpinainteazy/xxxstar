'use client';

import React, { useState, useEffect } from 'react';
import dynamic from 'next/dynamic';
import { 
  Globe, 
  Zap, 
  Shield, 
  TrendingUp, 
  Activity,
  ArrowRight,
  ChevronRight,
  Cpu,
  Network,
  DollarSign,
  Target,
  Layers
} from 'lucide-react';

// Dynamic import for Prometheus widget (client-side only)
const PrometheusWidget = dynamic(
  () => import('@/components/prometheus/PrometheusWidget'),
  { 
    ssr: false,
    loading: () => (
      <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 animate-pulse h-64">
        <div className="h-8 w-48 bg-slate-700 rounded mb-4"></div>
        <div className="grid grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => (
            <div key={i} className="h-20 bg-slate-700 rounded-xl"></div>
          ))}
        </div>
      </div>
    )
  }
);

// Dynamic imports for quantum showcase components
const HorizontalScrollTrack = dynamic(
  () => import('@/components/quantum/HorizontalGallery').then(mod => ({ default: mod.HorizontalScrollTrack })),
  { ssr: false, loading: () => <div className="h-80 animate-pulse bg-slate-800/50 rounded-2xl" /> }
);

const FloatingCardsGrid = dynamic(
  () => import('@/components/quantum/ProjectShowcase').then(mod => ({ default: mod.FloatingCardsGrid })),
  { ssr: false, loading: () => <div className="h-64 animate-pulse bg-slate-800/50 rounded-2xl" /> }
);

const ProjectShowcase = dynamic(
  () => import('@/components/quantum/ProjectShowcase').then(mod => ({ default: mod.ProjectShowcase })),
  { ssr: false, loading: () => <div className="h-96 animate-pulse bg-slate-800/50 rounded-2xl" /> }
);

const ScotchCassetteShowcase = dynamic(
  () => import('@/components/quantum/ScotchVHSShowcase').then(mod => ({ default: mod.ScotchCassetteShowcase })),
  { ssr: false, loading: () => <div className="h-96 animate-pulse bg-slate-800/50 rounded-2xl" /> }
);

interface Feature {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  stats: string;
  color: string;
}

interface MetricItem {
  label: string;
  value: string;
  icon: React.ComponentType<{ className?: string }>;
}

interface NavLink {
  title: string;
  description: string;
  href: string;
  color: string;
}

const formatNumber = (num: number) => {
  if (num >= 1e9) return '$' + (num / 1e9).toFixed(1) + 'B';
  if (num >= 1e6) return '$' + (num / 1e6).toFixed(1) + 'M';
  if (num >= 1e3) return '$' + (num / 1e3).toFixed(1) + 'K';
  return '$' + num.toFixed(0);
};

const featureGrid: Feature[] = [
  {
    icon: Globe,
    title: '103+ Chain Support',
    description: 'Universal EVM compatibility across all major networks',
    stats: '103 networks',
    color: 'from-blue-500 to-cyan-500'
  },
  {
    icon: Cpu,
    title: 'GPU AI Swarm',
    description: 'Real-time optimization and arbitrage detection',
    stats: '2,847 active nodes',
    color: 'from-purple-500 to-pink-500'
  },
  {
    icon: Shield,
    title: 'MEV Protection',
    description: 'Multi-layer security with private mempools',
    stats: '99.9% protection',
    color: 'from-green-500 to-emerald-500'
  },
  {
    icon: Zap,
    title: 'Atomic Swaps',
    description: 'Sub-1% slippage protection guaranteed',
    stats: '99.9% success rate',
    color: 'from-yellow-500 to-orange-500'
  },
  {
    icon: DollarSign,
    title: 'Auto Treasury',
    description: 'Sophisticated fee distribution system',
    stats: '40% to DAO',
    color: 'from-indigo-500 to-purple-500'
  },
  {
    icon: Layers,
    title: 'Full DeFi Suite',
    description: 'Lending, staking, launchpads, NFTs and more',
    stats: '15+ protocols',
    color: 'from-red-500 to-pink-500'
  }
];

const navLinks: NavLink[] = [
  { title: 'Cross-Chain Bridge', description: 'Atomic swaps across 103+ networks', href: '/bridge', color: 'from-blue-500 to-cyan-500' },
  { title: 'Lending Protocol', description: 'Aave-style lending with AI optimization', href: '/earn', color: 'from-green-500 to-emerald-500' },
  { title: 'Launchpad', description: 'Token and NFT launches with blockspace auctions', href: '/launch', color: 'from-purple-500 to-pink-500' },
  { title: 'AI Swarm', description: 'GPU-powered strategy optimization', href: '/x3/swarm', color: 'from-orange-500 to-red-500' }
];

function FeatureCard({ feature }: { feature: Feature }) {
  const IconComponent = feature.icon;
  return (
    <div className="group relative bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 hover:border-purple-500/50 transition-all duration-300 hover:transform hover:scale-105">
      <div className={'w-16 h-16 bg-gradient-to-r ' + feature.color + ' rounded-xl flex items-center justify-center mb-4 group-hover:scale-110 transition-transform'}>
        <IconComponent className="w-8 h-8 text-white" />
      </div>
      <h3 className="text-xl font-bold text-white mb-2">{feature.title}</h3>
      <p className="text-gray-400 mb-4">{feature.description}</p>
      <div className="flex items-center gap-2 text-purple-400 font-semibold">
        <span className="text-sm">{feature.stats}</span>
        <ChevronRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
      </div>
    </div>
  );
}

function MetricCard({ metric }: { metric: MetricItem }) {
  const IconComponent = metric.icon;
  return (
    <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 text-center">
      <IconComponent className="w-12 h-12 text-purple-400 mx-auto mb-4" />
      <div className="text-3xl font-bold text-white mb-2">{metric.value}</div>
      <div className="text-gray-400">{metric.label}</div>
    </div>
  );
}

function NavCard({ item }: { item: NavLink }) {
  return (
    <a href={item.href} className="group block bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 hover:border-purple-500/50 transition-all duration-300 hover:transform hover:scale-105">
      <div className={'w-12 h-12 bg-gradient-to-r ' + item.color + ' rounded-xl flex items-center justify-center mb-4 group-hover:scale-110 transition-transform'}>
        <ArrowRight className="w-6 h-6 text-white" />
      </div>
      <h3 className="text-xl font-bold text-white mb-2">{item.title}</h3>
      <p className="text-gray-400">{item.description}</p>
    </a>
  );
}

export default function HomePage() {
  const [metrics, setMetrics] = useState({
    tvl: 2800000000,
    chains: 103,
    users: 127000,
    uptime: 99.9,
    transactions: 2847392,
    atomicSuccess: 99.9,
    updateLatency: 0.3
  });

  const [activities, setActivities] = useState([
    { id: 1, text: 'Cross-chain swap: ETH to USDC on Arbitrum', amount: '$45,230', time: '2s ago' },
    { id: 2, text: 'Position opened: $125K USDC collateral', amount: '95K USDC', time: '5s ago' },
    { id: 3, text: 'MEV-protected arbitrage executed', amount: '+$2,340', time: '8s ago' },
    { id: 4, text: 'Token launch completed: 2.3M raised', amount: '$2.3M', time: '12s ago' },
    { id: 5, text: 'Validator stake: 50K ATLA delegated', amount: '50K ATLA', time: '15s ago' }
  ]);

  useEffect(() => {
    const interval = setInterval(() => {
      setMetrics(prev => ({
        ...prev,
        tvl: prev.tvl + Math.floor(Math.random() * 1000000),
        users: prev.users + Math.floor(Math.random() * 10),
        transactions: prev.transactions + Math.floor(Math.random() * 5),
      }));
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const activityTexts = [
      'Cross-chain swap: BTC to ETH on Polygon',
      'Liquidation protected: $89K position saved',
      'AI arbitrage: +$1,890 profit executed',
      'NFT launch: 100 items sold in 3min',
      'Staking rewards: 15.7% APY claimed'
    ];
    const interval = setInterval(() => {
      const newActivity = {
        id: Date.now(),
        text: activityTexts[Math.floor(Math.random() * 5)],
        amount: '$' + (Math.random() * 100000).toFixed(0),
        time: 'Just now'
      };
      setActivities(prev => [newActivity, ...prev.slice(0, 4)]);
    }, 4000);
    return () => clearInterval(interval);
  }, []);

  const performanceMetrics: MetricItem[] = [
    { label: 'Uptime', value: '99.9%', icon: Activity },
    { label: 'Update Latency', value: '<0.5s', icon: Zap },
    { label: 'Atomic Success', value: '99.9%+', icon: Target },
    { label: 'TVL', value: formatNumber(metrics.tvl), icon: TrendingUp }
  ];

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900">
      {/* Hero Section */}
      <div className="relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-r from-purple-600/20 to-blue-600/20 backdrop-blur-3xl"></div>
        <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16">
          <div className="text-center">
            <h1 className="text-6xl md:text-8xl font-bold bg-gradient-to-r from-white via-purple-200 to-blue-200 bg-clip-text text-transparent mb-6">
              Atlas Sphere
            </h1>
            <p className="text-xl md:text-2xl text-gray-300 mb-8 max-w-4xl mx-auto">
              The Worlds Most Advanced Multi-Chain DeFi Ecosystem
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <button className="bg-gradient-to-r from-purple-600 to-blue-600 text-white px-8 py-4 rounded-xl font-semibold text-lg hover:from-purple-700 hover:to-blue-700 transition-all transform hover:scale-105 flex items-center gap-2">
                Launch App <ArrowRight className="w-5 h-5" />
              </button>
              <button className="border border-purple-500 text-purple-300 px-8 py-4 rounded-xl font-semibold text-lg hover:bg-purple-500/10 transition-all">
                View Documentation
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Live Metrics */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="bg-gradient-to-r from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
          <div className="flex items-center gap-3 mb-6">
            <div className="w-3 h-3 bg-green-400 rounded-full animate-pulse"></div>
            <h2 className="text-2xl font-bold text-white">Live Network Status</h2>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-7 gap-6">
            <div className="text-center">
              <div className="text-2xl md:text-3xl font-bold text-white">{formatNumber(metrics.tvl)}</div>
              <div className="text-sm text-gray-400">Total Value Locked</div>
            </div>
            <div className="text-center">
              <div className="text-2xl md:text-3xl font-bold text-blue-400">{metrics.chains}+</div>
              <div className="text-sm text-gray-400">Supported Chains</div>
            </div>
            <div className="text-center">
              <div className="text-2xl md:text-3xl font-bold text-purple-400">{(metrics.users / 1000).toFixed(0)}K+</div>
              <div className="text-sm text-gray-400">Active Users</div>
            </div>
            <div className="text-center">
              <div className="text-2xl md:text-3xl font-bold text-green-400">{metrics.uptime}%</div>
              <div className="text-sm text-gray-400">Uptime</div>
            </div>
            <div className="text-center">
              <div className="text-2xl md:text-3xl font-bold text-yellow-400">{(metrics.transactions / 1000).toFixed(0)}K+</div>
              <div className="text-sm text-gray-400">Transactions</div>
            </div>
            <div className="text-center">
              <div className="text-2xl md:text-3xl font-bold text-cyan-400">{metrics.atomicSuccess}%</div>
              <div className="text-sm text-gray-400">Atomic Success</div>
            </div>
            <div className="text-center">
              <div className="text-2xl md:text-3xl font-bold text-pink-400">{metrics.updateLatency}s</div>
              <div className="text-sm text-gray-400">Update Speed</div>
            </div>
          </div>
        </div>
      </div>

      {/* Prometheus Node Metrics Widget */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
        <PrometheusWidget endpoint="http://127.0.0.1:9615/metrics" refreshInterval={10000} />
      </div>

      {/* Feature Grid */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">Production-Ready Features</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
          {featureGrid.map((feature, index) => (
            <FeatureCard key={index} feature={feature} />
          ))}
        </div>
      </div>

      {/* Performance Metrics */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">Performance Excellence</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {performanceMetrics.map((metric, index) => (
            <MetricCard key={index} metric={metric} />
          ))}
        </div>
      </div>

      {/* Live Activity Feed */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
          <div className="flex items-center gap-3 mb-6">
            <Activity className="w-6 h-6 text-green-400 animate-pulse" />
            <h2 className="text-2xl font-bold text-white">Live Activity Feed</h2>
            <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
          </div>
          <div className="space-y-4">
            {activities.map((activity) => (
              <div key={activity.id} className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl border border-slate-600/20">
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 bg-gradient-to-r from-purple-500 to-blue-500 rounded-full flex items-center justify-center">
                    <Network className="w-5 h-5 text-white" />
                  </div>
                  <div>
                    <div className="text-white font-medium">{activity.text}</div>
                    <div className="text-gray-400 text-sm">{activity.time}</div>
                  </div>
                </div>
                <div className="text-green-400 font-bold">{activity.amount}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Quick Navigation */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">Explore the Ecosystem</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {navLinks.map((item, index) => (
            <NavCard key={index} item={item} />
          ))}
        </div>
      </div>

      {/* Horizontal Scroll Gallery */}
      <div className="py-12 overflow-hidden">
        <HorizontalScrollTrack 
          title="ATLAS ECOSYSTEM"
          speed={0.8}
          direction="left"
          showOverlay={true}
        />
      </div>

      {/* Floating Capabilities Grid */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">
          <span className="bg-gradient-to-r from-purple-400 to-cyan-400 bg-clip-text text-transparent">
            Core Capabilities
          </span>
        </h2>
        <FloatingCardsGrid />
      </div>

      {/* Project Showcase */}
      <div className="py-12">
        <ProjectShowcase title="EXPLORE PROJECTS" />
      </div>

      {/* Scotch VHS Cassette Modules */}
      <div className="py-12 bg-gradient-to-b from-transparent via-slate-900/80 to-transparent">
        <ScotchCassetteShowcase title="QUANTUM MODULES" />
      </div>

      {/* CTA Section */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-20">
        <div className="bg-gradient-to-r from-purple-600/20 to-blue-600/20 backdrop-blur-xl rounded-3xl p-12 border border-purple-500/30 text-center">
          <h2 className="text-5xl font-bold text-white mb-6">Ready to Experience the Future of DeFi?</h2>
          <p className="text-xl text-gray-300 mb-8 max-w-3xl mx-auto">
            Join the most advanced multi-chain DeFi ecosystem with AI-powered optimization, 
            MEV protection, and atomic execution across 103+ networks.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <button className="bg-gradient-to-r from-purple-600 to-blue-600 text-white px-10 py-4 rounded-xl font-semibold text-lg hover:from-purple-700 hover:to-blue-700 transition-all transform hover:scale-105 flex items-center justify-center gap-2">
              Get Started Now <ArrowRight className="w-5 h-5" />
            </button>
            <button className="border border-purple-500 text-purple-300 px-10 py-4 rounded-xl font-semibold text-lg hover:bg-purple-500/10 transition-all">
              Read the Docs
            </button>
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="border-t border-slate-700/50 py-12">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex flex-col md:flex-row justify-between items-center">
            <div className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-blue-400 bg-clip-text text-transparent mb-4 md:mb-0">
              Atlas Sphere
            </div>
            <div className="flex gap-6 text-gray-400">
              <a href="/developers/docs" className="hover:text-purple-400 transition-colors">Docs</a>
              <a href="/ecosystem" className="hover:text-purple-400 transition-colors">Ecosystem</a>
              <a href="/x3/swarm" className="hover:text-purple-400 transition-colors">AI Swarm</a>
              <a href="https://github.com/atlas-sphere" className="hover:text-purple-400 transition-colors">GitHub</a>
            </div>
          </div>
          <div className="text-center text-gray-500 mt-8 text-sm">
            2025 Atlas Sphere. All rights reserved.
          </div>
        </div>
      </footer>
    </div>
  );
}
