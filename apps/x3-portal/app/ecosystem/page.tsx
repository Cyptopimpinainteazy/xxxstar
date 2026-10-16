'use client';

import React, { useState, useEffect } from 'react';
import dynamic from 'next/dynamic';
import { 
  Globe, 
  Zap, 
  Shield, 
  Coins, 
  TrendingUp, 
  Users, 
  Activity,
  ArrowRight,
  ChevronRight,
  Cpu,
  Network,
  Lock,
  DollarSign,
  BarChart3,
  Target,
  Layers,
  Bot,
  ShieldCheck,
  Clock,
  CheckCircle,
  Rocket,
  Database,
  Link
} from 'lucide-react';

// Dynamic imports for quantum showcase components
const ScotchCassetteShowcase = dynamic(
  () => import('@/components/quantum/ScotchVHSShowcase').then(mod => ({ default: mod.ScotchCassetteShowcase })),
  { ssr: false, loading: () => <div className="h-96 animate-pulse bg-slate-800/50 rounded-2xl" /> }
);

const FeatureShowcase = dynamic(
  () => import('@/components/quantum/ProjectShowcase').then(mod => ({ default: mod.FeatureShowcase })),
  { ssr: false, loading: () => <div className="h-64 animate-pulse bg-slate-800/50 rounded-2xl" /> }
);

// Ecosystem components data
const ecosystemComponents = [
  {
    category: 'Core Infrastructure',
    icon: Layers,
    color: 'from-blue-500 to-cyan-500',
    components: [
      { name: 'Cross-Chain Position Manager', status: 'Production', tvl: '$850M', chains: 103 },
      { name: 'Atomic Swap Router', status: 'Production', volume: '$2.1B', success: '99.9%' },
      { name: 'MEV Protection Layer', status: 'Production', protected: '$5.7B', attacks: '0' },
      { name: 'AI Swarm Coordinator', status: 'Production', nodes: '2,847', optimization: '47%' }
    ]
  },
  {
    category: 'DeFi Protocols',
    icon: Coins,
    color: 'from-green-500 to-emerald-500',
    components: [
      { name: 'Lending Protocol', status: 'Production', tvl: '$1.2B', apy: '8.4%' },
      { name: 'Staking Engine', status: 'Production', staked: '$890M', apy: '12.3%' },
      { name: 'Yield Optimizer', status: 'Production', yield: '15.7%', users: '47K' },
      { name: 'Flash Loan System', status: 'Production', volume: '$340M', loans: '89K' }
    ]
  },
  {
    category: 'Launchpad & Treasury',
    icon: Rocket,
    color: 'from-purple-500 to-pink-500',
    components: [
      { name: 'Token Launchpad', status: 'Production', raised: '$127M', launches: '234' },
      { name: 'NFT Marketplace', status: 'Production', volume: '$45M', items: '15.7K' },
      { name: 'Blockspace Auction', status: 'Production', revenue: '$23M', blocks: '1.2K' },
      { name: 'Treasury Management', status: 'Production', balance: '$89M', distribution: 'Auto' }
    ]
  },
  {
    category: 'AI & Analytics',
    icon: Bot,
    color: 'from-orange-500 to-red-500',
    components: [
      { name: 'Strategy Optimization', status: 'Production', improvement: '47%', strategies: '156' },
      { name: 'Risk Assessment', status: 'Production', accuracy: '94.7%', assessments: '2.1M' },
      { name: 'Arbitrage Detection', status: 'Production', opportunities: '1.2K/day', profit: '$340K' },
      { name: 'Predictive Analytics', status: 'Production', accuracy: '87.3%', predictions: '847K' }
    ]
  }
];

// Chain network data
const supportedChains = [
  { name: 'Ethereum', category: 'L1', status: 'active', tvl: '$450M', color: 'bg-blue-500' },
  { name: 'Arbitrum', category: 'L2', status: 'active', tvl: '$380M', color: 'bg-blue-400' },
  { name: 'Polygon', category: 'L2', status: 'active', tvl: '$290M', color: 'bg-purple-500' },
  { name: 'Optimism', category: 'L2', status: 'active', tvl: '$210M', color: 'bg-red-500' },
  { name: 'Base', category: 'L2', status: 'active', tvl: '$180M', color: 'bg-blue-600' },
  { name: 'Avalanche', category: 'L1', status: 'active', tvl: '$156M', color: 'bg-red-600' },
  { name: 'BNB Chain', category: 'L1', status: 'active', tvl: '$134M', color: 'bg-yellow-500' },
  { name: 'Fantom', category: 'L1', status: 'active', tvl: '$89M', color: 'bg-purple-600' },
  { name: 'Cronos', category: 'L1', status: 'active', tvl: '$67M', color: 'bg-blue-700' },
  { name: 'Harmony', category: 'L1', status: 'active', tvl: '$45M', color: 'bg-blue-800' }
];

// Performance metrics
const performanceMetrics = [
  { label: 'Total Value Locked', value: '$2.8B', change: '+12.4%', icon: Database },
  { label: 'Daily Volume', value: '$127M', change: '+8.7%', icon: TrendingUp },
  { label: 'Active Users', value: '127K', change: '+15.2%', icon: Users },
  { label: 'Transactions', value: '2.8M', change: '+23.1%', icon: Activity },
  { label: 'Cross-chain Bridges', value: '103', change: '+3 networks', icon: Link },
  { label: 'Uptime', value: '99.97%', change: 'Perfect', icon: ShieldCheck }
];

export default function EcosystemPage() {
  const [selectedCategory, setSelectedCategory] = useState(0);
  const [animatedMetrics, setAnimatedMetrics] = useState(performanceMetrics);

  // Animate metrics on load
  useEffect(() => {
    const timer = setTimeout(() => {
      setAnimatedMetrics(performanceMetrics.map((metric, index) => ({
        ...metric,
        delay: index * 100
      })));
    }, 500);

    return () => clearTimeout(timer);
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900">
      {/* Hero Section */}
      <div className="relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-r from-purple-600/20 to-blue-600/20 backdrop-blur-3xl"></div>
        <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16">
          <div className="text-center">
            <h1 className="text-6xl md:text-8xl font-bold bg-gradient-to-r from-white via-purple-200 to-blue-200 bg-clip-text text-transparent mb-6">
              Ecosystem
            </h1>
            <p className="text-xl md:text-2xl text-gray-300 mb-8 max-w-4xl mx-auto">
              Comprehensive Multi-Chain DeFi Infrastructure
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <button className="bg-gradient-to-r from-purple-600 to-blue-600 text-white px-8 py-4 rounded-xl font-semibold text-lg hover:from-purple-700 hover:to-blue-700 transition-all transform hover:scale-105 flex items-center gap-2">
                Explore Components <ArrowRight className="w-5 h-5" />
              </button>
              <button className="border border-purple-500 text-purple-300 px-8 py-4 rounded-xl font-semibold text-lg hover:bg-purple-500/10 transition-all">
                View Documentation
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Performance Metrics Dashboard */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">Ecosystem Performance</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-6">
          {animatedMetrics.map((metric, index) => (
            <div 
              key={index} 
              className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 text-center hover:border-purple-500/50 transition-all duration-300 hover:transform hover:scale-105"
              style={{ animationDelay: `${index * 100}ms` }}
            >
              <metric.icon className="w-12 h-12 text-purple-400 mx-auto mb-4" />
              <div className="text-2xl md:text-3xl font-bold text-white mb-2">{metric.value}</div>
              <div className="text-gray-400 text-sm mb-2">{metric.label}</div>
              <div className="text-green-400 text-xs font-semibold">{metric.change}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Ecosystem Components */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">Production-Ready Components</h2>
        
        {/* Category Tabs */}
        <div className="flex flex-wrap justify-center gap-4 mb-12">
          {ecosystemComponents.map((category, index) => (
            <button
              key={index}
              onClick={() => setSelectedCategory(index)}
              className={`flex items-center gap-3 px-6 py-3 rounded-xl font-semibold transition-all ${
                selectedCategory === index
                  ? 'bg-gradient-to-r from-purple-600 to-blue-600 text-white'
                  : 'bg-slate-800/50 text-gray-300 hover:bg-slate-700/50'
              }`}
            >
              <category.icon className="w-5 h-5" />
              {category.category}
            </button>
          ))}
        </div>

        {/* Selected Category Components */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {ecosystemComponents[selectedCategory].components.map((component, index) => (
            <div 
              key={index}
              className="group bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-6 border border-slate-600/30 hover:border-purple-500/50 transition-all duration-300 hover:transform hover:scale-105"
            >
              <div className="flex items-start justify-between mb-4">
                <h3 className="text-xl font-bold text-white group-hover:text-purple-300 transition-colors">
                  {component.name}
                </h3>
                <div className="flex items-center gap-2">
                  <CheckCircle className="w-5 h-5 text-green-400" />
                  <span className="text-green-400 text-sm font-semibold">{component.status}</span>
                </div>
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                {Object.entries(component).map(([key, value]) => {
                  if (key === 'name' || key === 'status') return null;
                  
                  return (
                    <div key={key} className="text-center">
                      <div className="text-lg font-bold text-purple-400">{String(value)}</div>
                      <div className="text-xs text-gray-400 capitalize">{key}</div>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Supported Networks */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">Supported Networks</h2>
        <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
          <div className="text-center mb-8">
            <div className="text-5xl font-bold text-purple-400 mb-2">103+</div>
            <div className="text-gray-400">Networks Supported</div>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
            {supportedChains.map((chain, index) => (
              <div 
                key={index}
                className="flex items-center gap-3 p-4 bg-slate-800/30 rounded-xl border border-slate-600/20 hover:border-purple-500/30 transition-all"
              >
                <div className={`w-3 h-3 ${chain.color} rounded-full`}></div>
                <div>
                  <div className="text-white font-semibold">{chain.name}</div>
                  <div className="text-gray-400 text-sm">{chain.category}</div>
                </div>
                <div className="ml-auto text-right">
                  <div className="text-green-400 text-sm">{chain.tvl}</div>
                  <div className="text-gray-500 text-xs">TVL</div>
                </div>
              </div>
            ))}
          </div>
          
          <div className="text-center mt-8">
            <button className="text-purple-400 hover:text-purple-300 font-semibold flex items-center gap-2 mx-auto">
              View All 103 Networks <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {/* AI Swarm Showcase */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <h2 className="text-4xl font-bold text-center text-white mb-12">AI-Powered Optimization</h2>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          
          {/* Real-time Optimization */}
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
            <div className="flex items-center gap-3 mb-6">
              <Cpu className="w-8 h-8 text-purple-400" />
              <h3 className="text-2xl font-bold text-white">Live Optimization</h3>
            </div>
            
            <div className="space-y-6">
              <div className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl">
                <div>
                  <div className="text-white font-semibold">Strategy Optimization</div>
                  <div className="text-gray-400 text-sm">Active across 47 strategies</div>
                </div>
                <div className="text-right">
                  <div className="text-green-400 text-xl font-bold">+47%</div>
                  <div className="text-gray-400 text-sm">Improvement</div>
                </div>
              </div>
              
              <div className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl">
                <div>
                  <div className="text-white font-semibold">Arbitrage Detection</div>
                  <div className="text-gray-400 text-sm">1,247 opportunities/day</div>
                </div>
                <div className="text-right">
                  <div className="text-green-400 text-xl font-bold">$340K</div>
                  <div className="text-gray-400 text-sm">Daily Profit</div>
                </div>
              </div>
              
              <div className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl">
                <div>
                  <div className="text-white font-semibold">Risk Assessment</div>
                  <div className="text-gray-400 text-sm">2.1M assessments completed</div>
                </div>
                <div className="text-right">
                  <div className="text-green-400 text-xl font-bold">94.7%</div>
                  <div className="text-gray-400 text-sm">Accuracy</div>
                </div>
              </div>
            </div>
          </div>

          {/* Network Status */}
          <div className="bg-gradient-to-br from-slate-800/50 to-slate-700/50 backdrop-blur-xl rounded-2xl p-8 border border-slate-600/30">
            <div className="flex items-center gap-3 mb-6">
              <Network className="w-8 h-8 text-purple-400" />
              <h3 className="text-2xl font-bold text-white">Network Status</h3>
            </div>
            
            <div className="space-y-6">
              <div className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl">
                <div>
                  <div className="text-white font-semibold">Active Nodes</div>
                  <div className="text-gray-400 text-sm">2,847 GPU nodes online</div>
                </div>
                <div className="text-right">
                  <div className="text-green-400 text-xl font-bold">99.9%</div>
                  <div className="text-gray-400 text-sm">Uptime</div>
                </div>
              </div>
              
              <div className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl">
                <div>
                  <div className="text-white font-semibold">Network Latency</div>
                  <div className="text-gray-400 text-sm">Average response time</div>
                </div>
                <div className="text-right">
                  <div className="text-cyan-400 text-xl font-bold">&lt;0.5s</div>
                  <div className="text-gray-400 text-sm">Response</div>
                </div>
              </div>
              
              <div className="flex items-center justify-between p-4 bg-slate-800/30 rounded-xl">
                <div>
                  <div className="text-white font-semibold">Consensus Health</div>
                  <div className="text-gray-400 text-sm">All validators synchronized</div>
                </div>
                <div className="text-right">
                  <div className="text-green-400 text-xl font-bold">100%</div>
                  <div className="text-gray-400 text-sm">Healthy</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Scotch VHS Cassette Modules */}
      <div className="py-16 bg-gradient-to-b from-transparent via-slate-900/80 to-transparent">
        <ScotchCassetteShowcase title="ECOSYSTEM MODULES" />
      </div>

      {/* Feature Showcase */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
        <FeatureShowcase title="PLATFORM FEATURES" />
      </div>

      {/* CTA Section */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-20">
        <div className="bg-gradient-to-r from-purple-600/20 to-blue-600/20 backdrop-blur-xl rounded-3xl p-12 border border-purple-500/30 text-center">
          <h2 className="text-5xl font-bold text-white mb-6">Join the Ecosystem</h2>
          <p className="text-xl text-gray-300 mb-8 max-w-3xl mx-auto">
            Build on the most advanced multi-chain infrastructure with AI-powered optimization 
            and enterprise-grade security across 103+ networks.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <button className="bg-gradient-to-r from-purple-600 to-blue-600 text-white px-10 py-4 rounded-xl font-semibold text-lg hover:from-purple-700 hover:to-blue-700 transition-all transform hover:scale-105 flex items-center justify-center gap-2">
              Start Building <ArrowRight className="w-5 h-5" />
            </button>
            <button className="border border-purple-500 text-purple-300 px-10 py-4 rounded-xl font-semibold text-lg hover:bg-purple-500/10 transition-all">
              Read Documentation
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
